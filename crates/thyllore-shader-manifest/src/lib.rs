mod bindings_codegen;
mod codegen;
mod flame_gpu_blocks;
mod gpu_block_codegen;
mod manifest;
mod naming;
mod slang;
mod spirv_files;
mod stage;

pub use bindings_codegen::{generate_shader_bindings_rust, BindingCodegenError};
pub use codegen::generate_pass_manifest_rust;
pub use flame_gpu_blocks::{
    gpu_blocks_source, FlameGpuBlocksError, GpuBlockTarget, GPU_BLOCK_TARGETS,
    REGENERATE_GPU_BLOCKS_COMMAND,
};
pub use gpu_block_codegen::{
    generate_gpu_blocks_rust, GpuBlockCodegenConfig, GpuBlockCodegenError,
};
pub use manifest::{
    collect_shader_sources, shader_entries, ManifestError, PassDefinition, PassManifest, SetRole,
    ShaderEntries, ShaderSource, StageSource,
};
pub use naming::{is_shader_source, parse_entry_points, spirv_output_name, EntryPoint};
pub use slang::slang_root;
pub use spirv_files::collect_spirv_files;
pub use stage::StageKind;
