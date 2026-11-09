use std::rc::Rc;

use anyhow::Result;

use crate::app::App;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::context::{CommandState, SwapchainState};
use crate::vulkanr::swapchain::RRSwapchain;

impl App {
    unsafe fn prepare_model_reload(&mut self) -> Result<(Rc<RRCommandPool>, RRSwapchain)> {
        self.rrdevice.wait_idle()?;
        let command_pool = self.resource::<CommandState>().pool.clone();
        let swapchain = self.resource::<SwapchainState>().swapchain.clone();
        Ok((command_pool, swapchain))
    }

    pub unsafe fn load_model(&mut self, path: &str) -> Result<()> {
        let (command_pool, swapchain) = self.prepare_model_reload()?;
        let (mut gpu, world, assets) =
            self.data
                .scene_parts(&self.instance, &self.rrdevice, &command_pool);
        match crate::ecs::systems::model_load_systems::load_model_keeping_scene(
            path, &mut gpu, &swapchain, world, assets,
        ) {
            Ok(()) => msg_info!("Model loaded: {}", path),
            Err(e) => msg_error!("Failed to load model: {:?}", e),
        }
        Ok(())
    }

    #[cfg(feature = "auto-rig")]
    pub unsafe fn load_model_from_glb(&mut self, glb_data: &[u8]) -> Result<()> {
        let (command_pool, swapchain) = self.prepare_model_reload()?;
        let (mut gpu, world, assets) =
            self.data
                .scene_parts(&self.instance, &self.rrdevice, &command_pool);
        crate::ecs::systems::model_load_systems::load_generated_mesh(
            glb_data, &mut gpu, &swapchain, world, assets,
        )?;
        msg_info!("Generated mesh loaded successfully");
        Ok(())
    }

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

                let mut min_x = f32::MAX;
                let mut max_x = f32::MIN;
                let mut min_y = f32::MAX;
                let mut max_y = f32::MIN;
                let mut min_z = f32::MAX;
                let mut max_z = f32::MIN;
                for v in &mesh.vertex_data.vertices {
                    min_x = min_x.min(v.pos.x);
                    max_x = max_x.max(v.pos.x);
                    min_y = min_y.min(v.pos.y);
                    max_y = max_y.max(v.pos.y);
                    min_z = min_z.min(v.pos.z);
                    max_z = max_z.max(v.pos.z);
                }
                log!(
                    "    bounds: X[{:.2}, {:.2}], Y[{:.2}, {:.2}], Z[{:.2}, {:.2}]",
                    min_x,
                    max_x,
                    min_y,
                    max_y,
                    min_z,
                    max_z
                );
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
