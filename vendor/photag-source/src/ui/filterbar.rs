//! Search + filter controls. Lives inside the unified command bar at the
//! top of the window. Every change here flips `state.media_dirty` so the
//! gallery re-queries on the next frame — no Apply button needed.
//!
//! Visual language: one inset pill search field with a Fluent focus
//! underline, then a single quiet row of chips — no "Sort:"/"Type:" labels,
//! no separator bars. Active state is carried by soft accent fills.

use crate::app::{AppState, MediaTypeFilter, TagClickMode};
use crate::db::queries::SortOrder;
use crate::settings::ExcludeModifier;
use crate::theme;
use egui::{self, Color32, FontId, Rect, RichText, Sense, Stroke, Ui, Vec2};

/// Height of the search field.
const SEARCH_H: f32 = 32.0;
/// Height of filter chips.
const CHIP_H: f32 = 26.0;

// ══════════════════════════════════════════════════════════════════
//  Search
// ══════════════════════════════════════════════════════════════════

/// Centered search box: a rounded inset field with the magnifier inside,
/// an in-field clear button, and a bottom underline that animates to the
/// accent color on focus (the Fluent text-field signature).
pub fn show_search(ui: &mut Ui, state: &mut AppState, total_w: f32) {
    let (rect, container) = ui.allocate_exact_size(Vec2::new(total_w, SEARCH_H), Sense::click());
    let round = theme::rounding(SEARCH_H / 2.0);

    // Inner text area: leave room for the icon (left) and clear (right).
    let text_left = rect.min.x + 34.0;
    let text_right = rect.max.x - if state.search_query.is_empty() { 12.0 } else { 30.0 };
    let text_rect = Rect::from_min_max(
        egui::pos2(text_left, rect.min.y + 4.0),
        egui::pos2(text_right.max(text_left + 40.0), rect.max.y - 4.0),
    );

    // Field body — paint before the TextEdit so the text sits on top.
    let hovered = container.hovered();
    let field_fill = if hovered {
        Color32::from_rgba_premultiplied(20, 20, 20, 20)
    } else {
        Color32::from_rgba_premultiplied(15, 15, 15, 15)
    };
    // Fill only — no outline; the inset fill + focus underline carry the
    // field's shape.
    ui.painter().rect_filled(rect, round, field_fill);

    let resp = ui.put(
        text_rect,
        egui::TextEdit::singleline(&mut state.search_query)
            .hint_text(RichText::new("Search your library").color(theme::TEXT_MUTED))
            .frame(false)
            .vertical_align(egui::Align::Center)
            .font(egui::TextStyle::Body),
    );
    if resp.changed() { state.media_dirty = true; }
    if container.clicked() { resp.request_focus(); }
    if hovered { ui.ctx().set_cursor_icon(egui::CursorIcon::Text); }

    // Magnifier — brightens with focus.
    let focused = resp.has_focus();
    let focus_t = ui.ctx().animate_bool_with_time(resp.id.with("focus"), focused, 0.15);
    let icon_col = if focused { theme::TEXT_PRIMARY } else { theme::TEXT_SECONDARY };
    ui.painter().text(
        egui::pos2(rect.min.x + 14.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        if theme::icons_available() { theme::ICON_SEARCH } else { "🔍" },
        theme::icon_font(13.0),
        icon_col,
    );

    // In-field clear button.
    if !state.search_query.is_empty() {
        let clear_c = egui::pos2(rect.max.x - 17.0, rect.center().y);
        let clear_rect = Rect::from_center_size(clear_c, Vec2::splat(18.0));
        let clear_resp = ui.interact(clear_rect, resp.id.with("clear"), Sense::click());
        if clear_resp.hovered() {
            ui.painter().circle_filled(clear_c, 9.0, Color32::from_white_alpha(20));
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        ui.painter().text(
            clear_c,
            egui::Align2::CENTER_CENTER,
            if theme::icons_available() { theme::ICON_CLOSE } else { "✕" },
            theme::icon_font(9.0),
            theme::TEXT_SECONDARY,
        );
        if clear_resp.clicked() {
            state.search_query.clear();
            state.media_dirty = true;
        }
    }

    // Focus underline — grows from the center, fades to accent.
    if focus_t > 0.01 {
        let inset = 14.0 + (1.0 - focus_t) * (rect.width() * 0.5 - 14.0);
        let y = rect.max.y - 1.0;
        ui.painter().line_segment(
            [egui::pos2(rect.min.x + inset, y), egui::pos2(rect.max.x - inset, y)],
            Stroke::new(2.0, theme::accent_alpha(0.9 * focus_t)),
        );
    }

    // Scope-toggle popup. Visibility rules (per design):
    //   • Hidden while the query is empty.
    //   • Shown when the query is non-empty AND the field has focus.
    //   • Stays alive while the pointer is over the popup itself, so
    //     clicking checkboxes doesn't dismiss it via the focus-loss path.
    let popup_rect_id = egui::Id::new("search_scope_popup_rect");
    let last_popup_rect: Option<Rect> =
        ui.memory(|m| m.data.get_temp(popup_rect_id));
    let pointer_in_popup = last_popup_rect
        .and_then(|r| ui.ctx().input(|i| i.pointer.hover_pos()).map(|p| r.contains(p)))
        .unwrap_or(false);
    let show_popup =
        !state.search_query.is_empty() && (focused || pointer_in_popup);

    if show_popup {
        let anchor = egui::pos2(rect.min.x + 8.0, rect.max.y + 6.0);
        let area = egui::Area::new(egui::Id::new("search_scope_popup"))
            .fixed_pos(anchor)
            .order(egui::Order::Foreground)
            .interactable(true)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                    .show(ui, |ui| {
                        ui.set_min_width(150.0);
                        ui.label(
                            RichText::new("SEARCH IN")
                                .size(theme::FONT_SIZE_TINY)
                                .color(theme::TEXT_MUTED),
                        );
                        ui.add_space(2.0);
                        let mut s = state.search_scopes;
                        let mut changed = false;
                        changed |= ui.checkbox(&mut s.filenames, "Filenames").changed();
                        changed |= ui.checkbox(&mut s.paths, "Paths").changed();
                        changed |= ui.checkbox(&mut s.tags, "Tags").changed();
                        if changed {
                            state.search_scopes = s;
                            state.media_dirty = true;
                        }
                    });
            });
        ui.memory_mut(|m| m.data.insert_temp(popup_rect_id, area.response.rect));
    } else if last_popup_rect.is_some() {
        ui.memory_mut(|m| m.data.remove::<Rect>(popup_rect_id));
    }
}

