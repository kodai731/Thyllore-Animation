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

/// How an imported model animates: skinned meshes play skeletal clips, otherwise clips move nodes.
pub fn classify_imported_animation(has_skinned_meshes: bool, has_clips: bool) -> AnimationType {
    if has_skinned_meshes {
        AnimationType::Skeletal
    } else if has_clips {
        AnimationType::Node
    } else {
        AnimationType::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skinning_decides_skeletal_before_clips_decide_node() {
        assert_eq!(
            classify_imported_animation(true, false),
            AnimationType::Skeletal
        );
        assert_eq!(
            classify_imported_animation(true, true),
            AnimationType::Skeletal
        );
        assert_eq!(
            classify_imported_animation(false, true),
            AnimationType::Node
        );
        assert_eq!(
            classify_imported_animation(false, false),
            AnimationType::None
        );
    }
}
