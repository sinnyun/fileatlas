//! Floating preview toolbar ported from the pinned source.
//!
//! Source: `vendor/photag-source/src/ui/preview.rs:2370-2675` (image toolbar),
//! `preview.rs:2681-2748` (drag handle + lock menu), `preview.rs:1299-1423`
//! (video toolbar), `settings.rs:203-282` (per-button visibility),
//! `theme.rs:505-512` (item spacing 8x6, button padding 12x5) and
//! `theme.rs:440-471` (ghost widget fill, rounding `ROUND_SM + 1`).
use gpui_kit::assets::IconName;
use std::sync::Arc;
use std::time::Duration;
mod more;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{ContextMenuExt, DropdownMenu, PopupMenuItem};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::slider::{Slider, SliderValue};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use photag_plugin_settings::ToolbarSettings;
use photag_ui_bridge::i18n::tr;
use photag_ui_bridge::theme;

use super::PreviewView;
use super::tag_picker;

/// Image toolbar anchor above the bottom edge (`preview.rs:2391`).
const IMAGE_BOTTOM: f32 = 0.0;
/// Video toolbar anchor above the bottom edge (`preview.rs:1304`).
const VIDEO_BOTTOM: f32 = 100.0;
/// `preview.rs:1708`: the strip lifts the image toolbar by height + 4.
pub(super) const FILMSTRIP_HEIGHT: f32 = 100.0;
const FILMSTRIP_LIFT_GAP: f32 = 0.0;
const FRAME_ROUND: f32 = 8.0;
/// `preview.rs:2404` frame fill RGBA(20, 20, 25, 230).
const IMAGE_FRAME: u32 = (theme::SURFACE_BASE << 8) | 0xff;
/// `preview.rs:1309` frame fill black alpha 160, no shadow.
const VIDEO_FRAME: u32 = 0x000000a0;
const PADDING_X: f32 = 10.0;
/// `preview.rs:2406` inner margin symmetric(10, 6).
const IMAGE_PADDING_Y: f32 = 6.0;
/// `preview.rs:1311` inner margin symmetric(10, 4).
const VIDEO_PADDING_Y: f32 = 4.0;
/// egui item spacing (`theme.rs:507`); `preview.rs:1314` narrows x to 6.
const IMAGE_GAP: f32 = theme::SOURCE_ITEM_SPACING_X;
const VIDEO_GAP: f32 = 6.0;
const VERTICAL_GAP: f32 = theme::SOURCE_ITEM_SPACING_Y;
/// `preview.rs:2682-2698`: 36x4 pill in a 10-high row; the source's hit rect
/// expands it by (8, 6) and overflows the row, GPUI hit areas are the layout
/// box, so only the 8 px horizontal expansion is reproduced.
const HANDLE_ROW_H: f32 = 4.0;
const HANDLE_PILL_W: f32 = 36.0;
const HANDLE_PILL_H: f32 = 2.0;
const HANDLE_HIT_X: f32 = 8.0;
/// `preview.rs:2740-2746`: neutral RGB(190, 190, 200) pill, alpha per state.
const HANDLE_RGB: u32 = 0xBEBEC800;
const HANDLE_IDLE: u32 = 150;
const HANDLE_HOVER: u32 = 220;
const HANDLE_LOCKED: u32 = 70;
/// egui button height: 13 body text plus 2x5 `button_padding` (`theme.rs:508`).
const BUTTON_H: f32 = 24.0;
/// `preview.rs:2509`, `preview.rs:1328`: popup gap above the button.
const POPUP_GAP: f32 = 6.0;
/// `preview.rs:2513`: the source reserves 16 above the button top.
const POPUP_HEADROOM: f32 = 16.0;
const POPUP_MIN_SPACE: f32 = 200.0;
/// `preview.rs:2512`: image popup minimum width.
const IMAGE_POPUP_MIN_W: f32 = 240.0;
/// `preview.rs:2514` / `preview.rs:1331`: height factors and floors.
const IMAGE_POPUP_FACTOR: f32 = 0.90;
const IMAGE_POPUP_MIN_H: f32 = 360.0;
const VIDEO_POPUP_FACTOR: f32 = 0.70;
const VIDEO_POPUP_MIN_H: f32 = 200.0;
/// `preview.rs:2454`: zoom preset combo width.
const PRESET_MENU_W: f32 = 100.0;
/// `preview.rs:2456`: zoom preset entries.
const ZOOM_PRESETS: [u16; 6] = [25, 50, 100, 125, 150, 200];