// ══════════════════════════════════════════════════════════════════
//  Chip + segmented primitives
// ══════════════════════════════════════════════════════════════════

fn chip_font() -> FontId { FontId::proportional(12.5) }

/// Quiet filter chip: invisible until hovered, soft accent fill when
/// active. Returns the click response.
fn chip(ui: &mut Ui, active: bool, text: impl Into<egui::WidgetText>) -> egui::Response {
    let wt: egui::WidgetText = text.into();
    let galley = wt.into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Small,
    );
    let w = galley.size().x + 24.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, CHIP_H), Sense::click());
    let round = theme::rounding(CHIP_H / 2.0);

    let hover_t = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.08);
    if active {
        // Soft accent fill only — no ring; outlines are reserved for
        // selection in the gallery.
        ui.painter().rect_filled(rect, round, theme::accent_soft());
    } else {
        let a = (hover_t * 18.0) as u8;
        if a > 0 {
            ui.painter().rect_filled(rect, round, Color32::from_white_alpha(a));
        }
    }
    let pos = egui::pos2(
        rect.center().x - galley.size().x / 2.0,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(pos, galley, theme::TEXT_PRIMARY);
    if resp.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    resp
}

/// Icon + label LayoutJob for chips, colored by active state.
fn chip_label(glyph: &str, fallback: &str, text: &str, active: bool) -> egui::text::LayoutJob {
    let (icon_c, text_c) = if active {
        (theme::accent_hover(), theme::TEXT_PRIMARY)
    } else {
        (theme::TEXT_MUTED, theme::TEXT_SECONDARY)
    };
    theme::icon_label(glyph, fallback, text, 11.0, chip_font(), icon_c, text_c)
}

