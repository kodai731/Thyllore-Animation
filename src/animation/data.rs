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

pub fn slerp(a: Quaternion<f32>, b: Quaternion<f32>, t: f32) -> Quaternion<f32> {
    let dot = a.s * b.s + a.v.x * b.v.x + a.v.y * b.v.y + a.v.z * b.v.z;

    let (b, dot) = if dot < 0.0 {
        (Quaternion::new(-b.s, -b.v.x, -b.v.y, -b.v.z), -dot)
    } else {
        (b, dot)
    };

    let dot = dot.clamp(-1.0, 1.0);

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
    let sin_theta_0 = theta_0.sin();

    let s0 = ((1.0 - t) * theta_0).sin() / sin_theta_0;
    let s1 = (t * theta_0).sin() / sin_theta_0;

    let result = Quaternion::new(
        s0 * a.s + s1 * b.s,
        s0 * a.v.x + s1 * b.v.x,
        s0 * a.v.y + s1 * b.v.y,
        s0 * a.v.z + s1 * b.v.z,
    );
    normalize_quat(result)
}

pub fn normalize_quat(q: Quaternion<f32>) -> Quaternion<f32> {
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
}

pub fn compose_transform(
    translation: Vector3<f32>,
    rotation: Quaternion<f32>,
    scale: Vector3<f32>,
) -> Matrix4<f32> {
    let t = Matrix4::from_translation(translation);
    let r = Matrix4::from(rotation);
    let s = Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z);
    t * r * s
}

pub fn decompose_transform(m: &Matrix4<f32>) -> (Vector3<f32>, Quaternion<f32>, Vector3<f32>) {
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
