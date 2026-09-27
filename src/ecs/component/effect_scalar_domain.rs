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
                renamed_from: param.renamed_from,
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::ecs::component::{scalar_channel_for_scene_name, scalar_code_block_for_domain};

    #[derive(Clone, Default)]
    pub struct SourceProbe {
        gain: f32,
        steps: u32,
        seed: f32,
    }

    thyllore_scene_core::declare_scene_format! {
        component: SourceProbe,
        record: SourceProbeRecord,
        items {
            key: "source_probe",
            snapshot: source_probe_snapshot,
            scalars: SOURCE_PROBE_SCALAR_PARAMS,
            ui: SOURCE_PROBE_UI_PARAMS,
            overwrite: source_probe_overwrite,
        },
        persisted {
            source_gain: f32 {
                get: |c: &SourceProbe| c.gain,
                set: |c: &mut SourceProbe, v: f32| { c.gain = v; }
                , code: 1280
                , renamed_from: ["OldName"]
                , ui {
                    label: "Gain Amount",
                    min: 0.0,
                    max: 2.0
                }
            },
            source_steps: u32 {
                get: |c: &SourceProbe| c.steps,
                set: |c: &mut SourceProbe, v: u32| { c.steps = v; }
                , code: 1281
                , debug_range: (1.0, 8.0)
            },
            source_seed: f32 {
                get: |c: &SourceProbe| c.seed,
                set: |c: &mut SourceProbe, v: f32| { c.seed = v; }
            },
        },
        runtime {},
    }

    struct TestScalars;

    impl ScalarDomainSource for TestScalars {
        type Component = SourceProbe;
        const NAME: &'static str = "TestScalars";
        const SCALARS: &'static [ScalarParam<SourceProbe>] = SOURCE_PROBE_SCALAR_PARAMS;
        const UI: &'static [UiParam] = SOURCE_PROBE_UI_PARAMS;

        fn local_time(_: &SourceProbe) -> f32 {
            0.0
        }
    }

    static TEST_DOMAIN: ScalarChannelDomain = effect_scalar_domain::<TestScalars>();

    crate::scalar_channel_domain!(TEST_DOMAIN);

    fn find_channel(domain: &ScalarChannelDomain, code: u16) -> &'static ScalarChannel {
        (domain.channels)()
            .iter()
            .find(|channel| channel.code == code)
            .unwrap_or_else(|| panic!("{} has no channel {code}", domain.name))
    }

    fn assert_channel(
        domain: &ScalarChannelDomain,
        code: u16,
        names: (&str, &str, &str),
        debug_value_range: (f32, f32),
    ) {
        let channel = find_channel(domain, code);
        assert_eq!(
            (channel.cli_name, channel.scene_name, channel.display_name),
            names
        );
        assert_eq!(channel.debug_value_range, debug_value_range);
    }

    fn assert_codes_unique_inside_block(domain: &ScalarChannelDomain) {
        let block = scalar_code_block_for_domain(domain.name)
            .unwrap_or_else(|| panic!("no code block for {}", domain.name));
        let mut codes = HashSet::new();
        for channel in (domain.channels)() {
            assert!(
                block.contains(channel.code),
                "{} outside block",
                channel.code
            );
            assert!(
                codes.insert(channel.code),
                "duplicate code {}",
                channel.code
            );
        }
    }

    #[test]
    fn test_effect_domains_expose_every_declared_channel() {
        use crate::ecs::component::{FLAME_DOMAIN, LIGHTNING_DOMAIN, WATER_DOMAIN, WIND_DOMAIN};

        for (domain, channel_count) in [
            (&FLAME_DOMAIN, 16),
            (&WATER_DOMAIN, 22),
            (&WIND_DOMAIN, 40),
            (&LIGHTNING_DOMAIN, 49),
        ] {
            assert_eq!((domain.channels)().len(), channel_count, "{}", domain.name);
            assert_codes_unique_inside_block(domain);
        }
    }

    #[test]
    fn test_effect_channels_keep_their_persisted_names() {
        use crate::ecs::component::{FLAME_DOMAIN, LIGHTNING_DOMAIN, WATER_DOMAIN, WIND_DOMAIN};

        assert_channel(
            &LIGHTNING_DOMAIN,
            768,
            ("end_offset_x", "EndOffsetX", "End Offset X"),
            (-20.0, 20.0),
        );
        let strikes_per_burst = find_channel(&LIGHTNING_DOMAIN, 771);
        assert_eq!(strikes_per_burst.cli_name, "strikes_per_burst");
        assert_eq!(strikes_per_burst.debug_value_range, (1.0, 8.0));
        assert_channel(
            &WIND_DOMAIN,
            512,
            ("column_height", "ColumnHeight", "Column Height"),
            (0.5, 10.0),
        );
        assert_channel(&WATER_DOMAIN, 258, ("ior", "Ior", "IOR"), (1.0, 2.5));
        assert_channel(
            &FLAME_DOMAIN,
            4,
            ("temperature_base_k", "TemperatureBaseK", "Temp Base K"),
            (800.0, 3000.0),
        );
        assert_channel(
            &FLAME_DOMAIN,
            12,
            ("wind_x", "WindX", "Wind X"),
            (-1.0, 1.0),
        );
    }

    #[test]
    fn test_applied_scalar_reads_back_through_the_domain() {
        use crate::ecs::component::{LightningEffect, LightningScalars, LIGHTNING_DOMAIN};

        let mut effect = LightningEffect::default();
        assert!(apply_scalar::<LightningScalars>(&mut effect, 768, 3.5));

        let mut world = World::new();
        let entity = world.spawn();
        world.insert_component(entity, effect);

        let value = (LIGHTNING_DOMAIN.read)(&world, entity, PropertyType::Custom(768));
        assert_eq!(value, Some(3.5));
    }

    #[test]
    fn test_channels_derive_from_the_scene_format_declaration() {
        assert_eq!((TEST_DOMAIN.channels)().len(), 2);
        assert_channel(
            &TEST_DOMAIN,
            1280,
            ("source_gain", "SourceGain", "Gain Amount"),
            (0.0, 2.0),
        );
        assert_channel(
            &TEST_DOMAIN,
            1281,
            ("source_steps", "SourceSteps", "Source Steps"),
            (1.0, 8.0),
        );
        assert_eq!(find_channel(&TEST_DOMAIN, 1280).renamed_from, ["OldName"]);
        assert!(find_channel(&TEST_DOMAIN, 1281).renamed_from.is_empty());
    }

    #[test]
    fn test_renamed_channel_resolves_by_its_former_scene_name() {
        let (domain, channel) = scalar_channel_for_scene_name("OldName").unwrap();
        assert_eq!(domain.name, "TestScalars");
        assert_eq!(channel.code, 1280);
        assert_eq!(channel.scene_name, "SourceGain");
    }
}
