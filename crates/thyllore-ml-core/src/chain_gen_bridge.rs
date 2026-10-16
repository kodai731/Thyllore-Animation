use cgmath::Vector3;
use thyllore_avatar_core::garment::components::chain_gen::{
    ChainGenInput, ChainGenParams, ChainRole,
};
use thyllore_avatar_core::garment::systems::chain_gen::{compute_chain_weights, place_chains};

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

pub fn run_chain_gen(
    positions: &[[f32; 3]],
    triangles: &[[u32; 3]],
    root_position: [f32; 3],
    up: [f32; 3],
    role: &str,
    joint_count: usize,
    column_count: usize,
) -> anyhow::Result<ChainGenOutput> {
    let role = role_from_str(role)?;

    let positions_vec: Vec<Vector3<f32>> = positions
        .iter()
        .map(|p| Vector3::new(p[0], p[1], p[2]))
        .collect();
    let root_position = Vector3::new(root_position[0], root_position[1], root_position[2]);
    let up = Vector3::new(up[0], up[1], up[2]);

    let input = ChainGenInput {
        positions: &positions_vec,
        triangles,
        root_position,
        up,
    };

    let params = ChainGenParams {
        role,
        joint_count,
        column_count,
        root_blend_width: None,
    };

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
                        thyllore_avatar_core::garment::components::chain_gen::InfluenceTarget::Root => "root".to_string(),
                        thyllore_avatar_core::garment::components::chain_gen::InfluenceTarget::Joint { chain, joint } => {
                            let chain = &chains_out[chain];
                            chain.joints[joint].name.clone()
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

    #[test]
    fn straight_strand_single_chain() {
        let positions: [[f32; 3]; 4] = [
            [0.0, 1.0, 0.0],
            [-0.5, 0.5, 0.0],
            [0.5, 0.5, 0.0],
            [0.0, 0.0, 0.0],
        ];
        let triangles: [[u32; 3]; 2] = [[0, 1, 2], [1, 3, 2]];
        let root_position = [0.0, 1.0, 0.0];
        let up = [0.0, -1.0, 0.0];

        let output = run_chain_gen(&positions, &triangles, root_position, up, "hair", 2, 1)
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
}
