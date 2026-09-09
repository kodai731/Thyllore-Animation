const SHADER_EXTENSIONS: [&str; 9] = [
    "vert", "frag", "geom", "comp", "rgen", "rint", "rahit", "rchit", "rmiss",
];

pub fn is_shader_source(file_name: &str) -> bool {
    file_extension(file_name).is_some_and(|extension| SHADER_EXTENSIONS.contains(&extension))
}

pub fn spirv_output_name(source_path: &str) -> Option<String> {
    let (directory, file_name) = match source_path.rsplit_once('/') {
        Some((directory, file_name)) => (format!("{directory}/"), file_name),
        None => (String::new(), source_path),
    };
    let extension = file_extension(file_name)?;
    let stem = &file_name[..file_name.len() - extension.len() - 1];

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

    if base_name.is_empty() {
        Some(format!(
            "{directory}{}.spv",
            stage_suffix.to_ascii_lowercase()
        ))
    } else {
        Some(format!("{directory}{base_name}{stage_suffix}.spv"))
    }
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
}
