use crate::loader::fbx::fbx::FbxModel;

pub struct FbxModelCache {
    pub fbx_model: Option<FbxModel>,
    pub source_path: Option<String>,
    pub needs_coord_conversion: bool,
}

impl FbxModelCache {
    pub fn new(fbx_model: FbxModel, source_path: String, needs_coord_conversion: bool) -> Self {
        Self {
            fbx_model: Some(fbx_model),
            source_path: Some(source_path),
            needs_coord_conversion,
        }
    }

    pub fn empty() -> Self {
        Self {
            fbx_model: None,
            source_path: None,
            needs_coord_conversion: false,
        }
    }

    pub fn clear(&mut self) {
        self.fbx_model = None;
        self.source_path = None;
    }
}
