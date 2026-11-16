mod effect;
mod flame;
mod lightning;
mod registration;
mod water;
mod wind;

#[cfg(test)]
use flame::{
    flame_bounds_corners, flame_effective_optical_depth, flame_preset_params,
    flame_shader_specialization, flame_ui_params, pack_flame_ubo,
};
#[cfg(test)]
use lightning::{lightning_preset_names, lightning_preset_params, pack_lightning_ubo};
#[cfg(test)]
use water::water_ui_params;
#[cfg(test)]
use wind::{pack_wind_ubo, wind_preset_params};

#[cfg(test)]
mod tests;
