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
        "shaders/model/vertex.slang",
        "shaders/model/fragment.slang",
        "shaders/gbuffer/vertex.slang",
        "shaders/gbuffer/fragment.slang",
        "shaders/postprocess/compositeVertex.slang",
        "shaders/postprocess/compositeFragment.slang",
        "shaders/editor/gridVertex.slang",
        "shaders/editor/gridFragment.slang",
        "shaders/editor/gizmoVertex.slang",
        "shaders/editor/gizmoFragment.slang",
        "shaders/editor/imguiVertex.slang",
        "shaders/editor/imguiFragment.slang",
        "shaders/editor/boneVertex.slang",
        "shaders/editor/boneFragment.slang",
        "shaders/raytracing/rayQueryShadowCompute.slang",
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
        ("shaders/model/vertex.slang", true),
        ("shaders/gbuffer/vertex.slang", true),
        ("shaders/postprocess/compositeVertex.slang", true),
        ("shaders/editor/gridVertex.slang", true),
        ("shaders/editor/gizmoVertex.slang", true),
        ("shaders/editor/imguiVertex.slang", true),
        ("shaders/editor/boneVertex.slang", true),
    ];

    for (shader, is_slang) in &vertex_shaders {
        if *is_slang {
            assert!(
                shader.ends_with(".slang"),
                "Slang vertex shader should have .slang extension: {}",
                shader
            );
            let stem = Path::new(shader).file_stem().unwrap().to_str().unwrap();
            assert!(
                stem.ends_with("Vertex") || stem.ends_with("vertex"),
                "Slang vertex shader stem should end with Vertex or vertex: {}",
                shader
            );
        } else {
            assert!(
                shader.ends_with(".vert"),
                "GLSL vertex shader should have .vert extension: {}",
                shader
            );
        }
    }
}

#[test]
fn test_fragment_shader_extension() {
    let fragment_shaders = [
        ("shaders/model/fragment.slang", true),
        ("shaders/gbuffer/fragment.slang", true),
        ("shaders/postprocess/compositeFragment.slang", true),
        ("shaders/editor/gridFragment.slang", true),
        ("shaders/editor/gizmoFragment.slang", true),
        ("shaders/editor/imguiFragment.slang", true),
        ("shaders/editor/boneFragment.slang", true),
    ];

    for (shader, is_slang) in &fragment_shaders {
        if *is_slang {
            assert!(
                shader.ends_with(".slang"),
                "Slang fragment shader should have .slang extension: {}",
                shader
            );
            let stem = Path::new(shader).file_stem().unwrap().to_str().unwrap();
            assert!(
                stem.ends_with("Fragment") || stem.ends_with("fragment"),
                "Slang fragment shader stem should end with Fragment or fragment: {}",
                shader
            );
        } else {
            assert!(
                shader.ends_with(".frag"),
                "GLSL fragment shader should have .frag extension: {}",
                shader
            );
        }
    }
}

#[test]
fn test_compute_shader_extension() {
    let compute_shaders = [("shaders/raytracing/rayQueryShadowCompute.slang", true)];

    for (shader, is_slang) in &compute_shaders {
        if *is_slang {
            assert!(
                shader.ends_with(".slang"),
                "Slang compute shader should have .slang extension: {}",
                shader
            );
            let stem = Path::new(shader).file_stem().unwrap().to_str().unwrap();
            assert!(
                stem.ends_with("Compute") || stem.ends_with("compute"),
                "Slang compute shader stem should end with Compute or compute: {}",
                shader
            );
        } else {
            assert!(
                shader.ends_with(".comp"),
                "GLSL compute shader should have .comp extension: {}",
                shader
            );
        }
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
                let file_name = path.file_name().and_then(|n| n.to_str());
                if file_name == Some("include") || file_name == Some("cpu") {
                    return 0;
                }
                return count_shader_sources(&path);
            }

            let extension = path.extension().and_then(|e| e.to_str());
            let is_glsl = extension.is_some_and(|ext| SHADER_SOURCE_EXTENSIONS.contains(&ext));
            let is_slang = extension == Some("slang");

            usize::from(is_glsl || is_slang)
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
