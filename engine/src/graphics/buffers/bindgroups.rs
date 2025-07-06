use std::any::Any;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, Device,
};
pub trait BindGroupInterface: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
}

pub fn create_bind_group(
    label: &str,
    device: &Device,
    model_bind_group_layout: &BindGroupLayout,
    entry: &Vec<BindGroupEntry>,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        label: Some(label),
        layout: model_bind_group_layout,
        entries: &entry,
    })
}
