use std::fs;
use std::path::Path;

use anyhow::{anyhow, Result};
use gltf::binary::Glb;
use gltf::json::{self};

use thyllore_anim_core::editable::{clip_to_animation, EditableAnimationClip};
use thyllore_anim_core::Skeleton;

use crate::systems::gltf::channels::{replace_animations, write_animation_channels};
use crate::systems::gltf::glb::write_glb;
use crate::systems::gltf::minimal_json::build_minimal_gltf_json;

pub fn export_gltf_animation(
    source_glb_path: &Path,
    clip: &EditableAnimationClip,
    skeleton: &Skeleton,
    output_path: &Path,
) -> Result<()> {
    let raw_bytes = fs::read(source_glb_path)?;
    export_gltf_animation_from_bytes(&raw_bytes, clip, skeleton, output_path)
}

pub fn export_gltf_animation_from_bytes(
    source_glb_bytes: &[u8],
    clip: &EditableAnimationClip,
    skeleton: &Skeleton,
    output_path: &Path,
) -> Result<()> {
    let glb =
        Glb::from_slice(source_glb_bytes).map_err(|e| anyhow!("Failed to parse GLB: {:?}", e))?;

    let mut root: json::Root = json::Root::from_slice(&glb.json)
        .map_err(|e| anyhow!("Failed to parse glTF JSON: {:?}", e))?;

    let mut bin = glb.bin.map(|b| b.into_owned()).unwrap_or_default();

    let baked_clip = clip_to_animation(clip);

    replace_animations(&mut root, &mut bin, &baked_clip, skeleton)?;

    write_glb(&root, bin, output_path)?;

    log!("glTF animation exported to {:?}", output_path);
    Ok(())
}

pub fn export_gltf_animation_only(
    clip: &EditableAnimationClip,
    skeleton: &Skeleton,
    output_path: &Path,
) -> anyhow::Result<()> {
    let baked_clip = clip_to_animation(clip);
    let mut root = build_minimal_gltf_json(skeleton)?;
    let mut bin = Vec::new();

    write_animation_channels(&mut root, &mut bin, &baked_clip, skeleton)?;

    write_glb(&root, bin, output_path)?;

    log!("Animation-only glTF exported to {:?}", output_path);
    Ok(())
}
