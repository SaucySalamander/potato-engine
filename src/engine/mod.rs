use std::{mem::transmute, process, sync::Arc, time::Instant};

use glam::Vec3;
use log::{debug, error, info};
use wgpu::{
    Color, DepthBiasState, DepthStencilState, FragmentState, Instance, MultisampleState,
    PipelineLayoutDescriptor, PrimitiveState, RenderPipeline, RenderPipelineDescriptor,
    ShaderModule, StencilState, Surface, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexState,
};
use winit::{
    application::ApplicationHandler,
    event::ElementState,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

use crate::{
    engine::{
        r#async::{FrameIndex, ThreadPool},
        buffers::CpuRingBuffer,
        cameras::{CameraUniform, fps_camera::FpsCamera},
        draw_queue::DrawQueue,
        graphics::buffers::GpuRingBuffer,
        mesh::{
            Vertex,
            mesh_allocator::{MeshAllocator, MeshHandle},
        },
        model::ModelUniform,
    },
    engine_loop::{self, EngineLoop},
    utils::{FPSCounter, Registry},
};
use graphics::{
    GPUContext,
    bindgroups::{BindGroupLayoutRegistry, BindGroupRegistry},
    buffers::{BufferInterface, BufferRegistry},
    init_render_pass,
    shaders::load_shader,
    viewports::{Viewport, ViewportDescription},
};

pub(crate) mod r#async;
pub(crate) mod buffers;
pub mod cameras;
mod draw_queue;
pub mod graphics;
mod mesh;
pub(crate) mod model;

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

pub const CUBE_INDICES: [u32; 36] = [
    0, 1, 2, 2, 3, 0, // Back
    4, 5, 6, 6, 7, 4, // Front
    0, 4, 7, 7, 3, 0, // Left
    1, 5, 6, 6, 2, 1, // Right
    3, 2, 6, 6, 7, 3, // Top
    0, 1, 5, 5, 4, 0, // Bottom
];
//

pub struct Engine {
    startup: bool,
    window: Option<Arc<Window>>,
    instance: Option<Arc<Instance>>,
    gpu_context: Option<Arc<GPUContext>>,
    viewports: Vec<Viewport>,
    render_pipeline: Option<RenderPipeline>,
    fps_counter: Option<FPSCounter>,
    frame_index: Option<FrameIndex>,
    engine_loop: Option<EngineLoop>,
    bind_group_layout_registry: Option<BindGroupLayoutRegistry>,
    gpu_buffer_registy: Option<BufferRegistry<Box<dyn BufferInterface>>>,
    mesh_allocator: Option<MeshAllocator>,
    mesh_handle: Option<MeshHandle>,
    draw_queue: Option<GpuRingBuffer<DrawQueue>>,
    thread_pool: Option<ThreadPool>,
    static_mesh_handles: Option<Vec<MeshHandle>>,
}

impl<'a> Default for Engine {
    fn default() -> Self {
        Engine {
            startup: true,
            window: None,
            instance: None,
            gpu_context: None,
            render_pipeline: None,
            frame_index: Some(FrameIndex::new(3)),
            fps_counter: None,
            engine_loop: None,
            bind_group_layout_registry: None,
            mesh_allocator: None,
            mesh_handle: None,
            gpu_buffer_registy: None,
            draw_queue: None,
            thread_pool: None,
            static_mesh_handles: None,
            viewports: Vec::new(),
        }
    }
}

