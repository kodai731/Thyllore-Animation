use crate::ecs::component::{LineMesh, RenderInfo};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpringChainGizmoMode {
    #[default]
    Selected,
    All,
    None,
}

impl SpringChainGizmoMode {
    pub const ALL: [SpringChainGizmoMode; 3] = [
        SpringChainGizmoMode::Selected,
        SpringChainGizmoMode::All,
        SpringChainGizmoMode::None,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            SpringChainGizmoMode::Selected => "Selected",
            SpringChainGizmoMode::All => "All",
            SpringChainGizmoMode::None => "None",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpringBoneGizmoData {
    pub colliders_visible: bool,
    pub chain_mode: SpringChainGizmoMode,
    pub wire_mesh: LineMesh,
    pub wire_render_info: RenderInfo,
}

impl SpringBoneGizmoData {
    pub fn draws_anything(&self) -> bool {
        self.colliders_visible || self.chain_mode != SpringChainGizmoMode::None
    }
}

impl Default for SpringBoneGizmoData {
    fn default() -> Self {
        Self {
            colliders_visible: true,
            chain_mode: SpringChainGizmoMode::default(),
            wire_mesh: LineMesh::default(),
            wire_render_info: RenderInfo::default(),
        }
    }
}
