use thyllore_anim_core::editable::PropertyType;
use thyllore_effect_core::{find_scalar_param, find_ui_param, ScalarParam, UiParam};

use crate::ecs::component::{
    scalar_channel_domains, ScalarChannel, ScalarChannelDomain, ScalarCodeBlock,
};
use crate::ecs::storage::Component;
use crate::ecs::world::{Entity, World};

/// Effect-core declarations a `ScalarChannelDomain` is derived from.
pub trait ScalarDomainSource: 'static {
    type Component: Component;

    const NAME: &'static str;
    const CODE_BLOCK: ScalarCodeBlock;

    fn scalars() -> &'static [ScalarParam<Self::Component>];
    fn ui() -> &'static [UiParam];
    fn local_time(component: &Self::Component) -> f32;
}

pub const fn effect_scalar_domain<S: ScalarDomainSource>(
    channel_table: fn() -> &'static [ScalarChannel],
) -> ScalarChannelDomain {
    ScalarChannelDomain {
        name: S::NAME,
        code_block: S::CODE_BLOCK,
        channel_table,
        has_component: has_effect_component::<S>,
        entities: effect_entities::<S>,
        read: read_effect_scalar::<S>,
        local_time: read_effect_local_time::<S>,
    }
}

/// Every scalar carrying `#[persist(code = N)]`, in code order.
pub fn build_effect_scalar_channels<S: ScalarDomainSource>() -> Vec<ScalarChannel> {
    let mut channels: Vec<ScalarChannel> = S::scalars()
        .iter()
        .filter_map(|scalar| {
            scalar
                .code
                .map(|code| build_effect_scalar_channel::<S>(scalar, code))
        })
        .collect();
    channels.sort_by_key(|channel| channel.code);
    channels
}

pub fn find_scalar_param_for_property<S: ScalarDomainSource>(
    property_type: PropertyType,
) -> Option<&'static ScalarParam<S::Component>> {
    let domain = scalar_channel_domains()
        .iter()
        .find(|domain| domain.name == S::NAME)?;
    let index = domain.channel_index(property_type)?;
    Some(find_source_scalar::<S>(domain.channels()[index].cli_name))
}

fn build_effect_scalar_channel<S: ScalarDomainSource>(
    scalar: &'static ScalarParam<S::Component>,
    code: u16,
) -> ScalarChannel {
    let name = scalar.name;
    let display_name = match find_ui_param(S::ui(), name).and_then(|ui| ui.label) {
        Some(label) => label,
        None => space_pascal_case(scalar.scene_name).leak(),
    };
    let debug_value_range = scalar
        .debug_range
        .unwrap_or_else(|| ui_value_range::<S>(name));

    ScalarChannel {
        code,
        display_name,
        cli_name: scalar.name,
        scene_name: scalar.scene_name,
        debug_value_range,
        renamed_from: scalar.renamed_from,
    }
}

fn find_source_scalar<S: ScalarDomainSource>(name: &str) -> &'static ScalarParam<S::Component> {
    find_scalar_param(S::scalars(), name)
        .unwrap_or_else(|| panic!("{} declares no scalar {name}", S::NAME))
}

fn ui_value_range<S: ScalarDomainSource>(scalar_name: &str) -> (f32, f32) {
    let ui = S::ui()
        .iter()
        .find(|ui| {
            ui.scalar_accessor_names()
                .iter()
                .any(|accessor| accessor == scalar_name)
        })
        .unwrap_or_else(|| {
            panic!(
                "{} scalar {scalar_name} has neither debug_range nor ui",
                S::NAME
            )
        });
    (ui.min, ui.max)
}

fn space_pascal_case(name: &str) -> String {
    let mut spaced = String::with_capacity(name.len() + 4);
    for (index, character) in name.char_indices() {
        if index > 0 && character.is_uppercase() {
            spaced.push(' ');
        }
        spaced.push(character);
    }
    spaced
}

fn has_effect_component<S: ScalarDomainSource>(world: &World, entity: Entity) -> bool {
    world.get_component::<S::Component>(entity).is_some()
}

fn effect_entities<S: ScalarDomainSource>(world: &World) -> Vec<Entity> {
    world.entities_with::<S::Component>()
}

fn read_effect_scalar<S: ScalarDomainSource>(
    world: &World,
    entity: Entity,
    property_type: PropertyType,
) -> Option<f32> {
    let scalar = find_scalar_param_for_property::<S>(property_type)?;
    world
        .get_component::<S::Component>(entity)
        .map(|component| (scalar.get)(component))
}

fn read_effect_local_time<S: ScalarDomainSource>(world: &World, entity: Entity) -> Option<f32> {
    world
        .get_component::<S::Component>(entity)
        .map(S::local_time)
}
