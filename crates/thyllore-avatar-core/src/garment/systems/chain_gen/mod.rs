mod columns;
mod frame;
mod placement;
mod root_band;
mod settings;
mod weights;

#[cfg(test)]
mod tests;

pub use columns::component_labels;
pub use placement::{place_chains, place_chains_with_strands};
pub use root_band::{hair_root_band, RootBand};
pub use settings::{apply_setting_assignments, setting_names};
pub use weights::compute_chain_weights;

pub(crate) use frame::{attachment_or_target, collect_target_vertices, progress_of};
pub(crate) use root_band::widest_slice;
