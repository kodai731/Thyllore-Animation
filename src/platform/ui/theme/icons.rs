use crate::ecs::component::EntityIcon;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Play,
    Pause,
    Stop,
    Repeat,
    Box,
    Bone,
    Camera,
    Move,
    Rotate,
    Scale,
    Globe,
    Magnet,
    FolderOpen,
    Image,
    Eye,
    EyeOff,
    Sun,
    Layers,
    Search,
    Close,
    ChevronRight,
    ChevronDown,
    File,
    CircleDot,
    Zap,
    Flame,
    Wind,
    Waves,
    Sparkles,
    Lightbulb,
    User,
    Spline,
    Undo,
    Redo,
    Save,
    Plus,
    Minus,
    Settings,
    Keyboard,
    Command,
    SkipBack,
    SkipForward,
}

impl Icon {
    pub const fn codepoint(self) -> char {
        match self {
            Icon::Play => '\u{e13c}',
            Icon::Pause => '\u{e12e}',
            Icon::Stop => '\u{e167}',
            Icon::Repeat => '\u{e146}',
            Icon::Box => '\u{e061}',
            Icon::Bone => '\u{e358}',
            Icon::Camera => '\u{e064}',
            Icon::Move => '\u{e121}',
            Icon::Rotate => '\u{e2ea}',
            Icon::Scale => '\u{e2ec}',
            Icon::Globe => '\u{e0e8}',
            Icon::Magnet => '\u{e2b5}',
            Icon::FolderOpen => '\u{e247}',
            Icon::Image => '\u{e0f6}',
            Icon::Eye => '\u{e0ba}',
            Icon::EyeOff => '\u{e0bb}',
            Icon::Sun => '\u{e178}',
            Icon::Layers => '\u{e529}',
            Icon::Search => '\u{e151}',
            Icon::Close => '\u{e1b2}',
            Icon::ChevronRight => '\u{e06f}',
            Icon::ChevronDown => '\u{e06d}',
            Icon::File => '\u{e0c0}',
            Icon::CircleDot => '\u{e345}',
            Icon::Zap => '\u{e1b4}',
            Icon::Flame => '\u{e0d2}',
            Icon::Wind => '\u{e1b0}',
            Icon::Waves => '\u{e283}',
            Icon::Sparkles => '\u{e412}',
            Icon::Lightbulb => '\u{e1c2}',
            Icon::User => '\u{e19f}',
            Icon::Spline => '\u{e38b}',
            Icon::Undo => '\u{e2a1}',
            Icon::Redo => '\u{e2a0}',
            Icon::Save => '\u{e14d}',
            Icon::Plus => '\u{e13d}',
            Icon::Minus => '\u{e11c}',
            Icon::Settings => '\u{e154}',
            Icon::Keyboard => '\u{e284}',
            Icon::Command => '\u{e09a}',
            Icon::SkipBack => '\u{e15f}',
            Icon::SkipForward => '\u{e160}',
        }
    }

    pub fn glyph(self) -> String {
        self.codepoint().to_string()
    }
}

pub fn entity_icon(icon: EntityIcon) -> Option<Icon> {
    match icon {
        EntityIcon::Model => Some(Icon::Box),
        EntityIcon::Mesh => Some(Icon::Box),
        EntityIcon::Light => Some(Icon::Lightbulb),
        EntityIcon::Camera => Some(Icon::Camera),
        EntityIcon::Grid => Some(Icon::Layers),
        EntityIcon::Gizmo => Some(Icon::Move),
        EntityIcon::Billboard => Some(Icon::Image),
        EntityIcon::Effect('F') => Some(Icon::Flame),
        EntityIcon::Effect('W') => Some(Icon::Waves),
        EntityIcon::Effect('T') => Some(Icon::Wind),
        EntityIcon::Effect('Z') => Some(Icon::Zap),
        EntityIcon::Effect(_) => Some(Icon::Sparkles),
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
    fn test_entity_icon_effect_flame() {
        assert_eq!(entity_icon(EntityIcon::Effect('F')), Some(Icon::Flame));
    }

    #[test]
    fn test_entity_icon_effect_water() {
        assert_eq!(entity_icon(EntityIcon::Effect('W')), Some(Icon::Waves));
    }

    #[test]
    fn test_entity_icon_effect_wind() {
        assert_eq!(entity_icon(EntityIcon::Effect('T')), Some(Icon::Wind));
    }

    #[test]
    fn test_entity_icon_effect_zap() {
        assert_eq!(entity_icon(EntityIcon::Effect('Z')), Some(Icon::Zap));
    }

    #[test]
    fn test_entity_icon_effect_other() {
        assert_eq!(entity_icon(EntityIcon::Effect('X')), Some(Icon::Sparkles));
    }

    #[test]
    fn test_entity_icon_empty() {
        assert_eq!(entity_icon(EntityIcon::Empty), None);
    }

    #[test]
    fn test_icon_codepoint_play() {
        assert_eq!(Icon::Play.codepoint(), '\u{e13c}');
    }

    #[test]
    fn test_icon_glyph_is_string() {
        let glyph = Icon::Play.glyph();
        assert_eq!(glyph, "\u{e13c}");
    }
}
