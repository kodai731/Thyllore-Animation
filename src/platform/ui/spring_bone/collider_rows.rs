use std::sync::LazyLock;

use thyllore_anim_core::spring_bone::{ColliderShape, SpringColliderDef};
use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields};

use crate::animation::BoneId;
use crate::ecs::systems::phases::event_dispatch::spring_bone::SpringBoneEvent;
use crate::ecs::world::Entity;

use super::super::bone_choice::bone_options;
use super::super::ui_rows::{ChoiceOption, ListRows, UiRow};
use super::section::SpringView;

#[derive(Clone)]
pub struct ColliderView {
    pub entity: Entity,
    pub bones: std::rc::Rc<[(BoneId, String)]>,
    pub selected_bone: Option<BoneId>,
    pub collider: SpringColliderDef,
    pub fields: ColliderFields,
}

#[derive(Clone, Debug, Default, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = thyllore_scene_core::SingleOwner, owner = Component)]
pub struct ColliderFields {
    #[persist(ui(kind = Offset, min = -1.0, max = 1.0))]
    pub offset: [f32; 3],
    #[persist(ui(min = 0.001, max = 1.0, format = "%.3f"))]
    pub radius: f32,
    #[persist(ui(kind = Offset, min = -1.0, max = 1.0))]
    pub tail: [f32; 3],
}

impl ColliderFields {
    pub fn from_collider(collider: &SpringColliderDef) -> Self {
        let (radius, tail) = match &collider.shape {
            ColliderShape::Sphere { radius } => match ColliderShape::default_capsule() {
                ColliderShape::Capsule { tail, .. } => (*radius, tail),
                ColliderShape::Sphere { .. } => unreachable!("default_capsule is a capsule"),
            },
            ColliderShape::Capsule { radius, tail } => (*radius, *tail),
        };
        Self {
            offset: collider.offset.into(),
            radius,
            tail: tail.into(),
        }
    }

    pub fn apply_to(&self, collider: &mut SpringColliderDef) {
        collider.offset.x = self.offset[0];
        collider.offset.y = self.offset[1];
        collider.offset.z = self.offset[2];
        match &mut collider.shape {
            ColliderShape::Sphere { radius } => {
                *radius = self.radius;
            }
            ColliderShape::Capsule { radius, tail } => {
                *radius = self.radius;
                tail.x = self.tail[0];
                tail.y = self.tail[1];
                tail.z = self.tail[2];
            }
        }
    }
}

struct ColliderPath;

impl FieldPath<ColliderView> for ColliderPath {
    type Field = ColliderFields;

    fn get(root: &ColliderView) -> &ColliderFields {
        &root.fields
    }

    fn get_mut(root: &mut ColliderView) -> &mut ColliderFields {
        &mut root.fields
    }
}

static COLLIDER_SCALARS: LazyLock<Vec<ScalarParam<ColliderView>>> = LazyLock::new(|| {
    let mut out = Vec::new();
    ColliderFields::collect_scalars::<ColliderView, ColliderPath>("", &mut out);
    out
});

static COLLIDER_UI_PARAMS: LazyLock<Vec<thyllore_scene_core::UiParam>> = LazyLock::new(|| {
    let mut out = Vec::new();
    ColliderFields::collect_ui("", "", &mut out);
    out
});

pub fn collider_list_row() -> UiRow<SpringView> {
    UiRow::List(Box::new(ListRows::<SpringView, ColliderView> {
        title: "Colliders",
        items: |_, view: &SpringView| {
            view.colliders
                .iter()
                .map(|collider| {
                    let shape_name = match &collider.shape {
                        ColliderShape::Sphere { .. } => "Sphere",
                        ColliderShape::Capsule { .. } => "Capsule",
                    };
                    let header = format!(
                        "{} (id:{}, bone:{})",
                        shape_name, collider.id, collider.bone_id
                    );
                    let item = ColliderView {
                        entity: view.entity,
                        bones: view.bones.clone(),
                        selected_bone: view.selected_bone,
                        collider: collider.clone(),
                        fields: ColliderFields::from_collider(collider),
                    };
                    (header, item)
                })
                .collect()
        },
        rows: collider_rows,
        on_change: |_, _, collider_view| {
            let mut collider = collider_view.collider.clone();
            collider_view.fields.apply_to(&mut collider);
            Box::new(SpringBoneEvent::ColliderUpdate {
                entity: collider_view.entity,
                collider_id: collider.id,
                collider,
            }) as _
        },
        initially_open: |_| false,
    }))
}

