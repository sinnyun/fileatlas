//! Local MobileNet suggestions and assignment actions, independent of file viewers.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::time::Duration;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use photag_plugin_ai::{AiService, TagSuggestion};
use photag_plugin_history::{HistoryService, UndoEntry};
use photag_plugin_settings::SettingsService;
use photag_plugin_tagging::TaggingService;
use photag_ui_bridge::{UiViewContribution, theme};

#[derive(Default)]
pub(crate) struct AiTarget(std::sync::Mutex<(Option<(PathBuf, i64)>, bool)>);
impl AiTarget {
    pub(crate) fn select(&self, path: PathBuf, id: i64, request: bool) {
        *self.0.lock().expect("AI selection") = (Some((path, id)), request);
    }
    fn snapshot(&self) -> Option<(PathBuf, i64)> {
        self.0.lock().expect("AI selection").0.clone()
    }
    fn take_request(&self) -> bool {
        std::mem::take(&mut self.0.lock().expect("AI selection").1)
    }
}

pub fn view(
    preview: Arc<AiTarget>,
    ai: Arc<AiService>,
    settings: Arc<SettingsService>,
    tagging: Arc<TaggingService>,
    history: Arc<HistoryService>,
) -> UiViewContribution {
    UiViewContribution::new(
        "gallery/ai-suggestions-view",
        "workspace.main",
        1,
        move |_, app| {
            let preview = Arc::clone(&preview);
            let ai = Arc::clone(&ai);
            let settings = Arc::clone(&settings);
            let tagging = Arc::clone(&tagging);
            let history = Arc::clone(&history);
            app.new(|cx| {
                let poll_ai = Arc::clone(&ai);
                let poll_preview = Arc::clone(&preview);
                let poll_settings = Arc::clone(&settings);
                photag_ui_bridge::refresh::poll(
                    cx,
                    Duration::from_millis(150),
                    move |view: &mut AiSuggestionsView, _| {
                        view.poll(&poll_preview, &poll_ai, &poll_settings).into()
                    },
                );
                AiSuggestionsView {
                    ai,
                    settings,
                    tagging,
                    history,
                    key: None,
                    analyzed: HashSet::new(),
                    result: None,
                    error: None,
                    inference: None,
                    download: None,
                    message: None,
                    model_read: Default::default(),
                    model_revision: 0,
                }
            })
            .into()
        },
    )
}

type Inference = mpsc::Receiver<Result<Vec<TagSuggestion>, String>>;
type Download = mpsc::Receiver<Result<(), String>>;

struct AiSuggestionsView {
    ai: Arc<AiService>,
    settings: Arc<SettingsService>,
    tagging: Arc<TaggingService>,
    history: Arc<HistoryService>,
    key: Option<(PathBuf, i64)>,
    analyzed: HashSet<(PathBuf, i64)>,
    result: Option<Vec<TagSuggestion>>,
    error: Option<String>,
    inference: Option<Inference>,
    download: Option<Download>,
    message: Option<String>,
    model_read: photag_ui_bridge::read_model::ReadModel<u64, bool>,
    model_revision: u64,
}

impl AiSuggestionsView {
    fn refresh_model_availability(&mut self) {
        self.model_revision += 1;
        let ai = Arc::clone(&self.ai);
        self.model_read
            .request(self.model_revision, move || Ok(ai.model_exists()));
    }