/// Win11-style segmented control. Paints an inset track with an elevated
/// pill on the active segment. Returns true when the value changed.
fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    id: &str,
    options: &[(T, &str)],
    value: &mut T,
) -> bool {
    let h = CHIP_H;
    let font = chip_font();
    let pad = 13.0;
    let widths: Vec<f32> = options.iter()
        .map(|(_, label)| {
            ui.fonts(|f| f.layout_no_wrap(label.to_string(), font.clone(), Color32::WHITE).size().x)
                + pad * 2.0
        })
        .collect();
    let total: f32 = widths.iter().sum::<f32>() + 6.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total, h), Sense::hover());

    // Inset track.
    ui.painter().rect_filled(rect, theme::rounding(h / 2.0), Color32::from_black_alpha(70));

    let mut changed = false;
    let mut x = rect.min.x + 3.0;
    for (i, (opt, label)) in options.iter().enumerate() {
        let seg_rect = Rect::from_min_size(
            egui::pos2(x, rect.min.y + 3.0),
            Vec2::new(widths[i], h - 6.0),
        );
        let resp = ui.interact(seg_rect, ui.id().with((id, i)), Sense::click());
        let active = *value == *opt;
        let seg_round = theme::rounding((h - 6.0) / 2.0);
        if active {
            ui.painter().rect_filled(seg_rect, seg_round, Color32::from_white_alpha(36));
        } else if resp.hovered() {
            ui.painter().rect_filled(seg_rect, seg_round, Color32::from_white_alpha(12));
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        ui.painter().text(
            seg_rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font.clone(),
            if active { theme::TEXT_PRIMARY } else { theme::TEXT_SECONDARY },
        );
        if resp.clicked() && !active {
            *value = *opt;
            changed = true;
        }
        x += widths[i];
    }
    changed
}

// ══════════════════════════════════════════════════════════════════
//  Filter row
// ══════════════════════════════════════════════════════════════════

