use imgui::MouseButton;

use crate::ecs::resource::{ActiveSplitter, PanelLayout};
use crate::ecs::systems::panel_layout_systems::panel_layout_clamp_to_display;

use super::layout_snapshot::LayoutSnapshot;

const SPLITTER_THICKNESS: f32 = 6.0;
const SPLITTER_VISUAL_THICKNESS: f32 = 2.0;

const COLOR_IDLE: [f32; 4] = [0.3, 0.3, 0.3, 0.8];
const COLOR_HOVERED: [f32; 4] = [0.6, 0.6, 0.6, 1.0];
const COLOR_ACTIVE: [f32; 4] = [0.8, 0.8, 0.2, 1.0];

struct SplitterRect {
    kind: ActiveSplitter,
    x_min: f32,
    y_min: f32,
    x_max: f32,
    y_max: f32,
    is_horizontal: bool,
}

pub fn handle_splitters(ui: &imgui::Ui, layout: &mut PanelLayout, snap: &LayoutSnapshot) {
    let rects = compute_splitter_rects(snap);
    let mouse_pos = ui.io().mouse_pos;
    let mouse_down = ui.is_mouse_down(MouseButton::Left);
    let mouse_clicked = ui.is_mouse_clicked(MouseButton::Left);
    let mouse_released = ui.is_mouse_released(MouseButton::Left);
    let mouse_double_clicked = ui.is_mouse_double_clicked(MouseButton::Left);

    let hovered_splitter = rects.iter().find(|r| hit_test(r, mouse_pos));

    if mouse_double_clicked {
        if let Some(rect) = &hovered_splitter {
            reset_to_default(layout, rect.kind);
            layout.active_splitter = None;
            return;
        }
    }

    if mouse_clicked && layout.active_splitter.is_none() {
        if let Some(rect) = &hovered_splitter {
            layout.active_splitter = Some(rect.kind);
            layout.drag_start_pos = if rect.is_horizontal {
                mouse_pos[1]
            } else {
                mouse_pos[0]
            };
            layout.drag_start_value = current_value(layout, rect.kind);
        }
    }

    if mouse_down {
        if let Some(splitter) = layout.active_splitter {
            let is_horiz = matches!(splitter, ActiveSplitter::Upper | ActiveSplitter::Lower);
            let current_pos = if is_horiz { mouse_pos[1] } else { mouse_pos[0] };
            let delta = current_pos - layout.drag_start_pos;
            apply_drag(layout, splitter, delta);
            panel_layout_clamp_to_display(layout, snap.display_size[0], snap.display_size[1]);
        }
    }

    if mouse_released {
        layout.active_splitter = None;
    }

    if layout.active_splitter.is_some() {
        let is_horiz = matches!(
            layout.active_splitter,
            Some(ActiveSplitter::Upper) | Some(ActiveSplitter::Lower)
        );
        if is_horiz {
            ui.set_mouse_cursor(Some(imgui::MouseCursor::ResizeNS));
        } else {
            ui.set_mouse_cursor(Some(imgui::MouseCursor::ResizeEW));
        }
    } else if let Some(rect) = &hovered_splitter {
        if rect.is_horizontal {
            ui.set_mouse_cursor(Some(imgui::MouseCursor::ResizeNS));
        } else {
            ui.set_mouse_cursor(Some(imgui::MouseCursor::ResizeEW));
        }
    }

    let draw_list = ui.get_foreground_draw_list();
    for rect in &rects {
        let color = if layout.active_splitter == Some(rect.kind) {
            COLOR_ACTIVE
        } else if hovered_splitter
            .as_ref()
            .map_or(false, |h| h.kind == rect.kind)
        {
            COLOR_HOVERED
        } else {
            COLOR_IDLE
        };

        if rect.is_horizontal {
            let center_y = (rect.y_min + rect.y_max) * 0.5;
            draw_list
                .add_line([rect.x_min, center_y], [rect.x_max, center_y], color)
                .thickness(SPLITTER_VISUAL_THICKNESS)
                .build();
        } else {
            let center_x = (rect.x_min + rect.x_max) * 0.5;
            draw_list
                .add_line([center_x, rect.y_min], [center_x, rect.y_max], color)
                .thickness(SPLITTER_VISUAL_THICKNESS)
                .build();
        }
    }
}

fn compute_splitter_rects(snap: &LayoutSnapshot) -> [SplitterRect; 4] {
    let half = SPLITTER_THICKNESS * 0.5;
    [
        SplitterRect {
            kind: ActiveSplitter::Left,
            x_min: snap.hierarchy_width - half,
            y_min: 0.0,
            x_max: snap.hierarchy_width + half,
            y_max: snap.main_height,
            is_horizontal: false,
        },
        SplitterRect {
            kind: ActiveSplitter::Right,
            x_min: snap.inspector_x - half,
            y_min: 0.0,
            x_max: snap.inspector_x + half,
            y_max: snap.main_height,
            is_horizontal: false,
        },
        SplitterRect {
            kind: ActiveSplitter::Upper,
            x_min: 0.0,
            y_min: snap.timeline_y - half,
            x_max: snap.display_size[0],
            y_max: snap.timeline_y + half,
            is_horizontal: true,
        },
        SplitterRect {
            kind: ActiveSplitter::Lower,
            x_min: 0.0,
            y_min: snap.debug_y - half,
            x_max: snap.display_size[0],
            y_max: snap.debug_y + half,
            is_horizontal: true,
        },
    ]
}

fn hit_test(rect: &SplitterRect, mouse_pos: [f32; 2]) -> bool {
    mouse_pos[0] >= rect.x_min
        && mouse_pos[0] <= rect.x_max
        && mouse_pos[1] >= rect.y_min
        && mouse_pos[1] <= rect.y_max
}

fn current_value(layout: &PanelLayout, splitter: ActiveSplitter) -> f32 {
    match splitter {
        ActiveSplitter::Left => layout.hierarchy_width,
        ActiveSplitter::Right => layout.inspector_width,
        ActiveSplitter::Upper => layout.timeline_height,
        ActiveSplitter::Lower => layout.debug_height,
    }
}

fn apply_drag(layout: &mut PanelLayout, splitter: ActiveSplitter, delta: f32) {
    let new_value = layout.drag_start_value
        + match splitter {
            ActiveSplitter::Left => delta,
            ActiveSplitter::Right => -delta,
            ActiveSplitter::Upper => -delta,
            ActiveSplitter::Lower => -delta,
        };

    match splitter {
        ActiveSplitter::Left => layout.hierarchy_width = new_value,
        ActiveSplitter::Right => layout.inspector_width = new_value,
        ActiveSplitter::Upper => layout.timeline_height = new_value,
        ActiveSplitter::Lower => layout.debug_height = new_value,
    }
}

fn reset_to_default(layout: &mut PanelLayout, splitter: ActiveSplitter) {
    let default = PanelLayout::default_value_for(splitter);
    match splitter {
        ActiveSplitter::Left => layout.hierarchy_width = default,
        ActiveSplitter::Right => layout.inspector_width = default,
        ActiveSplitter::Upper => layout.timeline_height = default,
        ActiveSplitter::Lower => layout.debug_height = default,
    }
}
