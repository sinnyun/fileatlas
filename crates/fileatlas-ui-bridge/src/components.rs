//! Shared workbench primitives. Callers own content, actions and lifecycle.
//! Import `WorkbenchStyle` for a Div whose ID/geometry the feature already owns.
use crate::theme;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::date_picker::{DatePicker, DatePickerState};
use gpui_kit::component::input::{Input, InputState, Textarea, TextareaState};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::radio::Radio;
use gpui_kit::component::separator::Separator;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

pub trait WorkbenchTypography: Sized {
    #[track_caller]
    fn workbench_title(self) -> Self;
    #[track_caller]
    fn workbench_body(self) -> Self;
    #[track_caller]
    fn workbench_assistant(self) -> Self;
}

/// Semantic foregrounds for shared controls and status labels. Plugins choose
/// a role from domain state; they do not resolve or paint theme colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForegroundRole {
    Primary,
    Secondary,
    Muted,
    Accent,
    AccentText,
    Selection,
    Warning,
    Danger,
    DangerText,
    Success,
    OnDanger,
    OnWarning,
    OnSuccess,
    OnMediaBadge,
    Star,
    StarOnMediaBadge,
    Custom(u32),
}

/// Shared fill semantics accepted by plugin views. Color-bearing data may pass
/// through `Custom`; ordinary surfaces resolve from the native theme here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceRole {
    Base,
    Faint,
    Panel,
    Card,
    Elevated,
    Sidebar,
    Rail,
    Modal,
    Accent,
    Danger,
    Warning,
    Success,
    Separator,
    GroupHeader,
    Selection,
    Tooltip,
    MediaBadge,
    CardOverlay,
    BaseOverlay,
    Transparent(u32),
    Custom(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CornerRole {
    Micro,
    Small,
    Medium,
    Large,
    ExtraLarge,
    Pill,
    Thumbnail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusTone {
    Neutral,
    Accent,
    Success,
    Warning,
    Danger,
    Custom(u32),
}

pub trait WorkbenchPaint: Sized {
    fn workbench_surface(self, role: SurfaceRole) -> Self;
    fn workbench_corner(self, role: CornerRole) -> Self;
    fn workbench_corner_right(self, role: CornerRole) -> Self;
    fn workbench_visible(self, visible: bool) -> Self;
}

impl<T: Styled> WorkbenchPaint for T {
    fn workbench_surface(self, role: SurfaceRole) -> Self {
        match role {
            SurfaceRole::Base => self.bg(rgb(theme::surface_base())),
            SurfaceRole::Faint => self.bg(rgb(theme::surface_faint())),
            SurfaceRole::Panel => self.bg(rgb(theme::surface_panel())),
            SurfaceRole::Card => self.bg(rgb(theme::surface_card())),
            SurfaceRole::Elevated => self.bg(rgb(theme::surface_elevated())),
            SurfaceRole::Sidebar => self.bg(rgb(theme::surface_sidebar())),
            SurfaceRole::Rail => self.bg(rgb(theme::surface_rail())),
            SurfaceRole::Modal => self.bg(rgb(theme::surface_modal())),
            SurfaceRole::Accent => self.bg(rgb(theme::accent())),
            SurfaceRole::Danger => self.bg(rgb(theme::danger())),
            SurfaceRole::Warning => self.bg(rgb(theme::warning())),
            SurfaceRole::Success => self.bg(rgb(theme::success())),
            SurfaceRole::Separator => self.bg(rgb(theme::separator())),
            SurfaceRole::GroupHeader => self.bg(rgb(theme::surface_group_header())),
            SurfaceRole::Selection => self.bg(rgb(theme::selection_color())),
            SurfaceRole::Tooltip => self.bg(rgb(theme::tooltip_surface())),
            SurfaceRole::MediaBadge => self.bg(rgb(theme::media_badge_background())),
            SurfaceRole::CardOverlay => self.bg(rgba(theme::panel_rgba(theme::surface_card()))),
            SurfaceRole::BaseOverlay => self.bg(rgba(theme::base_rgba(theme::surface_base()))),
            SurfaceRole::Transparent(color) => self.bg(rgba(color)),
            SurfaceRole::Custom(color) => self.bg(rgb(color)),
        }
    }

    fn workbench_corner(self, role: CornerRole) -> Self {
        let radius = match role {
            CornerRole::Micro => theme::ROUND_MICRO,
            CornerRole::Small => theme::ROUND_SM,
            CornerRole::Medium => theme::ROUND_MD,
            CornerRole::Large => theme::ROUND_LG,
            CornerRole::ExtraLarge => theme::ROUND_XL,
            CornerRole::Pill => theme::ROUND_PILL,
            CornerRole::Thumbnail => theme::THUMB_ROUND,
        };
        self.rounded(px(radius))
    }

    fn workbench_corner_right(self, role: CornerRole) -> Self {
        let radius = match role {
            CornerRole::Micro => theme::ROUND_MICRO,
            CornerRole::Small => theme::ROUND_SM,
            CornerRole::Medium => theme::ROUND_MD,
            CornerRole::Large => theme::ROUND_LG,
            CornerRole::ExtraLarge => theme::ROUND_XL,
            CornerRole::Pill => theme::ROUND_PILL,
            CornerRole::Thumbnail => theme::THUMB_ROUND,
        };
        self.rounded_r(px(radius))
    }

    fn workbench_visible(self, visible: bool) -> Self {
        self.opacity(if visible { 1. } else { 0. })
    }
}

pub trait WorkbenchForeground: Sized {
    fn workbench_foreground(self, role: ForegroundRole) -> Self;
}

impl<T: Styled> WorkbenchForeground for T {
    fn workbench_foreground(self, role: ForegroundRole) -> Self {
        let color = match role {
            ForegroundRole::Primary => theme::text_primary(),
            ForegroundRole::Secondary => theme::text_secondary(),
            ForegroundRole::Muted => theme::text_muted(),
            ForegroundRole::Accent => theme::accent(),
            ForegroundRole::AccentText => theme::accent_text(),
            ForegroundRole::Selection => theme::selection_text(),
            ForegroundRole::Warning => theme::warning_text(),
            ForegroundRole::Danger => theme::danger_text(),
            ForegroundRole::DangerText => theme::text_on_danger(),
            ForegroundRole::Success => theme::success_text(),
            ForegroundRole::OnDanger => theme::text_on_danger(),
            ForegroundRole::OnWarning => theme::text_on_warning(),
            ForegroundRole::OnSuccess => theme::text_on_success(),
            ForegroundRole::OnMediaBadge => theme::text_on_media_badge(),
            ForegroundRole::Star => theme::star_text(),
            ForegroundRole::StarOnMediaBadge => theme::star_on_media_badge(),
            ForegroundRole::Custom(color) => color,
        };
        self.text_color(rgb(color))
    }
}

impl<T: Styled + ParentElement> WorkbenchTypography for T {
    #[track_caller]
    fn workbench_title(self) -> Self {
        let style = self
            .text_size(px(theme::FONT_SIZE_TITLE))
            .line_height(px(theme::LINE_HEIGHT_TITLE))
            .font_semibold()
            .text_color(rgb(theme::text_title()));
        if theme::geometry_audit_enabled() {
            observe_typography(style, "title", theme::AUDIT_FRAME_TEXT)
        } else {
            style
        }
    }
    #[track_caller]
    fn workbench_assistant(self) -> Self {
        let style = self
            .text_size(px(theme::FONT_SIZE_ASSISTANT))
            .line_height(px(theme::LINE_HEIGHT_ASSISTANT))
            .font_weight(FontWeight::NORMAL)
            .text_color(rgb(theme::text_secondary()));
        if theme::geometry_audit_enabled() {
            observe_typography(style, "assistant", theme::AUDIT_FRAME_TEXT)
        } else {
            style
        }
    }
    #[track_caller]
    fn workbench_body(self) -> Self {
        let style = self
            .text_size(px(theme::FONT_SIZE_BODY))
            .line_height(px(theme::LINE_HEIGHT_BODY))
            .font_weight(FontWeight::NORMAL)
            .text_color(rgb(theme::text_primary()));
        if theme::geometry_audit_enabled() {
            observe_typography(style, "body", theme::AUDIT_FRAME_TEXT)
        } else {
            style
        }
    }
}

#[track_caller]
fn observe_typography<T: Styled + ParentElement>(element: T, tier: &'static str, color: u32) -> T {
    let location = std::panic::Location::caller();
    let source_file = location.file();
    let source_line = location.line();
    element.child(
        canvas(
            move |bounds, window, _| {
                crate::geometry_audit::record(
                    bounds,
                    window,
                    tier,
                    source_file,
                    source_line,
                    None,
                    crate::geometry_audit::ComponentTokens {
                        ..Default::default()
                    },
                );
            },
            move |bounds, _, window, _| {
                paint_audit_outline(bounds, color, 2., window);
            },
        )
        .absolute()
        .inset_0()
        .size_full(),
    )
}

/// Add the assistant-tier audit frame without changing a component's semantic
/// foreground color (for example, a user-colored tag capsule).
#[track_caller]
pub fn observe_assistant_typography<T: Styled + ParentElement>(element: T) -> T {
    if theme::geometry_audit_enabled() {
        observe_typography(element, "assistant", theme::AUDIT_FRAME_TEXT)
    } else {
        element
    }
}

pub trait WorkbenchStyle: Sized {
    fn workbench_region(self, sidebar: bool) -> Self;
    fn workbench_section(self) -> Self;
    fn workbench_card(self, selected: bool) -> Self;
    /// Subtle elevation for the file browser's media cards.
    fn workbench_media_card(self) -> Self;
    fn workbench_nav(self, selected: bool) -> Self;
    #[track_caller]
    fn workbench_hover_surface(self) -> Self;
    fn workbench_hover_accent_surface(self) -> Self;
    fn workbench_hover_accent_border(self) -> Self;
    #[track_caller]
    fn workbench_selectable_surface(self, selected: bool) -> Self;
    fn workbench_hover_foreground(self, foreground: u32) -> Self;
    #[track_caller]
    fn workbench_hover_group_header(self) -> Self;
    /// Fluent-style selection row shared by settings navigation, sidebar
    /// folder trees and the project tree: a filled row with a small accent
    /// indicator bar on the leading edge. Callers keep their own height,
    /// padding, gap and click handlers.
    fn workbench_tree_row(self, selected: bool) -> Self;
    fn workbench_separator(self, edge: DividerEdge) -> Self;
    fn workbench_control_frame(self, error: bool) -> Self;
    fn workbench_control_surface(self, active: bool) -> Self;
    fn workbench_group_frame(self, first: bool, last: bool, highlighted: bool) -> Self;
    fn workbench_reveal_on_group_hover(self, group: impl Into<SharedString>, opacity: f32) -> Self;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DividerEdge {
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DividerTone {
    Standard,
    Subtle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeycapSize {
    Standard,
    Compact,
    Tag,
}

/// Keyboard shortcut label surface shared by settings and help panels.
pub fn keycap(label: impl Into<SharedString>, size: KeycapSize) -> Div {
    let width = match size {
        KeycapSize::Standard => theme::KEYCAP_WIDTH_STANDARD,
        KeycapSize::Compact => theme::KEYCAP_WIDTH_COMPACT,
        KeycapSize::Tag => theme::KEYCAP_WIDTH_TAG,
    };
    let height = if size == KeycapSize::Tag {
        theme::KEYCAP_HEIGHT_TAG
    } else {
        theme::ui_height(theme::CONTROL_HEIGHT_STANDARD)
    };
    let frame = div()
        .w(px(width))
        .h(px(height))
        .flex_shrink_0()
        .h_flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::ROUND_SM))
        .border_1()
        .border_color(rgba(theme::hairline().to_rgba()))
        .bg(rgb(theme::surface_base()))
        .workbench_body()
        .child(label.into());
    review_card(frame, crate::geometry_audit::ComponentTokens::default())
}

impl<T: Styled + InteractiveElement + ParentElement + FluentBuilder> WorkbenchStyle for T {
    #[track_caller]
    fn workbench_region(self, sidebar: bool) -> Self {
        let background = if sidebar {
            theme::surface_sidebar()
        } else {
            theme::surface_panel()
        };
        self.rounded(px(theme::ROUND_LG))
            .border_1()
            .border_color(rgba(theme::separator_light()))
            .bg(rgb(background))
            .shadow(theme::region_shadow())
            .when(theme::geometry_audit_enabled(), |style: Self| {
                observe_layout(
                    style,
                    "region",
                    crate::geometry_audit::ComponentTokens::default(),
                )
            })
    }

    #[track_caller]
    fn workbench_section(self) -> Self {
        self.bg(rgb(theme::surface_panel()))
            .rounded(px(theme::ROUND_LG))
            .border_1()
            .border_color(rgba(theme::separator_light()))
            .shadow_none()
            .min_h_0()
            .when(theme::geometry_audit_enabled(), |style: Self| {
                observe_layout(
                    style,
                    "section",
                    crate::geometry_audit::ComponentTokens::default(),
                )
            })
    }

    #[track_caller]
    fn workbench_card(self, selected: bool) -> Self {
        apply_card_surface(self, selected).hover(move |style| {
            style.bg(rgb(if selected {
                theme::selection_color()
            } else {
                theme::surface_hover()
            }))
        })
    }

    fn workbench_media_card(self) -> Self {
        self.shadow(theme::card_shadow())
    }

    #[track_caller]
    fn workbench_nav(self, selected: bool) -> Self {
        self.h_flex()
            .items_center()
            .h(px(theme::nav_row_height()))
            .px(px(theme::space_expanded()))
            .gap(px(theme::space_expanded()))
            .rounded(px(theme::ROUND_MD))
            .cursor_pointer()
            .bg(rgb(if selected {
                theme::selection_color()
            } else {
                theme::surface_panel()
            }))
            .text_color(rgb(if selected {
                theme::selection_text()
            } else {
                theme::text_secondary()
            }))
            .hover(move |style| {
                style.bg(rgb(if selected {
                    theme::selection_color()
                } else {
                    theme::surface_hover()
                }))
            })
            .when(theme::geometry_audit_enabled(), |style: Self| {
                observe_layout(
                    style,
                    "navigation",
                    crate::geometry_audit::ComponentTokens {
                        height: Some(theme::nav_row_height()),
                        gap: Some(theme::space_expanded()),
                        padding_x: Some(theme::space_expanded()),
                        ..Default::default()
                    },
                )
            })
    }

    #[track_caller]
    fn workbench_hover_surface(self) -> Self {
        review_card(
            self.hover(|style| style.bg(rgb(theme::surface_hover()))),
            crate::geometry_audit::ComponentTokens::default(),
        )
    }

    fn workbench_hover_accent_surface(self) -> Self {
        self.hover(|style| style.bg(rgba((theme::accent() << 8) | 42)))
    }

    fn workbench_hover_accent_border(self) -> Self {
        self.hover(|style| style.border_color(rgb(theme::accent())))
    }

    #[track_caller]
    fn workbench_selectable_surface(self, selected: bool) -> Self {
        review_row(
            self.when(selected, |style| {
                style
                    .bg(rgb(theme::selection_color()))
                    .text_color(rgb(theme::selection_text()))
            })
            .hover(move |style| {
                style.bg(if selected {
                    rgb(theme::selection_color())
                } else {
                    rgba(theme::surface_hover())
                })
            }),
            crate::geometry_audit::ComponentTokens::default(),
        )
    }

    fn workbench_hover_foreground(self, foreground: u32) -> Self {
        self.hover(move |style| style.text_color(rgb(foreground)))
    }

    #[track_caller]
    fn workbench_hover_group_header(self) -> Self {
        self.hover(|style| style.bg(rgb(theme::surface_group_header_hover())))
    }

    #[track_caller]
    fn workbench_tree_row(self, selected: bool) -> Self {
        let row = self
            .relative()
            .h_flex()
            .items_center()
            .w_full()
            .rounded(px(theme::ROUND_MD))
            .cursor_pointer()
            .text_color(rgb(if selected {
                theme::selection_text()
            } else {
                theme::text_secondary()
            }))
            .when(selected, |row: Self| {
                row.bg(rgb(theme::selection_color())).child(
                    div()
                        .absolute()
                        .left(px(2.))
                        .top(px(4.))
                        .bottom(px(4.))
                        .w(px(theme::SELECTION_BAR_WIDTH))
                        .rounded(px(theme::SELECTION_BAR_WIDTH / 2.))
                        .bg(rgb(theme::accent())),
                )
            });
        let row = if selected {
            row.hover(move |style| style.bg(rgb(theme::selection_color())))
        } else {
            row.hover(move |style| style.bg(rgb(theme::surface_hover())))
        };
        if theme::geometry_audit_enabled() {
            observe_layout(
                row,
                "tree-row",
                crate::geometry_audit::ComponentTokens::default(),
            )
        } else {
            row
        }
    }

    fn workbench_separator(self, edge: DividerEdge) -> Self {
        let line = match edge {
            DividerEdge::Top => self.border_t_1(),
            DividerEdge::Right => self.border_r_1(),
            DividerEdge::Bottom => self.border_b_1(),
            DividerEdge::Left => self.border_l_1(),
        };
        line.border_color(rgba(theme::separator_light()))
    }

    fn workbench_control_frame(self, error: bool) -> Self {
        self.rounded(px(theme::ROUND_SM))
            .border_1()
            .border_color(rgb(if error {
                theme::danger()
            } else {
                theme::control_box()
            }))
    }

    fn workbench_control_surface(self, active: bool) -> Self {
        self.rounded(px(theme::ROUND_SM))
            .border_1()
            .border_color(rgb(if active {
                theme::accent()
            } else {
                theme::region_edge()
            }))
            .bg(rgb(theme::surface_elevated()))
    }

    fn workbench_group_frame(self, first: bool, last: bool, highlighted: bool) -> Self {
        self.border_l_1()
            .border_r_1()
            .when(first, |edge| {
                edge.border_t_1().rounded_t(px(theme::ROUND_SM))
            })
            .when(last, |edge| {
                edge.border_b_1().rounded_b(px(theme::ROUND_SM))
            })
            .border_color(rgb(if highlighted {
                theme::accent_hover()
            } else {
                theme::separator()
            }))
    }

    fn workbench_reveal_on_group_hover(self, group: impl Into<SharedString>, opacity: f32) -> Self {
        self.opacity(opacity)
            .group_hover(group, |style| style.opacity(1.))
    }
}

pub fn drop_target_style<T: Styled>(style: T) -> T {
    style.border_color(rgb(theme::accent_hover()))
}

/// Shared card frame for plugin content. The caller provides the semantic
/// selected state and body; geometry, stroke, fill, text and audit treatment
/// come from the workbench component layer.
#[track_caller]
pub fn card_surface<T: Styled + ParentElement>(content: T, selected: bool) -> T {
    apply_card_surface(content, selected)
}

/// Compact rounded label for categories, tags and state chips.
pub fn badge(content: Div, selected: bool) -> Div {
    let badge = content
        .h_flex()
        .items_center()
        .px(px(theme::space_compact()))
        .py(px(theme::space_compact()))
        .rounded(px(theme::ROUND_PILL))
        .border_1()
        .border_color(rgb(if selected {
            theme::accent()
        } else {
            theme::separator()
        }))
        .bg(rgb(if selected {
            theme::selection_color()
        } else {
            theme::surface_panel()
        }))
        .text_color(rgb(if selected {
            theme::selection_text()
        } else {
            theme::text_secondary()
        }));
    review_card(badge, crate::geometry_audit::ComponentTokens::default())
}

pub fn colored_badge(content: Div, color: u32) -> Div {
    review_card(
        content
            .h_flex()
            .items_center()
            .px(px(theme::space_compact()))
            .py(px(theme::space_compact()))
            .rounded(px(theme::ROUND_PILL))
            .border_1()
            .border_color(rgba((color << 8) | 0x90))
            .bg(rgb(theme::surface_faint())),
        crate::geometry_audit::ComponentTokens::default(),
    )
}

/// Compact status marker; plugins provide the semantic tone, while size and
/// circular treatment remain common.
pub fn status_dot(size: f32, tone: StatusTone) -> Div {
    let fill = match tone {
        StatusTone::Neutral => SurfaceRole::Separator,
        StatusTone::Accent => SurfaceRole::Accent,
        StatusTone::Success => SurfaceRole::Success,
        StatusTone::Warning => SurfaceRole::Warning,
        StatusTone::Danger => SurfaceRole::Danger,
        StatusTone::Custom(color) => SurfaceRole::Custom(color),
    };
    review_card(
        div()
            .size(px(size))
            .workbench_corner(CornerRole::Pill)
            .workbench_surface(fill),
        crate::geometry_audit::ComponentTokens::default(),
    )
}

/// Theme-owned dotted guides for a nested project tree. The caller supplies
/// positions only; every node uses the same guide color and diameter.
pub fn tree_guide_dots(points: impl IntoIterator<Item = (f32, f32)>, size: f32) -> Vec<AnyElement> {
    points
        .into_iter()
        .map(|(left, top)| {
            review_row(
                div()
                    .absolute()
                    .left(px(left))
                    .top(px(top))
                    .size(px(size))
                    .workbench_corner(CornerRole::Pill)
                    .workbench_surface(SurfaceRole::Transparent(theme::hairline().to_rgba())),
                crate::geometry_audit::ComponentTokens::default(),
            )
            .into_any_element()
        })
        .collect()
}

/// Shared media-card overlay badge used for ratings, media types and concise
/// state labels. The feature owns only its content and placement.
pub fn media_badge(content: Div) -> Div {
    review_card(
        content
            .h_flex()
            .items_center()
            .h(px(theme::MEDIA_BADGE_HEIGHT))
            .px(px(theme::space_standard()))
            .workbench_corner(CornerRole::Pill)
            .workbench_surface(SurfaceRole::MediaBadge)
            .workbench_foreground(ForegroundRole::OnMediaBadge)
            .workbench_assistant(),
        crate::geometry_audit::ComponentTokens::default(),
    )
}

/// Shared FileManager thumbnail surface. Features add the actual image and
/// content; the neutral clipped frame is fixed by the common component layer.
pub fn thumbnail_frame() -> Div {
    review_card(
        div()
            .relative()
            .absolute()
            .inset_0()
            .overflow_hidden()
            .workbench_corner(CornerRole::Thumbnail)
            .workbench_surface(SurfaceRole::Faint),
        crate::geometry_audit::ComponentTokens::default(),
    )
}

/// Selection-independent hover shade for a media thumbnail.
pub fn media_hover_overlay(amount: f32) -> Div {
    review_card(
        div()
            .absolute()
            .inset_0()
            .workbench_surface(SurfaceRole::Transparent(
                theme::fill_transparent_overlay() | (18. * amount.clamp(0., 1.)) as u32,
            )),
        crate::geometry_audit::ComponentTokens::default(),
    )
}

/// Semantic status capsule for short feedback text.
pub fn status_badge(content: Div, tone: StatusTone) -> Div {
    let (fill, foreground) = match tone {
        StatusTone::Neutral => (SurfaceRole::Faint, ForegroundRole::Secondary),
        StatusTone::Accent => (SurfaceRole::Accent, ForegroundRole::AccentText),
        StatusTone::Success => (SurfaceRole::Success, ForegroundRole::OnSuccess),
        StatusTone::Warning => (SurfaceRole::Warning, ForegroundRole::OnWarning),
        StatusTone::Danger => (SurfaceRole::Danger, ForegroundRole::OnDanger),
        StatusTone::Custom(color) => (SurfaceRole::Custom(color), ForegroundRole::OnMediaBadge),
    };
    review_card(
        content
            .h_flex()
            .items_center()
            .px(px(theme::space_standard()))
            .workbench_corner(CornerRole::Small)
            .workbench_surface(fill)
            .workbench_foreground(foreground)
            .workbench_assistant(),
        crate::geometry_audit::ComponentTokens::default(),
    )
}

/// Compact floating status message frame. Features provide its text and
/// placement; the shared layer owns its panel surface, padding and radius.
pub fn status_toast(content: Div) -> Div {
    review_section(
        content
            .p(px(theme::space_standard()))
            .workbench_corner(CornerRole::Medium)
            .workbench_surface(SurfaceRole::Panel),
        crate::geometry_audit::ComponentTokens::default(),
    )
}

/// Shared action menu surface for feature-owned menu contents.
pub fn menu_panel() -> Div {
    review_section(
        div()
            .v_flex()
            .gap(px(theme::space_compact()))
            .p(px(theme::space_standard()))
            .bg(rgb(theme::surface_panel()))
            .rounded(px(theme::ROUND_LG))
            .border_1()
            .border_color(rgba(theme::separator_light()))
            .shadow(theme::floating_shadow()),
        crate::geometry_audit::ComponentTokens::default(),
    )
}

/// Centered status content over an unavailable thumbnail.
pub fn thumbnail_message_overlay(content: Div) -> Div {
    review_card(
        content
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .workbench_surface(SurfaceRole::CardOverlay)
            .workbench_assistant(),
        crate::geometry_audit::ComponentTokens::default(),
    )
}

/// Theme-owned vertical timeline rail; callers provide only its placement.
pub fn timeline_rail(color: u32) -> Div {
    review_row(
        div()
            .absolute()
            .w(px(theme::TIMELINE_RAIL_WIDTH))
            .workbench_surface(SurfaceRole::Custom(color)),
        crate::geometry_audit::ComponentTokens::default(),
    )
}

/// Shared data-colored file-format capsule.
pub fn file_format_badge(content: Div, background: u32, foreground: u32) -> Div {
    review_card(
        content
            .h_flex()
            .items_center()
            .px(px(theme::space_compact()))
            .workbench_corner(CornerRole::Micro)
            .workbench_surface(SurfaceRole::Custom(background))
            .workbench_foreground(ForegroundRole::Custom(foreground))
            .workbench_assistant(),
        crate::geometry_audit::ComponentTokens::default(),
    )
}

pub fn timeline_marker(left: f32, top: f32, size: f32, color: u32, selected: bool) -> Div {
    div()
        .absolute()
        .left(px(left))
        .top(px(top))
        .size(px(size))
        .rounded(px(theme::ROUND_PILL))
        .border_2()
        .border_color(rgb(color))
        .bg(rgb(if selected {
            color
        } else {
            theme::surface_card()
        }))
}

pub fn timeline_connector(left: f32, top: f32, width: f32, height: f32, color: u32) -> Div {
    div()
        .absolute()
        .left(px(left))
        .top(px(top))
        .w(px(width))
        .h(px(height))
        .border_l_2()
        .border_b_2()
        .border_color(rgb(color))
        .rounded_bl(px(theme::TIMELINE_CORNER_RADIUS))
}

/// Theme-owned range-slider rail and thumbs. Feature code supplies only the
/// current value positions and active state; it owns input capture and value
/// updates, never the painted control geometry.
pub fn range_slider_visual(
    track_left: f32,
    track_right: f32,
    low: f32,
    high: f32,
    active: bool,
    show_low_thumb: bool,
) -> Div {
    fn thumb(x: f32) -> Div {
        div()
            .absolute()
            .left(px(x - theme::SLIDER_THUMB_SIZE / 2.))
            .top(px(theme::SLIDER_THUMB_TOP))
            .size(px(theme::SLIDER_THUMB_SIZE))
            .rounded_full()
            .bg(rgba(theme::track_shadow()))
            .child(
                div()
                    .absolute()
                    .inset(px(theme::SLIDER_THUMB_RING))
                    .rounded_full()
                    .bg(rgb(theme::surface_elevated())),
            )
            .child(
                div()
                    .absolute()
                    .inset(px(theme::SLIDER_THUMB_DOT))
                    .rounded_full()
                    .bg(rgb(theme::accent())),
            )
    }
    let rail = |left: f32, right: f32, color: Hsla| {
        div()
            .absolute()
            .top(px(theme::SLIDER_RAIL_TOP))
            .left(px(left))
            .w(px((right - left).max(0.)))
            .h(px(theme::SLIDER_RAIL_HEIGHT))
            .rounded_full()
            .bg(color)
    };
    let mut visual = div()
        .absolute()
        .inset_0()
        .child(rail(
            track_left,
            track_right,
            rgba(theme::track_highlight()).into(),
        ))
        .child(rail(
            low,
            high,
            if active {
                rgb(theme::accent()).into()
            } else {
                rgba(theme::track_active()).into()
            },
        ))
        .child(thumb(high));
    if show_low_thumb {
        visual = visual.child(thumb(low));
    }
    review_card(visual, crate::geometry_audit::ComponentTokens::default())
}

pub fn accent_card(content: Div, color: u32, selected: bool) -> Div {
    review_card(
        content
            .rounded(px(theme::ROUND_LG))
            .border_1()
            .border_color(if selected {
                rgb(color)
            } else {
                rgba(theme::separator())
            })
            .bg(rgba(if selected { (color << 8) | 0x18 } else { 0 })),
        crate::geometry_audit::ComponentTokens::default(),
    )
}

#[track_caller]
fn apply_card_surface<T: Styled + ParentElement>(content: T, selected: bool) -> T {
    let card = content
        .rounded(px(theme::ROUND_LG))
        .border_1()
        .border_color(rgb(if selected {
            theme::accent()
        } else {
            theme::separator()
        }))
        .bg(rgb(if selected {
            theme::selection_color()
        } else {
            theme::surface_panel()
        }))
        .shadow_none()
        .text_color(rgb(if selected {
            theme::selection_text()
        } else {
            theme::text_primary()
        }));
    review_card(card, crate::geometry_audit::ComponentTokens::default())
}

pub fn divider() -> Div {
    review_section(
        div()
            .w_full()
            .h(px(theme::DIVIDER_THICKNESS))
            .flex_shrink_0()
            .child(Separator::horizontal().color(rgb(theme::separator()))),
        crate::geometry_audit::ComponentTokens::default(),
    )
}

pub fn subtle_divider() -> Div {
    review_section(
        div()
            .w_full()
            .h(px(theme::DIVIDER_THICKNESS))
            .flex_shrink_0()
            .child(Separator::horizontal().color(rgba(theme::separator_light()))),
        crate::geometry_audit::ComponentTokens::default(),
    )
}

pub fn vertical_divider() -> Div {
    review_section(
        div()
            .w(px(theme::DIVIDER_THICKNESS))
            .h_full()
            .flex_shrink_0()
            .child(Separator::vertical().color(rgb(theme::separator()))),
        crate::geometry_audit::ComponentTokens::default(),
    )
}

/// Theme-owned separator for compact or positioned layouts. The caller passes
/// only its required layout extent; tint and painting remain shared.
pub fn separator_line(width: f32, height: f32, tone: DividerTone) -> Div {
    let color = match tone {
        DividerTone::Standard => theme::separator(),
        DividerTone::Subtle => theme::separator_light(),
    };
    let horizontal = width >= height;
    let line = if horizontal {
        Separator::horizontal().color(rgba(color))
    } else {
        Separator::vertical().color(rgba(color))
    };
    review_section(
        div().w(px(width)).h(px(height)).flex_shrink_0().child(line),
        crate::geometry_audit::ComponentTokens::default(),
    )
}

#[track_caller]
fn observe_layout<T: Styled + ParentElement>(
    element: T,
    tier: &'static str,
    tokens: crate::geometry_audit::ComponentTokens,
) -> T {
    let location = std::panic::Location::caller();
    let source_file = location.file();
    let source_line = location.line();
    element.child(
        canvas(
            move |bounds, window, _| {
                if theme::geometry_audit_enabled() {
                    crate::geometry_audit::record(
                        bounds,
                        window,
                        tier,
                        source_file,
                        source_line,
                        None,
                        tokens,
                    );
                }
            },
            move |bounds, _, window, _| {
                paint_audit_outline(bounds, audit_color(tier), 2., window);
            },
        )
        .absolute()
        .inset_0()
        .size_full(),
    )
}

fn audit_color(tier: &str) -> u32 {
    match tier {
        "region" | "section" => theme::AUDIT_FRAME_SECTION,
        "card" => theme::AUDIT_FRAME_CARD,
        "navigation" | "tree-row" | "row" => theme::AUDIT_FRAME_NAV,
        "button" => theme::AUDIT_FRAME_BUTTON,
        "input" => theme::AUDIT_FRAME_INPUT,
        _ => theme::AUDIT_FRAME_TEXT,
    }
}

fn paint_audit_outline(bounds: Bounds<Pixels>, color: u32, width: f32, window: &mut Window) {
    if theme::geometry_audit_enabled() {
        window.paint_quad(quad(
            bounds,
            px(0.),
            transparent_black(),
            px(width),
            rgb(color),
            BorderStyle::default(),
        ));
    }
}

pub(super) fn audit_overlay(color: u32, width: f32) -> Canvas<()> {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            paint_audit_outline(bounds, color, width, window);
        },
    )
    .absolute()
    .inset_0()
    .size_full()
}

/// Draw and record a shared section boundary during geometry review while
/// preserving the feature component's existing layout and visual styling.
#[track_caller]
pub fn review_section<T: Styled + ParentElement>(
    element: T,
    tokens: crate::geometry_audit::ComponentTokens,
) -> T {
    if theme::geometry_audit_enabled() {
        observe_layout(element, "section", tokens)
    } else {
        element
    }
}

/// Draw and record a shared card boundary during geometry review without
/// adding a product border to content cards.
#[track_caller]
pub fn review_card<T: Styled + ParentElement>(
    element: T,
    tokens: crate::geometry_audit::ComponentTokens,
) -> T {
    if theme::geometry_audit_enabled() {
        observe_layout(element, "card", tokens)
    } else {
        element
    }
}

/// Mark a reusable row boundary during geometry review without changing the
/// row's product colors, hit target, or layout.
#[track_caller]
pub fn review_row<T: Styled + ParentElement>(
    element: T,
    tokens: crate::geometry_audit::ComponentTokens,
) -> T {
    if theme::geometry_audit_enabled() {
        observe_layout(element, "row", tokens)
    } else {
        element
    }
}

/// Common entry for every product button. Native variants own every interaction
/// color; product plugins supply semantics, content, order and callbacks only.
/// Native size variants remain available for compact and dense toolbars.
#[track_caller]
pub fn button(id: impl Into<ElementId>) -> Button {
    let id = id.into();
    let button = Button::new(id.clone())
        .rounded(px(theme::ROUND_MD))
        .h(px(theme::button_height()))
        .min_w(px(theme::BUTTON_MIN_WIDTH))
        .font_weight(FontWeight::NORMAL)
        .text_size(px(theme::FONT_SIZE_BODY))
        .line_height(px(theme::LINE_HEIGHT_BODY))
        .text_color(rgb(theme::text_primary()))
        .shadow_none();
    if theme::geometry_audit_enabled() {
        let caller = std::panic::Location::caller();
        let file = caller.file();
        let line = caller.line();
        let component_id = format!("{id:?}");
        button.relative().child(
            canvas(
                move |bounds, window, _| {
                    crate::geometry_audit::record(
                        bounds,
                        window,
                        "button",
                        file,
                        line,
                        Some(&component_id),
                        crate::geometry_audit::ComponentTokens {
                            height: Some(theme::button_height()),
                            min_width: Some(theme::BUTTON_MIN_WIDTH),
                            ..Default::default()
                        },
                    );
                },
                move |bounds, _, window, _| {
                    paint_audit_outline(bounds, theme::AUDIT_FRAME_BUTTON, 2., window);
                },
            )
            .absolute()
            .inset_0()
            .size_full(),
        )
    } else {
        button
    }
}

pub fn section() -> Div {
    div().workbench_section()
}

/// Apply the shared fixed height and minimum width after a native size/variant
/// has been selected. GPUI Kit variants may otherwise replace constructor size.
pub fn fixed_button_geometry(button: Button) -> Button {
    button
        .h(px(theme::button_height()))
        .max_h(px(theme::button_height()))
        .min_w(px(theme::BUTTON_MIN_WIDTH))
}

/// A selectable settings choice with optional supporting text. This is a card
/// rather than a size-variable button, so content can set its height naturally.
pub fn choice_card(selected: bool, disabled: bool) -> Div {
    let card = div()
        .h_flex()
        .items_center()
        .w_full()
        .gap(px(theme::space_standard()))
        .p(px(theme::space_standard()))
        .rounded(px(theme::ROUND_LG))
        .border_1()
        .border_color(rgb(if selected {
            theme::accent()
        } else {
            theme::separator()
        }))
        .bg(rgb(if selected {
            theme::selection_color()
        } else {
            theme::surface_panel()
        }))
        .shadow_none()
        .text_color(rgb(if selected {
            theme::selection_text()
        } else {
            theme::text_primary()
        }));
    let card = if disabled {
        card.opacity(0.6)
    } else {
        card.cursor_pointer().hover(move |style| {
            style.bg(rgb(if selected {
                theme::selection_color()
            } else {
                theme::surface_hover()
            }))
        })
    };
    if theme::geometry_audit_enabled() {
        observe_layout(
            card,
            "card",
            crate::geometry_audit::ComponentTokens {
                gap: Some(theme::space_standard()),
                padding_x: Some(theme::space_standard()),
                padding_y: Some(theme::space_standard()),
                ..Default::default()
            },
        )
    } else {
        card
    }
}

/// Standard large action, including Kit-owned focus/hover/pressed/disabled states.
/// Use `.primary()`/`.danger()` on ordinary Kit buttons for other sizes.
#[track_caller]
pub fn action(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    primary: bool,
) -> Button {
    let button = button(ElementId::Name(id.into()))
        .label(label)
        .rounded(px(theme::ROUND_MD))
        .shadow_none();
    fixed_button_geometry(if primary { button.primary() } else { button })
}

#[track_caller]
pub fn icon_button(id: impl Into<SharedString>, name: gpui_kit::assets::IconName) -> Button {
    fixed_button_geometry(
        button(ElementId::Name(id.into()))
            .ghost()
            .icon(theme::icon(name))
            .shadow_none(),
    )
}

/// A user/data color choice. The Bridge owns its hit target, selected stroke,
/// foreground contrast, corner and audit frame; callers supply only the color.
pub fn color_swatch(id: impl Into<SharedString>, color: u32, selected: bool) -> Button {
    let swatch = button(ElementId::Name(id.into()))
        .h(px(theme::ui_height(theme::CONTROL_HEIGHT_STANDARD)))
        .min_w(px(theme::ui_height(theme::CONTROL_HEIGHT_STANDARD)))
        .rounded(px(theme::ROUND_MD))
        .border_2()
        .border_color(rgb(if selected {
            theme::text_primary()
        } else {
            theme::surface_panel()
        }))
        .bg(rgb(color))
        .text_color(rgb(theme::contrasting_text(color)))
        .shadow_none();
    let swatch = if selected {
        swatch.icon(theme::small_icon(gpui_kit::assets::IconName::Check))
    } else {
        swatch
    };
    if theme::geometry_audit_enabled() {
        swatch
            .relative()
            .child(audit_overlay(theme::AUDIT_FRAME_BUTTON, 2.))
    } else {
        swatch
    }
}

/// Standard icon-only choice control for selectors and preference panels.
pub fn icon_choice_button(
    id: impl Into<SharedString>,
    name: gpui_kit::assets::IconName,
    selected: bool,
) -> Button {
    let choice = button(ElementId::Name(id.into()))
        .ghost()
        .rounded(px(theme::ROUND_SM))
        .border_1()
        .border_color(rgb(if selected {
            theme::accent()
        } else {
            theme::separator()
        }))
        .bg(if selected {
            rgb(theme::selection_color())
        } else {
            rgba(theme::fill_subtle())
        })
        .text_color(rgb(if selected {
            theme::selection_text()
        } else {
            theme::text_primary()
        }))
        .icon(theme::small_icon(name))
        .toggled(selected)
        .shadow_none();
    if theme::geometry_audit_enabled() {
        choice
            .relative()
            .child(audit_overlay(theme::AUDIT_FRAME_BUTTON, 2.))
    } else {
        choice
    }
}

/// Entry point for the Kit-managed popover layer. Placement, open state and
/// body are content behavior; callers never implement a separate popup frame.
pub fn popover(id: impl Into<ElementId>) -> Popover {
    Popover::new(id)
}

pub fn checkbox(id: impl Into<ElementId>, checked: bool) -> Checkbox {
    Checkbox::new(id).checked(checked)
}

pub fn radio(id: impl Into<ElementId>, checked: bool) -> Radio {
    Radio::new(id).checked(checked)
}

pub fn date_picker(state: &Entity<DatePickerState>) -> DatePicker {
    DatePicker::new(state)
}

/// Underlined tab button with shared selected and inactive states.
pub fn tab_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    selected: bool,
) -> Button {
    button(ElementId::Name(id.into()))
        .ghost()
        .flex_1()
        .border_b_2()
        .border_color(rgb(if selected {
            theme::accent()
        } else {
            theme::surface_panel()
        }))
        .child(
            div()
                .workbench_body()
                .h_flex()
                .justify_center()
                .child(label.into()),
        )
        .toggled(selected)
        .shadow_none()
}