pub(super) fn filmstrip_lift(strip_active: bool) -> f32 {
    if strip_active {
        FILMSTRIP_HEIGHT + FILMSTRIP_LIFT_GAP
    } else {
        0.0
    }
}

fn tool_button(id: &str, label: String) -> Button {
    Button::new(format!("preview-tb-{id}"))
        .ghost()
        .label(label)
        .px(px(theme::SOURCE_BUTTON_PADDING_X))
        .py(px(theme::SOURCE_BUTTON_PADDING_Y))
        .rounded(px(theme::ROUND_SM + 1.0))
        .text_size(px(theme::FONT_SIZE_BUTTON))
}

/// `preview.rs:2357-2368`: icon-only button whose tooltip shows immediately.
fn icon_button(id: &str, tip: &str) -> Button {
    let icon = match id {
        "rotate-ccw" => IconName::RotateCcw,
        "rotate-cw" => IconName::RotateCw,
        "flip-h" => IconName::FlipHorizontal2,
        "flip-v" => IconName::FlipVertical2,
        "delete" | "video-delete" => IconName::Trash,
        "copy" => IconName::Copy,
        "move" => IconName::ArrowRightLeft,
        "explorer" => IconName::FolderOpen,
        _ => IconName::ExternalLink,
    };
    tool_button(id, String::new())
        .icon(theme::icon(icon))
        .tooltip(tr(tip))
        .tooltip_show_delay(Duration::ZERO)
}

/// `preview.rs:2467,2489,2624,2649`: categories are divided by `|` labels.
fn separator() -> Div {
    div().w(px(1.)).h(px(22.)).bg(rgba(theme::SEPARATOR_LIGHT))
}

/// Popup geometry from the two source popup blocks (image, video).
struct PopupGeom {
    min_width: Option<f32>,
    height_factor: f32,
    min_height: f32,
    /// Distance from the window bottom to the Tags button's top edge.
    button_top: f32,
}

impl PopupGeom {
    fn height(&self, viewport_height: f32) -> f32 {
        let space_above = (viewport_height - self.button_top - POPUP_HEADROOM).max(POPUP_MIN_SPACE);
        (viewport_height * self.height_factor).clamp(self.min_height, space_above)
    }
}

impl PreviewView {
    fn toolbar_settings(&self) -> ToolbarSettings {
        self.settings
            .snapshot()
            .map(|settings| settings.toolbar)
            .unwrap_or_default()
    }

    /// `preview.rs:2389-2392`: the user offset folds into the bottom anchor,
    /// where a positive `offset_y` moves the toolbar down toward the edge.
    fn toolbar_offset(&self) -> (f32, f32) {
        self.settings.snapshot().map_or((0., 0.), |settings| {
            (settings.toolbar_pos.offset_x, settings.toolbar_pos.offset_y)
        })
    }

    fn toolbar_locked(&self) -> bool {
        self.settings
            .snapshot()
            .is_ok_and(|settings| settings.toolbar_pos.locked)
    }

    /// `main.rs:2075,2087-2088`: the strip's visibility is the settings value,
    /// read per frame so a Settings-panel change reaches an open preview.
    pub(super) fn filmstrip_visible(&self) -> bool {
        self.settings
            .snapshot()
            .is_ok_and(|settings| settings.show_filmstrip)
    }

    /// `preview.rs:1696`: the strip only shows with a displayable image.
    fn strip_active(&self, has_display: bool) -> bool {
        self.filmstrip_visible() && has_display && self.service.chrome_alpha() > 0.01
    }

    pub(super) fn begin_toolbar_drag(&mut self, pointer: (f32, f32), cx: &mut Context<Self>) {
        if self.toolbar_locked() {
            return;
        }
        let (x, y) = self.toolbar_offset();
        self.toolbar_drag_origin = Some((pointer.0, pointer.1, x, y));
        self.toolbar_drag_offset = Some((x, y));
        cx.notify();
    }

