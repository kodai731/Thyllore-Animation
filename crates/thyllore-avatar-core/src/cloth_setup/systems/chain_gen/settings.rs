use thyllore_scene_core::find_scalar_param;

use crate::cloth_setup::components::chain_gen::{ChainGenSettings, CHAIN_GEN_SCALAR_PARAMS};

pub fn setting_names() -> Vec<&'static str> {
    CHAIN_GEN_SCALAR_PARAMS.iter().map(|p| p.name).collect()
}

pub fn apply_setting_assignments(
    settings: &mut ChainGenSettings,
    assignments: &[(String, f32)],
) -> anyhow::Result<()> {
    for (name, value) in assignments {
        let param = find_scalar_param(&CHAIN_GEN_SCALAR_PARAMS, name).ok_or_else(|| {
            anyhow::anyhow!(
                "unknown chain generation setting \"{name}\"; valid: {}",
                setting_names().join(", ")
            )
        })?;
        (param.set)(settings, *value);
    }
    Ok(())
}