/// Theme-owned neutral artboard frame around feature-provided canvas content.
pub fn artboard<T: Styled + ParentElement>(content: T) -> T {
    review_section(
        content
            .relative()
            .border_1()
            .border_color(rgba(theme::separator_light()))
            .bg(rgb(theme::surface_card())),
        crate::geometry_audit::ComponentTokens::default(),
    )
}

/// Shared visual selection frame and resize handles. The feature supplies the
/// measured rectangle; stroke, overlay fill, handle size and handle surfaces
/// remain owned by the theme component layer.
pub fn selection_frame(left: f32, top: f32, width: f32, height: f32) -> Div {
    let handle = theme::CANVAS_SELECTION_HANDLE_SIZE;
    let centers = [
        (left, top),
        (left + width / 2., top),
        (left + width, top),
        (left, top + height / 2.),
        (left + width, top + height / 2.),
        (left, top + height),
        (left + width / 2., top + height),
        (left + width, top + height),
    ];
    let frame = selection_region(left, top, width, height);
    let frame = frame.children(centers.into_iter().map(|(x, y)| {
        div()
            .absolute()
            .left(px(x - handle / 2.))
            .top(px(y - handle / 2.))
            .w(px(handle))
            .h(px(handle))
            .border_1()
            .border_color(rgb(theme::accent()))
            .bg(rgb(theme::surface_elevated()))
    }));
    frame
}