    /// `preview.rs:2703-2707`: drag deltas accumulate on the stored offset with
    /// no clamp; the handle menu's Reset is the source recovery path.
    pub(super) fn move_toolbar_drag(
        &mut self,
        pointer: (f32, f32),
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((start_x, start_y, offset_x, offset_y)) = self.toolbar_drag_origin else {
            return false;
        };
        let next = (
            offset_x + pointer.0 - start_x,
            offset_y + pointer.1 - start_y,
        );
        if self.toolbar_drag_offset != Some(next) {
            self.toolbar_drag_offset = Some(next);
            cx.notify();
        }
        true
    }

    pub(super) fn end_toolbar_drag(&mut self, cx: &mut Context<Self>) -> bool {
        if self.toolbar_drag_origin.take().is_none() {
            return false;
        }
        if let Some(offset) = self.toolbar_drag_offset.take() {
            match self.settings.update(|settings| {
                settings.toolbar_pos.offset_x = offset.0;
                settings.toolbar_pos.offset_y = offset.1;
            }) {
                Ok(_) => self.toolbar_error = None,
                Err(error) => self.toolbar_error = Some(error.to_string()),
            }
        }
        cx.notify();
        true
    }

    /// `preview.rs:2681-2748`: drag pill plus its right-click lock/reset menu.
    fn handle_row(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let locked = self.toolbar_locked();
        let dragging = self.toolbar_drag_origin.is_some();
        // `preview.rs:2736-2742`: dim when locked, brightest on hover or drag.
        let alpha = if locked {
            HANDLE_LOCKED
        } else if dragging || self.toolbar_handle_hover {
            HANDLE_HOVER
        } else {
            HANDLE_IDLE
        };
        let weak = cx.entity().downgrade();
        div()
            .id("preview-toolbar-drag-handle")
            .w(px(HANDLE_PILL_W + HANDLE_HIT_X * 2.))
            .h(px(HANDLE_ROW_H))
            .flex()
            .items_center()
            .justify_center()
            .map(|handle| {
                if dragging {
                    handle.cursor_grabbing()
                } else if locked {
                    handle
                } else {
                    handle.cursor_grab()
                }
            })
            .child(
                div()
                    .w(px(HANDLE_PILL_W))
                    .h(px(HANDLE_PILL_H))
                    .rounded(px(HANDLE_PILL_H / 2.))
                    .bg(rgba(HANDLE_RGB | alpha)),
            )
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                if this.toolbar_handle_hover != *hovered {
                    this.toolbar_handle_hover = *hovered;
                    cx.notify();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    this.begin_toolbar_drag(
                        (f32::from(event.position.x), f32::from(event.position.y)),
                        cx,
                    );
                }),
            )
            .context_menu(move |menu, _, _| {
                let lock_weak = weak.clone();
                let menu = menu.item(
                    PopupMenuItem::new(tr(if locked {
                        "Unlock position"
                    } else {
                        "Lock position"
                    }))
                    .icon(theme::small_icon(if locked {
                        IconName::LockOpen
                    } else {
                        IconName::Lock
                    }))
                    .on_click(move |_, _, app| {
                        let _ = lock_weak.update(app, |this: &mut PreviewView, cx| {
                            let _ = this.settings.update(|settings| {
                                settings.toolbar_pos.locked = !settings.toolbar_pos.locked
                            });
                            cx.notify();
                        });
                    }),
                );
                let reset_weak = weak.clone();
                menu.item(
                    PopupMenuItem::new(tr("Reset to default"))
                        .icon(theme::small_icon(IconName::RotateCcw))
                        .on_click(move |_, _, app| {
                            let _ = reset_weak.update(app, |this: &mut PreviewView, cx| {
                                this.toolbar_drag_offset = None;
                                match this.settings.update(|settings| {
                                    settings.toolbar_pos.offset_x = 0.;
                                    settings.toolbar_pos.offset_y = 0.;
                                }) {
                                    Ok(_) => this.toolbar_error = None,
                                    Err(error) => this.toolbar_error = Some(error.to_string()),
                                }
                                cx.notify();
                            });
                        }),
                )
            })
            .into_any_element()
    }

    /// `preview.rs:2419-2669`: image control order.
    fn image_controls(
        &mut self,
        popup: &PopupGeom,
        viewport_height: f32,
        cx: &mut Context<Self>,
    ) -> Div {
        let settings = self.toolbar_settings();
        let snapshot = self.service.snapshot();
        let assigned = assigned_tag_count(&snapshot.media_id, &self.tagging);
        let mut anything_in_zoom = false;
        let mut row = div().h_flex().items_center().gap(px(IMAGE_GAP));

        if settings.zoom_mode_button {
            anything_in_zoom = true;
            row = row.child(
                tool_button(
                    "zoom-mode",
                    tr(if snapshot.zoom_at_cursor {
                        "Zoom: Cursor"
                    } else {
                        "Zoom: Center"
                    }),
                )
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.service.toggle_zoom_at_cursor();
                    cx.notify();
                })),
            );
        }
        if settings.zoom_reset_button {
            anything_in_zoom = true;
            row = row.child(tool_button("reset", tr("Reset")).on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    this.service.set_fit();
                    cx.notify();
                },
            )));
        }
        if settings.zoom_presets {
            anything_in_zoom = true;
            let label = if snapshot.fit_to_window {
                tr("Fit")
            } else {
                format!("{}%", snapshot.zoom_percent)
            };
            let weak = cx.entity().downgrade();
            row = row.child(
                tool_button("zoom-presets", label)
                    .w(px(PRESET_MENU_W))
                    .dropdown_caret(true)
                    .dropdown_menu(move |mut menu, _, _| {
                        for percent in ZOOM_PRESETS {
                            let weak = weak.clone();
                            menu = menu.item(PopupMenuItem::new(format!("{percent}%")).on_click(
                                move |_, _, app| {
                                    let _ = weak.update(app, |this: &mut PreviewView, cx| {
                                        this.service.set_zoom_percent(percent);
                                        cx.notify();
                                    });
                                },
                            ));
                        }
                        menu
                    }),
            );
            row = row.child(
                div()
                    .w(px(128.))
                    .flex_shrink_0()
                    .child(Slider::new(&self.zoom_slider).w_full()),
            );
        }

        if anything_in_zoom && settings.manip_category_visible() {
            row = row.child(separator());
        }
        if settings.rotate_buttons {
            for (id, clockwise) in [("rotate-ccw", false), ("rotate-cw", true)] {
                let tip = if clockwise {
                    "Rotate clockwise"
                } else {
                    "Rotate counter-clockwise"
                };
                row = row.child(icon_button(id, tip).on_click(cx.listener(
                    move |this, _: &ClickEvent, _, cx| {
                        if clockwise {
                            this.service.rotate_clockwise();
                        } else {
                            this.service.rotate_counter_clockwise();
                        }
                        cx.notify();
                    },
                )));
            }
        }
        if settings.flip_buttons {
            for (id, horizontal) in [("flip-h", true), ("flip-v", false)] {
                let tip = if horizontal {
                    "Mirror horizontally"
                } else {
                    "Mirror vertically"
                };
                row = row.child(icon_button(id, tip).on_click(cx.listener(
                    move |this, _: &ClickEvent, _, cx| {
                        if horizontal {
                            this.service.flip_horizontal();
                        } else {
                            this.service.flip_vertical();
                        }
                        cx.notify();
                    },
                )));
            }
        }

        if (settings.manip_category_visible() || anything_in_zoom)
            && settings.tags_category_visible()
        {
            row = row.child(separator());
        }
        if settings.tags_button {
            row = row.child(tags_button(self, assigned, popup, viewport_height, cx));
            row = row.child(
                tool_button("clear-tags", tr("Clear tags"))
                    .icon(theme::small_icon(IconName::X))
                    .tooltip(tr("Remove every tag from this item"))
                    .tooltip_show_delay(Duration::ZERO)
                    .disabled(assigned == 0)
                    .on_click(cx.listener(|this, _, _, cx| this.clear_preview_tags(cx))),
            );
        }

        if (settings.tags_category_visible()
            || settings.manip_category_visible()
            || anything_in_zoom)
            && settings.file_category_visible()
        {
            row = row.child(separator());
        }
        if settings.file_category_visible() {
            row = row.child(more::button(self, &settings, cx));
        }

        if (settings.file_category_visible()
            || settings.tags_category_visible()
            || settings.manip_category_visible()
            || settings.zoom_category_visible())
            && settings.view_category_visible()
        {
            row = row.child(separator());
        }
        if settings.filmstrip_toggle {
            let show = self.filmstrip_visible();
            row = row.child(
                tool_button("filmstrip", String::new())
                    .icon(theme::icon(IconName::PanelBottom))
                    .toggled(show)
                    .tooltip(tr(if show {
                        "Hide filmstrip"
                    } else {
                        "Show filmstrip"
                    }))
                    .tooltip_show_delay(Duration::ZERO)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_filmstrip(cx))),
            );
        }
        row
    }

    /// `preview.rs:2398-2413` and `preview.rs:1308-1312`: the frame carries
    /// fill, radius, margin and (image only) the drop shadow.
    fn frame(
        &mut self,
        row: Div,
        fill: u32,
        padding_y: f32,
        offset_x: f32,
        handle: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pill = handle.then(|| self.handle_row(cx));
        div()
            .flex()
            .justify_center()
            // A one-sided pad of twice the source offset shifts the centered
            // frame by exactly `offset_x` (`preview.rs:2389-2392`).
            .map(|band| {
                if offset_x >= 0. {
                    band.pl(px(offset_x * 2.))
                } else {
                    band.pr(px(-offset_x * 2.))
                }
            })
            .child(
                div()
                    .v_flex()
                    .w_full()
                    .items_center()
                    .gap(px(VERTICAL_GAP))
                    .when_some(pill, |frame, pill| frame.child(pill))
                    .child(row)
                    .px(px(PADDING_X))
                    .py(px(padding_y))
                    .rounded(px(if handle { 0. } else { FRAME_ROUND }))
                    .border_t_1()
                    .border_color(rgba(theme::SEPARATOR_LIGHT))
                    .bg(rgba(fill)),
            )
            .on_scroll_wheel(cx.listener(|this, _: &ScrollWheelEvent, _, _| {
                // `preview.rs:2597-2601`: scroll over the toolbar stays there.
                this.service.mark_activity();
            }))
            .into_any_element()
    }

    /// `preview.rs:1301-1423`: video keeps Tags, Clear, `|` and Delete only.
    fn video_toolbar(&mut self, viewport_height: f32, cx: &mut Context<Self>) -> AnyElement {
        let assigned = assigned_tag_count(&self.service.snapshot().media_id, &self.tagging);
        let popup = PopupGeom {
            min_width: None,
            height_factor: VIDEO_POPUP_FACTOR,
            min_height: VIDEO_POPUP_MIN_H,
            button_top: VIDEO_BOTTOM + VIDEO_PADDING_Y + BUTTON_H,
        };
        let row = div()
            .h_flex()
            .items_center()
            .gap(px(VIDEO_GAP))
            .child(tags_button(self, assigned, &popup, viewport_height, cx))
            .child(
                tool_button("video-clear", tr("Clear"))
                    .icon(theme::small_icon(IconName::X))
                    .tooltip(tr("Remove all tags"))
                    .tooltip_show_delay(Duration::ZERO)
                    .disabled(assigned == 0)
                    .on_click(cx.listener(|this, _, _, cx| this.clear_preview_tags(cx))),
            )
            .child(separator())
            .child(
                icon_button("video-delete", "Delete (Del)")
                    .on_click(cx.listener(|this, _, window, cx| this.request_delete(window, cx))),
            );
        div()
            .absolute()
            .bottom(px(VIDEO_BOTTOM))
            .left_0()
            .right_0()
            .child(self.frame(row, VIDEO_FRAME, VIDEO_PADDING_Y, 0., false, cx))
            .into_any_element()
    }

    pub(super) fn render_toolbar(
        &mut self,
        has_display: bool,
        viewport: (f32, f32),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let viewport_height = viewport.1;
        let snapshot = self.service.snapshot();
        match snapshot.media_kind? {
            // `preview.rs:1108` and `:1242`: the audio and video branches return
            // before the image toolbar; audio keeps its controls in the stage.
            crate::PreviewMediaKind::Audio => None,
            // `preview.rs:1708-1711`: the image toolbar needs a display texture.
            // An unclassified file never decodes to one, so it gets no controls.
            crate::PreviewMediaKind::Other => None,
            crate::PreviewMediaKind::Video => Some(self.video_toolbar(viewport_height, cx)),
            crate::PreviewMediaKind::Image => {
                // `preview.rs:1709-1712`.
                let alpha = self.service.chrome_alpha();
                if !self.toolbar_settings().any_visible() || !has_display || alpha <= 0.01 {
                    return None;
                }
                let snapshot = self.service.snapshot();
                let current = match self.zoom_slider.read(cx).value() {
                    SliderValue::Single(value) => value,
                    SliderValue::Range(_, end) => end,
                };
                let next = snapshot.zoom_percent.clamp(25, 200) as f32;
                if (current - next).abs() > 0.5 {
                    self.zoom_slider
                        .update(cx, |state, cx| state.set_value(next, window, cx));
                }
                let (offset_x, offset_y) = self
                    .toolbar_drag_offset
                    .or_else(|| Some(self.toolbar_offset()))
                    .unwrap_or((0., 0.));
                let lift = filmstrip_lift(self.strip_active(has_display));
                // `preview.rs:2391`: anchor offset `-50 - lift + offset_y`.
                let bottom = IMAGE_BOTTOM + lift - offset_y;
                let popup = PopupGeom {
                    min_width: Some(IMAGE_POPUP_MIN_W),
                    height_factor: IMAGE_POPUP_FACTOR,
                    min_height: IMAGE_POPUP_MIN_H,
                    // The popup pivots off the button row, which sits at the
                    // frame's top-side handle above it (`preview.rs:2509-2513`).
                    button_top: bottom + IMAGE_PADDING_Y + BUTTON_H,
                };
                let row = self.image_controls(&popup, viewport_height, cx);
                let frame = self.frame(row, IMAGE_FRAME, IMAGE_PADDING_Y, offset_x, true, cx);
                Some(
                    div()
                        .absolute()
                        .bottom(px(bottom))
                        .left_0()
                        .right_0()
                        .child(frame)
                        .opacity(alpha)
                        .into_any_element(),
                )
            }
        }
    }
}

