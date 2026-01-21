use cgmath::{Matrix4, Quaternion, SquareMatrix, Vector3, Vector4};
use std::collections::HashMap;

pub type BoneId = u32;
pub type SkeletonId = u32;
pub type AnimationClipId = u32;

#[derive(Clone, Debug)]
pub struct Bone {
    pub id: BoneId,
    pub name: String,
    pub parent_id: Option<BoneId>,
    pub children: Vec<BoneId>,
    pub local_transform: Matrix4<f32>,
    pub inverse_bind_pose: Matrix4<f32>,
}

impl Default for Bone {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            parent_id: None,
            children: Vec::new(),
            local_transform: Matrix4::identity(),
            inverse_bind_pose: Matrix4::identity(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Skeleton {
    pub id: SkeletonId,
    pub name: String,
    pub bones: Vec<Bone>,
    pub bone_name_to_id: HashMap<String, BoneId>,
    pub root_bone_ids: Vec<BoneId>,
    pub root_transform: Matrix4<f32>,
}

impl Default for Skeleton {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            bones: Vec::new(),
            bone_name_to_id: HashMap::new(),
            root_bone_ids: Vec::new(),
            root_transform: Matrix4::identity(),
        }
    }
}

impl Skeleton {
    pub fn new(name: &str) -> Self {
        Self {
            id: 0,
            name: name.to_string(),
            bones: Vec::new(),
            bone_name_to_id: HashMap::new(),
            root_bone_ids: Vec::new(),
            root_transform: Matrix4::identity(),
        }
    }

    pub fn add_bone(&mut self, name: &str, parent_id: Option<BoneId>) -> BoneId {
        let id = self.bones.len() as BoneId;
        let bone = Bone {
            id,
            name: name.to_string(),
            parent_id,
            children: Vec::new(),
            local_transform: Matrix4::identity(),
            inverse_bind_pose: Matrix4::identity(),
        };

        self.bone_name_to_id.insert(name.to_string(), id);
        self.bones.push(bone);

        if let Some(parent) = parent_id {
            if let Some(parent_bone) = self.bones.get_mut(parent as usize) {
                parent_bone.children.push(id);
            }
        } else {
            self.root_bone_ids.push(id);
        }

        id
    }

    pub fn get_bone(&self, id: BoneId) -> Option<&Bone> {
        self.bones.get(id as usize)
    }

    pub fn get_bone_mut(&mut self, id: BoneId) -> Option<&mut Bone> {
        self.bones.get_mut(id as usize)
    }

    // TODO: FBX/glTFで未使用 - 必要時に有効化
    // pub fn get_bone_by_name(&self, name: &str) -> Option<&Bone> {
    //     self.bone_name_to_id
    //         .get(name)
    //         .and_then(|&id| self.get_bone(id))
    // }

    pub fn bone_count(&self) -> usize {
        self.bones.len()
    }

