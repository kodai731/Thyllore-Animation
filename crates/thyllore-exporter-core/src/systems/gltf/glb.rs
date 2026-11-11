use std::borrow::Cow;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Result};
use gltf::binary::Glb;
use gltf::json;

pub(crate) fn glb_bytes(root: &json::Root, bin: Vec<u8>) -> Result<Vec<u8>> {
    let json_bytes = root
        .to_vec()
        .map_err(|e| anyhow!("Failed to serialize glTF JSON: {:?}", e))?;

    let output_glb = Glb {
        header: gltf::binary::Header {
            magic: *b"glTF",
            version: 2,
            length: 0,
        },
        json: Cow::Owned(json_bytes),
        bin: if bin.is_empty() {
            None
        } else {
            Some(Cow::Owned(bin))
        },
    };

    output_glb
        .to_vec()
        .map_err(|e| anyhow!("Failed to encode GLB: {:?}", e))
}

pub(crate) fn write_glb(root: &json::Root, bin: Vec<u8>, output_path: &Path) -> Result<()> {
    fs::write(output_path, glb_bytes(root, bin)?)?;
    Ok(())
}
