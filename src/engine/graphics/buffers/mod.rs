use std::{any::Any, marker::PhantomData};

use wgpu::{
    BindGroup, Buffer, BufferDescriptor, BufferUsages, Device, Queue,
    util::{BufferInitDescriptor, DeviceExt},
};

use crate::{
    engine::{cameras::CameraUniform, model::ModelUniform},
    utils::Registry,
};

pub trait BufferInterface: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
}

#[derive(Debug)]
pub struct BufferRegistry<T> {
    pub registry: Vec<(String, T)>,
}

impl<T> Default for BufferRegistry<T> {
    fn default() -> Self {
        Self {
            registry: Vec::new(),
        }
    }
}

impl<T> Registry<String, T> for BufferRegistry<T> {
    fn insert(&mut self, key: String, value: T) {
        if let Some((_, v)) = self.registry.iter_mut().find(|(k, _)| *k == key) {
            *v = value;
        } else {
            self.registry.push((key, value));
        }
    }

    fn get(&self, key: &String) -> Option<&T> {
        self.registry
            .iter()
            .find_map(|(k, v)| if k == key { Some(v) } else { None })
    }

    fn get_mut(&mut self, key: &String) -> Option<&mut T> {
        self.registry
            .iter_mut()
            .find_map(|(k, v)| if k == key { Some(v) } else { None })
    }

    type KeysIterator<'a>
        = Box<dyn Iterator<Item = &'a String> + 'a>
    where
        String: 'a,
        T: 'a,
        Self: 'a;

    type ValuesIterator<'a>
        = Box<dyn Iterator<Item = &'a T> + 'a>
    where
        String: 'a,
        T: 'a,
        Self: 'a;

    fn keys(&self) -> Self::KeysIterator<'_> {
        Box::new(self.registry.iter().map(|(k, _)| k))
    }

    fn valuse(&self) -> Self::ValuesIterator<'_> {
        Box::new(self.registry.iter().map(|(_, v)| v))
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

    pub fn write(&mut self, queue: &Queue, data: &T, frame_index: usize)
    where
        T: bytemuck::Pod,
    {
        let entry = self.get_write(frame_index);
        queue.write_buffer(&entry.buffer, 0, bytemuck::bytes_of(data));
    }
}
