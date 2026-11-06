use std::rc::Rc;
use std::sync::LazyLock;

use cgmath::{Quaternion, Vector3};
use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields, UiParam};

use crate::animation::{BoneId, ConstraintId, ConstraintType};
use crate::ecs::systems::phases::event_dispatch::constraint::ConstraintEvent;
use crate::ecs::world::Entity;
use crate::math::{euler_degrees_to_quaternion, quaternion_to_euler_degrees};

use crate::platform::ui::bone_choice::bone_options;
use crate::platform::ui::ui_rows::UiRow;

const DEFAULT_POLE_VECTOR: [f32; 3] = [0.0, 0.0, 1.0];

#[derive(Clone)]
pub struct ConstraintView {
    pub entity: Entity,
    pub entry_id: ConstraintId,
    pub constraint: ConstraintType,
    pub bones: Rc<[(BoneId, String)]>,
    pub selected_bone: Option<BoneId>,
    pub fields: ConstraintFields,
}

#[derive(Clone, Debug, Default, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = thyllore_scene_core::SingleOwner, owner = Component)]
pub struct ConstraintFields {
    #[persist(ui(min = 0.0, max = 1.0))]
    pub weight: f32,
    #[persist(ui(min = -180.0, max = 180.0, format = "%.1f"))]
    pub twist: f32,
    #[persist(ui(min = 1.0, max = 16.0, format = "%.0f"))]
    pub chain_length: u32,
    #[persist(ui(kind = Offset, min = -10.0, max = 10.0))]
    pub pole_vector: [f32; 3],
    #[persist(ui(kind = Offset, min = -1.0, max = 1.0))]
    pub aim_axis: [f32; 3],
    #[persist(ui(kind = Offset, min = -1.0, max = 1.0))]
    pub up_axis: [f32; 3],
    #[persist(ui(kind = Offset, min = -10.0, max = 10.0))]
    pub offset: [f32; 3],
    #[persist(ui(kind = Offset, min = -180.0, max = 180.0, format = "%.1f"))]
    pub rotation_offset: [f32; 3],
}

impl ConstraintFields {
    pub fn from_constraint(constraint: &ConstraintType) -> Self {
        match constraint {
            ConstraintType::Ik(data) => Self {
                weight: data.weight,
                twist: data.twist,
                chain_length: data.chain_length,
                pole_vector: displayed_pole_vector(data.pole_vector),
                ..Self::default()
            },
            ConstraintType::Aim(data) => Self {
                weight: data.weight,
                aim_axis: data.aim_axis.into(),
                up_axis: data.up_axis.into(),
                ..Self::default()
            },
            ConstraintType::Parent(data) => Self {
                weight: data.weight,
                ..Self::default()
            },
            ConstraintType::Position(data) => Self {
                weight: data.weight,
                offset: data.offset.into(),
                ..Self::default()
            },
            ConstraintType::Rotation(data) => Self {
                weight: data.weight,
                rotation_offset: displayed_rotation_offset(data.offset),
                ..Self::default()
            },
            ConstraintType::Scale(data) => Self {
                weight: data.weight,
                offset: data.offset.into(),
                ..Self::default()
            },
        }
    }

    pub fn apply_to(&self, constraint: &mut ConstraintType) {
        match constraint {
            ConstraintType::Ik(data) => {
                data.weight = self.weight;
                data.twist = self.twist;
                data.chain_length = self.chain_length;
                if self.pole_vector != displayed_pole_vector(data.pole_vector) {
                    data.pole_vector = Some(self.pole_vector.into());
                }
            }
            ConstraintType::Aim(data) => {
                data.weight = self.weight;
                data.aim_axis = self.aim_axis.into();
                data.up_axis = self.up_axis.into();
            }
            ConstraintType::Parent(data) => {
                data.weight = self.weight;
            }
            ConstraintType::Position(data) => {
                data.weight = self.weight;
                data.offset = self.offset.into();
            }
            ConstraintType::Rotation(data) => {
                data.weight = self.weight;
                if self.rotation_offset != displayed_rotation_offset(data.offset) {
                    data.offset = euler_degrees_to_quaternion(&self.rotation_offset.into());
                }
            }
            ConstraintType::Scale(data) => {
                data.weight = self.weight;
                data.offset = self.offset.into();
            }
        }
    }
}

fn displayed_pole_vector(pole_vector: Option<Vector3<f32>>) -> [f32; 3] {
    pole_vector.map_or(DEFAULT_POLE_VECTOR, Into::into)
}