/// Shared rectangle used for drag-selection regions in list and canvas views.
pub fn selection_region(left: f32, top: f32, width: f32, height: f32) -> Div {
    let region = div()
        .absolute()
        .left(px(left))
        .top(px(top))
        .w(px(width))
        .h(px(height))
        .border_1()
        .border_color(rgb(theme::accent()))
        .bg(rgba(theme::fill_transparent_overlay() | 24));
    review_section(region, crate::geometry_audit::ComponentTokens::default())
}

/// File-card selection progress and keyboard focus treatment.
pub fn media_selection_overlay(size: f32, selection: f32, keyboard_focus: bool) -> Div {
    let mut overlay = div().absolute().inset_0().rounded(px(theme::THUMB_ROUND));
    if selection > 0.01 {
        overlay = overlay
            .border_2()
            .border_color(rgba((theme::accent() << 8) | (242. * selection) as u32));
    }
    if keyboard_focus && selection > 0.5 {
        overlay = overlay.child(
            div()
                .absolute()
                .top(px(-1.))
                .left(px(-1.))
                .right(px(-1.))
                .bottom(px(-1.))
                .w(px(size + 2.))
                .rounded(px(theme::THUMB_ROUND + 1.))
                .border_1()
                .border_color(rgba((theme::accent() << 8) | 0x80)),
        );
    }
    review_section(overlay, crate::geometry_audit::ComponentTokens::default())
}

