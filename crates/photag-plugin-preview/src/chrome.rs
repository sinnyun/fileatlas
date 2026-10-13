//! Preview chrome visibility and smooth wheel zoom progression.

use super::PreviewService;

impl PreviewService {
    /// Advance wheel zoom and cursor anchoring toward their targets.
    /// Returns true while another frame is needed.
    pub fn advance_wheel_animation(&self) -> bool {
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        let Ok(mut animation) = self.wheel_animation.lock() else {
            return false;
        };
        if state.fit_to_window {
            return false;
        }
        let now = std::time::Instant::now();
        let dt = now
            .duration_since(animation.last_tick)
            .as_secs_f32()
            .clamp(0.0, 0.1);
        animation.last_tick = now;
        let t = 1.0 - (-dt * 25.0).exp();
        state.zoom_scale += (animation.target_zoom - state.zoom_scale) * t;
        state.pan_x += (animation.target_pan_x - state.pan_x) * t;
        state.pan_y += (animation.target_pan_y - state.pan_y) * t;
        let zoom_done = (animation.target_zoom - state.zoom_scale).abs() < 0.0005;
        let pan_x = animation.target_pan_x - state.pan_x;
        let pan_y = animation.target_pan_y - state.pan_y;
        let pan_done = (pan_x * pan_x + pan_y * pan_y).sqrt() < 0.5;
        if zoom_done && pan_done {
            state.zoom_scale = animation.target_zoom;
            state.pan_x = animation.target_pan_x;
            state.pan_y = animation.target_pan_y;
            state.revision = state.revision.wrapping_add(1);
            drop(animation);
            drop(state);
            self.save_preview_transform();
            false
        } else {
            state.revision = state.revision.wrapping_add(1);
            true
        }
    }

    pub fn mark_activity(&self) {
        if let Ok(mut last_activity) = self.last_activity.lock() {
            *last_activity = std::time::Instant::now();
        }
    }

    pub(super) fn sync_wheel_animation(&self) {
        let snapshot = self.snapshot();
        if let Ok(mut animation) = self.wheel_animation.lock() {
            animation.target_zoom = snapshot.zoom_scale;
            animation.target_pan_x = snapshot.pan_x;
            animation.target_pan_y = snapshot.pan_y;
            animation.last_tick = std::time::Instant::now();
        }
    }

    pub fn chrome_alpha(&self) -> f32 {
        let idle = self
            .last_activity
            .lock()
            .map(|last_activity| last_activity.elapsed().as_secs_f32())
            .unwrap_or(0.0);
        if idle < 2.0 {
            1.0
        } else if idle < 2.8 {
            1.0 - (idle - 2.0) / 0.8
        } else {
            0.0
        }
    }
}
