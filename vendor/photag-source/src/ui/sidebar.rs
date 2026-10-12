use crate::app::{AppState, TagClickMode};
use crate::db::queries::{Folder, Tag};
use crate::settings::{ExcludeModifier, Settings, TagSearchMatch};
use crate::theme;
use egui::{self, Color32, Stroke, Ui, Vec2};

/// Parse "#RRGGBB" → [r, g, b]. Returns None on malformed input.
pub fn parse_hex_color(s: &str) -> Option<[u8; 3]> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 { return None; }
    Some([
        u8::from_str_radix(&s[0..2], 16).ok()?,
        u8::from_str_radix(&s[2..4], 16).ok()?,
        u8::from_str_radix(&s[4..6], 16).ok()?,
    ])
}

pub fn format_hex_color(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

/// True if `target_id` exists in `tags` (or its descendants) and has at least
/// one child. Used by the edit-tag modal to decide whether to show the
/// "apply to children" checkbox.
pub fn tag_has_children(tags: &[Tag], target_id: i64) -> bool {
    for tag in tags {
        if tag.id == target_id { return !tag.children.is_empty(); }
        if tag_has_children(&tag.children, target_id) { return true; }
    }
    false
}

/// Look up a tag's display name in the tree, recursing into descendants.
/// Returns `None` if the id isn't found.
pub fn find_tag_name(tags: &[Tag], target_id: i64) -> Option<String> {
    for tag in tags {
        if tag.id == target_id { return Some(tag.name.clone()); }
        if let Some(n) = find_tag_name(&tag.children, target_id) { return Some(n); }
    }
    None
}

/// Color to use when rendering a tag's swatch / chip background.
pub fn tag_color32(tag: &Tag) -> Color32 {
    tag.color
        .as_deref()
        .and_then(parse_hex_color)
        .map(|[r, g, b]| Color32::from_rgb(r, g, b))
        .unwrap_or(Color32::from_rgb(120, 120, 130))
}

#[derive(PartialEq)]
pub enum SidebarTab {
    Folders,
    Tags,
}

/// Standard sidebar row height — used by every full-width selectable row
/// so the count text lines up vertically across folders, subdirs, tags.
const SIDEBAR_ROW_H: f32 = 26.0;

/// Render a full-width sidebar row with **left-aligned** text and the
/// standard hover / selected states. Replaces `add_sized + SelectableLabel`
/// which forces centered text via `Layout::centered_and_justified`.
fn sidebar_row(
    ui: &mut Ui,
    is_selected: bool,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    let avail_w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(avail_w, SIDEBAR_ROW_H),
        egui::Sense::click(),
    );

    // Background — soft accent fill when selected, faint lift on hover.
    if is_selected {
        ui.painter().rect_filled(
            rect,
            egui::Rounding::same(theme::ROUND_SM),
            ui.visuals().selection.bg_fill,
        );
    } else if resp.hovered() {
        ui.painter().rect_filled(
            rect,
            egui::Rounding::same(theme::ROUND_SM),
            ui.visuals().widgets.hovered.weak_bg_fill,
        );
    }

    // Text — left-aligned, vertically centered. Reserve ~30px on the right
    // for the count badge so long names don't run under it.
    let widget_text: egui::WidgetText = text.into();
    let wrap_w = (avail_w - 36.0).max(20.0);
    let galley = widget_text.into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        wrap_w,
        egui::TextStyle::Button,
    );
    let text_pos = egui::pos2(
        rect.left() + 4.0,
        rect.center().y - galley.size().y / 2.0,
    );
    let fallback_col = ui.visuals().text_color();
    ui.painter().galley(text_pos, galley, fallback_col);

    resp
}

