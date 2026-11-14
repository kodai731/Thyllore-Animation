use thyllore_math_core::Aabb;

use crate::app::App;

impl App {
    pub fn dump_debug_info(&self) {
        log!("========== DUMP DEBUG INFORMATION ==========");

        use crate::ecs::{ClipLibrary, ModelState};
        let clip_library = self.resource::<ClipLibrary>();
        let model_state = self.resource::<ModelState>();

        log!("--- Model Info ---");
        log!("  current_model_path: {}", model_state.model_path);
        log!(
            "  meshes count: {}",
            self.data.graphics_resources.meshes.len()
        );
        log!("  has_skinned_meshes: {}", model_state.has_skinned_meshes);
        log!("  animation clips count: {}", clip_library.clip_count());
        log!(
            "  skeletons count: {}",
            clip_library.animation.skeletons.len()
        );

        log!("--- GraphicsResources Info ---");
        log!(
            "  meshes count: {}",
            self.data.graphics_resources.meshes.len()
        );
        log!(
            "  materials count: {}",
            self.data.graphics_resources.materials.materials.len()
        );
        log!(
            "  mesh_material_ids: {:?}",
            self.data.graphics_resources.mesh_material_ids
        );

        for (i, mesh) in self.data.graphics_resources.meshes.iter().enumerate() {
            log!(
                "  mesh[{}]: render_to_gbuffer={}, vertex_buffer={:?}, indices={}",
                i,
                mesh.render_to_gbuffer,
                mesh.vertex_buffer.buffer(),
                mesh.index_buffer.indices
            );
            log!(
                "    vertex_data.vertices count: {}",
                mesh.vertex_data.vertices.len()
            );
            log!("    object_index: {}", mesh.object_index);

            if !mesh.vertex_data.vertices.is_empty() {
                let v = &mesh.vertex_data.vertices[0];
                log!(
                    "    vertex_data[0].pos: ({:.4}, {:.4}, {:.4})",
                    v.pos.x,
                    v.pos.y,
                    v.pos.z
                );

                let positions = mesh.vertex_data.vertices.iter().map(|vertex| vertex.pos);
                if let Some(bounds) = Aabb::from_points(positions) {
                    log!(
                        "    bounds: X[{:.2}, {:.2}], Y[{:.2}, {:.2}], Z[{:.2}, {:.2}]",
                        bounds.min.x,
                        bounds.max.x,
                        bounds.min.y,
                        bounds.max.y,
                        bounds.min.z,
                        bounds.max.z
                    );
                }
            }
        }

        log!("--- Camera Info ---");
        log!(
            "  pivot: {:?}",
            self.resource::<crate::ecs::resource::Camera>().pivot
        );

        log!("--- Animation Info ---");
        let timeline = self
            .data
            .ecs_world
            .resource::<crate::ecs::resource::TimelineState>();
        log!("  animation_playing: {}", timeline.playing);
        log!("  clips count: {}", clip_library.clip_count());

        log!("========== END DEBUG INFORMATION ==========");
    }
}
