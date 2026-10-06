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
    pub role: HumanoidRole,
    pub dof: usize,
    pub min: f32,
    pub max: f32,
    pub rest: f32,
    pub slopes: Vec<MuscleSlope>,
}

#[derive(Clone, Debug)]
pub struct MuscleSlope {
    pub role: HumanoidRole,
    pub axis: usize,
    pub positive: f32,
    pub negative: f32,
}

#[derive(Deserialize)]
struct RawTable {
    muscle: Vec<RawMuscle>,
}

#[derive(Deserialize)]
struct RawMuscle {
    index: usize,
    name: String,
    role: String,
    dof: usize,
    min: f64,
    max: f64,
    rest: f64,
    slopes: Vec<RawSlope>,
}

#[derive(Deserialize)]
struct RawSlope {
    role: String,
    axis: String,
    positive: f64,
    negative: f64,
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
            let slopes = raw
                .slopes
                .into_iter()
                .map(|rs| MuscleSlope {
                    role: HumanoidRole::from_unity_name(&rs.role)
                        .unwrap_or_else(|| panic!("unknown slope role: {}", rs.role)),
                    axis: "xyz"
                        .find(rs.axis.chars().next().unwrap())
                        .unwrap_or_else(|| panic!("unknown slope axis: {}", rs.axis)),
                    positive: rs.positive as f32,
                    negative: rs.negative as f32,
                })
                .collect();
            UnityMuscle {
                index: raw.index,
                name: raw.name,
                role,
                dof: raw.dof,
                min: raw.min as f32,
                max: raw.max as f32,
                rest: raw.rest as f32,
                slopes,
            }
        })
        .collect();

    UnityMuscleTable { muscles }
}

pub fn default_unity_muscle_table() -> &'static UnityMuscleTable {
    static TABLE: OnceLock<UnityMuscleTable> = OnceLock::new();
    TABLE.get_or_init(load)
}
