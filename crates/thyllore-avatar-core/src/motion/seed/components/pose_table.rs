use std::collections::HashMap;
use std::sync::OnceLock;

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use thyllore_anim_core::editable::PropertyType;

use crate::humanoid::components::role::{HumanoidRole, HUMANOID_CHAINS};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PoseAxis {
    X,
    Y,
    Z,
    TranslationX,
    TranslationY,
    TranslationZ,
}

impl PoseAxis {
    pub fn property_type(self) -> PropertyType {
        match self {
            PoseAxis::X => PropertyType::RotationX,
            PoseAxis::Y => PropertyType::RotationY,
            PoseAxis::Z => PropertyType::RotationZ,
            PoseAxis::TranslationX => PropertyType::TranslationX,
            PoseAxis::TranslationY => PropertyType::TranslationY,
            PoseAxis::TranslationZ => PropertyType::TranslationZ,
        }
    }

    fn parse(text: &str) -> Option<PoseAxis> {
        match text {
            "x" => Some(PoseAxis::X),
            "y" => Some(PoseAxis::Y),
            "z" => Some(PoseAxis::Z),
            "tx" => Some(PoseAxis::TranslationX),
            "ty" => Some(PoseAxis::TranslationY),
            "tz" => Some(PoseAxis::TranslationZ),
            _ => None,
        }
    }

