use std::collections::HashMap;

use super::{BoneNode, FbxAnimation, FbxAxesInfo, FbxData, LoadedCamera, LoadedConstraint};

#[derive(Clone, Debug, Default)]
pub struct FbxModel {
    pub fbx_data: Vec<FbxData>,
    pub animations: Vec<FbxAnimation>,
    pub nodes: HashMap<String, BoneNode>,
    pub unit_scale: f32,
    pub fps: f32,
    pub constraints: Vec<LoadedConstraint>,
    pub axes: FbxAxesInfo,
    pub source_path: Option<String>,
    pub cameras: Vec<LoadedCamera>,
}