pub fn collider_rows(view: &ColliderView) -> Vec<UiRow<ColliderView>> {
    let [offset_param, radius_param, tail_param] = COLLIDER_UI_PARAMS.as_slice() else {
        unreachable!("ColliderFields declares offset, radius and tail");
    };
    let mut rows: Vec<UiRow<ColliderView>> = Vec::new();

    rows.push(UiRow::Choice {
        label: "Bone",
        options: |_, view: &ColliderView| {
            bone_options(&view.bones, view.selected_bone, view.collider.bone_id)
        },
        pick: |view: &mut ColliderView, index: usize| {
            if let Some((bone_id, _)) = view.bones.get(index) {
                view.collider.bone_id = *bone_id;
            }
        },
    });

    rows.push(UiRow::Apply {
        label: "Use Selected Bone",
        enabled: |_, view: &ColliderView| view.selected_bone.is_some(),
        apply: |view: &mut ColliderView| {
            if let Some(bone_id) = view.selected_bone {
                view.collider.bone_id = bone_id;
            }
        },
    });

    rows.push(UiRow::Scalar {
        param: offset_param,
        scalars: &COLLIDER_SCALARS,
    });

    rows.push(UiRow::Choice {
        label: "Shape",
        options: |_, view: &ColliderView| {
            let sphere_selected = matches!(view.collider.shape, ColliderShape::Sphere { .. });
            vec![
                ChoiceOption {
                    label: "Sphere".to_string(),
                    selected: sphere_selected,
                },
                ChoiceOption {
                    label: "Capsule".to_string(),
                    selected: !sphere_selected,
                },
            ]
        },
        pick: |view: &mut ColliderView, index: usize| {
            view.collider.shape = match index {
                0 => ColliderShape::default_sphere(),
                1 => ColliderShape::default_capsule(),
                _ => return,
            };
            let picked_fields = ColliderFields::from_collider(&view.collider);
            view.fields.radius = picked_fields.radius;
            view.fields.tail = picked_fields.tail;
        },
    });

    rows.push(UiRow::Scalar {
        param: radius_param,
        scalars: &COLLIDER_SCALARS,
    });

    if matches!(view.collider.shape, ColliderShape::Capsule { .. }) {
        rows.push(UiRow::Scalar {
            param: tail_param,
            scalars: &COLLIDER_SCALARS,
        });
    }

    rows.push(UiRow::Action {
        label: "Remove Collider",
        enabled: |_, _: &ColliderView| true,
        on_click: |view: &ColliderView| {
            Box::new(SpringBoneEvent::ColliderRemove {
                entity: view.entity,
                collider_id: view.collider.id,
            }) as _
        },
    });

    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn make_sphere_collider_view(bone_id: BoneId) -> ColliderView {
        let collider = SpringColliderDef {
            id: 0,
            bone_id,
            offset: cgmath::Vector3::new(0.0, 0.0, 0.0),
            shape: ColliderShape::default_sphere(),
        };
        ColliderView {
            entity: 0,
            bones: std::rc::Rc::new([(bone_id, "test".to_string())]),
            selected_bone: None,
            fields: ColliderFields::from_collider(&collider),
            collider,
        }
    }

    fn make_capsule_collider_view(bone_id: BoneId) -> ColliderView {
        let collider = SpringColliderDef {
            id: 0,
            bone_id,
            offset: cgmath::Vector3::new(0.0, 0.0, 0.0),
            shape: ColliderShape::default_capsule(),
        };
        ColliderView {
            entity: 0,
            bones: std::rc::Rc::new([(bone_id, "test".to_string())]),
            selected_bone: None,
            fields: ColliderFields::from_collider(&collider),
            collider,
        }
    }

    #[test]
    fn collider_rows_sphere_label_order() {
        let view = make_sphere_collider_view(0);
        let rows = collider_rows(&view);
        let labels = visible_labels(&rows);
        assert_eq!(
            labels,
            [
                "Bone",
                "Use Selected Bone",
                "offset",
                "Shape",
                "radius",
                "Remove Collider"
            ]
        );
    }

    #[test]
    fn collider_rows_capsule_label_order() {
        let view = make_capsule_collider_view(0);
        let rows = collider_rows(&view);
        let labels = visible_labels(&rows);
        assert_eq!(
            labels,
            [
                "Bone",
                "Use Selected Bone",
                "offset",
                "Shape",
                "radius",
                "tail",
                "Remove Collider"
            ]
        );
    }

    #[test]
    fn collider_rows_no_duplicate_labels() {
        let view = make_capsule_collider_view(0);
        let rows = collider_rows(&view);
        assert!(find_duplicate_labels(&rows).is_empty());
    }

    #[test]
    fn collider_fields_roundtrip_sphere() {
        let collider = SpringColliderDef {
            id: 1,
            bone_id: 5,
            offset: cgmath::Vector3::new(0.2, -0.3, 0.4),
            shape: ColliderShape::Sphere { radius: 0.25 },
        };
        let fields = ColliderFields::from_collider(&collider);
        assert_eq!(fields.offset, [0.2, -0.3, 0.4]);
        assert_eq!(fields.radius, 0.25);

        let mut cloned = collider.clone();
        fields.apply_to(&mut cloned);
        assert_eq!(cloned.offset.x, 0.2);
        assert_eq!(cloned.offset.y, -0.3);
        assert_eq!(cloned.offset.z, 0.4);
        match &cloned.shape {
            ColliderShape::Sphere { radius } => assert_eq!(*radius, 0.25),
            _ => panic!("expected Sphere"),
        }
    }

    #[test]
    fn collider_fields_roundtrip_capsule() {
        let collider = SpringColliderDef {
            id: 1,
            bone_id: 5,
            offset: cgmath::Vector3::new(0.2, -0.3, 0.4),
            shape: ColliderShape::Capsule {
                radius: 0.25,
                tail: cgmath::Vector3::new(0.0, 0.35, 0.0),
            },
        };
        let fields = ColliderFields::from_collider(&collider);
        assert_eq!(fields.offset, [0.2, -0.3, 0.4]);
        assert_eq!(fields.radius, 0.25);
        assert_eq!(fields.tail, [0.0, 0.35, 0.0]);

        let mut cloned = collider.clone();
        fields.apply_to(&mut cloned);
        assert_eq!(cloned.offset.x, 0.2);
        assert_eq!(cloned.offset.y, -0.3);
        assert_eq!(cloned.offset.z, 0.4);
        match &cloned.shape {
            ColliderShape::Capsule { radius, tail } => {
                assert_eq!(*radius, 0.25);
                assert_eq!(tail.x, 0.0);
                assert_eq!(tail.y, 0.35);
                assert_eq!(tail.z, 0.0);
            }
            _ => panic!("expected Capsule"),
        }
    }
}
