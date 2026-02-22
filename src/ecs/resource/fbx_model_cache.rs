use crate::loader::fbx::fbx::FbxModel;

pub struct FbxModelCache {
    pub fbx_model: Option<FbxModel>,
    pub source_path: Option<String>,
}

impl FbxModelCache {
    pub fn new(fbx_model: FbxModel, source_path: String) -> Self {
        Self {
            fbx_model: Some(fbx_model),
            source_path: Some(source_path),
        }
    }

    pub fn empty() -> Self {
        Self {
            fbx_model: None,
            source_path: None,
        }
    }

    pub fn clear(&mut self) {
        self.fbx_model = None;
        self.source_path = None;
    }
}
