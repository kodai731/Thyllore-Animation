use crate::water::{inverse_view_proj_f64, ABSORPTION_REFERENCE_DISTANCE};
use cgmath::Matrix4;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::de::DeserializeOwned;
use serde::Serialize;
use thyllore_scene_core::{UiKind, UiParam};
use thyllore_spirv_reflect::GpuBlock;

/// One effect as seen from Python: presets, UI parameters and the placement every pack call sets.
pub trait PyEffect: Default + Serialize + DeserializeOwned {
    const PRESET_NAMES: &'static [&'static str];
    const UI_PARAMS: &'static [UiParam];

    fn apply_preset(&mut self, name: &str) -> bool;
    fn overwrite_persisted_fields(&mut self, source: &Self);
    fn set_placement(&mut self, time: f32, position: [f32; 3], rotation: [f32; 4]);
    fn parameter_owner_name(name: &str) -> &'static str;
}

fn value_error(message: String) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(message)
}

fn ui_kind_name(kind: UiKind) -> &'static str {
    match kind {
        UiKind::Scalar => "scalar",
        UiKind::Color => "color",
        UiKind::Absorption => "absorption",
    }
}

fn fill_ui_param_dict(dict: &Bound<'_, PyDict>, param: &UiParam) -> PyResult<()> {
    dict.set_item("name", param.name)?;
    dict.set_item("label", param.display_label())?;
    dict.set_item("kind", ui_kind_name(param.kind))?;
    dict.set_item("min", param.min)?;
    dict.set_item("max", param.max)?;
    dict.set_item("format", param.format)?;
    dict.set_item("tooltip", param.tooltip)?;
    dict.set_item("persisted", param.persisted)?;
    match param.kind {
        UiKind::Scalar | UiKind::Color => {}
        UiKind::Absorption => {
            dict.set_item("reference_distance", ABSORPTION_REFERENCE_DISTANCE)?;
        }
    }
    Ok(())
}

fn effect_dict<'py, E: PyEffect>(py: Python<'py>, effect: &E) -> PyResult<Bound<'py, PyDict>> {
    Ok(pythonize::pythonize(py, effect)?.cast_into::<PyDict>()?)
}

pub fn preset_names<E: PyEffect>() -> Vec<&'static str> {
    E::PRESET_NAMES.to_vec()
}

pub fn ui_params<E: PyEffect>(py: Python<'_>) -> PyResult<Bound<'_, PyList>> {
    let default_dict = effect_dict(py, &E::default())?;
    let list = PyList::empty(py);
    for param in E::UI_PARAMS {
        let dict = PyDict::new(py);
        fill_ui_param_dict(&dict, param)?;

        let Some(default_value) = default_dict.get_item(param.name)? else {
            continue;
        };
        dict.set_item("default", default_value)?;
        dict.set_item("owner", E::parameter_owner_name(param.name))?;

        list.append(dict)?;
    }
    Ok(list)
}

pub fn preset_params<'py, E: PyEffect>(
    py: Python<'py>,
    name: &str,
) -> PyResult<Bound<'py, PyDict>> {
    let mut effect = E::default();
    if !effect.apply_preset(name) {
        return Err(value_error(format!("unknown preset: {}", name)));
    }
    effect_dict(py, &effect)
}

fn reject_unknown_parameters(
    params: &Bound<'_, PyDict>,
    known: &Bound<'_, PyDict>,
) -> PyResult<()> {
    for key in params.keys() {
        if !known.contains(&key)? {
            let key_str: &str = key.extract()?;
            return Err(value_error(format!("unknown parameter: {}", key_str)));
        }
    }
    Ok(())
}

/// Default effect overwritten by the persisted fields of `params`, placed at the given pose.
pub fn build_effect_from_params<E: PyEffect>(
    py: Python<'_>,
    params: &Bound<'_, PyDict>,
    time: f32,
    position: [f32; 3],
    rotation: [f32; 4],
) -> PyResult<E> {
    let merged = effect_dict(py, &E::default())?;
    reject_unknown_parameters(params, &merged)?;
    merged.update(params.as_mapping())?;

    let source: E = pythonize::depythonize(&merged)
        .map_err(|e| value_error(format!("failed to deserialize parameters: {}", e)))?;

    let mut effect = E::default();
    effect.overwrite_persisted_fields(&source);
    effect.set_placement(time, position, rotation);
    Ok(effect)
}

pub fn gpu_block_bytes<T: GpuBlock>(block: &T) -> Vec<u8> {
    block.as_bytes().to_vec()
}

/// Row-major Blender-to-engine axis conversion, the single source the addon's `coordinates.C` mirrors.
#[pyfunction]
pub fn blender_to_engine_matrix() -> [[f32; 4]; 4] {
    thyllore_math_core::blender_to_engine_rows()
}

pub fn matrix_from_column_major(values: [f32; 16]) -> Matrix4<f32> {
    Matrix4::new(
        values[0], values[1], values[2], values[3], values[4], values[5], values[6], values[7],
        values[8], values[9], values[10], values[11], values[12], values[13], values[14],
        values[15],
    )
}

pub fn inverse_view_proj_from_column_major(view: [f32; 16], proj: [f32; 16]) -> Matrix4<f32> {
    inverse_view_proj_f64(
        matrix_from_column_major(proj),
        matrix_from_column_major(view),
    )
}

/// Stamps out the preset / UI parameter pyfunctions of one effect and its module registration.
macro_rules! declare_effect_pyfunctions {
    (
        effect: $effect:ty,
        preset_names: $preset_names:ident,
        ui_params: $ui_params:ident,
        preset_params: $preset_params:ident,
        extra: [$($extra:ident),* $(,)?]
    ) => {
        #[pyo3::pyfunction]
        pub fn $preset_names() -> Vec<&'static str> {
            $crate::pybindings::effect::preset_names::<$effect>()
        }

        #[pyo3::pyfunction]
        pub fn $ui_params(
            py: pyo3::Python<'_>,
        ) -> pyo3::PyResult<pyo3::Bound<'_, pyo3::types::PyList>> {
            $crate::pybindings::effect::ui_params::<$effect>(py)
        }

        #[pyo3::pyfunction]
        pub fn $preset_params<'py>(
            py: pyo3::Python<'py>,
            name: &str,
        ) -> pyo3::PyResult<pyo3::Bound<'py, pyo3::types::PyDict>> {
            $crate::pybindings::effect::preset_params::<$effect>(py, name)
        }

        pub fn register(m: &pyo3::Bound<pyo3::types::PyModule>) -> pyo3::PyResult<()> {
            use pyo3::prelude::PyModuleMethods;
            m.add_function(pyo3::wrap_pyfunction!($preset_names, m)?)?;
            m.add_function(pyo3::wrap_pyfunction!($ui_params, m)?)?;
            m.add_function(pyo3::wrap_pyfunction!($preset_params, m)?)?;
            $(m.add_function(pyo3::wrap_pyfunction!($extra, m)?)?;)*
            Ok(())
        }
    };
}

pub(crate) use declare_effect_pyfunctions;
