use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::asset::AssetStorage;
use crate::ecs::resource::{HelmBatchState, HelmState};
use crate::ecs::world::World;
use crate::hooks::bootstrap::BootstrapOverrides;

#[derive(Args, Debug)]
pub struct HelmOverrides {
    #[arg(long = "batch-utterance")]
    pub utterance_input: Option<PathBuf>,
    #[arg(long = "batch-utterance-out")]
    pub utterance_output: Option<PathBuf>,
}

impl BootstrapOverrides for HelmOverrides {
    const NAME: &'static str = "helm";

    fn apply(&self, world: &mut World, _assets: &mut AssetStorage) -> Result<()> {
        if !world.contains_resource::<HelmState>() {
            world.insert_resource(HelmState::default());
        }

        if let Some(input_path) = &self.utterance_input {
            let rows = HelmBatchState::from_jsonl(input_path)?;
            let rss_start_kb = read_rss_kb();
            let output_path = self
                .utterance_output
                .clone()
                .unwrap_or_else(|| PathBuf::from("log/helm_batch_results.jsonl"));
            world.insert_resource(HelmBatchState {
                rows,
                next: 0,
                out: output_path,
                results: Vec::new(),
                injected_last_frame: false,
                ui_events_before: 0,
                rss_start_kb,
                drain_frames_left: None,
                exit_code: 0,
            });
            let mut helm_state = world.resource_mut::<HelmState>();
            helm_state.mode = crate::helm::components::route::HelmMode::AllowEdit;
        }

        Ok(())
    }
}

crate::bootstrap_hook!(HelmOverrides);

fn read_rss_kb() -> usize {
    let contents = match std::fs::read_to_string("/proc/self/status") {
        Ok(c) => c,
        Err(_) => return 0,
    };
    for line in contents.lines() {
        if let Some(value) = line.strip_prefix("VmRSS:") {
            let parts: Vec<&str> = value.split_whitespace().collect();
            if let Some(v) = parts.first() {
                if let Ok(kb) = v.parse::<usize>() {
                    return kb;
                }
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn resolve_no_flags() {
        let overrides = HelmOverrides::resolve(&args(&["bin"])).unwrap();
        assert!(overrides.utterance_input.is_none());
        assert!(overrides.utterance_output.is_none());
    }

    #[test]
    fn resolve_input_only() {
        let overrides = HelmOverrides::resolve(&args(&[
            "bin",
            "--batch-utterance",
            "tests/data/batch.jsonl",
        ]))
        .unwrap();
        assert_eq!(
            overrides.utterance_input.as_deref(),
            Some(Path::new("tests/data/batch.jsonl"))
        );
        assert!(overrides.utterance_output.is_none());
    }

    #[test]
    fn resolve_input_and_output() {
        let overrides = HelmOverrides::resolve(&args(&[
            "bin",
            "--batch-utterance",
            "tests/data/batch.jsonl",
            "--batch-utterance-out",
            "output.jsonl",
        ]))
        .unwrap();
        assert_eq!(
            overrides.utterance_input.as_deref(),
            Some(Path::new("tests/data/batch.jsonl"))
        );
        assert_eq!(
            overrides.utterance_output.as_deref(),
            Some(Path::new("output.jsonl"))
        );
    }

    #[test]
    fn apply_without_flags_inserts_default_helm_state() {
        let mut world = World::new();
        let mut assets = AssetStorage::new();
        let overrides = HelmOverrides::resolve(&args(&["bin"])).unwrap();
        overrides.apply(&mut world, &mut assets).unwrap();
        assert!(world.contains_resource::<HelmState>());
    }
}
