const GLSL_EXTENSIONS: [&str; 9] = [
    "vert", "frag", "geom", "comp", "rgen", "rint", "rahit", "rchit", "rmiss",
];
const SLANG_EXTENSION: &str = "slang";

pub fn is_shader_source(file_name: &str) -> bool {
    is_glsl_source(file_name) || file_extension(file_name) == Some(SLANG_EXTENSION)
}

pub fn is_glsl_source(file_name: &str) -> bool {
    file_extension(file_name).is_some_and(|extension| GLSL_EXTENSIONS.contains(&extension))
}

pub fn spirv_output_name(source_path: &str) -> Option<String> {
    let (directory, file_name) = match source_path.rsplit_once('/') {
        Some((directory, file_name)) => (format!("{directory}/"), file_name),
        None => (String::new(), source_path),
    };
    let extension = file_extension(file_name)?;
    let stem = &file_name[..file_name.len() - extension.len() - 1];

    if extension == SLANG_EXTENSION {
        return slang_spirv_output(&directory, stem);
    }

    glsl_spirv_output(&directory, stem, extension)
}

fn glsl_spirv_output(directory: &str, stem: &str, extension: &str) -> Option<String> {
    let base_name = stem
        .trim_end_matches("Vertex")
        .trim_end_matches("vertex")
        .trim_end_matches("Fragment")
        .trim_end_matches("fragment")
        .trim_end_matches("Geometry")
        .trim_end_matches("geometry")
        .trim_end_matches("Compute")
        .trim_end_matches("compute")
        .trim_end_matches("RayGen")
        .trim_end_matches("raygen")
        .trim_end_matches("Intersection")
        .trim_end_matches("intersection")
        .trim_end_matches("AnyHit")
        .trim_end_matches("anyhit")
        .trim_end_matches("ClosestHit")
        .trim_end_matches("closesthit")
        .trim_end_matches("Miss")
        .trim_end_matches("miss");

    let stage_suffix = match extension {
        "vert" => "Vert",
        "frag" => "Frag",
        "geom" => "Geom",
        "comp" => "Comp",
        "rgen" => "Rgen",
        "rint" => "Rint",
        "rahit" => "Rahit",
        "rchit" => "Rchit",
        "rmiss" => "Rmiss",
        _ => return None,
    };

    Some(spirv_file_name(directory, base_name, stage_suffix))
}

fn slang_spirv_output(directory: &str, stem: &str) -> Option<String> {
    let (base_name, stage_suffix) = slang_stage_from_stem(stem)?;
    Some(spirv_file_name(directory, base_name, stage_suffix))
}

fn spirv_file_name(directory: &str, base_name: &str, stage_suffix: &str) -> String {
    if base_name.is_empty() {
        format!("{directory}{}.spv", stage_suffix.to_ascii_lowercase())
    } else {
        format!("{directory}{base_name}{stage_suffix}.spv")
    }
}

fn slang_stage_from_stem(stem: &str) -> Option<(&str, &str)> {
    let suffixes: &[(&str, &str)] = &[
        ("Vertex", "Vert"),
        ("Fragment", "Frag"),
        ("Geometry", "Geom"),
        ("Compute", "Comp"),
        ("RayGen", "Rgen"),
        ("Intersection", "Rint"),
        ("AnyHit", "Rahit"),
        ("ClosestHit", "Rchit"),
        ("Miss", "Rmiss"),
    ];

    for (suffix, stage) in suffixes {
        if let Some(base) = stem.strip_suffix(suffix) {
            return Some((base, stage));
        }
    }
    None
}

fn file_extension(file_name: &str) -> Option<&str> {
    let (_, extension) = file_name.rsplit_once('.')?;
    Some(extension)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_stage_word_and_appends_stage_suffix() {
        assert_eq!(
            spirv_output_name("vertex.vert").as_deref(),
            Some("vert.spv")
        );
        assert_eq!(
            spirv_output_name("fragment.frag").as_deref(),
            Some("frag.spv")
        );
        assert_eq!(
            spirv_output_name("gbufferVertex.vert").as_deref(),
            Some("gbufferVert.spv")
        );
        assert_eq!(
            spirv_output_name("imguiFragment.frag").as_deref(),
            Some("imguiFrag.spv")
        );
        assert_eq!(
            spirv_output_name("rayQueryShadow.comp").as_deref(),
            Some("rayQueryShadowComp.spv")
        );
        assert_eq!(
            spirv_output_name("histogramCompute.comp").as_deref(),
            Some("histogramComp.spv")
        );
    }

    #[test]
    fn keeps_source_subdirectory_in_output_path() {
        assert_eq!(
            spirv_output_name("water/causticSplat.comp").as_deref(),
            Some("water/causticSplatComp.spv")
        );
        assert_eq!(
            spirv_output_name("flame/resolveFragment.frag").as_deref(),
            Some("flame/resolveFrag.spv")
        );
        assert_eq!(
            spirv_output_name("gbuffer/vertex.vert").as_deref(),
            Some("gbuffer/vert.spv")
        );
    }

    #[test]
    fn rt_strips_stage_word_and_appends_stage_suffix() {
        assert_eq!(
            spirv_output_name("traceRayGen.rgen").as_deref(),
            Some("traceRgen.spv")
        );
        assert_eq!(
            spirv_output_name("torusIntersection.rint").as_deref(),
            Some("torusRint.spv")
        );
        assert_eq!(
            spirv_output_name("torusClosestHit.rchit").as_deref(),
            Some("torusRchit.spv")
        );
        assert_eq!(
            spirv_output_name("traceMiss.rmiss").as_deref(),
            Some("traceRmiss.spv")
        );
    }

    #[test]
    fn rejects_non_shader_files() {
        assert_eq!(spirv_output_name("passes.toml"), None);
        assert_eq!(spirv_output_name("common.glsl"), None);
        assert!(!is_shader_source("include"));
        assert!(is_shader_source("dofFragment.frag"));
    }

    #[test]
    fn slang_strips_stage_word_and_appends_stage_suffix() {
        assert_eq!(
            spirv_output_name("wind/resolveFragment.slang").as_deref(),
            Some("wind/resolveFrag.spv")
        );
        assert_eq!(
            spirv_output_name("wind/shadowBakeCompute.slang").as_deref(),
            Some("wind/shadowBakeComp.spv")
        );
        assert_eq!(
            spirv_output_name("raytracing/traceRayGen.slang").as_deref(),
            Some("raytracing/traceRgen.spv")
        );
        assert_eq!(spirv_output_name("foo.slang"), None);
    }

    #[test]
    fn is_shader_source_accepts_slang() {
        assert!(is_shader_source("resolveFragment.slang"));
        assert!(is_shader_source("shadowBakeCompute.slang"));
        assert!(!is_glsl_source("resolveFragment.slang"));
        assert!(is_glsl_source("dofFragment.frag"));
    }
}
