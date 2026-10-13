use std::collections::BTreeMap;

use anyhow::{bail, Result};

use crate::humanoid::components::role::HumanoidRole;
use crate::motion::seed::components::motion_spec::{MotionSide, MotionSpec};
use crate::motion::seed::components::pose_table::{
    MotionDef, PoseAxis, PoseEntry, PoseSide, PoseTable, RoleKey,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeedKey {
    pub role: HumanoidRole,
    pub axis: PoseAxis,
    pub time: f32,
    pub value: f32,
}

/// Where the Copilot may add follow-through: a curve that just arrived at a value it then holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SettleRequest {
    pub role: HumanoidRole,
    pub axis: PoseAxis,
    pub time: f32,
    pub until: f32,
    pub max_frames: usize,
    pub blend: f32,
}

/// Arrivals into a hold whose change is at least `settle.min_change`, from the composed keys.
pub fn settle_requests(
    table: &PoseTable,
    spec: &MotionSpec,
    keys: &[SeedKey],
) -> Result<Vec<SettleRequest>> {
    let motion = table
        .motion(&spec.motion)
        .ok_or_else(|| anyhow::anyhow!("unknown motion '{}'", spec.motion))?;
    let Some(settle) = motion.settle else {
        return Ok(Vec::new());
    };

    let mut requests = Vec::new();
    for window in keys.windows(3) {
        let [previous, arrival, next] = [window[0], window[1], window[2]];
        let same_curve = previous.role == arrival.role
            && previous.axis == arrival.axis
            && next.role == arrival.role
            && next.axis == arrival.axis;
        if !same_curve || !arrival.axis.is_rotation() {
            continue;
        }
        let change = (arrival.value - previous.value).abs();
        let holds = (next.value - arrival.value).abs() < 1e-6 && next.time > arrival.time;
        if change >= settle.min_change && holds {
            requests.push(SettleRequest {
                role: arrival.role,
                axis: arrival.axis,
                time: arrival.time,
                until: next.time,
                max_frames: settle.frames,
                blend: settle.blend,
            });
        }
    }
    Ok(requests)
}

struct TimedPose {
    time: f32,
    keys: Vec<RoleKey>,
}

/// Turns a motion spec into sparse keys: rest at 0, every touched curve keyed at every pose
/// (unset curves hold their previous value), rest again at the end.
pub fn compose_motion(table: &PoseTable, spec: &MotionSpec) -> Result<Vec<SeedKey>> {
    let motion = table.motion(&spec.motion).ok_or_else(|| {
        anyhow::anyhow!(
            "unknown motion '{}'. Known motions: {}",
            spec.motion,
            table.motion_names().join(", ")
        )
    })?;

    let mut poses = expand_poses(table, motion, spec)?;
    let mut duration = motion.duration;
    repeat_cycle(motion, spec.count, &mut poses, &mut duration)?;
    scale_time(spec.speed, &mut poses, &mut duration);

    Ok(emit_keys_with_holds(&poses, duration))
}

fn expand_poses(
    table: &PoseTable,
    motion: &MotionDef,
    spec: &MotionSpec,
) -> Result<Vec<TimedPose>> {
    let mut poses = Vec::with_capacity(motion.poses.len());
    for pose in &motion.poses {
        let mut keys = Vec::new();
        for entry in &pose.set {
            match entry {
                PoseEntry::Pose { name, side } => {
                    let named = table
                        .pose(name)
                        .ok_or_else(|| anyhow::anyhow!("pose '{name}' vanished from the table"))?;
                    for key in &named.keys {
                        match side {
                            PoseSide::Same => keys.push(*key),
                            PoseSide::Other => keys.push(key.mirrored()),
                            PoseSide::Both => {
                                keys.push(*key);
                                keys.push(key.mirrored());
                            }
                        }
                    }
                }
                PoseEntry::Key(key) => keys.push(*key),
            }
        }

        for key in &mut keys {
            if spec.side == MotionSide::Left {
                *key = key.mirrored();
            }
            key.value *= spec.amount;
        }
        poses.push(TimedPose {
            time: pose.time,
            keys,
        });
    }
    Ok(poses)
}

fn repeat_cycle(
    motion: &MotionDef,
    count: u32,
    poses: &mut Vec<TimedPose>,
    duration: &mut f32,
) -> Result<()> {
    if count <= 1 {
        return Ok(());
    }
    let Some((start, end)) = motion.cycle else {
        bail!(
            "motion '{}' has no cycle, so count must be 1 (got {count})",
            motion.name
        );
    };
    let period = end - start;
    let added = period * (count - 1) as f32;

    let mut repeated = Vec::with_capacity(poses.len() * count as usize);
    for pose in poses.drain(..) {
        if pose.time >= end {
            repeated.push(TimedPose {
                time: pose.time + added,
                keys: pose.keys,
            });
        } else if pose.time >= start {
            for repeat in 0..count {
                repeated.push(TimedPose {
                    time: pose.time + period * repeat as f32,
                    keys: pose.keys.clone(),
                });
            }
        } else {
            repeated.push(pose);
        }
    }
    repeated.sort_by(|a, b| a.time.total_cmp(&b.time));
    *poses = repeated;
    *duration += added;
    Ok(())
}

