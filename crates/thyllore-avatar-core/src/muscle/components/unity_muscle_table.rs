use std::sync::OnceLock;

use serde::Deserialize;

use crate::humanoid::components::role::HumanoidRole;

#[derive(Clone, Debug)]
pub struct UnityMuscleTable {
    pub muscles: Vec<UnityMuscle>,
}

#[derive(Clone, Debug)]
pub struct UnityMuscle {
    pub index: usize,
    pub name: String,
    pub attribute: String,
    pub role: HumanoidRole,
    pub axis: usize,
    pub sign: f32,
    pub min: f32,
    pub max: f32,
    pub rest: f32,
}

#[derive(Deserialize)]
struct RawTable {
    muscle: Vec<RawMuscle>,
}

#[derive(Deserialize)]
struct RawMuscle {
    index: usize,
    name: String,
    attribute: String,
    role: String,
    axis: String,
    sign: i8,
    min: f64,
    max: f64,
    rest: f64,
}

fn load() -> UnityMuscleTable {
    let toml_str = include_str!("../../../data/unity_muscles.toml");
    let raw: RawTable = toml::from_str(toml_str).expect("failed to parse unity muscles TOML");

    let muscles = raw
        .muscle
        .into_iter()
        .map(|raw| {
            let role = HumanoidRole::from_unity_name(&raw.role)
                .unwrap_or_else(|| panic!("unknown muscle role: {}", raw.role));
            let axis = match raw.axis.as_str() {
                "x" => 0,
                "y" => 1,
                "z" => 2,
                other => panic!("unknown muscle axis: {}", other),
            };
            UnityMuscle {
                index: raw.index,
                name: raw.name,
                attribute: raw.attribute,
                role,
                axis,
                sign: f32::from(raw.sign),
                min: raw.min as f32,
                max: raw.max as f32,
                rest: raw.rest as f32,
            }
        })
        .collect();

    UnityMuscleTable { muscles }
}

pub fn default_unity_muscle_table() -> &'static UnityMuscleTable {
    static TABLE: OnceLock<UnityMuscleTable> = OnceLock::new();
    TABLE.get_or_init(load)
}
