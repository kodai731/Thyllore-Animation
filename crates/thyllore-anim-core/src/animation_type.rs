#[derive(Clone, Debug, PartialEq)]
pub enum AnimationType {
    None,
    Skeletal,
    Node,
}

impl Default for AnimationType {
    fn default() -> Self {
        Self::None
    }
}

/// What an imported model brings that decides how it animates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImportedAnimationTraits {
    pub has_skinned_meshes: bool,
    pub has_clips: bool,
}

/// How an imported model animates: skinned meshes play skeletal clips, otherwise clips move nodes.
pub fn classify_imported_animation(traits: ImportedAnimationTraits) -> AnimationType {
    match (traits.has_skinned_meshes, traits.has_clips) {
        (true, _) => AnimationType::Skeletal,
        (false, true) => AnimationType::Node,
        (false, false) => AnimationType::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classify(has_skinned_meshes: bool, has_clips: bool) -> AnimationType {
        classify_imported_animation(ImportedAnimationTraits {
            has_skinned_meshes,
            has_clips,
        })
    }

    #[test]
    fn skinning_decides_skeletal_before_clips_decide_node() {
        assert_eq!(classify(true, false), AnimationType::Skeletal);
        assert_eq!(classify(true, true), AnimationType::Skeletal);
        assert_eq!(classify(false, true), AnimationType::Node);
        assert_eq!(classify(false, false), AnimationType::None);
    }
}
