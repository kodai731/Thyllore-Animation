use std::fs;
use std::path::Path;

#[test]
fn test_shader_source_directory_exists() {
    assert!(
        Path::new("shaders").exists(),
        "Shader source directory should exist"
    );
}

#[test]
fn test_shader_output_directory_exists() {
    assert!(
        Path::new("assets/shaders").exists(),
        "Shader output directory should exist"
    );
}

#[test]
fn test_all_shader_sources_exist() {
    let shader_sources = [
        "shaders/model/vertex.vert",
        "shaders/model/fragment.frag",
        "shaders/gbuffer/vertex.vert",
        "shaders/gbuffer/fragment.frag",
        "shaders/postprocess/compositeVertex.vert",
        "shaders/postprocess/compositeFragment.frag",
        "shaders/editor/gridVertex.vert",
        "shaders/editor/gridFragment.frag",
        "shaders/editor/gizmoVertex.vert",
        "shaders/editor/gizmoFragment.frag",
        "shaders/editor/imguiVertex.vert",
        "shaders/editor/imguiFragment.frag",
        "shaders/raytracing/rayQueryShadow.comp",
    ];

    for shader in &shader_sources {
        assert!(
            Path::new(shader).exists(),
            "Shader source should exist: {}",
            shader
        );
    }
}

#[test]
fn test_all_compiled_shaders_exist() {
    let compiled_shaders = [
        "assets/shaders/model/vert.spv",
        "assets/shaders/model/frag.spv",
        "assets/shaders/gbuffer/vert.spv",
        "assets/shaders/gbuffer/frag.spv",
        "assets/shaders/postprocess/compositeVert.spv",
        "assets/shaders/postprocess/compositeFrag.spv",
        "assets/shaders/editor/gridVert.spv",
        "assets/shaders/editor/gridFrag.spv",
        "assets/shaders/editor/gizmoVert.spv",
        "assets/shaders/editor/gizmoFrag.spv",
        "assets/shaders/editor/imguiVert.spv",
        "assets/shaders/editor/imguiFrag.spv",
        "assets/shaders/raytracing/rayQueryShadowComp.spv",
    ];

    for shader in &compiled_shaders {
        assert!(
            Path::new(shader).exists(),
            "Compiled shader should exist: {}",
            shader
        );
    }
}

#[test]
fn test_compiled_shaders_not_empty() {
    let compiled_shaders = [
        "assets/shaders/model/vert.spv",
        "assets/shaders/model/frag.spv",
        "assets/shaders/gbuffer/vert.spv",
        "assets/shaders/gbuffer/frag.spv",
    ];

    for shader in &compiled_shaders {
        let metadata = fs::metadata(shader)
            .unwrap_or_else(|_| panic!("Failed to read shader metadata: {}", shader));

        assert!(
            metadata.len() > 0,
            "Compiled shader should not be empty: {}",
            shader
        );
    }
}

#[test]
fn test_shader_spv_header() {
    let shader = "assets/shaders/gbuffer/vert.spv";
    let data = fs::read(shader).expect("Failed to read shader file");

    assert!(data.len() >= 4, "Shader file should have at least 4 bytes");

    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    assert_eq!(
        magic, 0x07230203,
        "Shader should have correct SPIR-V magic number"
    );
}

#[test]
fn test_vertex_shader_extension() {
    let vertex_shaders = [
        "shaders/model/vertex.vert",
        "shaders/gbuffer/vertex.vert",
        "shaders/postprocess/compositeVertex.vert",
        "shaders/editor/gridVertex.vert",
        "shaders/editor/gizmoVertex.vert",
        "shaders/editor/imguiVertex.vert",
    ];

    for shader in &vertex_shaders {
        assert!(
            shader.ends_with(".vert"),
            "Vertex shader should have .vert extension: {}",
            shader
        );
    }
}

#[test]
fn test_fragment_shader_extension() {
    let fragment_shaders = [
        "shaders/model/fragment.frag",
        "shaders/gbuffer/fragment.frag",
        "shaders/postprocess/compositeFragment.frag",
        "shaders/editor/gridFragment.frag",
        "shaders/editor/gizmoFragment.frag",
        "shaders/editor/imguiFragment.frag",
    ];

    for shader in &fragment_shaders {
        assert!(
            shader.ends_with(".frag"),
            "Fragment shader should have .frag extension: {}",
            shader
        );
    }
}

#[test]
fn test_compute_shader_extension() {
    let compute_shaders = ["shaders/raytracing/rayQueryShadow.comp"];

    for shader in &compute_shaders {
        assert!(
            shader.ends_with(".comp"),
            "Compute shader should have .comp extension: {}",
            shader
        );
    }
}

const SHADER_SOURCE_EXTENSIONS: [&str; 8] = [
    "vert", "frag", "comp", "geom", "rchit", "rmiss", "rgen", "rint",
];

fn count_compiled_shaders(directory: &Path) -> usize {
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|_| panic!("Failed to read directory: {}", directory.display()));

    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| {
            let path = entry.path();

            if path.is_dir() {
                return count_compiled_shaders(&path);
            }

            usize::from(path.extension() == Some("spv".as_ref()))
        })
        .sum()
}

fn count_shader_sources(directory: &Path) -> usize {
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|_| panic!("Failed to read directory: {}", directory.display()));

    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| {
            let path = entry.path();

            if path.is_dir() {
                if path.file_name() == Some("include".as_ref()) {
                    return 0;
                }
                return count_shader_sources(&path);
            }

            let is_shader_source = path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| SHADER_SOURCE_EXTENSIONS.contains(&extension));

            usize::from(is_shader_source)
        })
        .sum()
}

#[test]
fn test_shader_count_matches() {
    let shader_sources_count = count_shader_sources(Path::new("shaders"));

    let compiled_shaders_count = count_compiled_shaders(Path::new("assets/shaders"));

    assert_eq!(
        shader_sources_count, compiled_shaders_count,
        "Number of shader sources should match compiled shaders"
    );
}
