use crate::{BoneId, Skeleton};

use super::origin::ChainOrigin;
use super::{SpringBoneSetup, SpringChain, SpringChainId, SpringJointParam};

fn build_chain_joints(
    root_bone_id: BoneId,
    chain_length: u32,
    skeleton: &Skeleton,
) -> Vec<SpringJointParam> {
    let mut joints = Vec::new();
    let mut current_bone = root_bone_id;

    for _ in 0..chain_length {
        joints.push(SpringJointParam {
            bone_id: current_bone,
            ..Default::default()
        });

        let children: Vec<BoneId> = skeleton
            .bones
            .iter()
            .filter(|b| b.parent_id == Some(current_bone))
            .map(|b| b.id)
            .collect();

        if let Some(&first_child) = children.first() {
            current_bone = first_child;
        } else {
            break;
        }
    }

    joints
}

pub fn add_chain(
    setup: &mut SpringBoneSetup,
    root_bone_id: BoneId,
    chain_length: u32,
    skeleton: &Skeleton,
) -> SpringChainId {
    let chain_id = setup.next_chain_id;
    setup.next_chain_id += 1;

    let joints = build_chain_joints(root_bone_id, chain_length, skeleton);
    let chain = SpringChain {
        id: chain_id,
        name: format!("Chain_{}", chain_id),
        joints,
        collider_group_ids: Vec::new(),
        center_bone_id: None,
        enabled: true,
        origin: ChainOrigin::Authored,
    };
    setup.chains.push(chain);

    chain_id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_linear_skeleton(bone_count: usize) -> Skeleton {
        let mut skeleton = Skeleton::new("test");
        let mut parent_id: Option<BoneId> = None;
        for i in 0..bone_count {
            let child = skeleton.add_bone(&format!("bone_{}", i), parent_id);
            parent_id = Some(child);
        }
        skeleton
    }

    #[test]
    fn add_chain_joints_follow_bone_hierarchy() {
        let skeleton = make_linear_skeleton(4);
        let mut setup = SpringBoneSetup::default();

        let chain_id = add_chain(&mut setup, 1, 3, &skeleton);

        assert_eq!(chain_id, 0);
        assert_eq!(setup.chains.len(), 1);
        let chain = &setup.chains[0];
        assert_eq!(chain.joints.len(), 3);
        assert_eq!(chain.joints[0].bone_id, 1);
        assert_eq!(chain.joints[1].bone_id, 2);
        assert_eq!(chain.joints[2].bone_id, 3);
    }

    #[test]
    fn add_chain_names_increment() {
        let skeleton = make_linear_skeleton(4);
        let mut setup = SpringBoneSetup::default();

        let id0 = add_chain(&mut setup, 1, 3, &skeleton);
        let id1 = add_chain(&mut setup, 1, 3, &skeleton);

        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
        assert_eq!(setup.chains[0].name, "Chain_0");
        assert_eq!(setup.chains[1].name, "Chain_1");
    }
}
