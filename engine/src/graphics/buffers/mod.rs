use std::{
    any::Any,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use bytemuck::Pod;
use log::debug;
use wgpu::{
    BindGroup, Buffer, BufferDescriptor, BufferUsages, Device, Queue,
    util::{BufferInitDescriptor, DeviceExt},
};

pub mod bindgroups;
pub mod layouts;
pub mod submissions;

use crate::{
    graphics::buffers::submissions::{CameraUniform, IndirectDraw, ModelUniform},
    utils::{RegisterKey, Registry},
};
use ecs::{
    commands::IndirectDrawCommand,
    queues::{CpuRingQueue, QueueInterface},
};

pub trait BufferInterface: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
}

#[derive(Clone)]
pub struct BufferEntry {
    pub buffer: Buffer,
    pub bind_group: Option<BindGroup>,
}

pub struct GpuRingBuffer<T> {
    entries: [BufferEntry; 3],
    _phantom: PhantomData<T>,
}

impl BufferInterface for GpuRingBuffer<CameraUniform> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl BufferInterface for GpuRingBuffer<ModelUniform> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl BufferInterface for GpuRingBuffer<IndirectDraw> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl<T> GpuRingBuffer<T> {
    pub fn new(entries: Vec<BufferEntry>) -> Self {
        Self {
            entries: [
                entries.get(0).unwrap().clone(),
                entries.get(1).unwrap().clone(),
                entries.get(2).unwrap().clone(),
            ],
            _phantom: PhantomData,
        }
    }

    pub fn get_read(&self, frame_index: usize) -> &BufferEntry {
        &self.entries[frame_index % 3]
    }

    pub fn get_write(&mut self, frame_index: usize) -> &mut BufferEntry {
        &mut self.entries[frame_index % 3]
    }

    pub fn write(&mut self, queue: &Queue, data: &[u8], frame_index: usize) {
        let entry = self.get_write(frame_index);
        queue.write_buffer(&entry.buffer, 0, data);
    }
}

pub fn create_buffer(
    device: &Device,
    name: &str,
    size: u64,
    buffer_uses: Vec<BufferUsages>,
    mapped_at_creation: bool,
) -> Buffer {
    let combined_buffer_uses = buffer_uses
        .iter()
        .fold(BufferUsages::empty(), |acc, &uses| acc | uses);

    device.create_buffer(&BufferDescriptor {
        label: Some(name),
        size,
        usage: combined_buffer_uses,
        mapped_at_creation,
    })
}

pub fn _create_buffer_with_data(
    device: &Device,
    name: &str,
    data: &[u8],
    buffer_uses: Vec<BufferUsages>,
) -> Buffer {
    let combined_buffer_uses = buffer_uses
        .iter()
        .fold(BufferUsages::empty(), |acc, &uses| acc | uses);

    device.create_buffer_init(&BufferInitDescriptor {
        label: Some(name),
        contents: data,
        usage: combined_buffer_uses,
    })
}

