/// A glyph of the editor's icon font.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    Image,
    Layers,
    FileBox,
    FileBraces,
    CircleDot,
    Zap,
    Flame,
    Wind,
    Waves,
    Lightbulb,
    Spline,
    Undo,
    Plus,
    Minus,
    Settings,
    PictureInPicture,
    Minimize,
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
            Icon::Image => '\u{e0f6}',
            Icon::Layers => '\u{e529}',
            Icon::FileBox => '\u{e310}',
            Icon::FileBraces => '\u{e36b}',
            Icon::CircleDot => '\u{e345}',
            Icon::Zap => '\u{e1b4}',
            Icon::Flame => '\u{e0d2}',
            Icon::Wind => '\u{e1b0}',
            Icon::Waves => '\u{e283}',
            Icon::Lightbulb => '\u{e1c2}',
            Icon::Spline => '\u{e38b}',
            Icon::Undo => '\u{e2a1}',
            Icon::Plus => '\u{e13d}',
            Icon::Minus => '\u{e11c}',
            Icon::Settings => '\u{e154}',
            Icon::PictureInPicture => '\u{e3af}',
            Icon::Minimize => '\u{e11b}',
        }
    }

    pub fn glyph(self) -> String {
        self.codepoint().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
