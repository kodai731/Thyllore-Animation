/// What the loader knows about a model entity beyond whether it has mesh children.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelLoadMark {
    /// The user added the model to the open scene; its entity leaves when the file does not load.
    Added,
    /// The file is read again although the entity has its meshes.
    ReloadRequested,
    /// The file of the scene's own model did not load; it is not requested again and the entity stays.
    Failed,
}
