use std::sync::Arc;

use wgpu::{Buffer, PrimitiveTopology, VertexBufferLayout, vertex_attr_array};

pub mod mesh_allocator;
pub struct Mesh {
    pub vertex_offset: u64,
    pub index_offset: u64,
    pub index_count: u32,
    pub vertex_count: u32,

    pub shared_vertex_buffer: Arc<Buffer>,
    pub shared_index_buffer: Option<Arc<Buffer>>,

    // pub vertex_layout: VertexLayout,
    pub primitive_topology: PrimitiveTopology,
    // pub bounds: Aabb,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
}

impl Vertex {
    pub fn create_buffer_layout<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &vertex_attr_array![0 => Float32x3],
        }
    }


}
