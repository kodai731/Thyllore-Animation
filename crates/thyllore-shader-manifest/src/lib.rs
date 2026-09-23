mod bindings_codegen;
mod codegen;
mod cpp_abi;
mod gpu_block_codegen;
mod manifest;
mod naming;
mod slang;
mod spirv_files;
mod stage;

pub use bindings_codegen::{generate_shader_bindings_rust, BindingCodegenError};
pub use codegen::generate_pass_manifest_rust;
pub use cpp_abi::{
    layout_asserts_cpp, layout_differences, natural_layout_block, parse_module, rust_bindings,
    slang_type_name, CppAbiCodegenError, CppAbiParseError, CppModule, UniformPassing,
};
pub use gpu_block_codegen::{
    find_declared_block, generate_gpu_blocks_rust, GpuBlockCodegenConfig, GpuBlockCodegenError,
};
pub use manifest::{
    collect_shader_sources, shader_entries, ManifestError, PassDefinition, PassManifest, SetRole,
    ShaderEntries, ShaderSource, StageSource,
};
pub use naming::{is_shader_source, parse_entry_points, spirv_output_name, EntryPoint};
pub use slang::{
    cpp_command, slang_root, slang_version, slangc_path, spirv_command, spirv_rerun_if_env_changed,
    verify_slangc_version,
};
pub use spirv_files::collect_spirv_files;
pub use stage::StageKind;
