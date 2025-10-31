use crate::log;
use fbxcel::low::FbxHeader;
use fbxcel::pull_parser::v7400::Parser;
use fbxcel_dom::any::AnyDocument;
use imgui::ColorFormat::U8;
use std::io::{Cursor, Read, SeekFrom};

pub unsafe fn load_fbx(path: &str) -> anyhow::Result<()> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    match AnyDocument::from_reader(reader).expect("failed to load FBX document") {
        AnyDocument::V7400(fbx_ver, doc) => {
            log!("FBX version: {:?}", fbx_ver);
            log!("root name {:?}", doc.scenes().last().unwrap().name());
        }
        _ => log!("unsupported FBX version"),
    }
    Ok(())
}
