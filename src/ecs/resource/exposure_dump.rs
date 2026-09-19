#[derive(Clone)]
pub struct ExposureDumpSink {
    pub path: String,
}

impl ExposureDumpSink {
    pub fn new(path: String) -> Self {
        Self { path }
    }
}
