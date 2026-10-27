use thyllore_scene_core::{find_scalar_param, ScalarParam, UiKind, UiParam};

#[derive(Clone, Debug, serde::Serialize)]
pub struct ParamRow {
    pub name: &'static str,
    pub label: String,
    pub min: f32,
    pub max: f32,
    pub format: &'static str,
    pub tooltip: &'static str,
    pub kind: &'static str,
    pub default: ParamDefault,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ParamDefault {
    Scalar(f32),
    Vector([f32; 3]),
}

pub fn param_rows<T: 'static>(
    ui_params: &[UiParam],
    scalar_params: &[ScalarParam<T>],
    defaults: &T,
) -> Vec<ParamRow> {
    ui_params
        .iter()
        .map(|param| {
            let kind = param.kind.name();
            let default = match param.kind {
                UiKind::Offset => {
                    let values: [f32; 3] = param
                        .scalar_accessor_names()
                        .iter()
                        .map(|accessor_name| {
                            let scalar = find_scalar_param(scalar_params, accessor_name)
                                .expect("every ui param component has a scalar entry");
                            (scalar.get)(defaults)
                        })
                        .collect::<Vec<_>>()
                        .try_into()
                        .expect("offset params have exactly 3 components");
                    ParamDefault::Vector(values)
                }
                _ => {
                    let scalar = find_scalar_param(scalar_params, param.name)
                        .expect("every ui param has a scalar entry");
                    ParamDefault::Scalar((scalar.get)(defaults))
                }
            };
            ParamRow {
                name: param.name,
                label: param.display_label().into_owned(),
                min: param.min,
                max: param.max,
                format: param.format,
                tooltip: param.tooltip,
                kind,
                default,
            }
        })
        .collect()
}

pub fn apply_scalar_pairs<T: 'static>(
    target: &mut T,
    scalar_params: &[ScalarParam<T>],
    pairs: &[(String, f32)],
) -> anyhow::Result<()> {
    let valid_names: Vec<&str> = scalar_params.iter().map(|p| p.name).collect();
    for (name, value) in pairs {
        let scalar = scalar_params
            .iter()
            .find(|p| p.name == name || p.renamed_from.contains(&name.as_str()))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "unknown setting \"{}\" (valid: {})",
                    name,
                    valid_names.join(", ")
                )
            })?;
        (scalar.set)(target, *value);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use thyllore_anim_core::spring_bone::{
        CapsuleFitParams, SpringJointParam, CAPSULE_FIT_SCALAR_PARAMS, CAPSULE_FIT_UI_PARAMS,
        SPRING_JOINT_SCALAR_PARAMS, SPRING_JOINT_UI_PARAMS,
    };

    #[test]
    fn param_rows_spring_joint_gravity_dir_is_vector() {
        let rows = param_rows(
            &SPRING_JOINT_UI_PARAMS,
            &SPRING_JOINT_SCALAR_PARAMS,
            &SpringJointParam::default(),
        );
        let gravity_dir = rows.iter().find(|r| r.name == "gravity_dir").unwrap();
        assert_eq!(gravity_dir.kind, "offset");
        assert_eq!(gravity_dir.default, ParamDefault::Vector([0.0, -1.0, 0.0]));
    }

    #[test]
    fn param_rows_spring_joint_drag_force_is_scalar() {
        let rows = param_rows(
            &SPRING_JOINT_UI_PARAMS,
            &SPRING_JOINT_SCALAR_PARAMS,
            &SpringJointParam::default(),
        );
        let drag_force = rows.iter().find(|r| r.name == "drag_force").unwrap();
        assert_eq!(drag_force.kind, "scalar");
        assert_eq!(
            drag_force.default,
            ParamDefault::Scalar(SpringJointParam::default().drag_force)
        );
    }

    #[test]
    fn param_rows_capsule_fit_defaults() {
        let rows = param_rows(
            &CAPSULE_FIT_UI_PARAMS,
            &CAPSULE_FIT_SCALAR_PARAMS,
            &CapsuleFitParams::default(),
        );
        assert_eq!(rows.len(), 4);
        let min_weight = rows.iter().find(|r| r.name == "min_weight").unwrap();
        assert_eq!(min_weight.default, ParamDefault::Scalar(0.5));
        let radius_percentile = rows.iter().find(|r| r.name == "radius_percentile").unwrap();
        assert_eq!(radius_percentile.default, ParamDefault::Scalar(0.9));
        let radius_scale = rows.iter().find(|r| r.name == "radius_scale").unwrap();
        assert_eq!(radius_scale.default, ParamDefault::Scalar(0.85));
        let min_vertex_count = rows.iter().find(|r| r.name == "min_vertex_count").unwrap();
        assert_eq!(min_vertex_count.default, ParamDefault::Scalar(8.0));
    }

    #[test]
    fn apply_scalar_pairs_unknown_name_is_err() {
        let mut target = SpringJointParam::default();
        let err = apply_scalar_pairs(
            &mut target,
            &SPRING_JOINT_SCALAR_PARAMS,
            &[("unknown".into(), 1.0)],
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown"),
            "error should mention 'unknown': {msg}"
        );
        assert!(
            msg.contains("drag_force"),
            "error should list valid names including 'drag_force': {msg}"
        );
    }
}