/// Shared bottom controls for the workspace rail (theme mode and Settings).
/// Plugins provide the controls and their behavior; the host owns the frame.
pub fn rail_footer(theme_control: Button, settings_control: Button) -> Div {
    let footer = div()
        .v_flex()
        .w(px(theme::RAIL_FOOTER_WIDTH))
        .h(px(theme::ui_height(theme::RAIL_FOOTER_HEIGHT)))
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .gap(px(theme::space_compact()))
        .bg(rgb(theme::surface_rail()))
        .child(theme_control)
        .child(settings_control);
    review_section(
        footer,
        crate::geometry_audit::ComponentTokens {
            height: Some(theme::ui_height(theme::RAIL_FOOTER_HEIGHT)),
            gap: Some(theme::space_compact()),
            ..Default::default()
        },
    )
}

/// Shared visual shell for in-workspace dialogs. Feature plugins provide only
/// dialog content and size; surface, stroke, corner and audit treatment stay
/// owned by the UI Bridge.
pub fn modal_frame(content: Div) -> Div {
    let frame = content
        .v_flex()
        .rounded(px(theme::ROUND_LG))
        .border_1()
        .border_color(rgba(theme::separator_light()))
        .bg(rgb(theme::surface_modal()))
        .shadow(theme::floating_shadow());
    review_section(frame, crate::geometry_audit::ComponentTokens::default())
}

