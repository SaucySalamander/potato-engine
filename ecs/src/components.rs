use std::any::{Any, TypeId};

use glam::{Mat4, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct Camera;

#[derive(Debug, Clone, Copy)]
pub struct FpsCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub speed: f32,
    pub sensitivity: f32,
}

#[derive(Debug, Copy, Clone)]
pub struct Position(pub Vec3);

#[derive(Debug, Copy, Clone)]
pub struct Transform(pub Mat4);

#[derive(Debug, Copy, Clone)]
pub struct MeshHandle {
    pub vertex_offset: u64,
    pub index_offset: u64,
    pub vertex_count: u32,
    pub index_count: u32,
}

pub struct ComponentTypeIndexRegistry {
    type_to_index: Vec<TypeId>,
    factories: Vec<Box<dyn Fn() -> Box<dyn ComponentStorage> + Send + Sync>>,
}

impl ComponentTypeIndexRegistry {
    pub fn new() -> Self {
        Self {
            type_to_index: Vec::new(),
            factories: Vec::new(),
        }
    }

    pub fn get_or_register<T: 'static + Send + Sync>(&mut self) -> usize {
        let type_id = TypeId::of::<T>();
        if let Some(i) = self.type_to_index.iter().position(|&id| id == type_id) {
            return i;
        }
        let index = self.type_to_index.len();
        self.type_to_index.push(type_id);

        self.factories.push(Box::new(|| {
            Box::new(Vec::<T>::new()) as Box<dyn ComponentStorage>
        }));
        index
    }

    pub fn get_index(&self, type_id: TypeId) -> Option<usize> {
        self.type_to_index.iter().position(|&id| id == type_id)
    }

    pub fn len(&self) -> usize {
        self.type_to_index.len()
    }

    pub fn create_empty_column(&self, index: usize) -> Box<dyn ComponentStorage> {
        (self.factories[index])()
    }
}

pub trait ComponentStorage: Send + Sync {
    fn push_from_other(&mut self, other: &mut Box<dyn ComponentStorage>);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Send + Sync + 'static> ComponentStorage for Vec<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn push_from_other(&mut self, other: &mut Box<dyn ComponentStorage>) {
        let other_vec = other
            .as_any_mut()
            .downcast_mut::<Vec<T>>()
            .expect("type mismatch");
        self.push(other_vec.remove(0));
    }
}

pub trait ComponentTuple {
    fn component_indices(registry: &mut ComponentTypeIndexRegistry) -> Vec<usize>;
    fn into_components(self) -> Vec<Box<dyn ComponentStorage>>;
}

macro_rules! impl_component_tuple {
    ($($name:ident),*) => {
        impl<$($name: Send + Sync + 'static),*> ComponentTuple for ($($name,)*) {
            fn component_indices(registry: &mut ComponentTypeIndexRegistry) -> Vec<usize> {
                vec![$(registry.get_or_register::<$name>()),*]
            }

            fn into_components(self) -> Vec<Box<dyn ComponentStorage>> {
                let ($($name,)*) = self;
                vec![$(Box::new(vec![$name]) as Box<dyn ComponentStorage>),*]
            }
        }
    };
}

impl_component_tuple!(A);
impl_component_tuple!(A, B);
impl_component_tuple!(A, B, C);
impl_component_tuple!(A, B, C, D);
impl_component_tuple!(A, B, C, D, E);
impl_component_tuple!(A, B, C, D, E, F);
impl_component_tuple!(A, B, C, D, E, F, G);
impl_component_tuple!(A, B, C, D, E, F, G, H);
impl_component_tuple!(A, B, C, D, E, F, G, H, I);
impl_component_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_component_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_component_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_component_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_component_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_component_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_component_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
