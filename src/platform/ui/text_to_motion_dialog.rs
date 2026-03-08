use crate::ecs::events::UIEventQueue;
use crate::ecs::resource::{TextToMotionState, TextToMotionStatus};
use crate::ecs::World;

pub struct TextToMotionDialogState {
    pub open: bool,
    pub prompt_buf: String,
    pub duration: f32,
}

impl Default for TextToMotionDialogState {
    fn default() -> Self {
        Self {
            open: false,
            prompt_buf: String::new(),
            duration: 3.0,
        }
    }
}

pub fn build_text_to_motion_dialog(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    dialog: &mut TextToMotionDialogState,
    world: &World,
) {
    if !dialog.open {
        return;
    }

    if !world.contains_resource::<TextToMotionState>() {
        return;
    }

    let state = world.resource::<TextToMotionState>();
    let status = state.status.clone();
    let error_msg = state.error_message.clone();
    let gen_time = state.generation_time_ms;
    let model_used = state.model_used.clone();
    let has_clip = state.generated_clip.is_some();
    let track_count = state
        .generated_clip
        .as_ref()
        .map(|c| c.tracks.len())
        .unwrap_or(0);
    drop(state);

    let mut should_close = false;

    ui.window("Text to Motion")
        .size([400.0, 320.0], imgui::Condition::FirstUseEver)
        .build(|| {
            build_input_section(
                ui,
                ui_events,
                &mut dialog.prompt_buf,
                &mut dialog.duration,
                &status,
                &mut should_close,
            );
            ui.separator();
            build_status_section(ui, &status, &error_msg, gen_time, &model_used);

            if has_clip {
                ui.separator();
                build_result_section(ui, ui_events, track_count, &mut should_close);
            }
        });

    if should_close {
        dialog.open = false;
    }
}

fn build_input_section(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    prompt_buf: &mut String,
    duration: &mut f32,
    status: &TextToMotionStatus,
    should_close: &mut bool,
) {
    let is_generating = *status == TextToMotionStatus::Generating;

    ui.text("Prompt:");
    ui.input_text("##prompt", prompt_buf)
        .hint("e.g. walking forward slowly")
        .build();

    ui.text("Duration (sec):");
    ui.same_line();
    ui.set_next_item_width(100.0);
    imgui::Drag::new("##duration")
        .range(0.5, 10.0)
        .speed(0.1)
        .display_format("%.1f")
        .build(ui, duration);

    let can_generate = !is_generating && !prompt_buf.trim().is_empty();

    ui.spacing();
    if is_generating {
        ui.text("Generating...");
    } else {
        let _disabled = ui.begin_disabled(!can_generate);
        if ui.button("Generate") {
            ui_events.send(crate::ecs::events::UIEvent::TextToMotionGenerate {
                prompt: prompt_buf.trim().to_string(),
                duration_seconds: *duration,
            });
        }
    }

    ui.same_line();
    if ui.button("Cancel") {
        if is_generating {
            ui_events.send(crate::ecs::events::UIEvent::TextToMotionCancel);
        } else {
            *should_close = true;
        }
    }
}

fn build_status_section(
    ui: &imgui::Ui,
    status: &TextToMotionStatus,
    error_msg: &Option<String>,
    gen_time: Option<f32>,
    model_used: &Option<String>,
) {
    let status_text = match status {
        TextToMotionStatus::Idle => "Idle",
        TextToMotionStatus::Connecting => "Connecting...",
        TextToMotionStatus::Generating => "Generating...",
        TextToMotionStatus::Generated => "Generated",
        TextToMotionStatus::Error => "Error",
    };
    ui.text(format!("Status: {}", status_text));

    if let Some(time_ms) = gen_time {
        ui.text(format!("Generation time: {:.0}ms", time_ms));
    }

    if let Some(model) = model_used {
        ui.text(format!("Model: {}", model));
    }

    if let Some(err) = error_msg {
        ui.text_colored([1.0, 0.3, 0.3, 1.0], format!("Error: {}", err));
    }
}

fn build_result_section(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    track_count: usize,
    should_close: &mut bool,
) {
    ui.text(format!("Result: {} bone tracks", track_count));
    ui.spacing();

    if ui.button("Apply to Timeline") {
        ui_events.send(crate::ecs::events::UIEvent::TextToMotionApply);
        *should_close = true;
    }

    ui.same_line();
    if ui.button("Dismiss") {
        ui_events.send(crate::ecs::events::UIEvent::TextToMotionCancel);
    }
}
