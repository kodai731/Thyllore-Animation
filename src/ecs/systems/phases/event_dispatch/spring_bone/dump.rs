use super::bake::find_spring_bake_inputs;
use crate::asset::AssetStorage;
use crate::ecs::events::UiCommand;
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use std::path::PathBuf;

#[derive(Debug)]
pub struct DumpSpringTails {
    pub path: PathBuf,
    pub sample_rate: f32,
}

impl UiCommand for DumpSpringTails {
    fn apply(self: Box<Self>, world: &mut World, assets: &mut AssetStorage, _: &GraphicsResources) {
        use crate::ecs::systems::spring_bone_bake_systems::{simulate_spring_tails, BakeConfig};

        let inputs = match find_spring_bake_inputs(world, assets) {
            Some(i) => i,
            None => return,
        };

        let config = BakeConfig {
            start_time: 0.0,
            end_time: inputs.anim_clip.duration,
            sample_rate: self.sample_rate,
            space: inputs.space,
        };

        let samples = simulate_spring_tails(
            &config,
            &inputs.setup,
            &inputs.skeleton,
            &inputs.anim_clip,
            inputs.constraints.as_ref(),
            inputs.looping,
        );

        let csv = crate::animation::spring_bone::format_tail_csv(&samples);
        match std::fs::write(&self.path, &csv) {
            Ok(()) => log!(
                "Wrote {} spring tail samples to {:?}",
                samples.len(),
                self.path
            ),
            Err(e) => msg_warn!("Failed to write spring tails to {:?}: {}", self.path, e),
        }
    }
}

#[derive(Debug)]
pub struct DumpSkinnedVertices {
    pub path: PathBuf,
    pub sample_rate: f32,
    pub submesh: thyllore_avatar_core::garment::components::submesh_role::SubmeshKey,
}

impl UiCommand for DumpSkinnedVertices {
    fn apply(
        self: Box<Self>,
        world: &mut World,
        assets: &mut AssetStorage,
        graphics: &GraphicsResources,
    ) {
        use crate::ecs::systems::spring_bone_bake_systems::{
            simulate_skinned_positions, BakeConfig,
        };

        let inputs = match find_spring_bake_inputs(world, assets) {
            Some(i) => i,
            None => return,
        };

        let submesh_meshes = {
            let node_assets = world.resource::<crate::ecs::resource::NodeAssets>();
            crate::ecs::systems::avatar_submesh_role_systems::collect_submesh_meshes(
                assets,
                graphics,
                &node_assets.nodes,
            )
        };

        let Some(&(_, graphics_mesh_index)) =
            submesh_meshes.iter().find(|(k, _)| *k == self.submesh)
        else {
            let available: Vec<String> = submesh_meshes
                .iter()
                .map(|(k, _)| format!("{}/{}", k.mesh, k.material))
                .collect();
            msg_warn!(
                "Submesh {}/{} not found; available: {}",
                self.submesh.mesh,
                self.submesh.material,
                available.join(", ")
            );
            return;
        };

        let Some(skin) = graphics.meshes[graphics_mesh_index].skin_data.as_ref() else {
            msg_warn!(
                "Submesh {}/{} has no skin data",
                self.submesh.mesh,
                self.submesh.material
            );
            return;
        };

        let config = BakeConfig {
            start_time: 0.0,
            end_time: inputs.anim_clip.duration,
            sample_rate: self.sample_rate,
            space: inputs.space,
        };

        let frames = simulate_skinned_positions(
            &config,
            &inputs.setup,
            &inputs.skeleton,
            &inputs.anim_clip,
            inputs.constraints.as_ref(),
            inputs.looping,
            skin,
        );

        let mut csv = String::from("frame,vertex,bind_x,bind_y,bind_z,x,y,z\n");
        for (frame_idx, positions) in frames.iter().enumerate() {
            for (v_idx, pos) in positions.iter().enumerate() {
                let bind = &skin.base_positions[v_idx];
                csv.push_str(&format!(
                    "{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}\n",
                    frame_idx, v_idx, bind.x, bind.y, bind.z, pos.x, pos.y, pos.z,
                ));
            }
        }

        match std::fs::write(&self.path, &csv) {
            Ok(()) => log!(
                "Wrote {} frames of skinned vertices to {:?}",
                frames.len(),
                self.path
            ),
            Err(e) => msg_warn!("Failed to write skinned vertices to {:?}: {}", self.path, e),
        }
    }
}

#[derive(Debug)]
pub struct FitSpringParams {
    pub reference: PathBuf,
    pub sample_rate: f32,
    pub params: thyllore_math_core::NelderMeadParams,
}

impl UiCommand for FitSpringParams {
    fn apply(self: Box<Self>, world: &mut World, assets: &mut AssetStorage, _: &GraphicsResources) {
        use crate::ecs::systems::spring_bone_bake_systems::BakeConfig;
        use crate::ecs::systems::spring_bone_edit_systems::reset_spring_bone_state;
        use crate::ecs::systems::spring_bone_fit_systems::{fit_spring_params, SpringFitInputs};

        let Some(reference) = read_spring_reference(&self.reference) else {
            return;
        };

        let inputs = match find_spring_bake_inputs(world, assets) {
            Some(i) => i,
            None => return,
        };

        let config = BakeConfig {
            start_time: 0.0,
            end_time: inputs.anim_clip.duration,
            sample_rate: self.sample_rate,
            space: inputs.space,
        };

        let spring_fit_inputs = SpringFitInputs {
            config: &config,
            skeleton: &inputs.skeleton,
            clip: &inputs.anim_clip,
            constraints: inputs.constraints.as_ref(),
            looping: inputs.looping,
            reference: &reference,
        };

        let (fitted_setup, fits) =
            fit_spring_params(&spring_fit_inputs, &inputs.setup, &self.params);

        if fits.is_empty() {
            msg_warn!("Spring parameter estimation: no chains to fit");
            return;
        }

        for fit in &fits {
            msg_info!(
                "Fitted spring chain {}: rms {:.4} -> {:.4} ({:?}, drag {:.3})",
                fit.chain,
                fit.before_rms,
                fit.after_rms,
                fit.force,
                fit.drag_force
            );
        }

        world.insert_component(inputs.entity, fitted_setup);
        reset_spring_bone_state(world);
    }
}

fn read_spring_reference(
    path: &PathBuf,
) -> Option<Vec<crate::animation::spring_bone::SpringTailSample>> {
    let csv = match std::fs::read_to_string(path) {
        Ok(csv) => csv,
        Err(e) => {
            msg_error!("Failed to read spring reference CSV {:?}: {}", path, e);
            return None;
        }
    };

    match crate::animation::spring_bone::parse_tail_csv(&csv) {
        Ok(samples) => Some(samples),
        Err(e) => {
            msg_error!("Failed to parse spring reference CSV {:?}: {}", path, e);
            None
        }
    }
}
