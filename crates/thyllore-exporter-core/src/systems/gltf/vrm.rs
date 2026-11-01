use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use cgmath::{Matrix4, Quaternion, SquareMatrix, Vector3};
use gltf::binary::Glb;
use gltf::json;

use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::vrchat::sidecar::AvatarSidecar;
use thyllore_avatar_core::vrchat::sidecar_spring::rest_frames_from_globals;
use thyllore_avatar_core::vrm::{build_vrm_extensions, VrmInput, VrmMeta};

use crate::systems::gltf::extensions::merge_extension;
use crate::systems::gltf::glb::glb_bytes;

const METRES_PER_GLTF_UNIT: f32 = 1.0;

/// A `.vrm` from a GLB whose nodes carry the sidecar's bone names: the standard VRM layer and the
/// `THYLLORE_avatar` layer are merged into the root extensions. Writing twice gives the same bytes.
pub fn write_vrm(glb_bytes_in: &[u8], sidecar: &AvatarSidecar, meta: &VrmMeta) -> Result<Vec<u8>> {
    let glb = Glb::from_slice(glb_bytes_in).map_err(|e| anyhow!("Failed to parse GLB: {:?}", e))?;
    let mut root: json::Root = json::Root::from_slice(&glb.json)
        .map_err(|e| anyhow!("Failed to parse glTF JSON: {:?}", e))?;
    let bin = glb.bin.map(|b| b.into_owned()).unwrap_or_default();

    let skeleton = NodeSkeleton::from_nodes(&root.nodes)?;
    let frames =
        rest_frames_from_globals(&skeleton.globals, &skeleton.bones, METRES_PER_GLTF_UNIT)?;
    let extensions = build_vrm_extensions(VrmInput {
        sidecar,
        bones: &skeleton.bones,
        frames: &frames,
        node_of_bone: &skeleton.node_of_bone,
        meta,
    })?;
    for (name, value) in extensions.entries {
        merge_extension(&mut root, name, value);
    }

    glb_bytes(&root, bin)
}

/// Every glTF node as a bone in node order, with its rest global transform in glTF units.
struct NodeSkeleton {
    bones: Vec<BoneInput>,
    globals: Vec<Matrix4<f32>>,
    node_of_bone: BTreeMap<String, u32>,
}

impl NodeSkeleton {
    fn from_nodes(nodes: &[json::scene::Node]) -> Result<Self> {
        let parents = parent_of_each_node(nodes);
        let globals = global_transforms(nodes, &parents);

        let mut node_of_bone = BTreeMap::new();
        for (index, node) in nodes.iter().enumerate() {
            let Some(name) = &node.name else { continue };
            if node_of_bone.insert(name.clone(), index as u32).is_some() {
                return Err(anyhow!("glTF node name {name:?} is not unique"));
            }
        }

        let bones = nodes
            .iter()
            .enumerate()
            .map(|(index, node)| BoneInput {
                name: node.name.clone().unwrap_or_default(),
                parent: parents[index],
                rest_position: (globals[index].w.truncate() * METRES_PER_GLTF_UNIT).into(),
            })
            .collect();

        Ok(Self {
            bones,
            globals,
            node_of_bone,
        })
    }
}

fn parent_of_each_node(nodes: &[json::scene::Node]) -> Vec<Option<usize>> {
    let mut parents = vec![None; nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        for child in node.children.iter().flatten() {
            if let Some(slot) = parents.get_mut(child.value()) {
                *slot = Some(index);
            }
        }
    }
    parents
}

fn global_transforms(nodes: &[json::scene::Node], parents: &[Option<usize>]) -> Vec<Matrix4<f32>> {
    let locals: Vec<Matrix4<f32>> = nodes.iter().map(local_transform).collect();
    let mut globals: Vec<Option<Matrix4<f32>>> = vec![None; nodes.len()];
    for index in 0..nodes.len() {
        resolve_global(index, parents, &locals, &mut globals);
    }
    globals
        .into_iter()
        .map(|global| global.unwrap_or_else(Matrix4::identity))
        .collect()
}

fn resolve_global(
    index: usize,
    parents: &[Option<usize>],
    locals: &[Matrix4<f32>],
    globals: &mut [Option<Matrix4<f32>>],
) -> Matrix4<f32> {
    if let Some(global) = globals[index] {
        return global;
    }
    let global = match parents[index] {
        Some(parent) => resolve_global(parent, parents, locals, globals) * locals[index],
        None => locals[index],
    };
    globals[index] = Some(global);
    global
}

