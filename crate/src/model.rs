// TODO: Switch over entirely to nalgebra to work well with rapier3d
use std::io::{BufReader, Cursor};

use crate::{resources, texture};
use cgmath::{vec3, Matrix4, Quaternion, Vector3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    vertex_attr_array, VertexBufferLayout,
};

use rapier3d::na;

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub struct ModelVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    normal: [f32; 3],
}

#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
    rotation: [[f32; 3]; 3],
}

#[derive(Debug)]
pub struct Instance {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
}

/// A 3d object that may be made up of multiple meshes,
/// which may refer to multiple materials.
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

/// A single 3d object. This struct contains a handle to a vertex and index
/// buffer on the GPU, as well as the index of its material (stored in the
/// parent Model struct).
pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub material: Option<usize>,
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: Option<texture::Texture>,
    pub diffuse_bind_group: Option<wgpu::BindGroup>,
}

impl Model {
    pub async fn load(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        filename: &str,
        texture_layout: Option<&wgpu::BindGroupLayout>,
    ) -> anyhow::Result<Self> {
        // Get the path of the parent so we can load materials
        let parent = std::path::Path::new(filename)
            .parent()
            .unwrap_or(std::path::Path::new(""));

        // After doing some testing, it seems like relative_path isn't very sophisticated
        // so TODO: Refactor this to just use normal paths and save a dependency?
        let format_path = |path: &str| {
            let new_path = relative_path::RelativePath::new(path).to_path(parent);
            new_path.as_path().to_str().unwrap().to_string()
        };

        let data = resources::load_string(filename).await?;
        // A cursor allows us to implement Read on a String so we can use it
        // in a buffered reader, which is required for tobj to load from memory.
        let cursor = Cursor::new(data);

        let mut reader = BufReader::new(cursor);

        let (meshes, materials) = tobj::load_obj_buf_async(
            &mut reader,
            &tobj::LoadOptions {
                single_index: true,
                triangulate: true,
                ignore_points: true,
                ignore_lines: true,
            },
            |p| async move {
                let filename = format_path(&p);
                let mat_string = resources::load_string(&filename).await.unwrap();
                let mat_cursor = Cursor::new(mat_string);
                let mut mat_reader = BufReader::new(mat_cursor);
                tobj::load_mtl_buf(&mut mat_reader)
            },
        )
        .await?;

        let meshes = meshes
            .into_iter()
            .map(|model| {
                let mesh = model.mesh;

                let vertices = (0..mesh.positions.len() / 3)
                    .map(|i| ModelVertex {
                        position: [
                            mesh.positions[3 * i],
                            mesh.positions[3 * i + 1],
                            mesh.positions[3 * i + 2],
                        ],
                        tex_coords: [mesh.texcoords[2 * i], 1.0 - mesh.texcoords[2 * i + 1]],
                        normal: [
                            mesh.normals[3 * i],
                            mesh.normals[3 * i + 1],
                            mesh.normals[3 * i + 2],
                        ],
                    })
                    .collect::<Vec<_>>();

                let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
                    label: Some(&format!("{}/{} vertex buffer", filename, model.name)),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

                let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
                    label: Some(&format!("{}/{} index buffer", filename, model.name)),
                    contents: bytemuck::cast_slice(&mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                Mesh {
                    name: model.name,
                    vertex_buffer,
                    index_buffer,
                    num_indices: mesh.indices.len() as _,
                    material: mesh.material_id,
                }
            })
            .collect::<Vec<_>>();

        let mut new_materials = Vec::new();

        for mat in materials?.into_iter() {
            let diffuse_filename = format_path(mat.diffuse_texture.as_ref().unwrap());
            let texture = texture::Texture::load_texture(&device, &queue, &diffuse_filename)
                .await
                .ok();

            // TODO: This rubs me the wrong way. We're passed in the texture bind group layout
            // but then we just go ahead and use this layout instead. Is there some way to
            // make it so the object loading function doesn't say anything about the layout
            // of the texture bind group?
            let bind_group = texture
                .as_ref()
                .and_then(|tex| Some((tex, texture_layout?)))
                .map(|(texture, layout)| {
                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some(&format!("{}/{} texture bind group", filename, mat.name)),
                        layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&texture.view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&texture.sampler),
                            },
                        ],
                    })
                });

            new_materials.push(Material {
                name: mat.name,
                diffuse_texture: texture,
                diffuse_bind_group: bind_group,
            });
        }

        Ok(Model {
            meshes,
            materials: new_materials,
        })
    }
}

impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (Matrix4::from_translation(self.position) * Matrix4::from(self.rotation)).into(),
            rotation: cgmath::Matrix3::from(self.rotation).into(),
        }
    }

    pub fn from_rapier_position(
        position: &na::Isometry<f32, na::Unit<na::Quaternion<f32>>, 3>,
    ) -> Self {
        let rotation = Quaternion::new(
            position.rotation.w,
            position.rotation.i,
            position.rotation.j,
            position.rotation.k,
        );
        let position = vec3(
            position.translation.x,
            position.translation.y,
            position.translation.z,
        );

        Self { rotation, position }
    }
}

impl ModelVertex {
    const ATTRS: &'static [wgpu::VertexAttribute] =
        &vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x3];
}

impl Vertex for ModelVertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<ModelVertex>() as _,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRS,
        }
    }
}

impl Vertex for InstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We'll have to reassemble the mat4 in
                // the shader.
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 19]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 22]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}