/// Filter chips row: sort, month grouping, type, extensions, date range,
/// rating, master clear. Rendered in Row 2 of the command bar; wraps on
/// narrow windows.
pub fn show_filters(
    ui: &mut Ui,
    state: &mut AppState,
    exclude_mod: ExcludeModifier,
    show_date_dividers: &mut bool,
) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);

        // ── Sort ────────────────────────────────────────────────────────
        let cur = state.sort_order;
        egui::ComboBox::from_id_salt("filter_sort")
            .selected_text(theme::icon_label(
                theme::ICON_SORT, "⇅", cur.label(),
                11.0, chip_font(), theme::TEXT_MUTED, theme::TEXT_PRIMARY,
            ))
            .height(300.0)
            .show_ui(ui, |ui| {
                for opt in [
                    SortOrder::NewestFirst, SortOrder::OldestFirst,
                    SortOrder::BiggestFirst, SortOrder::SmallestFirst,
                    SortOrder::NameAsc, SortOrder::NameDesc,
                    SortOrder::HighestRated, SortOrder::LowestRated,
                ] {
                    if ui.selectable_label(cur == opt, opt.label()).clicked()
                        && state.sort_order != opt
                    {
                        state.sort_order = opt;
                        state.media_dirty = true;
                    }
                }
            });

        // Month-grouping toggle — only relevant for date-based sorts.
        let date_sort = matches!(state.sort_order, SortOrder::NewestFirst | SortOrder::OldestFirst);
        if date_sort {
            let resp = chip(
                ui,
                *show_date_dividers,
                chip_label(theme::ICON_CALENDAR, "📅", "Months", *show_date_dividers),
            )
            .on_hover_text(if *show_date_dividers {
                "Grouped by month — click for a flat grid"
            } else {
                "Flat grid — click to group by month"
            });
            if resp.clicked() { *show_date_dividers = !*show_date_dividers; }
        }

        ui.add_space(6.0);

        // ── Media type segmented control ────────────────────────────────
        let mut mt = state.media_type_filter;
        if segmented(
            ui,
            "type_seg",
            &[
                (MediaTypeFilter::All, "All"),
                (MediaTypeFilter::Photos, "Photos"),
                (MediaTypeFilter::Videos, "Videos"),
                (MediaTypeFilter::Audio, "Audio"),
            ],
            &mut mt,
        ) {
            state.media_type_filter = mt;
            state.media_dirty = true;
        }

        ui.add_space(6.0);

        // ── Extensions popup ────────────────────────────────────────────
        let n_sel = state.selected_extensions.len();
        let n_ex = state.excluded_extensions.len();
        let ext_active = n_sel > 0 || n_ex > 0;
        let label = if !ext_active {
            "Extensions".to_string()
        } else {
            let mut parts = Vec::new();
            if n_sel > 0 { parts.push(format!("{}✓", n_sel)); }
            if n_ex > 0  { parts.push(format!("{}✕", n_ex)); }
            format!("Extensions · {}", parts.join(" "))
        };
        let ext_btn = chip(ui, ext_active, chip_label(theme::ICON_FILTER, "🧩", &label, ext_active));
        let popup_id = ui.make_persistent_id("filter_ext_popup");
        if ext_btn.clicked() { ui.memory_mut(|m| m.toggle_popup(popup_id)); }
        egui::popup::popup_below_widget(ui, popup_id, &ext_btn, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
            ui.set_min_width(220.0);
            // Mode toggle
            ui.horizontal(|ui| {
                ui.label(RichText::new("Click:").size(theme::FONT_SIZE_SMALL).color(theme::TEXT_SECONDARY));
                if ui.selectable_label(state.ext_click_mode == TagClickMode::Select, "✓ Select").clicked() {
                    state.ext_click_mode = TagClickMode::Select;
                }
                if ui.selectable_label(state.ext_click_mode == TagClickMode::Exclude, "🚫 Exclude").clicked() {
                    state.ext_click_mode = TagClickMode::Exclude;
                }
            });
            ui.label(
                RichText::new(format!("({} + click for the other)", exclude_mod.label()))
                    .small().color(theme::TEXT_MUTED),
            );
            ui.separator();

            if state.available_extensions.is_empty() {
                ui.colored_label(theme::TEXT_MUTED, "No files in library yet.");
                return;
            }
            let exts = state.available_extensions.clone();
            for ext in &exts {
                let is_sel = state.selected_extensions.contains(ext);
                let is_ex = state.excluded_extensions.contains(ext);
                let mut text = RichText::new(format!(".{}", ext));
                if is_ex { text = text.color(theme::DANGER).strikethrough(); }
                let resp = ui.selectable_label(is_sel, text);
                if resp.clicked() {
                    let mods = ui.input(|i| i.modifiers);
                    let mut want_exclude = state.ext_click_mode == TagClickMode::Exclude;
                    if exclude_mod.matches(&mods) { want_exclude = !want_exclude; }
                    if want_exclude {
                        state.selected_extensions.retain(|x| x != ext);
                        if is_ex {
                            state.excluded_extensions.retain(|x| x != ext);
                        } else {
                            state.excluded_extensions.push(ext.clone());
                        }
                    } else {
                        state.excluded_extensions.retain(|x| x != ext);
                        if is_sel {
                            state.selected_extensions.retain(|x| x != ext);
                        } else {
                            state.selected_extensions.push(ext.clone());
                        }
                    }
                    state.media_dirty = true;
                }
            }
            if !state.selected_extensions.is_empty() || !state.excluded_extensions.is_empty() {
                ui.separator();
                if ui.small_button("Clear all").clicked() {
                    state.selected_extensions.clear();
                    state.excluded_extensions.clear();
                    state.media_dirty = true;
                }
            }
        });

        ui.add_space(6.0);

        // ── Date range ──────────────────────────────────────────────────
        ui.label(theme::icon(theme::ICON_CALENDAR, "📅", 12.0, theme::TEXT_MUTED));
        date_picker(ui, "filter_date_from", "From", &mut state.date_from, &mut state.media_dirty);
        ui.label(RichText::new("–").color(theme::TEXT_MUTED));
        date_picker(ui, "filter_date_to",   "To",   &mut state.date_to,   &mut state.media_dirty);

        ui.add_space(6.0);

        // ── Rating range (inline 2-thumb slider) ────────────────────────
        ui.label(theme::icon(theme::ICON_STAR, "★", 12.0, theme::TEXT_MUTED));
        if rating_range_slider(ui, &mut state.rating_min, &mut state.rating_max) {
            state.media_dirty = true;
        }
        let rating_active = state.rating_min != 0 || state.rating_max != 5;
        ui.label(
            RichText::new(format!("{}–{}", state.rating_min, state.rating_max))
                .size(theme::FONT_SIZE_TINY)
                .color(if rating_active { theme::TEXT_PRIMARY } else { theme::TEXT_MUTED }),
        );

        // ── Master clear ────────────────────────────────────────────────
        if any_filter_active(state) {
            ui.add_space(6.0);
            let clear = chip(ui, false, theme::icon_label(
                theme::ICON_CLOSE, "✕", "Clear",
                10.0, chip_font(), theme::DANGER, theme::DANGER,
            )).on_hover_text("Clear every active filter");
            if clear.clicked() {
                state.search_query.clear();
                state.media_type_filter = MediaTypeFilter::All;
                state.selected_extensions.clear();
                state.excluded_extensions.clear();
                state.date_from = None;
                state.date_to = None;
                state.rating_min = 0;
                state.rating_max = 5;
                state.media_dirty = true;
            }
        }
    });
}

