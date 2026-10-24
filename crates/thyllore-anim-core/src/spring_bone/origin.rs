use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChainRole {
    Hair,
    #[serde(alias = "skirt")]
    Garment,
}

impl ChainRole {
    pub fn bone_prefix(self) -> &'static str {
        match self {
            ChainRole::Hair => "hair",
            ChainRole::Garment => "garment",
        }
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct SubmeshKey {
    pub mesh: String,
    pub material: String,
}

/// One submesh (`mesh/material`) or every submesh of a mesh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubmeshSelector {
    Submesh(SubmeshKey),
    Mesh(String),
}

impl SubmeshSelector {
    pub fn matches(&self, key: &SubmeshKey) -> bool {
        match self {
            SubmeshSelector::Submesh(selected) => selected == key,
            SubmeshSelector::Mesh(mesh) => key.mesh == *mesh,
        }
    }

    pub fn mesh(&self) -> &str {
        match self {
            SubmeshSelector::Submesh(key) => &key.mesh,
            SubmeshSelector::Mesh(mesh) => mesh,
        }
    }

    pub fn material(&self) -> Option<&str> {
        match self {
            SubmeshSelector::Submesh(key) => Some(&key.material),
            SubmeshSelector::Mesh(_) => None,
        }
    }

    pub fn from_parts(mesh: &str, material: Option<&str>) -> Self {
        match material {
            Some(material) => SubmeshSelector::Submesh(SubmeshKey {
                mesh: mesh.to_string(),
                material: material.to_string(),
            }),
            None => SubmeshSelector::Mesh(mesh.to_string()),
        }
    }
}

impl FromStr for SubmeshSelector {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        if text.is_empty() {
            return Err("submesh selector expects <mesh> or <mesh>/<material>".to_string());
        }
        match text.rsplit_once('/') {
            Some((mesh, material)) => Ok(SubmeshSelector::from_parts(mesh, Some(material))),
            None => Ok(SubmeshSelector::Mesh(text.to_string())),
        }
    }
}

impl fmt::Display for SubmeshSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SubmeshSelector::Submesh(key) => write!(f, "{}/{}", key.mesh, key.material),
            SubmeshSelector::Mesh(mesh) => f.write_str(mesh),
        }
    }
}

/// Where a spring chain came from: hand-authored, or generated on a submesh by the chain generator.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ChainOrigin {
    #[default]
    Authored,
    Generated {
        target: SubmeshSelector,
        role: ChainRole,
    },
}

impl ChainOrigin {
    pub fn is_generated_on(&self, target: &SubmeshSelector) -> bool {
        match self {
            ChainOrigin::Authored => false,
            ChainOrigin::Generated { target: own, .. } => own == target,
        }
    }
}
