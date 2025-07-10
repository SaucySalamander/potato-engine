use std::{
    any::Any,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use bytemuck::Pod;
use log::debug;
use wgpu::{
    BindGroup, Buffer, BufferDescriptor, BufferUsages, Device, Queue,
    util::{BufferInitDescriptor, DeviceExt, StagingBelt},
};

pub mod bindgroups;
pub mod layouts;
pub mod submissions;

use crate::{
    graphics::buffers::submissions::{CameraUniform, IndirectDraw, ModelUniform},
    utils::{RegisterKey, Registry},
};
use ecs::commands::IndirectDrawCommand;

pub trait BufferInterface: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
}

#[derive(Clone)]
pub struct BufferEntry {
    pub buffer: Buffer,
    pub bind_group: Option<BindGroup>,
    pub element_count: u32,
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
