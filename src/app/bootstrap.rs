use anyhow::Result;

use crate::app::config::AppConfig;
use crate::app::App;
use crate::ecs::resource::BatchRun;
use crate::ecs::systems::{
    animation_debug_dump::write_animation_debug_dump, apply_engine_overrides,
    batch_anim_dump_write, batch_run_report, format_validation_summary, EngineCliOverrides,
};
use crate::vulkanr::context::RenderTargets;

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
    crate::hooks::external_command::start_external_command_sources(
        &system.external_command_sender(),
    );

    app.data.ecs_world.insert_resource(system.ui_fonts);

    let rrrender = app.resource::<RenderTargets>().render.clone();

    crate::app::effect_hooks::run_effect_after_overrides(
        &app.instance,
        &app.rrdevice,
        &mut app.data,
        &rrrender,
    )?;
    App::init_imgui_rendering(&app.rrdevice, &mut app.data, &mut system.imgui, &rrrender)
}

pub fn finish_run(app: &App, overrides: &EngineCliOverrides, is_batch_mode: bool) {
    let stats = thyllore_log_core::validation_stats::validation_stats_snapshot();
    let first_issue_frame = app
        .data
        .ecs_world
        .get_resource::<crate::ecs::resource::ValidationReport>()
        .and_then(|report| report.first_issue_frame);
    let summary = format_validation_summary(&stats, first_issue_frame);
    log!("{}", summary);
    println!("{summary}");

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
        if let Err(e) = write_animation_debug_dump(
            &app.data.ecs_world,
            &app.data.ecs_assets,
            std::path::Path::new(&debug_dump.path),
            &debug_dump.times,
            true,
        ) {
            println!(
                "{}",
                serde_json::json!({"ok": false, "error": format!("anim debug dump failed: {e}")})
            );
            std::process::exit(1);
        }
    }

    if let Some(ref fbx_path) = overrides.export_fbx_path {
        crate::app::features::export_actions::export_current_clip_fbx(
            app,
            std::path::Path::new(fbx_path),
        );
    }

    if is_batch_mode {
        let batch = app.data.ecs_world.resource::<BatchRun>();
        let validation = app
            .data
            .ecs_world
            .resource::<crate::ecs::resource::ValidationReport>();
        let (ok, report_line) = batch_run_report(&batch, &validation);
        drop(batch);
        drop(validation);
        println!("{report_line}");
        if !ok {
            std::process::exit(1);
        }
    }
}
