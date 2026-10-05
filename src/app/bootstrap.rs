use anyhow::Result;

use crate::app::config::AppConfig;
use crate::app::App;
use crate::ecs::resource::BatchRun;
use crate::ecs::systems::{
    animation_debug_dump::{build_animation_debug_dump, resolve_animation_debug_target},
    apply_engine_overrides, batch_anim_dump_write, batch_run_report, EngineCliOverrides,
};
use crate::vulkanr::context::{CommandState, RenderTargets};

/// Applies the startup configuration: the engine's own overrides first, then every subsystem
/// that registered a `bootstrap_hook!`, without naming any of them (see `.claude/rules/hierarchy.md`).
pub fn apply_startup_overrides(app: &mut App, config: &AppConfig) -> Result<()> {
    apply_engine_overrides(
        &mut app.data.ecs_world,
        &mut app.data.ecs_assets,
        &config.overrides,
    );
    config
        .bootstrap
        .apply(&mut app.data.ecs_world, &mut app.data.ecs_assets)
}

/// GPU setup that has to wait for the startup overrides: subsystem hooks, then imgui.
pub unsafe fn finish_setup(app: &mut App, system: &mut crate::platform::System) -> Result<()> {
    let command_pool = app.resource::<CommandState>().pool.clone();
    let rrrender = app.resource::<RenderTargets>().render.clone();

    crate::app::effect_hooks::run_effect_after_overrides(
        &app.instance,
        &app.rrdevice,
        &mut app.data,
        &rrrender,
    )?;
    App::init_imgui_rendering(
        &app.instance,
        &app.rrdevice,
        &mut app.data,
        &mut system.imgui,
        &command_pool,
        &rrrender,
    )
}

pub fn finish_run(app: &App, overrides: &EngineCliOverrides, is_batch_mode: bool) {
    if let Some(ref dump_path) = overrides.anim_dump_path {
        if let Err(e) =
            batch_anim_dump_write(&app.data.ecs_world, dump_path, overrides.anim_dump_tracks)
        {
            println!(
                "{}",
                serde_json::json!({"ok": false, "error": format!("anim dump failed: {e}")})
            );
            std::process::exit(1);
        }
    }

    if let Some(ref debug_dump) = overrides.anim_debug_dump {
        let target = resolve_animation_debug_target(&app.data.ecs_world, &app.data.ecs_assets);
        let target = match target {
            Some(t) => t,
            None => {
                println!(
                    "{}",
                    serde_json::json!({"ok": false, "error": "animation debug target not found (no clip/skeleton/rig)"})
                );
                std::process::exit(1);
            }
        };
        let dump = build_animation_debug_dump(&target, &debug_dump.times, true);
        let json = serde_json::to_string_pretty(&dump).expect("json serialize");
        if let Err(e) = std::fs::write(&debug_dump.path, json) {
            println!(
                "{}",
                serde_json::json!({"ok": false, "error": format!("anim debug dump write failed: {e}")})
            );
            std::process::exit(1);
        }
    }

    if let Some(ref fbx_path) = overrides.export_fbx_path {
        let timeline_state = app
            .data
            .ecs_world
            .resource::<crate::ecs::resource::TimelineState>();
        let clip_id = timeline_state.current_clip_id;
        drop(timeline_state);
        if let Some(source_id) = clip_id {
            crate::app::features::export_actions::export_clip_fbx(
                app,
                source_id,
                std::path::Path::new(fbx_path),
            );
        }
    }

    if is_batch_mode {
        let batch = app.data.ecs_world.resource::<BatchRun>();
        let (ok, report_line) = batch_run_report(&batch);
        drop(batch);
        println!("{report_line}");
        if !ok {
            std::process::exit(1);
        }
    }
}
