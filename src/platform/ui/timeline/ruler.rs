use super::*;

pub(super) fn build_time_ruler_with_scrub(
    ui: &imgui::Ui,
    world: &World,
    state: &mut TimelineState,
    interaction: &mut TimelineInteractionState,
    timeline_width: f32,
    display_duration: f32,
) {
    let cursor_pos = ui.cursor_screen_pos();
    let ruler_start_x = cursor_pos[0] + TRACK_LABEL_WIDTH;
    let ruler_width = timeline_width;
    let pixels_per_second = PIXELS_PER_SECOND * state.zoom_level;

    ui.text("Time:");
    ui.same_line_with_pos(TRACK_LABEL_WIDTH);

    let draw_list = ui.get_window_draw_list();

    draw_list
        .add_rect(
            [ruler_start_x, cursor_pos[1]],
            [
                ruler_start_x + ruler_width,
                cursor_pos[1] + TIME_RULER_HEIGHT,
            ],
            [0.2, 0.2, 0.25, 1.0],
        )
        .filled(true)
        .build();

    let tick_interval = calculate_tick_interval(pixels_per_second);
    let mut time = 0.0;
    while time <= display_duration {
        let x = ruler_start_x + time * pixels_per_second;
        let is_major = (time / tick_interval).round() as i32 % 5 == 0;

        let tick_height = if is_major { 12.0 } else { 6.0 };
        let tick_color = if is_major {
            [0.7, 0.7, 0.7, 1.0]
        } else {
            [0.4, 0.4, 0.4, 1.0]
        };

        draw_list
            .add_line(
                [x, cursor_pos[1] + TIME_RULER_HEIGHT - tick_height],
                [x, cursor_pos[1] + TIME_RULER_HEIGHT],
                tick_color,
            )
            .build();

        if is_major {
            draw_list.add_text(
                [x - 10.0, cursor_pos[1] + 2.0],
                [0.8, 0.8, 0.8, 1.0],
                &format!("{:.1}s", time),
            );
        }

        time += tick_interval;
    }

    let playhead_x = ruler_start_x + state.current_time * pixels_per_second;
    draw_playhead_handle(&draw_list, playhead_x, cursor_pos[1], TIME_RULER_HEIGHT);

    let ruler_rect_min = [ruler_start_x, cursor_pos[1]];
    let ruler_rect_max = [
        ruler_start_x + ruler_width,
        cursor_pos[1] + TIME_RULER_HEIGHT,
    ];

    handle_scrub_interaction(
        ui,
        world,
        interaction,
        ruler_rect_min,
        ruler_rect_max,
        display_duration,
        pixels_per_second,
        ruler_start_x,
    );

    ui.dummy([ruler_width + TRACK_LABEL_WIDTH, TIME_RULER_HEIGHT]);
}

pub(super) fn draw_playhead_handle(
    draw_list: &imgui::DrawListMut,
    x: f32,
    y: f32,
    ruler_height: f32,
) {
    draw_list
        .add_triangle(
            [x - PLAYHEAD_HANDLE_SIZE, y],
            [x + PLAYHEAD_HANDLE_SIZE, y],
            [x, y + PLAYHEAD_HANDLE_SIZE + 4.0],
            [1.0, 0.3, 0.3, 1.0],
        )
        .filled(true)
        .build();

    draw_list
        .add_line(
            [x, y + PLAYHEAD_HANDLE_SIZE],
            [x, y + ruler_height],
            [1.0, 0.3, 0.3, 1.0],
        )
        .thickness(2.0)
        .build();
}

pub(super) fn handle_scrub_interaction(
    ui: &imgui::Ui,
    world: &World,
    interaction: &mut TimelineInteractionState,
    rect_min: [f32; 2],
    rect_max: [f32; 2],
    duration: f32,
    pixels_per_second: f32,
    ruler_start_x: f32,
) {
    let pointer = read_ui_pointer(ui);
    let mouse_pos = pointer.pos;

    if !pointer.is_down(MouseButton::Left) {
        interaction.scrubbing = false;
        return;
    }

    let is_mouse_in_ruler = mouse_pos[0] >= rect_min[0]
        && mouse_pos[0] <= rect_max[0]
        && mouse_pos[1] >= rect_min[1]
        && mouse_pos[1] <= rect_max[1];

    if !interaction.scrubbing
        && !(is_mouse_in_ruler
            && ui_pointer_begin(ui, world, UiPointerOwnerId::Timeline, PointerRegion::Window))
    {
        return;
    }

    interaction.scrubbing = true;
    let relative_x = mouse_pos[0] - ruler_start_x;
    let new_time = (relative_x / pixels_per_second).clamp(0.0, duration);
    world.send_command(TimelineEvent::SetTime(new_time));
}

pub(super) fn calculate_tick_interval(pixels_per_second: f32) -> f32 {
    const TARGET_MINOR_SPACING_PX: f32 = 20.0;
    let raw = TARGET_MINOR_SPACING_PX / pixels_per_second.max(0.001);
    nice_round_step(raw)
}

pub(super) fn nice_round_step(raw: f32) -> f32 {
    if raw <= 0.0 || !raw.is_finite() {
        return 1.0;
    }
    let exp = raw.log10().floor();
    let base = 10f32.powf(exp);
    let mantissa = raw / base;

    let nice_mantissa = if mantissa <= 1.0 {
        1.0
    } else if mantissa <= 2.0 {
        2.0
    } else if mantissa <= 2.5 {
        2.5
    } else if mantissa <= 5.0 {
        5.0
    } else {
        10.0
    };

    nice_mantissa * base
}
