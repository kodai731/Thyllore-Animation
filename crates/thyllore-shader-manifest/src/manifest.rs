use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use thiserror::Error;
use toml::Value;

use crate::naming::{is_shader_source, parse_entry_points, EntryPoint};
use crate::stage::StageKind;

/// Entry files under `shaders/` keyed by their path relative to it, each with its entry points.
pub type ShaderEntries = BTreeMap<String, Vec<EntryPoint>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SetRole {
    Frame,
    Material,
    Object,
    Local,
}

impl SetRole {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "frame" => Some(Self::Frame),
            "material" => Some(Self::Material),
            "object" => Some(Self::Object),
            "local" => Some(Self::Local),
            _ => None,
        }
    }

    pub fn fixed_set_index(self) -> Option<u32> {
        match self {
            Self::Frame => Some(0),
            Self::Material => Some(1),
            Self::Object => Some(2),
            Self::Local => None,
        }
    }

    pub fn variant_name(self) -> &'static str {
        match self {
            Self::Frame => "Frame",
            Self::Material => "Material",
            Self::Object => "Object",
            Self::Local => "Local",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageSource {
    pub source_file: String,
    pub entry: String,
    pub stage: StageKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassDefinition {
    pub name: String,
    pub stages: Vec<StageSource>,
    pub sets: Vec<(u32, SetRole)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PassManifest {
    pub passes: Vec<PassDefinition>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("passes.toml is not valid TOML: {0}")]
    Toml(String),
    #[error("passes.toml has no [pass.<name>] table")]
    NoPasses,
    #[error("passes.toml: `{0}` has an unexpected shape (expected `[pass.<name>]` with `stages` and `sets`)")]
    Shape(String),
    #[error("pass `{0}` is not a valid pass name (use [a-z][a-z0-9_]*)")]
    InvalidPassName(String),
    #[error("pass `{pass}`: `{file}` declares no `[shader(\"..\")]` entry point")]
    NoEntryPoint { pass: String, file: String },
    #[error("pass `{pass}`: {reason}")]
    StageComposition { pass: String, reason: String },
    #[error("pass `{pass}`: shader source `{file}` does not exist in shaders/")]
    MissingSource { pass: String, file: String },
    #[error("shader `{0}` exists in shaders/ but no pass in passes.toml references it")]
    OrphanShader(String),
    #[error("pass `{pass}`: unknown set role `{role}` (frame / material / object / local)")]
    UnknownSetRole { pass: String, role: String },
    #[error("pass `{pass}`: set key `{key}` is not a set index")]
    InvalidSetIndex { pass: String, key: String },
    #[error("pass `{pass}`: set {set} is declared twice")]
    DuplicateSetIndex { pass: String, set: u32 },
    #[error("pass `{pass}`: role {role:?} must be bound at set {expected}, not {set}")]
    RoleAtWrongSet {
        pass: String,
        role: SetRole,
        set: u32,
        expected: u32,
    },
    #[error("pass `{pass}`: role {role:?} is declared for more than one set")]
    DuplicateRole { pass: String, role: SetRole },
}

impl PassManifest {
    /// Parses `passes.toml`; every stage file must be one of `entries` and every entry file must be
    /// referenced by a pass.
    pub fn parse(toml_text: &str, entries: &ShaderEntries) -> Result<Self, ManifestError> {
        let root: Value = toml_text
            .parse()
            .map_err(|error: toml::de::Error| ManifestError::Toml(error.to_string()))?;
        let pass_table = root
            .get("pass")
            .and_then(Value::as_table)
            .ok_or(ManifestError::NoPasses)?;

        let mut passes = Vec::with_capacity(pass_table.len());
        for (name, definition) in pass_table {
            passes.push(parse_pass(name, definition, entries)?);
        }
        if passes.is_empty() {
            return Err(ManifestError::NoPasses);
        }

        let manifest = Self { passes };
        manifest.reject_orphans(entries)?;
        Ok(manifest)
    }

    fn reject_orphans(&self, entries: &ShaderEntries) -> Result<(), ManifestError> {
        let referenced: BTreeSet<&str> = self
            .passes
            .iter()
            .flat_map(|pass| pass.stages.iter().map(|stage| stage.source_file.as_str()))
            .collect();
        match entries
            .keys()
            .find(|file_name| !referenced.contains(file_name.as_str()))
        {
            Some(orphan) => Err(ManifestError::OrphanShader(orphan.clone())),
            None => Ok(()),
        }
    }
}

/// Every `.slang` file under `shader_dir` that declares an entry point, keyed by its path relative
/// to it (`wind/resolveFragment.slang`); modules without an entry point compile to no SPIR-V.
pub fn collect_shader_sources(
    shader_dir: &Path,
) -> Result<BTreeMap<String, ShaderSource>, ManifestError> {
    let mut sources = BTreeMap::new();
    let mut pending = vec![shader_dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let entries = std::fs::read_dir(&dir)
            .map_err(|error| ManifestError::Toml(format!("read {}: {error}", dir.display())))?;
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !is_shader_source(file_name) {
                continue;
            }
            let Some(source_key) = relative_source_key(shader_dir, &path) else {
                continue;
            };
            let text = std::fs::read_to_string(&path).map_err(|error| {
                ManifestError::Toml(format!("read {}: {error}", path.display()))
            })?;
            let entry_points = parse_entry_points(&text);
            if entry_points.is_empty() {
                continue;
            }
            sources.insert(source_key, ShaderSource { path, entry_points });
        }
    }
    Ok(sources)
}

/// An entry file on disk and the entry points it declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShaderSource {
    pub path: PathBuf,
    pub entry_points: Vec<EntryPoint>,
}

