use crate::log;
use fbxcel::low::FbxHeader;
use fbxcel::pull_parser::v7400::Parser;
use fbxcel_dom::any::AnyDocument;
use imgui::ColorFormat::U8;
use std::io::{Cursor, Read, SeekFrom};
use assimp::Importer;

pub unsafe fn load_fbx(path: &str) -> anyhow::Result<()> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let mut cursor = Cursor::new(buffer);
    let header = FbxHeader::load(cursor.by_ref())?;
    let parser = Parser::from_seekable_reader(header, &mut cursor);
    let doc = AnyDocument::from_seekable_reader(cursor);
    log!("FBX Document: {:?}", doc?.fbx_version());
    Ok(())
}
