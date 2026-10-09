use crate::stage::StageKind;

const SLANG_EXTENSION: &str = "slang";

/// An entry point of a Slang file: the function name and the stage of its `[shader("..")]` attribute.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryPoint {
    pub name: String,
    pub stage: StageKind,
}

pub fn is_shader_source(file_name: &str) -> bool {
    file_extension(file_name) == Some(SLANG_EXTENSION)
}

/// `<directory>/<stem without a trailing stage word><stage suffix>.spv`: `wind/resolveFragment.slang`
/// -> `wind/resolveFrag.spv`, `editor/bone.slang` -> `editor/boneVert.spv` / `editor/boneFrag.spv`.
pub fn spirv_output_name(source_path: &str, stage: StageKind) -> Option<String> {
    let (directory, file_name) = match source_path.rsplit_once('/') {
        Some((directory, file_name)) => (format!("{directory}/"), file_name),
        None => (String::new(), source_path),
    };
    if !is_shader_source(file_name) {
        return None;
    }
    let stem = &file_name[..file_name.len() - SLANG_EXTENSION.len() - 1];
    let base_name = strip_stage_word(stem);
    let suffix = stage.spirv_suffix();
    if base_name.is_empty() {
        Some(format!("{directory}{}.spv", suffix.to_ascii_lowercase()))
    } else {
        Some(format!("{directory}{base_name}{suffix}.spv"))
    }
}

fn strip_stage_word(stem: &str) -> &str {
    StageKind::ALL
        .iter()
        .find_map(|stage| strip_suffix_ignoring_first_case(stem, stage.file_word()))
        .unwrap_or(stem)
}

fn strip_suffix_ignoring_first_case<'a>(stem: &'a str, word: &str) -> Option<&'a str> {
    stem.strip_suffix(word)
        .or_else(|| stem.strip_suffix(&word.to_ascii_lowercase()))
}

/// Every `[shader("<stage>")]` entry point declared in `source`, in file order.
pub fn parse_entry_points(source: &str) -> Vec<EntryPoint> {
    const ATTRIBUTE: &str = "[shader(\"";
    let mut entries = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find(ATTRIBUTE) {
        let after_attribute = &rest[start + ATTRIBUTE.len()..];
        let Some((stage_name, after_stage)) = after_attribute.split_once("\")]") else {
            break;
        };
        if let Some(stage) = StageKind::from_attribute(stage_name) {
            if let Some(name) = function_name_after_attributes(after_stage) {
                entries.push(EntryPoint { name, stage });
            }
        }
        rest = after_stage;
    }
    entries
}

/// The identifier before the parameter list of the next declaration, skipping other `[..]` attributes.
fn function_name_after_attributes(text: &str) -> Option<String> {
    let mut rest = text.trim_start();
    while rest.starts_with('[') {
        let close = rest.find(']')?;
        rest = rest[close + 1..].trim_start();
    }
    let signature = &rest[..rest.find('(')?];
    signature
        .split(|c: char| c.is_whitespace() || c == ':')
        .filter(|token| !token.is_empty())
        .last()
        .map(str::to_string)
}

fn file_extension(file_name: &str) -> Option<&str> {
    let (_, extension) = file_name.rsplit_once('.')?;
    Some(extension)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_shader_files() {
        assert_eq!(spirv_output_name("passes.toml", StageKind::Vertex), None);
        assert_eq!(
            spirv_output_name("dofFragment.frag", StageKind::Fragment),
            None
        );
        assert!(!is_shader_source("include"));
        assert!(!is_shader_source("dofFragment.frag"));
    }

    #[test]
    fn strips_a_trailing_stage_word_and_appends_the_stage_suffix() {
        assert_eq!(
            spirv_output_name("wind/resolveFragment.slang", StageKind::Fragment).as_deref(),
            Some("wind/resolveFrag.spv")
        );
        assert_eq!(
            spirv_output_name("wind/shadowBakeCompute.slang", StageKind::Compute).as_deref(),
            Some("wind/shadowBakeComp.spv")
        );
        assert_eq!(
            spirv_output_name("raytracing/traceRayGen.slang", StageKind::RayGeneration).as_deref(),
            Some("raytracing/traceRgen.spv")
        );
        assert_eq!(
            spirv_output_name("gbuffer/vertex.slang", StageKind::Vertex).as_deref(),
            Some("gbuffer/vert.spv")
        );
    }

    #[test]
    fn a_file_without_a_stage_word_names_one_spirv_per_stage() {
        assert_eq!(
            spirv_output_name("editor/bone.slang", StageKind::Vertex).as_deref(),
            Some("editor/boneVert.spv")
        );
        assert_eq!(
            spirv_output_name("editor/bone.slang", StageKind::Fragment).as_deref(),
            Some("editor/boneFrag.spv")
        );
    }

    #[test]
    fn parses_entry_points_with_their_stages() {
        let source = r#"
[shader("vertex")]
VSOutput vertexMain([[vk::location(0)]] float3 inPosition : POSITION) { }

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 id : SV_DispatchThreadID) { }

[shader("fragment")]
float4 fragmentMain(VSOutput input) : SV_Target0 { }
"#;
        assert_eq!(
            parse_entry_points(source),
            vec![
                EntryPoint {
                    name: "vertexMain".into(),
                    stage: StageKind::Vertex
                },
                EntryPoint {
                    name: "main".into(),
                    stage: StageKind::Compute
                },
                EntryPoint {
                    name: "fragmentMain".into(),
                    stage: StageKind::Fragment
                },
            ]
        );
        assert!(parse_entry_points("module noise;\npublic float hash13(float3 p) { }").is_empty());
    }
}
