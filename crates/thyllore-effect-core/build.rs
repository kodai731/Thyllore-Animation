use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use thyllore_shader_manifest::{
    cpp_command, generate_gpu_blocks_rust, generate_spirv_block_rust, layout_asserts_cpp,
    layout_differences, natural_layout_block, parse_module, reflect_stage_block, rust_bindings,
    slang_root, slang_type_name, spirv_rerun_if_env_changed, verify_slangc_version, CppModule,
    GpuBlockCodegenConfig, SpirvBlock, UniformPassing,
};

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

/// Lightning fills its blocks from Rust alone (the segments are built on the CPU in Rust, not in
/// a Slang CPU module), so like `TracePush` in thyllore-vulkan-core the mirror comes from SPIR-V.
struct GpuOnlyBlock {
    block: SpirvBlock,
    gpu_blocks_file: &'static str,
}

const GPU_ONLY_BLOCKS: [GpuOnlyBlock; 2] = [
    GpuOnlyBlock {
        block: SpirvBlock {
            name: "LightningUBO",
            stage_source: "lightning/resolveFragment.slang",
            imports: &["cgmath::Matrix4"],
        },
        gpu_blocks_file: "lightning_gpu_blocks.rs",
    },
    GpuOnlyBlock {
        block: SpirvBlock {
            name: "LightningSegmentsUBO",
            stage_source: "lightning/resolveFragment.slang",
            imports: &[],
        },
        gpu_blocks_file: "lightning_segments_gpu_blocks.rs",
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

    for entry in &GPU_ONLY_BLOCKS {
        build_gpu_only_block(entry, &slang_root, &shader_root, &out_dir);
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
        )
        .unwrap_or_else(|error| fail(error));
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

fn build_gpu_only_block(
    entry: &GpuOnlyBlock,
    slang_root: &Path,
    shader_root: &Path,
    out_dir: &Path,
) {
    let block = &entry.block;
    let std140 = reflect_stage_block(
        block.name,
        block.stage_source,
        slang_root,
        shader_root,
        out_dir,
    )
    .unwrap_or_else(|error| fail(error));
    let gpu_blocks = generate_spirv_block_rust(&std140, block.stage_source, block.imports)
        .unwrap_or_else(|error| fail(error));
    write(&out_dir.join(entry.gpu_blocks_file), &gpu_blocks);
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
