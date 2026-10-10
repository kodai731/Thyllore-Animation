use std::collections::BTreeSet;
use std::env;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

const RIG_FILE: &str = "data/humanoid_rig.toml";
const GENERATED_FILE: &str = "humanoid_rig.rs";
const SIDES: [&str; 2] = ["Left", "Right"];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RigDefinition {
    role: Vec<PartDefinition>,
    chain: Vec<ChainDefinition>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PartDefinition {
    part: String,
    #[serde(default)]
    paired: bool,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    translation: bool,
    patterns: Vec<Vec<String>>,
    #[serde(default)]
    vrm1_part: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChainDefinition {
    parts: Vec<String>,
}

struct Role<'a> {
    name: String,
    side: Option<&'static str>,
    part: &'a PartDefinition,
}

fn main() {
    println!("cargo:rerun-if-changed={RIG_FILE}");
    println!("cargo:rerun-if-changed=build.rs");

    let rig_text =
        fs::read_to_string(RIG_FILE).unwrap_or_else(|error| panic!("{RIG_FILE}: {error}"));
    let rig: RigDefinition =
        toml::from_str(&rig_text).unwrap_or_else(|error| panic!("{RIG_FILE}: {error}"));
    validate_rig(&rig);

    let out_path =
        PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join(GENERATED_FILE);
    fs::write(&out_path, generate_rig_rust(&rig))
        .unwrap_or_else(|error| panic!("{}: {error}", out_path.display()));
}

fn validate_rig(rig: &RigDefinition) {
    let mut known_parts = BTreeSet::new();
    for part in &rig.role {
        assert!(
            is_variant_name(&part.part),
            "{RIG_FILE}: part `{}` must be an UpperCamelCase identifier",
            part.part
        );
        assert!(
            known_parts.insert(part.part.as_str()),
            "{RIG_FILE}: part `{}` is declared twice",
            part.part
        );
        assert!(
            !part.patterns.is_empty() && part.patterns.iter().all(|pattern| !pattern.is_empty()),
            "{RIG_FILE}: part `{}` needs at least one non-empty pattern",
            part.part
        );
    }

    for chain in &rig.chain {
        let chain_parts: Vec<&PartDefinition> = chain
            .parts
            .iter()
            .map(|name| find_part(rig, name))
            .collect();
        assert!(
            chain_parts
                .windows(2)
                .all(|pair| pair[0].paired == pair[1].paired),
            "{RIG_FILE}: chain {:?} mixes paired and centre parts",
            chain.parts
        );
    }
}

fn is_variant_name(name: &str) -> bool {
    name.starts_with(|first: char| first.is_ascii_uppercase())
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
}

fn find_part<'a>(rig: &'a RigDefinition, name: &str) -> &'a PartDefinition {
    rig.role
        .iter()
        .find(|part| part.part == name)
        .unwrap_or_else(|| panic!("{RIG_FILE}: chain names the undeclared part `{name}`"))
}

fn expand_roles(part: &PartDefinition) -> Vec<Role<'_>> {
    if !part.paired {
        return vec![Role {
            name: part.part.clone(),
            side: None,
            part,
        }];
    }

    SIDES
        .iter()
        .map(|side| Role {
            name: format!("{side}{}", part.part),
            side: Some(side),
            part,
        })
        .collect()
}

fn expand_chain(rig: &RigDefinition, chain: &ChainDefinition) -> Vec<Vec<String>> {
    let is_paired = chain
        .parts
        .first()
        .is_some_and(|name| find_part(rig, name).paired);
    if !is_paired {
        return vec![chain.parts.clone()];
    }

    SIDES
        .iter()
        .map(|side| {
            chain
                .parts
                .iter()
                .map(|part| format!("{side}{part}"))
                .collect()
        })
        .collect()
}

fn generate_rig_rust(rig: &RigDefinition) -> String {
    let roles: Vec<Role> = rig.role.iter().flat_map(expand_roles).collect();
    let all_names: Vec<&str> = roles.iter().map(|role| role.name.as_str()).collect();
    let required_names: Vec<&str> = roles
        .iter()
        .filter(|role| role.part.required)
        .map(|role| role.name.as_str())
        .collect();

    let mut out = String::new();
    write_role_enum(&mut out, &all_names);

    out.push_str("impl HumanoidRole {\n");
    let _ = writeln!(
        out,
        "    pub const ALL: [HumanoidRole; {}] = {};\n",
        all_names.len(),
        role_array(&all_names)
    );
    write_role_method(
        &mut out,
        "unity_name(self) -> &'static str",
        &roles,
        |role| format!("{:?}", role.name),
    );
    write_role_method(
        &mut out,
        "side(self) -> Option<Side>",
        &roles,
        |role| match role.side {
            Some(side) => format!("Some(Side::{side})"),
            None => "None".to_string(),
        },
    );
    write_role_method(
        &mut out,
        "name_patterns(self) -> &'static [&'static [&'static str]]",
        &roles,
        pattern_table,
    );
    write_role_method(
        &mut out,
        "allows_translation(self) -> bool",
        &roles,
        |role| role.part.translation.to_string(),
    );
    write_role_method(&mut out, "index(self) -> usize", &roles, |role| {
        let idx = all_names.iter().position(|n| *n == role.name).unwrap();
        idx.to_string()
    });
    write_from_unity_name(&mut out, &roles);
    write_vrm_name(&mut out, &roles);
    write_from_vrm_name(&mut out, &roles);
    write_role_method(&mut out, "mirrored(self) -> HumanoidRole", &roles, |role| {
        if let Some(side) = role.side {
            let opposite_side = SIDES.iter().find(|s| **s != side).unwrap();
            format!("Self::{}{}", opposite_side, role.part.part)
        } else {
            format!("Self::{}", role.name)
        }
    });
    out.push_str("}\n\n");

    let _ = writeln!(
        out,
        "pub const REQUIRED: [HumanoidRole; {}] = {};\n",
        required_names.len(),
        role_array(&required_names)
    );
    write_chains(&mut out, rig);
    out
}