/// Shared modal scrim and centered placement. Plugins must not paint their own
/// modal backdrop or position a separate dialog frame.
pub fn modal_overlay(content: Div) -> Div {
    div()
        .absolute()
        .inset_0()
        .h_flex()
        .items_center()
        .justify_center()
        .child(div().absolute().inset_0().bg(rgba(theme::scrim())))
        .child(modal_frame(content))
}

/// Shared vertical shortcut used by plugins in the main navigation rail.
/// Callers provide only their identity, label and icon; hover, typography,
/// spacing and footprint stay identical across every plugin contribution.
pub fn rail_shortcut(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    icon: impl IntoElement,
    selected: bool,
) -> Stateful<Div> {
    let shortcut = div()
        .id(id.into())
        .v_flex()
        .w(px(theme::RAIL_SHORTCUT_WIDTH))
        .h(px(theme::ui_height(theme::RAIL_SHORTCUT_HEIGHT)))
        .items_center()
        .justify_center()
        .gap(px(theme::space_compact()))
        .rounded(px(theme::ROUND_MD))
        .cursor_pointer()
        .workbench_assistant()
        .when(selected, |style| {
            style
                .bg(rgb(theme::selection_color()))
                .text_color(rgb(theme::selection_text()))
        })
        .hover(move |style| {
            style.bg(if selected {
                rgb(theme::selection_color())
            } else {
                rgba(theme::surface_hover())
            })
        })
        .child(icon)
        .child(
            div()
                .workbench_assistant()
                .when(selected, |style| {
                    style.text_color(rgb(theme::selection_text()))
                })
                .child(label.into()),
        );
    observe_layout(
        shortcut,
        "navigation",
        crate::geometry_audit::ComponentTokens {
            height: Some(theme::ui_height(theme::RAIL_SHORTCUT_HEIGHT)),
            ..Default::default()
        },
    )
    .when(theme::geometry_audit_enabled(), |style| {
        style
            .relative()
            .child(audit_overlay(theme::AUDIT_FRAME_NAV, 2.))
    })
}

