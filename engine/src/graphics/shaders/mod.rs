use std::{borrow::Cow, fs};

use wgpu::{Device, ShaderModule, ShaderModuleDescriptor, ShaderSource};

pub fn load_shader(device: &Device, shader_name: String) -> ShaderModule {
    let shader = match fs::read_to_string(shader_name) {
        Ok(shader) => shader,
        Err(err) => panic!("failed to load file, {}", err),
    };

    let shader_module = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("hello triangle"),
        source: ShaderSource::Wgsl(Cow::Borrowed(&shader)),
    });

    shader_module
}
