use serde::{Deserialize, Serialize};

pub const VRM_1_0_LICENSE_URL: &str = "https://vrm.dev/licenses/1.0/";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AvatarPermission {
    #[default]
    OnlyAuthor,
    OnlySeparatelyLicensedPerson,
    Everyone,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommercialUsage {
    #[default]
    PersonalNonProfit,
    PersonalProfit,
    Corporation,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CreditNotation {
    #[default]
    Required,
    Unnecessary,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Modification {
    #[default]
    Prohibited,
    AllowModification,
    AllowModificationRedistribution,
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