/// Row 3 of the command bar: dismissible pills summarising every active
/// filter. Hidden entirely when no filter is active. Each pill removes
/// exactly the filter it represents — the master "Clear filters" button
/// in Row 2 still wipes everything in one click.
pub fn show_active_pills(ui: &mut Ui, state: &mut AppState) {
    if !any_filter_active(state) { return; }

    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;

        // Search pill — lists the active scopes inline so the user can
        // see at a glance whether the search is hitting just filenames,
        // tags too, etc. e.g.  🔍 "vacation" in filenames, tags
        if !state.search_query.trim().is_empty() {
            let q = state.search_query.clone();
            let s = state.search_scopes;
            let mut parts: Vec<&str> = Vec::with_capacity(3);
            if s.filenames { parts.push("filenames"); }
            if s.paths     { parts.push("paths"); }
            if s.tags      { parts.push("tags"); }
            let label = if parts.is_empty() {
                format!("\"{}\"  (no scopes)", q)
            } else {
                format!("\"{}\" in {}", q, parts.join(", "))
            };
            if filter_pill(ui, &label, PillKind::Normal) {
                state.search_query.clear();
                state.media_dirty = true;
            }
        }

        // Type pill
        if state.media_type_filter != MediaTypeFilter::All {
            let label = state.media_type_filter.label();
            if filter_pill(ui, &format!("Type: {}", label), PillKind::Normal) {
                state.media_type_filter = MediaTypeFilter::All;
                state.media_dirty = true;
            }
        }

        // Date pills
        if let Some(d) = state.date_from {
            if filter_pill(ui, &format!("From {}", d.format("%Y-%m-%d")), PillKind::Normal) {
                state.date_from = None;
                state.media_dirty = true;
            }
        }
        if let Some(d) = state.date_to {
            if filter_pill(ui, &format!("To {}", d.format("%Y-%m-%d")), PillKind::Normal) {
                state.date_to = None;
                state.media_dirty = true;
            }
        }

        // Selected extensions
        let selected = state.selected_extensions.clone();
        for ext in &selected {
            if filter_pill(ui, &format!(".{}", ext), PillKind::Normal) {
                state.selected_extensions.retain(|x| x != ext);
                state.media_dirty = true;
            }
        }

        // Excluded extensions (danger-tinted)
        let excluded = state.excluded_extensions.clone();
        for ext in &excluded {
            if filter_pill(ui, &format!(".{}", ext), PillKind::Exclude) {
                state.excluded_extensions.retain(|x| x != ext);
                state.media_dirty = true;
            }
        }
    });
}