fn assigned_tag_count(
    media_id: &Option<i64>,
    tagging: &Arc<photag_plugin_tagging::TaggingService>,
) -> usize {
    media_id
        .and_then(|id| tagging.for_media(id).ok())
        .map_or(0, |tags| tags.len())
}

/// `preview.rs:2499-2604`: Tags button plus the popup anchored above it, which
/// stays open after an assignment so several tags can be applied in a row.
fn tags_button(
    view: &mut PreviewView,
    assigned: usize,
    geom: &PopupGeom,
    viewport_height: f32,
    cx: &mut Context<PreviewView>,
) -> AnyElement {
    let open = view.tag_picker_open;
    let height = px(geom.height(viewport_height));
    let body = tag_picker::popup(view, geom.min_width, height, cx);
    let trigger = tool_button("tags", format!("{} ({assigned})", tr("Tags")))
        .icon(theme::icon(IconName::Tag))
        .dropdown_caret(true);
    let weak = cx.entity().downgrade();
    Popover::new("preview-tags-popup")
        .anchor(Anchor::BottomLeft)
        .offset(px(POPUP_GAP))
        .arrow(false)
        .open(open)
        .on_open_change(move |next: &bool, _, app| {
            let next = *next;
            let _ = weak.update(app, move |this: &mut PreviewView, cx| {
                if this.tag_picker_open != next {
                    this.tag_picker_open = next;
                    let _ = this.surfaces.set_open("preview.tags", next);
                    cx.notify();
                }
            });
        })
        .trigger(trigger)
        .child(body)
        .into_any_element()
}
