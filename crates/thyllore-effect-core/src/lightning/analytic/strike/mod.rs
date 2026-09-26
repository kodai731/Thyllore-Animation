mod branch;
mod construct;
mod path;
mod segment;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

pub use construct::{
    build_lightning_segments, build_strike_segments, compute_lightning_segment_aabb,
};
pub(crate) use segment::segment_length;
pub use segment::{Segment, LIGHTNING_MAX_SEGMENTS};
