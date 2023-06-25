use std::{f32::consts::PI, sync::OnceLock};

use cgmath::{perspective, vec3, Deg, InnerSpace, Matrix3, Matrix4, Point3, Rad, Vector3};
use winit::event::VirtualKeyCode;

use crate::input::KeyboardWatcher;

const ROTATION_SPEED: f32 = 0.03;
const MOVE_SPEED: f32 = 0.1;
const HALFPI: f32 = PI / 2.0;

static CAMERA_BIND_GROUP_LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();

pub struct Camera {
    pub eye: Point3<f32>,
    pub h_angle: f32, // Horizontal angle in radians (h_angle \in [0, 2pi))
    pub v_angle: f32, // Vertical angle in radians (v_angle \in [-pi/2, pi/2])
    pub up: Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,

    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct CameraUniform {
    position: [f32; 4],
    matrix: [[f32; 4]; 4],
}

#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

impl Camera {
    pub fn bind_group_layout(device: &wgpu::Device) -> &wgpu::BindGroupLayout {
        CAMERA_BIND_GROUP_LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<CameraUniform>() as _,
                        ),
                    },
                    count: None,
                }],
            })
        })
    }

    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        position: Point3<f32>,
        aspect: f32,
    ) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera uniform buffer"),
            size: std::mem::size_of::<CameraUniform>() as _,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera bind group"),
            layout: &Self::bind_group_layout(device),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        let camera = Self {
            eye: position,
            h_angle: 0.0,
            v_angle: 0.0,
            up: cgmath::Vector3::unit_y(),
            aspect,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
            buffer,
            bind_group,
        };

        queue.write_buffer(
            &camera.buffer,
            0,
            bytemuck::cast_slice(&[camera.to_uniform()]),
        );

        camera
    }

    pub fn build_camera_matrix(&self) -> Matrix4<f32> {
        let direction = self.direction_matrix() * (-1f32 * Vector3::unit_z());
        let target = self.eye + direction;
        let view = Matrix4::look_at_rh(self.eye, target, self.up);
        let projection = perspective(Deg(self.fovy), self.aspect, self.znear, self.zfar);

        OPENGL_TO_WGPU_MATRIX * projection * view
    }

    fn direction_matrix(&self) -> Matrix3<f32> {
        Matrix3::from_angle_y(Rad(self.h_angle)) * Matrix3::from_angle_x(Rad(self.v_angle))
    }

    pub fn to_uniform(&self) -> CameraUniform {
        CameraUniform {
            position: self.eye.to_homogeneous().into(),
            matrix: self.build_camera_matrix().into(),
        }
    }

    // Updates the direction of the camera in response to input.
    // returns true if the camera changed.
    pub fn update(&mut self, queue: &wgpu::Queue, keyboard: &KeyboardWatcher) {
        let mut vdir = 0.0;
        let mut hdir = 0.0;
        let mut fdir = 0.0;
        let mut vrot = 0.0;
        let mut hrot = 0.0;

        // There has to be a better way to do this
        if keyboard.pressed(VirtualKeyCode::A) {
            hdir -= 1.0;
        }
        if keyboard.pressed(VirtualKeyCode::D) {
            hdir += 1.0;
        }
        if keyboard.pressed(VirtualKeyCode::W) {
            fdir -= 1.0;
        }
        if keyboard.pressed(VirtualKeyCode::S) {
            fdir += 1.0;
        }
        if keyboard.pressed(VirtualKeyCode::Space) {
            vdir += 1.0;
        }
        if keyboard.pressed(VirtualKeyCode::LShift) {
            vdir -= 1.0;
        }

        if keyboard.pressed(VirtualKeyCode::Left) {
            hrot += 1.0;
        }
        if keyboard.pressed(VirtualKeyCode::Right) {
            hrot -= 1.0;
        }
        if keyboard.pressed(VirtualKeyCode::Up) {
            vrot += 1.0;
        }
        if keyboard.pressed(VirtualKeyCode::Down) {
            vrot -= 1.0;
        }

        self.v_angle = (self.v_angle + vrot * ROTATION_SPEED).clamp(-HALFPI + 0.05, HALFPI - 0.05);
        self.h_angle = (self.h_angle + hrot * ROTATION_SPEED) % (2.0 * PI);

        if hdir != 0.0 || fdir != 0.0 {
            let xz_dir = self.direction_matrix() * vec3(hdir, 0.0, fdir);
            let xz_move = vec3(xz_dir.x, 0.0, xz_dir.z).normalize() * MOVE_SPEED;
            self.eye += xz_move;
        }

        if vdir != 0.0 {
            self.eye.y += vdir * MOVE_SPEED;
        }

        let did_update = vrot != 0.0 || hrot != 0.0 || hdir != 0.0 || vdir != 0.0 || fdir != 0.0;

        if did_update {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.to_uniform()]));
        }
    }
}
