use serde::{Deserialize, Serialize};

use super::curve::PropertyCurve;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MorphTrack {
    pub source_mesh: String,
    pub channel: String,
    pub curve: PropertyCurve,
}

#[derive(Clone, Debug)]
pub struct MorphTrackSample {
    pub source_mesh: String,
    pub channel: String,
    pub weight: f32,
}
