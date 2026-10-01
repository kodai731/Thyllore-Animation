use crate::humanoid::components::skeleton_input::BoneInput;

pub fn iter_ancestors(bones: &[BoneInput], bone_index: usize) -> impl Iterator<Item = usize> + '_ {
    std::iter::successors(bones[bone_index].parent, |&index| bones[index].parent).take(bones.len())
}

pub fn is_ancestor(bones: &[BoneInput], ancestor_index: usize, bone_index: usize) -> bool {
    iter_ancestors(bones, bone_index).any(|index| index == ancestor_index)
}
