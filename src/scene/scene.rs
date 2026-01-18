use std::cell::{Ref, RefCell, RefMut};

use cgmath::Vector3;

use crate::debugview::gizmo::{GridGizmoData, LightGizmoData};
use crate::ecs::RenderData;
use crate::scene::billboard::BillboardData;
use crate::scene::grid::GridData;

pub struct Scene {
    pub grid: RefCell<GridData>,
    pub gizmo: RefCell<GridGizmoData>,
    pub light_gizmo: RefCell<LightGizmoData>,
    pub billboard: RefCell<BillboardData>,
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene {
    pub fn new() -> Self {
        Self {
            grid: RefCell::new(GridData::default()),
            gizmo: RefCell::new(GridGizmoData::new()),
            light_gizmo: RefCell::new(LightGizmoData::default()),
            billboard: RefCell::new(BillboardData::new()),
        }
    }

    pub fn collect_render_data(&self, camera_position: Vector3<f32>) -> Vec<RenderData> {
        vec![
            self.grid.borrow().render_data(),
            self.gizmo.borrow().render_data(),
            self.light_gizmo.borrow().render_data(camera_position),
            self.billboard.borrow().render_data(),
        ]
    }

    pub fn grid(&self) -> Ref<GridData> {
        self.grid.borrow()
    }

    pub fn grid_mut(&self) -> RefMut<GridData> {
        self.grid.borrow_mut()
    }

    pub fn gizmo(&self) -> Ref<GridGizmoData> {
        self.gizmo.borrow()
    }

    pub fn gizmo_mut(&self) -> RefMut<GridGizmoData> {
        self.gizmo.borrow_mut()
    }

    pub fn light_gizmo(&self) -> Ref<LightGizmoData> {
        self.light_gizmo.borrow()
    }

    pub fn light_gizmo_mut(&self) -> RefMut<LightGizmoData> {
        self.light_gizmo.borrow_mut()
    }

    pub fn billboard(&self) -> Ref<BillboardData> {
        self.billboard.borrow()
    }

    pub fn billboard_mut(&self) -> RefMut<BillboardData> {
        self.billboard.borrow_mut()
    }
}
