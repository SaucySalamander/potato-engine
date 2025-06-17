use std::{
    process,
    sync::Arc,
    time::{Duration, Instant},
};

use glam::{Mat4, Vec3};
use log::{debug, error, info};
use wgpu::{
    Buffer, Color, FragmentState, Instance, MultisampleState, PipelineLayoutDescriptor,
    PrimitiveState, RenderPipeline, RenderPipelineDescriptor, ShaderModule, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, util::DeviceExt,
};
use winit::{
    application::ApplicationHandler,
    window::{Window, WindowAttributes},
};

use crate::{
    engine::{camera::CameraUniform, mesh::Vertex, model::ModelUniform},
    graphics::{
        GPUContext,
        bindgroups::{BindGroupLayoutRegistry, BindGroupRegistry},
        init_render_pass,
        shaders::load_shader,
        viewports::{Viewport, ViewportDescription},
    },
    utils::{FPSCounter, Registry},
};

mod camera;
mod mesh;
mod model;

//TODO move to the ecs
pub const CUBE_VERTICES: [Vec3; 8] = [
    Vec3::new(-0.5, -0.5, -0.5),
    Vec3::new(0.5, -0.5, -0.5),
    Vec3::new(0.5, 0.5, -0.5),
    Vec3::new(-0.5, 0.5, -0.5),
    Vec3::new(-0.5, -0.5, 0.5),
    Vec3::new(0.5, -0.5, 0.5),
    Vec3::new(0.5, 0.5, 0.5),
    Vec3::new(-0.5, 0.5, 0.5),
];

pub const CUBE_INDICES: [u16; 36] = [
    0, 1, 2, 2, 3, 0, // Back
    4, 5, 6, 6, 7, 4, // Front
    0, 4, 7, 7, 3, 0, // Left
    1, 5, 6, 6, 2, 1, // Right
    3, 2, 6, 6, 7, 3, // Top
    0, 1, 5, 5, 4, 0, // Bottom
];
//

#[derive(Debug)]
pub struct Engine<'a> {
    startup: bool,
    window: Option<Arc<Window>>,
    instance: Option<Arc<Instance>>,
    gpu_context: Option<Arc<GPUContext>>,
    viewports: Vec<Viewport<'a>>,
    render_pipeline: Option<RenderPipeline>,
    fps_counter: Option<FPSCounter>,
    engine_loop: Option<EngineLoop>,
    bind_group_layout_registry: Option<BindGroupLayoutRegistry>,
    bind_group_registry: Option<BindGroupRegistry>,
    vertex_buffer: Option<Buffer>,
    index_buffer: Option<Buffer>,
}
impl<'a> Default for Engine<'a> {
    fn default() -> Self {
        Engine {
            startup: true,
            window: None,
            instance: None,
            gpu_context: None,
            render_pipeline: None,
            fps_counter: None,
            engine_loop: None,
            bind_group_layout_registry: None,
            bind_group_registry: None,
            vertex_buffer: None,
            index_buffer: None,
            viewports: Vec::new(),
        }
    }
}

impl<'a> Engine<'a> {
    fn init(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        info!("configuring control flow");
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

        info!("creating instance");
        self.instance = Some(Arc::new(Instance::default()));

        info!("creating window");
        self.window = match event_loop.create_window(WindowAttributes::default()) {
            Ok(window) => Some(Arc::new(window)),
            Err(err) => {
                error!("Failed to create window. {:?}", err);
                process::exit(1);
            }
        };

        self.create_main_viewport();

        let shader = &self.load_shaders();

        self.setup_buffers();

        self.create_render_pipeline(shader);

        self.start_engine_loop();
    }

