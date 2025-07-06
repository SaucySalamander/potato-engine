use crate::{
    graphics::buffers::{
        BufferEntry, BufferInterface, GpuRingBuffer, bindgroups::create_bind_group, create_buffer,
    },
    utils::{RegisterKey, Registry},
};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::{
    BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BufferSize,
    BufferUsages, Device, Queue, ShaderStages,
};

const MAX_INDIRECT_DRAWS: u64 = 65536;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct IndirectDraw {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
}

impl Default for IndirectDraw {
    fn default() -> Self {
        Self {
            index_count: 0,
            instance_count: 0,
            first_index: 0,
            base_vertex: 0,
            first_instance: 0,
        }
    }
}

impl IndirectDraw {
    pub fn _new() -> Self {
        Self {
            index_count: 0,
            instance_count: 0,
            first_index: 0,
            base_vertex: 0,
            first_instance: 0,
        }
    }

    pub fn create_bind_group_layout(self, device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("indirect_draw_bind_group_layout"),
        })
    }

    pub fn create_and_store_buffers(
        self,
        device: &Device,
        queue: &Queue,
        bind_group_layout: &BindGroupLayout,
        gpu_buffer_registry: &mut Registry<Box<dyn BufferInterface>>,
        frame_index: usize,
    ) -> Result<(), String> {
        let buffer_uses = vec![
            BufferUsages::INDIRECT,
            BufferUsages::STORAGE,
            BufferUsages::COPY_DST,
        ];

        let mut buffer_entries: Vec<BufferEntry> = Vec::new();
        for _ in 0..3 {
            let buffer = create_buffer(
                device,
                "indirect_draw_gpu",
                MAX_INDIRECT_DRAWS * size_of::<IndirectDraw>() as u64,
                buffer_uses.clone(),
                false,
            );

            let bind_group = create_bind_group(
                "camera_gpu_uniform_bind_group",
                device,
                bind_group_layout,
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

        let mut triple_buffered_indirect_draw = GpuRingBuffer::<IndirectDraw>::new(buffer_entries);
        triple_buffered_indirect_draw.write(queue, bytemuck::cast_slice(&vec![self]), frame_index);
        gpu_buffer_registry.register_key(
            RegisterKey::from_label::<GpuRingBuffer<IndirectDraw>>("indirect_draw_buffer"),
            Box::new(triple_buffered_indirect_draw),
        );
        Ok(())
    }
}

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

    pub fn create_bind_group_layout(self, device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("camera_bind_group_layout"),
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

    pub fn create_and_store_buffers(
        self,
        device: &Device,
        queue: &Queue,
        bind_group_layout: &BindGroupLayout,
        gpu_buffer_registry: &mut Registry<Box<dyn BufferInterface>>,
        frame_index: usize,
    ) -> Result<(), String> {
        let buffer_uses = vec![BufferUsages::UNIFORM, BufferUsages::COPY_DST];

        let mut buffer_entries: Vec<BufferEntry> = Vec::new();
        for _ in 0..3 {
            let buffer = create_buffer(
                device,
                "camera_gpu_uniform",
                size_of::<CameraUniform>() as u64,
                buffer_uses.clone(),
                false,
            );

            let bind_group = create_bind_group(
                "camera_gpu_uniform_bind_group",
                device,
                bind_group_layout,
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
        triple_buffered_camera_uniform.write(queue, bytemuck::bytes_of(&self), frame_index);
        gpu_buffer_registry.register_key(
            RegisterKey::from_label::<GpuRingBuffer<CameraUniform>>("camera_gpu_uniform_triple"),
            Box::new(triple_buffered_camera_uniform),
        );
        Ok(())
    }
}

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

    pub fn create_bind_group_layout(self, device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("model_bind_group_layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                count: None,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(
                        MAX_INDIRECT_DRAWS * size_of::<ModelUniform>() as u64,
                    ),
                },
                visibility: ShaderStages::VERTEX,
            }],
        })
    }

    pub fn create_and_store_buffers(
        self,
        device: &Device,
        queue: &Queue,
        bind_group_layout: &BindGroupLayout,
        gpu_buffer_registry: &mut Registry<Box<dyn BufferInterface>>,
        frame_index: usize,
    ) -> Result<(), String> {
        let buffer_uses = vec![BufferUsages::STORAGE, BufferUsages::COPY_DST];

        let mut buffer_entires: Vec<BufferEntry> = Vec::new();
        for _ in 0..3 {
            let buffer = create_buffer(
                device,
                "model_gpu_uniform",
                MAX_INDIRECT_DRAWS * size_of::<ModelUniform>() as u64,
                buffer_uses.clone(),
                false,
            );

            let bind_group = create_bind_group(
                "model_gpu_uniform_bind_group",
                device,
                bind_group_layout,
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

        let empty_models = vec![
            ModelUniform {
                model: Mat4::IDENTITY.to_cols_array_2d(),
            };
            MAX_INDIRECT_DRAWS as usize
        ];

        let mut triple_buffered_model_uniform = GpuRingBuffer::<ModelUniform>::new(buffer_entires);
        triple_buffered_model_uniform.write(
            queue,
            bytemuck::cast_slice(&empty_models),
            frame_index,
        );
        gpu_buffer_registry.register_key(
            RegisterKey::from_label::<GpuRingBuffer<ModelUniform>>("model_gpu_uniform_triple"),
            Box::new(triple_buffered_model_uniform),
        );
        Ok(())
    }
}