    pub fn is_rotation(self) -> bool {
        matches!(self, PoseAxis::X | PoseAxis::Y | PoseAxis::Z)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoleKey {
    pub role: HumanoidRole,
    pub axis: PoseAxis,
    pub value: f32,
}

impl RoleKey {
    pub fn mirrored(self) -> RoleKey {
        let value = match self.axis {
            PoseAxis::X | PoseAxis::TranslationY | PoseAxis::TranslationZ => self.value,
            PoseAxis::Y | PoseAxis::Z | PoseAxis::TranslationX => -self.value,
        };
        RoleKey {
            role: self.role.mirrored(),
            axis: self.axis,
            value,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoseSide {
    Same,
    Other,
    Both,
}

impl PoseSide {
    fn parse(text: &str) -> Option<PoseSide> {
        match text {
            "same" => Some(PoseSide::Same),
            "other" => Some(PoseSide::Other),
            "both" => Some(PoseSide::Both),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PoseEntry {
    Pose { name: String, side: PoseSide },
    Key(RoleKey),
}

#[derive(Clone, Debug)]
pub struct NamedPose {
    pub name: String,
    pub labels: Vec<String>,
    pub keys: Vec<RoleKey>,
}

#[derive(Clone, Debug)]
pub struct MotionPose {
    pub time: f32,
    pub set: Vec<PoseEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyRegion {
    Head,
    UpperBody,
    LowerBody,
    FullBody,
}

impl BodyRegion {
    fn parse(text: &str) -> Option<BodyRegion> {
        match text {
            "head" => Some(BodyRegion::Head),
            "upper_body" => Some(BodyRegion::UpperBody),
            "lower_body" => Some(BodyRegion::LowerBody),
            "full_body" => Some(BodyRegion::FullBody),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Limb {
    Torso,
    Arm,
    Leg,
}

fn limb_of(role: HumanoidRole) -> Option<Limb> {
    HUMANOID_CHAINS
        .iter()
        .position(|chain| chain.contains(&role))
        .map(|chain_index| match chain_index {
            0 => Limb::Torso,
            1 | 2 => Limb::Arm,
            _ => Limb::Leg,
        })
}

/// Copilot follow-through after a pose arrival: how much of the forecast to keep, for how long,
/// and how large a change qualifies as an arrival.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SettleDef {
    pub blend: f32,
    pub frames: usize,
    pub min_change: f32,
}

pub const SETTLE_MAX_FRAMES: usize = 16;

#[derive(Clone, Debug)]
pub struct MotionDef {
    pub name: String,
    pub labels: Vec<String>,
    pub region: BodyRegion,
    pub duration: f32,
    pub cycle: Option<(f32, f32)>,
    pub settle: Option<SettleDef>,
    pub poses: Vec<MotionPose>,
}

#[derive(Debug, Default)]
pub struct PoseTable {
    poses: HashMap<String, NamedPose>,
    motions: Vec<MotionDef>,
}

impl PoseTable {
    pub fn builtin() -> &'static PoseTable {
        static TABLE: OnceLock<PoseTable> = OnceLock::new();
        TABLE.get_or_init(|| {
            PoseTable::parse(include_str!("../../../../data/pose_table.toml"))
                .expect("built-in pose_table.toml is invalid")
        })
    }

    pub fn parse(toml_str: &str) -> Result<PoseTable> {
        let raw: RawTable = toml::from_str(toml_str).context("pose table TOML")?;

        let mut poses = HashMap::new();
        for raw_pose in raw.pose {
            let keys = raw_pose
                .keys
                .iter()
                .map(parse_role_key)
                .collect::<Result<Vec<_>>>()
                .with_context(|| format!("pose '{}'", raw_pose.name))?;
            if poses
                .insert(
                    raw_pose.name.clone(),
                    NamedPose {
                        name: raw_pose.name.clone(),
                        labels: raw_pose.labels,
                        keys,
                    },
                )
                .is_some()
            {
                bail!("pose '{}' is defined twice", raw_pose.name);
            }
        }

        let mut motions = Vec::new();
        for raw_motion in raw.motion {
            let motion = parse_motion(raw_motion, &poses)?;
            if motions.iter().any(|m: &MotionDef| m.name == motion.name) {
                bail!("motion '{}' is defined twice", motion.name);
            }
            motions.push(motion);
        }

        Ok(PoseTable { poses, motions })
    }

    pub fn pose(&self, name: &str) -> Option<&NamedPose> {
        self.poses.get(name)
    }

    pub fn motion(&self, name: &str) -> Option<&MotionDef> {
        self.motions.iter().find(|m| m.name == name)
    }

    pub fn motions(&self) -> &[MotionDef] {
        &self.motions
    }

    pub fn motion_names(&self) -> Vec<&str> {
        self.motions.iter().map(|m| m.name.as_str()).collect()
    }
}

#[derive(Deserialize)]
struct RawTable {
    #[serde(default)]
    pose: Vec<RawPose>,
    #[serde(default)]
    motion: Vec<RawMotion>,
}

#[derive(Deserialize)]
struct RawPose {
    name: String,
    #[serde(default)]
    labels: Vec<String>,
    keys: Vec<RawKey>,
}

#[derive(Deserialize)]
struct RawKey {
    role: String,
    axis: String,
    #[serde(default)]
    degrees: Option<f32>,
    #[serde(default)]
    offset: Option<f32>,
}

#[derive(Deserialize)]
struct RawMotion {
    name: String,
    #[serde(default)]
    labels: Vec<String>,
    region: String,
    duration: f32,
    #[serde(default)]
    cycle: Option<[f32; 2]>,
    #[serde(default)]
    settle: Option<RawSettle>,
    poses: Vec<RawMotionPose>,
}

#[derive(Deserialize)]
struct RawSettle {
    blend: f32,
    frames: usize,
    min_change: f32,
}

#[derive(Deserialize)]
struct RawMotionPose {
    time: f32,
    set: Vec<RawSetEntry>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawSetEntry {
    Pose {
        pose: String,
        #[serde(default)]
        side: Option<String>,
    },
    Key {
        role: String,
        axis: String,
        #[serde(default)]
        degrees: Option<f32>,
        #[serde(default)]
        offset: Option<f32>,
    },
}

fn parse_role_key(raw: &RawKey) -> Result<RoleKey> {
    let role = HumanoidRole::from_unity_name(&raw.role)
        .ok_or_else(|| anyhow::anyhow!("unknown role '{}'", raw.role))?;
    let axis = PoseAxis::parse(&raw.axis)
        .ok_or_else(|| anyhow::anyhow!("axis must be x, y, z, tx, ty or tz, got '{}'", raw.axis))?;
    if !axis.is_rotation() && role != HumanoidRole::Hips {
        bail!(
            "translation axis '{}' is only allowed for Hips, not {}",
            raw.axis,
            raw.role
        );
    }

    let value = match (axis.is_rotation(), raw.degrees, raw.offset) {
        (true, Some(degrees), None) => degrees,
        (false, None, Some(offset)) => offset,
        (true, _, _) => bail!("axis '{}' of {} needs degrees only", raw.axis, raw.role),
        (false, _, _) => bail!("axis '{}' of {} needs offset only", raw.axis, raw.role),
    };
    if !value.is_finite() {
        bail!("value for {} must be finite", raw.role);
    }
    Ok(RoleKey { role, axis, value })
}

fn parse_motion(raw: RawMotion, poses: &HashMap<String, NamedPose>) -> Result<MotionDef> {
    let name = raw.name;
    if !(raw.duration.is_finite() && raw.duration > 0.0) {
        bail!("motion '{name}': duration must be positive");
    }

    let cycle = match raw.cycle {
        Some([start, end]) => {
            if !(0.0 <= start && start < end && end <= raw.duration) {
                bail!("motion '{name}': cycle must satisfy 0 <= start < end <= duration");
            }
            Some((start, end))
        }
        None => None,
    };

    let mut motion_poses = Vec::with_capacity(raw.poses.len());
    for raw_pose in raw.poses {
        if !(raw_pose.time.is_finite() && raw_pose.time > 0.0 && raw_pose.time <= raw.duration) {
            bail!(
                "motion '{name}': pose time {} must lie in (0, duration]",
                raw_pose.time
            );
        }
        if motion_poses
            .iter()
            .any(|p: &MotionPose| (p.time - raw_pose.time).abs() < 1e-6)
        {
            bail!("motion '{name}': two poses share time {}", raw_pose.time);
        }
        let set = raw_pose
            .set
            .into_iter()
            .map(|entry| parse_set_entry(entry, poses))
            .collect::<Result<Vec<_>>>()
            .with_context(|| format!("motion '{name}' pose at {}", raw_pose.time))?;
        motion_poses.push(MotionPose {
            time: raw_pose.time,
            set,
        });
    }
    motion_poses.sort_by(|a, b| a.time.total_cmp(&b.time));

    let region = BodyRegion::parse(&raw.region).ok_or_else(|| {
        anyhow::anyhow!(
            "motion '{name}': region must be head, upper_body, lower_body or full_body, got '{}'",
            raw.region
        )
    })?;
    check_region_covers_touched_limbs(&name, region, &motion_poses, poses)?;

    let settle = raw
        .settle
        .map(|raw_settle| parse_settle(&name, raw_settle))
        .transpose()?;

    Ok(MotionDef {
        name,
        labels: raw.labels,
        region,
        duration: raw.duration,
        cycle,
        settle,
        poses: motion_poses,
    })
}

fn parse_settle(name: &str, raw: RawSettle) -> Result<SettleDef> {
    if !(raw.blend.is_finite() && (0.0..=1.0).contains(&raw.blend)) {
        bail!("motion '{name}': settle.blend must be within 0..=1");
    }
    if !(1..=SETTLE_MAX_FRAMES).contains(&raw.frames) {
        bail!("motion '{name}': settle.frames must be within 1..={SETTLE_MAX_FRAMES}");
    }
    if !(raw.min_change.is_finite() && raw.min_change >= 0.0) {
        bail!("motion '{name}': settle.min_change must be >= 0");
    }
    Ok(SettleDef {
        blend: raw.blend,
        frames: raw.frames,
        min_change: raw.min_change,
    })
}

fn check_region_covers_touched_limbs(
    name: &str,
    region: BodyRegion,
    motion_poses: &[MotionPose],
    poses: &HashMap<String, NamedPose>,
) -> Result<()> {
    let mut touched: Vec<Limb> = Vec::new();
    for pose in motion_poses {
        for entry in &pose.set {
            let roles: Vec<HumanoidRole> = match entry {
                PoseEntry::Pose {
                    name: pose_name, ..
                } => poses[pose_name].keys.iter().map(|k| k.role).collect(),
                PoseEntry::Key(key) => vec![key.role],
            };
            for role in roles {
                if let Some(limb) = limb_of(role) {
                    if !touched.contains(&limb) {
                        touched.push(limb);
                    }
                }
            }
        }
    }
    let has = |limb: Limb| touched.contains(&limb);

    let consistent = match region {
        BodyRegion::Head => !has(Limb::Arm) && !has(Limb::Leg),
        BodyRegion::UpperBody => !has(Limb::Leg),
        BodyRegion::LowerBody => !has(Limb::Arm),
        BodyRegion::FullBody => has(Limb::Leg) && (has(Limb::Arm) || has(Limb::Torso)),
    };
    if !consistent {
        bail!(
            "motion '{name}': region {region:?} does not match the limbs its poses key (full_body needs legs and arms or torso; upper_body must not key legs; lower_body must not key arms)"
        );
    }
    Ok(())
}

fn parse_set_entry(raw: RawSetEntry, poses: &HashMap<String, NamedPose>) -> Result<PoseEntry> {
    match raw {
        RawSetEntry::Pose { pose, side } => {
            if !poses.contains_key(&pose) {
                bail!("unknown pose '{pose}'");
            }
            let side = match side {
                Some(text) => PoseSide::parse(&text).ok_or_else(|| {
                    anyhow::anyhow!("side must be same, other or both, got '{text}'")
                })?,
                None => PoseSide::Same,
            };
            Ok(PoseEntry::Pose { name: pose, side })
        }
        RawSetEntry::Key {
            role,
            axis,
            degrees,
            offset,
        } => Ok(PoseEntry::Key(parse_role_key(&RawKey {
            role,
            axis,
            degrees,
            offset,
        })?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_table_parses_and_names_every_role() {
        let table = PoseTable::builtin();
        assert!(table.motion("punch").is_some());
        assert!(table.pose("hikite").is_some());
        for motion in table.motions() {
            assert!(
                !motion.labels.is_empty(),
                "motion {} has no labels",
                motion.name
            );
        }
    }

    #[test]
    fn unknown_role_or_pose_is_rejected() {
        let bad_role = r#"
[[pose]]
name = "p"
keys = [{ role = "RightElbow", axis = "y", degrees = 10 }]
"#;
        assert!(PoseTable::parse(bad_role).is_err());

        let bad_pose = r#"
[[motion]]
name = "m"
region = "head"
duration = 1.0
poses = [{ time = 0.5, set = [{ pose = "missing" }] }]
"#;
        assert!(PoseTable::parse(bad_pose).is_err());
    }

    #[test]
    fn cycle_outside_duration_is_rejected() {
        let text = r#"
[[motion]]
name = "m"
region = "head"
duration = 1.0
cycle = [0.5, 1.5]
poses = [{ time = 0.5, set = [{ role = "Head", axis = "x", degrees = 10 }] }]
"#;
        assert!(PoseTable::parse(text).is_err());
    }

    #[test]
    fn region_must_match_the_limbs_the_poses_key() {
        let upper_body_keys_legs = r#"
[[motion]]
name = "m"
region = "upper_body"
duration = 1.0
poses = [{ time = 0.5, set = [{ role = "RightUpperLeg", axis = "x", degrees = -30 }] }]
"#;
        assert!(PoseTable::parse(upper_body_keys_legs).is_err());

        let full_body_without_legs = r#"
[[motion]]
name = "m"
region = "full_body"
duration = 1.0
poses = [{ time = 0.5, set = [{ role = "RightUpperArm", axis = "z", degrees = 30 }] }]
"#;
        assert!(PoseTable::parse(full_body_without_legs).is_err());

        for motion in PoseTable::builtin().motions() {
            if motion.name == "punch" {
                assert_eq!(motion.region, BodyRegion::FullBody);
            }
        }
    }

    #[test]
    fn settle_bounds_are_validated() {
        for settle in [
            "{ blend = 1.5, frames = 8, min_change = 20 }",
            "{ blend = 0.3, frames = 0, min_change = 20 }",
            "{ blend = 0.3, frames = 64, min_change = 20 }",
        ] {
            let text = format!(
                r#"
[[motion]]
name = "m"
region = "head"
duration = 1.0
settle = {settle}
poses = [{{ time = 0.5, set = [{{ role = "Head", axis = "x", degrees = 30 }}] }}]
"#
            );
            assert!(PoseTable::parse(&text).is_err(), "{settle}");
        }
        assert!(PoseTable::builtin()
            .motion("punch")
            .unwrap()
            .settle
            .is_some());
    }

    #[test]
    fn mirrored_key_flips_side_and_yz_sign() {
        let key = RoleKey {
            role: HumanoidRole::RightUpperArm,
            axis: PoseAxis::Z,
            value: 90.0,
        };
        let mirrored = key.mirrored();
        assert_eq!(mirrored.role, HumanoidRole::LeftUpperArm);
        assert_eq!(mirrored.value, -90.0);

        let center = RoleKey {
            role: HumanoidRole::Head,
            axis: PoseAxis::X,
            value: 20.0,
        };
        assert_eq!(center.mirrored(), center);
    }

    #[test]
    fn translation_axis_only_allowed_on_hips() {
        let non_hips = r#"
[[pose]]
name = "p"
keys = [{ role = "Spine", axis = "ty", offset = 0.5 }]
"#;
        assert!(PoseTable::parse(non_hips).is_err());
    }

    #[test]
    fn translation_axis_parses_for_hips() {
        let text = r#"
[[pose]]
name = "squat"
keys = [{ role = "Hips", axis = "ty", offset = -0.3 }]
"#;
        let table = PoseTable::parse(text).unwrap();
        let pose = table.pose("squat").unwrap();
        assert_eq!(pose.keys.len(), 1);
        let key = &pose.keys[0];
        assert_eq!(key.role, HumanoidRole::Hips);
        assert_eq!(key.axis, PoseAxis::TranslationY);
        assert_eq!(key.value, -0.3);
    }

    #[test]
    fn translation_mirror_only_flips_tx() {
        let tx = RoleKey {
            role: HumanoidRole::Hips,
            axis: PoseAxis::TranslationX,
            value: 0.5,
        };
        assert_eq!(tx.mirrored().value, -0.5);

        let ty = RoleKey {
            role: HumanoidRole::Hips,
            axis: PoseAxis::TranslationY,
            value: 0.5,
        };
        assert_eq!(ty.mirrored().value, 0.5);

        let tz = RoleKey {
            role: HumanoidRole::Hips,
            axis: PoseAxis::TranslationZ,
            value: 0.5,
        };
        assert_eq!(tz.mirrored().value, 0.5);
    }

    #[test]
    fn missing_value_is_rejected() {
        let no_value = r#"
[[pose]]
name = "p"
keys = [{ role = "Head", axis = "x" }]
"#;
        assert!(PoseTable::parse(no_value).is_err());
    }

    #[test]
    fn both_degrees_and_offset_is_rejected() {
        let both = r#"
[[pose]]
name = "p"
keys = [{ role = "Head", axis = "x", degrees = 10, offset = 0.5 }]
"#;
        assert!(PoseTable::parse(both).is_err());
    }

    #[test]
    fn rotation_axis_needs_degrees_not_offset() {
        let wrong = r#"
[[pose]]
name = "p"
keys = [{ role = "Head", axis = "x", offset = 0.5 }]
"#;
        assert!(PoseTable::parse(wrong).is_err());
    }

    #[test]
    fn translation_axis_needs_offset_not_degrees() {
        let wrong = r#"
[[pose]]
name = "p"
keys = [{ role = "Hips", axis = "ty", degrees = 0.5 }]
"#;
        assert!(PoseTable::parse(wrong).is_err());
    }
}
