mod card_strands;
mod columns;
mod frame;
mod head_capsule;
mod placement;
mod root_band;
mod settings;
mod strand_merge;
mod strand_mode;
mod weights;

#[cfg(test)]
mod tests;

pub use columns::component_labels;
pub use head_capsule::hair_head_capsule;
pub use placement::{place_chains, place_chains_with_strands};
pub use root_band::{hair_root_band, RootBand};
pub use settings::{apply_setting_assignments, setting_names};
pub use weights::compute_chain_weights;
