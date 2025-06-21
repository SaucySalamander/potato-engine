pub mod fps_camera;

use super::graphics::{
    bindgroups::{BindGroupLayoutRegistry, BindGroupRegistry},
    buffers::{BufferEntry, BufferInterface, BufferRegistry},
};
use crate::{
    engine::graphics::buffers::{GpuRingBuffer, create_buffer},
    utils::Registry,
};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::{
    BindGroupEntry, BindGroupLayoutEntry, BufferSize, BufferUsages, Device, Queue, ShaderStages,
};

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

impl CameraUniform {
    pub fn _new(view: [[f32; 4]; 4], projection: [[f32; 4]; 4]) -> Self {
        Self {
            view: view,
            projection: projection,
        }
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

        //potentially move the layout creation out of this method.
        let bind_group_layout = BindGroupLayoutRegistry::create_bind_group_layout(
            "camera bind group layout",
            device,
            &vec![BindGroupLayoutEntry {
                binding: 0,
                count: None,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(size_of::<CameraUniform>() as u64),
                },
                visibility: ShaderStages::VERTEX,
            }],
        );
        //---------------

        let mut buffer_entries: Vec<BufferEntry> = Vec::new();
        for _ in 0..3 {
            let buffer = create_buffer(
                device,
                "camera_gpu_uniform",
                size_of::<CameraUniform>() as u64,
                buffer_uses.clone(),
                false,
            );

            let bind_group = BindGroupRegistry::create_bind_group(
                "camera_gpu_uniform_bind_group",
                device,
                &bind_group_layout,
                &vec![BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            );

            buffer_entries.push(BufferEntry {
                buffer: buffer,
                bind_group: Some(bind_group),
            });
        }

        let mut triple_buffered_camera_uniform =
            GpuRingBuffer::<CameraUniform>::new(buffer_entries);
        triple_buffered_camera_uniform.write(queue, &self, frame_index);
        bind_group_layout_registry
            .insert(String::from("camera_bind_group_layout"), bind_group_layout);
        gpu_buffer_registry.insert(
            String::from("camera_gpu_uniform_triple"),
            Box::new(triple_buffered_camera_uniform),
        );
        Ok(())
    }
}