pub fn section_heading(label: impl Into<SharedString>) -> Div {
    review_section(
        div()
            .workbench_title()
            .pt(px(theme::space_expanded()))
            .pb(px(theme::space_standard()))
            .child(label.into()),
        crate::geometry_audit::ComponentTokens {
            padding_y: Some(theme::space_expanded()),
            ..Default::default()
        },
    )
}

/// Shared title/description block with a consistent title-to-assistant gap.
pub fn heading_block(title: impl Into<SharedString>, description: impl Into<SharedString>) -> Div {
    review_section(
        div()
            .v_flex()
            .gap(px(theme::space_standard()))
            .child(div().workbench_title().child(title.into()))
            .child(div().workbench_assistant().child(description.into())),
        crate::geometry_audit::ComponentTokens {
            gap: Some(theme::space_standard()),
            ..Default::default()
        },
    )
}

/// Search fields share native focus/disabled states instead of custom outer paints.
pub fn search_input(state: &Entity<InputState>) -> Div {
    let input = Input::new(state)
        .h(px(theme::ui_height(theme::CONTROL_HEIGHT_STANDARD)))
        .bg(rgb(theme::surface_faint()))
        .text_size(px(theme::FONT_SIZE_BODY))
        .line_height(px(theme::LINE_HEIGHT_BODY))
        .text_color(rgb(theme::text_primary()))
        .prefix(div().child(theme::icon(gpui_kit::assets::IconName::Search)))
        .cleanable(true);
    audited_input(input).w_full()
}

