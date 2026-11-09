use crate::app::App;
use crate::ecs::events::DebugPrimitiveKind;
use crate::vulkanr::context::CommandState;
use anyhow::Result;
use glam::Vec3;

impl App {
    pub unsafe fn spawn_debug_primitive(&mut self, kind: DebugPrimitiveKind) -> Result<()> {
        let position = default_debug_primitive_position(kind);
        self.spawn_debug_primitive_at(kind, position)
    }

    pub unsafe fn spawn_debug_primitive_at(
        &mut self,
        kind: DebugPrimitiveKind,
        position: Vec3,
    ) -> Result<()> {
        log!("Spawning debug primitive: {:?}", kind);
        let (load_result, part_name) = match kind {
            DebugPrimitiveKind::Cube => (
                thyllore_importer_core::primitive::build_cube_model(1.0),
                "Cube",
            ),
            DebugPrimitiveKind::Sphere => (
                thyllore_importer_core::primitive::build_uv_sphere_model(0.6, 32, 16),
                "Sphere",
            ),
            DebugPrimitiveKind::Floor => (
                thyllore_importer_core::primitive::build_box_model(
                    12.0,
                    0.2,
                    12.0,
                    [0.8, 0.8, 0.8, 1.0],
                ),
                "Floor",
            ),
        };
        let parent_entity = self.rebuild_scene(|gpu, swapchain, world, assets| {
            crate::ecs::systems::model_load_systems::append_model_to_scene(
                &load_result,
                part_name,
                gpu,
                swapchain,
                world,
                assets,
            )
        })?;

        self.data.ecs_world.insert_component(
            parent_entity,
            crate::ecs::component::DebugPrimitiveTag { kind },
        );

        let mut transform = self
            .data
            .ecs_world
            .get_component_mut::<crate::ecs::world::Transform>(parent_entity)
            .unwrap();
        transform.translation = position;

        msg_info!(
            "Debug primitive spawned: {:?} at ({:.1}, {:.1}, {:.1})",
            kind,
            position.x,
            position.y,
            position.z
        );
        Ok(())
    }

    /// Debug primitives restored from a scene carry only tag and placement until the GPU mesh
    /// is built here.
    pub unsafe fn spawn_pending_debug_primitives(&mut self) {
        let awaiting =
            crate::ecs::systems::take_debug_primitives_awaiting_mesh(&mut self.data.ecs_world);
        for record in awaiting {
            let spawned = record
                .kind()
                .and_then(|kind| self.spawn_debug_primitive_at(kind, record.position.into()));
            if let Err(e) = spawned {
                log_error!(
                    "Failed to spawn scene debug primitive {}: {:?}",
                    record.kind,
                    e
                );
            }
        }
    }

    pub unsafe fn delete_entities(&mut self, entities: &[u64]) -> Result<()> {
        self.rrdevice.wait_idle()?;

        for &entity in entities {
            let mesh_ref = self
                .data
                .ecs_world
                .get_component::<crate::ecs::world::MeshRef>(entity)
                .cloned();

            if let Some(mesh_ref) = mesh_ref {
                let graphics_mesh_index = self
                    .data
                    .ecs_assets
                    .get_mesh(mesh_ref.mesh_asset_id)
                    .map(|m| m.graphics_mesh_index);

                if let Some(idx) = graphics_mesh_index {
                    if idx < self.data.graphics_resources.meshes.len() {
                        self.data.graphics_resources.meshes[idx].render_to_gbuffer = false;
                        self.data.graphics_resources.meshes[idx].destroy(&self.rrdevice);
                    }
                }
            }

            self.data.ecs_world.despawn(entity);
        }

        let command_pool = self.resource::<CommandState>().pool.clone();
        let (mut gpu, world, assets) =
            self.data
                .scene_parts(&self.instance, &self.rrdevice, &command_pool);
        crate::ecs::systems::rebuild_scene_acceleration(&mut gpu, world, assets)?;

        log!("Deleted {} entities with GPU cleanup", entities.len());
        Ok(())
    }
}

fn default_debug_primitive_position(kind: DebugPrimitiveKind) -> Vec3 {
    match kind {
        DebugPrimitiveKind::Cube => Vec3::new(3.0, 0.5, 0.0),
        DebugPrimitiveKind::Sphere => Vec3::new(-3.0, 0.6, 0.0),
        DebugPrimitiveKind::Floor => Vec3::new(0.0, -1.6, 0.0),
    }
}