pub fn shader_entries(sources: &BTreeMap<String, ShaderSource>) -> ShaderEntries {
    sources
        .iter()
        .map(|(file, source)| (file.clone(), source.entry_points.clone()))
        .collect()
}

fn relative_source_key(shader_dir: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(shader_dir).ok()?;
    let mut key = String::new();
    for component in relative.components() {
        if !key.is_empty() {
            key.push('/');
        }
        key.push_str(component.as_os_str().to_str()?);
    }
    Some(key)
}

fn parse_pass(
    name: &str,
    definition: &Value,
    entries: &ShaderEntries,
) -> Result<PassDefinition, ManifestError> {
    validate_pass_name(name)?;
    let table = definition
        .as_table()
        .ok_or_else(|| ManifestError::Shape(format!("pass.{name}")))?;

    let stage_files = table
        .get("stages")
        .and_then(Value::as_array)
        .ok_or_else(|| ManifestError::Shape(format!("pass.{name}.stages")))?;
    let mut stages = Vec::with_capacity(stage_files.len());
    for file in stage_files {
        let file = file
            .as_str()
            .ok_or_else(|| ManifestError::Shape(format!("pass.{name}.stages")))?;
        let entry_points = entries
            .get(file)
            .ok_or_else(|| ManifestError::MissingSource {
                pass: name.to_string(),
                file: file.to_string(),
            })?;
        if entry_points.is_empty() {
            return Err(ManifestError::NoEntryPoint {
                pass: name.to_string(),
                file: file.to_string(),
            });
        }
        stages.extend(entry_points.iter().map(|entry_point| StageSource {
            source_file: file.to_string(),
            entry: entry_point.name.clone(),
            stage: entry_point.stage,
        }));
    }
    validate_stage_composition(name, &stages)?;

    let set_table = table
        .get("sets")
        .and_then(Value::as_table)
        .ok_or_else(|| ManifestError::Shape(format!("pass.{name}.sets")))?;
    let mut sets = Vec::with_capacity(set_table.len());
    for (key, role) in set_table {
        let set = key
            .parse::<u32>()
            .map_err(|_| ManifestError::InvalidSetIndex {
                pass: name.to_string(),
                key: key.clone(),
            })?;
        let role_name = role
            .as_str()
            .ok_or_else(|| ManifestError::Shape(format!("pass.{name}.sets.{key}")))?;
        let role = SetRole::parse(role_name).ok_or_else(|| ManifestError::UnknownSetRole {
            pass: name.to_string(),
            role: role_name.to_string(),
        })?;
        sets.push((set, role));
    }
    sets.sort_by_key(|(set, _)| *set);
    validate_set_roles(name, &sets)?;

    Ok(PassDefinition {
        name: name.to_string(),
        stages,
        sets,
    })
}

