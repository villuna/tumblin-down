use std::f32::consts::PI;

use cgmath::{Point3, Matrix4, perspective, Deg, Vector3, Rad, Matrix3, vec3, InnerSpace};
use winit::event::VirtualKeyCode;

use crate::input::KeyboardWatcher;

const ROTATION_SPEED: f32 = 0.03;
const MOVE_SPEED: f32 = 0.1;
const HALFPI: f32 = PI / 2.0;

pub struct Camera {
    pub eye: Point3<f32>,
    pub h_angle: f32, // Horizontal angle in radians (h_angle \in [0, 2pi))
    pub v_angle: f32, // Vertical angle in radians (v_angle \in [-pi/2, pi/2])
    pub up: Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct CameraUniform {
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
        CameraUniform { matrix: self.build_camera_matrix().into() }
    }

    // Updates the direction of the camera in response to input.
    // returns true if the camera changed.
    pub fn update(&mut self, keyboard: &KeyboardWatcher) -> bool {
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
        self.h_angle = (self.h_angle + hrot * ROTATION_SPEED) % (2.0*PI);

        if hdir != 0.0 || fdir != 0.0 {
            let xz_dir = self.direction_matrix() * vec3(hdir, 0.0, fdir);
            let xz_move = vec3(xz_dir.x, 0.0, xz_dir.z).normalize() * MOVE_SPEED;
            self.eye += xz_move;
        }

        if vdir != 0.0 {
            self.eye.y += vdir * MOVE_SPEED;
        }

        vrot != 0.0 || hrot != 0.0 || hdir != 0.0 || vdir != 0.0 || fdir != 0.0
    }
}