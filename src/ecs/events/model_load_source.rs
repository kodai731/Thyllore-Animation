/// Where a model loaded from memory came from, so the ML pipeline can continue after the load.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelLoadSource {
    UserFile,
    AutoRigOutput,
    TextToMeshOutput,
}
