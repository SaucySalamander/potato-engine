use std::fmt::Debug;

use log::info;
use wgpu::{Buffer, BufferUsages, Device, Queue};

use crate::graphics::buffers::{self, BufferEntry, GpuRingBuffer};
use ecs::components::MeshHandle;

pub struct MeshAllocator {
    vertex_buffers: GpuRingBuffer<Buffer>,
    index_buffers: GpuRingBuffer<Buffer>,

    vertex_offset: [u64; 3],
    index_offset: [u64; 3],

    vertex_capacity: u64,
    index_capacity: u64,
}

impl MeshAllocator {
    pub fn new(device: &Device, vertex_capacity: u64, index_capacity: u64) -> Self {
        Self {
            vertex_buffers: GpuRingBuffer::new(vec![
                Self::create_vertex_buffer_entry(device, vertex_capacity),
                Self::create_vertex_buffer_entry(device, vertex_capacity),
                Self::create_vertex_buffer_entry(device, vertex_capacity),
            ]),
            index_buffers: GpuRingBuffer::new(vec![
                Self::create_index_buffer_entry(device, vertex_capacity),
                Self::create_index_buffer_entry(device, vertex_capacity),
                Self::create_index_buffer_entry(device, vertex_capacity),
            ]),
            vertex_offset: [0; 3],
            index_offset: [0; 3],
            vertex_capacity: vertex_capacity,
            index_capacity: index_capacity,
        }
    }

    fn create_vertex_buffer_entry(device: &Device, vertex_capacity: u64) -> BufferEntry {
        let buffer = buffers::create_buffer(
            device,
            "Shared Vertex Buffer",
            vertex_capacity,
            vec![BufferUsages::VERTEX, BufferUsages::COPY_DST],
            false,
        );
        BufferEntry {
            buffer: buffer,
            bind_group: None,
            element_count: 0,
        }
    }

    fn create_index_buffer_entry(device: &Device, index_capacity: u64) -> BufferEntry {
        let buffer = buffers::create_buffer(
            device,
            "Shared Index Buffer",
            index_capacity,
            vec![BufferUsages::INDEX, BufferUsages::COPY_DST],
            false,
        );
        BufferEntry {
            buffer: buffer,
            bind_group: None,
            element_count: 0,
        }
    }

    pub fn upload_static_mesh<V: bytemuck::Pod + Debug, I: bytemuck::Pod + Debug>(
        &mut self,
        queue: &Queue,
        vertices: &[V],
        indices: &[I],
    ) -> Option<Vec<MeshHandle>> {
        let mut handles = Vec::new();
        for i in 0..3 {
            let vertex_size = size_of::<V>() as u64;
            let index_size = size_of::<I>() as u64;

            let vertex_data_len = vertex_size * vertices.len() as u64;
            let index_data_len = index_size * indices.len() as u64;

            if self.vertex_offset[i] + vertex_data_len > self.vertex_capacity
                || self.index_offset[i] + index_data_len > self.index_capacity
            {
                return None;
            } else {
                info!(
                    "writing vertices {:?} to buffer {} at {}",
                    vertices, i, self.vertex_offset[i]
                );
                queue.write_buffer(
                    &self.vertex_buffers.get_write(i).buffer,
                    self.vertex_offset[i],
                    bytemuck::cast_slice(vertices),
                );
                info!(
                    "writing indices {:?} to buffer {} at {}",
                    indices, i, self.index_offset[i]
                );
                queue.write_buffer(
                    &self.index_buffers.get_write(i).buffer,
                    self.index_offset[i],
                    bytemuck::cast_slice(indices),
                );

                let handle = MeshHandle {
                    vertex_offset: self.vertex_offset[i],
                    index_offset: self.index_offset[i],
                    vertex_count: vertices.len() as u32,
                    index_count: indices.len() as u32,
                };

                self.vertex_offset[i] += vertex_data_len;
                self.index_offset[i] += index_data_len;

                handles.push(handle);
            }
        }
        Some(handles)
    }

    pub fn upload_mesh<V: bytemuck::Pod + Debug, I: bytemuck::Pod + Debug>(
        &mut self,
        queue: &Queue,
        frame_index: usize,
        vertices: &[V],
        indices: &[I],
    ) -> Option<MeshHandle> {
        let vertex_size = size_of::<V>() as u64;
        let index_size = size_of::<I>() as u64;

        let vertex_data_len = vertex_size * vertices.len() as u64;
        let index_data_len = index_size * indices.len() as u64;

        if self.vertex_offset[frame_index] + vertex_data_len > self.vertex_capacity
            || self.index_offset[frame_index] + index_data_len > self.index_capacity
        {
            return None;
        } else {
            info!(
                "writing vertices {:?} to buffer {} at {}",
                vertices, frame_index, self.vertex_offset[frame_index]
            );
            queue.write_buffer(
                &self.vertex_buffers.get_write(frame_index).buffer,
                self.vertex_offset[frame_index],
                bytemuck::cast_slice(vertices),
            );
            info!(
                "writing indices {:?} to buffer {} at {}",
                indices, frame_index, self.index_offset[frame_index]
            );
            queue.write_buffer(
                &self.index_buffers.get_write(frame_index).buffer,
                self.index_offset[frame_index],
                bytemuck::cast_slice(indices),
            );

            let handle = MeshHandle {
                vertex_offset: self.vertex_offset[frame_index],
                index_offset: self.index_offset[frame_index],
                vertex_count: vertices.len() as u32,
                index_count: indices.len() as u32,
            };

            self.vertex_offset[frame_index] += vertex_data_len;
            self.index_offset[frame_index] += index_data_len;

            Some(handle)
        }
    }

    pub fn get_current_vertex_buffer(&self, frame_index: usize) -> &Buffer {
        &self.vertex_buffers.get_read(frame_index).buffer
    }

    pub fn get_current_index_buffer(&self, frame_index: usize) -> &Buffer {
        &self.index_buffers.get_read(frame_index).buffer
    }

    pub fn clear_current_frame(&mut self, frame_index: usize) {
        self.vertex_offset[frame_index] = 0;
        self.index_offset[frame_index] = 0;
    }
}
