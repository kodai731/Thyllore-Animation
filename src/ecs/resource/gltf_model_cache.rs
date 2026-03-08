pub struct GltfModelCache {
    pub source_path: Option<String>,
}

impl GltfModelCache {
    pub fn new(source_path: String) -> Self {
        Self {
            source_path: Some(source_path),
        }
    }

    pub fn empty() -> Self {
        Self { source_path: None }
    }

    pub fn clear(&mut self) {
        self.source_path = None;
    }
}
