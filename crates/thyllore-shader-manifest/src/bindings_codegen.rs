use std::fmt::Write;

use thiserror::Error;
use thyllore_spirv_reflect::{
    binding_const_name, DescriptorCount, ReflectedBinding, ReflectedBlock, ShaderReflection,
    ShaderStage,
};

use crate::manifest::{PassDefinition, PassManifest};
use crate::naming::spirv_output_name;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BindingCodegenError {
    #[error("pass `{pass}`: no reflection provided for SPIR-V `{file}`")]
    MissingReflection { pass: String, file: String },
    #[error("pass `{pass}`: set {set} binding {binding} is `{first}` in one stage but `{second}` in another; stages of one pass must agree on descriptor names")]
    NameConflict {
        pass: String,
        set: u32,
        binding: u32,
        first: String,
        second: String,
    },
    #[error("pass `{pass}`: `{first}` and `{second}` both map to constant `{constant}`; rename one descriptor in GLSL")]
    ConstantCollision {
        pass: String,
        first: String,
        second: String,
        constant: String,
    },
    #[error("pass `{pass}`: push constant block differs between `{first}` and `{second}`; every stage must include the same block declaration")]
    PushConstantDiffers {
        pass: String,
        first: String,
        second: String,
    },
}

struct PassPushConstant {
    block: ReflectedBlock,
    stages: Vec<ShaderStage>,
}

/// `reflection_of` is keyed by the SPIR-V path relative to the output directory (`spirv_output_name`).
pub fn generate_shader_bindings_rust(
    manifest: &PassManifest,
    reflection_of: impl Fn(&str) -> Option<ShaderReflection>,
) -> Result<String, BindingCodegenError> {
    let mut out = String::new();
    out.push_str("use thyllore_spirv_reflect::{DescriptorCount, DescriptorKind, ShaderBinding};\n");
    for pass in &manifest.passes {
        write_pass_module(&mut out, pass, &reflection_of)?;
    }
    Ok(out)
}

fn write_pass_module(
    out: &mut String,
    pass: &PassDefinition,
    reflection_of: &impl Fn(&str) -> Option<ShaderReflection>,
) -> Result<(), BindingCodegenError> {
    let reflections = load_pass_reflections(pass, reflection_of)?;
    let bindings = merge_pass_bindings(pass, &reflections)?;
    let push_constant = merge_pass_push_constant(pass, &reflections)?;

    let _ = writeln!(out, "\npub mod {} {{\n    use super::*;\n", pass.name);
    let mut emitted: Vec<(String, String)> = Vec::new();
    for binding in &bindings {
        let constant = binding_const_name(&binding.name);
        if let Some((_, previous)) = emitted.iter().find(|(name, _)| *name == constant) {
            return Err(BindingCodegenError::ConstantCollision {
                pass: pass.name.clone(),
                first: previous.clone(),
                second: binding.name.clone(),
                constant,
            });
        }
        emitted.push((constant.clone(), binding.name.clone()));
        let _ = writeln!(
            out,
            "    pub const {constant}: ShaderBinding = ShaderBinding {{ set: {}, binding: {}, kind: DescriptorKind::{:?}, count: {} }};",
            binding.set,
            binding.binding,
            binding.kind,
            count_expression(binding.count),
        );
    }
    if let Some(push_constant) = push_constant {
        write_push_constant(out, &push_constant);
    }
    out.push_str("}\n");
    Ok(())
}

fn write_push_constant(out: &mut String, push_constant: &PassPushConstant) {
    let stages: Vec<String> = push_constant
        .stages
        .iter()
        .map(|stage| format!("thyllore_spirv_reflect::ShaderStage::{stage:?}"))
        .collect();
    let _ = writeln!(
        out,
        "    pub const PUSH_CONSTANT: thyllore_spirv_reflect::PushConstantLayout = thyllore_spirv_reflect::PushConstantLayout {{ block: \"{}\", stages: &[{}], size: {} }};",
        push_constant.block.type_name,
        stages.join(", "),
        push_constant.block.size,
    );
}

fn load_pass_reflections(
    pass: &PassDefinition,
    reflection_of: &impl Fn(&str) -> Option<ShaderReflection>,
) -> Result<Vec<(String, ShaderReflection)>, BindingCodegenError> {
    pass.stages
        .iter()
        .map(|stage| {
            let spirv_name = spirv_output_name(&stage.source_file, stage.stage)
                .expect("manifest validation guarantees a shader extension");
            let reflection = reflection_of(&spirv_name).ok_or_else(|| {
                BindingCodegenError::MissingReflection {
                    pass: pass.name.clone(),
                    file: spirv_name.clone(),
                }
            })?;
            Ok((spirv_name, reflection))
        })
        .collect()
}

