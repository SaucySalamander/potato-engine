use std::process;

use log::{error, info};
use pollster::FutureExt;
use wgpu::{
    Adapter, Buffer, CommandEncoder, Device, DeviceDescriptor, Features, Instance,
    Limits, Operations, Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RequestAdapterOptions, Surface, TextureView, Trace,
};

use crate::{
    graphics::{bindgroups::BindGroupRegistry, viewports::ViewportDescription},
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
    bind_group_registry: &BindGroupRegistry,
    //TODO refactor this
    vertex_buffer: &Buffer,
    index_buffer: &Buffer,
    cube_indices_var: [u16; 36],
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
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    };
    let mut render_pass = encoder.begin_render_pass(render_pass_descriptor);

    render_pass.set_pipeline(render_pipeline);

    render_pass.set_bind_group(
        0,
        bind_group_registry
            .get(&String::from("camera_bind_group"))
            .unwrap(),
        &[],
    );
    render_pass.set_bind_group(
        1,
        bind_group_registry
            .get(&String::from("model_bind_group"))
            .unwrap(),
        &[],
    );

    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);

    render_pass.draw_indexed(0..cube_indices_var.len() as u32, 0, 0..1);
}