#[derive(Copy, Clone, PartialEq)]
enum PillKind { Normal, Exclude }

/// Render a dismissible filter pill. Returns true if the user clicked it
/// (anywhere — the whole pill is the dismiss target, the ✕ is just a
/// visual hint).
fn filter_pill(ui: &mut Ui, text: &str, kind: PillKind) -> bool {
    let font = FontId::proportional(theme::FONT_SIZE_SMALL);
    let text_w = ui.fonts(|f|
        f.layout_no_wrap(text.to_string(), font.clone(), theme::TEXT_PRIMARY).size().x
    );
    let h = 22.0;
    let pad_l = 10.0;
    let pad_r = 8.0;
    let close_w = 10.0;
    let gap = 6.0;
    let total_w = pad_l + text_w + gap + close_w + pad_r;

    let (rect, resp) = ui.allocate_exact_size(egui::vec2(total_w, h), Sense::click());

    // Background
    let bg = match (kind, resp.hovered()) {
        (PillKind::Normal, false) => theme::accent_soft(),
        (PillKind::Normal, true) => theme::accent_alpha(0.42),
        (PillKind::Exclude, false) => Color32::from_rgba_premultiplied(70, 30, 30, 90),
        (PillKind::Exclude, true)  => Color32::from_rgba_premultiplied(110, 50, 50, 130),
    };
    ui.painter().rect_filled(rect, egui::Rounding::same(h * 0.5), bg);

    let text_col = if matches!(kind, PillKind::Exclude) {
        Color32::from_rgb(230, 170, 170)
    } else {
        theme::TEXT_PRIMARY
    };
    ui.painter().text(
        egui::pos2(rect.min.x + pad_l, rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        font,
        text_col,
    );

    let close_col = if resp.hovered() { theme::TEXT_PRIMARY } else { theme::TEXT_SECONDARY };
    ui.painter().text(
        egui::pos2(rect.max.x - pad_r, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        if theme::icons_available() { theme::ICON_CLOSE } else { "✕" },
        theme::icon_font(9.0),
        close_col,
    );

    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    resp.clicked()
}

fn any_filter_active(state: &AppState) -> bool {
    !state.search_query.trim().is_empty()
        || state.media_type_filter != MediaTypeFilter::All
        || !state.selected_extensions.is_empty()
        || !state.excluded_extensions.is_empty()
        || state.date_from.is_some()
        || state.date_to.is_some()
        || state.rating_min != 0
        || state.rating_max != 5
}

/// Custom 2-thumb range slider for the rating filter (discrete steps
/// 0..=5). Egui has no built-in range slider; this paints a track +
/// two draggable thumbs (Win11 style: neutral thumb, accent center dot),
/// with the active span between the thumbs in the accent colour.
///
/// Drag-target is decided once at `drag_started` (whichever thumb is
/// closer to the press point); the rest of the drag locks to that
/// thumb so dragging across the other thumb doesn't accidentally
/// swap which one is moving.
///
/// Returns true if either bound changed this frame.
pub fn rating_range_slider(ui: &mut egui::Ui, min: &mut i32, max: &mut i32) -> bool {
    let width = 150.0;
    let height = 22.0;
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(width, height),
        egui::Sense::click_and_drag(),
    );
    let painter = ui.painter();
    let track_y = rect.center().y;
    let pad = 9.0; // breathing room at each end so thumbs don't clip
    let track_left = rect.min.x + pad;
    let track_right = rect.max.x - pad;
    let track_w = track_right - track_left;
    let pos_for = |v: i32| -> f32 {
        track_left + (v.clamp(0, 5) as f32 / 5.0) * track_w
    };
    let value_at = |x: f32| -> i32 {
        let frac = ((x - track_left) / track_w).clamp(0.0, 1.0);
        (frac * 5.0).round() as i32
    };

    // Track (background line)
    painter.line_segment(
        [egui::pos2(track_left, track_y), egui::pos2(track_right, track_y)],
        egui::Stroke::new(3.0, Color32::from_white_alpha(18)),
    );
    // Span between thumbs — accent only when the range actually filters
    // something; a full 0–5 range stays neutral so an inactive filter
    // doesn't draw attention.
    let range_active = *min != 0 || *max != 5;
    let min_x = pos_for(*min);
    let max_x = pos_for(*max);
    painter.line_segment(
        [egui::pos2(min_x, track_y), egui::pos2(max_x, track_y)],
        egui::Stroke::new(
            3.0,
            if range_active { theme::accent() } else { Color32::from_white_alpha(40) },
        ),
    );

    // Drag state — remember which thumb was grabbed at press time so
    // dragging across the other thumb doesn't suddenly switch which
    // one is moving.
    let drag_id = ui.id().with("rating_slider_active_thumb");
    let mut changed = false;
    if resp.drag_started() {
        if let Some(p) = resp.interact_pointer_pos() {
            // 0 = min, 1 = max. On exact tie, prefer min when the
            // press is to the left of equal-position thumbs.
            let dist_min = (p.x - min_x).abs();
            let dist_max = (p.x - max_x).abs();
            let drag_min = if (dist_min - dist_max).abs() < 0.001 {
                p.x <= min_x
            } else {
                dist_min < dist_max
            };
            ui.memory_mut(|m| m.data.insert_temp(drag_id, drag_min));
        }
    }
    if resp.dragged() {
        if let Some(p) = resp.interact_pointer_pos() {
            let v = value_at(p.x);
            let drag_min: bool = ui.memory(|m| m.data.get_temp(drag_id).unwrap_or(true));
            if drag_min {
                let clamped = v.min(*max);
                if *min != clamped { *min = clamped; changed = true; }
            } else {
                let clamped = v.max(*min);
                if *max != clamped { *max = clamped; changed = true; }
            }
        }
    }
    // A bare click (no drag) snaps the nearer thumb to the click value.
    if resp.clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            let v = value_at(p.x);
            let dist_min = (p.x - min_x).abs();
            let dist_max = (p.x - max_x).abs();
            if dist_min < dist_max {
                let clamped = v.min(*max);
                if *min != clamped { *min = clamped; changed = true; }
            } else {
                let clamped = v.max(*min);
                if *max != clamped { *max = clamped; changed = true; }
            }
        }
    }

    // Thumbs — Win11 style: neutral disc with an accent center dot. The
    // dark halo separates the disc from the track; no outline needed.
    let thumb_r = 6.5;
    for x in [min_x, max_x] {
        let c = egui::pos2(x, track_y);
        ui.painter().circle_filled(c, thumb_r + 1.5, Color32::from_black_alpha(110));
        ui.painter().circle_filled(c, thumb_r, Color32::from_rgb(72, 72, 72));
        ui.painter().circle_filled(c, 3.2, theme::accent());
    }

    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    changed
}

