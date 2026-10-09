use crate::ecs::systems::{
    debug_actions_json, resolve_engine_cli_overrides, run_sequence_analyze_from_args,
    EngineCliOverrides, BATCH_LIST_DEBUG_ACTIONS_FLAG,
};
use crate::hooks::bootstrap::ResolvedBootstrap;

pub struct AppConfig {
    pub overrides: EngineCliOverrides,
    pub bootstrap: ResolvedBootstrap,
    pub is_batch_mode: bool,
}

pub fn from_args(args: Vec<String>) -> anyhow::Result<Option<AppConfig>> {
    if args.iter().any(|a| a == BATCH_LIST_DEBUG_ACTIONS_FLAG) {
        println!("{}", debug_actions_json());
        return Ok(None);
    }

    if let Some(result) = run_sequence_analyze_from_args(args.clone()) {
        result?;
        return Ok(None);
    }

    let resolved = resolve_engine_cli_overrides(&args)
        .and_then(|overrides| Ok((overrides, ResolvedBootstrap::resolve(&args)?)));
    let (overrides, bootstrap) = match resolved {
        Ok(resolved) => resolved,
        Err(e) => {
            println!(
                "{}",
                serde_json::json!({"ok": false, "error": e.to_string()})
            );
            std::process::exit(1);
        }
    };

    let is_batch_mode = overrides.batch_run.is_some();

    Ok(Some(AppConfig {
        overrides,
        bootstrap,
        is_batch_mode,
    }))
}
