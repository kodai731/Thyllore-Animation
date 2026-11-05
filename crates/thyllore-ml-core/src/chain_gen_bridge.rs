use std::collections::BTreeMap;

use cgmath::Vector3;
use thyllore_avatar_core::cloth_setup::components::chain_gen::{
    attachment_or_default, chain_column, chain_name, BodyCollider, ChainGenInput, ChainGenParams,
    ChainGenSettings, GeneratedChain, GivenChain, InfluenceTarget, RootAttachment, SkinSurface,
    VertexInfluences, CHAIN_GEN_SCALAR_PARAMS, CHAIN_GEN_UI_PARAMS,
};
use thyllore_avatar_core::cloth_setup::systems::chain_gen::root_head_capsule;
use thyllore_avatar_core::cloth_setup::systems::chain_gen::{
    apply_setting_assignments, compute_chain_weights, place_chains_with_strands,
};
use thyllore_avatar_core::cloth_setup::systems::collider_surface::{
    collider_surfaces, retain_torso_skin, supports_garments,
};
use thyllore_avatar_core::cloth_setup::systems::joint_radius::joint_half_thicknesses;
use thyllore_avatar_core::collider::systems::torso_volume::RoleBone;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_model_core::BoneId;

use crate::avatar_setup_bridge::BoneSpec;
use crate::collider_bridge::FitSkin;
use crate::param_rows::{param_rows, ParamRow};

#[derive(serde::Serialize)]
pub struct ChainGenOutput {
    pub chains: Vec<ChainOut>,
    pub weights: Vec<WeightOut>,
    /// The capsule a hair role fits around the head, in the same space as the positions
    pub head_capsule: Option<CapsuleOut>,
    /// The garment vertices the body covers, marked or found from the supported rings
    pub covered: Vec<u32>,
    /// The ring a garment hangs from and the bone that supports it, when the body supports one
    pub support: Option<SupportOut>,
    pub attachment: RootAttachment,
}

/// A sidecar collider in model axes relative to its bone's rest head; `tail` is absent for a sphere.
#[derive(Clone, serde::Deserialize)]
pub struct BodyColliderIn {
    pub bone: usize,
    pub head: [f32; 3],
    pub tail: Option<[f32; 3]>,
    pub radius: f32,
}

fn role_of_bone(roles: &[RoleBone], bone: BoneId) -> Option<HumanoidRole> {
    roles
        .iter()
        .find(|role_bone| role_bone.bone == bone)
        .map(|role_bone| role_bone.role)
}

/// The humanoid roles of the bone list, by Unity role name.
pub fn role_bones(humanoid_bones: &BTreeMap<String, usize>, bones: &[BoneSpec]) -> Vec<RoleBone> {
    humanoid_bones
        .iter()
        .filter_map(|(role, &bone)| {
            Some(RoleBone {
                role: HumanoidRole::from_unity_name(role)?,
                bone: bone as BoneId,
                head: Vector3::from(bones.get(bone)?.rest_position),
            })
        })
        .collect()
}

/// The colliders a garment can rest on, placed at their bones' rest heads: those on torso bones and
/// on bones without a role, never the limbs.
pub fn body_colliders(
    colliders: &[BodyColliderIn],
    roles: &[RoleBone],
    bones: &[BoneSpec],
) -> Vec<BodyCollider> {
    colliders
        .iter()
        .filter(|collider| {
            role_of_bone(roles, collider.bone as BoneId).is_none_or(supports_garments)
        })
        .filter_map(|collider| {
            let rest_head = Vector3::from(bones.get(collider.bone)?.rest_position);
            let head = rest_head + Vector3::from(collider.head);
            Some(BodyCollider {
                bone: collider.bone as BoneId,
                head,
                tail: collider
                    .tail
                    .map_or(head, |tail| rest_head + Vector3::from(tail)),
                radius: collider.radius,
            })
        })
        .collect()
}

#[derive(Clone, Copy, serde::Serialize)]
pub struct SupportOut {
    pub boundary_height: f32,
    pub bone: BoneId,
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

pub fn parse_attachment(s: &str) -> anyhow::Result<RootAttachment> {
    serde_json::from_value::<RootAttachment>(serde_json::Value::String(s.to_owned()))
        .map_err(|_| anyhow::anyhow!("attachment must be \"cap\" or \"ring\", got \"{s}\""))
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

/// The skin meshes and body colliders the chains attach to, the humanoid roles that tell limb skin
/// from torso skin, the bone the chains hang from (a bone-list index) and the garment vertices the
/// user marked as covered.
pub struct SkinAttachment<'a> {
    pub skins: &'a [FitSkin],
    pub body_colliders: &'a [BodyCollider],
    pub roles: &'a [RoleBone],
    pub root_bone: usize,
    pub covered_vertices: &'a [u32],
}

/// A chain the user assigned on the target, named with its chain name; `joints` are its joint heads followed
/// by the chain tail when the user placed its bones.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct GivenChainIn {
    pub name: String,
    pub vertices: Vec<u32>,
    #[serde(default)]
    pub joints: Option<Vec<[f32; 3]>>,
}

/// The given chains that belong to this submesh, by column; a chain named for another submesh of the
/// same mesh is left out.
pub fn given_chains(given: &[GivenChainIn], params: &ChainGenParams) -> Vec<GivenChain> {
    given
        .iter()
        .filter_map(|chain| {
            Some(GivenChain {
                column: chain_column(params, &chain.name)?,
                vertices: chain.vertices.clone(),
                joints: chain.joints.as_deref().map(to_vectors),
            })
        })
        .collect()
}

