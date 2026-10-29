use cgmath::{Matrix, Matrix4, Quaternion, Vector3};
use thyllore_anim_core::spring_bone::{
    collect_affected_bone_ids, spring_bone_initialize, spring_bone_reseat_tails,
    spring_bone_update, spring_bone_write_back_to_pose, SpringBoneSetup, SpringSimulation,
    SpringSpace,
};
use thyllore_anim_core::{
    compute_pose_global_transforms, create_pose_from_rest, decompose_transform, BoneId,
    BoneLocalPose, Skeleton, SkeletonPose,
};
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::vrchat::sidecar_spring::{
    spring_setup_from_sidecar, BoneRestFrame, SidecarSpring,
};

use crate::spring_edit_bridge::skeleton_from_bones;

/// A row-major 4x4 as the addon sends it; the engine stores matrices column-major.
pub type MatrixRows = [[f32; 4]; 4];

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct JointRotation {
    pub bone: String,
    /// Parent-relative rotation as (w, x, y, z).
    pub rotation: [f32; 4],
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct JointTail {
    pub chain: String,
    pub bone: String,
    pub position: [f32; 3],
}

/// The engine's spring solver driven from Blender one frame at a time.
///
/// Every matrix is a bone's parent-relative transform; root bones carry the armature's world
/// transform converted to engine axes and metres, so the propagated globals are in engine space.
pub struct SpringPreview {
    setup: SpringBoneSetup,
    skeleton: Skeleton,
    simulation: SpringSimulation,
    affected_bone_ids: Vec<BoneId>,
}

impl SpringPreview {
    pub fn new(
        spring: &SidecarSpring,
        bones: &[BoneInput],
        rest_locals: &[MatrixRows],
    ) -> anyhow::Result<Self> {
        let mut skeleton = skeleton_from_bones(bones);
        for (bone, rest_local) in skeleton.bones.iter_mut().zip(rest_locals) {
            bone.local_transform = column_major(rest_local);
        }
        check_bone_count(&skeleton, rest_locals.len())?;

        let setup =
            spring_setup_from_sidecar(spring, bones, &BoneRestFrame::identity_for(bones.len()));
        let rest_globals =
            compute_pose_global_transforms(&skeleton, &create_pose_from_rest(&skeleton));
        let simulation = spring_bone_initialize(&setup, &skeleton, &rest_globals);
        let affected_bone_ids = collect_affected_bone_ids(&setup);

        Ok(Self {
            setup,
            skeleton,
            simulation,
            affected_bone_ids,
        })
    }

    /// Puts every tail where the bones point now, so the next step starts without a jump.
    pub fn reset(&mut self, animated_locals: &[MatrixRows]) -> anyhow::Result<()> {
        check_bone_count(&self.skeleton, animated_locals.len())?;
        let pose = pose_from_locals(&self.skeleton, animated_locals);
        let globals = compute_pose_global_transforms(&self.skeleton, &pose);
        spring_bone_reseat_tails(&self.setup, &mut self.simulation, &self.skeleton, &globals);
        self.simulation.frame_count = 0;
        Ok(())
    }

    /// Advances the simulation by `dt` seconds and returns the swung joints' parent-relative rotations.
    pub fn step(
        &mut self,
        animated_locals: &[MatrixRows],
        dt: f32,
    ) -> anyhow::Result<Vec<JointRotation>> {
        check_bone_count(&self.skeleton, animated_locals.len())?;
        let mut pose = pose_from_locals(&self.skeleton, animated_locals);
        let mut globals = compute_pose_global_transforms(&self.skeleton, &pose);

        spring_bone_update(
            &self.setup,
            &mut self.simulation,
            &self.skeleton,
            &mut globals,
            &pose,
            dt,
            SpringSpace::IDENTITY,
        );
        spring_bone_write_back_to_pose(
            &self.skeleton,
            &globals,
            &mut pose,
            &self.affected_bone_ids,
        );

        Ok(self
            .affected_bone_ids
            .iter()
            .map(|&bone_id| JointRotation {
                bone: self.skeleton.bones[bone_id as usize].name.clone(),
                rotation: wxyz(pose.bone_poses[bone_id as usize].rotation),
            })
            .collect())
    }

    /// The current tail of every joint in engine space, the same samples the engine's tail dump writes.
    pub fn tails(&self) -> Vec<JointTail> {
        let enabled_chains = self.setup.chains.iter().filter(|chain| chain.enabled);
        enabled_chains
            .zip(&self.simulation.chain_states)
            .flat_map(|(chain, chain_state)| {
                chain
                    .joints
                    .iter()
                    .zip(&chain_state.joint_states)
                    .map(|(joint, joint_state)| JointTail {
                        chain: chain.name.clone(),
                        bone: self.skeleton.bones[joint.bone_id as usize].name.clone(),
                        position: joint_state.current_tail.into(),
                    })
            })
            .collect()
    }
}

fn check_bone_count(skeleton: &Skeleton, matrix_count: usize) -> anyhow::Result<()> {
    if matrix_count != skeleton.bones.len() {
        anyhow::bail!(
            "expected one matrix per bone ({}), got {}",
            skeleton.bones.len(),
            matrix_count
        );
    }
    Ok(())
}

fn column_major(rows: &MatrixRows) -> Matrix4<f32> {
    Matrix4::from(*rows).transpose()
}

fn pose_from_locals(skeleton: &Skeleton, locals: &[MatrixRows]) -> SkeletonPose {
    let bone_poses = locals
        .iter()
        .map(|rows| {
            let (translation, rotation, scale) = decompose_transform(&column_major(rows));
            BoneLocalPose {
                translation,
                rotation,
                scale,
            }
        })
        .collect();
    SkeletonPose {
        skeleton_id: skeleton.id,
        bone_poses,
    }
}

fn wxyz(q: Quaternion<f32>) -> [f32; 4] {
    [q.s, q.v.x, q.v.y, q.v.z]
}

pub fn translation_rows(translation: Vector3<f32>) -> MatrixRows {
    [
        [1.0, 0.0, 0.0, translation.x],
        [0.0, 1.0, 0.0, translation.y],
        [0.0, 0.0, 1.0, translation.z],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

#[cfg(test)]
mod tests;
