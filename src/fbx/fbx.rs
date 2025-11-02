/*
reference from bevy_mod_fbx
https://github.com/FizzWizZleDazzle/bevy_mod_fbx/blob/main/src/loader.rs#L217
 */
use crate::log;
use crate::math::math::*;
use anyhow::{anyhow, Result};
use cgmath::{Matrix4, Quaternion};
use fbxcel::tree::v7400::NodeHandle;
use fbxcel_dom::any::AnyDocument;
use fbxcel_dom::v7400::data::{mesh::layer::TypedLayerElementHandle, texture::WrapMode};
use fbxcel_dom::v7400::object::property::loaders::StrictF64Loader;
use fbxcel_dom::v7400::object::{
    model::{ModelHandle, TypedModelHandle},
    ObjectHandle, TypedObjectHandle,
};
use std::ptr::{null, null_mut};

pub unsafe fn load_fbx(path: &str) -> anyhow::Result<()> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    match AnyDocument::from_reader(reader).expect("failed to load FBX document") {
        AnyDocument::V7400(fbx_ver, doc) => {
            for object in doc.objects() {
                if let TypedObjectHandle::Model(TypedModelHandle::Mesh(mesh)) = object.get_typed() {
                    log!("Loading mesh {:?}", mesh);
                    let mut fbx_data = FbxData::new(object.node());
                    fbx_data.name = mesh.name().expect("mesh name not found").to_string();
                    log!("mesh node name {}", fbx_data.name);
                }
            }
        }
        _ => log!("unsupported FBX version"),
    }
    Ok(())
}

#[derive(Clone, Debug, Default)]
pub struct FbxModel<'a> {
    fbx_data: Vec<FbxData<'a>>,
}

#[derive(Clone, Debug)]
struct FbxData<'a> {
    pub name: String,
    pub mesh_node_handle: NodeHandle<'a>,
}

impl<'a> FbxData<'a> {
    pub fn new(node_handle: NodeHandle<'a>) -> Self {
        Self {
            name: String::new(),
            mesh_node_handle: node_handle,
        }
    }
}