/// Accent-tinted "+" button used in section headers. Bigger and brighter
/// than `small_button("+")` so it reads as an obvious primary action even
/// at a glance. Returns true on click.
pub fn accent_plus_button(ui: &mut Ui, hover_text: &str) -> bool {
    let size = Vec2::new(22.0, 22.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let bg = if resp.hovered() { theme::accent_hover() } else { theme::accent() };
    ui.painter().rect_filled(rect, egui::Rounding::same(theme::ROUND_SM + 1.0), bg);
    if theme::icons_available() {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            theme::ICON_ADD,
            theme::icon_font(10.0),
            Color32::WHITE,
        );
    } else {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "+",
            egui::FontId::proportional(16.0),
            Color32::WHITE,
        );
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.on_hover_text(hover_text).clicked()
}

/// Render a small-caps section header with an optional right-aligned action
/// (typically the "+" button). The label is uppercase, dim, and small —
/// reads as a section divider without needing a horizontal rule.
fn section_header<R>(ui: &mut Ui, label: &str, action: impl FnOnce(&mut Ui) -> R) -> R {
    ui.add_space(2.0);
    let result = ui.horizontal(|ui| {
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(label.to_uppercase())
                .size(theme::FONT_SIZE_TINY)
                .color(theme::TEXT_MUTED),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            action(ui)
        }).inner
    }).inner;
    ui.add_space(4.0);
    result
}

/// Paint a right-aligned count next to a row. Plain muted text — no pill
/// background — so it reads as metadata, not a button.
fn paint_count_badge(ui: &Ui, row_rect: egui::Rect, count: i64) {
    if count <= 0 { return; }
    let text = if count >= 10000 {
        format!("{}k", count / 1000)
    } else {
        count.to_string()
    };
    let font = egui::FontId::proportional(theme::FONT_SIZE_TINY);
    ui.painter().text(
        egui::pos2(row_rect.max.x - 6.0, row_rect.center().y),
        egui::Align2::RIGHT_CENTER,
        text,
        font,
        theme::TEXT_MUTED,
    );
}

/// Underline a tab label when active — the horizontal counterpart of
/// `theme::paint_selection_bar` for vertical row selection.
fn paint_tab_underline(ui: &Ui, rect: egui::Rect, active: bool) {
    if !active { return; }
    // Win11 pivot indicator: a short, rounded accent bar centered under the
    // active tab rather than a full-width hard line.
    let inset = (rect.width() * 0.28).max(12.0);
    let bar = egui::Rect::from_min_max(
        egui::pos2(rect.left() + inset, rect.bottom() - 3.0),
        egui::pos2(rect.right() - inset, rect.bottom()),
    );
    ui.painter().rect_filled(bar, egui::Rounding::same(1.5), theme::accent());
}

/// Reserve a fixed-width slot for the expand/collapse triangle. When
/// `has_children` is true, draws a clickable arrow and returns whether
/// it was clicked. When false, draws nothing but consumes the same space
/// so labels stay vertically aligned across rows.
fn arrow_slot(ui: &mut Ui, has_children: bool, expanded: bool) -> bool {
    let size = Vec2::new(16.0, 16.0);
    if !has_children {
        ui.allocate_exact_size(size, egui::Sense::hover());
        return false;
    }
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let color = if resp.hovered() { theme::TEXT_PRIMARY } else { theme::TEXT_MUTED };
    if theme::icons_available() {
        let glyph = if expanded { theme::ICON_CHEVRON_DOWN } else { theme::ICON_CHEVRON_RIGHT };
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            glyph,
            theme::icon_font(8.0),
            color,
        );
    } else {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            if expanded { "▼" } else { "▶" },
            egui::TextStyle::Small.resolve(ui.style()),
            color,
        );
    }
    resp.clicked()
}

