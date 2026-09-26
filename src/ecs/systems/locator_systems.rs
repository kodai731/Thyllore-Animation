use crate::ecs::component::Locator;
use crate::ecs::world::{Transform, World};
use crate::ecs::FrameContext;

/// Keeps the persisted placement equal to the Transform the gizmo and the inspector edit.
pub fn follow_locator_transforms(world: &mut World) {
    for entity in world.entities_with::<Locator>() {
        let Some((translation, rotation)) = world
            .get_component::<Transform>(entity)
            .map(|transform| (transform.translation, transform.rotation))
        else {
            continue;
        };
        if let Some(locator) = world.get_component_mut::<Locator>(entity) {
            locator.position = translation.into();
            locator.rotation = [rotation.s, rotation.v.x, rotation.v.y, rotation.v.z];
        }
    }
}

fn locator_advance(ctx: &mut FrameContext) {
    follow_locator_transforms(ctx.world);
}

crate::frame_prep_hook!("locator", Advance, locator_advance);
