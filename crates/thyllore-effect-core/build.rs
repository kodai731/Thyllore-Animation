use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use thyllore_shader_manifest::{
    cpp_command, find_declared_block, generate_gpu_blocks_rust, layout_asserts_cpp,
    layout_differences, natural_layout_block, parse_entry_points, parse_module, rust_bindings,
    slang_root, slang_type_name, spirv_command, spirv_rerun_if_env_changed, verify_slangc_version,
    CppModule, GpuBlockCodegenConfig, UniformPassing,
};
use thyllore_spirv_reflect::{reflect_shader_bytes, ReflectedBlock};

/// A uniform block the module reads through its kernel context: its Rust struct is generated
/// from the C++ natural layout and proven equal to the std140 layout of `stage_source`.
struct UniformBlock {
    name: &'static str,
    stage_source: &'static str,
    rust_type: &'static str,
    gpu_blocks_file: &'static str,
    extra_derives: fn() -> BTreeMap<String, Vec<String>>,
}

struct CpuModule {
    exports: &'static str,
    bindings_file: &'static str,
    uniform: Option<UniformBlock>,
}

const MODULES: [CpuModule; 4] = [
    CpuModule {
        exports: "volume_exports.slang",
        bindings_file: "volume_exports_bindings.rs",
        uniform: None,
    },
    CpuModule {
        exports: "wind_exports.slang",
        bindings_file: "wind_exports_bindings.rs",
        uniform: Some(UniformBlock {
            name: "WindUBO",
            stage_source: "wind/resolveFragment.slang",
            rust_type: "crate::wind::WindUBO",
            gpu_blocks_file: "wind_gpu_blocks.rs",
            extra_derives: BTreeMap::new,
        }),
    },
    CpuModule {
        exports: "water_exports.slang",
        bindings_file: "water_exports_bindings.rs",
        uniform: Some(UniformBlock {
            name: "WaterUBO",
            stage_source: "water/resolveFragment.slang",
            rust_type: "crate::water::WaterUBO",
            gpu_blocks_file: "water_gpu_blocks.rs",
            extra_derives: BTreeMap::new,
        }),
    },
    CpuModule {
        exports: "flame_exports.slang",
        bindings_file: "flame_exports_bindings.rs",
        uniform: Some(UniformBlock {
            name: "FlameUBO",
            stage_source: "flame/resolveFragment.slang",
            rust_type: "crate::flame::FlameUBO",
            gpu_blocks_file: "flame_gpu_blocks.rs",
            extra_derives: flame_extra_derives,
        }),
    },
];

/// A uniform block read only on the GPU: its Rust struct is generated straight from the std140
/// layout reflected out of the stage that declares it (no CPU module proves a natural layout).
struct SpirvBlock {
    name: &'static str,
    stage_source: &'static str,
    gpu_blocks_file: &'static str,
    imports: &'static [&'static str],
}

const SPIRV_BLOCKS: [SpirvBlock; 2] = [
    SpirvBlock {
        name: "LightningUBO",
        stage_source: "lightning/resolveFragment.slang",
        gpu_blocks_file: "lightning_gpu_blocks.rs",
        imports: &["cgmath::Matrix4"],
    },
    SpirvBlock {
        name: "LightningSegmentsUBO",
        stage_source: "lightning/resolveFragment.slang",
        gpu_blocks_file: "lightning_segments_gpu_blocks.rs",
        imports: &[],
    },
];

fn flame_extra_derives() -> BTreeMap<String, Vec<String>> {
    let mut derives = BTreeMap::new();
    for name in ["FlameBranchElement", "FlameBranchField"] {
        derives.insert(name.to_string(), vec!["Default".into(), "PartialEq".into()]);
    }
    derives.insert("FlameBranchAgeProfile".into(), vec!["PartialEq".into()]);
    derives
}

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("{message}");
    std::process::exit(1)
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let shader_root = manifest_dir.join("../../shaders");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let slang_root = slang_root();

    spirv_rerun_if_env_changed();
    println!("cargo:rerun-if-changed={}", shader_root.display());
    if let Err(message) = verify_slangc_version(&slang_root) {
        fail(message);
    }

    let mut cpp_files = Vec::new();
    for module in &MODULES {
        cpp_files.push(build_module(module, &slang_root, &shader_root, &out_dir));
    }

    for block in &SPIRV_BLOCKS {
        build_spirv_block(block, &slang_root, &shader_root, &out_dir);
    }

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .flag("-ffp-contract=off")
        .include(slang_root.join("include"))
        .files(cpp_files)
        .compile("thyllore_shader_exports");
}

