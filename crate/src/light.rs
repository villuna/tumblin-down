use cgmath::{Deg, Quaternion, Rotation3, Vector3};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct LightUniform {
    pub position: [f32; 3],
    pub scale: f32,
    pub colour: [f32; 3],
    pub brightness: f32,
}

impl LightUniform {
    pub fn new(position: [f32; 3], colour: [f32; 3], scale: f32, brightness: f32) -> Self {
        Self {
            position,
            scale,
            colour,
            brightness,
        }
    }

    pub fn update(&mut self) {
        let position: Vector3<f32> = self.position.into();
        self.position =
            (Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), Deg(0.8)) * position).into();
    }
}
