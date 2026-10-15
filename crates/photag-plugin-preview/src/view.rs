//! GPUI presentation for the source preview controls.

mod actions;
mod context_menu;
mod filmstrip;
mod image_stage;
mod keyboard;
mod overlays;
mod read_models;
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
    let surfaces = cordis
        .try_service::<photag_ui_bridge::UiBridgeService>()
        .expect("preview UI requires the registered UI Bridge")
        .surfaces
        .clone();
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
            let content = app
                .new(|cx| {
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
                    let mut revision =
                        photag_ui_bridge::refresh::ChangeGate::new(service.snapshot().revision);
                    let mut decode_revision =
                        photag_ui_bridge::refresh::ChangeGate::new(service.decode_revision());
                    let mut showing_progressive = photag_ui_bridge::refresh::ChangeGate::new(
                        service
                            .snapshot()
                            .source
                            .as_deref()
                            .and_then(|source| service.progressive_path(source))
                            .is_some(),
                    );
                    let mut chrome_alpha = photag_ui_bridge::refresh::ChangeGate::new(
                        (service.chrome_alpha() * 100.).round() as u8,
                    );
                    let collapsed_dropdown_tag_ids = view_session
                        .snapshot()
                        .map(|session| session.collapsed_dropdown_tag_ids)
                        .unwrap_or_default();
                    let decode_service = Arc::clone(&service);
                    let mut filmstrip_request =
                        photag_ui_bridge::refresh::ChangeGate::new(None::<String>);
                    photag_ui_bridge::refresh::poll(
                        cx,
                        std::time::Duration::from_millis(150),
                        move |view: &mut PreviewView, _| {
                            decode_service.advance_wheel_animation();
                            decode_service.request_progressive_if_pending();
                            let next_decode_revision = decode_service.decode_revision();
                            let snapshot = decode_service.snapshot();
                            let next_showing_progressive = snapshot
                                .source
                                .as_deref()
                                .and_then(|source| decode_service.progressive_path(source))
                                .is_some();
                            let mut changed = false;
                            let next = view.service.snapshot().revision;
                            let more = view.surfaces.is_open("preview.more");
                            let tags = view.surfaces.is_open("preview.tags");
                            let request = view.surfaces.value("preview.filmstrip.request");
                            if filmstrip_request.changed(request.clone()) {
                                if let Some(value) = &request {
                                    if let Err(error) = view.settings.update(|settings| {
                                        settings.show_filmstrip = value == "true"
                                    }) {
                                        view.operation_status = Some(error.to_string());
                                    }
                                    view.service.mark_activity();
                                }
                                changed = true;
                            }
                            if more != view.more_actions_open || tags != view.tag_picker_open {
                                view.more_actions_open = more;
                                view.tag_picker_open = tags;
                                view.service.mark_activity();
                                changed = true;
                            }
                            for (did_change, reason) in
                                [
                                    (revision.changed(next), "state"),
                                    (decode_revision.changed(next_decode_revision), "decode"),
                                    (
                                        showing_progressive.changed(next_showing_progressive),
                                        "progressive",
                                    ),
                                    (
                                        chrome_alpha.changed(
                                            (view.service.chrome_alpha() * 100.).round() as u8,
                                        ),
                                        "chrome",
                                    ),
                                ]
                            {
                                if did_change {
                                    view.service.note_poll_wake(reason);
                                    changed = true;
                                }
                            }
                            changed |= view.poll_reads();
                            changed.into()
                        },
                    );
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
                        surfaces: surfaces.clone(),
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
                        collapsed_info_sections: ["Metadata (EXIF)", "File path", "Video"]
                            .into_iter()
                            .collect(),
                        context_menu_open: false,
                        context_menu_anchor: None,
                        last_playback_source: None,
                        last_seen_source: service.snapshot().source,
                        file_revision_gate: Default::default(),
                        index_revision_gate: Default::default(),
                        image_asset_revision: u64::MAX,
                        image_dimensions_cache: Default::default(),
                        file_info_cache: Default::default(),
                        exif_cache: Default::default(),
                        video_info_cache: Default::default(),
                        display_path_cache: Default::default(),
                        indexed_info_cache: Default::default(),
                        tag_tree_cache: None,
                        tag_picker_cache: None,
                        tag_picker_read: Default::default(),
                    }
                })
                .into();
            let visible = service.clone();
            let close = service.clone();
            photag_ui_bridge::panels::mount(
                content,
                photag_ui_bridge::panels::PanelOptions::new(
                    "preview",
                    "Image preview",
                    (1100., 760.),
                ),
                surfaces.clone(),
                move || visible.snapshot().source.is_some(),
                move || close.close(),
                app,
            )
        },
    )
    .with_panel_id("preview")
}
pub(crate) struct PreviewView {
    surfaces: Arc<photag_ui_bridge::UiSurfaceState>,
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
    file_revision_gate: photag_ui_bridge::refresh::ChangeGate<u64>,
    index_revision_gate: photag_ui_bridge::refresh::ChangeGate<u64>,
    image_asset_revision: u64,
    image_dimensions_cache:
        photag_ui_bridge::read_model::ReadModel<std::path::PathBuf, Option<(u32, u32)>>,
    file_info_cache:
        photag_ui_bridge::read_model::ReadModel<std::path::PathBuf, Option<crate::info::FileInfo>>,
    exif_cache: photag_ui_bridge::read_model::ReadModel<std::path::PathBuf, Vec<(String, String)>>,
    video_info_cache: photag_ui_bridge::read_model::ReadModel<
        std::path::PathBuf,
        Option<crate::info::VideoProbe>,
    >,
    display_path_cache:
        photag_ui_bridge::read_model::ReadModel<DisplayPathKey, Option<std::path::PathBuf>>,
    indexed_info_cache:
        photag_ui_bridge::read_model::ReadModel<i64, Option<crate::info::IndexedInfo>>,
    tag_tree_cache: Option<Vec<photag_plugin_database::db::queries::Tag>>,
    tag_picker_cache: Option<(i64, std::collections::HashSet<i64>)>,
    tag_picker_read: photag_ui_bridge::read_model::ReadModel<
        i64,
        (
            Vec<photag_plugin_database::db::queries::Tag>,
            std::collections::HashSet<i64>,
        ),
    >,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DisplayPathKey {
    source: std::path::PathBuf,
    quarter_turns: u8,
    flip_horizontal: bool,
    flip_vertical: bool,
}
