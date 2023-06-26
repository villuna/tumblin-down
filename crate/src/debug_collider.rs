use crate::model::Instance;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    vertex_attr_array,
};

/// Contains a [rapier3d] collider, along with various things needed
/// to draw the collider to the screen for debug purposes.
pub struct DebugCollider {
    collider: rapier3d::prelude::Collider,
    // Stuff for rendering
    collider_vertex_buffer: wgpu::Buffer,
    collider_index_buffer: wgpu::Buffer,
    collider_indices: u32,
    outline_vertex_buffer: wgpu::Buffer,
    outline_index_buffer: wgpu::Buffer,
    outline_indices: u32,
    instance_buffer: wgpu::Buffer,
}

impl DebugCollider {
    pub fn new_capsule(device: &wgpu::Device, collider: rapier3d::prelude::Collider) -> Self {
        let (vertices, indices) = collider.shape().as_capsule().unwrap().to_trimesh(20, 20);

        let vertices = vertices.iter().map(|p| [p.x, p.y, p.z]).collect::<Vec<_>>();

        let indices = indices.iter().flatten().copied().collect::<Vec<u32>>();

        let (outline_vertices, outline_indices) =
            collider.shape().as_capsule().unwrap().to_outline(20);

        let outline_vertices = outline_vertices
            .iter()
            .map(|p| [p.x, p.y, p.z])
            .collect::<Vec<_>>();

        let outline_indices = outline_indices
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<u32>>();

        Self::new(
            device,
            collider,
            vertices,
            indices,
            outline_vertices,
            outline_indices,
        )
    }

    pub fn new_round_cylinder(
        device: &wgpu::Device,
        collider: rapier3d::prelude::Collider,
    ) -> Self {
        let (vertices, indices) = collider
            .shape()
            .as_round_cylinder()
            .unwrap()
            .inner_shape
            .to_trimesh(20);

        let vertices = vertices.iter().map(|p| [p.x, p.y, p.z]).collect::<Vec<_>>();

        let indices = indices.iter().flatten().copied().collect::<Vec<u32>>();

        let (outline_vertices, outline_indices) = collider
            .shape()
            .as_round_cylinder()
            .unwrap()
            .to_outline(20, 20);

        let outline_vertices = outline_vertices
            .iter()
            .map(|p| [p.x, p.y, p.z])
            .collect::<Vec<_>>();

        let outline_indices = outline_indices
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<u32>>();

        Self::new(
            device,
            collider,
            vertices,
            indices,
            outline_vertices,
            outline_indices,
        )
    }

    fn new(
        device: &wgpu::Device,
        collider: rapier3d::prelude::Collider,
        vertices: Vec<[f32; 3]>,
        indices: Vec<u32>,
        outline_vertices: Vec<[f32; 3]>,
        outline_indices: Vec<u32>,
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("collider vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Collider index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        let outline_vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("collider vertex buffer"),
            contents: bytemuck::cast_slice(&outline_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let outline_index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Collider index buffer"),
            contents: bytemuck::cast_slice(&outline_indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        let instance = Instance::from_rapier_position(collider.position());

        let instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Collider instance buffer"),
            contents: bytemuck::cast_slice(&[instance.to_raw()]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            collider,
            collider_vertex_buffer: vertex_buffer,
            collider_index_buffer: index_buffer,
            collider_indices: indices.len() as _,
            outline_vertex_buffer,
            outline_index_buffer,
            outline_indices: outline_indices.len() as _,
            instance_buffer,
        }
    }

    pub fn draw<'r, 's>(&'s self, render_pass: &mut wgpu::RenderPass<'r>)
    where
        's: 'r,
    {
        render_pass.set_vertex_buffer(0, self.collider_vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(
            self.collider_index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(0..self.collider_indices, 0, 0..1);
    }

    pub fn draw_outline<'r, 's>(&'s self, render_pass: &mut wgpu::RenderPass<'r>)
    where
        's: 'r,
    {
        render_pass.set_vertex_buffer(0, self.outline_vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(
            self.outline_index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(0..self.outline_indices, 0, 0..1);
    }

    pub fn update_capsule(&self, queue: &wgpu::Queue) {
        let (vertices, indices) = self
            .collider
            .shape()
            .as_capsule()
            .unwrap()
            .to_trimesh(20, 20);

        let vertices = vertices.iter().map(|p| [p.x, p.y, p.z]).collect::<Vec<_>>();

        let indices = indices.iter().flatten().copied().collect::<Vec<u32>>();

        let (outline_vertices, outline_indices) =
            self.collider.shape().as_capsule().unwrap().to_outline(20);

        let outline_vertices = outline_vertices
            .iter()
            .map(|p| [p.x, p.y, p.z])
            .collect::<Vec<_>>();

        let outline_indices = outline_indices
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<u32>>();

        queue.write_buffer(
            &self.collider_vertex_buffer,
            0,
            bytemuck::cast_slice(&vertices),
        );
        queue.write_buffer(
            &self.collider_index_buffer,
            0,
            bytemuck::cast_slice(&indices),
        );
        queue.write_buffer(
            &self.outline_vertex_buffer,
            0,
            bytemuck::cast_slice(&outline_vertices),
        );
        queue.write_buffer(
            &self.outline_index_buffer,
            0,
            bytemuck::cast_slice(&outline_indices),
        );
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&[
                Instance::from_rapier_position(self.collider.position()).to_raw()
            ]),
        );
    }

    pub fn update_round_cylinder(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let (vertices, indices) = self
            .collider
            .shape()
            .as_round_cylinder()
            .unwrap()
            .inner_shape
            .to_trimesh(20);

        let vertices = vertices.iter().map(|p| [p.x, p.y, p.z]).collect::<Vec<_>>();

        let indices = indices.iter().flatten().copied().collect::<Vec<u32>>();

        let (outline_vertices, outline_indices) = self
            .collider
            .shape()
            .as_round_cylinder()
            .unwrap()
            .to_outline(20, 20);

        let outline_vertices = outline_vertices
            .iter()
            .map(|p| [p.x, p.y, p.z])
            .collect::<Vec<_>>();

        let outline_indices = outline_indices
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<u32>>();

        queue.write_buffer(
            &self.collider_vertex_buffer,
            0,
            bytemuck::cast_slice(&vertices),
        );
        queue.write_buffer(
            &self.collider_index_buffer,
            0,
            bytemuck::cast_slice(&indices),
        );

        self.outline_vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("collider vertex buffer"),
            contents: bytemuck::cast_slice(&outline_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        self.outline_index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Collider index buffer"),
            contents: bytemuck::cast_slice(&outline_indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&[
                Instance::from_rapier_position(self.collider.position()).to_raw()
            ]),
        );
    }

    pub fn vertex_desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 3]>() as _,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &vertex_attr_array![0 => Float32x3],
        }
    }
}
