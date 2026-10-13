use crate::ecs::resource::PostProcessPreview;
use crate::ecs::world::World;

pub fn current_post_process_preview(world: &World) -> PostProcessPreview {
    world
        .get_resource::<PostProcessPreview>()
        .map(|preview| *preview)
        .unwrap_or_default()
}

pub fn post_process_preview_enabled(world: &World) -> bool {
    current_post_process_preview(world) == PostProcessPreview::Enabled
}