/// Continuous slider (used for thumbnail size). egui's built-in slider
/// paints its knob as a fill + `fg_stroke` ring, which on this theme comes
/// out as a hollow white circle; this one matches the rating slider's
/// language instead — thin track, accent trailing fill, neutral thumb with
/// an accent centre dot, no ring. Returns true when the value changed.
pub fn value_slider(
    ui: &mut Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    width: f32,
) -> bool {
    let height = 22.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click_and_drag());
    let track_y = rect.center().y;
    let pad = 8.0;
    let left = rect.min.x + pad;
    let right = rect.max.x - pad;
    let track_w = (right - left).max(1.0);
    let (lo, hi) = (*range.start(), *range.end());
    let span = (hi - lo).max(f32::EPSILON);

    // Handle input first so the thumb tracks the pointer in the same frame.
    let mut changed = false;
    if resp.dragged() || resp.clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            let f = ((p.x - left) / track_w).clamp(0.0, 1.0);
            let v = lo + f * span;
            if (v - *value).abs() > 0.01 {
                *value = v;
                changed = true;
            }
        }
    }

    let frac = ((*value - lo) / span).clamp(0.0, 1.0);
    let x = left + frac * track_w;

    ui.painter().line_segment(
        [egui::pos2(left, track_y), egui::pos2(right, track_y)],
        Stroke::new(3.0, Color32::from_white_alpha(22)),
    );
    ui.painter().line_segment(
        [egui::pos2(left, track_y), egui::pos2(x, track_y)],
        Stroke::new(3.0, theme::accent()),
    );

    let c = egui::pos2(x, track_y);
    ui.painter().circle_filled(c, 8.0, Color32::from_black_alpha(110));
    ui.painter().circle_filled(c, 6.5, Color32::from_rgb(72, 72, 72));
    ui.painter().circle_filled(c, 3.2, theme::accent());

    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    changed
}

