use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;

use anyhow::Result;
use vulkanalia::vk;

use crate::debugview::gizmo::{GridGizmoData, LightGizmoData};
use crate::scene::billboard::BillboardData;
use crate::ecs::{RenderContext, Renderable, Updatable, UpdateContext};
use crate::scene::grid::GridData;
use crate::scene::graphics_resource::ObjectDescriptorSet;
use crate::vulkanr::device::RRDevice;

pub trait SceneObject: Updatable + Renderable + Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn type_id(&self) -> TypeId;
}

impl<T: Updatable + Renderable + Any> SceneObject for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

pub struct Scene {
    objects: Vec<RefCell<Box<dyn SceneObject>>>,
    type_index: HashMap<TypeId, usize>,
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            type_index: HashMap::new(),
        }
    }

    pub fn add<T: SceneObject + 'static>(&mut self, object: T) {
        let type_id = TypeId::of::<T>();
        let index = self.objects.len();
        self.objects.push(RefCell::new(Box::new(object)));
        self.type_index.insert(type_id, index);
    }

    pub fn update_all(&mut self, ctx: &UpdateContext) {
        for obj in &self.objects {
            obj.borrow_mut().update(ctx);
        }
    }

    pub unsafe fn update_object_ubos(
        &self,
        ctx: &RenderContext,
        objects: &ObjectDescriptorSet,
        rrdevice: &RRDevice,
    ) -> Result<()> {
        use crate::scene::graphics_resource::ObjectUBO;

        for obj_cell in &self.objects {
            let obj = obj_cell.borrow();
            let ubo = ObjectUBO {
                model: obj.model_matrix(ctx),
            };
            objects.update(rrdevice, ctx.image_index, obj.object_index(), &ubo)?;
        }
        Ok(())
    }

    pub unsafe fn render_all(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
        frame_set: vk::DescriptorSet,
        objects: &ObjectDescriptorSet,
        rrdevice: &RRDevice,
    ) {
        use crate::vulkanr::vulkan::*;

        for obj_cell in &self.objects {
            let obj = obj_cell.borrow();

            let vertex_buffer = obj.vertex_buffer();
            let index_buffer = obj.index_buffer();

            if vertex_buffer == vk::Buffer::null() || index_buffer == vk::Buffer::null() {
                continue;
            }

            rrdevice.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                obj.pipeline().pipeline,
            );

            rrdevice.device.cmd_set_line_width(command_buffer, 1.0);

            rrdevice
                .device
                .cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);

            rrdevice.device.cmd_bind_index_buffer(
                command_buffer,
                index_buffer,
                0,
                vk::IndexType::UINT32,
            );

            rrdevice.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                obj.pipeline().pipeline_layout,
                0,
                &[frame_set],
                &[],
            );

            let object_set_idx = objects.get_set_index(image_index, obj.object_index());
            let object_set = objects.sets[object_set_idx];
            rrdevice.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                obj.pipeline().pipeline_layout,
                2,
                &[object_set],
                &[],
            );

            rrdevice
                .device
                .cmd_draw_indexed(command_buffer, obj.index_count(), 1, 0, 0, 0);
        }
    }

    fn get_cell<T: 'static>(&self) -> Option<&RefCell<Box<dyn SceneObject>>> {
        let type_id = TypeId::of::<T>();
        self.type_index.get(&type_id).map(|&idx| &self.objects[idx])
    }

    pub fn grid(&self) -> Ref<GridData> {
        let cell = self
            .get_cell::<GridData>()
            .expect("GridData not found in scene");
        Ref::map(cell.borrow(), |obj| {
            obj.as_any().downcast_ref::<GridData>().unwrap()
        })
    }

    pub fn grid_mut(&self) -> RefMut<GridData> {
        let cell = self
            .get_cell::<GridData>()
            .expect("GridData not found in scene");
        RefMut::map(cell.borrow_mut(), |obj| {
            obj.as_any_mut().downcast_mut::<GridData>().unwrap()
        })
    }

    pub fn gizmo(&self) -> Ref<GridGizmoData> {
        let cell = self
            .get_cell::<GridGizmoData>()
            .expect("GridGizmoData not found in scene");
        Ref::map(cell.borrow(), |obj| {
            obj.as_any().downcast_ref::<GridGizmoData>().unwrap()
        })
    }

    pub fn gizmo_mut(&self) -> RefMut<GridGizmoData> {
        let cell = self
            .get_cell::<GridGizmoData>()
            .expect("GridGizmoData not found in scene");
        RefMut::map(cell.borrow_mut(), |obj| {
            obj.as_any_mut().downcast_mut::<GridGizmoData>().unwrap()
        })
    }

    pub fn light_gizmo(&self) -> Ref<LightGizmoData> {
        let cell = self
            .get_cell::<LightGizmoData>()
            .expect("LightGizmoData not found in scene");
        Ref::map(cell.borrow(), |obj| {
            obj.as_any().downcast_ref::<LightGizmoData>().unwrap()
        })
    }

    pub fn light_gizmo_mut(&self) -> RefMut<LightGizmoData> {
        let cell = self
            .get_cell::<LightGizmoData>()
            .expect("LightGizmoData not found in scene");
        RefMut::map(cell.borrow_mut(), |obj| {
            obj.as_any_mut().downcast_mut::<LightGizmoData>().unwrap()
        })
    }

    pub fn billboard(&self) -> Ref<BillboardData> {
        let cell = self
            .get_cell::<BillboardData>()
            .expect("BillboardData not found in scene");
        Ref::map(cell.borrow(), |obj| {
            obj.as_any().downcast_ref::<BillboardData>().unwrap()
        })
    }

    pub fn billboard_mut(&self) -> RefMut<BillboardData> {
        let cell = self
            .get_cell::<BillboardData>()
            .expect("BillboardData not found in scene");
        RefMut::map(cell.borrow_mut(), |obj| {
            obj.as_any_mut().downcast_mut::<BillboardData>().unwrap()
        })
    }
}
