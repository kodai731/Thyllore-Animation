use thyllore_anim_core::editable::PropertyType;
use thyllore_effect_core::{
    find_scalar_param, find_ui_param, title_case_snake, ScalarParam, UiParam,
};

use crate::ecs::component::{scalar_channel_domains, ScalarChannel, ScalarChannelDomain};
use crate::ecs::storage::Component;
use crate::ecs::world::{Entity, World};

/// Effect-core declarations a `ScalarChannelDomain` is derived from.
pub trait ScalarDomainSource: 'static {
    type Component: Component;

    const NAME: &'static str;

    fn scalars() -> &'static [ScalarParam<Self::Component>];
    fn ui() -> &'static [UiParam];
    fn local_time(component: &Self::Component) -> f32;
}

pub const fn effect_scalar_domain<S: ScalarDomainSource>(
    channel_table: fn() -> &'static [ScalarChannel],
) -> ScalarChannelDomain {
    ScalarChannelDomain {
        name: S::NAME,
        channel_table,
        has_component: has_effect_component::<S>,
        entities: effect_entities::<S>,
        read: read_effect_scalar::<S>,
        local_time: read_effect_local_time::<S>,
    }
}

/// Every scalar carrying `#[persist(curve)]`, in declaration order.
pub fn build_effect_scalar_channels<S: ScalarDomainSource>() -> Vec<ScalarChannel> {
    S::scalars()
        .iter()
        .filter(|scalar| scalar.curve)
        .map(build_effect_scalar_channel::<S>)
        .collect()
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
) -> ScalarChannel {
    let name = scalar.name;
    let display_name = match find_ui_param(S::ui(), name).and_then(|ui| ui.label) {
        Some(label) => label,
        None => title_case_snake(name).leak(),
    };
    let debug_value_range = scalar
        .debug_range
        .unwrap_or_else(|| ui_value_range::<S>(name));

    ScalarChannel {
        display_name,
        cli_name: scalar.name,
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

#[cfg(test)]
mod tests {
    use super::*;
    use thyllore_scene_core::SceneFields;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ProbeOwnerTag {
        Frame,
    }

    #[derive(Clone, Debug, Default, PartialEq, SceneFields)]
    #[scene(key = "test_probe_params", tag = ProbeOwnerTag, owner = Frame, tags = PROBE_TAGS, snapshot = probe_snapshot, scalars = PROBE_SCALARS, ui = PROBE_UI, overwrite = overwrite_probe)]
    pub struct ProbeParams {
        #[persist(curve, ui(min = 0.0, max = 1.0))]
        pub level: f32,
        #[persist(ui(min = 0.0, max = 1.0))]
        pub gain: f32,
        #[nested]
        pub wind: ProbeWind,
        #[runtime(ui(min = 0.0, max = 10.0))]
        pub time: f32,
    }

    #[derive(Clone, Debug, Default, PartialEq, SceneFields)]
    #[params(tag = ProbeOwnerTag, owner = Frame, group = "wind")]
    pub struct ProbeWind {
        #[persist(curve, scalars, debug_range = (-1.0, 1.0))]
        pub direction: [f32; 2],
        #[persist(curve, ui(label = "Bend", min = 0.0, max = 1.0), renamed_from = ["lean"])]
        pub bend: f32,
    }

    struct ProbeSource;

    impl ScalarDomainSource for ProbeSource {
        type Component = ProbeParams;

        const NAME: &'static str = "ProbeParams";

        fn scalars() -> &'static [ScalarParam<ProbeParams>] {
            &PROBE_SCALARS
        }

        fn ui() -> &'static [UiParam] {
            &PROBE_UI
        }

        fn local_time(component: &ProbeParams) -> f32 {
            component.time
        }
    }

    #[test]
    fn test_curve_fields_become_channels_in_declaration_order() {
        let channels = build_effect_scalar_channels::<ProbeSource>();
        let names: Vec<&str> = channels.iter().map(|c| c.cli_name).collect();
        assert_eq!(
            names,
            ["level", "wind_direction_x", "wind_direction_y", "wind_bend"]
        );

        assert_eq!(channels[0].display_name, "Level");
        assert_eq!(channels[0].debug_value_range, (0.0, 1.0));
        assert_eq!(channels[1].debug_value_range, (-1.0, 1.0));
        assert_eq!(channels[3].display_name, "Bend");
        assert_eq!(channels[3].renamed_from, &["lean"]);
    }
}
