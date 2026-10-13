//! Session-backed persistence of the preview transform for the current source path.

use std::path::PathBuf;

use super::PreviewService;

impl PreviewService {
    /// Re-apply the saved transform when the session still points at this source path.
    pub(super) fn restore_preview_transform(&self, path: &std::path::Path) {
        let Some(session) = &self.session else { return };
        let Ok(saved) = session.snapshot() else {
            return;
        };
        let source_path = path.to_string_lossy().into_owned();
        if saved.preview_path.as_deref() != Some(source_path.as_str()) {
            return;
        }
        if let Ok(mut state) = self.state.lock() {
            state.quarter_turns = saved.preview_quarter_turns.unwrap_or(0).min(3);
            state.flip_horizontal = saved.preview_flip_horizontal.unwrap_or(false);
            state.flip_vertical = saved.preview_flip_vertical.unwrap_or(false);
            state.fit_to_window = saved.preview_fit_to_window.unwrap_or(true);
            state.zoom_scale = saved
                .preview_zoom_scale
                .filter(|value| value.is_finite())
                .unwrap_or(1.0)
                .clamp(0.05, 16.0);
            state.zoom_percent = (state.zoom_scale * 100.0).round() as u16;
            state.pan_x = saved.preview_pan_x.filter(|v| v.is_finite()).unwrap_or(0.0);
            state.pan_y = saved.preview_pan_y.filter(|v| v.is_finite()).unwrap_or(0.0);
            state.zoom_at_cursor = saved.preview_zoom_at_cursor.unwrap_or(false);
            state.show_info = saved.preview_show_info.unwrap_or(false);
        }
    }

    pub(super) fn save_preview_transform(&self) {
        let Some(session) = &self.session else { return };
        let state = self.snapshot();
        let Ok(mut saved) = session.snapshot() else {
            return;
        };
        let current_path = state
            .source
            .as_deref()
            .map(|path| path.to_string_lossy().into_owned());
        if saved.preview_path != current_path {
            return;
        }
        saved.preview_quarter_turns = Some(state.quarter_turns);
        saved.preview_flip_horizontal = Some(state.flip_horizontal);
        saved.preview_flip_vertical = Some(state.flip_vertical);
        saved.preview_fit_to_window = Some(state.fit_to_window);
        saved.preview_zoom_scale = Some(state.zoom_scale);
        saved.preview_pan_x = Some(state.pan_x);
        saved.preview_pan_y = Some(state.pan_y);
        saved.preview_zoom_at_cursor = Some(state.zoom_at_cursor);
        saved.preview_show_info = Some(state.show_info);
        if let Err(error) = session.replace(saved) {
            eprintln!("preview transform session save failed: {error}");
        }
    }

    pub(super) fn save_preview_path(&self, path: Option<PathBuf>) {
        let Some(session) = &self.session else { return };
        if let Ok(mut saved) = session.snapshot() {
            saved.preview_path = path.map(|path| path.to_string_lossy().into_owned());
            if let Err(error) = session.replace(saved) {
                eprintln!("preview session save failed: {error}");
            }
        }
    }
}
