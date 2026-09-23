use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CppAbiParseError {
    #[error("export signature `{0}` has no parameter list")]
    MalformedSignature(String),
    #[error("`{context}` names `{type_name}`, which is not a struct of the generated C++")]
    UnknownStruct { context: String, type_name: String },
    #[error("`GlobalParams_0` should hold exactly one uniform block pointer, found {0}")]
    MalformedGlobalParams(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CppField {
    pub type_name: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CppStruct {
    pub name: String,
    pub fields: Vec<CppField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CppParam {
    pub type_name: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CppExport {
    pub return_type: String,
    pub name: String,
    pub params: Vec<CppParam>,
}

/// How the `-target cpp` backend hands the global `ConstantBuffer` to the exports: through
/// `KernelContext_0 { GlobalParams_0* } / GlobalParams_0 { UBO*, <other globals> }` or with the
/// block embedded by value in `KernelContext_0`. Modules without a global block get no context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UniformPassing {
    None,
    ByPointer {
        block: String,
        field: String,
        other_globals: Vec<CppField>,
    },
    ByValue {
        block: String,
        field: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CppModule {
    pub structs: Vec<CppStruct>,
    pub exports: Vec<CppExport>,
    pub uniform: UniformPassing,
}

impl CppModule {
    pub fn find_struct(&self, name: &str) -> Option<&CppStruct> {
        self.structs.iter().find(|item| item.name == name)
    }
}

pub fn parse_module(content: &str) -> Result<CppModule, CppAbiParseError> {
    let structs = parse_structs(content);
    let exports = parse_exports(content)?;
    let uniform = parse_uniform_passing(&structs)?;
    Ok(CppModule {
        structs,
        exports,
        uniform,
    })
}

fn parse_structs(content: &str) -> Vec<CppStruct> {
    let mut structs = Vec::new();
    let mut lines = content.lines().map(str::trim).peekable();
    while let Some(line) = lines.next() {
        let Some(name) = line.strip_prefix("struct ") else {
            continue;
        };
        let name = name.trim_end_matches(';').trim();
        if name.is_empty() || name.contains(' ') || lines.peek() != Some(&"{") {
            continue;
        }
        lines.next();

        let mut fields = Vec::new();
        for body_line in lines.by_ref() {
            if body_line.starts_with("};") {
                break;
            }
            if let Some(field) = parse_field(body_line) {
                fields.push(field);
            }
        }
        structs.push(CppStruct {
            name: name.to_string(),
            fields,
        });
    }
    structs
}

fn parse_field(line: &str) -> Option<CppField> {
    let declaration = line.strip_suffix(';')?;
    let (type_name, name) = split_trailing_identifier(declaration)?;
    Some(CppField {
        type_name: normalize_spaces(type_name),
        name: name.to_string(),
    })
}

fn parse_exports(content: &str) -> Result<Vec<CppExport>, CppAbiParseError> {
    let lines: Vec<&str> = content.lines().map(str::trim).collect();
    let mut exports = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if *line != "SLANG_PRELUDE_EXPORT" {
            continue;
        }
        let Some(signature) = lines.get(index + 1) else {
            continue;
        };
        if signature.starts_with("static ") {
            continue;
        }
        exports.push(parse_signature(signature)?);
    }
    Ok(exports)
}

fn parse_signature(signature: &str) -> Result<CppExport, CppAbiParseError> {
    let malformed = || CppAbiParseError::MalformedSignature(signature.to_string());
    let signature = signature.trim().trim_end_matches(';');
    let open = signature.find('(').ok_or_else(malformed)?;
    let close = signature.rfind(')').ok_or_else(malformed)?;
    let (return_type, name) =
        split_trailing_identifier(&signature[..open]).ok_or_else(malformed)?;

    let params = split_top_level(&signature[open + 1..close], ',')
        .into_iter()
        .filter(|param| !param.trim().is_empty())
        .map(|param| {
            let (type_name, name) = split_trailing_identifier(param).ok_or_else(malformed)?;
            Ok(CppParam {
                type_name: normalize_spaces(type_name),
                name: name.to_string(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(CppExport {
        return_type: normalize_spaces(return_type),
        name: name.to_string(),
        params,
    })
}

fn parse_uniform_passing(structs: &[CppStruct]) -> Result<UniformPassing, CppAbiParseError> {
    let Some(context) = structs.iter().find(|item| item.name == "KernelContext_0") else {
        return Ok(UniformPassing::None);
    };
    let [field] = context.fields.as_slice() else {
        return Ok(UniformPassing::None);
    };

    if let Some(pointee) = field.type_name.strip_suffix('*') {
        let global_params = structs
            .iter()
            .find(|item| item.name == pointee.trim())
            .ok_or_else(|| CppAbiParseError::UnknownStruct {
                context: "KernelContext_0".into(),
                type_name: pointee.trim().to_string(),
            })?;
        let (block_fields, other_globals): (Vec<&CppField>, Vec<&CppField>) = global_params
            .fields
            .iter()
            .partition(|field| field.type_name.ends_with('*'));
        let [block_field] = block_fields.as_slice() else {
            return Err(CppAbiParseError::MalformedGlobalParams(block_fields.len()));
        };
        let block = block_field
            .type_name
            .trim_end_matches('*')
            .trim()
            .to_string();
        return Ok(UniformPassing::ByPointer {
            block,
            field: strip_suffix_index(&block_field.name).to_string(),
            other_globals: other_globals.into_iter().cloned().collect(),
        });
    }

    Ok(UniformPassing::ByValue {
        block: field.type_name.clone(),
        field: strip_suffix_index(&field.name).to_string(),
    })
}

/// `Vector<float, 3>  o_13` -> (`Vector<float, 3>`, `o_13`); `KernelContext_0 * ctx` keeps the
/// `*` on the type side.
pub fn split_trailing_identifier(text: &str) -> Option<(&str, &str)> {
    let text = text.trim();
    let name_start = text.rfind(|c: char| !c.is_alphanumeric() && c != '_')? + 1;
    let (type_name, name) = (text[..name_start].trim(), &text[name_start..]);
    if type_name.is_empty() || name.is_empty() {
        return None;
    }
    Some((type_name, name))
}

pub fn split_top_level(text: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (index, character) in text.char_indices() {
        match character {
            '<' => depth += 1,
            '>' => depth -= 1,
            c if c == separator && depth == 0 => {
                parts.push(&text[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    if start < text.len() {
        parts.push(&text[start..]);
    }
    parts
}

/// The backend suffixes every identifier with `_<n>` to keep it unique; `pad0_11` -> `pad0`,
/// `WindUBO_natural_0` -> `WindUBO_natural`.
pub fn strip_suffix_index(name: &str) -> &str {
    match name.rsplit_once('_') {
        Some((base, suffix))
            if !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()) =>
        {
            base
        }
        _ => name,
    }
}

/// `WindUBO_natural_0` / `WaterUBO_0` -> `WindUBO` / `WaterUBO`, the Slang type name.
pub fn slang_type_name(cpp_name: &str) -> &str {
    let base = strip_suffix_index(cpp_name);
    base.strip_suffix("_natural").unwrap_or(base)
}

fn normalize_spaces(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
#include "slang-cpp-prelude.h"

struct VolumeShell_0
{
    float height_0;
    float radiusBase_0;
};

struct WindUBO_natural_0
{
    _MatrixStorage_float4x4_ColMajornatural_0 model_0;
    Vector<float, 4>  shape_0;
    FixedArray<Vector<float, 4> , 96>  puffs_0;
};

struct GlobalParams_0
{
    WindUBO_natural_0* wind_0;
    Texture2D<Vector<float, 4> > windSdfSampler_0;
};

struct KernelContext_0
{
    GlobalParams_0* globalParams_0;
};

SLANG_PRELUDE_EXPORT
static void helper_0(float x)
{
}

SLANG_PRELUDE_EXPORT
float thylloreWindOpticalDepth(Vector<float, 3>  o_13, Vector<float, 3>  d_16, float tNear_7, KernelContext_0 * kernelContext_48)
{
}

SLANG_PRELUDE_EXPORT
bool thylloreWindClampRayToCone(VolumeShell_0 shell_0, float * tNearOut_0, KernelContext_0 * kernelContext_49)
{
}
"#;

    #[test]
    fn parses_structs_exports_and_pointer_uniform_passing() {
        let module = parse_module(SAMPLE).unwrap();
        assert_eq!(
            module
                .structs
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>(),
            [
                "VolumeShell_0",
                "WindUBO_natural_0",
                "GlobalParams_0",
                "KernelContext_0"
            ]
        );
        assert_eq!(
            module.find_struct("WindUBO_natural_0").unwrap().fields[2],
            CppField {
                type_name: "FixedArray<Vector<float, 4> , 96>".into(),
                name: "puffs_0".into()
            }
        );
        assert_eq!(
            module.uniform,
            UniformPassing::ByPointer {
                block: "WindUBO_natural_0".into(),
                field: "wind".into(),
                other_globals: vec![CppField {
                    type_name: "Texture2D<Vector<float, 4> >".into(),
                    name: "windSdfSampler_0".into()
                }],
            }
        );

        assert_eq!(module.exports.len(), 2);
        let clamp = &module.exports[1];
        assert_eq!(clamp.return_type, "bool");
        assert_eq!(clamp.name, "thylloreWindClampRayToCone");
        assert_eq!(
            clamp.params,
            vec![
                CppParam {
                    type_name: "VolumeShell_0".into(),
                    name: "shell_0".into()
                },
                CppParam {
                    type_name: "float *".into(),
                    name: "tNearOut_0".into()
                },
                CppParam {
                    type_name: "KernelContext_0 *".into(),
                    name: "kernelContext_49".into()
                },
            ]
        );
    }

    #[test]
    fn value_uniform_passing_and_no_context() {
        let by_value = "struct WaterUBO_0\n{\n    float x_0;\n};\nstruct KernelContext_0\n{\n    WaterUBO_0 water_0;\n};\n";
        assert_eq!(
            parse_module(by_value).unwrap().uniform,
            UniformPassing::ByValue {
                block: "WaterUBO_0".into(),
                field: "water".into()
            }
        );
        assert_eq!(parse_module("").unwrap().uniform, UniformPassing::None);
    }

    #[test]
    fn strips_backend_suffixes() {
        assert_eq!(strip_suffix_index("pad0_11"), "pad0");
        assert_eq!(
            strip_suffix_index("trail_coefficients_0"),
            "trail_coefficients"
        );
        assert_eq!(strip_suffix_index("hash01"), "hash01");
        assert_eq!(slang_type_name("WindUBO_natural_0"), "WindUBO");
        assert_eq!(slang_type_name("WaterUBO_0"), "WaterUBO");
        assert_eq!(
            slang_type_name("FlameBranchElement_0"),
            "FlameBranchElement"
        );
    }
}