fn scale_time(speed: f32, poses: &mut [TimedPose], duration: &mut f32) {
    for pose in poses.iter_mut() {
        pose.time /= speed;
    }
    *duration /= speed;
}

fn emit_keys_with_holds(poses: &[TimedPose], duration: f32) -> Vec<SeedKey> {
    let mut held: BTreeMap<(usize, PoseAxis), (HumanoidRole, f32)> = BTreeMap::new();
    for pose in poses {
        for key in &pose.keys {
            held.insert((key.role.index(), key.axis), (key.role, 0.0));
        }
    }

    let mut keys = Vec::new();
    let mut emit_frame =
        |time: f32, set: &[RoleKey], held: &mut BTreeMap<_, (HumanoidRole, f32)>| {
            for key in set {
                if let Some(slot) = held.get_mut(&(key.role.index(), key.axis)) {
                    slot.1 = key.value;
                }
            }
            for (&(_, axis), &(role, value)) in held.iter() {
                keys.push(SeedKey {
                    role,
                    axis,
                    time,
                    value,
                });
            }
        };

    let starts_at_zero = poses.first().is_some_and(|p| p.time <= f32::EPSILON);
    if !starts_at_zero {
        emit_frame(0.0, &[], &mut held);
    }
    for pose in poses {
        emit_frame(pose.time, &pose.keys, &mut held);
    }
    let ends_at_duration = poses
        .last()
        .is_some_and(|p| (p.time - duration).abs() <= f32::EPSILON);
    if !ends_at_duration {
        let rest: Vec<RoleKey> = held
            .values()
            .map(|(role, _)| RoleKey {
                role: *role,
                axis: PoseAxis::X,
                value: 0.0,
            })
            .collect();
        for slot in held.values_mut() {
            slot.1 = 0.0;
        }
        emit_frame(duration, &rest, &mut held);
    }

    keys.sort_by(|a, b| {
        (a.role.index(), a.axis)
            .cmp(&(b.role.index(), b.axis))
            .then(a.time.total_cmp(&b.time))
    });
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys_of(keys: &[SeedKey], role: HumanoidRole, axis: PoseAxis) -> Vec<(f32, f32)> {
        keys.iter()
            .filter(|k| k.role == role && k.axis == axis)
            .map(|k| (k.time, k.value))
            .collect()
    }

    #[test]
    fn punch_holds_unset_curves_and_rests_at_both_ends() {
        let keys = compose_motion(PoseTable::builtin(), &MotionSpec::new("punch")).unwrap();

        assert_eq!(
            keys_of(&keys, HumanoidRole::RightUpperArm, PoseAxis::Y),
            vec![
                (0.0, 0.0),
                (0.4, 0.0),
                (0.55, -85.0),
                (0.9, -85.0),
                (1.3, 0.0),
                (2.0, 0.0)
            ]
        );
        assert_eq!(
            keys_of(&keys, HumanoidRole::LeftUpperArm, PoseAxis::Z),
            vec![
                (0.0, 0.0),
                (0.4, 80.0),
                (0.55, 80.0),
                (0.9, 80.0),
                (1.3, 80.0),
                (2.0, 0.0)
            ]
        );
        assert_eq!(
            keys_of(&keys, HumanoidRole::Spine, PoseAxis::Y),
            vec![
                (0.0, 0.0),
                (0.4, 15.0),
                (0.55, -15.0),
                (0.9, -15.0),
                (1.3, 15.0),
                (2.0, 0.0)
            ]
        );
    }

    #[test]
    fn left_side_mirrors_roles_and_yz_signs() {
        let mut spec = MotionSpec::new("punch");
        spec.side = MotionSide::Left;
        let keys = compose_motion(PoseTable::builtin(), &spec).unwrap();

        assert_eq!(
            keys_of(&keys, HumanoidRole::LeftUpperArm, PoseAxis::Y),
            vec![
                (0.0, 0.0),
                (0.4, 0.0),
                (0.55, 85.0),
                (0.9, 85.0),
                (1.3, 0.0),
                (2.0, 0.0)
            ]
        );
        assert_eq!(
            keys_of(&keys, HumanoidRole::Spine, PoseAxis::Y),
            vec![
                (0.0, 0.0),
                (0.4, -15.0),
                (0.55, 15.0),
                (0.9, 15.0),
                (1.3, -15.0),
                (2.0, 0.0)
            ]
        );
    }

    #[test]
    fn count_repeats_the_cycle_and_extends_duration() {
        let mut spec = MotionSpec::new("wave");
        spec.count = 2;
        let keys = compose_motion(PoseTable::builtin(), &spec).unwrap();

        assert_eq!(
            keys_of(&keys, HumanoidRole::RightLowerArm, PoseAxis::Z),
            vec![
                (0.0, 0.0),
                (0.4, 90.0),
                (0.8, 110.0),
                (1.2, 70.0),
                (1.6, 110.0),
                (2.0, 70.0),
                (2.4, 90.0),
                (2.8, 0.0)
            ]
        );
    }

    #[test]
    fn count_without_cycle_is_an_error() {
        let mut spec = MotionSpec::new("bow");
        spec.count = 2;
        assert!(compose_motion(PoseTable::builtin(), &spec).is_err());
    }

    #[test]
    fn amount_and_speed_scale_degrees_and_time() {
        let mut spec = MotionSpec::new("bow");
        spec.amount = 0.5;
        spec.speed = 2.0;
        let keys = compose_motion(PoseTable::builtin(), &spec).unwrap();

        assert_eq!(
            keys_of(&keys, HumanoidRole::Spine, PoseAxis::X),
            vec![(0.0, 0.0), (0.3, 10.0), (0.5, 10.0), (0.8, 0.0)]
        );
    }

    #[test]
    fn settle_requests_cover_arrivals_into_holds_only() {
        let spec = MotionSpec::new("punch");
        let keys = compose_motion(PoseTable::builtin(), &spec).unwrap();
        let requests = settle_requests(PoseTable::builtin(), &spec, &keys).unwrap();

        let mut found: Vec<(HumanoidRole, PoseAxis, f32, f32)> = requests
            .iter()
            .map(|r| (r.role, r.axis, r.time, r.until))
            .collect();
        found.sort_by(|a, b| {
            (a.0.index(), a.1, a.2)
                .partial_cmp(&(b.0.index(), b.1, b.2))
                .unwrap()
        });
        assert_eq!(
            found,
            vec![
                (HumanoidRole::Spine, PoseAxis::Y, 0.55, 0.9),
                (HumanoidRole::LeftUpperArm, PoseAxis::Z, 0.4, 0.55),
                (HumanoidRole::RightUpperArm, PoseAxis::Y, 0.55, 0.9),
                (HumanoidRole::RightUpperArm, PoseAxis::Y, 1.3, 2.0),
                (HumanoidRole::RightUpperArm, PoseAxis::Z, 0.55, 0.9),
                (HumanoidRole::LeftLowerArm, PoseAxis::Y, 0.4, 0.55),
                (HumanoidRole::RightLowerArm, PoseAxis::Y, 0.55, 0.9),
            ]
        );
        assert!(requests
            .iter()
            .all(|r| r.blend == 0.3 && r.max_frames == 16));
    }

    #[test]
    fn motions_without_settle_request_nothing() {
        let spec = MotionSpec::new("bow");
        let keys = compose_motion(PoseTable::builtin(), &spec).unwrap();
        assert!(settle_requests(PoseTable::builtin(), &spec, &keys)
            .unwrap()
            .is_empty());
    }

    fn crouch_table() -> PoseTable {
        PoseTable::parse(
            r#"
[[motion]]
name = "crouch"
region = "lower_body"
duration = 2.0
settle = { blend = 0.3, frames = 8, min_change = 0.0 }
poses = [
    { time = 0.5, set = [{ role = "Hips", axis = "ty", offset = -0.4 }, { role = "Hips", axis = "x", degrees = 10 }] },
    { time = 1.0, set = [{ role = "Hips", axis = "ty", offset = -0.4 }, { role = "Hips", axis = "x", degrees = 10 }] },
]
"#,
        )
        .unwrap()
    }

    #[test]
    fn amount_scales_hips_translation() {
        let table = crouch_table();
        let mut spec = MotionSpec::new("crouch");
        spec.amount = 0.5;
        let keys = compose_motion(&table, &spec).unwrap();

        assert_eq!(
            keys_of(&keys, HumanoidRole::Hips, PoseAxis::TranslationY),
            vec![(0.0, 0.0), (0.5, -0.2), (1.0, -0.2), (2.0, 0.0)]
        );
    }

    #[test]
    fn settle_requests_skip_translation_curves() {
        let table = crouch_table();
        let spec = MotionSpec::new("crouch");
        let keys = compose_motion(&table, &spec).unwrap();
        let requests = settle_requests(&table, &spec, &keys).unwrap();

        let found: Vec<(HumanoidRole, PoseAxis)> =
            requests.iter().map(|r| (r.role, r.axis)).collect();
        assert_eq!(found, vec![(HumanoidRole::Hips, PoseAxis::X)]);
    }

    #[test]
    fn unknown_motion_lists_the_vocabulary() {
        let err = compose_motion(PoseTable::builtin(), &MotionSpec::new("moonwalk")).unwrap_err();
        assert!(err.to_string().contains("punch"));
    }
}
