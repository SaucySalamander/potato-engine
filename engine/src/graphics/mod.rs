use std::process;

use log::{error, info};
use pollster::FutureExt;
use wgpu::{
    Adapter, CommandEncoder, Device, DeviceDescriptor, Features, Instance, Limits, Operations,
    Queue, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, RequestAdapterOptions, Surface, TextureView, Trace,
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
    draw_count: usize,
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

    for i in 0..draw_count {
        render_pass.draw_indexed_indirect(
            indirect_draw_buffer,
            (i * std::mem::size_of::<IndirectDraw>()) as u64,
        );
        // info!("gpu frame_index drawn: {}, drawcount: {}, i: {}", frame_index.index(), draw_count, i);
    }
}
