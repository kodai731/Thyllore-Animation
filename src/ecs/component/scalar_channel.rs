use std::sync::OnceLock;

use thyllore_anim_core::editable::PropertyType;

use crate::ecs::world::{Entity, World};

/// Width of the `PropertyType::Custom` code range each registered domain owns.
pub const CODES_PER_DOMAIN: u16 = 256;

/// One animatable scalar channel exposed by a component domain. Its `PropertyType::Custom`
/// code is process-local: the domain derives it from the channel's position in its table.
#[derive(Clone, Copy, Debug)]
pub struct ScalarChannel {
    pub display_name: &'static str,
    /// Stable snake_case identifier used by clip files, batch CLI flags and anim dumps.
    /// Unique across every registered domain.
    pub cli_name: &'static str,
    /// Conservative value range for generated debug keys.
    pub debug_value_range: (f32, f32),
    /// Former `cli_name`s still accepted when a clip file or CLI flag names the channel.
    pub renamed_from: &'static [&'static str],
}

impl ScalarChannel {
    pub fn answers_to(&self, name: &str) -> bool {
        self.cli_name == name || self.renamed_from.contains(&name)
    }
}

/// A component domain whose scalar fields animate through clip scalar curves.
/// Registering a domain with `scalar_channel_domain!` is all the curve editor,
/// timeline, batch CLI and clip files need to animate its channels.
/// Applying sampled curve values back to the component stays inside the
/// domain's own system.
pub struct ScalarChannelDomain {
    /// Display name of the domain (also the name of the clip it creates).
    pub name: &'static str,
    pub channel_table: fn() -> &'static [ScalarChannel],
    pub has_component: fn(&World, Entity) -> bool,
    pub entities: fn(&World) -> Vec<Entity>,
    /// Current component value of a channel (None when the entity lost the
    /// component or the property belongs to another domain).
    pub read: fn(&World, Entity, PropertyType) -> Option<f32>,
    /// Domain-local playback time used to sample curves.
    pub local_time: fn(&World, Entity) -> Option<f32>,
}

impl ScalarChannelDomain {
    pub fn channels(&self) -> &'static [ScalarChannel] {
        (self.channel_table)()
    }

    pub fn property_type_at(&self, channel_index: usize) -> PropertyType {
        PropertyType::Custom(self.first_code() + channel_index as u16)
    }

    pub fn property_type_of(&self, channel: &ScalarChannel) -> Option<PropertyType> {
        self.property_type_for_cli_name(channel.cli_name)
    }

    pub fn property_type_for_cli_name(&self, name: &str) -> Option<PropertyType> {
        self.channels()
            .iter()
            .position(|c| c.answers_to(name))
            .map(|index| self.property_type_at(index))
    }

    pub fn channel_index(&self, property_type: PropertyType) -> Option<usize> {
        let PropertyType::Custom(code) = property_type else {
            return None;
        };
        let index = code.checked_sub(self.first_code())? as usize;
        (index < self.channels().len()).then_some(index)
    }

    fn first_code(&self) -> u16 {
        let position = scalar_channel_domains()
            .iter()
            .position(|domain| domain.name == self.name)
            .unwrap_or_else(|| panic!("scalar domain {} is not registered", self.name));
        position as u16 * CODES_PER_DOMAIN
    }
}

/// Link-time registration of a domain: `scalar_channel_domain!(MY_DOMAIN)` next to the static.
pub struct ScalarChannelDomainRegistration(pub &'static ScalarChannelDomain);

inventory::collect!(ScalarChannelDomainRegistration);

#[macro_export]
macro_rules! scalar_channel_domain {
    ($domain:expr) => {
        inventory::submit! { $crate::ecs::component::ScalarChannelDomainRegistration(&$domain) }
    };
}

/// Every registered domain in name order; the position in this list fixes the domain's code
/// range, and the shared code never lists the domains itself.
pub fn scalar_channel_domains() -> &'static [&'static ScalarChannelDomain] {
    static DOMAINS: OnceLock<Vec<&'static ScalarChannelDomain>> = OnceLock::new();
    DOMAINS.get_or_init(|| {
        let mut domains: Vec<_> = inventory::iter::<ScalarChannelDomainRegistration>
            .into_iter()
            .map(|registration| registration.0)
            .collect();
        domains.sort_by_key(|domain| domain.name);
        domains
    })
}

pub fn scalar_domain_for_entity(
    world: &World,
    entity: Entity,
) -> Option<&'static ScalarChannelDomain> {
    scalar_channel_domains()
        .iter()
        .copied()
        .find(|domain| (domain.has_component)(world, entity))
}

pub fn scalar_channel_for_property(
    property_type: PropertyType,
) -> Option<(&'static ScalarChannelDomain, &'static ScalarChannel)> {
    scalar_channel_domains().iter().find_map(|domain| {
        domain
            .channel_index(property_type)
            .map(|index| (*domain, &domain.channels()[index]))
    })
}

pub fn scalar_channel_for_cli_name(
    name: &str,
) -> Option<(&'static ScalarChannelDomain, &'static ScalarChannel)> {
    scalar_channel_domains().iter().find_map(|domain| {
        domain
            .channels()
            .iter()
            .find(|c| c.answers_to(name))
            .map(|c| (*domain, c))
    })
}

pub fn scalar_cli_names_joined() -> String {
    scalar_channel_domains()
        .iter()
        .flat_map(|domain| domain.channels().iter().map(|c| c.cli_name))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_domains_and_names_are_unique() {
        let mut domain_names = HashSet::new();
        let mut cli_names = HashSet::new();
        for domain in scalar_channel_domains() {
            assert!(
                domain_names.insert(domain.name),
                "domain {} registered twice",
                domain.name
            );
            assert!(
                !domain.channels().is_empty(),
                "empty domain {}",
                domain.name
            );
            for channel in domain.channels() {
                for name in std::iter::once(&channel.cli_name).chain(channel.renamed_from) {
                    assert!(cli_names.insert(*name), "duplicate cli name {name}");
                }
            }
        }
        assert!(!cli_names.is_empty());
    }

    #[test]
    fn test_every_domain_fits_its_code_range() {
        for domain in scalar_channel_domains() {
            assert!(
                domain.channels().len() <= CODES_PER_DOMAIN as usize,
                "{} declares more than {CODES_PER_DOMAIN} channels",
                domain.name
            );
        }
    }

    #[test]
    fn test_lookups_roundtrip_every_channel() {
        for domain in scalar_channel_domains() {
            for (index, channel) in domain.channels().iter().enumerate() {
                let property_type = domain.property_type_at(index);
                assert_eq!(domain.property_type_of(channel), Some(property_type));
                assert_eq!(domain.channel_index(property_type), Some(index));
                let (d, c) = scalar_channel_for_property(property_type).unwrap();
                assert_eq!(d.name, domain.name);
                assert_eq!(c.cli_name, channel.cli_name);
                let (d, c) = scalar_channel_for_cli_name(channel.cli_name).unwrap();
                assert_eq!(d.property_type_of(c), Some(property_type));
            }
        }
        assert!(scalar_channel_for_cli_name("no_such_channel").is_none());
        assert!(scalar_channel_for_property(PropertyType::TranslationX).is_none());
    }
}
