use std::{process, sync::Arc};

use log::{error, info};
use wgpu::{Color, Instance, Surface, SurfaceConfiguration};
use winit::window::Window;

use crate::graphics::GPUContext;

#[derive(Debug)]
pub struct Viewport<'a> {
    pub description: Arc<ViewportDescription<'a>>,
    pub config: SurfaceConfiguration,
}

#[derive(Debug)]

pub struct ViewportDescription<'a> {
    pub window: Arc<Window>,
    pub background: Color,
    pub surface: Surface<'a>,
}

impl<'a> ViewportDescription<'a> {
    pub fn new(window: Arc<Window>, background: Color, instance: Arc<Instance>) -> Self {
        let surface = {
            instance
                .create_surface(window.clone())
                .unwrap_or_else(|err| {
                    error!("failed to create surface. {}", err);
                    process::exit(1);
                })
        };

        Self {
            window: window.clone(),
            background,
            surface,
        }
    }

    pub fn build_viewport(
        viewport_description: Arc<Self>,
        gpu_context: &Arc<GPUContext>,
    ) -> Viewport<'a> {
        info!("building viewport");
        let adapter = &gpu_context.adapter;
        let device = &gpu_context.device;
        info!("getting size");
        let size = viewport_description.window.as_ref().inner_size();
        info!("checking size");

        if size.width == 0 || size.height == 0 {
            info!("Window size is zero; skipping surface configuration");
        } else {
            info!("size is not zero");
        }
        info!("getting surface config");

        let config = viewport_description
            .surface
            .get_default_config(adapter, size.width, size.height)
            .unwrap_or_else(|| {
                error!("failed to create surface config");
                process::exit(1);
            });
        info!("configuring surface");

        viewport_description.surface.configure(device, &config);
        info!("finished settingup viewport");
        Viewport {
            description: viewport_description.clone(),
            config: config,
        }
    }
}
