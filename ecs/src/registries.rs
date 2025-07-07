use std::any::TypeId;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RegisterKey {
    pub type_id: TypeId,
    pub label: &'static str,
}

impl RegisterKey {
    pub fn from_label<T: 'static>(label: &'static str) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            label,
        }
    }
}

#[derive(Debug)]
pub struct Registry<T> {
    keys: Vec<RegisterKey>,
    registry: Vec<T>,
}

impl<T> Default for Registry<T> {
    fn default() -> Self {
        Self {
            keys: Vec::new(),
            registry: Vec::new(),
        }
    }
}

impl<T: Send + Sync> Registry<T> {
    pub fn register_key(&mut self, key: RegisterKey, value: T) {
        if self.keys.contains(&key) {
            return;
        }
        self.keys.push(key);
        self.registry.push(value);
    }

    #[inline(always)]
    pub fn get(&self, key: &RegisterKey) -> Option<&T> {
        self.keys
            .iter()
            .position(|k| k == key)
            .map(|index| &self.registry[index])
    }

    #[inline(always)]
    pub fn get_mut(&mut self, key: &RegisterKey) -> Option<&mut T> {
        self.keys
            .iter()
            .position(|k| k == key)
            .map(|index| &mut self.registry[index])
    }

    pub fn keys(&self) -> impl Iterator<Item = &RegisterKey> {
        self.keys.iter()
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.registry.iter()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.registry.iter_mut()
    }
}
