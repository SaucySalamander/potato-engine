use wgpu::{
    Buffer, BufferDescriptor, BufferUsages, Device,
    util::{BufferInitDescriptor, DeviceExt},
};

pub struct BufferRegistry {
    pub registry: Vec<Buffer>
}

pub fn create_buffer<T>(
    device: &Device,
    name: &str,
    buffer_uses: Vec<BufferUsages>,
    mapped_at_creation: bool,
) -> Buffer {
    let combined_buffer_uses = buffer_uses
        .iter()
        .fold(BufferUsages::empty(), |acc, &uses| acc | uses);

    device.create_buffer(&BufferDescriptor {
        label: Some(name),
        size: size_of::<T>() as u64,
        usage: combined_buffer_uses,
        mapped_at_creation,
    })
}

pub fn create_buffer_with_data(
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
