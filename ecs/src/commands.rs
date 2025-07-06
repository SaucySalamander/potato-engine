use glam::Mat4;

use crate::components::{MeshHandle, Transform};

#[derive(Debug, Clone)]
pub struct IndirectDrawCommand {
    pub first_instance: u32,
    pub instance_count: u32,
    pub mesh: MeshHandle,
    pub transform: Vec<Transform>,
}

impl Default for IndirectDrawCommand {
    fn default() -> Self {
        Self {
            instance_count: 0,
            first_instance: 0,
            mesh: MeshHandle {
                vertex_offset: 0,
                index_offset: 0,
                vertex_count: 0,
                index_count: 0,
            },
            transform: vec![Transform(Mat4::IDENTITY)],
        }
    }
}
