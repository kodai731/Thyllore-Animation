use std::path::{Path, PathBuf};

use crate::ecs::component::{FlameBaked, FlameEffect, FlameTemporalAccum};
use crate::ecs::resource::{Camera, FlameRenderSettings};
use crate::ecs::systems::camera_systems::{
    compute_camera_direction, compute_camera_position, compute_camera_right, compute_camera_up,
};
use crate::ecs::systems::flame_dump_systems::{
    write_flame_field_traces, write_flame_wall_probe_dump,
};
use crate::ecs::World;
use thyllore_effect_core::{probe_flame_wall, WallProbeReport, WallProbeView};
use thyllore_log_core::{log, log_warn};

pub const BATCH_VIEWPORT_SIZE: [f32; 2] = [1680.0, 840.0];

type ProbedFlame = (FlameEffect, FlameBaked, FlameTemporalAccum, WallProbeReport);

struct WallProbeScene {
    camera: Camera,
    settings: FlameRenderSettings,
    view: WallProbeView,
    flames: Vec<ProbedFlame>,
}

fn probe_wall_scene(world: &World, viewport_size: [f32; 2]) -> Option<WallProbeScene> {
    let camera = (*world.resource::<Camera>()).clone();
    let settings = world
        .get_resource::<FlameRenderSettings>()
        .map(|s| *s)
        .unwrap_or_default();
    let view = WallProbeView {
        position: compute_camera_position(&camera).into(),
        forward: compute_camera_direction(&camera).into(),
        right: compute_camera_right(&camera).into(),
        up: compute_camera_up(&camera).into(),
        fov_y_radians: camera.fov_y.0.to_radians(),
        viewport_size_px: viewport_size,
    };

    let flames: Vec<ProbedFlame> = world
        .query_flames()
        .into_iter()
        .filter_map(|entity| {
            let effect = world.get_component::<FlameEffect>(entity)?;
            let baked = world
                .get_component::<FlameBaked>(entity)
                .cloned()
                .unwrap_or_default();
            let temporal = world
                .get_component::<FlameTemporalAccum>(entity)
                .cloned()
                .unwrap_or_default();
            let report = probe_flame_wall(&effect, &baked, &view);
            Some((effect.clone(), baked, temporal, report))
        })
        .collect();
    if flames.is_empty() {
        log_warn!("wall probe dump skipped: no flame entity");
        return None;
    }

    Some(WallProbeScene {
        camera,
        settings,
        view,
        flames,
    })
}

fn write_wall_probe(scene: &WallProbeScene, viewport_size: [f32; 2], path: Option<&Path>) {
    match write_flame_wall_probe_dump(
        &scene.camera,
        &scene.settings,
        viewport_size,
        &scene.flames,
        path,
    ) {
        Ok(written) => log!("wall probe dumped to {}", written.display()),
        Err(error) => log_warn!("wall probe dump failed: {}", error),
    }
}

fn write_field_traces(scene: &WallProbeScene, path: Option<&Path>) {
    match write_flame_field_traces(&scene.view, &scene.flames, path) {
        Ok(paths) => {
            for written in paths {
                log!("flame field trace dumped to {}", written.display());
            }
        }
        Err(error) => log_warn!("flame field trace dump failed: {}", error),
    }
}

/// Wall-probe and field-trace dump of every flame into the default dump locations.
pub fn perform_flame_wall_probe_dump(world: &World, viewport_size: [f32; 2]) {
    let Some(scene) = probe_wall_scene(world, viewport_size) else {
        return;
    };
    write_wall_probe(&scene, viewport_size, None);
    write_field_traces(&scene, None);
}

/// The same dump written to the paths a batch run asked for, at the screenshot frame.
pub fn batch_run_flame_dump(
    world: &World,
    flame_trace_path: Option<&Path>,
    wall_probe_path: Option<&Path>,
) {
    let Some(scene) = probe_wall_scene(world, BATCH_VIEWPORT_SIZE) else {
        return;
    };
    if let Some(path) = wall_probe_path {
        write_wall_probe(&scene, BATCH_VIEWPORT_SIZE, Some(path));
    }
    if let Some(path) = flame_trace_path {
        write_field_traces(&scene, Some(path));
    }
}

pub fn flame_dump_npy_path(json_path: &Path) -> PathBuf {
    let mut npy = json_path.to_path_buf();
    if npy.extension().is_some() {
        npy.set_extension("npy");
    }
    npy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npy_path_replaces_the_extension() {
        let json = Path::new("/tmp/flame_dump_frame_0001.json");
        assert_eq!(
            flame_dump_npy_path(json),
            PathBuf::from("/tmp/flame_dump_frame_0001.npy")
        );

        let jsonl = Path::new("/tmp/flame_dump_frame_0001.jsonl");
        assert_eq!(
            flame_dump_npy_path(jsonl),
            PathBuf::from("/tmp/flame_dump_frame_0001.npy")
        );
    }
}