    fn setup_buffers(&mut self) {
        let gpu_context = self.gpu_context.as_ref().expect("gpu context should exist");
        let device = &gpu_context.device;

        info!("creating bind group layout registry");
        self.bind_group_layout_registry = Some(BindGroupLayoutRegistry::default());

        info!("creating bind group registry");
        self.bind_group_registry = Some(BindGroupRegistry::default());

        info!("creating uniform buffers");
        //TODO clean up and group into files/functions properly
        //------------------------
        // Convert to Vertex structs
        let vertices: Vec<Vertex> = CUBE_VERTICES
            .iter()
            .map(|v| Vertex {
                position: v.to_array(),
            })
            .collect();

        // Upload vertex buffer
        self.vertex_buffer = Some(gpu_context.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Cube Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            },
        ));

        // Upload index buffer
        self.index_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cube Index Buffer"),
                contents: bytemuck::cast_slice(&CUBE_INDICES),
                usage: wgpu::BufferUsages::INDEX,
            }),
        );

        let camera_uniform = CameraUniform::default();
        let _ = camera_uniform.create_and_store_camera_uniform_bindings(
            device,
            self.bind_group_layout_registry
                .as_mut()
                .expect("bind group layout registry should exist"),
            self.bind_group_registry
                .as_mut()
                .expect("bind group registry should exist"),
        );

        let model_uniform = ModelUniform::new(Mat4::IDENTITY);
        let _ = model_uniform.create_and_store_model_uniform_bindings(
            device,
            self.bind_group_layout_registry
                .as_mut()
                .expect("bind group layout registry should exist"),
            self.bind_group_registry
                .as_mut()
                .expect("bind group registry should exist"),
        );
        //------------------------
    }

    fn create_main_viewport(&mut self) {
        info!("creating main viewport");
        let viewport_description: Arc<ViewportDescription<'a>> =
            Arc::new(ViewportDescription::new(
                self.window.as_ref().expect("window should exist").clone(),
                Color::BLACK,
                self.instance
                    .as_ref()
                    .expect("instance should exist")
                    .clone(),
            ));
        let surface = &viewport_description.clone().surface;

        self.gpu_context = Some(Arc::new(GPUContext::init(
            self.instance.as_ref().expect("instance must exist"),
            surface,
        )));

        let viewport = ViewportDescription::build_viewport(
            viewport_description.clone(),
            self.gpu_context.as_ref().expect("gpu context should exist"),
        );

        self.viewports.push(viewport);
    }

    fn start_engine_loop(&mut self) {
        info!("init engine_loop");
        self.engine_loop = Some(EngineLoop::default());
    }

    fn load_shaders(&mut self) -> ShaderModule {
        info!("loading shaders");
        let gpu_context = self.gpu_context.as_ref().expect("gpu context should exist");
        let device = &gpu_context.device;
        let shader_name = String::from("./src/shaders/shader.wgsl");
        load_shader(device, shader_name)
    }

    fn create_render_pipeline(&mut self, shader: &ShaderModule) {
        let gpu_context = self.gpu_context.as_ref().expect("gpu context should exist");
        let device = &gpu_context.device;
        let adapter = &gpu_context.adapter;
        let surface = &self
            .viewports
            .get(0)
            .as_ref()
            .expect("viewport must exist")
            .description
            .surface;
        let bind_group_layout_registry = self
            .bind_group_layout_registry
            .as_ref()
            .expect("bind group layout registry must exist");

        info!("creating rendering pipeline");
        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: size_of::<[f32; 3]>() as wgpu::BufferAddress,
            attributes: &[VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: VertexFormat::Float32x3,
            }],
            step_mode: wgpu::VertexStepMode::Vertex,
        };

        let vertex = VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[vertex_buffer_layout],
        };
        let fragment = FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(surface.get_capabilities(&adapter).formats[0].into())],
        };
        let camera_bind_group_layout = bind_group_layout_registry
            .get(&String::from("camera_bind_group_layout"))
            .unwrap();
        let model_bind_group_layout = bind_group_layout_registry
            .get(&String::from("model_bind_group_layout"))
            .unwrap();

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("simple pipeline layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &model_bind_group_layout],
            push_constant_ranges: &[],
        });
        let render_pipeline_descriptor = &RenderPipelineDescriptor {
            label: Some("render pipeline descriptor"),
            layout: Some(&pipeline_layout),
            vertex,
            fragment: Some(fragment),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        };
        self.render_pipeline = Some(device.create_render_pipeline(render_pipeline_descriptor));
    }
}
#[derive(Debug)]
struct EngineLoop {
    last_time: Instant,
    accumulator: Duration,
    delta_time: Duration,
    previous_state: f32,
    current_state: f32,
}
impl Default for EngineLoop {
    fn default() -> Self {
        Self {
            last_time: Instant::now(),
            accumulator: Duration::ZERO,
            delta_time: Duration::from_secs_f64(1.0 / 60.0),
            previous_state: 0.0,
            current_state: 0.0,
        }
    }
}

impl EngineLoop {
    fn update(&mut self) {
        // Todo update state of game
        // self.previous_state = self.current_state;
        // self.current_state += 50.0 * self.delta_time.as_secs_f32();
    }
}

impl<'a> ApplicationHandler for Engine<'a> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.startup {
            self.init(event_loop);

            info!("creating fps counter");
            self.fps_counter = Some(FPSCounter::default());