fn write_role_enum(out: &mut String, names: &[&str]) {
    out.push_str(
        "#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]\n\
         pub enum HumanoidRole {\n",
    );
    for name in names {
        let _ = writeln!(out, "    {name},");
    }
    out.push_str("}\n\n");
}

fn write_role_method(
    out: &mut String,
    signature: &str,
    roles: &[Role],
    value_of: impl Fn(&Role) -> String,
) {
    let _ = writeln!(out, "    pub fn {signature} {{\n        match self {{");
    for role in roles {
        let _ = writeln!(
            out,
            "            Self::{} => {},",
            role.name,
            value_of(role)
        );
    }
    out.push_str("        }\n    }\n\n");
}

fn write_chains(out: &mut String, rig: &RigDefinition) {
    let chains: Vec<Vec<String>> = rig
        .chain
        .iter()
        .flat_map(|chain| expand_chain(rig, chain))
        .collect();

    let _ = writeln!(
        out,
        "pub const HUMANOID_CHAINS: [&[HumanoidRole]; {}] = [",
        chains.len()
    );
    for chain in &chains {
        let names: Vec<&str> = chain.iter().map(String::as_str).collect();
        let _ = writeln!(out, "    &{},", role_array(&names));
    }
    out.push_str("];\n");
}

fn role_array(names: &[&str]) -> String {
    let roles: Vec<String> = names
        .iter()
        .map(|name| format!("HumanoidRole::{name}"))
        .collect();
    format!("[{}]", roles.join(", "))
}

fn pattern_table(role: &Role) -> String {
    let patterns: Vec<String> = role
        .part
        .patterns
        .iter()
        .map(|pattern| format!("&{pattern:?}"))
        .collect();
    format!("&[{}]", patterns.join(", "))
}

fn write_from_unity_name(out: &mut String, roles: &[Role]) {
    let _ = writeln!(
        out,
        "    pub fn from_unity_name(name: &str) -> Option<HumanoidRole> {{\n        match name {{"
    );
    for role in roles {
        let _ = writeln!(
            out,
            "            {:?} => Some(Self::{name}),",
            role.name,
            name = role.name
        );
    }
    out.push_str("            _ => None,\n        }\n    }\n\n");
}

fn write_vrm_name(out: &mut String, roles: &[Role]) {
    let _ = writeln!(
        out,
        "    pub fn vrm_name(self, version: VrmVersion) -> &'static str {{\n        match (self, version) {{"
    );
    for role in roles {
        let v0_name = vrm_bone_name_for_version(role, false);
        let v1_name = vrm_bone_name_for_version(role, true);
        if v0_name == v1_name {
            let _ = writeln!(
                out,
                "            (Self::{name}, _) => {:?},",
                v0_name,
                name = role.name
            );
        } else {
            let _ = writeln!(
                out,
                "            (Self::{name}, VrmVersion::V0) => {:?},",
                v0_name,
                name = role.name
            );
            let _ = writeln!(
                out,
                "            (Self::{name}, VrmVersion::V1) => {:?},",
                v1_name,
                name = role.name
            );
        }
    }
    out.push_str("        }\n    }\n\n");
}

fn write_from_vrm_name(out: &mut String, roles: &[Role]) {
    let _ = writeln!(
        out,
        "    pub fn from_vrm_name(name: &str, version: VrmVersion) -> Option<HumanoidRole> {{\n        match (name, version) {{"
    );
    for role in roles {
        let v0_name = vrm_bone_name_for_version(role, false);
        let v1_name = vrm_bone_name_for_version(role, true);
        if v0_name == v1_name {
            let _ = writeln!(
                out,
                "            ({:?}, _) => Some(Self::{name}),",
                v0_name,
                name = role.name
            );
        } else {
            let _ = writeln!(
                out,
                "            ({:?}, VrmVersion::V0) => Some(Self::{name}),",
                v0_name,
                name = role.name
            );
            let _ = writeln!(
                out,
                "            ({:?}, VrmVersion::V1) => Some(Self::{name}),",
                v1_name,
                name = role.name
            );
        }
    }
    out.push_str("            _ => None,\n        }\n    }\n\n");
}

fn vrm_bone_name_for_version(role: &Role, is_v1: bool) -> String {
    let part = if is_v1 {
        role.part.vrm1_part.as_deref().unwrap_or(&role.part.part)
    } else {
        &role.part.part
    };
    match role.side {
        Some(side) => format!("{}{}", side.to_ascii_lowercase(), part),
        None => {
            let first_lower = part
                .chars()
                .enumerate()
                .map(|(i, c)| if i == 0 { c.to_ascii_lowercase() } else { c })
                .collect::<String>();
            first_lower
        }
    }
}
