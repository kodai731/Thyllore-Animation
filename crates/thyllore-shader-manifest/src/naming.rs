const SLANG_EXTENSION: &str = "slang";
const SLANG_STAGE_WORDS: [(&str, &str); 9] = [
    ("Vertex", "vert"),
    ("Fragment", "frag"),
    ("Geometry", "geom"),
    ("Compute", "comp"),
    ("RayGen", "rgen"),
    ("Intersection", "rint"),
    ("AnyHit", "rahit"),
    ("ClosestHit", "rchit"),
    ("Miss", "rmiss"),
];

pub fn is_shader_source(file_name: &str) -> bool {
    file_extension(file_name) == Some(SLANG_EXTENSION)
}

pub fn spirv_output_name(source_path: &str) -> Option<String> {
    let (directory, file_name) = match source_path.rsplit_once('/') {
        Some((directory, file_name)) => (format!("{directory}/"), file_name),
        None => (String::new(), source_path),
    };
    let extension = file_extension(file_name)?;
    if extension != SLANG_EXTENSION {
        return None;
    }
    let stem = &file_name[..file_name.len() - extension.len() - 1];
    slang_spirv_output(&directory, stem)
}

fn slang_spirv_output(directory: &str, stem: &str) -> Option<String> {
    let (base_name, extension) = slang_stage_from_stem(stem)?;
    Some(spirv_file_name(
        directory,
        base_name,
        stage_suffix(extension)?,
    ))
}

fn stage_suffix(extension: &str) -> Option<&'static str> {
    match extension {
        "vert" => Some("Vert"),
        "frag" => Some("Frag"),
        "geom" => Some("Geom"),
        "comp" => Some("Comp"),
        "rgen" => Some("Rgen"),
        "rint" => Some("Rint"),
        "rahit" => Some("Rahit"),
        "rchit" => Some("Rchit"),
        "rmiss" => Some("Rmiss"),
        _ => None,
    }
}

pub fn slang_stage_extension(file_name: &str) -> Option<&'static str> {
    let extension = file_extension(file_name)?;
    if extension != SLANG_EXTENSION {
        return None;
    }
    let stem = &file_name[..file_name.len() - extension.len() - 1];
    slang_stage_from_stem(stem).map(|(_, stage_extension)| stage_extension)
}

fn spirv_file_name(directory: &str, base_name: &str, stage_suffix: &str) -> String {
    if base_name.is_empty() {
        format!("{directory}{}.spv", stage_suffix.to_ascii_lowercase())
    } else {
        format!("{directory}{base_name}{stage_suffix}.spv")
    }
}

fn slang_stage_from_stem(stem: &str) -> Option<(&str, &'static str)> {
    let candidates: [(&str, &str); 18] = [
        ("ClosestHit", "rchit"),
        ("closesthit", "rchit"),
        ("Intersection", "rint"),
        ("intersection", "rint"),
        ("AnyHit", "rahit"),
        ("anyhit", "rahit"),
        ("Miss", "rmiss"),
        ("miss", "rmiss"),
        ("Vertex", "vert"),
        ("vertex", "vert"),
        ("Fragment", "frag"),
        ("fragment", "frag"),
        ("Geometry", "geom"),
        ("geometry", "geom"),
        ("Compute", "comp"),
        ("compute", "comp"),
        ("RayGen", "rgen"),
        ("raygen", "rgen"),
    ];
    candidates
        .iter()
        .find_map(|(word, extension)| stem.strip_suffix(*word).map(|base| (base, *extension)))
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
        assert_eq!(spirv_output_name("passes.toml"), None);
        assert_eq!(spirv_output_name("dofFragment.frag"), None);
        assert!(!is_shader_source("include"));
        assert!(!is_shader_source("dofFragment.frag"));
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
        assert!(!is_shader_source("common.glsl"));
    }

    #[test]
    fn slang_lowercase_stage_suffixes() {
        assert_eq!(
            spirv_output_name("gbuffer/vertex.slang").as_deref(),
            Some("gbuffer/vert.spv")
        );
        assert_eq!(
            spirv_output_name("model/fragment.slang").as_deref(),
            Some("model/frag.spv")
        );
        assert_eq!(
            spirv_output_name("raytracing/shadowQuerycompute.slang").as_deref(),
            Some("raytracing/shadowQueryComp.spv")
        );
    }
}
