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
    patterns: Vec<Vec<String>>,
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