fn validate_pass_name(name: &str) -> Result<(), ManifestError> {
    let mut chars = name.chars();
    let starts_lower = chars.next().is_some_and(|c| c.is_ascii_lowercase());
    let rest_valid = chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if starts_lower && rest_valid {
        Ok(())
    } else {
        Err(ManifestError::InvalidPassName(name.to_string()))
    }
}

fn validate_stage_composition(name: &str, stages: &[StageSource]) -> Result<(), ManifestError> {
    let count = |kind: StageKind| stages.iter().filter(|stage| stage.stage == kind).count();
    let (vertex, fragment, geometry, compute) = (
        count(StageKind::Vertex),
        count(StageKind::Fragment),
        count(StageKind::Geometry),
        count(StageKind::Compute),
    );

    let is_graphics = vertex == 1 && fragment == 1 && geometry <= 1 && compute == 0;
    let is_compute = compute == 1 && stages.len() == 1;
    if is_graphics || is_compute {
        return Ok(());
    }

    // RT pipeline: must have at least one RT stage (rgen/rint/rahit/rchit/rmiss) and no graphics/compute stages
    let rt_count = count(StageKind::RayGeneration)
        + count(StageKind::Intersection)
        + count(StageKind::AnyHit)
        + count(StageKind::ClosestHit)
        + count(StageKind::Miss);
    let is_rt = rt_count > 0 && vertex == 0 && fragment == 0 && geometry == 0 && compute == 0;
    if is_rt {
        return Ok(());
    }

    Err(ManifestError::StageComposition {
        pass: name.to_string(),
        reason: format!(
            "stages must be one vertex + one fragment (+ optional geometry) entry or exactly one compute entry, got {} vertex / {} fragment / {} geometry / {} compute",
            vertex, fragment, geometry, compute
        ),
    })
}

