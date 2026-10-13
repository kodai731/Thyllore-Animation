pub const TOAST_DURATION_SECS: f32 = 1.5;

#[derive(Clone, Debug, PartialEq)]
pub struct ToastMessage {
    pub text: String,
    pub remaining_secs: f32,
}

impl ToastMessage {
    pub fn new(text: String) -> Self {
        Self {
            text,
            remaining_secs: TOAST_DURATION_SECS,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct UiToast {
    pub message: Option<ToastMessage>,
}

pub fn advance_toast(toast: Option<ToastMessage>, dt: f32) -> Option<ToastMessage> {
    let mut message = toast?;
    message.remaining_secs -= dt;
    (message.remaining_secs > 0.0).then_some(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advance_toast_reduces_remaining_time() {
        let toast = ToastMessage::new("Undo: Move keyframe".to_string());

        let advanced = advance_toast(Some(toast), 0.5).expect("toast still visible");

        assert_eq!(advanced.text, "Undo: Move keyframe");
        assert!((advanced.remaining_secs - (TOAST_DURATION_SECS - 0.5)).abs() < 1e-6);
    }

    #[test]
    fn test_advance_toast_clears_when_time_runs_out() {
        let toast = ToastMessage::new("Redo: Move keyframe".to_string());

        assert_eq!(advance_toast(Some(toast), TOAST_DURATION_SECS), None);
    }

    #[test]
    fn test_advance_toast_keeps_none() {
        assert_eq!(advance_toast(None, 0.1), None);
    }
}
