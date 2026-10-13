//! Keyboard navigation and shortcuts for the focused image preview.

use gpui_kit::*;

use super::PreviewView;

impl PreviewView {
    pub(super) fn handle_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.service.mark_activity();
        let key = event.keystroke.key.as_str();
        if self.rename_target.is_some() {
            match key {
                "enter" | "Enter" => self.commit_rename(cx),
                "escape" | "Escape" => {
                    self.rename_target = None;
                    cx.notify();
                }
                _ => {}
            }
            return;
        }
        if self.delete_confirmation_open {
            return;
        }
        if self
            .tag_picker_search
            .read(cx)
            .focus_handle(cx)
            .is_focused(window)
        {
            if key.eq_ignore_ascii_case("escape") {
                self.tag_picker_open = false;
                cx.notify();
            }
            return;
        }
        if self.service.snapshot().media_kind == Some(crate::PreviewMediaKind::Video) {
            match key {
                "space" | "Space" | " " => {
                    if let Some(playback) = self.service.playback() {
                        let _ = playback.toggle_play_pause();
                    }
                    cx.notify();
                    return;
                }
                "left" | "ArrowLeft" => {
                    if let Some(playback) = self.service.playback() {
                        let position = playback.snapshot().position_seconds;
                        let _ = playback.seek(position - 5.0);
                    }
                    cx.notify();
                    return;
                }
                "right" | "ArrowRight" => {
                    if let Some(playback) = self.service.playback() {
                        let position = playback.snapshot().position_seconds;
                        let _ = playback.seek(position + 5.0);
                    }
                    cx.notify();
                    return;
                }
                "escape" | "Escape" => {
                    self.service.close();
                    cx.notify();
                    return;
                }
                _ => {}
            }
        }
        if let Some(rating) = match key {
            "0" => Some(0),
            "1" => Some(1),
            "2" => Some(2),
            "3" => Some(3),
            "4" => Some(4),
            "5" => Some(5),
            _ => None,
        } {
            self.apply_rating_shortcut(rating, cx);
            return;
        }
        if self.handle_tag_hotkey(key, cx) {
            return;
        }
        match key {
            "left" | "ArrowLeft" => {
                if !self.operation_busy {
                    self.service.navigate_filmstrip(-1);
                }
                cx.notify();
            }
            "right" | "ArrowRight" => {
                if !self.operation_busy {
                    self.service.navigate_filmstrip(1);
                }
                cx.notify();
            }
            "escape" | "Escape" => {
                self.service.close();
                cx.notify();
            }
            "i" | "I" => {
                self.service.toggle_info();
                cx.notify();
            }
            "r" | "R" => {
                self.service.rotate_clockwise();
                cx.notify();
            }
            "f" | "F" => {
                self.service.set_fit();
                cx.notify();
            }
            "+" | "=" => {
                let zoom = self.service.snapshot().zoom_percent;
                let next = [25, 50, 100, 125, 150, 200]
                    .into_iter()
                    .find(|preset| *preset > zoom)
                    .unwrap_or(200);
                self.service.set_zoom_percent(next);
                cx.notify();
            }
            "-" => {
                let zoom = self.service.snapshot().zoom_percent;
                let next = [200, 150, 125, 100, 50, 25]
                    .into_iter()
                    .find(|preset| *preset < zoom)
                    .unwrap_or(25);
                self.service.set_zoom_percent(next);
                cx.notify();
            }
            "f2" | "F2" if !self.operation_busy => self.begin_rename(window, cx),
            "delete" | "Delete" | "backspace" | "Backspace" => {
                if !self.operation_busy {
                    self.request_delete(window, cx);
                }
            }
            _ => {}
        }
    }
}