impl Engine {
    fn init(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        info!("starting threadpool");
        self.thread_pool = Some(ThreadPool::new(4));
        event_loop.listen_device_events(winit::event_loop::DeviceEvents::Always);

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
        let queue = &gpu_context.queue;

        info!("creating bind group layout registry");
        self.bind_group_layout_registry = Some(BindGroupLayoutRegistry::default());

        info!("creating buffer reg");
        self.gpu_buffer_registy = Some(BufferRegistry::<Box<dyn BufferInterface>>::default());

        info!("creating uniform buffers");
        self.mesh_allocator = Some(MeshAllocator::new(device, 3000.0 as u64, 3000.0 as u64));

        let camera_uniform = CameraUniform::default();
        info!("{:?}", camera_uniform);
        let _ = camera_uniform
            .create_and_store_buffers(
                device,
                queue,
                self.bind_group_layout_registry
                    .as_mut()
                    .expect("bind group layout registry should exist"),
                self.gpu_buffer_registy
                    .as_mut()
                    .expect("buffer registry should exist"),
                0,
            )
            .unwrap_or_else(|err| {
                error!("failed to init camera buffer {err}");
                process::exit(1)
            });

        let model_uniform = ModelUniform::default();
        let _ = model_uniform.create_and_store_buffers(
            device,
            queue,
            self.bind_group_layout_registry
                .as_mut()
                .expect("bind group layout registry should exist"),
            self.gpu_buffer_registy
                .as_mut()
                .expect("buffer registry should exist"),
            0,
        );
    }

    fn create_main_viewport(&mut self) {
        let surface = self
            .instance
            .as_ref()
            .expect("instance must exist")
            .create_surface(self.window.as_ref().unwrap().clone())
            .map_err(|err| {
                error!("failed to create surface {err}");
                std::process::exit(1);
            })
            .map(|surface| unsafe { transmute::<Surface<'_>, Surface<'static>>(surface) });

        let surface = surface.unwrap();

        info!("creating main viewport");
        let viewport_description: ViewportDescription = ViewportDescription::new(
            self.window.as_ref().expect("window should exist").clone(),
            Color::BLACK,
            surface,
        );

        let gpu_context = Arc::new(GPUContext::init(
            self.instance.as_ref().expect("instance must exist"),
            &viewport_description.surface,
        ));

        self.gpu_context = Some(gpu_context.clone());

        let viewport = viewport_description
            .build_viewport(self.gpu_context.as_ref().expect("gpu context should exist"));

        self.viewports.push(viewport);
    }

    fn start_engine_loop(&mut self) {
        info!("init engine_loop");
        self.engine_loop = Some(EngineLoop::default());
        self.engine_loop.as_mut().unwrap().cpu_buffer_registry =
            Some(BufferRegistry::<Box<dyn BufferInterface>>::default());
        let cpu_buffer_registry = self
            .engine_loop
            .as_mut()
            .unwrap()
            .cpu_buffer_registry
            .as_mut()
            .unwrap();
        cpu_buffer_registry.insert(
            String::from("camera_cpu_uniform_triple"),
            Box::new(CpuRingBuffer::<CameraUniform>::new(CameraUniform::default())),
        );
        cpu_buffer_registry.insert(
            String::from("model_cpu_uniform_triple"),
            Box::new(CpuRingBuffer::<ModelUniform>::new(ModelUniform::default())),
        );

        let vertices: Vec<Vertex> = CUBE_VERTICES
            .iter()
            .map(|v| Vertex {
                position: v.to_array(),
            })
            .collect();
        self.static_mesh_handles = self.mesh_allocator.as_mut().unwrap().upload_static_mesh(
            &self.gpu_context.as_ref().unwrap().queue,
            &vertices,
            &CUBE_INDICES,
        );

        info!("create fps camera");
        self.engine_loop.as_mut().unwrap().fps_camera = Some(FpsCamera::new(Vec3 {
            x: 0.0,
            y: 0.0,
            z: 5.0,
        }));
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
            depth_stencil: Some(DepthStencilState {
                format: self
                    .viewports
                    .get(0)
                    .unwrap()
                    .description
                    .depth
                    .as_ref()
                    .unwrap()
                    .format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        };
        self.render_pipeline = Some(device.create_render_pipeline(render_pipeline_descriptor));
    }
}

impl ApplicationHandler for Engine {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.startup {
            self.init(event_loop);

            info!("creating fps counter");
            self.fps_counter = Some(FPSCounter::default());

            self.startup = false;
        }