fn displayed_rotation_offset(offset: Quaternion<f32>) -> [f32; 3] {
    quaternion_to_euler_degrees(&offset).into()
}

struct ConstraintFieldsPath;

impl FieldPath<ConstraintView> for ConstraintFieldsPath {
    type Field = ConstraintFields;

    fn get(root: &ConstraintView) -> &ConstraintFields {
        &root.fields
    }

    fn get_mut(root: &mut ConstraintView) -> &mut ConstraintFields {
        &mut root.fields
    }
}

static CONSTRAINT_SCALARS: LazyLock<Vec<ScalarParam<ConstraintView>>> = LazyLock::new(|| {
    let mut out = Vec::new();
    ConstraintFields::collect_scalars::<ConstraintView, ConstraintFieldsPath>("", &mut out);
    out
});

static CONSTRAINT_UI_PARAMS: LazyLock<Vec<UiParam>> = LazyLock::new(|| {
    let mut out = Vec::new();
    ConstraintFields::collect_ui("", "", &mut out);
    out
});

pub struct ConstraintParams {
    pub weight: &'static UiParam,
    pub twist: &'static UiParam,
    pub chain_length: &'static UiParam,
    pub pole_vector: &'static UiParam,
    pub aim_axis: &'static UiParam,
    pub up_axis: &'static UiParam,
    pub offset: &'static UiParam,
    pub rotation_offset: &'static UiParam,
}

pub fn constraint_params() -> ConstraintParams {
    let [weight, twist, chain_length, pole_vector, aim_axis, up_axis, offset, rotation_offset] =
        CONSTRAINT_UI_PARAMS.as_slice()
    else {
        unreachable!("ConstraintFields declares weight, twist, chain_length, pole_vector, aim_axis, up_axis, offset and rotation_offset");
    };
    ConstraintParams {
        weight,
        twist,
        chain_length,
        pole_vector,
        aim_axis,
        up_axis,
        offset,
        rotation_offset,
    }
}

pub fn scalar_row(param: &'static UiParam) -> UiRow<ConstraintView> {
    UiRow::Scalar {
        param,
        scalars: &CONSTRAINT_SCALARS,
    }
}

pub fn common_rows() -> Vec<UiRow<ConstraintView>> {
    vec![
        UiRow::Toggle {
            label: "Enabled",
            get: |view: &ConstraintView| view.constraint.is_enabled(),
            set: |view: &mut ConstraintView, value: bool| view.constraint.set_enabled(value),
        },
        scalar_row(constraint_params().weight),
    ]
}

pub fn applied_to_rows() -> [UiRow<ConstraintView>; 2] {
    [
        UiRow::Choice {
            label: "Applied To",
            options: |_, view: &ConstraintView| {
                bone_options(
                    &view.bones,
                    view.selected_bone,
                    view.constraint.constrained_bone_id(),
                )
            },
            pick: |view: &mut ConstraintView, index: usize| {
                if let Some(&(bone_id, _)) = view.bones.get(index) {
                    view.constraint.set_constrained_bone_id(bone_id);
                }
            },
        },
        UiRow::Apply {
            label: "Use Selected for Applied To",
            enabled: |_, view: &ConstraintView| view.selected_bone.is_some(),
            apply: |view: &mut ConstraintView| {
                if let Some(bone_id) = view.selected_bone {
                    view.constraint.set_constrained_bone_id(bone_id);
                }
            },
        },
    ]
}

pub fn remove_row() -> UiRow<ConstraintView> {
    UiRow::Action {
        label: "Remove",
        enabled: |_, _: &ConstraintView| true,
        on_click: |view: &ConstraintView| Box::new(remove_event(view)) as _,
    }
}

pub fn update_event(view: &ConstraintView) -> ConstraintEvent {
    let mut constraint = view.constraint.clone();
    view.fields.apply_to(&mut constraint);
    ConstraintEvent::Update {
        entity: view.entity,
        constraint_id: view.entry_id,
        constraint,
    }
}

pub fn remove_event(view: &ConstraintView) -> ConstraintEvent {
    ConstraintEvent::Remove {
        entity: view.entity,
        constraint_id: view.entry_id,
    }
}

#[cfg(test)]
mod tests {
    use cgmath::InnerSpace;

    use super::*;
    use crate::animation::{
        AimConstraintData, IkConstraintData, PositionConstraintData, RotationConstraintData,
        ScaleConstraintData,
    };

    fn round_trip(constraint: &ConstraintType) -> ConstraintType {
        let mut applied = constraint.clone();
        ConstraintFields::from_constraint(constraint).apply_to(&mut applied);
        applied
    }

