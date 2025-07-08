use std::any::TypeId;

use crate::{
    archetypes::{Archetype, ArchetypeKey},
    components::{
        ComponentTuple, ComponentTypeIndexRegistry
    },
    entities::{EntityAllocator, EntityId},
    input::InputState,
    queries::Query,
};

mod archetypes;
pub mod commands;
pub mod components;
mod entities;
pub mod input;
mod queries;

pub struct World {
    archetypes: Vec<(ArchetypeKey, Archetype)>,
    type_registry: ComponentTypeIndexRegistry,
    entity_allocator: EntityAllocator,
    entity_location_map: Vec<Option<(usize, usize)>>,
}

impl World {
    pub fn new() -> Self {
        Self {
            archetypes: Vec::new(),
            type_registry: ComponentTypeIndexRegistry::new(),
            entity_allocator: EntityAllocator::new(),
            entity_location_map: Vec::new(),
        }
    }

    pub fn run_systems(
        &mut self,
        frame_index: usize,
        input: &InputState,
        delta_time: f32,
    ) {
        self.run_transform_system();
    }

    fn run_transform_system(&mut self) {}

    pub fn spawn<T: ComponentTuple>(&mut self, components: T) -> EntityId {
        let entity = self.entity_allocator.allocate();
        let component_indices = T::component_indices(&mut self.type_registry);
        let component_data = components.into_components();
        let layout_key = ArchetypeKey::new_sorted(&component_indices);
        let archetype_index = self.find_or_create_archetype(&layout_key, &component_indices);
        let (_, archetype) = &mut self.archetypes[archetype_index];
        let row = archetype.entities.len();
        archetype.insert(entity.clone(), component_indices, component_data);

        self.entity_location_map
            .resize_with(entity.index as usize + 1, || None);

        self.entity_location_map[entity.index as usize] = Some((archetype_index, row));
        entity
    }

    pub fn get_component<T: 'static>(&self, entity: EntityId) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        let index = self.type_registry.get_index(type_id).unwrap();

        let (archetype_index, row) = self
            .entity_location_map
            .get(entity.index as usize)
            .unwrap()
            .as_ref()
            .unwrap();
        let (_, archetype) = &self.archetypes[*archetype_index];
        archetype
            .get_column::<T>(index)
            .and_then(|vec| vec.get(*row))
    }

    fn find_or_create_archetype(
        &mut self,
        key: &ArchetypeKey,
        component_indices: &[usize],
    ) -> usize {
        for (i, arch) in self.archetypes.iter().enumerate() {
            if &arch.0 == key {
                return i;
            }
        }

        let new_arch = Archetype::new(component_indices, &self.type_registry);
        self.archetypes.push((key.clone(), new_arch));
        self.archetypes.len() - 1
    }

    pub fn query<'world, Q>(&'world mut self) -> impl Iterator<Item = Q::Item>
    where
        Q: Query<'world>,
    {
        self.archetypes
            .iter_mut()
            .filter_map(|(_, archetype)| Q::query_archetype(archetype, &self.type_registry))
            .flat_map(|it| it)
    }
}
