use crate::engine::{cameras::CameraUniform, graphics::buffers::BufferInterface, model::ModelUniform};

pub struct CpuRingBuffer<T> {
    queues: [T; 3],
}

impl BufferInterface for CpuRingBuffer<CameraUniform> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl BufferInterface for CpuRingBuffer<ModelUniform> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl<T> CpuRingBuffer<T> {
    pub fn new(init: T) -> Self
    where
        T: Clone,
    {
        Self {
            queues: [init.clone(), init.clone(), init],
        }
    }

    pub fn get_read(&self, frame_index: usize) -> &T {
        &self.queues[frame_index]
    }

    pub fn get_write(&mut self, frame_index: usize) -> &mut T {
        &mut self.queues[frame_index]
    }
}