fn merge_pass_push_constant(
    pass: &PassDefinition,
    reflections: &[(String, ShaderReflection)],
) -> Result<Option<PassPushConstant>, BindingCodegenError> {
    let mut merged: Option<PassPushConstant> = None;
    let mut first_file = "";
    for (file, reflection) in reflections {
        let Some(block) = &reflection.push_constant else {
            continue;
        };
        match &mut merged {
            None => {
                merged = Some(PassPushConstant {
                    block: block.clone(),
                    stages: reflection.stages.clone(),
                });
                first_file = file;
            }
            Some(known) if known.block == *block => {
                for stage in &reflection.stages {
                    if !known.stages.contains(stage) {
                        known.stages.push(*stage);
                    }
                }
            }
            Some(_) => {
                return Err(BindingCodegenError::PushConstantDiffers {
                    pass: pass.name.clone(),
                    first: first_file.to_string(),
                    second: file.clone(),
                });
            }
        }
    }
    Ok(merged)
}

fn merge_pass_bindings(
    pass: &PassDefinition,
    reflections: &[(String, ShaderReflection)],
) -> Result<Vec<ReflectedBinding>, BindingCodegenError> {
    let mut merged: Vec<ReflectedBinding> = Vec::new();
    for (_, reflection) in reflections {
        for binding in reflection.bindings.iter().cloned() {
            match merged
                .iter()
                .find(|known| known.set == binding.set && known.binding == binding.binding)
            {
                Some(known) if known.name != binding.name => {
                    return Err(BindingCodegenError::NameConflict {
                        pass: pass.name.clone(),
                        set: binding.set,
                        binding: binding.binding,
                        first: known.name.clone(),
                        second: binding.name,
                    });
                }
                Some(_) => {}
                None => merged.push(binding),
            }
        }
    }
    merged.sort_by_key(|binding| (binding.set, binding.binding));
    Ok(merged)
}

