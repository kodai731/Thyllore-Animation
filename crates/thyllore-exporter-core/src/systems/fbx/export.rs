use std::path::Path;

use fbxcel::low::FbxVersion;
use fbxcel::writer::v7400::binary::Writer;

use thyllore_anim_core::editable::EditableAnimationClip;
use thyllore_anim_core::Skeleton;
use thyllore_file_format_core::fbx::FbxModel;

use crate::systems::fbx::build::build_full_export_data;
use crate::systems::fbx::writer::*;

pub fn export_full_fbx(
    fbx_model: &FbxModel,
    clip: Option<&EditableAnimationClip>,
    skeleton: &Skeleton,
    path: &Path,
) -> anyhow::Result<()> {
    let export_data = build_full_export_data(fbx_model, clip, skeleton, path)?;

    let file = std::fs::File::create(path)?;
    let writer = Writer::new(file, FbxVersion::V7_4)
        .map_err(|e| anyhow::anyhow!("FBX writer init failed: {}", e))?;

    write_full_fbx_binary(writer, &export_data)
        .map_err(|e| anyhow::anyhow!("FBX write failed: {}", e))?;

    Ok(())
}
