use std::io::{BufReader, Cursor};

use crate::{texture, resources};
use wgpu::{vertex_attr_array, VertexBufferLayout, util::{DeviceExt, BufferInitDescriptor}};

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

// By the grace of god these values add up to a multiple of 4
// allelujah!
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub struct ModelVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    normal: [f32; 3],
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
    pub material: usize,
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub diffuse_bind_group: wgpu::BindGroup,
}

impl Model {
    pub async fn load(device: &wgpu::Device, queue: &wgpu::Queue, filename: &str, texture_layout: &wgpu::BindGroupLayout) -> anyhow::Result<Self> {
        // Get the path of the parent so we can load materials
        let parent = std::path::Path::new(filename).parent().unwrap_or(std::path::Path::new(""));

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
            }
        ).await?;

        let meshes = meshes.into_iter()
            .map(|model| {
                let mesh = model.mesh;

                let vertices = (0..mesh.positions.len() / 3).map(|i| {
                    ModelVertex {
                        position: [
                            mesh.positions[3 * i],
                            mesh.positions[3 * i + 1],
                            mesh.positions[3 * i + 2],
                        ],
                        tex_coords: [
                            mesh.texcoords[2 * i],
                            1.0 - mesh.texcoords[2 * i + 1]
                        ],
                        normal: [
                            mesh.normals[3 * i],
                            mesh.normals[3 * i + 1],
                            mesh.normals[3 * i + 2],
                        ],
                    }
                }).collect::<Vec<_>>();

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
                    material: mesh.material_id.unwrap_or(0),
                }
            }).collect::<Vec<_>>();
        
        let mut new_materials = Vec::new();

        for mat in materials?.into_iter() {
            let diffuse_filename = format_path(&mat.diffuse_texture);
            let texture = texture::Texture::load_texture(&device, &queue, &diffuse_filename)
                .await?;

            // TODO: This rubs me the wrong way. We're passed in the texture bind group layout
            // but then we just go ahead and use this layout instead. Is there some way to
            // make it so the object loading function doesn't say anything about the layout
            // of the texture bind group?
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor { 
                label: Some(&format!("{}/{} texture bind group", filename, mat.name)), 
                layout: texture_layout, 
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
            });

            new_materials.push(Material {
                name: mat.name,
                diffuse_texture: texture,
                diffuse_bind_group: bind_group,
            })
        }

        Ok(Model {
            meshes,
            materials: new_materials,
        })
    }
}

impl ModelVertex {
    const ATTRS: &[wgpu::VertexAttribute] =
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
