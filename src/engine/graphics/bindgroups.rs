use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, Device,
};

use crate::utils::Registry;

#[derive(Debug)]
pub struct BindGroupRegistry {
    pub registry: Vec<(String, BindGroup)>,
}

impl BindGroupRegistry {
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

    type KeysIterator<'a>
        = Box<dyn Iterator<Item = &'a String> + 'a>
    where
        String: 'a,
        BindGroup: 'a;

    type ValuesIterator<'a>
        = Box<dyn Iterator<Item = &'a BindGroup> + 'a>
    where
        String: 'a,
        BindGroup: 'a;

    fn keys(&self) -> Self::KeysIterator<'_> {
        Box::new(self.registry.iter().map(|(k, _)| k))
    }

    fn valuse(&self) -> Self::ValuesIterator<'_> {
        Box::new(self.registry.iter().map(|(_, v)| v))
    }
}

#[derive(Debug)]
pub struct BindGroupLayoutRegistry {
    pub registry: Vec<(String, BindGroupLayout)>,
}

impl BindGroupLayoutRegistry {
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

    type KeysIterator<'a>
        = Box<dyn Iterator<Item = &'a String> + 'a>
    where
        String: 'a,
        BindGroupLayout: 'a,
        Self: 'a;

    type ValuesIterator<'a>
        = Box<dyn Iterator<Item = &'a BindGroupLayout> + 'a>
    where
        String: 'a,
        BindGroupLayout: 'a,
        Self: 'a;

    fn keys(&self) -> Self::KeysIterator<'_> {
        Box::new(self.registry.iter().map(|(k, _)| k))
    }

    fn valuse(&self) -> Self::ValuesIterator<'_> {
        Box::new(self.registry.iter().map(|(_, v)| v))
    }
}