/// One Fluent pivot tab: plain text (semibold + bright when active) with
/// the rounded accent underline. No chip fill — the pivot indicator does
/// all the work.
fn pivot_tab(ui: &mut Ui, width: f32, active: bool, label: &str) -> egui::Response {
    let tab_height = 32.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, tab_height), egui::Sense::click());
    let hover_t = ui.ctx().animate_bool_with_time(resp.id, resp.hovered() && !active, 0.08);
    if hover_t > 0.01 {
        ui.painter().rect_filled(
            rect.shrink2(egui::vec2(4.0, 4.0)),
            egui::Rounding::same(theme::ROUND_SM),
            Color32::from_white_alpha((10.0 * hover_t) as u8),
        );
    }
    let (font, color) = if active {
        (theme::semibold(theme::FONT_SIZE_BUTTON), theme::TEXT_PRIMARY)
    } else {
        (egui::FontId::proportional(theme::FONT_SIZE_BUTTON), theme::TEXT_SECONDARY)
    };
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, font, color);
    paint_tab_underline(ui, rect, active);
    if resp.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    resp
}

pub fn show_sidebar(ui: &mut Ui, state: &mut AppState, settings: &Settings) {
    let exclude_mod = settings.exclude_modifier;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        // Split the row evenly up front — deriving each tab's width from the
        // *remaining* space would make the second tab half the size of the first.
        let tab_w = (ui.available_width() / 2.0).max(60.0);
        let folders_resp = pivot_tab(ui, tab_w, state.sidebar_tab == SidebarTab::Folders, "Folders");
        if folders_resp.clicked() && state.sidebar_tab != SidebarTab::Folders {
            // Leaving Tags → optionally clear the tag-search query.
            if !settings.tag_search_persist {
                state.tag_search_query.clear();
            }
            state.sidebar_tab = SidebarTab::Folders;
            state.media_dirty = true;
        }
        let tags_resp = pivot_tab(ui, tab_w, state.sidebar_tab == SidebarTab::Tags, "Tags");
        if tags_resp.clicked() && state.sidebar_tab != SidebarTab::Tags {
            state.sidebar_tab = SidebarTab::Tags;
            state.media_dirty = true;
        }
    });
    ui.add_space(6.0);

    match state.sidebar_tab {
        SidebarTab::Folders => show_folder_tree(ui, state),
        SidebarTab::Tags => show_tag_tree(ui, state, exclude_mod, settings.tag_search_match),
    }
}

fn show_folder_tree(ui: &mut Ui, state: &mut AppState) {
    section_header(ui, "Folders", |ui| {
        if accent_plus_button(ui, "Add folder…") {
            state.pending_action = Some(crate::app::PendingAction::AddFolder);
        }
    });

    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 1.0;
        let all_selected = state.selected_folder_id.is_none() && state.selected_path.is_none();
        let total: i64 = state.folder_counts.values().sum();
        let all_resp = sidebar_row(ui, all_selected, theme::icon_label(
            theme::ICON_LIBRARY, "📁", "All files", 12.0,
            egui::FontId::proportional(theme::FONT_SIZE_BUTTON),
            if all_selected { theme::accent_hover() } else { theme::TEXT_SECONDARY },
            theme::TEXT_PRIMARY,
        ));
        theme::paint_selection_bar(ui, all_resp.rect, all_selected);
        paint_count_badge(ui, all_resp.rect, total);
        if all_resp.clicked() {
            state.selected_folder_id = None;
            state.selected_path = None;
            state.media_dirty = true;
        }
        ui.add_space(2.0);
        // Iterate without cloning the folder list. `show_folder_item`
        // borrows `&mut state` but doesn't touch `state.folders` itself,
        // so we can temporarily move the vec out, iterate, and put it
        // back. Saves a per-frame Vec<Folder>+Strings allocation that
        // was contributing to the window-drag jitter.
        let folders = std::mem::take(&mut state.folders);
        for folder in &folders { show_folder_item(ui, state, folder); }
        state.folders = folders;
    });
}

