use crate::loader::fbx::fbx::FbxModel;

pub enum FbxModelCache {
    Loaded {
        fbx_model: FbxModel,
        source_path: String,
        needs_coord_conversion: bool,
    },
    Empty,
}

impl FbxModelCache {
    pub fn new(fbx_model: FbxModel, source_path: String, needs_coord_conversion: bool) -> Self {
        Self::Loaded {
            fbx_model,
            source_path,
            needs_coord_conversion,
        }
    }

    pub fn empty() -> Self {
        Self::Empty
    }

    pub fn fbx_model(&self) -> Option<&FbxModel> {
        match self {
            Self::Loaded { fbx_model, .. } => Some(fbx_model),
            Self::Empty => None,
        }
    }

    pub fn source_path(&self) -> Option<&str> {
        match self {
            Self::Loaded { source_path, .. } => Some(source_path),
            Self::Empty => None,
        }
    }

    pub fn needs_coord_conversion(&self) -> bool {
        match self {
            Self::Loaded {
                needs_coord_conversion,
                ..
            } => *needs_coord_conversion,
            Self::Empty => false,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::Empty;
    }
}
