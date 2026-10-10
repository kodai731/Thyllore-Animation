use crate::animation::editable::{
    clip_add_keyframe, clip_recalculate_duration, EditableAnimationClip, PropertyType, SourceClip,
    SourceClipId,
};
use crate::ecs::resource::{ClipLibrary, SelectedKeyframe, TimelineState};

pub(crate) fn clip_with_three_keys() -> (TimelineState, ClipLibrary) {
    let clip_id: SourceClipId = 1;
    let mut clip = EditableAnimationClip::new(clip_id, "test".to_string());
    let bone_id = 0;
    clip.add_track(bone_id, "bone0".to_string());

    let kf1_id =
        clip_add_keyframe(&mut clip, bone_id, PropertyType::TranslationX, 0.5, 1.0).unwrap();
    let kf2_id =
        clip_add_keyframe(&mut clip, bone_id, PropertyType::TranslationX, 1.0, 2.0).unwrap();
    let kf3_id =
        clip_add_keyframe(&mut clip, bone_id, PropertyType::TranslationY, 0.8, 3.0).unwrap();
    clip_recalculate_duration(&mut clip);

    let mut library = ClipLibrary::new();
    library
        .source_clips
        .insert(clip_id, SourceClip::new(clip_id, clip));

    let mut state = TimelineState::new();
    state.current_clip_id = Some(clip_id);
    state.selected_keyframes.insert(SelectedKeyframe::for_bone(
        bone_id,
        PropertyType::TranslationX,
        kf1_id,
    ));
    state.selected_keyframes.insert(SelectedKeyframe::for_bone(
        bone_id,
        PropertyType::TranslationX,
        kf2_id,
    ));
    state.selected_keyframes.insert(SelectedKeyframe::for_bone(
        bone_id,
        PropertyType::TranslationY,
        kf3_id,
    ));

    (state, library)
}