fn build_module(
    module: &CpuModule,
    slang_root: &Path,
    shader_root: &Path,
    out_dir: &Path,
) -> PathBuf {
    let stem = module
        .exports
        .strip_suffix(".slang")
        .unwrap_or(module.exports);
    let cpp_path = out_dir.join(format!("{stem}.cpp"));
    run(
        cpp_command(
            slang_root,
            shader_root,
            &shader_root.join("cpu").join(module.exports),
        )
        .arg("-o")
        .arg(&cpp_path),
        module.exports,
    );

    let mut cpp_source = fs::read_to_string(&cpp_path).unwrap_or_else(|error| fail(error));
    let parsed = parse_module(&cpp_source).unwrap_or_else(|error| fail(error));

    let rust_type = module.uniform.as_ref().map(|uniform| uniform.rust_type);
    let bindings = rust_bindings(&parsed, stem, rust_type).unwrap_or_else(|error| fail(error));
    write(&out_dir.join(module.bindings_file), &bindings);

    if let Some(uniform) = &module.uniform {
        let block_cpp_name = uniform_block_cpp_name(&parsed, uniform.name, module.exports);
        let std140 = reflect_stage_block(
            uniform.name,
            uniform.stage_source,
            slang_root,
            shader_root,
            out_dir,
        );
        let natural =
            natural_layout_block(&parsed, &block_cpp_name).unwrap_or_else(|error| fail(error));

        let differences = layout_differences(&natural, &std140);
        if !differences.is_empty() {
            fail(format!(
                "{}: the C++ natural layout of {} differs from the std140 layout of {}:\n  {}",
                module.exports,
                uniform.name,
                uniform.stage_source,
                differences.join("\n  ")
            ));
        }

        let config = GpuBlockCodegenConfig {
            header: format!("Generated by build.rs from {stem}.cpp (layout proven equal to the std140 block of {}); do not edit.", uniform.stage_source),
            imports: vec!["cgmath::Matrix4".into()],
            extra_derives: (uniform.extra_derives)(),
        };
        let gpu_blocks =
            generate_gpu_blocks_rust(&natural, &config).unwrap_or_else(|error| fail(error));
        write(&out_dir.join(uniform.gpu_blocks_file), &gpu_blocks);

        let asserts = layout_asserts_cpp(&parsed, &block_cpp_name, &std140)
            .unwrap_or_else(|error| fail(error));
        cpp_source.push_str(&asserts);
        write(&cpp_path, &cpp_source);
    }
    cpp_path
}

fn uniform_block_cpp_name(parsed: &CppModule, expected: &str, exports: &str) -> String {
    let block = match &parsed.uniform {
        UniformPassing::ByPointer { block, .. } | UniformPassing::ByValue { block, .. } => block,
        UniformPassing::None => fail(format!(
            "{exports}: expected the exports to read `{expected}` through a kernel context, but the generated C++ has no KernelContext_0"
        )),
    };
    if slang_type_name(block) != expected {
        fail(format!(
            "{exports}: the kernel context holds `{block}`, not the expected `{expected}`"
        ));
    }
    block.clone()
}

fn build_spirv_block(block: &SpirvBlock, slang_root: &Path, shader_root: &Path, out_dir: &Path) {
    let std140 = reflect_stage_block(
        block.name,
        block.stage_source,
        slang_root,
        shader_root,
        out_dir,
    );
    let config = GpuBlockCodegenConfig {
        header: format!(
            "Generated by build.rs from the std140 block of {}; do not edit.",
            block.stage_source
        ),
        imports: block
            .imports
            .iter()
            .map(|import| import.to_string())
            .collect(),
        extra_derives: BTreeMap::new(),
    };
    let gpu_blocks = generate_gpu_blocks_rust(&std140, &config).unwrap_or_else(|error| fail(error));
    write(&out_dir.join(block.gpu_blocks_file), &gpu_blocks);
}

/// Compiles the stage that reads the block to SPIR-V with the renderer's flags and reflects it.
fn reflect_stage_block(
    name: &str,
    stage_source: &str,
    slang_root: &Path,
    shader_root: &Path,
    out_dir: &Path,
) -> ReflectedBlock {
    let source_path = shader_root.join(stage_source);
    let source = fs::read_to_string(&source_path).unwrap_or_else(|error| fail(error));
    let entry = parse_entry_points(&source)
        .into_iter()
        .next()
        .unwrap_or_else(|| fail(format!("{stage_source}: no [shader] entry point")));

    let spirv_path = out_dir.join(format!("{name}.spv"));
    run(
        spirv_command(slang_root, shader_root, &source_path)
            .args(["-entry", &entry.name])
            .args(["-stage", entry.stage.attribute()])
            .arg("-o")
            .arg(&spirv_path),
        stage_source,
    );

    let bytes = fs::read(&spirv_path).unwrap_or_else(|error| fail(error));
    let reflection = reflect_shader_bytes(&bytes)
        .unwrap_or_else(|error| fail(format!("{stage_source}: {error}")));
    find_declared_block(&reflection, name).unwrap_or_else(|| {
        fail(format!(
            "{stage_source}: SPIR-V declares no uniform block `{name}`"
        ))
    })
}

fn run(command: &mut std::process::Command, what: &str) {
    match command.output() {
        Ok(output) if output.status.success() => {}
        Ok(output) => fail(format!(
            "slangc failed on {what}:\n{}",
            String::from_utf8_lossy(&output.stderr)
        )),
        Err(error) => fail(format!(
            "cannot run slangc for {what}: {error} (SLANG_ROOT/bin/slangc, default ~/.local/slang)"
        )),
    }
}

fn write(path: &Path, content: &str) {
    fs::write(path, content)
        .unwrap_or_else(|error| fail(format!("write {}: {error}", path.display())));
}