    fn poll(&mut self, preview: &AiTarget, ai: &AiService, settings: &SettingsService) -> bool {
        let current = preview.snapshot();
        let mut changed = current != self.key;
        if changed {
            let had_pending_work = self.inference.is_some() || self.download.is_some();
            self.key = current.clone();
            self.result = None;
            self.error = None;
            self.message = had_pending_work.then(|| "Previous analysis was abandoned after the preview changed; its worker may finish in the background.".into());
            self.inference = None;
            self.download = None;
            self.refresh_model_availability();
        }
        changed |= self.model_read.poll();
        if preview.take_request() {
            self.request();
            changed = true;
        }
        let download_result = self.download.as_ref().map(|receiver| receiver.try_recv());
        if let Some(result) = download_result {
            changed |= !matches!(&result, Err(mpsc::TryRecvError::Empty));
            match result {
                Ok(Ok(())) => {
                    self.download = None;
                    self.refresh_model_availability();
                    self.message = Some("Model ready. Analyzing image…".into());
                    self.begin_inference(current.clone(), ai, settings);
                }
                Ok(Err(error)) => {
                    self.download = None;
                    self.error = Some(format!("Model download failed: {error}"));
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.download = None;
                    self.error = Some("Model download stopped unexpectedly.".into());
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let inference_result = self.inference.as_ref().map(|receiver| receiver.try_recv());
        if let Some(result) = inference_result {
            changed |= !matches!(&result, Err(mpsc::TryRecvError::Empty));
            match result {
                Ok(Ok(suggestions)) => {
                    self.inference = None;
                    if let Some(key) = current.clone() {
                        self.analyzed.insert(key);
                    }
                    self.message = None;
                    self.result = Some(suggestions);
                }
                Ok(Err(error)) => {
                    self.inference = None;
                    if let Some(key) = current.clone() {
                        self.analyzed.insert(key);
                    }
                    self.error = Some(error);
                    self.message = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.inference = None;
                    if let Some(key) = current.clone() {
                        self.analyzed.insert(key);
                    }
                    self.error = Some("AI inference stopped unexpectedly.".into());
                    self.message = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if let Some(key) = current.as_ref() {
            let auto = settings.snapshot().is_ok_and(|value| value.ai.auto_analyze);
            if auto
                && !self.analyzed.contains(key)
                && self.inference.is_none()
                && self.download.is_none()
                && matches!(self.model_read.value(), Some(Ok(true)))
            {
                self.begin_inference(current, ai, settings);
                changed = true;
            }
        }
        changed
    }

    fn begin_inference(
        &mut self,
        current: Option<(PathBuf, i64)>,
        ai: &AiService,
        settings: &SettingsService,
    ) {
        let Some((path, _)) = current else { return };
        let Ok(settings) = settings.snapshot() else {
            self.error = Some("Could not read AI settings.".into());
            return;
        };
        self.message = Some("Analyzing image…".into());
        self.error = None;
        self.inference =
            Some(ai.suggest_tags(path.to_string_lossy().into_owned(), 8, &settings.ai));
    }

    fn request(&mut self) {
        if self.inference.is_some() || self.download.is_some() {
            return;
        }
        let current = self.key.clone();
        if current.is_none() {
            return;
        }
        self.refresh_model_availability();
        if !self.ai.model_exists() {
            self.error = None;
            self.message = Some("Downloading MobileNet model…".into());
            self.download = Some(self.ai.download_model_async());
        } else {
            let ai = Arc::clone(&self.ai);
            let settings = Arc::clone(&self.settings);
            self.begin_inference(current, &ai, &settings);
        }
    }

    fn apply_label(&mut self, label: &str) -> Result<(), String> {
        self.apply_labels(&[label.to_string()])
    }

    fn apply_labels(&mut self, labels: &[String]) -> Result<(), String> {
        let (_, media_id) = self.key.as_ref().ok_or("No preview image is selected")?;
        let assigned = self
            .tagging
            .apply_ai_suggestions(*media_id, labels)
            .map_err(|e| e.to_string())?;
        if !assigned.is_empty() {
            self.history.push(UndoEntry::AssignedTags {
                media_ids: vec![*media_id],
                tag_ids: assigned,
            });
        }
        Ok(())
    }

    fn remove_label(&mut self, label: &str) -> Result<(), String> {
        let (_, media_id) = self.key.as_ref().ok_or("No preview image is selected")?;
        let assigned = self
            .tagging
            .for_media(*media_id)
            .map_err(|e| e.to_string())?;
        let Some(tag) = assigned
            .iter()
            .find(|tag| tag.name.eq_ignore_ascii_case(label))
        else {
            return Ok(());
        };
        self.tagging
            .remove(&[*media_id], &[tag.id])
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AiService, AiSuggestionsView, AiTarget, HistoryService, SettingsService, TagSuggestion,
        TaggingService,
    };
    use std::{
        collections::HashSet,
        sync::{Arc, mpsc},
    };
    #[test]
    fn unchanged_selection_and_empty_worker_channel_do_not_invalidate() {
        let directory = std::env::temp_dir().join(format!("photag-ai-idle-{}", std::process::id()));
        let database = Arc::new(
            photag_plugin_database::DatabaseService::open_at(directory.join("test.db")).unwrap(),
        );
        let tagging = Arc::new(TaggingService::new(Arc::clone(&database)));
        let files = Arc::new(photag_plugin_file_operations::FileOperationsService::new(
            Arc::clone(&database),
            cordis::Context::new(),
        ));
        let history = Arc::new(HistoryService::new(database, Arc::clone(&tagging), files));
        let settings = Arc::new(SettingsService::open_at(directory.join("settings.json")));
        settings
            .update(|settings| settings.ai.auto_analyze = false)
            .unwrap();
        let ai = Arc::new(AiService);
        let preview = AiTarget::default();
        let mut view = AiSuggestionsView {
            ai: Arc::clone(&ai),
            settings: Arc::clone(&settings),
            tagging,
            history,
            key: None,
            analyzed: HashSet::new(),
            result: None,
            error: None,
            inference: None,
            download: None,
            message: None,
            model_read: Default::default(),
            model_revision: 0,
        };
        for _ in 0..10 {
            assert!(!view.poll(&preview, &ai, &settings));
        }
        preview.select("fixture.jpg".into(), 1, false);
        assert!(view.poll(&preview, &ai, &settings));
        for _ in 0..100 {
            if !view.model_read.is_pending() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
            view.poll(&preview, &ai, &settings);
        }
        assert!(!view.model_read.is_pending());
        let (send, receive) = mpsc::channel();
        view.inference = Some(receive);
        for _ in 0..10 {
            assert!(!view.poll(&preview, &ai, &settings));
        }
        send.send(Ok(vec![TagSuggestion {
            label: "example".into(),
            confidence: 0.9,
            group_name: None,
        }]))
        .unwrap();
        assert!(view.poll(&preview, &ai, &settings));
        assert_eq!(view.result.as_ref().unwrap().len(), 1);
        for _ in 0..10 {
            assert!(!view.poll(&preview, &ai, &settings));
        }
        drop(view);
        drop(preview);
        drop(settings);
        let _ = std::fs::remove_dir_all(directory);
    }
}

impl Render for AiSuggestionsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let unavailable = self.key.is_none();
        if unavailable {
            return div().size_0();
        }
        let request = cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.request();
            cx.notify();
        });
        let mut rows = Vec::new();
        if let Some(suggestions) = &self.result {
            for suggestion in suggestions {
                let label = suggestion.label.clone();
                let add = label.clone();
                let remove = label.clone();
                let apply = cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.message = match this.apply_label(&add) {
                        Ok(()) => Some(format!("Applied {add}.")),
                        Err(e) => Some(e),
                    };
                    cx.notify();
                });
                let remove_action = cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.message = match this.remove_label(&remove) {
                        Ok(()) => Some(format!("Removed {remove}.")),
                        Err(e) => Some(e),
                    };
                    cx.notify();
                });
                let group = suggestion
                    .group_name
                    .as_ref()
                    .map(|name| format!("{name}: "))
                    .unwrap_or_default();
                rows.push(
                    div()
                        .h_flex()
                        .items_center()
                        .gap_2()
                        .child(div().flex_1().child(format!(
                            "{group}{}  {:.0}%",
                            suggestion.label,
                            suggestion.confidence * 100.0
                        )))
                        .child(button("Apply").on_click(apply))
                        .child(button("Remove").on_click(remove_action))
                        .into_any_element(),
                );
            }
            if !suggestions.is_empty() {
                let all: Vec<String> = suggestions.iter().map(|item| item.label.clone()).collect();
                let top: Vec<String> = all.iter().take(3).cloned().collect();
                let apply_all = cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.message = match this.apply_labels(&all) {
                        Ok(()) => Some("Applied all suggestions.".into()),
                        Err(e) => Some(e),
                    };
                    cx.notify();
                });
                let apply_top = cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.message = match this.apply_labels(&top) {
                        Ok(()) => Some("Applied the top three suggestions.".into()),
                        Err(e) => Some(e),
                    };
                    cx.notify();
                });
                rows.push(
                    div()
                        .h_flex()
                        .gap_2()
                        .child(button("Apply all").on_click(apply_all))
                        .when(suggestions.len() > 3, |row| {
                            row.child(button("Top 3").on_click(apply_top))
                        })
                        .into_any_element(),
                );
            }
        }
        let mut content = div()
            .v_flex()
            .gap_2()
            .p_3()
            .rounded_md()
            .bg(rgba(theme::modal_rgba(theme::surface_modal())));
        content = content.child(
            div()
                .h_flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .font_medium()
                        .child(photag_ui_bridge::i18n::tr("AI tag suggestions")),
                )
                .child(
                    button(if self.inference.is_some() || self.download.is_some() {
                        "Working…"
                    } else {
                        "Analyze"
                    })
                    .on_click(request),
                ),
        );
        if unavailable {
            content = content.child(
                div()
                    .text_sm()
                    .text_color(rgb(theme::text_secondary()))
                    .child(photag_ui_bridge::i18n::tr(
                        "Open an indexed image to analyze it.",
                    )),
            );
        } else if matches!(self.model_read.value(), Some(Ok(false)))
            && self.inference.is_none()
            && self.download.is_none()
        {
            content = content.child(
                div()
                    .text_sm()
                    .text_color(rgb(theme::text_secondary()))
                    .child(photag_ui_bridge::i18n::tr(
                        "The first analysis downloads the MobileNet model (~14 MB).",
                    )),
            );
        }
        if let Some(message) = &self.message {
            content = content.child(div().text_sm().child(message.clone()));
        }
        if let Some(error) = &self.error {
            content = content.child(
                div()
                    .text_sm()
                    .text_color(rgb(theme::danger_text()))
                    .child(error.clone()),
            );
        }
        if rows.is_empty() && self.inference.is_none() && self.error.is_none() && !unavailable {
            content = content.child(
                div()
                    .text_sm()
                    .text_color(rgb(theme::text_secondary()))
                    .child(photag_ui_bridge::i18n::tr("No suggestions yet.")),
            );
        }
        content.children(rows)
    }
}

fn button(label: &'static str) -> Button {
    Button::new(format!("ai-{label}")).label(photag_ui_bridge::i18n::tr(&(label).to_string()))
}
