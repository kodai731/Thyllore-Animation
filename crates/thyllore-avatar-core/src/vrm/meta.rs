use serde::{Deserialize, Serialize};

pub const VRM_1_0_LICENSE_URL: &str = "https://vrm.dev/licenses/1.0/";
pub const VRM_META_DEFAULT_VERSION: &str = "1.0";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AvatarPermission {
    #[default]
    OnlyAuthor,
    OnlySeparatelyLicensedPerson,
    Everyone,
}

impl AvatarPermission {
    pub const ALL: &'static [Self] = &[
        Self::OnlyAuthor,
        Self::OnlySeparatelyLicensedPerson,
        Self::Everyone,
    ];
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommercialUsage {
    #[default]
    PersonalNonProfit,
    PersonalProfit,
    Corporation,
}

impl CommercialUsage {
    pub const ALL: &'static [Self] = &[
        Self::PersonalNonProfit,
        Self::PersonalProfit,
        Self::Corporation,
    ];
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CreditNotation {
    #[default]
    Required,
    Unnecessary,
}

impl CreditNotation {
    pub const ALL: &'static [Self] = &[Self::Required, Self::Unnecessary];
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Modification {
    #[default]
    Prohibited,
    AllowModification,
    AllowModificationRedistribution,
}

impl Modification {
    pub const ALL: &'static [Self] = &[
        Self::Prohibited,
        Self::AllowModification,
        Self::AllowModificationRedistribution,
    ];
}

/// The `VRMC_vrm.meta` block of VRM 1.0; `name`, `authors` and the license URL are the required fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VrmMeta {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub authors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copyright_information: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_information: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub third_party_licenses: Option<String>,
    #[serde(default = "vrm_license_url")]
    pub license_url: String,
    #[serde(default)]
    pub avatar_permission: AvatarPermission,
    #[serde(default)]
    pub allow_excessively_violent_usage: bool,
    #[serde(default)]
    pub allow_excessively_sexual_usage: bool,
    #[serde(default)]
    pub commercial_usage: CommercialUsage,
    #[serde(default)]
    pub allow_political_or_religious_usage: bool,
    #[serde(default)]
    pub allow_antisocial_or_hate_usage: bool,
    #[serde(default)]
    pub credit_notation: CreditNotation,
    #[serde(default)]
    pub allow_redistribution: bool,
    #[serde(default)]
    pub modification: Modification,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other_license_url: Option<String>,
}

fn vrm_license_url() -> String {
    VRM_1_0_LICENSE_URL.to_string()
}

impl VrmMeta {
    /// The most restrictive VRM 1.0 meta for an avatar with the given name and author.
    pub fn new(name: impl Into<String>, author: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: None,
            authors: vec![author.into()],
            copyright_information: None,
            contact_information: None,
            references: Vec::new(),
            third_party_licenses: None,
            license_url: vrm_license_url(),
            avatar_permission: AvatarPermission::default(),
            allow_excessively_violent_usage: false,
            allow_excessively_sexual_usage: false,
            commercial_usage: CommercialUsage::default(),
            allow_political_or_religious_usage: false,
            allow_antisocial_or_hate_usage: false,
            credit_notation: CreditNotation::default(),
            allow_redistribution: false,
            modification: Modification::default(),
            other_license_url: None,
        }
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.name.trim().is_empty() {
            anyhow::bail!("VRM meta needs a non-empty name");
        }
        if self.authors.iter().all(|author| author.trim().is_empty()) {
            anyhow::bail!("VRM meta needs at least one author");
        }
        if self.license_url != VRM_1_0_LICENSE_URL {
            anyhow::bail!("VRM 1.0 meta licenseUrl must be {VRM_1_0_LICENSE_URL}");
        }
        Ok(())
    }
}

/// How a `VrmMeta` field is entered: free text, one of the choices (their VRM keys), or a flag.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VrmMetaFieldKind {
    Text,
    Choice { choices: Vec<String> },
    Flag,
}

/// One editable field of `VrmMeta` besides `name` and `authors`: `name` is the Rust field, `key`
/// the `VRMC_vrm.meta` key.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VrmMetaField {
    pub name: &'static str,
    pub key: String,
    pub label: &'static str,
    #[serde(flatten)]
    pub kind: VrmMetaFieldKind,
}

fn camel_case(snake: &str) -> String {
    let mut words = snake.split('_');
    let mut key = words.next().unwrap_or_default().to_string();
    for word in words {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            key.extend(first.to_uppercase());
            key.push_str(chars.as_str());
        }
    }
    key
}

fn choices<T: Serialize>(variants: &[T]) -> Vec<String> {
    variants
        .iter()
        .map(|variant| {
            serde_json::to_value(variant)
                .ok()
                .and_then(|value| value.as_str().map(str::to_string))
                .expect("a VRM meta choice serialises to its VRM key")
        })
        .collect()
}

/// The editable meta fields in display order: the texts, the choices, then the usage flags.
pub fn vrm_meta_fields() -> Vec<VrmMetaField> {
    let text = |name, label| VrmMetaField {
        name,
        key: camel_case(name),
        label,
        kind: VrmMetaFieldKind::Text,
    };
    let choice = |name, label, choices| VrmMetaField {
        name,
        key: camel_case(name),
        label,
        kind: VrmMetaFieldKind::Choice { choices },
    };
    let flag = |name, label| VrmMetaField {
        name,
        key: camel_case(name),
        label,
        kind: VrmMetaFieldKind::Flag,
    };
    vec![
        text("version", "Version"),
        text("copyright_information", "Copyright"),
        text("contact_information", "Contact"),
        text("other_license_url", "Other License"),
        choice(
            "avatar_permission",
            "Permission",
            choices(AvatarPermission::ALL),
        ),
        choice(
            "commercial_usage",
            "Commercial",
            choices(CommercialUsage::ALL),
        ),
        choice("credit_notation", "Credit", choices(CreditNotation::ALL)),
        choice("modification", "Modification", choices(Modification::ALL)),
        flag("allow_excessively_violent_usage", "Violent Usage"),
        flag("allow_excessively_sexual_usage", "Sexual Usage"),
        flag(
            "allow_political_or_religious_usage",
            "Political Or Religious Usage",
        ),
        flag("allow_antisocial_or_hate_usage", "Antisocial Or Hate Usage"),
        flag("allow_redistribution", "Redistribution"),
    ]
}

#[cfg(test)]
mod field_tests {
    use super::*;

    #[test]
    fn every_field_key_is_a_serialised_meta_key() {
        let mut meta = VrmMeta::new("avatar", "author");
        meta.version = Some(VRM_META_DEFAULT_VERSION.into());
        meta.copyright_information = Some("c".into());
        meta.contact_information = Some("c".into());
        meta.other_license_url = Some("u".into());
        let json = serde_json::to_value(&meta).unwrap();
        for field in vrm_meta_fields() {
            assert!(
                json.get(&field.key).is_some(),
                "{} is not a VrmMeta key",
                field.key
            );
        }
    }

    #[test]
    fn choices_are_the_vrm_keys_of_every_variant() {
        let fields = vrm_meta_fields();
        let permission = fields
            .iter()
            .find(|f| f.name == "avatar_permission")
            .unwrap();
        assert_eq!(
            permission.kind,
            VrmMetaFieldKind::Choice {
                choices: vec![
                    "onlyAuthor".into(),
                    "onlySeparatelyLicensedPerson".into(),
                    "everyone".into()
                ]
            }
        );
    }
}
