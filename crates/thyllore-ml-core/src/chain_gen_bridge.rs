use cgmath::Vector3;
use thyllore_avatar_core::cloth_setup::components::chain_gen::{
    chain_name, ChainGenInput, ChainGenParams, ChainGenSettings, ChainRole, InfluenceTarget,
    SkinSurface, CHAIN_GEN_SCALAR_PARAMS, CHAIN_GEN_UI_PARAMS,
};
use thyllore_avatar_core::cloth_setup::components::submesh_role::SubmeshSelector;
use thyllore_avatar_core::cloth_setup::systems::chain_gen::hair_head_capsule;
use thyllore_avatar_core::cloth_setup::systems::chain_gen::{
    apply_setting_assignments, compute_chain_weights, place_chains_with_strands,
};
use thyllore_avatar_core::cloth_setup::systems::joint_radius::joint_half_thicknesses;
use thyllore_model_core::BoneId;

use crate::collider_bridge::FitSkin;
use crate::param_rows::{param_rows, ParamRow};

#[derive(serde::Serialize)]
pub struct ChainGenOutput {
    pub chains: Vec<ChainOut>,
    pub weights: Vec<WeightOut>,
    /// The capsule a hair role fits around the head, in the same space as the positions
    pub head_capsule: Option<CapsuleOut>,
    /// The garment vertices the skin covers, marked or measured
    pub covered: Vec<u32>,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct CapsuleOut {
    pub bottom: [f32; 3],
    pub top: [f32; 3],
    pub radius: f32,
}

#[derive(serde::Serialize)]
pub struct ChainOut {
    pub column: usize,
    pub name: String,
    pub joints: Vec<JointOut>,
    pub tail: [f32; 3],
    pub vertices: Vec<u32>,
}

#[derive(serde::Serialize)]
pub struct JointOut {
    pub name: String,
    pub head: [f32; 3],
    pub hit_radius: f32,
}

#[derive(serde::Serialize)]
pub struct WeightOut {
    pub vertex: u32,
    pub influences: Vec<(WeightTarget, f32)>,
}

/// `"root"`, a generated joint name, or a skin bone by index for a vertex the skin covers.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(untagged)]
pub enum WeightTarget {
    Name(String),
    Bone { bone: BoneId },
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
    apply_setting_assignments(&mut settings, pairs)?;
    Ok(settings)
}

pub(crate) fn role_from_str(role: &str) -> anyhow::Result<ChainRole> {
    match role {
        "hair" => Ok(ChainRole::Hair),
        "garment" | "skirt" => Ok(ChainRole::Garment),
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

fn skin_surface_from_fit_skin(skin: &FitSkin) -> SkinSurface {
    SkinSurface {
        positions: to_vectors(&skin.positions),
        triangles: skin.triangles.clone(),
        influences: skin
            .influences
            .iter()
            .map(|influences| {
                influences
                    .iter()
                    .map(|&(bone, weight)| (bone as BoneId, weight))
                    .collect()
            })
            .collect(),
    }
}

/// The skin meshes the chains attach to, the bone the chains hang from (a bone-list index) and the
/// garment vertices the user marked as covered.
pub struct SkinAttachment<'a> {
    pub skins: &'a [FitSkin],
    pub root_bone: usize,
    pub covered_vertices: &'a [u32],
}

pub fn run_chain_gen(
    positions: &[[f32; 3]],
    triangles: &[[u32; 3]],
    root_position: [f32; 3],
    role: &str,
    target: &str,
    settings: ChainGenSettings,
    attachment: SkinAttachment,
    tex_coords: &[[f32; 2]],
) -> anyhow::Result<ChainGenOutput> {
    let role = role_from_str(role)?;
    let target: SubmeshSelector = target.parse().map_err(anyhow::Error::msg)?;

    let positions_vec = to_vectors(positions);
    let skins: Vec<SkinSurface> = attachment
        .skins
        .iter()
        .map(skin_surface_from_fit_skin)
        .collect();
    let input = ChainGenInput {
        positions: &positions_vec,
        triangles,
        root_position: Vector3::new(root_position[0], root_position[1], root_position[2]),
        root_bone: attachment.root_bone as BoneId,
        skins: &skins,
        tex_coords,
        covered_vertices: attachment.covered_vertices,
    };
    let params = ChainGenParams {
        role,
        target,
        settings,
    };

    let head_capsule = hair_head_capsule(&input, &params).map(|capsule| CapsuleOut {
        bottom: vector3_to_array(capsule.bottom),
        top: vector3_to_array(capsule.top),
        radius: capsule.radius,
    });
    let placement = place_chains_with_strands(&input, &params)?;
    let chains = placement.chains;
    let weights = compute_chain_weights(&input, &params, &chains, &placement.covered);
    let half_thicknesses = joint_half_thicknesses(&positions_vec, &chains, &weights);

    let chains_out: Vec<ChainOut> = chains
        .iter()
        .zip(&half_thicknesses)
        .map(|(chain, radii)| ChainOut {
            column: chain.column,
            name: chain_name(&params, chain.column),
            joints: chain
                .joints
                .iter()
                .zip(radii)
                .map(|(joint, &hit_radius)| JointOut {
                    name: joint.name.clone(),
                    head: vector3_to_array(joint.head),
                    hit_radius,
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
                    let target = match target {
                        InfluenceTarget::Root => WeightTarget::Name("root".to_string()),
                        InfluenceTarget::Joint { chain, joint } => {
                            WeightTarget::Name(chains_out[chain].joints[joint].name.clone())
                        }
                        InfluenceTarget::Bone(bone) => WeightTarget::Bone { bone },
                    };
                    (target, weight)
                })
                .collect(),
        })
        .collect();

    Ok(ChainGenOutput {
        chains: chains_out,
        weights: weights_out,
        head_capsule,
        covered: placement.covered.iter().map(|c| c.vertex).collect(),
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

        let head = FitSkin {
            positions: vec![
                [0.2, 0.1, 0.0],
                [-0.2, 0.1, 0.0],
                [0.0, 0.1, 0.2],
                [0.0, 0.1, -0.2],
            ],
            influences: vec![vec![(0, 1.0)]; 4],
            triangles: Vec::new(),
        };

        let output = run_chain_gen(
            &positions,
            &triangles,
            [0.0, 0.0, 0.0],
            "hair",
            "strip/hair",
            settings,
            SkinAttachment {
                skins: std::slice::from_ref(&head),
                root_bone: 0,
                covered_vertices: &[],
            },
            &[],
        )
        .expect("chain generation should succeed");

        assert_eq!(output.chains.len(), 1, "should have exactly 1 chain");
        let chain = &output.chains[0];
        assert_eq!(chain.joints.len(), 2, "should have 2 joints");
        assert!(chain.vertices.len() >= 3, "should cover the mesh vertices");
        assert!(chain
            .joints
            .iter()
            .all(|joint| joint.hit_radius.is_finite()));
        let capsule = output
            .head_capsule
            .expect("a hair role fits a head capsule");
        assert!(capsule.radius > 0.0);

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
