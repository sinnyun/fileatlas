//! Automatic acquisition of native document changes.
//!
//! Browsing reads only the cached catalogue. This watcher compares the cached source stamp
//! with the saved document's own stamp; only a real external save starts the silent official
//! session that rebuilds the catalogue and previews. The user's running GUI is never driven,
//! focused or activated by this path.
use super::*;
use std::time::{Duration, Instant};

const PREVIEW_STATE_INTERVAL: Duration = Duration::from_millis(1500);
const AUTO_SYNC_RETRY: Duration = Duration::from_secs(30);

impl WorkCratePanel {
    pub(super) fn start_preview_watcher(&mut self, cx: &mut ViewContext<Self>) {
        refresh::poll(cx, PREVIEW_STATE_INTERVAL, |view, cx| {
            view.take_canvas_drop_request(cx);
            view.request_preview_state()
        });
    }

    /// One cheap sample per tick: stat the saved document and read its cached source stamp.
    /// Nothing is requested while an official session, a save or a reload owns the panel.
    pub(super) fn request_preview_state(&mut self) -> refresh::PollResult {
        if self.busy
            || self.canvas_busy
            || self.canvas.is_some()
            || self.auto_sync
            || !self.field_saves.is_empty()
            || self.document.is_none()
            || self.preview_state_read.is_pending()
        {
            return refresh::PollResult::until(false, true);
        }
        if self
            .auto_sync_retry_at
            .is_some_and(|retry| Instant::now() < retry)
        {
            return refresh::PollResult::until(false, true);
        }
        let Some(project) = self.selected_project.clone() else {
            return refresh::PollResult::until(false, true);
        };
        self.preview_state_tick += 1;
        let boards = self.boards.clone();
        self.preview_state_read
            .request(self.preview_state_tick, move || {
                boards.preview_state(&project).map_err(|e| e.to_string())
            });
        refresh::PollResult::until(false, true)
    }

    /// A stale cache starts one background refresh. The catalogue and its bitmaps are patched
    /// only after the official session returns, so the page matrix never empties while a
    /// refresh is running and the user can keep browsing.
    pub(super) fn deliver_preview_state(&mut self, cx: &mut ViewContext<Self>) -> bool {
        if !self.preview_state_read.poll() {
            return false;
        }
        let state = match self.preview_state_read.value().cloned() {
            Some(Ok(state)) => state,
            Some(Err(error)) => return self.note_auto_sync_error(error),
            None => return false,
        };
        if self.selected_project.as_deref() != Some(state.project_id.as_str()) {
            return false;
        }
        if !state.stale() {
            self.auto_sync_error = None;
            self.auto_sync_retry_at = None;
            return false;
        }
        if self.busy
            || self.canvas_busy
            || self.canvas.is_some()
            || self.auto_sync
            || !self.field_saves.is_empty()
            || self
                .auto_sync_retry_at
                .is_some_and(|retry| Instant::now() < retry)
        {
            return false;
        }
        let Some(project) = self.selected_project.clone() else {
            return false;
        };
        self.start_auto_sync(project, cx);
        false
    }

    fn start_auto_sync(&mut self, project: String, cx: &mut ViewContext<Self>) {
        self.auto_sync = true;
        cx.notify();
        let boards = self.boards.clone();
        let job_project = project.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    boards
                        .auto_refresh_project(&job_project)
                        .map_err(|e| e.to_string())
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.auto_sync = false;
                match result {
                    Ok(sync) => {
                        view.auto_sync_error = None;
                        view.auto_sync_retry_at = None;
                        if sync.synced {
                            view.auto_syncs += 1;
                            view.apply_auto_sync(sync, &project, cx);
                        }
                    }
                    Err(error) => {
                        view.note_auto_sync_error(error);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Replace the catalogue from the refreshed cache while keeping every decoded frame as a
    /// placeholder; new previews are decoded in the background and swap in page by page.
    fn apply_auto_sync(&mut self, sync: AutoSync, project: &str, cx: &mut ViewContext<Self>) {
        if self.selected_project.as_deref() != Some(project)
            || self.canvas.is_some()
            || !self.field_saves.is_empty()
        {
            return;
        }
        let mut previous = std::mem::take(&mut self.boards_view);
        let mut cards = Vec::with_capacity(sync.boards.len());
        for board in sync.boards {
            let image = previous
                .iter()
                .position(|card| card.board.id == board.id)
                .map(|index| previous.remove(index).image)
                .flatten();
            cards.push(BoardCard {
                board,
                image,
                error: None,
                preview_checked: false,
                canvas_revision: None,
            });
        }
        self.boards_view = cards;
        self.load_generation += 1;
        self.preview_queue.clear();
        self.preview_current = None;
        self.selected_pages
            .retain(|id| self.boards_view.iter().any(|card| &card.board.id == id));
        if self
            .editing
            .as_ref()
            .is_some_and(|(id, _)| !self.boards_view.iter().any(|card| &card.board.id == id))
        {
            self.editing = None;
        }
        if self
            .selected_board
            .as_ref()
            .is_some_and(|id| !self.boards_view.iter().any(|card| &card.board.id == id))
        {
            self.selected_board = None;
        }
        self.preview_queue = self
            .boards_view
            .iter()
            .map(|card| card.board.id.clone())
            .collect();
        if let Some(selected) = &self.selected_board {
            self.preview_queue.retain(|id| id != selected);
            self.preview_queue.push_front(selected.clone());
        }
        self.start_preview();
        if self.message.as_deref().is_none_or(|message| {
            message.starts_with("已自动同步") || message.starts_with("自动同步失败")
        }) {
            self.message = Some("已自动同步 VectorCraft 的更改".into());
        }
        cx.notify();
    }

    fn note_auto_sync_error(&mut self, error: String) -> bool {
        let first = self.auto_sync_error.as_deref() != Some(error.as_str());
        self.auto_sync_error = Some(error.clone());
        self.auto_sync_retry_at = Some(Instant::now() + AUTO_SYNC_RETRY);
        let visible = self.message.as_deref().is_none_or(|message| {
            message.starts_with("已自动同步") || message.starts_with("自动同步失败")
        });
        if first && visible {
            self.message = Some(format!("自动同步失败：{error}"));
        }
        first && visible
    }
}
