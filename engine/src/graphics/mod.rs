use std::{process, sync::Mutex};

use ecs::{
    World,
    components::{Camera, FpsCamera, MeshHandle, Position, Transform},
};
use glam::{Mat4, Vec3};
use log::{error, info};
use pollster::FutureExt;
use wgpu::{
    Adapter, BufferSize, CommandEncoder, Device, DeviceDescriptor, Features, Instance, Limits,
    Operations, Queue, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, RenderPipeline, RequestAdapterOptions, Surface, TextureView, Trace,
    util::StagingBelt,
};

use crate::{
    r#async::FrameIndex,
    graphics::buffers::{
        BufferInterface, GpuRingBuffer,
        submissions::{CameraUniform, IndirectDraw, ModelUniform},
    },
    graphics::mesh::mesh_allocator::MeshAllocator,
    graphics::viewports::ViewportDescription,
    utils::{RegisterKey, Registry},
};

pub mod buffers;
pub mod mesh;
pub mod shaders;
pub mod viewports;

#[derive(Debug)]
pub struct GPUContext {
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

impl GPUContext {
    pub fn init(instance: &Instance, surface: &Surface) -> Self {
        info!("requesting adpater");
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                compatible_surface: Some(surface),
                ..Default::default()
            })
            .block_on()
            .unwrap_or_else(|err| {
                error!("failed to request an adapter {}", err);
                process::exit(1);
            });

        info!("requesting device and queue");
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: None,
                required_features: Features::empty(),
                required_limits: Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: Trace::Off,
            })
            .block_on()
            .unwrap_or_else(|err| {
                error!("failed to retrieve device and queue {}", err);
                process::exit(1);
            });

        Self {
            adapter: adapter,
            device: device,
            queue: queue,
        }
    }
}