pub fn text_input(state: &Entity<InputState>) -> Div {
    text_input_with_disabled(state, false)
}

pub fn text_input_with_disabled(state: &Entity<InputState>, disabled: bool) -> Div {
    audited_input(
        Input::new(state)
            .h(px(theme::ui_height(theme::CONTROL_HEIGHT_STANDARD)))
            .bg(rgb(theme::surface_faint()))
            .text_size(px(theme::FONT_SIZE_BODY))
            .line_height(px(theme::LINE_HEIGHT_BODY))
            .text_color(rgb(theme::text_primary()))
            .disabled(disabled),
    )
}

pub fn textarea(state: &Entity<TextareaState>) -> Div {
    audited_input(
        Textarea::new(state)
            .bg(rgb(theme::surface_faint()))
            .text_size(px(theme::FONT_SIZE_BODY))
            .line_height(px(theme::LINE_HEIGHT_BODY)),
    )
}

fn audited_input(control: impl IntoElement) -> Div {
    let frame = div().relative().w_full().child(control);
    if theme::geometry_audit_enabled() {
        frame.child(audit_overlay(theme::AUDIT_FRAME_INPUT, 2.))
    } else {
        frame
    }
}

/// Compact Explorer/sidebar frame shared by drives, quick access and favorite groups.
pub fn sidebar_section(body: Div) -> Div {
    let section = theme::separate_region(
        body.v_flex()
            .w_full()
            .flex_shrink_0()
            .gap(px(theme::space_compact()))
            .p(px(theme::space_standard()))
            .rounded(px(theme::ROUND_MD))
            .bg(rgb(theme::surface_region())),
        theme::RegionEdge::All,
    );
    if theme::geometry_audit_enabled() {
        observe_layout(
            section,
            "section",
            crate::geometry_audit::ComponentTokens {
                gap: Some(theme::space_compact()),
                padding_x: Some(theme::space_standard()),
                padding_y: Some(theme::space_standard()),
                ..Default::default()
            },
        )
    } else {
        section
    }
}

pub fn sidebar_header(
    label: impl Into<SharedString>,
    toggle: Button,
    menu: impl IntoElement,
    trailing: Vec<Button>,
) -> Div {
    review_section(
        div()
            .h_flex()
            .items_center()
            .gap(px(theme::space_compact()))
            .pt(px(theme::space_standard()))
            .pb(px(theme::space_compact()))
            .child(toggle)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .workbench_title()
                    .text_ellipsis()
                    .child(label.into()),
            )
            .child(menu)
            .children(trailing),
        crate::geometry_audit::ComponentTokens {
            gap: Some(theme::space_compact()),
            padding_y: Some(theme::space_standard()),
            ..Default::default()
        },
    )
}
