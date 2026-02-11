use std::collections::HashMap;

use cgmath::{Quaternion, Vector3};
use serde::{Deserialize, Serialize};

use crate::animation::{AnimationClip, BoneId, Keyframe, TransformChannel};

use super::curve::{PropertyCurve, PropertyType};
use super::keyframe::{InterpolationType, SourceClipId};
use super::track::BoneTrack;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditableAnimationClip {
    pub id: SourceClipId,
    pub name: String,
    pub duration: f32,
    pub tracks: HashMap<BoneId, BoneTrack>,
    pub source_path: Option<String>,
    next_curve_id: u64,
}

impl EditableAnimationClip {
    pub fn new(id: SourceClipId, name: String) -> Self {
        Self {
            id,
            name,
            duration: 0.0,
            tracks: HashMap::new(),
            source_path: None,
            next_curve_id: 1,
        }
    }

    pub fn from_animation_clip(
        id: SourceClipId,
        clip: &AnimationClip,
        bone_names: &HashMap<BoneId, String>,
    ) -> Self {
        let mut editable = Self::new(id, clip.name.clone());
        editable.duration = clip.duration;

        for (&bone_id, channel) in &clip.channels {
            let bone_name = bone_names
                .get(&bone_id)
                .cloned()
                .unwrap_or_else(|| format!("Bone_{}", bone_id));

            let base_curve_id = editable.next_curve_id;
            editable.next_curve_id += 10;

            let mut track = BoneTrack::new(bone_id, bone_name, base_curve_id);

            for kf in &channel.translation {
                track.translation_x.add_keyframe(kf.time, kf.value.x);
                track.translation_y.add_keyframe(kf.time, kf.value.y);
                track.translation_z.add_keyframe(kf.time, kf.value.z);
            }

            for kf in &channel.rotation {
                let euler = quaternion_to_euler_degrees(&kf.value);
                track.rotation_x.add_keyframe(kf.time, euler.x);
                track.rotation_y.add_keyframe(kf.time, euler.y);
                track.rotation_z.add_keyframe(kf.time, euler.z);
            }

            for kf in &channel.scale {
                track.scale_x.add_keyframe(kf.time, kf.value.x);
                track.scale_y.add_keyframe(kf.time, kf.value.y);
                track.scale_z.add_keyframe(kf.time, kf.value.z);
            }

            editable.tracks.insert(bone_id, track);
        }

        editable
    }

    pub fn to_animation_clip(&self) -> AnimationClip {
        let mut clip = AnimationClip::new(&self.name);
        clip.duration = self.duration;

        for (&bone_id, track) in &self.tracks {
            let mut channel = TransformChannel::default();

            let translation_curves = [
                &track.translation_x,
                &track.translation_y,
                &track.translation_z,
            ];
            let translation_times = collect_bake_times(&translation_curves);
            for time in translation_times {
                let x = track.translation_x.sample(time).unwrap_or(0.0);
                let y = track.translation_y.sample(time).unwrap_or(0.0);
                let z = track.translation_z.sample(time).unwrap_or(0.0);
                channel.translation.push(Keyframe {
                    time,
                    value: Vector3::new(x, y, z),
                });
            }

            let rotation_curves = [
                &track.rotation_x,
                &track.rotation_y,
                &track.rotation_z,
            ];
            let rotation_times = collect_bake_times(&rotation_curves);
            for time in rotation_times {
                let ex = track.rotation_x.sample(time).unwrap_or(0.0);
                let ey = track.rotation_y.sample(time).unwrap_or(0.0);
                let ez = track.rotation_z.sample(time).unwrap_or(0.0);
                let q = euler_degrees_to_quaternion(ex, ey, ez);

                channel.rotation.push(Keyframe {
                    time,
                    value: q,
                });
            }

            let scale_curves =
                [&track.scale_x, &track.scale_y, &track.scale_z];
            let scale_times = collect_bake_times(&scale_curves);
            for time in scale_times {
                let x = track.scale_x.sample(time).unwrap_or(1.0);
                let y = track.scale_y.sample(time).unwrap_or(1.0);
                let z = track.scale_z.sample(time).unwrap_or(1.0);
                channel.scale.push(Keyframe {
                    time,
                    value: Vector3::new(x, y, z),
                });
            }

            if !channel.translation.is_empty()
                || !channel.rotation.is_empty()
                || !channel.scale.is_empty()
            {
                clip.add_channel(bone_id, channel);
            }
        }

        clip
    }

    pub fn add_track(&mut self, bone_id: BoneId, bone_name: String) -> &mut BoneTrack {
        let base_curve_id = self.next_curve_id;
        self.next_curve_id += 10;

        let track = BoneTrack::new(bone_id, bone_name, base_curve_id);
        self.tracks.insert(bone_id, track);
        self.tracks.get_mut(&bone_id).unwrap()
    }

