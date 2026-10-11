#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum EntityIcon {
    #[default]
    Empty,
    Model,
    Mesh,
    Light,
    Camera,
    Grid,
    Gizmo,
    Billboard,
    Effect(char),
}

#[derive(Clone, Debug, Default)]
pub struct EditorDisplay {
    pub icon: EntityIcon,
    pub expanded: bool,
    pub hidden_in_hierarchy: bool,
}

impl EditorDisplay {
    pub fn new(icon: EntityIcon) -> Self {
        Self {
            icon,
            expanded: false,
            hidden_in_hierarchy: false,
        }
    }

    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }
}
