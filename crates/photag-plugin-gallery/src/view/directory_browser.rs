//! Native directory tiles and Navigation-service routing for ungrouped views.
use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::prelude::FluentBuilder as _;
use photag_plugin_navigation::{NavigationService, NavigationUiRequest};

impl GalleryView {
    pub(super) fn locate_file(&mut self, id: i64, cx: &mut Context<Self>) {
        let Some(item) = self.service.media_by_id(id).ok().flatten() else {
            return;
        };
        let Some(parent) = std::path::Path::new(&item.full_path)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
        else {
            return;
        };
        self.restored_file_paths = vec![item.full_path];
        self.restore_in_directory = true;
        self.selection_restore = Default::default();
        self.route_directory(NavigationUiRequest::SelectPath(parent), cx);
        self.media_actions_id = None;
    }

    pub(super) fn pick_browse_location(&mut self, cx: &mut Context<Self>) {
        if self.location_picker_pending {
            return;
        }
        self.location_picker_pending = true;
        let task = self.runtime.spawn(async {
            rfd::AsyncFileDialog::new()
                .pick_folder()
                .await
                .map(|p| p.path().to_string_lossy().into_owned())
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |view, cx| {
                view.location_picker_pending = false;
                if let Ok(Some(path)) = result {
                    view.route_directory(NavigationUiRequest::SelectPath(path), cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn set_descendants(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.service.set_show_descendants(enabled);
        if enabled {
            self.before_descendants_grouping = Some(self.grouping_mode);
            self.grouping_mode = photag_plugin_grouping::GroupingMode::Subfolder;
        } else {
            self.grouping_mode = self
                .before_descendants_grouping
                .take()
                .unwrap_or(photag_plugin_grouping::GroupingMode::None);
        }
        if let Some(settings) = &self.settings {
            let mode = self.grouping_mode;
            let _ = settings.update(|s| {
                s.show_date_dividers = mode == photag_plugin_grouping::GroupingMode::Month
            });
        }
        self.refresh_index();
        cx.notify();
    }

    #[tracing::instrument(skip_all, fields(path=?path))]
    pub(super) fn open_directory(&mut self, path: String, cx: &mut Context<Self>) {
        if !self.service.browser_enabled()
            && !self.snapshot.as_ref().is_ok_and(|s| {
                s.browser.active && s.browser.directories.iter().any(|f| f.path == path)
            })
        {
            self.operation_status =
                Some("Folder is no longer visible. Refresh and try again.".into());
            cx.notify();
            return;
        }
        self.route_directory(NavigationUiRequest::SelectPath(path), cx);
    }
    #[tracing::instrument(skip_all)]
    pub(super) fn directory_up(&mut self, cx: &mut Context<Self>) {
        let Ok(snapshot) = &self.snapshot else {
            return;
        };
        if !snapshot.browser.active || snapshot.browser.directory.is_none() {
            return;
        }
        let request = snapshot
            .browser
            .parent
            .clone()
            .map(NavigationUiRequest::SelectPath)
            .unwrap_or(NavigationUiRequest::AllFiles);
        self.route_directory(request, cx);
    }
    fn route_directory(&mut self, request: NavigationUiRequest, cx: &mut Context<Self>) {
        let result = self
            .cordis
            .try_service::<NavigationService>()
            .map_err(|e| e.to_string())
            .and_then(|service| service.request_ui(request).map_err(|e| e.to_string()));
        if let Err(error) = result {
            self.operation_status = Some(format!("Could not open folder: {error}"));
        } else {
            self.selected_directory = None;
            self.operation_status = None;
        }
        cx.notify();
    }
}

pub(super) fn up_button(view: &GalleryView, cx: &mut Context<GalleryView>) -> Button {
    Button::new("gallery-directory-up")
        .ghost()
        .compact()
        .icon(theme::icon(IconName::ArrowUp))
        .tooltip(photag_ui_bridge::i18n::tr("Up one folder"))
        .disabled(
            !view
                .snapshot
                .as_ref()
                .is_ok_and(|s| s.browser.active && s.browser.directory.is_some()),
        )
        .on_click(cx.listener(|this, _, _, cx| this.directory_up(cx)))
}

pub(super) fn refresh_button(cx: &mut Context<GalleryView>) -> Button {
    Button::new("gallery-directory-refresh")
        .ghost()
        .compact()
        .icon(theme::icon(IconName::RefreshCw))
        .tooltip(photag_ui_bridge::i18n::tr("Refresh folder"))
        .on_click(cx.listener(|this, _, _, cx| {
            this.service.invalidate_directory();
            this.refresh_index();
            cx.notify();
        }))
}

pub(super) fn tile(
    folder: &crate::GalleryDirectory,
    selected: bool,
    thumb: f32,
    list_width: Option<f32>,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let path = folder.path.clone();
    let list = list_width.is_some();
    let icon_area = div()
        .relative()
        .flex_shrink_0()
        .size(px(thumb))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::THUMB_ROUND))
        .bg(rgb(theme::SURFACE_FAINT))
        .border_2()
        .border_color(if selected {
            rgb(theme::accent())
        } else {
            rgba(0)
        })
        .child(
            theme::icon(IconName::Folder)
                .size(px(if list {
                    34.
                } else {
                    (thumb * 0.46).clamp(40., 96.)
                }))
                .text_color(rgb(0xe3bd68)),
        );
    let caption = div()
        .min_w_0()
        .overflow_hidden()
        .text_ellipsis()
        .text_size(px(theme::FONT_SIZE_BUTTON))
        .child(folder.name.clone());
    div()
        .id(format!("gallery-directory-{}", folder.path))
        .relative()
        .w(px(list_width.unwrap_or(thumb)))
        .h(px(if list {
            56.
        } else {
            thumb + card::CARD_CAPTION_HEIGHT
        }))
        .when(list, |row| {
            row.h_flex()
                .items_center()
                .rounded(px(4.))
                .bg(rgb(theme::SURFACE_CARD))
        })
        .when(!list, |row| row.v_flex())
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
            window.focus(&this.focus_handle, cx);
            this.selection = GallerySelection::default();
            this.selected_directory = Some(path.clone());
            this.persist_filter_session();
            if event.click_count() >= 2 {
                this.open_directory(path.clone(), cx);
            }
            cx.notify();
        }))
        .child(icon_area)
        .child(if list {
            div()
                .flex_1()
                .min_w_0()
                .v_flex()
                .px_3()
                .gap_1()
                .child(caption)
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme::TEXT_MUTED))
                        .child(photag_ui_bridge::i18n::tr("Folder")),
                )
        } else {
            div()
                .h(px(card::CARD_CAPTION_HEIGHT))
                .w_full()
                .pt(px(9.))
                .text_center()
                .child(caption)
        })
        .hover(|style| style.bg(rgba(theme::FILL_HOVER)))
        .cursor_pointer()
        .into_any_element()
}

pub(super) fn empty(
    view: &GalleryView,
    message: Option<&str>,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let loading = view.service.directory_status().2;
    let message = if loading {
        Some("Loading folder…")
    } else {
        message
    };
    div()
        .track_focus(&view.focus_handle)
        .on_key_down(cx.listener(|this, event, window, cx| this.handle_key(event, window, cx)))
        .v_flex()
        .flex_1()
        .size_full()
        .items_center()
        .justify_center()
        .gap_3()
        .child(
            theme::icon(IconName::FolderOpen)
                .size(px(48.))
                .text_color(rgb(0xe3bd68)),
        )
        .child(
            div().text_color(rgb(theme::TEXT_SECONDARY)).child(
                message
                    .map(|text| photag_ui_bridge::i18n::tr(text).to_string())
                    .unwrap_or_else(|| {
                        photag_ui_bridge::i18n::tr("This folder is empty").to_string()
                    }),
            ),
        )
        .into_any_element()
}