    #[test]
    fn ik_round_trip_keeps_missing_pole_vector() {
        let constraint = ConstraintType::Ik(IkConstraintData {
            chain_length: 3,
            weight: 0.5,
            twist: 45.0,
            pole_vector: None,
            ..Default::default()
        });

        let ConstraintType::Ik(data) = round_trip(&constraint) else {
            panic!("round trip changed the constraint kind");
        };
        assert_eq!(data.chain_length, 3);
        assert_eq!(data.weight, 0.5);
        assert_eq!(data.twist, 45.0);
        assert_eq!(data.pole_vector, None);
    }

    #[test]
    fn ik_round_trip_keeps_pole_vector() {
        let constraint = ConstraintType::Ik(IkConstraintData {
            pole_vector: Some(Vector3::new(1.0, 2.0, 3.0)),
            ..Default::default()
        });

        let ConstraintType::Ik(data) = round_trip(&constraint) else {
            panic!("round trip changed the constraint kind");
        };
        assert_eq!(data.pole_vector, Some(Vector3::new(1.0, 2.0, 3.0)));
    }

    #[test]
    fn ik_pole_vector_is_written_once_edited() {
        let mut constraint = ConstraintType::Ik(IkConstraintData {
            pole_vector: None,
            ..Default::default()
        });
        let mut fields = ConstraintFields::from_constraint(&constraint);
        fields.pole_vector = [1.0, 0.0, 0.0];

        fields.apply_to(&mut constraint);

        let ConstraintType::Ik(data) = constraint else {
            panic!("apply_to changed the constraint kind");
        };
        assert_eq!(data.pole_vector, Some(Vector3::new(1.0, 0.0, 0.0)));
    }

    #[test]
    fn aim_round_trip_keeps_axes() {
        let constraint = ConstraintType::Aim(AimConstraintData {
            weight: 0.6,
            aim_axis: Vector3::new(1.0, 0.0, 0.0),
            up_axis: Vector3::new(0.0, 0.0, 1.0),
            ..Default::default()
        });

        let ConstraintType::Aim(data) = round_trip(&constraint) else {
            panic!("round trip changed the constraint kind");
        };
        assert_eq!(data.weight, 0.6);
        assert_eq!(data.aim_axis, Vector3::new(1.0, 0.0, 0.0));
        assert_eq!(data.up_axis, Vector3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn position_round_trip_keeps_offset() {
        let constraint = ConstraintType::Position(PositionConstraintData {
            weight: 0.4,
            offset: Vector3::new(1.0, -2.0, 3.0),
            ..Default::default()
        });

        let ConstraintType::Position(data) = round_trip(&constraint) else {
            panic!("round trip changed the constraint kind");
        };
        assert_eq!(data.weight, 0.4);
        assert_eq!(data.offset, Vector3::new(1.0, -2.0, 3.0));
    }

    #[test]
    fn scale_round_trip_keeps_offset() {
        let constraint = ConstraintType::Scale(ScaleConstraintData {
            offset: Vector3::new(2.0, 3.0, 4.0),
            ..Default::default()
        });

        let ConstraintType::Scale(data) = round_trip(&constraint) else {
            panic!("round trip changed the constraint kind");
        };
        assert_eq!(data.offset, Vector3::new(2.0, 3.0, 4.0));
    }

    #[test]
    fn rotation_round_trip_keeps_quaternion() {
        let offset = euler_degrees_to_quaternion(&Vector3::new(10.0, 20.0, 30.0));
        let constraint = ConstraintType::Rotation(RotationConstraintData {
            weight: 0.7,
            offset,
            ..Default::default()
        });

        let ConstraintType::Rotation(data) = round_trip(&constraint) else {
            panic!("round trip changed the constraint kind");
        };
        assert_eq!(data.weight, 0.7);
        assert_eq!(data.offset, offset);
    }

    #[test]
    fn rotation_offset_is_written_once_edited() {
        let mut constraint = ConstraintType::Rotation(RotationConstraintData::default());
        let mut fields = ConstraintFields::from_constraint(&constraint);
        fields.rotation_offset = [0.0, 45.0, 0.0];

        fields.apply_to(&mut constraint);

        let ConstraintType::Rotation(data) = constraint else {
            panic!("apply_to changed the constraint kind");
        };
        let displayed = quaternion_to_euler_degrees(&data.offset);
        assert!((displayed - Vector3::new(0.0, 45.0, 0.0)).magnitude() < 1e-3);
    }
}
