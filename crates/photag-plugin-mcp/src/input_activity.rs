//! Bounded, memory-only window input observations for idle test diagnostics.
use super::CaptureWindowHandle;

#[derive(Default)]
pub(super) struct InputActivity {
    move_events: u64,
    position_changes: u64,
    button_events: u64,
    key_events: u64,
    pointer: Option<(f32, f32)>,
}

impl CaptureWindowHandle {
    pub fn observe_pointer(&self, x: f32, y: f32) {
        if let Ok(mut input) = self.input_activity.lock() {
            input.move_events += 1;
            if input.pointer != Some((x, y)) {
                input.pointer = Some((x, y));
                input.position_changes += 1;
            }
        }
    }

    pub fn observe_button(&self) {
        if let Ok(mut input) = self.input_activity.lock() {
            input.button_events += 1;
        }
    }

    pub fn observe_key(&self) {
        if let Ok(mut input) = self.input_activity.lock() {
            input.key_events += 1;
        }
    }

    pub(super) fn input_snapshot(&self) -> serde_json::Value {
        self.input_activity.lock().map(|input| serde_json::json!({
            "move_events":input.move_events,"position_changes":input.position_changes,
            "button_events":input.button_events,"key_events":input.key_events,"pointer":input.pointer,
        })).unwrap_or(serde_json::Value::Null)
    }
}
