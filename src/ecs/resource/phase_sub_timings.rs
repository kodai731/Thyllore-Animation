use std::collections::HashMap;

#[derive(Default)]
pub struct PhaseSubTimings {
    pub phases: HashMap<&'static str, HashMap<String, f32>>,
}

crate::startup_resource!(PhaseSubTimings, CoreResources);