    pub fn compute_global_transforms(&self) -> Vec<Matrix4<f32>> {
        let mut global_transforms = vec![Matrix4::identity(); self.bones.len()];

        fn compute_recursive(
            skeleton: &Skeleton,
            bone_id: BoneId,
            parent_transform: Matrix4<f32>,
            global_transforms: &mut Vec<Matrix4<f32>>,
        ) {
            if let Some(bone) = skeleton.get_bone(bone_id) {
                let global = parent_transform * bone.local_transform;
                global_transforms[bone_id as usize] = global;

                for &child_id in &bone.children {
                    compute_recursive(skeleton, child_id, global, global_transforms);
                }
            }
        }

        for &root_id in &self.root_bone_ids {
            compute_recursive(self, root_id, self.root_transform, &mut global_transforms);
        }

        global_transforms
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Interpolation {
    Step,
    Linear,
    CubicSpline,
}

impl Default for Interpolation {
    fn default() -> Self {
        Self::Linear
    }
}

#[derive(Clone, Debug)]
pub struct Keyframe<T> {
    pub time: f32,
    pub value: T,
}

#[derive(Clone, Debug, Default)]
pub struct TransformChannel {
    pub translation: Vec<Keyframe<Vector3<f32>>>,
    pub rotation: Vec<Keyframe<Quaternion<f32>>>,
    pub scale: Vec<Keyframe<Vector3<f32>>>,
    pub interpolation: Interpolation,
}

impl TransformChannel {
    pub fn sample_translation(&self, time: f32) -> Option<Vector3<f32>> {
        Self::sample_vec3(&self.translation, time, &self.interpolation, None)
    }

    pub fn sample_translation_looped(&self, time: f32, duration: f32) -> Option<Vector3<f32>> {
        Self::sample_vec3(&self.translation, time, &self.interpolation, Some(duration))
    }

    pub fn sample_rotation(&self, time: f32) -> Option<Quaternion<f32>> {
        Self::sample_quat(&self.rotation, time, &self.interpolation, None)
    }

    pub fn sample_rotation_looped(&self, time: f32, duration: f32) -> Option<Quaternion<f32>> {
        Self::sample_quat(&self.rotation, time, &self.interpolation, Some(duration))
    }

    pub fn sample_scale(&self, time: f32) -> Option<Vector3<f32>> {
        Self::sample_vec3(&self.scale, time, &self.interpolation, None)
    }

    pub fn sample_scale_looped(&self, time: f32, duration: f32) -> Option<Vector3<f32>> {
        Self::sample_vec3(&self.scale, time, &self.interpolation, Some(duration))
    }

    fn sample_vec3(
        keyframes: &[Keyframe<Vector3<f32>>],
        time: f32,
        interpolation: &Interpolation,
        duration: Option<f32>,
    ) -> Option<Vector3<f32>> {
        if keyframes.is_empty() {
            return None;
        }
        if keyframes.len() == 1 {
            return Some(keyframes[0].value);
        }

        if time <= keyframes[0].time {
            return Some(keyframes[0].value);
        }

        let last_kf = keyframes.last().unwrap();
        if time >= last_kf.time {
            if let Some(dur) = duration {
                if dur > last_kf.time && time < dur {
                    let first_kf = &keyframes[0];
                    let wrap_duration = dur - last_kf.time + first_kf.time;
                    if wrap_duration > 0.0001 {
                        let t = (time - last_kf.time) / wrap_duration;
                        return Some(last_kf.value + (first_kf.value - last_kf.value) * t);
                    }
                }
            }
            return Some(last_kf.value);
        }

        for i in 0..keyframes.len() - 1 {
            let k0 = &keyframes[i];
            let k1 = &keyframes[i + 1];

            if time >= k0.time && time < k1.time {
                let t = (time - k0.time) / (k1.time - k0.time);
                return match interpolation {
                    Interpolation::Step => Some(k1.value),
                    Interpolation::Linear | Interpolation::CubicSpline => {
                        Some(k0.value + (k1.value - k0.value) * t)
                    }
                };
            }
        }

        Some(last_kf.value)
    }

    fn sample_quat(
        keyframes: &[Keyframe<Quaternion<f32>>],
        time: f32,
        interpolation: &Interpolation,
        duration: Option<f32>,
    ) -> Option<Quaternion<f32>> {
        if keyframes.is_empty() {
            return None;
        }
        if keyframes.len() == 1 {
            return Some(keyframes[0].value);
        }

        if time <= keyframes[0].time {
            return Some(keyframes[0].value);
        }

        let last_kf = keyframes.last().unwrap();
        if time >= last_kf.time {
            if let Some(dur) = duration {
                if dur > last_kf.time && time < dur {
                    let first_kf = &keyframes[0];
                    let wrap_duration = dur - last_kf.time + first_kf.time;
                    if wrap_duration > 0.0001 {
                        let t = (time - last_kf.time) / wrap_duration;
                        return Some(slerp(last_kf.value, first_kf.value, t));
                    }
                }
            }
            return Some(last_kf.value);
        }

        for i in 0..keyframes.len() - 1 {
            let k0 = &keyframes[i];
            let k1 = &keyframes[i + 1];

            if time >= k0.time && time < k1.time {
                let t = (time - k0.time) / (k1.time - k0.time);
                return match interpolation {
                    Interpolation::Step => Some(k1.value),
                    Interpolation::Linear | Interpolation::CubicSpline => {
                        Some(slerp(k0.value, k1.value, t))
                    }
                };
            }
        }

        Some(last_kf.value)
    }
}

fn slerp(a: Quaternion<f32>, b: Quaternion<f32>, t: f32) -> Quaternion<f32> {
    let dot = a.s * b.s + a.v.x * b.v.x + a.v.y * b.v.y + a.v.z * b.v.z;

    let (b, dot) = if dot < 0.0 {
        (Quaternion::new(-b.s, -b.v.x, -b.v.y, -b.v.z), -dot)
    } else {
        (b, dot)
    };

    if dot > 0.9995 {
        let result = Quaternion::new(
            a.s + t * (b.s - a.s),
            a.v.x + t * (b.v.x - a.v.x),
            a.v.y + t * (b.v.y - a.v.y),
            a.v.z + t * (b.v.z - a.v.z),
        );
        return normalize_quat(result);
    }

    let theta_0 = dot.acos();
    let theta = theta_0 * t;
    let sin_theta = theta.sin();
    let sin_theta_0 = theta_0.sin();

    let s0 = (theta_0 - theta).cos() - dot * sin_theta / sin_theta_0;
    let s1 = sin_theta / sin_theta_0;

    Quaternion::new(
        s0 * a.s + s1 * b.s,
        s0 * a.v.x + s1 * b.v.x,
        s0 * a.v.y + s1 * b.v.y,
        s0 * a.v.z + s1 * b.v.z,
    )
}

fn normalize_quat(q: Quaternion<f32>) -> Quaternion<f32> {
    let len = (q.s * q.s + q.v.x * q.v.x + q.v.y * q.v.y + q.v.z * q.v.z).sqrt();
    if len > 0.0 {
        Quaternion::new(q.s / len, q.v.x / len, q.v.y / len, q.v.z / len)
    } else {
        Quaternion::new(1.0, 0.0, 0.0, 0.0)
    }
}

#[derive(Clone, Debug, Default)]
pub struct AnimationClip {
    pub id: AnimationClipId,
    pub name: String,
    pub duration: f32,
    pub channels: HashMap<BoneId, TransformChannel>,
}

impl AnimationClip {
    pub fn new(name: &str) -> Self {
        Self {
            id: 0,
            name: name.to_string(),
            duration: 0.0,
            channels: HashMap::new(),
        }
    }

    pub fn add_channel(&mut self, bone_id: BoneId, channel: TransformChannel) {
        self.channels.insert(bone_id, channel);
    }

    pub fn sample(&self, time: f32, skeleton: &mut Skeleton) {
        self.sample_with_loop(time, skeleton, false)
    }

    pub fn sample_with_loop(&self, time: f32, skeleton: &mut Skeleton, looping: bool) {
        for (&bone_id, channel) in &self.channels {
            if let Some(bone) = skeleton.get_bone_mut(bone_id) {
                let (rest_t, rest_r, rest_s) = decompose_transform(&bone.local_transform);

                let (translation, rotation, scale) = if looping && self.duration > 0.0 {
                    (
                        channel
                            .sample_translation_looped(time, self.duration)
                            .unwrap_or(rest_t),
                        channel
                            .sample_rotation_looped(time, self.duration)
                            .unwrap_or(rest_r),
                        channel
                            .sample_scale_looped(time, self.duration)
                            .unwrap_or(rest_s),
                    )
                } else {
                    (
                        channel.sample_translation(time).unwrap_or(rest_t),
                        channel.sample_rotation(time).unwrap_or(rest_r),
                        channel.sample_scale(time).unwrap_or(rest_s),
                    )
                };

                bone.local_transform = compose_transform(translation, rotation, scale);
            }
        }
    }
}

fn compose_transform(
    translation: Vector3<f32>,
    rotation: Quaternion<f32>,
    scale: Vector3<f32>,
) -> Matrix4<f32> {
    let t = Matrix4::from_translation(translation);
    let r = Matrix4::from(rotation);
    let s = Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z);
    t * r * s
}

fn decompose_transform(m: &Matrix4<f32>) -> (Vector3<f32>, Quaternion<f32>, Vector3<f32>) {
    let translation = Vector3::new(m[3][0], m[3][1], m[3][2]);

    let sx = (m[0][0] * m[0][0] + m[0][1] * m[0][1] + m[0][2] * m[0][2]).sqrt();
    let sy = (m[1][0] * m[1][0] + m[1][1] * m[1][1] + m[1][2] * m[1][2]).sqrt();
    let sz = (m[2][0] * m[2][0] + m[2][1] * m[2][1] + m[2][2] * m[2][2]).sqrt();
    let scale = Vector3::new(sx, sy, sz);

    let rot_matrix = cgmath::Matrix3::new(
        m[0][0] / sx.max(0.0001),
        m[0][1] / sx.max(0.0001),
        m[0][2] / sx.max(0.0001),
        m[1][0] / sy.max(0.0001),
        m[1][1] / sy.max(0.0001),
        m[1][2] / sy.max(0.0001),
        m[2][0] / sz.max(0.0001),
        m[2][1] / sz.max(0.0001),
        m[2][2] / sz.max(0.0001),
    );
    let rotation = Quaternion::from(rot_matrix);

    (translation, rotation, scale)
}

#[derive(Clone, Debug)]
pub struct SkinData {
    pub skeleton_id: SkeletonId,
    pub bone_indices: Vec<Vector4<u32>>,
    pub bone_weights: Vec<Vector4<f32>>,
    pub base_positions: Vec<Vector3<f32>>,
    pub base_normals: Vec<Vector3<f32>>,
}

impl Default for SkinData {
    fn default() -> Self {
        Self {
            skeleton_id: 0,
            bone_indices: Vec::new(),
            bone_weights: Vec::new(),
            base_positions: Vec::new(),
            base_normals: Vec::new(),
        }
    }
}

impl SkinData {
    pub fn apply_skinning(
        &self,
        skeleton: &Skeleton,
        out_positions: &mut [Vector3<f32>],
        out_normals: &mut [Vector3<f32>],
    ) {
        let is_gltf = skeleton.name.contains("gltf");
        static mut GLTF_LOG_DONE: bool = false;
        let should_log = unsafe {
            if is_gltf && !GLTF_LOG_DONE {
                GLTF_LOG_DONE = true;
                true
            } else {
                false
            }
        };

        let global_transforms = skeleton.compute_global_transforms();
        let mut skin_matrices = Vec::with_capacity(skeleton.bone_count());

        for bone in &skeleton.bones {
            let global = global_transforms[bone.id as usize];
            let skin_matrix = global * bone.inverse_bind_pose;

            if should_log && bone.id < 3 {
                crate::log!("=== Skinning Debug bone {} ({}) ===", bone.id, bone.name);
                crate::log!(
                    "  local_transform diag: [{:.4}, {:.4}, {:.4}]",
                    bone.local_transform[0][0],
                    bone.local_transform[1][1],
                    bone.local_transform[2][2]
                );
                crate::log!(
                    "  local_transform trans: [{:.4}, {:.4}, {:.4}]",
                    bone.local_transform[3][0],
                    bone.local_transform[3][1],
                    bone.local_transform[3][2]
                );
                crate::log!(
                    "  global_transform diag: [{:.4}, {:.4}, {:.4}]",
                    global[0][0],
                    global[1][1],
                    global[2][2]
                );
                crate::log!(
                    "  global_transform trans: [{:.4}, {:.4}, {:.4}]",
                    global[3][0],
                    global[3][1],
                    global[3][2]
                );
                crate::log!(
                    "  inverse_bind_pose trans: [{:.4}, {:.4}, {:.4}]",
                    bone.inverse_bind_pose[3][0],
                    bone.inverse_bind_pose[3][1],
                    bone.inverse_bind_pose[3][2]
                );
                crate::log!(
                    "  skin_matrix diag: [{:.4}, {:.4}, {:.4}]",
                    skin_matrix[0][0],
                    skin_matrix[1][1],
                    skin_matrix[2][2]
                );
                crate::log!(
                    "  skin_matrix trans: [{:.4}, {:.4}, {:.4}]",
                    skin_matrix[3][0],
                    skin_matrix[3][1],
                    skin_matrix[3][2]
                );
            }

            skin_matrices.push(skin_matrix);
        }

        if should_log && !self.base_positions.is_empty() {
            let pos = self.base_positions[0];
            crate::log!(
                "=== base_positions[0]: [{:.4}, {:.4}, {:.4}] ===",
                pos.x,
                pos.y,
                pos.z
            );
        }

        for i in 0..self.base_positions.len() {
            let indices = &self.bone_indices[i];
            let weights = &self.bone_weights[i];

            let mut skinned_pos = Vector3::new(0.0, 0.0, 0.0);
            let mut skinned_normal = Vector3::new(0.0, 0.0, 0.0);

            for j in 0..4 {
                let bone_idx = match j {
                    0 => indices.x,
                    1 => indices.y,
                    2 => indices.z,
                    3 => indices.w,
                    _ => 0,
                } as usize;

                let weight = match j {
                    0 => weights.x,
                    1 => weights.y,
                    2 => weights.z,
                    3 => weights.w,
                    _ => 0.0,
                };

                if weight > 0.0 && bone_idx < skin_matrices.len() {
                    let m = &skin_matrices[bone_idx];

                    let pos = self.base_positions[i];
                    let transformed = m * Vector4::new(pos.x, pos.y, pos.z, 1.0);
                    skinned_pos +=
                        Vector3::new(transformed.x, transformed.y, transformed.z) * weight;

                    if i < self.base_normals.len() {
                        let normal = self.base_normals[i];
                        let transformed_n = m * Vector4::new(normal.x, normal.y, normal.z, 0.0);
                        skinned_normal +=
                            Vector3::new(transformed_n.x, transformed_n.y, transformed_n.z)
                                * weight;
                    }
                }
            }

            if i < out_positions.len() {
                out_positions[i] = skinned_pos;
            }
            if i < out_normals.len() {
                let len = (skinned_normal.x * skinned_normal.x
                    + skinned_normal.y * skinned_normal.y
                    + skinned_normal.z * skinned_normal.z)
                    .sqrt();
                if len > 0.0 {
                    out_normals[i] = skinned_normal / len;
                }
            }

            if should_log && i < 3 {
                let base = self.base_positions[i];
                crate::log!("=== Skinning vertex {} ===", i);
                crate::log!("  base_pos: [{:.4}, {:.4}, {:.4}]", base.x, base.y, base.z);
                crate::log!(
                    "  bone_indices: [{}, {}, {}, {}]",
                    indices.x,
                    indices.y,
                    indices.z,
                    indices.w
                );
                crate::log!(
                    "  bone_weights: [{:.4}, {:.4}, {:.4}, {:.4}]",
                    weights.x,
                    weights.y,
                    weights.z,
                    weights.w
                );
                crate::log!(
                    "  skinned_pos: [{:.4}, {:.4}, {:.4}]",
                    skinned_pos.x,
                    skinned_pos.y,
                    skinned_pos.z
                );
            }
        }

        if should_log {
            let mut max_coord = 0.0f32;
            for pos in out_positions.iter() {
                max_coord = max_coord.max(pos.x.abs()).max(pos.y.abs()).max(pos.z.abs());
            }
            crate::log!(
                "=== Skinning result: max_coord={:.4}, vertex_count={} ===",
                max_coord,
                out_positions.len()
            );
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct AnimationSystem {
    pub skeletons: Vec<Skeleton>,
    pub clips: Vec<AnimationClip>,
    next_skeleton_id: SkeletonId,
    next_clip_id: AnimationClipId,
}

impl AnimationSystem {
    pub fn new() -> Self {
        Self {
            skeletons: Vec::new(),
            clips: Vec::new(),
            next_skeleton_id: 0,
            next_clip_id: 0,
        }
    }

    pub fn add_skeleton(&mut self, mut skeleton: Skeleton) -> SkeletonId {
        let id = self.next_skeleton_id;
        self.next_skeleton_id += 1;
        skeleton.id = id;
        self.skeletons.push(skeleton);
        id
    }

    pub fn add_clip(&mut self, mut clip: AnimationClip) -> AnimationClipId {
        let id = self.next_clip_id;
        self.next_clip_id += 1;
        clip.id = id;
        self.clips.push(clip);
        id
    }

    pub fn get_skeleton(&self, id: SkeletonId) -> Option<&Skeleton> {
        self.skeletons.iter().find(|s| s.id == id)
    }

    pub fn get_skeleton_mut(&mut self, id: SkeletonId) -> Option<&mut Skeleton> {
        self.skeletons.iter_mut().find(|s| s.id == id)
    }

    pub fn get_clip(&self, id: AnimationClipId) -> Option<&AnimationClip> {
        self.clips.iter().find(|c| c.id == id)
    }

    pub fn apply_to_skeleton(
        &mut self,
        skeleton_id: SkeletonId,
        playback: &crate::ecs::AnimationPlayback,
    ) {
        let clip_id = match playback.current_clip_id {
            Some(id) => id,
            None => return,
        };

        let clip = match self.clips.iter().find(|c| c.id == clip_id) {
            Some(c) => c.clone(),
            None => return,
        };

        if let Some(skeleton) = self.get_skeleton_mut(skeleton_id) {
            clip.sample_with_loop(playback.time, skeleton, playback.looping);
        }
    }

    pub fn clear(&mut self) {
        self.skeletons.clear();
        self.clips.clear();
        self.next_skeleton_id = 0;
        self.next_clip_id = 0;
    }
}

#[derive(Clone, Debug, Default)]
pub struct MorphTarget {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub tangents: Vec<[f32; 3]>,
}

#[derive(Clone, Debug, Default)]
pub struct MorphAnimation {
    pub key_frame: f32,
    pub weights: Vec<f32>,
}

#[derive(Clone, Debug)]
pub struct MorphAnimationSystem {
    pub animations: Vec<MorphAnimation>,
    pub targets: Vec<Vec<MorphTarget>>,
    pub base_vertices: Vec<Vec<[f32; 3]>>,
    pub scale_factor: f32,
}

impl Default for MorphAnimationSystem {
    fn default() -> Self {
        Self {
            animations: Vec::new(),
            targets: Vec::new(),
            base_vertices: Vec::new(),
            scale_factor: 1.0,
        }
    }
}

impl MorphAnimationSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.animations.is_empty()
    }

    pub fn get_animation_index(&self, time: f32) -> usize {
        if self.animations.is_empty() {
            return 0;
        }

        let start_key_frame = self
            .animations
            .first()
            .expect("morph_animations is empty")
            .key_frame;
        let end_key_frame = self
            .animations
            .last()
            .expect("morph_animations is empty")
            .key_frame;
        let period = end_key_frame - start_key_frame;
        let mod_time = time.rem_euclid(period);

        for i in 0..self.animations.len() {
            if mod_time <= self.animations[i].key_frame {
                return i;
            }
        }
        self.animations.len() - 1
    }

    pub fn clear(&mut self) {
        self.animations.clear();
        self.targets.clear();
        self.base_vertices.clear();
    }
}
