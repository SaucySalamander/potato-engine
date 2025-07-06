use crate::{
    components::{ComponentStorage, ComponentTypeIndexRegistry},
    entities::EntityId,
};

pub struct Archetype {
    components: Vec<Option<Box<dyn ComponentStorage>>>,
    pub entities: Vec<EntityId>,
}

impl Archetype {
    pub fn new(component_indices: &[usize], registry: &ComponentTypeIndexRegistry) -> Self {
        let total_types = registry.len();
        let mut components = Vec::with_capacity(total_types);
        components.resize_with(total_types, || None);
        for &index in component_indices {
            assert!(
                index < total_types,
                "component index {} out of bounds",
                index
            );
            components[index] = Some(registry.create_empty_column(index));
        }
        Self {
            components,
            entities: Vec::new(),
        }
    }

    pub fn get_column<T: 'static>(&self, index: usize) -> Option<&Vec<T>> {
        self.components.get(index).and_then(|opt_storage| {
            opt_storage
                .as_ref()
                .and_then(|storage| storage.as_any().downcast_ref::<Vec<T>>())
        })
    }

    pub fn get_column_mut<T: 'static>(&mut self, index: usize) -> Option<&mut Vec<T>> {
        self.components.get_mut(index).and_then(|opt_storage| {
            opt_storage
                .as_mut()
                .and_then(|storage| storage.as_any_mut().downcast_mut::<Vec<T>>())
        })
    }

    pub fn insert(
        &mut self,
        entity: EntityId,
        component_indices: Vec<usize>,
        mut components: Vec<Box<dyn ComponentStorage>>,
    ) {
        self.entities.push(entity);

        for (i, storage) in component_indices.iter().enumerate() {
            let column = self.components[*storage]
                .as_mut()
                .expect("column should exist for registerd component type");

            column.push_from_other(&mut components[i]);
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct ArchetypeKey(Vec<usize>);

impl ArchetypeKey {
    pub fn new_sorted(indices: &[usize]) -> Self {
        let mut key = indices.to_vec();
        key.sort_unstable();
        ArchetypeKey(key)
    }
}

pub trait GetColumns<'world, T> {
    fn get_columns(&'world self, indices: &[usize]) -> Option<T>;
    fn get_columns_mut(&'world mut self, indices: &[usize]) -> Option<T>;
}

macro_rules! impl_get_columns {
    ($($name:ident),*) => {
        impl<'world, $($name: 'static),*> GetColumns<'world, ($(&'world Vec<$name>,)*)> for Archetype {
            fn get_columns(&'world self, indices: &[usize]) -> Option<($(&'world Vec<$name>,)*)> {
                let mut iter = indices.iter();
                Some(($(self.get_column::<$name>(*iter.next()?)?,)*))
            }
            fn get_columns_mut(&'world mut self, _indices: &[usize]) -> Option<($(&'world Vec<$name>,)*)> {
                // Mutable access to immutable types is disallowed
                None
            }
        }
    };
}

impl_get_columns!(A);
impl_get_columns!(A, B);
impl_get_columns!(A, B, C);
impl_get_columns!(A, B, C, D);
impl_get_columns!(A, B, C, D, E);
impl_get_columns!(A, B, C, D, E, F);
impl_get_columns!(A, B, C, D, E, F, G);
impl_get_columns!(A, B, C, D, E, F, G, H);
impl_get_columns!(A, B, C, D, E, F, G, H, I);
impl_get_columns!(A, B, C, D, E, F, G, H, I, J);
impl_get_columns!(A, B, C, D, E, F, G, H, I, J, K);
impl_get_columns!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_get_columns!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_get_columns!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_get_columns!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_get_columns!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

macro_rules! impl_get_columns_mut {
    ($($name:ident),*) => {
        impl<'world, $($name: 'static),*> GetColumns<'world, ($(&'world mut Vec<$name>,)*)> for Archetype {
            fn get_columns(&'world self, _indices: &[usize]) -> Option<($(&'world mut Vec<$name>,)*)> {
                // Immutable access to mut types is disallowed
                None
            }
            fn get_columns_mut(&'world mut self, indices: &[usize]) -> Option<($(&'world mut Vec<$name>,)*)> {
                let mut iter = indices.iter();
                let test = ($({
                    let idx = *iter.next()?;
                    let ptr = self as *mut Self;
                    unsafe { &mut *ptr }.get_column_mut::<$name>(idx)?
                },)*);

                Some(test)
            }
        }
    };
}

impl_get_columns_mut!(A);
impl_get_columns_mut!(A, B);
impl_get_columns_mut!(A, B, C);
impl_get_columns_mut!(A, B, C, D);
impl_get_columns_mut!(A, B, C, D, E);
impl_get_columns_mut!(A, B, C, D, E, F);
impl_get_columns_mut!(A, B, C, D, E, F, G);
impl_get_columns_mut!(A, B, C, D, E, F, G, H);
impl_get_columns_mut!(A, B, C, D, E, F, G, H, I);
impl_get_columns_mut!(A, B, C, D, E, F, G, H, I, J);
impl_get_columns_mut!(A, B, C, D, E, F, G, H, I, J, K);
impl_get_columns_mut!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_get_columns_mut!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_get_columns_mut!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_get_columns_mut!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_get_columns_mut!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
