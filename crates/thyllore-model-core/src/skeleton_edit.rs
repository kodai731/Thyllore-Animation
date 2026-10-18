use std::fmt;

use cgmath::{Matrix4, SquareMatrix, Vector3};

use crate::skeleton::{Bone, BoneId, Skeleton};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkeletonEditError {
    UnknownBone(BoneId),
    DuplicateName(String),
    EmptyName,
    SingularBindPose(String),
}

impl fmt::Display for SkeletonEditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownBone(id) => write!(f, "bone {id} does not exist"),
            Self::DuplicateName(name) => write!(f, "bone \"{name}\" already exists"),
            Self::EmptyName => write!(f, "a bone name must not be empty"),
            Self::SingularBindPose(name) => write!(f, "bind pose of bone \"{name}\" is singular"),
        }
    }
}

impl std::error::Error for SkeletonEditError {}

/// Where every bone of the skeleton before a removal ended up afterwards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoneRemap {
    old_to_new: Vec<Option<BoneId>>,
}

impl BoneRemap {
    pub fn map(&self, old: BoneId) -> Option<BoneId> {
        self.old_to_new.get(old as usize).copied().flatten()
    }

    pub fn old_bone_count(&self) -> usize {
        self.old_to_new.len()
    }
}

/// Result of removing a subtree: `parent` is where the subtree hung and receives its skin weights.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedBones {
    pub removed: Vec<BoneId>,
    pub parent: Option<BoneId>,
    pub remap: BoneRemap,
}

fn bind_pose(bone: &Bone) -> Result<Matrix4<f32>, SkeletonEditError> {
    bone.inverse_bind_pose
        .invert()
        .ok_or_else(|| SkeletonEditError::SingularBindPose(bone.name.clone()))
}

impl Skeleton {
    pub fn add_child_bone(
        &mut self,
        parent: BoneId,
        name: &str,
        local_offset: Vector3<f32>,
    ) -> Result<BoneId, SkeletonEditError> {
        if name.is_empty() {
            return Err(SkeletonEditError::EmptyName);
        }
        if self.bone_name_to_id.contains_key(name) {
            return Err(SkeletonEditError::DuplicateName(name.to_string()));
        }
        let parent_bone = self
            .get_bone(parent)
            .ok_or(SkeletonEditError::UnknownBone(parent))?;
        let parent_bind = bind_pose(parent_bone)?;

        let local_transform = Matrix4::from_translation(local_offset);
        let bind = parent_bind * local_transform;
        let inverse_bind_pose = bind
            .invert()
            .ok_or_else(|| SkeletonEditError::SingularBindPose(name.to_string()))?;

        let id = self.add_bone(name, Some(parent));
        let bone = self.get_bone_mut(id).expect("bone was just added");
        bone.local_transform = local_transform;
        bone.inverse_bind_pose = inverse_bind_pose;
        Ok(id)
    }

    /// Moves a bone's rest position relative to its parent; its subtree follows rigidly.
    pub fn set_bone_local_offset(
        &mut self,
        bone_id: BoneId,
        local_offset: Vector3<f32>,
    ) -> Result<(), SkeletonEditError> {
        let bone = self
            .get_bone(bone_id)
            .ok_or(SkeletonEditError::UnknownBone(bone_id))?;
        let old_bind = bind_pose(bone)?;
        let old_local_inverse = bone
            .local_transform
            .invert()
            .ok_or_else(|| SkeletonEditError::SingularBindPose(bone.name.clone()))?;

        let mut new_local = bone.local_transform;
        new_local.w = local_offset.extend(1.0);
        let new_bind = old_bind * old_local_inverse * new_local;
        let delta = new_bind * bone.inverse_bind_pose;

        for id in std::iter::once(bone_id).chain(self.collect_descendants(bone_id)) {
            let subtree_bone = self.get_bone_mut(id).expect("descendant ids are valid");
            let moved_bind = delta * bind_pose(subtree_bone)?;
            subtree_bone.inverse_bind_pose = moved_bind
                .invert()
                .ok_or_else(|| SkeletonEditError::SingularBindPose(subtree_bone.name.clone()))?;
        }
        self.get_bone_mut(bone_id)
            .expect("checked above")
            .local_transform = new_local;
        Ok(())
    }

    /// Removes a bone together with every descendant.
    pub fn remove_bone(&mut self, bone_id: BoneId) -> Result<RemovedBones, SkeletonEditError> {
        let removed = self
            .get_bone(bone_id)
            .ok_or(SkeletonEditError::UnknownBone(bone_id))?
            .clone();
        let mut removed_ids = vec![bone_id];
        removed_ids.extend(self.collect_descendants(bone_id));

        match removed.parent_id {
            Some(parent_id) => {
                let parent = self.get_bone_mut(parent_id).expect("parent id is valid");
                parent.children.retain(|&id| id != bone_id);
            }
            None => self.root_bone_ids.retain(|&id| id != bone_id),
        }

        let mut old_to_new = Vec::with_capacity(self.bones.len());
        let mut next_id: BoneId = 0;
        for old in 0..self.bones.len() as BoneId {
            if removed_ids.contains(&old) {
                old_to_new.push(None);
            } else {
                old_to_new.push(Some(next_id));
                next_id += 1;
            }
        }
        let remap = BoneRemap { old_to_new };
        self.bones.retain(|bone| !removed_ids.contains(&bone.id));
        self.compact_after_removal(&remap);

        Ok(RemovedBones {
            removed: removed_ids,
            parent: removed.parent_id.and_then(|parent| remap.map(parent)),
            remap,
        })
    }