            self.startup = false;
        }

        info!("requesting first redraw");
        self.window
            .as_ref()
            .expect("window must exist")
            .request_redraw();
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        if let Some(engine_loop) = &mut self.engine_loop {
            let now = Instant::now();
            let frame_time = now - engine_loop.last_time;
            engine_loop.last_time = now;

            engine_loop.accumulator += frame_time;

            while engine_loop.accumulator >= engine_loop.delta_time {
                engine_loop.update();
                engine_loop.accumulator -= engine_loop.delta_time;
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        debug!("processing event {:?}", event);
        match event {
            winit::event::WindowEvent::ActivationTokenDone {
                serial: _,
                token: _,
            } => {}
            winit::event::WindowEvent::Resized(physical_size) => {
                let window = self.window.as_ref().expect("window must exist");
                let viewport = self.viewports.get(0).expect("viewport must exist");
                let device = &self.gpu_context.as_ref().expect("device must exist").device;

                let mut config = viewport.config.clone();

                config.width = physical_size.width;
                config.height = physical_size.height;

                viewport.description.surface.configure(device, &config);

                window.request_redraw();
            }
            winit::event::WindowEvent::Moved(_physical_position) => {}
            winit::event::WindowEvent::CloseRequested => {
                info!("Close request processing");
                event_loop.exit();
            }
            winit::event::WindowEvent::Destroyed => {}
            winit::event::WindowEvent::DroppedFile(_path_buf) => {}
            winit::event::WindowEvent::HoveredFile(_path_buf) => {}
            winit::event::WindowEvent::HoveredFileCancelled => {}
            winit::event::WindowEvent::Focused(_) => {}
            winit::event::WindowEvent::KeyboardInput {
                device_id: _,
                event: _,
                is_synthetic: _,
            } => {}
            winit::event::WindowEvent::ModifiersChanged(_modifiers) => {}
            winit::event::WindowEvent::Ime(_ime) => {}
            winit::event::WindowEvent::CursorMoved {
                device_id: _,
                position: _,
            } => {}
            winit::event::WindowEvent::CursorEntered { device_id: _ } => {}
            winit::event::WindowEvent::CursorLeft { device_id: _ } => {}
            winit::event::WindowEvent::MouseWheel {
                device_id: _,
                delta: _,
                phase: _,
            } => {}
            winit::event::WindowEvent::MouseInput {
                device_id: _,
                state: _,
                button: _,
            } => {}
            winit::event::WindowEvent::PinchGesture {
                device_id: _,
                delta: _,
                phase: _,
            } => {}
            winit::event::WindowEvent::PanGesture {
                device_id: _,
                delta: _,
                phase: _,
            } => {}
            winit::event::WindowEvent::DoubleTapGesture { device_id: _ } => {}
            winit::event::WindowEvent::RotationGesture {
                device_id: _,
                delta: _,
                phase: _,
            } => {}
            winit::event::WindowEvent::TouchpadPressure {
                device_id: _,
                pressure: _,
                stage: _,
            } => {}
            winit::event::WindowEvent::AxisMotion {
                device_id: _,
                axis: _,
                value: _,
            } => {}
            winit::event::WindowEvent::Touch(_touch) => {}
            winit::event::WindowEvent::ScaleFactorChanged {
                scale_factor: _,
                inner_size_writer: _,
            } => {}
            winit::event::WindowEvent::ThemeChanged(_theme) => {}
            winit::event::WindowEvent::Occluded(_) => {}
            winit::event::WindowEvent::RedrawRequested => {
                // info!("redraw requested");
                let viewport = self.viewports.get(0).expect("viewport must exist");
                let descriptor = viewport.description.as_ref();
                let render_pipeline = self
                    .render_pipeline
                    .as_ref()
                    .expect("render pipeline must exist");
                let engine_loop = self.engine_loop.as_ref().expect("engine loop must exist");
                let bind_group_registry = self
                    .bind_group_registry
                    .as_ref()
                    .expect("bindgroups must exist");
                descriptor.window.pre_present_notify();

                let alpha =
                    engine_loop.accumulator.as_secs_f32() / engine_loop.delta_time.as_secs_f32();

                let output = descriptor.surface.get_current_texture().unwrap();

                let view = output.texture.create_view(&Default::default());

                let mut encoder = self
                    .gpu_context
                    .as_ref()
                    .expect("gpu_context should exist")
                    .device
                    .create_command_encoder(&Default::default());

                init_render_pass(
                    &mut encoder,
                    &view,
                    descriptor,
                    render_pipeline,
                    bind_group_registry,
                    self.vertex_buffer
                        .as_ref()
                        .expect("vertex buffer must be created"),
                    self.index_buffer.as_ref().expect("index buffer must exist"),
                    CUBE_INDICES,
                );

                let _ = self
                    .gpu_context
                    .as_ref()
                    .expect("gpu_context should exist")
                    .queue
                    .submit(Some(encoder.finish()));

                output.present();

                let interpolated =
                    engine_loop.previous_state * (1.0 - alpha) + engine_loop.current_state * alpha;

                self.fps_counter
                    .as_mut()
                    .expect("fps counter must exist")
                    .tick();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