/// Date picker pair: a checkbox-like toggle + a calendar popup button. The
/// `DatePickerButton` always needs a real `NaiveDate`, so we keep a scratch
/// value in egui memory; whether the bound is "active" is tracked by the
/// `Option`. Clearing the bound resets to None.
fn date_picker(
    ui: &mut Ui,
    id_source: &str,
    label: &str,
    value: &mut Option<chrono::NaiveDate>,
    dirty: &mut bool,
) {
    use egui_extras::DatePickerButton;

    let mem_id = egui::Id::new(("date_picker_scratch", id_source));
    let today = chrono::Local::now().date_naive();
    let mut scratch: chrono::NaiveDate = ui.memory(|m| m.data.get_temp(mem_id))
        .unwrap_or_else(|| value.unwrap_or(today));
    let before = scratch;

    // Inactive bound → "From"/"To" placeholder chip that, when clicked,
    // activates the bound at today's date. Once active, render the real
    // picker + clear.
    if value.is_none() {
        if chip(ui, false, RichText::new(label).size(12.5).color(theme::TEXT_SECONDARY)).clicked() {
            *value = Some(scratch);
            *dirty = true;
        }
    } else {
        ui.add(
            DatePickerButton::new(&mut scratch)
                .id_salt(id_source)
                .format("%Y-%m-%d")
                .calendar(true)
                .calendar_week(false)
                .show_icon(false)
                .highlight_weekends(true),
        );
        if scratch != before {
            *value = Some(scratch);
            *dirty = true;
        }
        if ui.small_button(theme::icon(theme::ICON_CLOSE, "✕", 9.0, theme::TEXT_SECONDARY))
            .on_hover_text("Clear").clicked()
        {
            *value = None;
            *dirty = true;
        }
    }
    ui.memory_mut(|m| m.data.insert_temp(mem_id, scratch));
}
