use wgpu::{BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Device};

pub fn create_bind_group_layout(
    label: &str,
    device: &Device,
    entry: &Vec<BindGroupLayoutEntry>,
) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &entry,
    })
}
