use std::sync::Arc;

use log::info;
use wgpu::{
    Color, Device, Extent3d, Surface, SurfaceConfiguration, Texture, TextureFormat, TextureUsages,
    TextureView, wgt::TextureDescriptor,
};
use winit::window::Window;

use crate::engine::graphics::GPUContext;

#[derive(Debug)]
pub struct Viewport {
    pub description: ViewportDescription,
    pub config: SurfaceConfiguration,
}

#[derive(Debug)]
pub struct DepthResources {
    pub texture: Texture,
    pub view: TextureView,
    pub format: TextureFormat,
}

#[derive(Debug)]
pub struct ViewportDescription {
    pub window: Arc<Window>,
    pub background: Color,
    pub surface: Surface<'static>,
    pub depth: Option<DepthResources>,
}

impl ViewportDescription {
    pub fn new(window: Arc<Window>, background: Color, surface: Surface<'static>) -> Self {
        Self {
            window: window.clone(),
            background,
            surface,
            depth: None,
        }
    }

    pub fn create_depth_resources(&mut self, device: &Device, config: &SurfaceConfiguration) {
        let size = Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };

        let format = TextureFormat::Depth32Float;

        let texture = device.create_texture(&TextureDescriptor {
            label: Some("depth texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let view = texture.create_view(&Default::default());

        self.depth = Some(DepthResources {
            texture,
            view,
            format,
        })
    }

    pub fn build_viewport(mut self, gpu_context: &Arc<GPUContext>) -> Viewport {
        info!("building viewport");
        let adapter = &gpu_context.adapter;
        let device = &gpu_context.device;
        info!("getting size");
        let size = self.window.as_ref().inner_size();
        info!("checking size");

        if size.width == 0 || size.height == 0 {
            info!("Window size is zero; skipping surface configuration");
        } else {
            info!("size is not zero");
        }
        info!("getting surface config");
        let format = self.surface.get_capabilities(adapter).formats[0];
        let config = SurfaceConfiguration {
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            view_formats: vec![],
            usage: TextureUsages::RENDER_ATTACHMENT,
            desired_maximum_frame_latency: 3,
        };
        info!("configuring surface");
        self.surface.configure(device, &config);
        self.create_depth_resources(device, &config);
        info!("finished settingup viewport");
        Viewport {
            description: self,
            config,
        }
    }
}
