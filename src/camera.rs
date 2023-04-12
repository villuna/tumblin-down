use cgmath::{Point3, Matrix4, perspective, Deg, Vector3, Rad, Matrix3};

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
        let direction = Matrix3::from_angle_x(Rad(self.v_angle)) * Matrix3::from_angle_y(Rad(self.h_angle)) * (-1f32 * Vector3::unit_z());
        let target = self.eye + direction;
        let view = Matrix4::look_at_rh(self.eye, target, self.up);
        let projection = perspective(Deg(self.fovy), self.aspect, self.znear, self.zfar);

        OPENGL_TO_WGPU_MATRIX * projection * view
    }

    pub fn to_uniform(&self) -> CameraUniform {
        CameraUniform { matrix: self.build_camera_matrix().into() }
    }
}