//! Native directory tiles and Navigation-service routing for ungrouped views.
use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::prelude::FluentBuilder as _;
use photag_plugin_navigation::{NavigationService, NavigationUiRequest};

impl GalleryView {
    #[tracing::instrument(skip_all, fields(path=?path))]
    pub(super) fn open_directory(&mut self, path: String, cx: &mut Context<Self>) {
        if !self
            .snapshot
            .as_ref()
            .is_ok_and(|s| s.browser.active && s.browser.directories.iter().any(|f| f.path == path))
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