pub fn init_render_pass(
    encoder: &mut CommandEncoder,
    view: &TextureView,
    descriptor: &ViewportDescription,
    render_pipeline: &RenderPipeline,
    gpu_buffer_registry: &mut Registry<Box<dyn BufferInterface>>,
    frame_index: &mut FrameIndex,
    mesh_allocator: &mut MeshAllocator,
) {
    let render_pass_descriptor = &RenderPassDescriptor {
        label: Some("Example render pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: view,
            resolve_target: None,
            ops: Operations {
                load: wgpu::LoadOp::Clear(descriptor.background),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: &descriptor.depth.as_ref().unwrap().view,
            depth_ops: Some(Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    };
    let mut render_pass = encoder.begin_render_pass(render_pass_descriptor);

    render_pass.set_pipeline(render_pipeline);

    let main_gpu_camera_key =
        RegisterKey::from_label::<GpuRingBuffer<CameraUniform>>("camera_gpu_uniform_triple");
    if let Some(camera_uniform_buffer_entry) = gpu_buffer_registry.get(&main_gpu_camera_key) {
        if let Some(gpu_ring_buffer) = camera_uniform_buffer_entry
            .as_any()
            .downcast_ref::<GpuRingBuffer<CameraUniform>>()
        {
            let camera_bind_group = gpu_ring_buffer
                .get_read(frame_index.index())
                .bind_group
                .as_ref()
                .unwrap();
            render_pass.set_bind_group(0, Some(camera_bind_group), &[]);
        }
    }

    let main_gpu_model_key =
        RegisterKey::from_label::<GpuRingBuffer<ModelUniform>>("model_gpu_uniform_triple");

    if let Some(model_uniform_buffer_entry) = gpu_buffer_registry.get(&main_gpu_model_key) {
        if let Some(gpu_ring_buffer) = model_uniform_buffer_entry
            .as_any()
            .downcast_ref::<GpuRingBuffer<ModelUniform>>()
        {
            let model_bind_group = gpu_ring_buffer
                .get_read(frame_index.index())
                .bind_group
                .as_ref()
                .unwrap();
            render_pass.set_bind_group(1, Some(model_bind_group), &[]);
        }
    }

    let indirect_draw_gpu_key =
        RegisterKey::from_label::<GpuRingBuffer<IndirectDraw>>("indirect_draw_buffer");
    let indirect_draw_gpu_entry = gpu_buffer_registry
        .get(&indirect_draw_gpu_key)
        .unwrap()
        .as_any()
        .downcast_ref::<GpuRingBuffer<IndirectDraw>>()
        .unwrap();
    let indirect_draw_bind_group = indirect_draw_gpu_entry
        .get_read(frame_index.index())
        .bind_group
        .as_ref()
        .unwrap();
    render_pass.set_bind_group(2, Some(indirect_draw_bind_group), &[]);
    let indirect_draw_buffer = &indirect_draw_gpu_entry.get_read(frame_index.index()).buffer;

    render_pass.set_vertex_buffer(
        0,
        mesh_allocator
            .get_current_vertex_buffer(frame_index.index())
            .slice(..),
    );
    render_pass.set_index_buffer(
        mesh_allocator
            .get_current_index_buffer(frame_index.index())
            .slice(..),
        wgpu::IndexFormat::Uint32,
    );

    let draw_count = indirect_draw_gpu_entry.get_read(frame_index.index()).element_count;

    for i in 0..draw_count {
        render_pass.draw_indexed_indirect(
            indirect_draw_buffer,
            i as u64 * std::mem::size_of::<IndirectDraw>() as u64,
        );
        // info!("gpu frame_index drawn: {}, drawcount: {}, i: {}", frame_index.index(), draw_count, i);
    }
}

pub fn upload_camera_data(
    world: &mut World,
    frame_index: usize,
    staging_belt: &mut StagingBelt,
    device: &Device,
    encoder: &mut CommandEncoder,
    gpu_buffer_registry: &mut Registry<Box<dyn BufferInterface>>,
) {
    let camera_buffer_key =
        RegisterKey::from_label::<GpuRingBuffer<CameraUniform>>("camera_gpu_uniform_triple");
    let camera_ring_buffer = gpu_buffer_registry
        .get_mut(&camera_buffer_key)
        .unwrap()
        .as_mut_any()
        .downcast_mut::<GpuRingBuffer<CameraUniform>>()
        .unwrap();
    for (camera, pos, _) in world.query::<(&mut FpsCamera, &mut Position, &Camera)>() {
        let forward = Vec3::new(
            camera.yaw.cos() * camera.pitch.cos(),
            camera.pitch.sin(),
            camera.yaw.sin() * camera.pitch.cos(),
        )
        .normalize();

        let camera_uniform = CameraUniform {
            view: Mat4::look_to_rh(pos.0, forward, Vec3::Y).to_cols_array_2d(),
            projection: Mat4::perspective_rh(0.785, 16.0 / 9.0, 0.1, 1000.0).to_cols_array_2d(),
        };

        let camera_entry = camera_ring_buffer.get_write(frame_index);
        camera_entry.element_count = 1;

        let mut view_mut = staging_belt.write_buffer(
            encoder,
            &camera_entry.buffer,
            0,
            BufferSize::new(size_of::<CameraUniform>() as u64).unwrap(),
            device,
        );

        view_mut.copy_from_slice(bytemuck::bytes_of(&camera_uniform));
    }
}

pub fn upload_indirect_draw_commands(
    world: &mut World,
    frame_index: usize,
    staging_belt: &mut StagingBelt,
    device: &Device,
    encoder: &mut CommandEncoder,
    gpu_buffer_registry: &mut Registry<Box<dyn BufferInterface>>,
) {
    let first_instance_counter = 0;

    let mut batch: Vec<Transform> = Vec::new();
    let mut mesh_handle = MeshHandle {
        vertex_offset: 0,
        index_offset: 0,
        vertex_count: 0,
        index_count: 0,
    };

    for (_i, (transform, mesh)) in world.query::<(&Transform, &MeshHandle)>().enumerate() {
        batch.push(transform.clone());
        mesh_handle = mesh.clone();
    }

    let indirect_draw = IndirectDraw {
        index_count: mesh_handle.index_count,
        instance_count: batch.len() as u32,
        first_index: mesh_handle.index_offset as u32,
        base_vertex: mesh_handle.vertex_offset as i32,
        first_instance: first_instance_counter,
        ..Default::default()
    };

    // indirect_draws.iter().for_each(|x| info!("{:?}", x));

    let mut model_matrices: Vec<ModelUniform> = Vec::new();
    batch.iter().for_each(|x| {
        model_matrices.push(ModelUniform {
            model: x.0.to_cols_array_2d(),
        });
    });

    let indirect_draw_buffer_key =
        RegisterKey::from_label::<GpuRingBuffer<IndirectDraw>>("indirect_draw_buffer");
    let indirect_draw_buffer = gpu_buffer_registry
        .get_mut(&indirect_draw_buffer_key)
        .unwrap()
        .as_mut_any()
        .downcast_mut::<GpuRingBuffer<IndirectDraw>>()
        .unwrap();

    let indirect_entry = indirect_draw_buffer.get_write(frame_index);
    indirect_entry.element_count = 1;

    let mut indirect_draw_view_mut = staging_belt.write_buffer(
        encoder,
        &indirect_entry.buffer,
        0,
        BufferSize::new(size_of::<IndirectDraw>() as u64).unwrap(),
        device,
    );
    indirect_draw_view_mut.copy_from_slice(bytemuck::bytes_of(&indirect_draw));
    std::mem::drop(indirect_draw_view_mut);

    let model_buffer_key =
        RegisterKey::from_label::<GpuRingBuffer<ModelUniform>>("model_gpu_uniform_triple");
    let model_buffer = gpu_buffer_registry
        .get_mut(&model_buffer_key)
        .unwrap()
        .as_mut_any()
        .downcast_mut::<GpuRingBuffer<ModelUniform>>()
        .unwrap();

    let model_entry = model_buffer.get_write(frame_index);
    model_entry.element_count = model_matrices.len() as u32;

    let model_matrices_bytes = bytemuck::cast_slice(&model_matrices);
    let total_model_matrices_size = BufferSize::new(model_matrices_bytes.len() as u64).unwrap();
    let mut model_matrices_view_mut = staging_belt.write_buffer(
        encoder,
        &model_entry.buffer,
        0,
        total_model_matrices_size,
        device,
    );
    model_matrices_view_mut.copy_from_slice(model_matrices_bytes);
}
