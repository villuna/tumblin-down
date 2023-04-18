use cgmath::{Deg, Quaternion, Rotation3, Vector3};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct LightUniform {
    pub position: [f32; 3],
    _padding0: u32,
    pub colour: [f32; 3],
    _padding1: u32,
}

impl LightUniform {
    pub fn new(position: [f32; 3], colour: [f32; 3]) -> Self {
        Self {
            position,
            _padding0: 0,
            colour,
            _padding1: 0,
        }
    }

    pub fn update(&mut self) {
        let position: Vector3<f32> = self.position.into();
        self.position =
            (Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), Deg(0.8)) * position).into();
    }
}