    fn compact_after_removal(&mut self, remap: &BoneRemap) {
        for bone in &mut self.bones {
            bone.id = remap.map(bone.id).expect("surviving bones stay mapped");
            bone.parent_id = bone.parent_id.and_then(|parent| remap.map(parent));
            bone.children = bone
                .children
                .iter()
                .filter_map(|&child| remap.map(child))
                .collect();
        }
        self.root_bone_ids = self
            .root_bone_ids
            .iter()
            .filter_map(|&root| remap.map(root))
            .collect();
        self.bone_name_to_id = self
            .bones
            .iter()
            .map(|bone| (bone.name.clone(), bone.id))
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::InnerSpace;

    fn bind_translation(skeleton: &Skeleton, id: BoneId) -> Vector3<f32> {
        let bind = bind_pose(skeleton.get_bone(id).unwrap()).unwrap();
        bind.w.truncate()
    }

    fn build_chain() -> Skeleton {
        let mut skeleton = Skeleton::new("test");
        let root = skeleton.add_bone("root", None);
        skeleton.get_bone_mut(root).unwrap().local_transform =
            Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0));
        skeleton.get_bone_mut(root).unwrap().inverse_bind_pose =
            Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0))
                .invert()
                .unwrap();
        let middle = skeleton
            .add_child_bone(root, "middle", Vector3::new(0.0, 0.5, 0.0))
            .unwrap();
        skeleton
            .add_child_bone(middle, "tip", Vector3::new(0.0, 0.25, 0.0))
            .unwrap();
        skeleton
            .add_child_bone(root, "side", Vector3::new(1.0, 0.0, 0.0))
            .unwrap();
        skeleton
    }

    #[test]
    fn add_child_bone_places_bind_pose_under_the_parent() {
        let skeleton = build_chain();

        assert_eq!(skeleton.bone_name_to_id["middle"], 1);
        assert_eq!(skeleton.get_bone(1).unwrap().parent_id, Some(0));
        assert_eq!(skeleton.get_bone(0).unwrap().children, vec![1, 3]);
        assert!((bind_translation(&skeleton, 1) - Vector3::new(0.0, 1.5, 0.0)).magnitude() < 1e-6);
        assert!((bind_translation(&skeleton, 2) - Vector3::new(0.0, 1.75, 0.0)).magnitude() < 1e-6);
    }

    #[test]
    fn add_child_bone_rejects_duplicate_and_empty_names() {
        let mut skeleton = build_chain();

        assert_eq!(
            skeleton.add_child_bone(0, "middle", Vector3::new(0.0, 0.0, 0.0)),
            Err(SkeletonEditError::DuplicateName("middle".into()))
        );
        assert_eq!(
            skeleton.add_child_bone(0, "", Vector3::new(0.0, 0.0, 0.0)),
            Err(SkeletonEditError::EmptyName)
        );
        assert_eq!(
            skeleton.add_child_bone(9, "x", Vector3::new(0.0, 0.0, 0.0)),
            Err(SkeletonEditError::UnknownBone(9))
        );
    }

    #[test]
    fn set_bone_local_offset_moves_the_subtree_rigidly() {
        let mut skeleton = build_chain();

        skeleton
            .set_bone_local_offset(1, Vector3::new(0.0, 0.5, 0.2))
            .unwrap();

        assert!((bind_translation(&skeleton, 1) - Vector3::new(0.0, 1.5, 0.2)).magnitude() < 1e-6);
        assert!((bind_translation(&skeleton, 2) - Vector3::new(0.0, 1.75, 0.2)).magnitude() < 1e-6);
        assert!((bind_translation(&skeleton, 3) - Vector3::new(1.0, 1.0, 0.0)).magnitude() < 1e-6);
        assert_eq!(
            skeleton.get_bone(1).unwrap().local_transform.w.truncate(),
            Vector3::new(0.0, 0.5, 0.2)
        );
    }

    #[test]
    fn remove_bone_takes_its_subtree_and_compacts_ids() {
        let mut skeleton = build_chain();

        let removed = skeleton.remove_bone(1).unwrap();

        assert_eq!(removed.removed, vec![1, 2]);
        assert_eq!(removed.parent, Some(0));
        assert_eq!(removed.remap.map(0), Some(0));
        assert_eq!(removed.remap.map(1), None);
        assert_eq!(removed.remap.map(2), None);
        assert_eq!(removed.remap.map(3), Some(1));
        assert_eq!(skeleton.bone_count(), 2);
        assert!(!skeleton.bone_name_to_id.contains_key("tip"));
        let side = skeleton.get_bone(skeleton.bone_name_to_id["side"]).unwrap();
        assert_eq!(side.id, 1);
        assert_eq!(side.parent_id, Some(0));
        assert_eq!(skeleton.get_bone(0).unwrap().children, vec![1]);
        assert!((bind_translation(&skeleton, 1) - Vector3::new(1.0, 1.0, 0.0)).magnitude() < 1e-6);
    }

    #[test]
    fn remove_root_bone_empties_the_skeleton_below_it() {
        let mut skeleton = build_chain();

        let removed = skeleton.remove_bone(0).unwrap();

        assert_eq!(removed.parent, None);
        assert_eq!(removed.removed, vec![0, 1, 2, 3]);
        assert!(skeleton.bones.is_empty());
        assert!(skeleton.root_bone_ids.is_empty());
        assert!(skeleton.bone_name_to_id.is_empty());
    }

    #[test]
    fn remove_unknown_bone_is_rejected() {
        let mut skeleton = build_chain();
        assert_eq!(
            skeleton.remove_bone(7),
            Err(SkeletonEditError::UnknownBone(7))
        );
    }
}