/// The names among `candidates` that `generate_chains` would give a chain of the submesh `target`.
pub fn submesh_chain_names(target: &str, candidates: &[String]) -> anyhow::Result<Vec<String>> {
    let params = ChainGenParams {
        root: None,
        attachment: RootAttachment::Ring,
        target: target.parse().map_err(anyhow::Error::msg)?,
        settings: ChainGenSettings::default(),
    };
    Ok(candidates
        .iter()
        .filter(|name| chain_column(&params, name).is_some())
        .cloned()
        .collect())
}

/// The surfaces the chains attach to: the torso part of the skins, then the body colliders.
fn attachment_surfaces(attachment: &SkinAttachment) -> Vec<SkinSurface> {
    let mut skins: Vec<SkinSurface> = attachment
        .skins
        .iter()
        .map(skin_surface_from_fit_skin)
        .collect();
    retain_torso_skin(&mut skins, |bone| role_of_bone(attachment.roles, bone));
    skins.extend(collider_surfaces(attachment.body_colliders));
    skins
}

fn chains_to_output(
    params: &ChainGenParams,
    chains: &[GeneratedChain],
    half_thicknesses: &[Vec<f32>],
) -> Vec<ChainOut> {
    chains
        .iter()
        .zip(half_thicknesses)
        .map(|(chain, radii)| ChainOut {
            column: chain.column,
            name: chain_name(params, chain.column),
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
        .collect()
}

fn weight_target(target: InfluenceTarget, chains: &[ChainOut]) -> WeightTarget {
    match target {
        InfluenceTarget::Root => WeightTarget::Name("root".to_string()),
        InfluenceTarget::Joint { chain, joint } => {
            WeightTarget::Name(chains[chain].joints[joint].name.clone())
        }
        InfluenceTarget::Bone(bone) => WeightTarget::Bone { bone },
    }
}

fn weights_to_output(weights: Vec<VertexInfluences>, chains: &[ChainOut]) -> Vec<WeightOut> {
    weights
        .into_iter()
        .map(|vertex| WeightOut {
            vertex: vertex.vertex,
            influences: vertex
                .influences
                .into_iter()
                .map(|(target, weight)| (weight_target(target, chains), weight))
                .collect(),
        })
        .collect()
}

pub fn run_chain_gen(
    positions: &[[f32; 3]],
    triangles: &[[u32; 3]],
    root_position: [f32; 3],
    target: &str,
    settings: ChainGenSettings,
    skin_attachment: SkinAttachment,
    attachment: Option<RootAttachment>,
    tex_coords: &[[f32; 2]],
    given: &[GivenChainIn],
) -> anyhow::Result<ChainGenOutput> {
    let root = role_of_bone(skin_attachment.roles, skin_attachment.root_bone as BoneId);
    let resolved_attachment = attachment_or_default(root, attachment);
    let params = ChainGenParams {
        root,
        attachment: resolved_attachment,
        target: target.parse().map_err(anyhow::Error::msg)?,
        settings,
    };
    let given_chains = given_chains(given, &params);
    let positions_vec = to_vectors(positions);
    let skins = attachment_surfaces(&skin_attachment);
    let input = ChainGenInput {
        positions: &positions_vec,
        triangles,
        root_position: Vector3::from(root_position),
        root_bone: skin_attachment.root_bone as BoneId,
        skins: &skins,
        tex_coords,
        covered_vertices: skin_attachment.covered_vertices,
        given_chains: &given_chains,
    };

    let head_capsule = root_head_capsule(&input, &params).map(|capsule| CapsuleOut {
        bottom: vector3_to_array(capsule.bottom),
        top: vector3_to_array(capsule.top),
        radius: capsule.radius,
    });
    let placement = place_chains_with_strands(&input, &params)?;
    let weights = compute_chain_weights(&input, &params, &placement.chains, &placement.covered);
    let half_thicknesses = joint_half_thicknesses(&positions_vec, &placement.chains, &weights);
    let chains = chains_to_output(&params, &placement.chains, &half_thicknesses);
    let weights = weights_to_output(weights, &chains);

    Ok(ChainGenOutput {
        chains,
        weights,
        head_capsule,
        covered: placement.covered.iter().map(|c| c.vertex).collect(),
        support: placement.support.map(|support| SupportOut {
            boundary_height: support.boundary_height,
            bone: support.bone,
        }),
        attachment: resolved_attachment,
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
            "strip/hair",
            settings,
            SkinAttachment {
                skins: std::slice::from_ref(&head),
                body_colliders: &[],
                roles: &[RoleBone {
                    role: HumanoidRole::Head,
                    bone: 0,
                    head: [0.0, 0.0, 0.0].into(),
                }],
                root_bone: 0,
                covered_vertices: &[],
            },
            Some(RootAttachment::Cap),
            &[],
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

    #[test]
    fn attachment_defaults_from_the_root_role() {
        let resolved = attachment_or_default(Some(HumanoidRole::Head), None);
        assert_eq!(resolved, RootAttachment::Cap);

        let resolved = attachment_or_default(None, None);
        assert_eq!(resolved, RootAttachment::Ring);

        let resolved = attachment_or_default(Some(HumanoidRole::Head), Some(RootAttachment::Ring));
        assert_eq!(resolved, RootAttachment::Ring);

        let resolved = attachment_or_default(Some(HumanoidRole::Hips), None);
        assert_eq!(resolved, RootAttachment::Ring);
    }
}
