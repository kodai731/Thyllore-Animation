use std::marker::PhantomData;

use crate::{ScalarParam, SceneComponent, UiParam};

/// Typed access from a root component down to one of its (possibly nested) fields.
pub trait FieldPath<Root> {
    type Field: 'static;
    fn get(root: &Root) -> &Self::Field;
    fn get_mut(root: &mut Root) -> &mut Self::Field;
}

pub struct RootPath<R>(PhantomData<R>);

impl<R: 'static> FieldPath<R> for RootPath<R> {
    type Field = R;

    fn get(root: &R) -> &R {
        root
    }

    fn get_mut(root: &mut R) -> &mut R {
        root
    }
}

pub struct Then<A, B>(PhantomData<(A, B)>);

impl<R, A, B> FieldPath<R> for Then<A, B>
where
    A: FieldPath<R>,
    B: FieldPath<A::Field>,
{
    type Field = B::Field;

    fn get(root: &R) -> &B::Field {
        B::get(A::get(root))
    }

    fn get_mut(root: &mut R) -> &mut B::Field {
        B::get_mut(A::get_mut(root))
    }
}

/// A struct whose fields are scene parameters; nested structs compose through `FieldPath`.
pub trait SceneFields: Default + Clone + 'static {
    type Tag: Copy + 'static;

    fn overwrite_persisted(&mut self, loaded: &Self);

    fn collect_scalars<R: 'static, P: FieldPath<R, Field = Self>>(
        prefix: &str,
        out: &mut Vec<ScalarParam<R>>,
    );

    fn collect_ui(prefix: &str, path_prefix: &str, out: &mut Vec<UiParam>);

    fn collect_ownership(prefix: &str, out: &mut Vec<(&'static str, Self::Tag)>);

    /// Every persisted parameter as (public name, dotted serde path).
    fn collect_paths(prefix: &str, path_prefix: &str, out: &mut Vec<(&'static str, &'static str)>);

    fn collect_snapshot(&self, out: &mut Vec<Vec<f32>>);
}

/// Joins a table-name prefix and a field name; the result lives for the process because the
/// tables that hold it are built once.
pub fn intern_name(prefix: &str, name: &'static str) -> &'static str {
    if prefix.is_empty() {
        return name;
    }
    Box::leak(format!("{prefix}{name}").into_boxed_str())
}

pub fn nested_prefix(prefix: &str, field: &str, separator: &str) -> String {
    format!("{prefix}{field}{separator}")
}

/// The composed parameter tables of a scene component; `#[scene(...)]` on the root struct generates it.
pub trait SceneTables: SceneComponent + SceneFields {
    fn scalar_params() -> &'static [ScalarParam<Self>];
    fn ui_params() -> &'static [UiParam];
    fn ownership() -> &'static [(&'static str, <Self as SceneFields>::Tag)];
    fn parameter_paths() -> &'static [(&'static str, &'static str)];
    fn snapshot(&self) -> Vec<(&'static str, Vec<f32>)>;
}