        self.window.as_ref().unwrap().set_cursor_visible(false);
        let _ = self
            .window
            .as_ref()
            .unwrap()
            .set_cursor_grab(winit::window::CursorGrabMode::Locked);
        info!("requesting first redraw");
        self.window
            .as_ref()
            .expect("window must exist")
            .request_redraw();
    }

    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _ = self
            .window
            .as_ref()
            .unwrap()
            .set_cursor_grab(winit::window::CursorGrabMode::None);
        self.window.as_ref().unwrap().set_cursor_visible(true);
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        debug!("processing event {:?}", event);
        match event {
            winit::event::WindowEvent::Resized(physical_size) => {
                let window = self.window.as_ref().expect("window must exist");
                let viewport = self.viewports.get_mut(0).expect("viewport must exist");
                let device = &self.gpu_context.as_ref().expect("device must exist").device;

                let mut config = viewport.config.clone();

                config.width = physical_size.width;
                config.height = physical_size.height;

                viewport.description.surface.configure(device, &config);
                viewport.description.create_depth_resources(device, &config);

                window.request_redraw();
            }
            winit::event::WindowEvent::CloseRequested => {
                info!("Close request processing");
                event_loop.exit();
            }
            winit::event::WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => {
                let pressed = event.state == ElementState::Pressed;
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyW) => {
                        self.engine_loop.as_mut().unwrap().input_state.key_w = pressed
                    }
                    PhysicalKey::Code(KeyCode::KeyA) => {
                        self.engine_loop.as_mut().unwrap().input_state.key_a = pressed
                    }
                    PhysicalKey::Code(KeyCode::KeyD) => {
                        self.engine_loop.as_mut().unwrap().input_state.key_d = pressed
                    }
                    PhysicalKey::Code(KeyCode::KeyS) => {
                        self.engine_loop.as_mut().unwrap().input_state.key_s = pressed
                    }
                    PhysicalKey::Code(KeyCode::Space) => {
                        self.engine_loop.as_mut().unwrap().input_state.key_space = pressed
                    }
                    PhysicalKey::Code(KeyCode::ControlLeft) => {
                        self.engine_loop.as_mut().unwrap().input_state.key_ctrl = pressed
                    }
                    _ => {}
                }
            }
            winit::event::WindowEvent::RedrawRequested => {
                let viewport = self.viewports.get(0).expect("viewport must exist");
                let descriptor = &viewport.description;
                let render_pipeline = self
                    .render_pipeline
                    .as_ref()
                    .expect("render pipeline must exist");
                self.engine_loop.as_mut().unwrap().sync_buffers(
                    self.gpu_buffer_registy.as_mut().unwrap(),
                    self.frame_index.as_ref().unwrap().index(),
                    &self
                        .gpu_context
                        .as_ref()
                        .expect("gpu context must exist")
                        .queue,
                );

                descriptor.window.pre_present_notify();
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
                    self.gpu_buffer_registy
                        .as_mut()
                        .expect("gpu buffer registry should exist"),
                    self.frame_index.as_mut().unwrap(),
                    self.mesh_allocator.as_mut().unwrap(),
                    self.static_mesh_handles.as_ref(),
                    self.mesh_handle.as_ref(),
                );

                let _ = self
                    .gpu_context
                    .as_ref()
                    .expect("gpu_context should exist")
                    .queue
                    .submit(Some(encoder.finish()));

                output.present();

                self.frame_index.as_mut().unwrap().advance();
                self.fps_counter
                    .as_mut()
                    .expect("fps counter must exist")
                    .tick();
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        match event {
            winit::event::DeviceEvent::MouseMotion { delta: (dx, dy) } => {
                let mut input = &mut self.engine_loop.as_mut().unwrap().input_state;
                input.mouse_delta_x += dx as f32;
                input.mouse_delta_y += dy as f32;
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(window) = &self.window {
            if let Some(engine_loop) = &mut self.engine_loop {
                let now = Instant::now();
                let frame_time = now - engine_loop.last_time;
                engine_loop.last_time = now;
                engine_loop.accumulator += frame_time;

                while engine_loop.accumulator >= engine_loop.delta_time {
                    engine_loop.update_logic();

                    engine_loop.sim_frame_index.advance();
                    engine_loop.accumulator -= engine_loop.delta_time;
                }

                window.request_redraw();

                debug!("configuring control flow");
                let next_logic_update = now + (engine_loop.delta_time - engine_loop.accumulator);
                event_loop
                    .set_control_flow(winit::event_loop::ControlFlow::WaitUntil(next_logic_update));
            }
        }
    }
}
