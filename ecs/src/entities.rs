#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EntityId {
    pub index: u32,
    generation: u32,
}

pub struct EntityAllocator {
    generations: Vec<u32>,
    free_list: Vec<u32>,
}

impl EntityAllocator {
    pub fn new() -> Self {
        Self {
            generations: Vec::new(),
            free_list: Vec::new(),
        }
    }

    pub fn allocate(&mut self) -> EntityId {
        if let Some(index) = self.free_list.pop() {
            let generation = self.generations[index as usize];
            EntityId { index, generation }
        } else {
            let index = self.generations.len() as u32;
            self.generations.push(0);
            EntityId {
                index,
                generation: 0,
            }
        }
    }

    pub fn deallocate(&mut self, entity: EntityId) {
        let index = entity.index as usize;
        if self.generations[index] == entity.generation {
            self.generations[index] += 1;
            self.free_list.push(entity.index);
        }
    }

    pub fn is_alive(&self, entity: EntityId) -> bool {
        self.generations
            .get(entity.index as usize)
            .map_or(false, |&generation| generation == entity.generation)
    }
}

type ArchetypeIndex = usize;
type RowIndex = usize;

pub struct EntityLocationMap {
    slots: Vec<Option<(ArchetypeIndex, RowIndex)>>,
}

impl EntityLocationMap {
    pub fn new() -> Self {
        Self { slots: Vec::new() }
    }

    pub fn insert(&mut self, entity: EntityId, location: (usize, usize)) {
        let idx = entity.index as usize;
        if self.slots.len() <= idx {
            self.slots.resize(idx + 1, None);
        }
        self.slots[idx] = Some(location);
    }

    pub fn get(&self, entity: EntityId) -> Option<(usize, usize)> {
        self.slots.get(entity.index as usize).copied().flatten()
    }

    pub fn remove(&mut self, entity: EntityId) {
        if let Some(slot) = self.slots.get_mut(entity.index as usize) {
            *slot = None;
        }
    }
}
