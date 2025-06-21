use super::graphics::bindgroups::{BindGroupLayoutRegistry, BindGroupRegistry};
use crate::{
    engine::graphics::buffers::{
        BufferEntry, BufferInterface, BufferRegistry, GpuRingBuffer, create_buffer,
    },
    utils::Registry,
};

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wgpu::{
    BindGroupEntry, BindGroupLayoutEntry, BufferSize, BufferUsages, Device, Queue, ShaderStages,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct ModelUniform {
    pub model: [[f32; 4]; 4],
}

impl Default for ModelUniform {
    fn default() -> Self {
        Self {
            model: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}

impl ModelUniform {
    pub fn _new(model: [[f32; 4]; 4]) -> Self {
        Self { model }
    }

    pub fn create_and_store_buffers(
        self,
        device: &Device,
        queue: &Queue,
        bind_group_layout_registry: &mut BindGroupLayoutRegistry,
        gpu_buffer_registry: &mut BufferRegistry<Box<dyn BufferInterface>>,
        frame_index: usize,
    ) -> Result<(), String> {
        let buffer_uses = vec![BufferUsages::UNIFORM, BufferUsages::COPY_DST];

        let bind_group_layout = BindGroupLayoutRegistry::create_bind_group_layout(
            "model bind group layout",
            device,
            &vec![BindGroupLayoutEntry {
                binding: 0,
                count: None,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(size_of::<ModelUniform>() as u64),
                },
                visibility: ShaderStages::VERTEX,
            }],
        );

        let mut buffer_entires: Vec<BufferEntry> = Vec::new();
        for _ in 0..3 {
            let buffer = create_buffer(
                device,
                "model_gpu_uniform",
                size_of::<ModelUniform>() as u64,
                buffer_uses.clone(),
                false,
            );

            let bind_group = BindGroupRegistry::create_bind_group(
                "model_gpu_uniform_bind_group",
                device,
                &bind_group_layout,
                &vec![BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            );
            buffer_entires.push(BufferEntry {
                buffer,
                bind_group: Some(bind_group),
            });
        }

        let mut triple_buffered_model_uniform = GpuRingBuffer::<ModelUniform>::new(buffer_entires);
        triple_buffered_model_uniform.write(queue, &self, frame_index);
        bind_group_layout_registry
            .insert(String::from("model_bind_group_layout"), bind_group_layout);
        gpu_buffer_registry.insert(
            String::from("model_gpu_uniform_triple"),
            Box::new(triple_buffered_model_uniform),
        );
        Ok(())
    }
}
