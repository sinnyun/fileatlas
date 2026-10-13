//! GPUI presentation for the source preview controls.

mod actions;
mod context_menu;
mod filmstrip;
mod keyboard;
mod overlays;
mod render;
mod support;
mod tag_picker;
mod toolbar;

use std::sync::Arc;

use cordis::Context as CordisContext;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::InputState;
use gpui_kit::component::slider::{SliderEvent, SliderState, SliderValue};
use gpui_kit::*;
use photag_plugin_file_operations::FileOperationsService;
use photag_plugin_history::HistoryService;
use photag_plugin_session::SessionService;
use photag_plugin_settings::SettingsService;
use photag_plugin_tagging::TaggingService;
use photag_ui_bridge::{UiViewContribution, theme};

use crate::{PreviewMediaKind, PreviewService};
use support::button;

pub fn preview_view(
    service: Arc<PreviewService>,
    settings: Arc<SettingsService>,
    session: Arc<SessionService>,
    tagging: Arc<TaggingService>,
    history: Option<Arc<HistoryService>>,
    cordis: CordisContext,
    file_operations: Arc<FileOperationsService>,
    runtime: tokio::runtime::Handle,
) -> UiViewContribution {
    UiViewContribution::new(
        "preview/image-view",
        "workspace.modal",
        0,
        move |window, app| {
            let service = Arc::clone(&service);
            let view_session = Arc::clone(&session);
            let rename_input = app.new(|cx| {
                InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("New filename"))
            });
            let tag_picker_search = app.new(|cx| {
                InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("Filter tags…"))
            });
            app.new(|cx| {
                let view_service = Arc::clone(&service);
                let focus_handle = cx.focus_handle();
                let zoom_slider = cx.new(|_| {
                    SliderState::new()
                        .min(25.0)
                        .max(200.0)
                        .step(1.0)
                        .default_value(service.snapshot().zoom_percent as f32)
                });
                let zoom_service = Arc::clone(&service);
                let zoom_subscription = cx.subscribe(
                    &zoom_slider,
                    move |_: &mut PreviewView, _, event: &SliderEvent, cx| match event {
                        SliderEvent::Change(SliderValue::Single(value))
                        | SliderEvent::Release(SliderValue::Single(value)) => {
                            zoom_service.set_zoom_scale(*value);
                            cx.notify();
                        }
                        _ => {}
                    },
                );
                let mut revision = service.snapshot().revision;
                let mut decode_revision = service.decode_revision();
                let mut showing_progressive = service
                    .snapshot()
                    .source
                    .as_deref()
                    .and_then(|source| service.progressive_path(source))
                    .is_some();
                let mut chrome_alpha = service.chrome_alpha();
                let collapsed_dropdown_tag_ids = view_session
                    .snapshot()
                    .map(|session| session.collapsed_dropdown_tag_ids)
                    .unwrap_or_default();
                let decode_service = Arc::clone(&service);
                cx.spawn(async move |this, cx| {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(150))
                            .await;
                        decode_service.advance_wheel_animation();
                        let next_decode_revision = decode_service.decode_revision();
                        let snapshot = decode_service.snapshot();
                        let next_showing_progressive = snapshot
                            .source
                            .as_deref()
                            .and_then(|source| decode_service.progressive_path(source))
                            .is_some();
                        let changed = this
                            .update(cx, |view: &mut PreviewView, cx| {
                                let next = view.service.snapshot().revision;
                                let next_chrome_alpha = view.service.chrome_alpha();
                                let chrome_changed =
                                    (next_chrome_alpha - chrome_alpha).abs() > 0.01;
                                if next != revision
                                    || next_decode_revision != decode_revision
                                    || next_showing_progressive != showing_progressive
                                    || chrome_changed
                                {
                                    revision = next;
                                    decode_revision = next_decode_revision;
                                    showing_progressive = next_showing_progressive;
                                    chrome_alpha = next_chrome_alpha;
                                    cx.notify();
                                }
                            })
                            .is_ok();
                        if !changed {
                            break;
                        }
                    }
                })
                .detach();
                PreviewView {
                    locale_revision: u64::MAX,
                    service: view_service,
                    zoom_slider,
                    _zoom_subscription: zoom_subscription,
                    more_actions_open: false,
                    settings: Arc::clone(&settings),
                    session: view_session,
                    tagging: Arc::clone(&tagging),
                    history: history.clone(),
                    cordis: cordis.clone(),
                    last_drag_position: None,
                    toolbar_drag_origin: None,
                    toolbar_drag_offset: None,
                    toolbar_error: None,
                    toolbar_handle_hover: false,
                    file_operations: Arc::clone(&file_operations),
                    runtime: runtime.clone(),
                    rename_input,
                    rename_target: None,
                    delete_confirmation_open: false,
                    operation_status: None,
                    operation_busy: false,
                    focus_handle,
                    tag_picker_search: tag_picker_search.clone(),
                    tag_picker_open: false,
                    collapsed_dropdown_tag_ids,
                    collapsed_info_sections: std::collections::HashSet::new(),
                    context_menu_open: false,
                    context_menu_anchor: None,
                    last_playback_source: None,
                    last_seen_source: service.snapshot().source,
                }
            })
            .into()
        },
    )
}
pub(crate) struct PreviewView {
    locale_revision: u64,
    service: Arc<PreviewService>,
    zoom_slider: Entity<SliderState>,
    _zoom_subscription: Subscription,
    more_actions_open: bool,
    settings: Arc<SettingsService>,
    session: Arc<SessionService>,
    tagging: Arc<TaggingService>,
    history: Option<Arc<HistoryService>>,
    cordis: CordisContext,
    last_drag_position: Option<(f32, f32)>,
    toolbar_drag_origin: Option<(f32, f32, f32, f32)>,
    toolbar_drag_offset: Option<(f32, f32)>,
    toolbar_handle_hover: bool,
    toolbar_error: Option<String>,
    file_operations: Arc<FileOperationsService>,
    runtime: tokio::runtime::Handle,
    rename_input: gpui::Entity<InputState>,
    rename_target: Option<i64>,
    delete_confirmation_open: bool,
    operation_status: Option<String>,
    operation_busy: bool,
    focus_handle: FocusHandle,
    tag_picker_search: gpui::Entity<InputState>,
    tag_picker_open: bool,
    collapsed_dropdown_tag_ids: std::collections::HashSet<i64>,
    pub(crate) collapsed_info_sections: std::collections::HashSet<&'static str>,
    context_menu_open: bool,
    context_menu_anchor: Option<(f32, f32)>,
    last_playback_source: Option<std::path::PathBuf>,
    last_seen_source: Option<std::path::PathBuf>,
}
