/// The GLB a generated model was built from; a model loaded from a file has a `ModelSource` instead.
#[derive(Clone)]
pub struct GeneratedGlb {
    pub bytes: Vec<u8>,
}
