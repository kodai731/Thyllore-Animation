use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use thyllore_anim_core::editable::PropertyType;
use thyllore_scene_core::{find_ui_param, title_case_snake, ScalarParam, UiParam};

use crate::ecs::world::{Entity, World};

use super::{ScalarChannel, ScalarChannelDomain};

pub trait ScalarDomainSource: 'static {
    type Component: 'static;
    const NAME: &'static str;
    const SCALARS: &'static [ScalarParam<Self::Component>];
    const UI: &'static [UiParam];
    fn local_time(component: &Self::Component) -> f32;
}

pub const fn effect_scalar_domain<S: ScalarDomainSource>() -> ScalarChannelDomain {
    ScalarChannelDomain {
        name: S::NAME,
        channels: channels::<S>,
        has_component: has_component::<S>,
        entities: entities::<S>,
        read: read::<S>,
        local_time: local_time::<S>,
    }
}

pub fn apply_scalar<S: ScalarDomainSource>(
    component: &mut S::Component,
    code: u16,
    value: f32,
) -> bool {
    match find_scalar_by_code::<S>(code) {
        Some(param) => {
            (param.set)(component, value);
            true
        }
        None => false,
    }
}

fn find_scalar_by_code<S: ScalarDomainSource>(
    code: u16,
) -> Option<&'static ScalarParam<S::Component>> {
    S::SCALARS.iter().find(|param| param.code == Some(code))
}

fn channels<S: ScalarDomainSource>() -> &'static [ScalarChannel] {
    static CHANNELS_BY_SOURCE: OnceLock<Mutex<HashMap<TypeId, &'static [ScalarChannel]>>> =
        OnceLock::new();

    let mut channels_by_source = CHANNELS_BY_SOURCE
        .get_or_init(Default::default)
        .lock()
        .expect("scalar channel table lock is never poisoned");
    channels_by_source
        .entry(TypeId::of::<S>())
        .or_insert_with(|| Box::leak(build_channels(S::SCALARS, S::UI).into_boxed_slice()))
}

fn build_channels<C>(scalars: &[ScalarParam<C>], ui: &[UiParam]) -> Vec<ScalarChannel> {
    scalars
        .iter()
        .filter_map(|param| {
            let code = param.code?;
            Some(ScalarChannel {
                code,
                display_name: leak_str(display_name(ui, param.name)),
                cli_name: param.name,
                scene_name: leak_str(title_case_snake(param.name).replace(' ', "")),
                debug_value_range: debug_value_range(param, ui),
            })
        })
        .collect()
}

fn display_name(ui: &[UiParam], name: &str) -> String {
    match find_ui_param(ui, name) {
        Some(ui_param) => ui_param.display_label().into_owned(),
        None => title_case_snake(name),
    }
}

fn debug_value_range<C>(param: &ScalarParam<C>, ui: &[UiParam]) -> (f32, f32) {
    if let Some(debug_range) = param.debug_range {
        return debug_range;
    }

    let name = param.name;
    let ui_param = find_ui_param(ui, name)
        .or_else(|| {
            let (parent_name, _) = name.rsplit_once('_')?;
            find_ui_param(ui, parent_name)
        })
        .unwrap_or_else(|| panic!("scalar {name} has no debug range and no ui entry"));
    (ui_param.min, ui_param.max)
}

fn leak_str(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

fn has_component<S: ScalarDomainSource>(world: &World, entity: Entity) -> bool {
    world.get_component::<S::Component>(entity).is_some()
}

fn entities<S: ScalarDomainSource>(world: &World) -> Vec<Entity> {
    world.entities_with::<S::Component>()
}

fn read<S: ScalarDomainSource>(
    world: &World,
    entity: Entity,
    property_type: PropertyType,
) -> Option<f32> {
    let PropertyType::Custom(code) = property_type else {
        return None;
    };
    let param = find_scalar_by_code::<S>(code)?;
    world
        .get_component::<S::Component>(entity)
        .map(|component| (param.get)(component))
}

fn local_time<S: ScalarDomainSource>(world: &World, entity: Entity) -> Option<f32> {
    world
        .get_component::<S::Component>(entity)
        .map(S::local_time)
}