fn show_folder_item(ui: &mut Ui, state: &mut AppState, folder: &Folder) {
    let is_selected = state.selected_folder_id == Some(folder.id) && state.selected_path.is_none();
    let expanded = state.expanded_subdirs.contains(&folder.path);
    // Cached after the first call. Used to (a) decide whether to draw the
    // ▶/▼ arrow, and (b) decide whether the subdir tree should be rendered.
    let subdirs = state.get_subdirs(&folder.path);
    let has_subdirs = !subdirs.is_empty();

    let resp = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        if arrow_slot(ui, has_subdirs, expanded) {
            if expanded { state.expanded_subdirs.remove(&folder.path); }
            else { state.expanded_subdirs.insert(folder.path.clone()); }
        }
        let count = state.folder_counts.get(&folder.id).copied().unwrap_or(0);
        let glyph = if expanded { theme::ICON_FOLDER_OPEN } else { theme::ICON_FOLDER };
        let label = theme::icon_label(
            glyph, "📁", &folder.name, 12.0,
            egui::FontId::proportional(theme::FONT_SIZE_BUTTON),
            if is_selected { theme::accent_hover() } else { theme::TEXT_SECONDARY },
            theme::TEXT_PRIMARY,
        );
        let r = sidebar_row(ui, is_selected, label);
        theme::paint_selection_bar(ui, r.rect, is_selected);
        paint_count_badge(ui, r.rect, count);
        r
    }).inner;

    if resp.clicked() {
        state.selected_folder_id = Some(folder.id);
        state.selected_path = None;
        state.media_dirty = true;
    }

    resp.context_menu(|ui| {
        if ui.button("➕ New subfolder…").clicked() {
            state.new_folder_parent = Some(folder.path.clone());
            state.new_folder_name.clear();
            ui.close_menu();
        }
        if ui.button("Rescan").clicked() {
            state.pending_action = Some(crate::app::PendingAction::Rescan(folder.id, folder.path.clone()));
            ui.close_menu();
        }
        if ui.button("Open in Explorer").clicked() {
            open::that_detached(&folder.path).ok();
            ui.close_menu();
        }
        ui.separator();
        if ui.button("🗑 Remove").clicked() {
            state.pending_action = Some(crate::app::PendingAction::RemoveFolder(folder.id));
            ui.close_menu();
        }
    });

    // Render subdirs whenever the folder is expanded — independent of selection
    // so the arrow toggle works on first click.
    if expanded && has_subdirs {
        show_subdirs(ui, state, &subdirs, 1);
    }
}

fn show_subdirs(ui: &mut Ui, state: &mut AppState, subdirs: &[(String, String, bool)], depth: usize) {
    // Fluent trees separate levels by indentation alone — no connector
    // lines. (The old ASCII-style └─ lines read like a 2005 file manager.)
    let indent = 6.0 + depth as f32 * 15.0;
    for (name, path, has_children) in subdirs.iter() {
        let is_selected = state.selected_path.as_ref() == Some(path);

        let expanded = state.expanded_subdirs.contains(path);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(indent);

            if arrow_slot(ui, *has_children, expanded) {
                if expanded { state.expanded_subdirs.remove(path); }
                else { state.expanded_subdirs.insert(path.clone()); }
            }

            let count = state.subdir_count(path);
            let label = theme::icon_label(
                if expanded { theme::ICON_FOLDER_OPEN } else { theme::ICON_FOLDER },
                "📂", name, 11.0,
                egui::FontId::proportional(theme::FONT_SIZE_BUTTON),
                if is_selected { theme::accent_hover() } else { theme::TEXT_MUTED },
                theme::TEXT_PRIMARY,
            );
            let resp = sidebar_row(ui, is_selected, label);
            theme::paint_selection_bar(ui, resp.rect, is_selected);
            paint_count_badge(ui, resp.rect, count);
            if resp.clicked() {
                state.selected_folder_id = None;
                state.selected_path = Some(path.clone());
                state.media_dirty = true;
            }
            resp.context_menu(|ui| {
                if ui.button("➕ New subfolder…").clicked() {
                    state.new_folder_parent = Some(path.clone());
                    state.new_folder_name.clear();
                    ui.close_menu();
                }
                if ui.button("Open in Explorer").clicked() {
                    open::that_detached(path).ok();
                    ui.close_menu();
                }
            });
        });

        if expanded && *has_children {
            let children = state.get_subdirs(path);
            if !children.is_empty() { show_subdirs(ui, state, &children, depth + 1); }
        }
    }
}