fn validate_set_roles(name: &str, sets: &[(u32, SetRole)]) -> Result<(), ManifestError> {
    let mut seen_sets = BTreeSet::new();
    let mut seen_roles = BTreeSet::new();
    for (set, role) in sets {
        if !seen_sets.insert(*set) {
            return Err(ManifestError::DuplicateSetIndex {
                pass: name.to_string(),
                set: *set,
            });
        }
        if !seen_roles.insert(role.variant_name()) {
            return Err(ManifestError::DuplicateRole {
                pass: name.to_string(),
                role: *role,
            });
        }
        if let Some(expected) = role.fixed_set_index() {
            if *set != expected {
                return Err(ManifestError::RoleAtWrongSet {
                    pass: name.to_string(),
                    role: *role,
                    set: *set,
                    expected,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
[pass.model]
stages = ["model/raster.slang"]
sets = { 0 = "frame", 1 = "material", 2 = "object" }

[pass.blur]
stages = ["blurCompute.slang"]
sets = { 0 = "local" }
"#;

    fn entry(name: &str, stage: StageKind) -> EntryPoint {
        EntryPoint {
            name: name.into(),
            stage,
        }
    }

    fn entries() -> ShaderEntries {
        ShaderEntries::from([
            (
                "model/raster.slang".to_string(),
                vec![
                    entry("vertexMain", StageKind::Vertex),
                    entry("fragmentMain", StageKind::Fragment),
                ],
            ),
            (
                "blurCompute.slang".to_string(),
                vec![entry("main", StageKind::Compute)],
            ),
        ])
    }

    fn pair_entries() -> ShaderEntries {
        ShaderEntries::from([
            (
                "aVertex.slang".to_string(),
                vec![entry("main", StageKind::Vertex)],
            ),
            (
                "bFragment.slang".to_string(),
                vec![entry("main", StageKind::Fragment)],
            ),
        ])
    }

    #[test]
    fn parses_graphics_and_compute_passes() {
        let manifest = PassManifest::parse(VALID, &entries()).unwrap();
        assert_eq!(manifest.passes.len(), 2);
        let blur = &manifest.passes[0];
        assert_eq!(blur.name, "blur");
        assert_eq!(blur.stages[0].stage, StageKind::Compute);
        assert_eq!(blur.sets, vec![(0, SetRole::Local)]);
        let model = &manifest.passes[1];
        assert_eq!(model.stages.len(), 2);
        assert_eq!(model.stages[0].entry, "vertexMain");
        assert_eq!(model.stages[1].stage, StageKind::Fragment);
        assert_eq!(
            model.sets,
            vec![
                (0, SetRole::Frame),
                (1, SetRole::Material),
                (2, SetRole::Object)
            ]
        );
    }

    #[test]
    fn rejects_duplicate_pass_names() {
        let text =
            format!("{VALID}\n[pass.model]\nstages = [\"model/raster.slang\"]\nsets = {{}}\n");
        assert!(matches!(
            PassManifest::parse(&text, &entries()),
            Err(ManifestError::Toml(_))
        ));
    }

    #[test]
    fn rejects_bad_stage_composition() {
        let text = "[pass.p]\nstages = [\"aVertex.slang\"]\nsets = {}\n";
        assert!(matches!(
            PassManifest::parse(text, &pair_entries()),
            Err(ManifestError::StageComposition { .. })
        ));
        let text =
            "[pass.p]\nstages = [\"blurCompute.slang\", \"model/raster.slang\"]\nsets = {}\n";
        assert!(matches!(
            PassManifest::parse(text, &entries()),
            Err(ManifestError::StageComposition { .. })
        ));
    }

    #[test]
    fn rejects_roles_at_wrong_set() {
        let text = "[pass.p]\nstages = [\"aVertex.slang\", \"bFragment.slang\"]\nsets = { 1 = \"frame\" }\n";
        assert_eq!(
            PassManifest::parse(text, &pair_entries()),
            Err(ManifestError::RoleAtWrongSet {
                pass: "p".into(),
                role: SetRole::Frame,
                set: 1,
                expected: 0
            })
        );
    }

    #[test]
    fn rejects_unknown_role_and_bad_pass_name() {
        let text = "[pass.p]\nstages = [\"aVertex.slang\", \"bFragment.slang\"]\nsets = { 0 = \"world\" }\n";
        assert!(matches!(
            PassManifest::parse(text, &pair_entries()),
            Err(ManifestError::UnknownSetRole { .. })
        ));
        let text = "[pass.BadName]\nstages = [\"aVertex.slang\", \"bFragment.slang\"]\nsets = {}\n";
        assert_eq!(
            PassManifest::parse(text, &pair_entries()),
            Err(ManifestError::InvalidPassName("BadName".into()))
        );
    }

    #[test]
    fn detects_missing_and_orphan_sources() {
        let mut with_orphan = entries();
        with_orphan.insert(
            "nested/orphanFragment.slang".into(),
            vec![entry("main", StageKind::Fragment)],
        );
        assert_eq!(
            PassManifest::parse(VALID, &with_orphan),
            Err(ManifestError::OrphanShader(
                "nested/orphanFragment.slang".into()
            ))
        );

        let mut without_blur = entries();
        without_blur.remove("blurCompute.slang");
        assert!(matches!(
            PassManifest::parse(VALID, &without_blur),
            Err(ManifestError::MissingSource { .. })
        ));
    }

    #[test]
    fn collects_entry_files_and_skips_modules() {
        let dir = std::env::temp_dir().join(format!(
            "thyllore_shader_manifest_dirs_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(dir.join("flame")).unwrap();
        std::fs::create_dir_all(dir.join("include")).unwrap();
        std::fs::write(
            dir.join("flame/blurCompute.slang"),
            "[shader(\"compute\")]\n[numthreads(8, 8, 1)]\nvoid main(uint3 id : SV_DispatchThreadID) {}\n",
        )
        .unwrap();
        std::fs::write(dir.join("include/noise.slang"), "module noise;\n").unwrap();

        let sources = collect_shader_sources(&dir).unwrap();
        assert_eq!(
            shader_entries(&sources),
            ShaderEntries::from([(
                "flame/blurCompute.slang".to_string(),
                vec![entry("main", StageKind::Compute)]
            )])
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
