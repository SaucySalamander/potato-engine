use std::any::TypeId;

use glam::{Mat4, Vec3};

use crate::{
    archetypes::{Archetype, ArchetypeKey},
    cameras::CameraUniform,
    commands::IndirectDrawCommand,
    components::{
        Camera, ComponentTuple, ComponentTypeIndexRegistry, FpsCamera, MeshHandle, Position,
        Transform,
    },
    entities::{EntityAllocator, EntityId},
    input::InputState,
    queries::Query,
    queues::{CpuRingQueue, QueueInterface},
    registries::{RegisterKey, Registry},
};

mod archetypes;
pub mod cameras;
pub mod commands;
pub mod components;
mod entities;
pub mod input;
mod queries;
pub mod queues;
pub mod registries;

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
        cpu_queue_registry: &mut Registry<Box<dyn QueueInterface + Send + Sync>>,
    ) {
        self.run_transform_system();
        self.update_fps_camera_system(input, delta_time, frame_index, cpu_queue_registry);
        self.run_render_submission_system(frame_index, cpu_queue_registry);
    }

    fn run_transform_system(&mut self) {}

    fn update_fps_camera_system(
        &mut self,
        input: &InputState,
        delta_time: f32,
        sim_frame_index: usize,
        cpu_queue_registry: &mut Registry<Box<dyn QueueInterface + Send + Sync>>,
    ) {
        for (camera, pos, _) in self.query::<(&mut FpsCamera, &mut Position, &Camera)>() {
            let forward = Vec3::new(
                camera.yaw.cos() * camera.pitch.cos(),
                camera.pitch.sin(),
                camera.yaw.sin() * camera.pitch.cos(),
            )
            .normalize();
            let right = forward.cross(Vec3::Y).normalize();
            let up = right.cross(forward).normalize();

            // Movement
            let mut velocity = Vec3::ZERO;
            if input.key_w {
                velocity += forward;
            }
            if input.key_s {
                velocity -= forward;
            }
            if input.key_d {
                velocity += right;
            }
            if input.key_a {
                velocity -= right;
            }
            if input.key_space {
                velocity += up;
            }
            if input.key_ctrl {
                velocity -= up;
            }

            if velocity.length_squared() > 0.0 {
                *pos = Position(pos.0 + velocity.normalize() * camera.speed * delta_time);
            }

            camera.yaw += input.mouse_delta_x * camera.sensitivity;
            camera.pitch -= input.mouse_delta_y * camera.sensitivity;
            camera.pitch = camera
                .pitch
                .clamp(-89.9_f32.to_radians(), 89.9_f32.to_radians());

            //updating cpu buffers
            let camera_queue_key =
                RegisterKey::from_label::<CpuRingQueue<CameraUniform>>("camera_cpu_uniform_triple");
            let camera_queue_entry = cpu_queue_registry.get_mut(&camera_queue_key).unwrap();
            let camera_uniform_triple = camera_queue_entry
                .as_mut_any()
                .downcast_mut::<CpuRingQueue<CameraUniform>>()
                .unwrap();
            let camera_uniform = camera_uniform_triple.get_write(sim_frame_index);
            camera_uniform.view = Mat4::look_to_rh(pos.0, forward, Vec3::Y).to_cols_array_2d();
            camera_uniform.projection =
                Mat4::perspective_rh(0.785, 16.0 / 9.0, 0.1, 1000.0).to_cols_array_2d();
        }
    }

    fn run_render_submission_system(
        &mut self,
        sim_frame_index: usize,
        cpu_queue_registry: &mut Registry<Box<dyn QueueInterface + Send + Sync>>,
    ) {
        let key = RegisterKey::from_label::<CpuRingQueue<Vec<IndirectDrawCommand>>>(
            "indirect_draw_queue",
        );
        if let Some(queue) = cpu_queue_registry.get_mut(&key) {
            let queue = queue
                .as_mut_any()
                .downcast_mut::<CpuRingQueue<Vec<IndirectDrawCommand>>>()
                .expect("Failed to downcast indirect draw queue");

            // let prev_index = (sim_frame_index + 2) % 3;
            // let prev_data = queue.get_read(prev_index).clone();

            let mut first_instance_counter = 0;
            let current_data = queue.get_write(sim_frame_index);
            current_data.clear();

            let mut batch: Vec<Transform> = Vec::new();
            let mut mesh_handle = MeshHandle {
                vertex_offset: 0,
                index_offset: 0,
                vertex_count: 0,
                index_count: 0,
            };

            for (i, (transform, mesh)) in self.query::<(&Transform, &MeshHandle)>().enumerate() {
                batch.push(transform.clone());
                mesh_handle = mesh.clone();
            }

            current_data.push(IndirectDrawCommand {
                instance_count: batch.len() as u32,
                first_instance: first_instance_counter,
                mesh: mesh_handle,
                transform: batch.clone(),
            });
        }
    }

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
