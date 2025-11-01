use crate::log;
use crate::math::math::*;
use anyhow::{anyhow, Result};
use cgmath::{Matrix4, Quaternion};
use fbxcel_dom::any::AnyDocument;
use fbxcel_dom::v7400::object::TypedObjectHandle;

pub unsafe fn load_fbx(path: &str) -> anyhow::Result<()> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    match AnyDocument::from_reader(reader).expect("failed to load FBX document") {
        AnyDocument::V7400(fbx_ver, doc) => {
            for object in doc.objects() {
                let class = object.class();
                let subclass = object.subclass();
                if class == "Model" && subclass == "Mesh" {
                    log!("Mesh found: {:?}", subclass);
                    if let TypedObjectHandle::Geometry(geometry_handle) = object.get_typed() {
                        log!("Geometry found: {:?}", geometry_handle);
                    }
                    if let TypedObjectHandle::Model(model_handle) = object.get_typed() {
                        log!("Model found: {:?}", model_handle);
                        for model_sources in model_handle.source_objects() {
                            if let Some(source_handle) = model_sources.object_handle() {
                                if let TypedObjectHandle::Geometry(geometry_handle) =
                                    source_handle.get_typed()
                                {
                                    log!("Geometry found: {:?}", geometry_handle);
                                }
                            }
                        }
                    }
                    if let TypedObjectHandle::Material(material_handle) = object.get_typed() {
                        log!("Material found: {:?}", material_handle);
                    }
                }
            }
        }
        _ => log!("unsupported FBX version"),
    }
    Ok(())
}

#[derive(Clone, Debug, Default)]
struct FbxData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub texcoords: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}
