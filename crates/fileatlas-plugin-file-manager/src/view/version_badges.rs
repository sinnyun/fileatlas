//! Bounded visible-file reads. No filesystem or Lore IO in card rendering.
use super::*;
use fileatlas_plugin_versions::{FileVersionState, FileVersionStatus};
use gpui_kit::assets::IconName;
use gpui_kit::component::button::ButtonVariants as _;

pub(super) type StateKey = (u64, u64, Vec<(i64, String)>);

impl FileManagerView {
    pub(super) fn version_state_snapshot(&self) -> Vec<crate::FileManagerVersionState> {
        self.version_states
            .value()
            .and_then(|read| read.as_ref().ok())
            .cloned()
            .unwrap_or_default()
    }
    pub(super) fn request_version_states(&mut self, visible: &[i64]) {
        let paths = self
            .snapshot
            .as_ref()
            .map(|snapshot| {
                snapshot
                    .media
                    .iter()
                    .filter(|media| visible.contains(&media.id))
                    .take(200)
                    .map(|media| (media.id, media.full_path.clone()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        // Filesystem/index changes reload the snapshot; version actions advance
        // history_generation. The database-wide write revision also includes
        // unrelated clipboard/launcher/cache writes and must not reopen Lore.
        let key = (
            self.snapshot_generation,
            self.history_generation,
            paths.clone(),
        );
        let versions = self.versions.clone();
        let runtime = self.runtime.clone();
        self.version_states.set_wake(self.service.wake.clone());
        self.version_states.request(key, move || {
            runtime.block_on(async move {
                let mut tasks = tokio::task::JoinSet::new();
                let mut pending = paths.into_iter();
                let mut states = Vec::new();
                loop {
                    while tasks.len() < 8 {
                        let Some((media_id, path)) = pending.next() else {
                            break;
                        };
                        let versions = versions.clone();
                        tasks.spawn(async move {
                            (media_id, path.clone(), versions.file_state(&path).await)
                        });
                    }
                    let Some(result) = tasks.join_next().await else {
                        break;
                    };
                    let (media_id, path, result) =
                        result.map_err(|error| format!("version state task failed: {error}"))?;
                    let (state, error) = match result {
                        Ok(state) => (Some(state), None),
                        Err(error) => (None, Some(error.to_string())),
                    };
                    states.push(crate::FileManagerVersionState {
                        media_id,
                        path,
                        state,
                        error,
                    });
                }
                Ok(states)
            })
        });
    }

    pub(super) fn version_state(
        &self,
        id: i64,
        path: &str,
    ) -> Option<&crate::FileManagerVersionState> {
        self.version_states
            .value()?
            .as_ref()
            .ok()?
            .iter()
            .find(|state| state.media_id == id && state.path == path)
    }

    fn open_unsaved_history(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        self.select_media(id, Modifiers::default(), window, cx);
        let _ = self.surfaces.set_open("inspector", true);
        let _ = self
            .surfaces
            .set_value("file_manager.inspector.tab", "history");
        cx.notify();
    }
}

pub(super) fn message(state: &FileVersionState) -> String {
    let base = state.current_title.clone().unwrap_or_default();
    let text = match state.status {
        FileVersionStatus::Modified => {
            "Unsaved version changes. Base: {base}. Open History to save a version."
        }
        FileVersionStatus::Saved => "File content matches the saved version: {base}.",
        FileVersionStatus::MatchesSavedVersion => {
            "Content already matches {matched}; selected version: {base}. No new version is needed."
        }
        FileVersionStatus::Unversioned => return String::new(),
    };
    fileatlas_ui_bridge::i18n::tr_args(
        text,
        &[
            ("base", base),
            ("matched", state.matching_title.clone().unwrap_or_default()),
        ],
    )
}

pub(super) fn badge(
    id: i64,
    state: &FileVersionState,
    cx: &mut Context<FileManagerView>,
) -> Button {
    Button::new(format!("file_manager-unsaved-version-{id}"))
        .ghost()
        .compact()
        .w(px(24.))
        .h(px(24.))
        .rounded_full()
        .bg(rgba(theme::tooltip_surface()))
        .icon(
            theme::icon(IconName::Circle)
                .size(px(13.))
                .rounded_full()
                .bg(rgb(theme::surface_elevated()))
                .text_color(rgb(theme::warning_text())),
        )
        .tooltip(message(state))
        .on_click(cx.listener(move |view, _, window, cx| view.open_unsaved_history(id, window, cx)))
}
