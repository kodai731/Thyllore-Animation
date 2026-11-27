use crate::ecs::component::{EntityIcon, Icon};

pub(crate) fn entity_icon(icon: EntityIcon) -> Option<Icon> {
    match icon {
        EntityIcon::Model => Some(Icon::Box),
        EntityIcon::Mesh => Some(Icon::Box),
        EntityIcon::Light => Some(Icon::Lightbulb),
        EntityIcon::Camera => Some(Icon::Camera),
        EntityIcon::Grid => Some(Icon::Layers),
        EntityIcon::Gizmo => Some(Icon::Move),
        EntityIcon::Billboard => Some(Icon::Image),
        EntityIcon::Effect(icon) => Some(icon),
        EntityIcon::Empty => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_icon_model() {
        assert_eq!(entity_icon(EntityIcon::Model), Some(Icon::Box));
    }

    #[test]
    fn test_entity_icon_mesh() {
        assert_eq!(entity_icon(EntityIcon::Mesh), Some(Icon::Box));
    }

    #[test]
    fn test_entity_icon_light() {
        assert_eq!(entity_icon(EntityIcon::Light), Some(Icon::Lightbulb));
    }

    #[test]
    fn test_entity_icon_camera() {
        assert_eq!(entity_icon(EntityIcon::Camera), Some(Icon::Camera));
    }

    #[test]
    fn test_entity_icon_grid() {
        assert_eq!(entity_icon(EntityIcon::Grid), Some(Icon::Layers));
    }

    #[test]
    fn test_entity_icon_gizmo() {
        assert_eq!(entity_icon(EntityIcon::Gizmo), Some(Icon::Move));
    }

    #[test]
    fn test_entity_icon_billboard() {
        assert_eq!(entity_icon(EntityIcon::Billboard), Some(Icon::Image));
    }

    #[test]
    fn test_entity_icon_effect_shows_the_icon_it_declares() {
        assert_eq!(
            entity_icon(EntityIcon::Effect(Icon::Flame)),
            Some(Icon::Flame)
        );
    }

    #[test]
    fn test_entity_icon_empty() {
        assert_eq!(entity_icon(EntityIcon::Empty), None);
    }
}
