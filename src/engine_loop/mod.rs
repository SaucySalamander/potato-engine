use std::{
    time::{Duration, Instant},
};

use bytemuck::Pod;
use glam::{Mat4, Vec2};
use log::debug;
use wgpu::Queue;

use crate::{
    engine::{
        r#async::FrameIndex,
        buffers::CpuRingBuffer,
        cameras::{CameraUniform, fps_camera::FpsCamera},
        graphics::buffers::{BufferInterface, BufferRegistry, GpuRingBuffer},
        model::ModelUniform,
    },
    engine_loop::input::InputState,
    utils::Registry,
};

pub mod input;

pub struct EngineLoop {
    pub last_time: Instant,
    pub accumulator: Duration,
    pub delta_time: Duration,
    pub fps_camera: Option<FpsCamera>,
    pub sim_frame_index: FrameIndex,
    pub last_cusor_pos: Vec2,
    pub input_state: InputState,
    pub cpu_buffer_registry: Option<BufferRegistry<Box<dyn BufferInterface>>>,
}

impl Default for EngineLoop {
    fn default() -> Self {
        Self {
            last_time: Instant::now(),
            accumulator: Duration::ZERO,
            delta_time: Duration::from_secs_f64(1.0 / 240.0),
            input_state: InputState::default(),
            sim_frame_index: FrameIndex::new(3),
            fps_camera: None,
            last_cusor_pos: Vec2::default(),
            cpu_buffer_registry: None,
        }
    }
}

impl EngineLoop {
    pub fn update_logic(&mut self) {
        self.fps_camera
            .as_mut()
            .expect("fps camera must exist")
            .update(&self.input_state, self.delta_time.as_secs_f32());

        debug!(
            "internal camera debug log {:?}",
            self.fps_camera.as_ref().unwrap()
        );

        self.input_state.mouse_delta_x = 0.0;
        self.input_state.mouse_delta_y = 0.0;

        match self.cpu_buffer_registry.as_mut().unwrap() {
            cpu_buffer_registry => {
                let camera_buffer_entry = cpu_buffer_registry
                            .get_mut(&String::from("camera_cpu_uniform_triple"))
                            .unwrap();
                let camera_uniform_triple = camera_buffer_entry
                            .as_mut_any()
                            .downcast_mut::<CpuRingBuffer<CameraUniform>>()
                            .unwrap();
                let camera_uniform = camera_uniform_triple.get_write(self.sim_frame_index.index());
                camera_uniform.view = self
                            .fps_camera
                            .as_ref()
                            .unwrap()
                            .view_matrix()
                            .to_cols_array_2d();
                camera_uniform.projection =
                            Mat4::perspective_rh(0.785, 16.0 / 9.0, 0.1, 100.0).to_cols_array_2d();
                debug!("internal camera buffer debug log {:?}", camera_uniform);
                let model_buffer_entry = cpu_buffer_registry
                            .get_mut(&String::from("model_cpu_uniform_triple"))
                            .unwrap();
                let model_uniform_triple = model_buffer_entry
                            .as_mut_any()
                            .downcast_mut::<CpuRingBuffer<ModelUniform>>()
                            .unwrap();
                let model_uniform = model_uniform_triple.get_write(self.sim_frame_index.index());
                model_uniform.model = Mat4::IDENTITY.to_cols_array_2d();
            }
            _ => (),
        }

        debug!("cpu frame_index writen: {}", self.sim_frame_index.index());
    }

    pub fn sync_buffers(
        &mut self,
        gpu_uniform_triple_buffers: &mut BufferRegistry<Box<dyn BufferInterface>>,
        gpu_frame_index: usize,
        queue: &Queue,
    ) {
        let cpu_buffer_registry = self.cpu_buffer_registry.as_ref().unwrap();
        let gpu_buffer_registry = gpu_uniform_triple_buffers;
        let sim_frame_index = self.sim_frame_index.index();

        //Camera buffer
        Self::sync_buffer::<CameraUniform>(
            String::from("camera_cpu_uniform_triple"),
            cpu_buffer_registry,
            String::from("camera_gpu_uniform_triple"),
            gpu_buffer_registry,
            queue,
            gpu_frame_index,
            sim_frame_index,
        );

        //Model buffer
        Self::sync_buffer::<ModelUniform>(
            String::from("model_cpu_uniform_triple"),
            cpu_buffer_registry,
            String::from("model_gpu_uniform_triple"),
            gpu_buffer_registry,
            queue,
            gpu_frame_index,
            sim_frame_index,
        );

        //interpolation which I have to figure out a scalable solution for all my buffers
        // let cpu_buffer_previous =
        //     cpu_uniform_triple.get_read((self.sim_frame_index.index() + 3 - 2) % 3);
        // let cpu_buffer_current =
        //     cpu_uniform_triple.get_read((self.sim_frame_index.index() + 3 - 1) % 3);

        // let alpha = self.accumulator.as_secs_f32() / self.delta_time.as_secs_f32();

        debug!(
            "synced cpu_frame_index: {}, gpu_frame_index: {}",
            (self.sim_frame_index.index() + 3 - 1) % 3,
            gpu_frame_index
        );
    }

    fn sync_buffer<T: Pod>(
        cpu_key: String,
        cpu_buffer_registry: &BufferRegistry<Box<dyn BufferInterface>>,
        gpu_key: String,
        gpu_buffer_registry: &mut BufferRegistry<Box<dyn BufferInterface>>,
        queue: &Queue,
        gpu_frame_index: usize,
        sim_frame_index: usize,
    ) {
        let cpu_buffer_entry = cpu_buffer_registry.get(&cpu_key).unwrap();
        let cpu_uniform_triple = cpu_buffer_entry
            .as_any()
            .downcast_ref::<CpuRingBuffer<T>>()
            .unwrap();
        let gpu_buffer_entry = gpu_buffer_registry.get_mut(&gpu_key).unwrap();
        let gpu_uniform_triple = gpu_buffer_entry
            .as_mut_any()
            .downcast_mut::<GpuRingBuffer<T>>()
            .unwrap();
        let data = cpu_uniform_triple.get_read((sim_frame_index + 3 - 1) % 3);
        gpu_uniform_triple.write(queue, data, gpu_frame_index);
    }
}
