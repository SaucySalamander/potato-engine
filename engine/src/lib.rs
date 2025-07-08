use std::{
    mem::transmute,
    process,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use glam::{Mat4, Vec3};
use log::{debug, error, info};
use wgpu::{
    BindGroupLayout, Color, DepthBiasState, DepthStencilState, FragmentState, Instance,
    MultisampleState, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, StencilState, Surface, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState,
};
use winit::{
    application::ApplicationHandler,
    event::ElementState,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

use crate::{
    r#async::FrameIndex,
    graphics::buffers::{
        BufferInterface,
        submissions::{CameraUniform, IndirectDraw, ModelUniform},
        sync_buffers,
    },
    graphics::mesh::{Vertex, mesh_allocator::MeshAllocator},
    utils::{FPSCounter, RegisterKey, Registry, ThreadPool},
};
use ecs::{
    World,
    commands::IndirectDrawCommand,
    components::{self, Camera, FpsCamera, Position},
};
use graphics::{
    GPUContext, init_render_pass,
    shaders::load_shader,
    viewports::{Viewport, ViewportDescription},
};

pub(crate) mod r#async;
pub mod graphics;
pub mod input;
pub mod utils;

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

pub const CUBE_VERTICES_2: [Vec3; 8] = [
    Vec3::new(1.5, 1.5, 1.5),
    Vec3::new(2.5, 1.5, 1.5),
    Vec3::new(2.5, 2.5, 1.5),
    Vec3::new(1.5, 2.5, 1.5),
    Vec3::new(1.5, 1.5, 2.5),
    Vec3::new(2.5, 1.5, 2.5),
    Vec3::new(2.5, 2.5, 2.5),
    Vec3::new(1.5, 2.5, 2.5),
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
    thread_pool: Option<ThreadPool>,
    world: Arc<Mutex<World>>,
    window: Option<Arc<Window>>,
    instance: Option<Arc<Instance>>,
    gpu_context: Option<Arc<GPUContext>>,
    viewports: Vec<Viewport>,
    render_pipeline: Option<RenderPipeline>,
    fps_counter: Option<FPSCounter>,
    sim_frame_index: FrameIndex,
    frame_index: FrameIndex,
    bind_group_layout_registry: Option<Registry<BindGroupLayout>>,
    gpu_buffer_registry: Option<Registry<Box<dyn BufferInterface>>>,
    mesh_allocator: Option<MeshAllocator>,
    input_state: ecs::input::InputState,
    last_time: Instant,
    accumulator: Duration,
    delta_time: Duration,
}

impl<'a> Default for Engine {
    fn default() -> Self {
        Engine {
            startup: true,
            world: Arc::new(Mutex::new(World::new())),
            window: None,
            instance: None,
            gpu_context: None,
            render_pipeline: None,
            sim_frame_index: FrameIndex::new(3),
            frame_index: FrameIndex::new(3),
            fps_counter: None,
            bind_group_layout_registry: None,
            mesh_allocator: None,
            gpu_buffer_registry: None,
            thread_pool: None,
            viewports: Vec::new(),
            input_state: ecs::input::InputState::default(),
            last_time: Instant::now(),
            accumulator: Duration::ZERO,
            delta_time: Duration::from_secs_f64(1.0 / 240.0),
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

        Self::init_scene(
            &mut self.world.lock().unwrap(),
            self.mesh_allocator.as_mut().unwrap(),
            &self.gpu_context.as_ref().unwrap().queue,
        );
    }

    fn setup_buffers(&mut self) {
        let gpu_context = self.gpu_context.as_ref().expect("gpu context should exist");
        let device = &gpu_context.device;
        let queue = &gpu_context.queue;

        info!("creating bind group layout registry");
        self.bind_group_layout_registry = Some(Registry::<BindGroupLayout>::default());

        info!("creating gpu buffer registry");
        self.gpu_buffer_registry = Some(Registry::<Box<dyn BufferInterface>>::default());

        info!("creating buffer layouts");
        let camera_uniform = CameraUniform::default();
        let camera_bind_group_layout_key =
            RegisterKey::from_label::<BindGroupLayout>("camera_bind_group_layout");
        let camera_uniform_bind_group_layout = camera_uniform.create_bind_group_layout(device);

        let model_uniform = ModelUniform::default();
        let model_bind_group_layout_key =
            RegisterKey::from_label::<BindGroupLayout>("model_bind_group_layout");
        let model_uniform_bind_group_layout = model_uniform.create_bind_group_layout(device);

        let indirect_draw = IndirectDraw::default();
        let indirect_draw_bind_group_layout_key =
            RegisterKey::from_label::<BindGroupLayout>("indirect_draw_bind_group_layout");
        let indirect_draw_bind_group_layout = indirect_draw.create_bind_group_layout(device);

        info!("creating uniform buffers");
        self.mesh_allocator = Some(MeshAllocator::new(device, 3000.0 as u64, 3000.0 as u64));

        info!("{:?}", camera_uniform);
        let _ = camera_uniform
            .create_and_store_buffers(
                device,
                queue,
                &camera_uniform_bind_group_layout,
                self.gpu_buffer_registry
                    .as_mut()
                    .expect("buffer registry should exist"),
                0,
            )
            .unwrap_or_else(|err| {
                error!("failed to init camera buffer {err}");
                process::exit(1)
            });

        info!("{:?}", model_uniform);
        let _ = model_uniform.create_and_store_buffers(
            device,
            queue,
            &model_uniform_bind_group_layout,
            self.gpu_buffer_registry
                .as_mut()
                .expect("buffer registry should exist"),
            0,
        );

        info!("creating other buffers");
        info!("{:?}", indirect_draw);
        let _ = indirect_draw.create_and_store_buffers(
            device,
            queue,
            &indirect_draw_bind_group_layout,
            self.gpu_buffer_registry
                .as_mut()
                .expect("buffer registry should exist"),
            0,
        );

        let bind_group_layout_registry = self.bind_group_layout_registry.as_mut().unwrap();
        bind_group_layout_registry.register_key(
            camera_bind_group_layout_key,
            camera_uniform_bind_group_layout,
        );
        bind_group_layout_registry
            .register_key(model_bind_group_layout_key, model_uniform_bind_group_layout);
        bind_group_layout_registry.register_key(
            indirect_draw_bind_group_layout_key,
            indirect_draw_bind_group_layout,
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

    fn init_scene(world: &mut World, mesh_allocator: &mut MeshAllocator, queue: &Queue) {
        world.spawn((
            Camera,
            FpsCamera {
                yaw: 0.0,
                pitch: 0.0,
                speed: 5.0,
                sensitivity: 0.002,
            },
            Position(Vec3::new(0.0, 0.0, 0.0)),
        ));

        let vertices: Vec<Vertex> = CUBE_VERTICES
            .iter()
            .map(|v| Vertex {
                position: v.to_array(),
            })
            .collect();

        let static_mesh_handles = mesh_allocator
            .upload_static_mesh(queue, &vertices, &CUBE_INDICES)
            .unwrap();

        for i in (0..30).step_by(2) {
            for j in (0..10).step_by(2) {
                for k in (0..20).step_by(2) {
                    world.spawn((
                        components::Transform(Mat4::from_translation(Vec3 {
                            x: i as f32,
                            y: j as f32,
                            z: k as f32,
                        })),
                        static_mesh_handles[0],
                    ));
                }
            }
        }
    }

    fn load_shaders(&mut self) -> ShaderModule {
        info!("loading shaders");
        let gpu_context = self.gpu_context.as_ref().expect("gpu context should exist");
        let device = &gpu_context.device;
        let shader_name = String::from(
            "/home/sevenofnine/Git/potato-engine/engine/src/graphics/shaders/shader.wgsl",
        );
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
        let camera_bind_group_layout_key =
            RegisterKey::from_label::<BindGroupLayout>("camera_bind_group_layout");
        let camera_bind_group_layout = bind_group_layout_registry
            .get(&camera_bind_group_layout_key)
            .unwrap();
        let model_bind_group_layout_key =
            RegisterKey::from_label::<BindGroupLayout>("model_bind_group_layout");
        let model_bind_group_layout = bind_group_layout_registry
            .get(&model_bind_group_layout_key)
            .unwrap();
        let indirect_draw_bind_group_layout_key =
            RegisterKey::from_label::<BindGroupLayout>("indirect_draw_bind_group_layout");
        let indirect_draw_bind_group_layout = bind_group_layout_registry
            .get(&indirect_draw_bind_group_layout_key)
            .unwrap();

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("simple pipeline layout"),
            bind_group_layouts: &[
                &camera_bind_group_layout,
                &model_bind_group_layout,
                &indirect_draw_bind_group_layout,
            ],
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
                    PhysicalKey::Code(KeyCode::KeyW) => self.input_state.key_w = pressed,
                    PhysicalKey::Code(KeyCode::KeyA) => self.input_state.key_a = pressed,
                    PhysicalKey::Code(KeyCode::KeyD) => self.input_state.key_d = pressed,
                    PhysicalKey::Code(KeyCode::KeyS) => self.input_state.key_s = pressed,
                    PhysicalKey::Code(KeyCode::Space) => self.input_state.key_space = pressed,
                    PhysicalKey::Code(KeyCode::ControlLeft) => self.input_state.key_ctrl = pressed,
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
                sync_buffers(
                    self.cpu_buffer_registry.as_mut().unwrap(),
                    self.gpu_buffer_registry.as_mut().unwrap(),
                    self.sim_frame_index.index(),
                    self.frame_index.index(),
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
                    self.gpu_buffer_registry
                        .as_mut()
                        .expect("gpu buffer registry should exist"),
                    &mut self.frame_index,
                    self.mesh_allocator.as_mut().unwrap(),
                    draw_count,
                );

                let _ = self
                    .gpu_context
                    .as_ref()
                    .expect("gpu_context should exist")
                    .queue
                    .submit(Some(encoder.finish()));

                output.present();

                self.frame_index.advance();
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
                let mut input = &mut self.input_state;
                input.mouse_delta_x += dx as f32;
                input.mouse_delta_y += dy as f32;
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(window) = &self.window {
            let now = Instant::now();
            let frame_time = now - self.last_time;
            self.last_time = now;
            self.accumulator += frame_time;

            while self.accumulator >= self.delta_time {
                let world = self.world.clone();
                let frame_index = self.frame_index.index();
                let input_state = self.input_state.clone();
                debug!("{:?}", input_state);
                let delta_time = self.delta_time;
                self.thread_pool.as_ref().unwrap().submit(move || {
                    let mut world = world.lock().unwrap();
                    world.run_systems(
                        frame_index,
                        &input_state,
                        delta_time.as_secs_f32(),
                    );
                });

                self.input_state.mouse_delta_x = 0.0;
                self.input_state.mouse_delta_y = 0.0;

                self.sim_frame_index.advance();
                self.accumulator -= self.delta_time;
            }

            window.request_redraw();

            let next_logic_update = now + (self.delta_time - self.accumulator);
            event_loop
                .set_control_flow(winit::event_loop::ControlFlow::WaitUntil(next_logic_update));
        }
    }
}