pub fn sync_buffers(
    cpu_queue_registry: &mut Arc<Mutex<ecs::registries::Registry<Box<dyn QueueInterface + Send + Sync>>>>,
    gpu_buffer_registry: &mut Registry<Box<dyn BufferInterface>>,
    sim_frame_index: usize,
    gpu_frame_index: usize,
    queue: &Queue,
) {
    let camera_queue_key =
        ecs::registries::RegisterKey::from_label::<CpuRingQueue<ecs::cameras::CameraUniform>>("camera_cpu_uniform_triple");
    let camera_buffer_key =
        RegisterKey::from_label::<GpuRingBuffer<CameraUniform>>("camera_gpu_uniform_triple");
    //Camera buffer
    sync_buffer_with_queue::<CameraUniform, ecs::cameras::CameraUniform>(
        &camera_queue_key,
        cpu_queue_registry,
        &camera_buffer_key,
        gpu_buffer_registry,
        queue,
        gpu_frame_index,
        sim_frame_index,
    );

    //IndirectDraw
    let indirect_draw_queue_key =
        ecs::registries::RegisterKey::from_label::<CpuRingQueue<Vec<IndirectDrawCommand>>>("indirect_draw_queue");
    let indirect_draw_buffer_key =
        RegisterKey::from_label::<GpuRingBuffer<IndirectDraw>>("indirect_draw_buffer");
    let model_buffer_key =
        RegisterKey::from_label::<GpuRingBuffer<ModelUniform>>("model_gpu_uniform_triple");
    let cpu_queue_registry = cpu_queue_registry.lock().unwrap();

    let indirect_draw_queue = cpu_queue_registry
        .get(&indirect_draw_queue_key)
        .unwrap()
        .as_any()
        .downcast_ref::<CpuRingQueue<Vec<IndirectDrawCommand>>>()
        .unwrap()
        .get_read(sim_frame_index);

    let indirect_draws = indirect_draw_queue
        .iter()
        .map(|cmd| IndirectDraw {
            index_count: cmd.mesh.index_count,
            instance_count: cmd.instance_count,
            first_index: cmd.mesh.index_offset as u32,
            base_vertex: cmd.mesh.vertex_offset as i32,
            first_instance: cmd.first_instance,
            ..Default::default()
        })
        .collect::<Vec<IndirectDraw>>();

    // indirect_draws.iter().for_each(|x| info!("{:?}", x));

    let mut model_matrices: Vec<ModelUniform> = Vec::new();
    indirect_draw_queue.iter().for_each(|cmd| {
        cmd.transform.iter().for_each(|x| {
            model_matrices.push(ModelUniform {
                model: x.0.to_cols_array_2d(),
            });
        });
    });

    // model_matrices.iter().for_each(|x| info!("{:?}", x));

    debug!("size of indirect_draws {}", indirect_draws.len());
    sync_buffer_with_data::<IndirectDraw>(
        &indirect_draw_buffer_key,
        gpu_buffer_registry,
        queue,
        gpu_frame_index,
        bytemuck::cast_slice(&indirect_draws),
    );

    sync_buffer_with_data::<ModelUniform>(
        &model_buffer_key,
        gpu_buffer_registry,
        queue,
        gpu_frame_index,
        bytemuck::cast_slice(&model_matrices),
    );

    //interpolation which I have to figure out a scalable solution for all my buffers
    // let cpu_buffer_previous =
    //     cpu_uniform_triple.get_read((self.sim_frame_index.index() + 3 - 2) % 3);
    // let cpu_buffer_current =
    //     cpu_uniform_triple.get_read((self.sim_frame_index.index() + 3 - 1) % 3);

    // let alpha = self.accumulator.as_secs_f32() / self.delta_time.as_secs_f32();

    debug!(
        "synced cpu_frame_index: {}, gpu_frame_index: {}",
        (sim_frame_index + 3 - 1) % 3,
        gpu_frame_index
    );
}

fn sync_buffer_with_queue<T: 'static + Send + Sync + Pod, U: 'static + Send + Sync + Pod>(
    cpu_key: &ecs::registries::RegisterKey,
    cpu_buffer_registry: &Arc<Mutex<ecs::registries::Registry<Box<dyn QueueInterface + Send + Sync>>>>,
    gpu_key: &RegisterKey,
    gpu_buffer_registry: &mut Registry<Box<dyn BufferInterface>>,
    queue: &Queue,
    gpu_frame_index: usize,
    sim_frame_index: usize,
) {
    debug!("syncing {:?} to {:?}", cpu_key, gpu_key);
    let binding = cpu_buffer_registry.lock().unwrap();
    let cpu_buffer_entry = binding.get(cpu_key).unwrap();
    let cpu_uniform_triple = cpu_buffer_entry
        .as_any()
        .downcast_ref::<CpuRingQueue<U>>()
        .unwrap();
    let gpu_buffer_entry = gpu_buffer_registry.get_mut(gpu_key).unwrap();
    let gpu_uniform_triple = gpu_buffer_entry
        .as_mut_any()
        .downcast_mut::<GpuRingBuffer<T>>()
        .unwrap();

    let data = bytemuck::bytes_of(cpu_uniform_triple.get_read((sim_frame_index + 2) % 3));
    gpu_uniform_triple.write(queue, data, gpu_frame_index);
}

fn sync_buffer_with_data<T: 'static + Send + Sync + Pod>(
    gpu_key: &RegisterKey,
    gpu_buffer_registry: &mut Registry<Box<dyn BufferInterface>>,
    queue: &Queue,
    gpu_frame_index: usize,
    data: &[u8],
) {
    let gpu_buffer_entry = gpu_buffer_registry.get_mut(gpu_key).unwrap();
    let gpu_uniform_triple = gpu_buffer_entry
        .as_mut_any()
        .downcast_mut::<GpuRingBuffer<T>>()
        .unwrap();
    gpu_uniform_triple.write(queue, data, gpu_frame_index);
}
