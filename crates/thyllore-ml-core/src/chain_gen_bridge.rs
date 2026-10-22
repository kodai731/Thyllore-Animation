use cgmath::Vector3;
use thyllore_avatar_core::garment::components::chain_gen::{
    ChainGenInput, ChainGenParams, ChainGenSettings, ChainRole, InfluenceTarget,
    CHAIN_GEN_SCALAR_PARAMS, CHAIN_GEN_UI_PARAMS,
};
use thyllore_avatar_core::garment::systems::chain_gen::{compute_chain_weights, place_chains};

use crate::param_rows::{apply_scalar_pairs, param_rows, ParamRow};

#[derive(serde::Serialize)]
pub struct ChainGenOutput {
    pub chains: Vec<ChainOut>,
    pub weights: Vec<WeightOut>,
}

#[derive(serde::Serialize)]
pub struct ChainOut {
    pub column: usize,
    pub joints: Vec<JointOut>,
    pub tail: [f32; 3],
    pub vertices: Vec<u32>,
}

#[derive(serde::Serialize)]
pub struct JointOut {
    pub name: String,
    pub head: [f32; 3],
}

#[derive(serde::Serialize)]
pub struct WeightOut {
    pub vertex: u32,
    pub influences: Vec<(String, f32)>,
}

pub fn chain_gen_params() -> Vec<ParamRow> {
    param_rows(
        &CHAIN_GEN_UI_PARAMS,
        &CHAIN_GEN_SCALAR_PARAMS,
        &ChainGenSettings::default(),
    )
}

pub fn settings_from_pairs(pairs: &[(String, f32)]) -> anyhow::Result<ChainGenSettings> {
    let mut settings = ChainGenSettings::default();
    apply_scalar_pairs(&mut settings, &CHAIN_GEN_SCALAR_PARAMS, pairs)?;
    Ok(settings)
}

fn role_from_str(role: &str) -> anyhow::Result<ChainRole> {
    match role {
        "hair" => Ok(ChainRole::Hair),
        "skirt" => Ok(ChainRole::Skirt),
        _ => anyhow::bail!("chain generation role must be \"hair\" or \"skirt\", got \"{role}\""),
    }
}

fn vector3_to_array(v: Vector3<f32>) -> [f32; 3] {
    [v.x, v.y, v.z]
}

fn to_vectors(points: &[[f32; 3]]) -> Vec<Vector3<f32>> {
    points
        .iter()
        .map(|p| Vector3::new(p[0], p[1], p[2]))
        .collect()
}

pub fn run_chain_gen(
    positions: &[[f32; 3]],
    triangles: &[[u32; 3]],
    root_position: [f32; 3],
    role: &str,
    settings: ChainGenSettings,
    attachment_positions: &[[f32; 3]],
    tex_coords: &[[f32; 2]],
) -> anyhow::Result<ChainGenOutput> {
    let role = role_from_str(role)?;

    let positions_vec = to_vectors(positions);
    let attachment_vec = to_vectors(attachment_positions);
    let input = ChainGenInput {
        positions: &positions_vec,
        triangles,
        root_position: Vector3::new(root_position[0], root_position[1], root_position[2]),
        attachment_positions: &attachment_vec,
        tex_coords,
    };
    let params = ChainGenParams { role, settings };

    let chains = place_chains(&input, &params)?;
    let weights = compute_chain_weights(&input, &params, &chains);

    let chains_out: Vec<ChainOut> = chains
        .iter()
        .map(|chain| ChainOut {
            column: chain.column,
            joints: chain
                .joints
                .iter()
                .map(|joint| JointOut {
                    name: joint.name.clone(),
                    head: vector3_to_array(joint.head),
                })
                .collect(),
            tail: vector3_to_array(chain.tail),
            vertices: chain.vertices.clone(),
        })
        .collect();

    let weights_out: Vec<WeightOut> = weights
        .into_iter()
        .map(|vi| WeightOut {
            vertex: vi.vertex,
            influences: vi
                .influences
                .into_iter()
                .map(|(target, weight)| {
                    let name = match target {
                        InfluenceTarget::Root => "root".to_string(),
                        InfluenceTarget::Joint { chain, joint } => {
                            chains_out[chain].joints[joint].name.clone()
                        }
                    };
                    (name, weight)
                })
                .collect(),
        })
        .collect();

    Ok(ChainGenOutput {
        chains: chains_out,
        weights: weights_out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::param_rows::ParamDefault;

    #[test]
    fn straight_strand_single_chain() {
        let positions: [[f32; 3]; 4] = [
            [0.0, -0.1, 0.0],
            [-0.5, -0.5, 0.0],
            [0.5, -0.5, 0.0],
            [0.0, -1.0, 0.0],
        ];
        let triangles: [[u32; 3]; 2] = [[0, 1, 2], [1, 3, 2]];
        let settings =
            settings_from_pairs(&[("joint_count".into(), 2.0), ("column_count".into(), 1.0)])
                .unwrap();

        let head: [[f32; 3]; 4] = [
            [0.2, 0.1, 0.0],
            [-0.2, 0.1, 0.0],
            [0.0, 0.1, 0.2],
            [0.0, 0.1, -0.2],
        ];

        let output = run_chain_gen(
            &positions,
            &triangles,
            [0.0, 0.0, 0.0],
            "hair",
            settings,
            &head,
            &[],
        )
        .expect("chain generation should succeed");

        assert_eq!(output.chains.len(), 1, "should have exactly 1 chain");
        let chain = &output.chains[0];
        assert_eq!(chain.joints.len(), 2, "should have 2 joints");
        assert!(chain.vertices.len() >= 3, "should cover the mesh vertices");

        for weight in &output.weights {
            let sum: f32 = weight.influences.iter().map(|(_, w)| *w).sum();
            assert!(
                (sum - 1.0).abs() < 1e-5,
                "weight sum should be 1.0, got {} for vertex {}",
                sum,
                weight.vertex
            );
        }
    }

    #[test]
    fn unknown_setting_is_rejected() {
        assert!(settings_from_pairs(&[("joints".into(), 2.0)]).is_err());
    }

    #[test]
    fn params_table_carries_defaults() {
        let params = chain_gen_params();
        let joint_count = params.iter().find(|p| p.name == "joint_count").unwrap();
        assert_eq!(joint_count.default, ParamDefault::Scalar(4.0));
        assert_eq!(joint_count.label, "Joint Count");
    }
}
