use wgpu::{BindGroup, BindGroupLayout};

use crate::utils::Registry;

#[derive(Debug)]
pub struct BindGroupRegistry {
    pub registry: Vec<(String, BindGroup)>,
}
impl Default for BindGroupRegistry {
    fn default() -> Self {
        Self {
            registry: Vec::new(),
        }
    }
}

impl Registry<String, BindGroup> for BindGroupRegistry {
    fn insert(&mut self, key: String, value: BindGroup) {
        if let Some((_, v)) = self.registry.iter_mut().find(|(k, _)| *k == key) {
            *v = value;
        } else {
            self.registry.push((key, value));
        }
    }

    fn get(&self, key: &String) -> Option<&BindGroup> {
        self.registry
            .iter()
            .find_map(|(k, v)| if k == key { Some(v) } else { None })
    }

    fn get_mut(&mut self, key: &String) -> Option<&mut BindGroup> {
        self.registry
            .iter_mut()
            .find_map(|(k, v)| if k == key { Some(v) } else { None })
    }
}

#[derive(Debug)]
pub struct BindGroupLayoutRegistry {
    pub registry: Vec<(String, BindGroupLayout)>,
}

impl Default for BindGroupLayoutRegistry {
    fn default() -> Self {
        Self {
            registry: Vec::new(),
        }
    }
}

impl Registry<String, BindGroupLayout> for BindGroupLayoutRegistry {
    fn insert(&mut self, key: String, value: BindGroupLayout) {
        if let Some((_, v)) = self.registry.iter_mut().find(|(k, _)| *k == key) {
            *v = value;
        } else {
            self.registry.push((key, value));
        }
    }

    fn get(&self, key: &String) -> Option<&BindGroupLayout> {
        self.registry
            .iter()
            .find_map(|(k, v)| if k == key { Some(v) } else { None })
    }

    fn get_mut(&mut self, key: &String) -> Option<&mut BindGroupLayout> {
        self.registry
            .iter_mut()
            .find_map(|(k, v)| if k == key { Some(v) } else { None })
    }
}
