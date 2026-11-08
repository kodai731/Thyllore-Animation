use glam::{Mat4, Quat, Vec3};
use thyllore_anim_core::spring_bone::{
    collect_affected_bone_ids, spring_bone_initialize, spring_bone_reseat_tails,
    spring_bone_update, spring_bone_write_back_to_pose, SpringBoneSetup, SpringSimulation,
    SpringSpace,
};
use thyllore_anim_core::{
    compose_transform, compute_pose_global_transforms, create_pose_from_rest, decompose_transform,
    BoneId, BoneLocalPose, Skeleton, SkeletonPose,
};
use thyllore_avatar_core::vrchat::sidecar_spring::{
    spring_setup_from_sidecar, BoneRestFrame, SidecarSpring,
};

use crate::avatar_setup_bridge::{bone_inputs, BoneSpec};
use crate::blender_space::{engine_frame_metres, parent_relative_locals, rows};
use crate::spring_edit_bridge::skeleton_from_bones;
use thyllore_math_core::invert;

pub use crate::blender_space::MatrixRows;

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct JointPose {
    pub bone: String,
    /// Parent-relative rotation as (w, x, y, z).
    pub rotation: [f32; 4],
    /// The swung local relative to the rest local, row-major: Blender's `matrix_basis`.
    pub basis: MatrixRows,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct JointTail {
    pub chain: String,
    pub bone: String,
    pub position: [f32; 3],
}

/// The engine's spring solver driven from Blender one frame at a time; `rest_matrices` are the bones' `matrix_local` rows.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpringPreviewRequest {
    pub spring: SidecarSpring,
    pub bones: Vec<BoneSpec>,
    pub armature_world: MatrixRows,
    pub rest_matrices: Vec<MatrixRows>,
    pub scale_length: f32,
}

/// The armature's world matrix and every bone's `PoseBone.matrix` at the current frame.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoseRequest {
    pub armature_world: MatrixRows,
    pub pose_matrices: Vec<MatrixRows>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepRequest {
    pub armature_world: MatrixRows,
    pub pose_matrices: Vec<MatrixRows>,
    pub dt: f32,
}

pub struct SpringPreview {
    setup: SpringBoneSetup,
    skeleton: Skeleton,
    simulation: SpringSimulation,
    affected_bone_ids: Vec<BoneId>,
    parents: Vec<Option<usize>>,
    scale_length: f32,
}

impl SpringPreview {
    pub fn new(request: &SpringPreviewRequest) -> anyhow::Result<Self> {
        let bones = bone_inputs(&request.bones);
        let parents: Vec<Option<usize>> = request.bones.iter().map(|bone| bone.parent).collect();
        let rest_locals = parent_relative_locals(
            engine_frame_metres(&request.armature_world, request.scale_length),
            &parents,
            &request.rest_matrices,
        )?;

        let mut skeleton = skeleton_from_bones(&bones);
        for (bone, rest_local) in skeleton.bones.iter_mut().zip(&rest_locals) {
            bone.local_transform = *rest_local;
        }

        let setup = spring_setup_from_sidecar(
            &request.spring,
            &bones,
            &BoneRestFrame::identity_for(bones.len()),
        );
        let rest_globals =
            compute_pose_global_transforms(&skeleton, &create_pose_from_rest(&skeleton));
        let simulation =
            spring_bone_initialize(&setup, &skeleton, &rest_globals, SpringSpace::IDENTITY);
        let affected_bone_ids = collect_affected_bone_ids(&setup);

        Ok(Self {
            setup,
            skeleton,
            simulation,
            affected_bone_ids,
            parents,
            scale_length: request.scale_length,
        })
    }

    /// Puts every tail where the bones point now, so the next step starts without a jump.
    pub fn reset(&mut self, request: &PoseRequest) -> anyhow::Result<()> {
        let pose = self.animated_pose(&request.armature_world, &request.pose_matrices)?;
        let globals = compute_pose_global_transforms(&self.skeleton, &pose);
        spring_bone_reseat_tails(&self.setup, &mut self.simulation, &self.skeleton, &globals);
        self.simulation.frame_count = 0;
        Ok(())
    }

    /// Advances the simulation by `dt` seconds and returns the swung joints' poses.
    pub fn step(&mut self, request: &StepRequest) -> anyhow::Result<Vec<JointPose>> {
        let mut pose = self.animated_pose(&request.armature_world, &request.pose_matrices)?;
        let mut globals = compute_pose_global_transforms(&self.skeleton, &pose);

        spring_bone_update(
            &self.setup,
            &mut self.simulation,
            &self.skeleton,
            &mut globals,
            &pose,
            request.dt,
            SpringSpace::IDENTITY,
        );
        spring_bone_write_back_to_pose(
            &self.skeleton,
            &globals,
            &mut pose,
            &self.affected_bone_ids,
        );

        self.affected_bone_ids
            .iter()
            .map(|&bone_id| self.joint_pose(&pose, bone_id))
            .collect()
    }

    fn animated_pose(
        &self,
        armature_world: &MatrixRows,
        pose_matrices: &[MatrixRows],
    ) -> anyhow::Result<SkeletonPose> {
        let mut locals = parent_relative_locals(
            engine_frame_metres(armature_world, self.scale_length),
            &self.parents,
            pose_matrices,
        )?;
        for &bone_id in &self.affected_bone_ids {
            locals[bone_id as usize] = self.skeleton.bones[bone_id as usize].local_transform;
        }
        Ok(pose_from_locals(&self.skeleton, &locals))
    }

    fn joint_pose(&self, pose: &SkeletonPose, bone_id: BoneId) -> anyhow::Result<JointPose> {
        let bone = &self.skeleton.bones[bone_id as usize];
        let local = &pose.bone_poses[bone_id as usize];
        let rest_inverse = invert(&bone.local_transform, || {
            format!("rest local of bone \"{}\"", bone.name)
        })?;
        let swung = compose_transform(local.translation, local.rotation, local.scale);
        Ok(JointPose {
            bone: bone.name.clone(),
            rotation: wxyz(local.rotation),
            basis: rows(rest_inverse * swung),
        })
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

fn pose_from_locals(skeleton: &Skeleton, locals: &[Mat4]) -> SkeletonPose {
    let bone_poses = locals
        .iter()
        .map(|local| {
            let (translation, rotation, scale) = decompose_transform(local);
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

fn wxyz(q: Quat) -> [f32; 4] {
    [q.w, q.x, q.y, q.z]
}

pub fn translation_rows(translation: Vec3) -> MatrixRows {
    rows(Mat4::from_translation(translation))
}

#[cfg(test)]
mod tests;
