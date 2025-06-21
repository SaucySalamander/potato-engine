use std::process;

use log::{debug, error, info};
use pollster::FutureExt;
use wgpu::{
    Adapter, CommandEncoder, Device, DeviceDescriptor, Features, Instance, Limits, Operations,
    Queue, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, RequestAdapterOptions, Surface, TextureView, Trace,
};

use crate::{
    engine::{
        r#async::FrameIndex,
        cameras::CameraUniform,
        graphics::{
            bindgroups::BindGroupRegistry,
            buffers::{BufferInterface, BufferRegistry, GpuRingBuffer},
            viewports::ViewportDescription,
        },
        mesh::mesh_allocator::{MeshAllocator, MeshHandle},
        model::ModelUniform,
    },
    utils::Registry,
};

pub mod bindgroups;
pub mod buffers;
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
    gpu_buffer_registry: &mut BufferRegistry<Box<dyn BufferInterface>>,
    frame_index: &mut FrameIndex,
    mesh_allocator: &mut MeshAllocator,
    static_mesh_handles: Option<&Vec<MeshHandle>>,
    mesh_handle: Option<&MeshHandle>,
) {
    let static_mesh_handles = static_mesh_handles.unwrap();
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

    let camera_uniform_buffer_entry = gpu_buffer_registry
        .get(&String::from("camera_gpu_uniform_triple"))
        .unwrap()
        .as_any()
        .downcast_ref::<GpuRingBuffer<CameraUniform>>()
        .unwrap();

    let camera_bind_group = camera_uniform_buffer_entry
        .get_read(frame_index.index())
        .bind_group
        .as_ref()
        .unwrap();

    let model_uniform_buffer_entry = gpu_buffer_registry
        .get(&String::from("model_gpu_uniform_triple"))
        .unwrap()
        .as_any()
        .downcast_ref::<GpuRingBuffer<ModelUniform>>()
        .unwrap();

    let model_bind_group = model_uniform_buffer_entry
        .get_read(frame_index.index())
        .bind_group
        .as_ref()
        .unwrap();

    render_pass.set_bind_group(0, Some(camera_bind_group), &[]);
    render_pass.set_bind_group(1, Some(model_bind_group), &[]);

    let static_mesh_handle = static_mesh_handles.get(frame_index.index()).unwrap();

    render_pass.set_vertex_buffer(
        0,
        mesh_allocator
            .get_curret_vertex_buffer(frame_index.index())
            .slice(static_mesh_handle.vertex_offset..),
    );
    render_pass.set_index_buffer(
        mesh_allocator
            .get_current_index_buffer(frame_index.index())
            .slice(static_mesh_handle.index_offset..),
        wgpu::IndexFormat::Uint32,
    );

    render_pass.draw_indexed(0..static_mesh_handle.index_count, 0, 0..1);
    debug!(
        "drawing mesh with handle {:?} from index {}",
        static_mesh_handle,
        frame_index.index()
    );
    debug!("gpu frame_index drawn: {}", frame_index.index());
}