/// True if `tag` itself matches `query` under `mode`, OR any of its
/// descendants does. Used to decide whether to render a node when the
/// tag-search box has a non-empty query — we keep the path to every
/// match visible, even if some ancestors don't match by name.
fn tag_or_descendant_matches(tag: &Tag, query: &str, mode: TagSearchMatch) -> bool {
    if query.is_empty() { return true; }
    if mode.matches(&tag.name, query) { return true; }
    tag.children.iter().any(|c| tag_or_descendant_matches(c, query, mode))
}

fn show_tag_tree(
    ui: &mut Ui,
    state: &mut AppState,
    exclude_mod: ExcludeModifier,
    search_match: TagSearchMatch,
) {
    section_header(ui, "Tags", |ui| {
        if accent_plus_button(ui, "Create tag…") {
            state.create_tag_parent = None;
            state.create_tag_name.clear();
            state.show_create_tag = true;
        }
    });

    // ── Tag tree search ─────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        ui.label(egui::RichText::new("🔍").size(11.0).color(theme::TEXT_SECONDARY));
        let avail = ui.available_width();
        let clear_w = if state.tag_search_query.is_empty() { 0.0 } else { 22.0 };
        let resp = ui.add(
            egui::TextEdit::singleline(&mut state.tag_search_query)
                .hint_text("Search tags…")
                .desired_width(avail - clear_w - 4.0)
                .margin(egui::vec2(6.0, 2.0)),
        );
        if resp.changed() {
            // Force-expand every ancestor of a match by clearing
            // collapse state — the renderer below treats an empty
            // query as "honour user's collapsed/expanded state"
            // and a non-empty query as "show every matching node
            // expanded so its descendants are visible".
        }
        if !state.tag_search_query.is_empty()
            && ui.small_button("✕").on_hover_text("Clear").clicked()
        {
            state.tag_search_query.clear();
        }
    });
    ui.add_space(4.0);

    // Mode toggle: plain click selects vs. excludes. The modifier hint
    // lives in the tooltips now instead of a caption row.
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        let hint = format!("Plain click applies this mode; {} + click does the other.", exclude_mod.label());
        let select_btn = ui.selectable_label(state.tag_click_mode == TagClickMode::Select, "✓ Select")
            .on_hover_text(format!("Clicking a tag filters TO it. {}", hint));
        if select_btn.clicked() { state.tag_click_mode = TagClickMode::Select; }
        let exclude_btn = ui.selectable_label(state.tag_click_mode == TagClickMode::Exclude, "⊘ Exclude")
            .on_hover_text(format!("Clicking a tag filters it OUT. {}", hint));
        if exclude_btn.clicked() { state.tag_click_mode = TagClickMode::Exclude; }
        if ui.small_button("All").on_hover_text("Select/exclude all tags (click again to clear)").clicked() {
            fn collect_ids(tags: &[crate::db::queries::Tag], out: &mut Vec<i64>) {
                for t in tags { out.push(t.id); collect_ids(&t.children, out); }
            }
            let mut all = Vec::new();
            collect_ids(&state.tag_tree, &mut all);
            match state.tag_click_mode {
                TagClickMode::Select => {
                    if state.selected_tag_ids.len() == all.len() {
                        state.selected_tag_ids.clear();
                    } else {
                        state.selected_tag_ids = all;
                    }
                }
                TagClickMode::Exclude => {
                    if state.excluded_tag_ids.len() == all.len() {
                        state.excluded_tag_ids.clear();
                    } else {
                        state.excluded_tag_ids = all;
                    }
                }
            }
            state.media_dirty = true;
        }
    });
    ui.add_space(4.0);

    if !state.selected_tag_ids.is_empty() || !state.excluded_tag_ids.is_empty() {
        ui.horizontal(|ui| {
            ui.add_space(2.0);
            if !state.selected_tag_ids.is_empty()
                && ui.small_button(format!("Clear ✓ ({})", state.selected_tag_ids.len())).clicked()
            {
                state.selected_tag_ids.clear();
                state.media_dirty = true;
            }
            if !state.excluded_tag_ids.is_empty()
                && ui.small_button(format!("Clear 🚫 ({})", state.excluded_tag_ids.len())).clicked()
            {
                state.excluded_tag_ids.clear();
                state.media_dirty = true;
            }
        });
        ui.add_space(4.0);
    }

    let query = state.tag_search_query.trim().to_string();
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 1.0;
        // Avoid the deep recursive `state.tag_tree.clone()` that was
        // running every frame (and recursively cloning every Tag's
        // `children: Vec<Tag>` plus its `String` fields). For dozens of
        // nested tags that's hundreds of allocations per frame — a major
        // source of window-drag jitter. `show_tag_node` doesn't read
        // `state.tag_tree`, so we can temp-swap it out for the iteration.
        let tags = std::mem::take(&mut state.tag_tree);
        // Filter to nodes whose subtree contains a match. Empty query
        // → everything is visible (matches() returns true).
        let visible: Vec<&Tag> = tags.iter()
            .filter(|t| tag_or_descendant_matches(t, &query, search_match))
            .collect();
        let vlen = visible.len();
        let mut any_match = false;
        for (i, tag) in visible.iter().enumerate() {
            show_tag_node(ui, state, tag, 0, exclude_mod, &query, search_match);
            any_match = true;
        }
        let is_empty = tags.is_empty();
        state.tag_tree = tags;
        if is_empty {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("No tags yet. Click + to create one.")
                        .size(theme::FONT_SIZE_SMALL)
                        .color(theme::TEXT_MUTED),
                );
            });
        } else if !any_match && !query.is_empty() {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(format!("No tags match \"{}\".", query))
                        .size(theme::FONT_SIZE_SMALL)
                        .color(theme::TEXT_MUTED),
                );
            });
        }
    });
}