    pub fn remove_track(&mut self, bone_id: BoneId) -> Option<BoneTrack> {
        self.tracks.remove(&bone_id)
    }

    pub fn get_track(&self, bone_id: BoneId) -> Option<&BoneTrack> {
        self.tracks.get(&bone_id)
    }

    pub fn get_track_mut(&mut self, bone_id: BoneId) -> Option<&mut BoneTrack> {
        self.tracks.get_mut(&bone_id)
    }

    pub fn add_keyframe(
        &mut self,
        bone_id: BoneId,
        property_type: PropertyType,
        time: f32,
        value: f32,
    ) -> Option<u64> {
        self.tracks
            .get_mut(&bone_id)
            .map(|track| track.get_curve_mut(property_type).add_keyframe(time, value))
    }

    pub fn recalculate_duration(&mut self) {
        let mut max_time: f32 = 0.0;

        for track in self.tracks.values() {
            for curve in track.all_curves() {
                if let Some(last_kf) = curve.keyframes.last() {
                    max_time = max_time.max(last_kf.time);
                }
            }
        }

        self.duration = max_time;
    }

    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    pub fn total_keyframe_count(&self) -> usize {
        self.tracks
            .values()
            .map(|t| t.total_keyframe_count())
            .sum()
    }
}

fn collect_unique_times(curves: &[&PropertyCurve]) -> Vec<f32> {
    let mut times: Vec<f32> = curves
        .iter()
        .flat_map(|c| c.keyframes.iter().map(|k| k.time))
        .collect();

    times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    times.dedup_by(|a, b| (*a - *b).abs() < 0.0001);
    times
}

fn collect_bake_times(curves: &[&PropertyCurve]) -> Vec<f32> {
    let has_bezier = curves
        .iter()
        .any(|c| c.has_bezier_keyframes());

    if !has_bezier {
        return collect_unique_times(curves);
    }

    let mut times: Vec<f32> = Vec::new();

    for curve in curves {
        for kf in &curve.keyframes {
            times.push(kf.time);
        }

        for i in 0..curve.keyframes.len().saturating_sub(1) {
            let k0 = &curve.keyframes[i];
            let k1 = &curve.keyframes[i + 1];

            if k0.interpolation == InterpolationType::Bezier {
                let bezier_subdivisions = 10;
                for s in 1..bezier_subdivisions {
                    let frac = s as f32 / bezier_subdivisions as f32;
                    let mid_time = k0.time + (k1.time - k0.time) * frac;
                    times.push(mid_time);
                }
            }
        }
    }

    times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    times.dedup_by(|a, b| (*a - *b).abs() < 0.0001);
    times
}

pub(crate) fn quaternion_to_euler_degrees(q: &Quaternion<f32>) -> Vector3<f32> {
    let w = q.s;
    let x = q.v.x;
    let y = q.v.y;
    let z = q.v.z;

    let sinp = 2.0 * (w * x + y * z);
    let cosp = 1.0 - 2.0 * (x * x + y * y);
    let pitch = sinp.atan2(cosp);

    let siny = 2.0 * (w * y - z * x);
    let yaw = if siny.abs() >= 1.0 {
        std::f32::consts::FRAC_PI_2.copysign(siny)
    } else {
        siny.asin()
    };

    let sinr = 2.0 * (w * z + x * y);
    let cosr = 1.0 - 2.0 * (y * y + z * z);
    let roll = sinr.atan2(cosr);

    Vector3::new(
        pitch.to_degrees(),
        yaw.to_degrees(),
        roll.to_degrees(),
    )
}

fn euler_degrees_to_quaternion(x_deg: f32, y_deg: f32, z_deg: f32) -> Quaternion<f32> {
    let x_rad = x_deg.to_radians();
    let y_rad = y_deg.to_radians();
    let z_rad = z_deg.to_radians();

    let cx = (x_rad * 0.5).cos();
    let sx = (x_rad * 0.5).sin();
    let cy = (y_rad * 0.5).cos();
    let sy = (y_rad * 0.5).sin();
    let cz = (z_rad * 0.5).cos();
    let sz = (z_rad * 0.5).sin();

    Quaternion::new(
        cx * cy * cz + sx * sy * sz,
        sx * cy * cz - cx * sy * sz,
        cx * sy * cz + sx * cy * sz,
        cx * cy * sz - sx * sy * cz,
    )
}

impl Default for EditableAnimationClip {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            duration: 0.0,
            tracks: HashMap::new(),
            source_path: None,
            next_curve_id: 1,
        }
    }
}