fn local_transform(node: &json::scene::Node) -> Matrix4<f32> {
    if let Some(matrix) = node.matrix {
        return Matrix4::from(matrix_columns(matrix));
    }
    let translation = Vector3::from(node.translation.unwrap_or([0.0; 3]));
    let [x, y, z, w] = node.rotation.unwrap_or_default().0;
    let scale = node.scale.unwrap_or([1.0; 3]);
    Matrix4::from_translation(translation)
        * Matrix4::from(Quaternion::new(w, x, y, z))
        * Matrix4::from_nonuniform_scale(scale[0], scale[1], scale[2])
}

fn matrix_columns(flat: [f32; 16]) -> [[f32; 4]; 4] {
    let mut columns = [[0.0; 4]; 4];
    for (column, values) in columns.iter_mut().enumerate() {
        values.copy_from_slice(&flat[column * 4..column * 4 + 4]);
    }
    columns
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use cgmath::{InnerSpace, Matrix4, Vector3};
    use thyllore_anim_core::spring_bone::{ColliderShape, JointForce, VrmJointForce};
    use thyllore_anim_core::Skeleton;
    use thyllore_avatar_core::stats::components::stats::AvatarStats;
    use thyllore_avatar_core::vrchat::sidecar::{
        AvatarSidecar, SidecarBlink, SIDECAR_SCHEMA_VERSION,
    };
    use thyllore_avatar_core::vrchat::sidecar_spring::{
        SidecarCollider, SidecarColliderGroup, SidecarColliderOrigin, SidecarColliderShape,
        SidecarCollisionMode, SidecarJointForce, SidecarSpring, SidecarSpringChain,
        SidecarSpringJoint,
    };
    use thyllore_avatar_core::vrm::{THYLLORE_AVATAR, VRMC_SPRING_BONE, VRMC_VRM};
    use thyllore_importer_core::gltf::spring_bone_extension::parse_vrmc_spring_bone;

    use super::*;
    use crate::systems::gltf::minimal_json::build_minimal_gltf_json;

    fn source_glb() -> Vec<u8> {
        let mut skeleton = Skeleton::new("test");
        let hips = skeleton.add_bone("Hips", None);
        let head = skeleton.add_bone("Head", Some(hips));
        let hair1 = skeleton.add_bone("Hair1", Some(head));
        let hair2 = skeleton.add_bone("Hair2", Some(hair1));
        skeleton.add_bone("Hair3", Some(hair2));
        let heights = [1.0, 0.5, 0.1, -0.1, -0.1];
        for (bone, height) in skeleton.bones.iter_mut().zip(heights) {
            bone.local_transform = Matrix4::from_translation(Vector3::new(0.0, height, 0.0));
        }
        let root = build_minimal_gltf_json(&skeleton).expect("minimal glTF");
        glb_bytes(&root, Vec::new()).expect("GLB bytes")
    }

    fn joint(bone: &str) -> SidecarSpringJoint {
        SidecarSpringJoint {
            bone: bone.to_string(),
            force: SidecarJointForce::RestRelative {
                bending_relaxation: 0.06,
                gravity_scale: 1.0,
            },
            drag_force: 0.4,
            gravity_dir: [0.0, -1.0, 0.0],
            hit_radius: 0.02,
        }
    }

    fn sidecar() -> AvatarSidecar {
        let spring = SidecarSpring {
            collision_mode: SidecarCollisionMode::VrmCompatible,
            colliders: vec![SidecarCollider {
                name: "Head#0".to_string(),
                bone: "Head".to_string(),
                offset: [0.0, 0.05, 0.02],
                shape: SidecarColliderShape::Capsule,
                radius: 0.08,
                tail: Some([0.0, 0.15, 0.02]),
                origin: SidecarColliderOrigin::Authored,
            }],
            collider_groups: vec![SidecarColliderGroup {
                name: "head".to_string(),
                colliders: vec!["Head#0".to_string()],
            }],
            chains: vec![SidecarSpringChain {
                name: "hair".to_string(),
                center: Some("Head".to_string()),
                collider_groups: vec!["head".to_string()],
                joints: vec![joint("Hair1"), joint("Hair2"), joint("Hair3")],
                tail: Some("Hair3".to_string()),
                enabled: true,
                origin: None,
            }],
        };
        AvatarSidecar {
            schema: SIDECAR_SCHEMA_VERSION,
            humanoid: [("Hips", "Hips"), ("Head", "Head")]
                .into_iter()
                .map(|(role, bone)| (role.to_string(), bone.to_string()))
                .collect(),
            visemes: BTreeMap::new(),
            blink: SidecarBlink {
                both: None,
                left: None,
                right: None,
            },
            expression_mesh: None,
            expressions: vec![],
            spring,
            materials: BTreeMap::new(),
            submesh_roles: vec![],
            hidden_triangles: vec![],
            stats: AvatarStats::default(),
        }
    }

    fn root_extensions(vrm_bytes: &[u8]) -> (json::Root, serde_json::Value) {
        let glb = Glb::from_slice(vrm_bytes).expect("GLB parses");
        let root = json::Root::from_slice(&glb.json).expect("glTF parses");
        let value: serde_json::Value = serde_json::from_slice(&glb.json).expect("JSON parses");
        (root, value["extensions"].clone())
    }

    #[test]
    fn vrm_carries_the_three_extensions_and_lists_them() {
        let meta = VrmMeta::new("test avatar", "tester");
        let vrm_bytes = write_vrm(&source_glb(), &sidecar(), &meta).expect("vrm");
        let (root, extensions) = root_extensions(&vrm_bytes);

        assert_eq!(
            root.extensions_used,
            [VRMC_VRM, VRMC_SPRING_BONE, THYLLORE_AVATAR]
        );
        assert!(root.extensions_required.is_empty());
        assert_eq!(
            extensions[VRMC_VRM]["humanoid"]["humanBones"]["hips"]["node"],
            0
        );
        assert_eq!(
            extensions[VRMC_VRM]["humanoid"]["humanBones"]["head"]["node"],
            1
        );
        assert_eq!(extensions[VRMC_VRM]["meta"]["name"], "test avatar");
        let restored: AvatarSidecar =
            serde_json::from_value(extensions[THYLLORE_AVATAR].clone()).expect("sidecar");
        assert_eq!(restored, sidecar());
    }

    #[test]
    fn importer_reads_the_spring_bone_from_the_vrm() {
        let meta = VrmMeta::new("test avatar", "tester");
        let vrm_bytes = write_vrm(&source_glb(), &sidecar(), &meta).expect("vrm");
        let (_, extensions) = root_extensions(&vrm_bytes);

        let setup = parse_vrmc_spring_bone(&extensions[VRMC_SPRING_BONE], &Some).expect("parses");

        let collider = &setup.colliders[0];
        assert_eq!(collider.bone_id, 1);
        assert!((collider.offset - Vector3::new(0.0, 0.05, 0.02)).magnitude() < 1e-6);
        assert!(
            matches!(collider.shape, ColliderShape::Capsule { radius, .. } if (radius - 0.08).abs() < 1e-6)
        );
        let chain = &setup.chains[0];
        assert_eq!(chain.center_bone_id, Some(1));
        assert_eq!(chain.tail_bone_id, Some(4));
        assert_eq!(chain.joints.len(), 2);
        let strand_length = 0.2;
        assert!(matches!(
            chain.joints[0].force,
            JointForce::Vrm(VrmJointForce { stiffness, gravity_power })
                if (stiffness - 0.06 / strand_length).abs() < 1e-5 && gravity_power == 0.0
        ));
    }

    #[test]
    fn writing_twice_is_idempotent() {
        let meta = VrmMeta::new("test avatar", "tester");
        let once = write_vrm(&source_glb(), &sidecar(), &meta).expect("vrm");
        let twice = write_vrm(&once, &sidecar(), &meta).expect("vrm again");

        assert_eq!(once, twice);
    }

    #[test]
    fn sidecar_bone_missing_from_the_glb_is_an_error() {
        let meta = VrmMeta::new("test avatar", "tester");
        let mut sidecar = sidecar();
        sidecar.spring.chains[0].joints[0].bone = "Ghost".to_string();

        let error = write_vrm(&source_glb(), &sidecar, &meta).expect_err("missing node");

        assert!(error.to_string().contains("Ghost"));
    }
}
