use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct CameraUniform {
    pub view: [[f32; 4]; 4],
    pub projection: [[f32; 4]; 4],
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            view: Mat4::look_at_rh(
                Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 5.0,
                },
                Vec3::ZERO,
                Vec3::Y,
            )
            .to_cols_array_2d(),
            projection: Mat4::perspective_rh(0.785, 16.0 / 9.0, 0.1, 10.0).to_cols_array_2d(),
        }
    }
}
