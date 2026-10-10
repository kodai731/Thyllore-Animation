use std::collections::HashMap;

#[derive(Default)]
pub struct PhaseSubTimings {
    pub phases: HashMap<&'static str, HashMap<String, f32>>,
}
