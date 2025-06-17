use crate::{
    graphics::{
        bindgroups::{BindGroupLayoutRegistry, BindGroupRegistry},
        buffers::create_buffer_with_data,
    },
    utils::Registry,
};
use encase::{ShaderType, UniformBuffer};
use glam::Mat4;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, Buffer, BufferSize, BufferUsages, Device, ShaderStages,
};

#[derive(Debug, Copy, Clone, ShaderType)]
pub struct ModelUniform {
    pub model: Mat4,
}

impl ModelUniform {
    pub fn new(model: Mat4) -> Self {
        Self { model }
    }

    pub fn create_and_store_model_uniform_bindings(
        self,
        device: &Device,
        bind_group_layout_registry: &mut BindGroupLayoutRegistry,
        bind_group_registry: &mut BindGroupRegistry,
    ) -> Result<(), String> {
        let mut virt_uniform_buf =UniformBuffer::new(Vec::new());
        virt_uniform_buf.write(&self.model).unwrap();
        let buffer = Self::create_model_uniform_buffer(device, &virt_uniform_buf.into_inner());

        let model_bind_group_layout = Self::create_model_uniform_bind_group_layout(device);

        let bind_group = Self::create_bind_group(device, &buffer, &model_bind_group_layout);

        bind_group_layout_registry.insert(
            String::from("model_bind_group_layout"),
            model_bind_group_layout,
        );

        bind_group_registry.insert(String::from("model_bind_group"), bind_group);
        Ok(())
    }

    fn create_model_uniform_buffer(device: &Device, data: &[u8]) -> Buffer {
        let buffer_uses = vec![BufferUsages::UNIFORM, BufferUsages::COPY_DST];
        create_buffer_with_data(device, "uniform model buffer", data, buffer_uses)
    }

    fn create_model_uniform_bind_group_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("model bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                count: None,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(size_of::<Mat4>() as u64),
                },
                visibility: ShaderStages::VERTEX,
            }],
        })
    }

    fn create_bind_group(
        device: &Device,
        buffer: &Buffer,
        model_bind_group_layout: &BindGroupLayout,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("Model Bind Group"),
            layout: model_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        })
    }
}
