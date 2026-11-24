/// What the loader knows about a model entity beyond whether it has mesh children.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelLoadMark {
    /// The file is read again although the entity has its meshes.
    ReloadRequested,
    /// The file did not load; it is not requested again and the entity stays in the scene.
    Failed,
}
