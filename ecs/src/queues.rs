use std::any::Any;

use crate::{
    cameras::CameraUniform, commands::IndirectDrawCommand
};

pub trait QueueInterface: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
}

pub struct CpuRingQueue<T> {
    queues: [T; 3],
}

impl QueueInterface for CpuRingQueue<CameraUniform> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// impl QueueInterface for CpuRingQueue<ModelUniform> {
//     fn as_any(&self) -> &dyn std::any::Any {
//         self
//     }

//     fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
//         self
//     }
// }

impl QueueInterface for CpuRingQueue<Vec<IndirectDrawCommand>> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl<T> CpuRingQueue<T> {
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

impl<T> CpuRingQueue<Vec<T>> {
    pub fn clear(&mut self, frame_index: usize) {
        self.queues[frame_index].clear();
    }
}
