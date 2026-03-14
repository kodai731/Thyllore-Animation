use std::collections::HashSet;

use crate::animation::editable::{BezierHandle, KeyframeId, PropertyType};
use crate::animation::BoneId;

#[derive(Clone, Debug)]
pub struct CurveSelectedKeyframe {
    pub property_type: PropertyType,
    pub keyframe_id: KeyframeId,
    pub original_time: f32,
    pub original_value: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TangentHandleType {
    In,
    Out,
}

#[derive(Clone, Debug)]
pub struct DraggingTangent {
    pub property_type: PropertyType,
    pub keyframe_id: KeyframeId,
    pub handle_type: TangentHandleType,
    pub original_handle: BezierHandle,
}

pub struct CurveEditorState {
    pub is_open: bool,
    pub selected_bone_id: Option<BoneId>,
    pub visible_curves: HashSet<PropertyType>,
    pub window_size: [f32; 2],
    pub selected_keyframes: Vec<CurveSelectedKeyframe>,
    pub selection_anchor: Option<(PropertyType, KeyframeId)>,
    pub is_dragging_keyframe: bool,
    pub drag_start_mouse_pos: [f32; 2],
    pub zoom_x: f32,
    pub zoom_y: f32,
    pub view_time_offset: f32,
    pub view_value_offset: f32,
    pub view_val_range: f32,
    pub view_duration: f32,
    pub view_initialized: bool,
    pub is_scrubbing_ruler: bool,
    pub is_panning: bool,
    pub pan_start_mouse_pos: [f32; 2],
    pub pan_start_offset: [f32; 2],
    pub dragging_tangent: Option<DraggingTangent>,
    pub context_menu_keyframe: Option<CurveSelectedKeyframe>,
    pub context_menu_click_time: f32,
    pub context_menu_click_value: f32,
    pub needs_focus: bool,
}

impl Default for CurveEditorState {
    fn default() -> Self {
        let mut visible_curves = HashSet::new();
        visible_curves.insert(PropertyType::TranslationX);
        visible_curves.insert(PropertyType::TranslationY);
        visible_curves.insert(PropertyType::TranslationZ);
        visible_curves.insert(PropertyType::RotationX);
        visible_curves.insert(PropertyType::RotationY);
        visible_curves.insert(PropertyType::RotationZ);

        Self {
            is_open: false,
            selected_bone_id: None,
            visible_curves,
            window_size: [800.0, 500.0],
            selected_keyframes: Vec::new(),
            selection_anchor: None,
            is_dragging_keyframe: false,
            drag_start_mouse_pos: [0.0, 0.0],
            zoom_x: 1.0,
            zoom_y: 1.0,
            view_time_offset: 0.0,
            view_value_offset: 0.0,
            view_val_range: 2.0,
            view_duration: 2.0,
            view_initialized: false,
            is_scrubbing_ruler: false,
            is_panning: false,
            pan_start_mouse_pos: [0.0, 0.0],
            pan_start_offset: [0.0, 0.0],
            dragging_tangent: None,
            context_menu_keyframe: None,
            context_menu_click_time: 0.0,
            context_menu_click_value: 0.0,
            needs_focus: false,
        }
    }
}