fn show_tag_node(
    ui: &mut Ui,
    state: &mut AppState,
    tag: &Tag,
    depth: usize,
    exclude_mod: ExcludeModifier,
    query: &str,
    search_match: TagSearchMatch,
) {
    let indent = depth as f32 * 16.0;
    let is_selected = state.selected_tag_ids.contains(&tag.id);
    let is_excluded = state.excluded_tag_ids.contains(&tag.id);
    let has_children = !tag.children.is_empty();
    // While a search is active, force-expand every visible node so
    // matches deep in the tree are reachable without manual expansion.
    let expanded = if query.is_empty() {
        state.expanded_tag_ids.contains(&tag.id)
    } else {
        true
    };

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        // Fluent trees separate levels by indentation alone — no
        // connector lines.
        if depth > 0 {
            ui.add_space(indent);
        }

        if arrow_slot(ui, has_children, expanded) {
            if expanded { state.expanded_tag_ids.remove(&tag.id); }
            else { state.expanded_tag_ids.insert(tag.id); }
        }

        let custom_icon = tag.icon.as_deref().filter(|s| !s.is_empty());
        let tag_col = tag_color32(tag);
        let body_col = if is_excluded { theme::DANGER } else { ui.visuals().text_color() };
        let strike = if is_excluded { Stroke::new(1.0, body_col) } else { Stroke::NONE };
        let font_id = egui::TextStyle::Button.resolve(ui.style());
        let mut job = egui::text::LayoutJob::default();
        match custom_icon {
            // User-picked emoji icon — keep it, tinted with the tag color.
            Some(icon) => job.append(icon, 0.0, egui::TextFormat {
                font_id: font_id.clone(),
                color: tag_col,
                strikethrough: strike,
                valign: egui::Align::Center,
                ..Default::default()
            }),
            // Default: the Fluent tag glyph in the tag's color.
            None => {
                let (glyph, ifont) = if theme::icons_available() {
                    (theme::ICON_TAG, theme::icon_font(11.0))
                } else {
                    ("🏷", font_id.clone())
                };
                job.append(glyph, 1.0, egui::TextFormat {
                    font_id: ifont,
                    color: tag_col,
                    strikethrough: strike,
                    valign: egui::Align::Center,
                    ..Default::default()
                });
            }
        }
        job.append(&tag.name, 6.0, egui::TextFormat {
            font_id,
            color: body_col,
            strikethrough: strike,
            valign: egui::Align::Center,
            ..Default::default()
        });
        let label_resp = sidebar_row(ui, is_selected, job);
        theme::paint_selection_bar(ui, label_resp.rect, is_selected);
        paint_count_badge(ui, label_resp.rect, tag.media_count);
        let resp = ui.interact(
            label_resp.rect,
            egui::Id::new(("tag_row", tag.id)),
            egui::Sense::click_and_drag(),
        );
        if resp.drag_started() {
            egui::DragAndDrop::set_payload(ui.ctx(), tag.id);
        }

        // ── Drag-to-reorder/reparent ─────────────────────────────────
        // When a tag is being dragged and the pointer hovers this row,
        // split the row into three zones (top 25% / middle 50% /
        // bottom 25%). Top/bottom = sibling-before/after; middle =
        // become-child. Visual feedback: a thin accent line for
        // sibling drops, full-row highlight for "into".
        let pointer = ui.ctx().pointer_interact_pos();
        let dragging_payload = egui::DragAndDrop::payload::<i64>(ui.ctx());
        let dragged_tag_id = dragging_payload.as_deref().copied();
        let row_rect = label_resp.rect;
        let mut hovered_zone: Option<u8> = None; // 0=before, 1=into, 2=after
        if dragged_tag_id.is_some() && dragged_tag_id != Some(tag.id) {
            if let Some(p) = pointer {
                if row_rect.contains(p) {
                    let rel = (p.y - row_rect.top()) / row_rect.height().max(1.0);
                    hovered_zone = Some(if rel < 0.25 { 0 } else if rel > 0.75 { 2 } else { 1 });
                }
            }
        }
        if let Some(zone) = hovered_zone {
            let accent = theme::accent();
            match zone {
                0 => {
                    let y = row_rect.top();
                    ui.painter().line_segment(
                        [egui::pos2(row_rect.left(), y), egui::pos2(row_rect.right(), y)],
                        Stroke::new(2.0, accent),
                    );
                }
                2 => {
                    let y = row_rect.bottom();
                    ui.painter().line_segment(
                        [egui::pos2(row_rect.left(), y), egui::pos2(row_rect.right(), y)],
                        Stroke::new(2.0, accent),
                    );
                }
                _ => {
                    ui.painter().rect_stroke(
                        row_rect,
                        egui::Rounding::same(theme::ROUND_SM),
                        Stroke::new(1.5, accent),
                    );
                }
            }
        }
        if let Some(payload) = resp.dnd_release_payload::<i64>() {
            let src = *payload;
            if src != tag.id {
                let drop = match hovered_zone {
                    Some(0) => Some(crate::db::queries::TagDropPos::Before(tag.id)),
                    Some(2) => Some(crate::db::queries::TagDropPos::After(tag.id)),
                    // Any other / unresolved zone: treat as "make child of",
                    // matching the strongest visual signal.
                    _ => Some(crate::db::queries::TagDropPos::Into(tag.id)),
                };
                if let Some(d) = drop {
                    state.pending_action = Some(crate::app::PendingAction::MoveTag {
                        source: src, drop: d,
                    });
                }
            }
        }
        if resp.clicked() {
            let mods = ui.input(|i| i.modifiers);
            let mut want_exclude = state.tag_click_mode == TagClickMode::Exclude;
            if exclude_mod.matches(&mods) { want_exclude = !want_exclude; }
            if want_exclude {
                state.selected_tag_ids.retain(|&id| id != tag.id);
                if is_excluded {
                    state.excluded_tag_ids.retain(|&id| id != tag.id);
                } else {
                    state.excluded_tag_ids.push(tag.id);
                }
            } else {
                state.excluded_tag_ids.retain(|&id| id != tag.id);
                if is_selected {
                    state.selected_tag_ids.retain(|&id| id != tag.id);
                } else {
                    state.selected_tag_ids.push(tag.id);
                }
            }
            state.media_dirty = true;
        }
        resp.context_menu(|ui| {
            if ui.button("Create child tag").clicked() {
                state.create_tag_parent = Some(tag.id);
                state.create_tag_name.clear();
                state.show_create_tag = true;
                ui.close_menu();
            }
            if ui.button("Edit…").clicked() {
                state.edit_tag = Some(tag.id);
                state.edit_tag_name = tag.name.clone();
                state.edit_tag_color_enabled = tag.color.is_some();
                state.edit_tag_color = tag.color
                    .as_deref()
                    .and_then(parse_hex_color)
                    .unwrap_or([200, 200, 200]);
                state.edit_tag_icon = tag.icon.clone().unwrap_or_default();
                state.edit_tag_apply_children = false;
                state.edit_tag_exclude_edited = true;
                state.edit_tag_display = tag.display;
                state.edit_tag_associated = {
                    let conn = state.read_db.lock().unwrap();
                    crate::db::queries::get_associated_tags(&conn, tag.id).unwrap_or_default()
                };
                ui.close_menu();
            }
            if ui.button("Merge into…").clicked() {
                state.merge_tag_source = Some(tag.id);
                state.merge_tag_filter.clear();
                ui.close_menu();
            }
            ui.separator();
            if ui.button("🗑 Delete").clicked() {
                if tag.parent_id.is_some() {
                    let conn = state.read_db.lock().unwrap();
                    let affected = crate::db::queries::get_media_ids_with_tag(&conn, tag.id)
                        .unwrap_or_default();
                    drop(conn);
                    if affected.is_empty() {
                        state.pending_action = Some(crate::app::PendingAction::DeleteTag(tag.id));
                    } else {
                        fn collect_ancestors(tags: &[crate::db::queries::Tag], id: i64) -> Vec<i64> {
                            fn find(tags: &[crate::db::queries::Tag], id: i64, path: &mut Vec<i64>) -> bool {
                                for t in tags {
                                    if t.id == id { return true; }
                                    path.push(t.id);
                                    if find(&t.children, id, path) { return true; }
                                    path.pop();
                                }
                                false
                            }
                            let mut path = Vec::new();
                            find(tags, id, &mut path);
                            path
                        }
                        let parents = collect_ancestors(&state.tag_tree, tag.id);
                        state.confirm_delete_tag = Some((
                            tag.id,
                            tag.name.clone(),
                            parents,
                            affected.len(),
                        ));
                    }
                } else {
                    state.pending_action = Some(crate::app::PendingAction::DeleteTag(tag.id));
                }
                ui.close_menu();
            }
        });
    });

    if expanded {
        let children = &tag.children;
        // Filter children to those whose subtree contains a match,
        // mirroring the root-level filter.
        let visible: Vec<&Tag> = children.iter()
            .filter(|c| tag_or_descendant_matches(c, query, search_match))
            .collect();
        let vlen = visible.len();
        for (i, child) in visible.iter().enumerate() {
            show_tag_node(ui, state, child, depth + 1, exclude_mod, query, search_match);
        }
    }
}
