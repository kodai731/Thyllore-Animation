#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Side {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MarkerPosition {
    Suffix,
    Prefix,
}

#[derive(Clone, Debug)]
pub struct SideMarker {
    pub left: &'static str,
    pub right: &'static str,
    pub position: MarkerPosition,
}

pub const SIDE_MARKERS: &[SideMarker] = &[
    SideMarker {
        left: "_L",
        right: "_R",
        position: MarkerPosition::Suffix,
    },
    SideMarker {
        left: ".L",
        right: ".R",
        position: MarkerPosition::Suffix,
    },
    SideMarker {
        left: "_l",
        right: "_r",
        position: MarkerPosition::Suffix,
    },
    SideMarker {
        left: "Left_",
        right: "Right_",
        position: MarkerPosition::Prefix,
    },
];