fn count_expression(count: DescriptorCount) -> String {
    match count {
        DescriptorCount::Fixed(count) => format!("DescriptorCount::Fixed({count})"),
        DescriptorCount::Unbounded => "DescriptorCount::Unbounded".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::StageKind;
    use thyllore_spirv_reflect::{DescriptorKind, ShaderStage};

    fn reflection(bindings: Vec<ReflectedBinding>) -> ShaderReflection {
        ShaderReflection {
            stages: vec![ShaderStage::Fragment],
            bindings,
            push_constant: None,
        }
    }

    fn manifest(text: &str, files: &[(&str, crate::stage::StageKind)]) -> PassManifest {
        let entries = files
            .iter()
            .map(|(file, stage)| {
                (
                    file.to_string(),
                    vec![crate::naming::EntryPoint {
                        name: "main".into(),
                        stage: *stage,
                    }],
                )
            })
            .collect();
        PassManifest::parse(text, &entries).unwrap()
    }

    fn binding(set: u32, index: u32, name: &str, kind: DescriptorKind) -> ReflectedBinding {
        ReflectedBinding {
            set,
            binding: index,
            name: name.to_string(),
            kind,
            count: DescriptorCount::Fixed(1),
            block: None,
        }
    }

    #[test]
    fn generates_one_module_per_pass_with_merged_constants() {
        let manifest = manifest(
            "[pass.flame_resolve]\nstages = [\"tonemapVertex.slang\", \"resolveFragment.slang\"]\nsets = { 0 = \"local\" }\n",
            &[
                ("tonemapVertex.slang", StageKind::Vertex),
                ("resolveFragment.slang", StageKind::Fragment),
            ],
        );
        let code = generate_shader_bindings_rust(&manifest, |file| match file {
            "tonemapVert.spv" => Some(reflection(vec![])),
            "resolveFrag.spv" => Some(reflection(vec![
                binding(0, 0, "flame", DescriptorKind::UniformBuffer),
                binding(0, 4, "historySampler", DescriptorKind::CombinedImageSampler),
            ])),
            _ => None,
        })
        .unwrap();
        assert!(code.contains("pub mod flame_resolve {"));
        assert!(code.contains("pub const FLAME: ShaderBinding = ShaderBinding { set: 0, binding: 0, kind: DescriptorKind::UniformBuffer, count: DescriptorCount::Fixed(1) };"));
        assert!(code.contains("pub const HISTORY_SAMPLER: ShaderBinding = ShaderBinding { set: 0, binding: 4, kind: DescriptorKind::CombinedImageSampler, count: DescriptorCount::Fixed(1) };"));
    }

    #[test]
    fn rejects_same_slot_with_different_names_across_stages() {
        let manifest = manifest(
            "[pass.model]\nstages = [\"vertex.slang\", \"fragment.slang\"]\nsets = { 0 = \"frame\" }\n",
            &[
                ("vertex.slang", StageKind::Vertex),
                ("fragment.slang", StageKind::Fragment),
            ],
        );
        let error = generate_shader_bindings_rust(&manifest, |file| match file {
            "vert.spv" => Some(reflection(vec![binding(
                0,
                0,
                "frameData",
                DescriptorKind::UniformBuffer,
            )])),
            "frag.spv" => Some(reflection(vec![binding(
                0,
                0,
                "frame",
                DescriptorKind::UniformBuffer,
            )])),
            _ => None,
        })
        .unwrap_err();
        assert!(matches!(error, BindingCodegenError::NameConflict { .. }));
    }

    #[test]
    fn rejects_two_descriptors_mapping_to_one_constant() {
        let manifest = manifest(
            "[pass.dof]\nstages = [\"tonemapVertex.slang\", \"dofFragment.slang\"]\nsets = { 0 = \"local\" }\n",
            &[
                ("tonemapVertex.slang", StageKind::Vertex),
                ("dofFragment.slang", StageKind::Fragment),
            ],
        );
        let error = generate_shader_bindings_rust(&manifest, |file| match file {
            "tonemapVert.spv" => Some(reflection(vec![])),
            "dofFrag.spv" => Some(reflection(vec![
                binding(0, 0, "hdrSampler", DescriptorKind::CombinedImageSampler),
                binding(0, 1, "hdr_sampler", DescriptorKind::CombinedImageSampler),
            ])),
            _ => None,
        })
        .unwrap_err();
        assert!(matches!(
            error,
            BindingCodegenError::ConstantCollision { .. }
        ));
    }

    fn block(type_name: &str, size: u32) -> ReflectedBlock {
        ReflectedBlock {
            type_name: type_name.to_string(),
            size,
            members: Vec::new(),
        }
    }

    #[test]
    fn emits_push_constant_layout_merged_over_the_stages_declaring_it() {
        let manifest = manifest(
            "[pass.effect_trace]\nstages = [\"traceRayGen.slang\", \"traceMiss.slang\", \"sceneClosestHit.slang\"]\nsets = { 0 = \"local\" }\n",
            &[
                ("traceRayGen.slang", StageKind::RayGeneration),
                ("traceMiss.slang", StageKind::Miss),
                ("sceneClosestHit.slang", StageKind::ClosestHit),
            ],
        );
        let with_push = |stage: ShaderStage| ShaderReflection {
            stages: vec![stage],
            bindings: vec![],
            push_constant: Some(block("TracePush", 112)),
        };
        let code = generate_shader_bindings_rust(&manifest, |file| match file {
            "traceRgen.spv" => Some(with_push(ShaderStage::RayGeneration)),
            "traceRmiss.spv" => Some(ShaderReflection {
                stages: vec![ShaderStage::Miss],
                bindings: vec![],
                push_constant: None,
            }),
            "sceneRchit.spv" => Some(with_push(ShaderStage::ClosestHit)),
            _ => None,
        })
        .unwrap();
        assert!(code.contains("pub const PUSH_CONSTANT: thyllore_spirv_reflect::PushConstantLayout = thyllore_spirv_reflect::PushConstantLayout { block: \"TracePush\", stages: &[thyllore_spirv_reflect::ShaderStage::RayGeneration, thyllore_spirv_reflect::ShaderStage::ClosestHit], size: 112 };"));
    }

    #[test]
    fn rejects_stages_declaring_different_push_constant_blocks() {
        let manifest = manifest(
            "[pass.effect_trace]\nstages = [\"traceRayGen.slang\", \"sceneClosestHit.slang\"]\nsets = { 0 = \"local\" }\n",
            &[
                ("traceRayGen.slang", StageKind::RayGeneration),
                ("sceneClosestHit.slang", StageKind::ClosestHit),
            ],
        );
        let error = generate_shader_bindings_rust(&manifest, |file| match file {
            "traceRgen.spv" => Some(ShaderReflection {
                stages: vec![ShaderStage::RayGeneration],
                bindings: vec![],
                push_constant: Some(block("TraceCamera", 80)),
            }),
            "sceneRchit.spv" => Some(ShaderReflection {
                stages: vec![ShaderStage::ClosestHit],
                bindings: vec![],
                push_constant: Some(block("TraceLight", 32)),
            }),
            _ => None,
        })
        .unwrap_err();
        assert!(matches!(
            error,
            BindingCodegenError::PushConstantDiffers { .. }
        ));
    }
}
