#[derive(Clone, Debug)]
pub enum DialogRequest {
    PickMaterialTexture { material: String },
}
