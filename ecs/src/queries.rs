use crate::archetypes::Archetype;
use crate::components::ComponentTypeIndexRegistry;

// ecs_macros::impl_query!();

use ecs_macros::impl_query_combinations;

impl_query_combinations!(crate);

pub trait Query<'world> {
    type Item;

    fn query_archetype(
        archetype: &'world mut Archetype,
        registry: &ComponentTypeIndexRegistry,
    ) -> Option<Box<dyn Iterator<Item = Self::Item> + 'world>>;
}
