use crate::{
    graphics::{
        bindgroups::{BindGroupLayoutRegistry, BindGroupRegistry},
        buffers::create_buffer_with_data,
    },
    utils::Registry,
};
use encase::{ShaderType, UniformBuffer};
use glam::{Mat4, Vec3};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, Buffer, BufferSize, BufferUsages, Device, ShaderStages,
};

#[derive(Debug, Copy, Clone, ShaderType)]
pub struct CameraUniform {
    pub view: Mat4,
    pub projection: Mat4,
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            view: Mat4::look_at_rh(
                Vec3 {
                    x: 2.5,
                    y: 0.0,
                    z: 5.0,
                },
                Vec3::ZERO,
                Vec3::Y,
            ),
            projection: Mat4::perspective_rh(0.785, 16.0 / 9.0, 0.1, 100.0),
        }
    }
}

impl CameraUniform {
    pub fn _new(view: Mat4, projection: Mat4) -> Self {
        Self {
            view: view,
            projection: projection,
        }
    }

    pub fn create_and_store_camera_uniform_bindings(
        self,
        device: &Device,
        bind_group_layout_registry: &mut BindGroupLayoutRegistry,
        bind_group_registry: &mut BindGroupRegistry,
    ) -> Result<(), String> {        
        let mut virt_uniform_buf = UniformBuffer::new(Vec::new());
        virt_uniform_buf.write(&self).unwrap();
        let buffer = Self::create_buffer(device, &virt_uniform_buf.into_inner());

        let model_bind_group_layout = Self::create_bind_group_layout(device);

        let bind_group = Self::create_bind_group(device, &buffer, &model_bind_group_layout);

        bind_group_layout_registry.insert(
            String::from("camera_bind_group_layout"),
            model_bind_group_layout,
        );

        bind_group_registry.insert(String::from("camera_bind_group"), bind_group);
        Ok(())
    }

    fn create_buffer(device: &Device, data: &[u8]) -> Buffer {
        let buffer_uses = vec![BufferUsages::UNIFORM, BufferUsages::COPY_DST];
        create_buffer_with_data(device, "uniform camera buffer", data, buffer_uses)
    }

    fn create_bind_group_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("camera bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                count: None,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(size_of::<CameraUniform>() as u64),
                },
                visibility: ShaderStages::VERTEX,
            }],
        })
    }

    fn create_bind_group(
        device: &Device,
        buffer: &Buffer,
        camera_bind_group_layout: &BindGroupLayout,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        })
    }
}
