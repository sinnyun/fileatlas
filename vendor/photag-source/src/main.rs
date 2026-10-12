#![windows_subsystem = "windows"]

// mimalloc as the global allocator. The default Windows HeapAlloc gets
// slow when the process has tens of thousands of small live allocations
// — exactly what a media library with thousands of `MediaItem` structs
// (each holding several Strings) creates. mimalloc's per-thread free lists
// keep allocation/free at near-constant time regardless of total live
// allocation count, which the diagnostics traced as the per-frame ~60ms
// cost on the loaded library.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod ai;
mod app;
mod db;
mod drag;
mod scan;
mod session;
mod settings;
mod theme;
mod thumbs;
mod ui;
mod update;
mod video;
mod watcher;
mod winfx;
mod xmp;

use app::AppState;
use db::queries;
use session::SessionState;
use ui::sidebar::SidebarTab;
use settings::Settings;
use ui::gallery::{GalleryCache, TextureManager};
use ui::preview::PreviewState;
use video::VideoPlayer;

fn main() -> eframe::Result<()> {
    let conn = db::open_db().expect("Failed to open database");
    let mut state = AppState::new(conn);

    // Restore "where I left off" state. Folder/path selection is validated
    // against the live DB (folders may have been removed since last run).
    let session = SessionState::load();
    state.expanded_subdirs = session.expanded_subdirs.clone();
    state.expanded_tag_ids = session.expanded_tag_ids.clone();
    if session.sidebar_tab == "Tags" { state.sidebar_tab = SidebarTab::Tags; }
    let restored_folder = session.selected_folder_id
        .filter(|id| state.folders.iter().any(|f| f.id == *id));
    state.selected_folder_id = restored_folder;
    state.selected_path = session.selected_path.clone()
        .filter(|p| std::path::Path::new(p).is_dir());
    if state.selected_path.is_some() { state.selected_folder_id = None; }
    state.pending_gallery_scroll = Some(session.gallery_scroll);
    if session.sidebar_width >= 160.0 && session.sidebar_width <= 500.0 {
        state.sidebar_width = session.sidebar_width;
    }

    // Restore filters
    state.search_query = session.search_query.clone();
    if let Some(sort) = session.sort_order { state.sort_order = sort; }
    if let Some(mt) = session.media_type_filter { state.media_type_filter = mt; }
    state.selected_extensions = session.selected_extensions.clone();
    state.excluded_extensions = session.excluded_extensions.clone();
    state.date_from = session.date_from.as_deref()
        .and_then(|s| s.parse::<chrono::NaiveDate>().ok());
    state.date_to = session.date_to.as_deref()
        .and_then(|s| s.parse::<chrono::NaiveDate>().ok());
    state.rating_min = session.rating_min;
    state.rating_max = session.rating_max.unwrap_or(5);
    state.selected_tag_ids = session.selected_tag_ids.clone();
    state.excluded_tag_ids = session.excluded_tag_ids.clone();
    if let Some(ts) = session.thumb_size {
        if ts >= 80.0 && ts <= 300.0 { state.thumb_size = ts; }
    }
    state.collapsed_dropdown_tag_ids = session.collapsed_dropdown_tag_ids.clone();
    state.collapsed_months = session.collapsed_months.clone();
    if let Some(m) = session.tag_click_mode { state.tag_click_mode = m; }
    if let Some(m) = session.ext_click_mode { state.ext_click_mode = m; }

    // Filters were restored after new() already did an initial load_media(),
    // so flag dirty to re-query with the restored filter state.
    state.media_dirty = true;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0])
            // Transparent-capable so the Win11 Mica / Win10 Acrylic backdrop
            // can show through. The window only actually goes see-through once
            // a backdrop is applied (see `clear_color`); otherwise it paints
            // an opaque surface and looks like a normal window.
            .with_transparent(true)
            .with_title("PhoTag"),
        ..Default::default()
    };

    eframe::run_native(
        "PhoTag",
        options,
        Box::new(|cc| {
            theme::install_fonts(&cc.egui_ctx);
            theme::apply(&cc.egui_ctx);

            let settings = Settings::load();
            // Clear the binary the previous update left behind. Best-effort:
            // if that process is still shutting down the file is locked, and
            // the next launch picks it up instead.
            update::cleanup_previous();
            let mut update_ui = UpdateUi::default();
            if settings.auto_check_updates {
                update_ui.check_rx = Some(update::check_async());
            }
            // Apply the saved accent before the first frame so the user sees
            // their chosen color immediately. `resolve_accent_color` handles
            // the "System" sentinel by reading the live Windows accent.
            theme::set_accent(&cc.egui_ctx, theme::resolve_accent_color(&settings.accent_name));
            // Hydrate the search-scope toggles from the persisted settings so
            // a fresh launch immediately respects the user's last-chosen set.
            state.search_scopes = settings.search_scopes;

            Ok(Box::new(PhoTagApp {
                state,
                tex_mgr: TextureManager::new(),
                gallery_cache: GalleryCache::new(),
                preview: PreviewState::new(),
                video_player: VideoPlayer::new(),
                settings,
                show_settings: false,
                settings_tab: SettingsTab::Toolbar,
                capturing_open_key: false,
                backdrop: winfx::Backdrop::None,
                window_inited: false,
                update: update_ui,
                pending_install: None,
            }))
        }),
    )
}

/// "12345" → "12,345" — thousands separators for status-bar counts.
fn group_digits(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 { out.push(','); }
        out.push(ch);
    }
    out
}

/// Default position helper for modal windows. Returns the top-left
/// position that would center a window of `(w, h)` on the screen.
/// Used together with `egui::Window::default_pos(...)` so the window
/// opens centred but stays draggable (vs `.anchor()` which locks it).
fn centered_default_pos(ctx: &egui::Context, w: f32, h: f32) -> egui::Pos2 {
    let screen = ctx.screen_rect();
    egui::pos2(
        screen.center().x - w * 0.5,
        screen.center().y - h * 0.5,
    )
}

/// Bulk action modal: pick action (Delete / Copy / Move), source
/// scope (whole library / selected folder / tag / current selection),
/// and filters (date range, age, rating, name, extension, size).
/// Lives entirely on `state.bulk_actions` — clears the state when the
/// user clicks Cancel or Run.
fn show_bulk_action_modal(ctx: &egui::Context, state: &mut app::AppState) {
    let Some(mut ba) = state.bulk_actions.clone() else { return };
    let mut do_run = false;
    let mut do_close = false;

    let mut window_open = true;
    egui::Window::new("Bulk actions")
        .collapsible(false)
        .resizable(false)
        .default_width(440.0)
        .default_pos(centered_default_pos(ctx, 440.0, 480.0))
        .open(&mut window_open)
        .show(ctx, |ui| {
            // ── Action ─────────────────────────────────────────────
            ui.label(egui::RichText::new("Action").strong());
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                for opt in [
                    app::BulkActionKind::Delete,
                    app::BulkActionKind::CopyTo,
                    app::BulkActionKind::MoveTo,
                ] {
                    if ui.selectable_label(ba.action == opt, opt.label()).clicked() {
                        ba.action = opt;
                    }
                }
            });

            ui.add_space(8.0);

            // ── Source ─────────────────────────────────────────────
            ui.label(egui::RichText::new("Source").strong());
            ui.add_space(2.0);
            for opt in [
                app::BulkActionSource::WholeLibrary,
                app::BulkActionSource::SelectedFolder,
                app::BulkActionSource::SelectedTag,
                app::BulkActionSource::CurrentSelection,
            ] {
                if ui.selectable_label(ba.source == opt, opt.label()).clicked() {
                    ba.source = opt;
                }
            }

            ui.add_space(8.0);

            // ── Filters (skipped entirely when Source = Current Selection) ──
            if !matches!(ba.source, app::BulkActionSource::CurrentSelection) {
                ui.label(egui::RichText::new("Filters (AND)").strong());
                ui.add_space(2.0);
                egui::Grid::new("bulk_filter_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        // Date range
                        ui.label("Date from:");
                        ui.horizontal(|ui| {
                            bulk_date_input(ui, "ba_date_from", &mut ba.filters.date_from);
                        });
                        ui.end_row();

                        ui.label("Date to:");
                        ui.horizontal(|ui| {
                            bulk_date_input(ui, "ba_date_to", &mut ba.filters.date_to);
                        });
                        ui.end_row();

                        ui.label("Older than (days):");
                        bulk_optional_i64(ui, "ba_older", &mut ba.filters.older_than_days);
                        ui.end_row();

                        ui.label("Newer than (days):");
                        bulk_optional_i64(ui, "ba_newer", &mut ba.filters.newer_than_days);
                        ui.end_row();

                        ui.label("Rating:");
                        ui.horizontal(|ui| {
                            ui::filterbar::rating_range_slider(
                                ui,
                                &mut ba.filters.rating_min,
                                &mut ba.filters.rating_max,
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}–{}★",
                                    ba.filters.rating_min, ba.filters.rating_max,
                                ))
                                .small()
                                .color(theme::TEXT_SECONDARY),
                            );
                        });
                        ui.end_row();

                        ui.label("Name contains:");
                        ui.add(egui::TextEdit::singleline(&mut ba.filters.name_contains).desired_width(240.0));
                        ui.end_row();

                        ui.label("Extension:");
                        ui.add(
                            egui::TextEdit::singleline(&mut ba.filters.extension)
                                .hint_text("e.g. jpg")
                                .desired_width(80.0),
                        );
                        ui.end_row();

                        ui.label("Size ≥ (MB):");
                        bulk_optional_f64(ui, "ba_size_min", &mut ba.filters.size_min_mb);
                        ui.end_row();

                        ui.label("Size ≤ (MB):");
                        bulk_optional_f64(ui, "ba_size_max", &mut ba.filters.size_max_mb);
                        ui.end_row();
                    });
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // ── Live count + run / cancel ──────────────────────────
            let count = state.bulk_action_matching_ids(&ba).len();
            let action_verb = match ba.action {
                app::BulkActionKind::Delete => "Delete",
                app::BulkActionKind::CopyTo => "Copy",
                app::BulkActionKind::MoveTo => "Move",
            };
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("Matches: {} item{}",
                        count, if count == 1 { "" } else { "s" }))
                    .strong()
                    .color(if count == 0 { theme::TEXT_MUTED } else { theme::accent() }),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Cancel").clicked() { do_close = true; }
                    let run_label = format!("{} {}", action_verb, count);
                    let run_btn = ui.add_enabled(
                        count > 0,
                        egui::Button::new(run_label),
                    );
                    if run_btn.clicked() { do_run = true; }
                });
            });
        });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) { do_close = true; }
    if !window_open { do_close = true; }

    if do_run {
        // Hand the configured action off to the AppState pending-
        // action queue so it dispatches alongside the rest of our
        // mutation traffic (DB lock discipline, post-action refresh).
        state.pending_action = Some(app::PendingAction::ExecuteBulkAction {
            state: Box::new(ba),
        });
        state.bulk_actions = None;
    } else if do_close {
        state.bulk_actions = None;
    } else {
        // Persist the in-flight edits back to AppState for next frame.
        state.bulk_actions = Some(ba);
    }
}

/// Small date input used inside the bulk action modal's Grid. Renders
/// the same DatePickerButton pattern as the filter bar but compressed
/// to fit a grid cell.
fn bulk_date_input(
    ui: &mut egui::Ui,
    id_source: &str,
    value: &mut Option<chrono::NaiveDate>,
) {
    use egui_extras::DatePickerButton;
    let today = chrono::Local::now().date_naive();
    let mut scratch = value.unwrap_or(today);
    let before = scratch;
    if value.is_none() {
        if ui.button("Set…").clicked() {
            *value = Some(scratch);
        }
    } else {
        ui.add(
            DatePickerButton::new(&mut scratch)
                .id_salt(id_source)
                .format("%Y-%m-%d"),
        );
        if scratch != before { *value = Some(scratch); }
        if ui.small_button("✕").clicked() { *value = None; }
    }
}

/// Optional integer input (e.g. days). Empty box = no filter.
fn bulk_optional_i64(ui: &mut egui::Ui, id_source: &str, value: &mut Option<i64>) {
    let mut text = match *value {
        Some(v) => v.to_string(),
        None => String::new(),
    };
    let resp = ui.add(
        egui::TextEdit::singleline(&mut text)
            .id_salt(id_source)
            .hint_text("—")
            .desired_width(80.0),
    );
    if resp.changed() {
        *value = text.trim().parse::<i64>().ok().filter(|&v| v >= 0);
    }
}

fn bulk_optional_f64(ui: &mut egui::Ui, id_source: &str, value: &mut Option<f64>) {
    let mut text = match *value {
        Some(v) => format!("{}", v),
        None => String::new(),
    };
    let resp = ui.add(
        egui::TextEdit::singleline(&mut text)
            .id_salt(id_source)
            .hint_text("—")
            .desired_width(80.0),
    );
    if resp.changed() {
        *value = text.trim().parse::<f64>().ok().filter(|v| *v >= 0.0);
    }
}

/// Recursively gather every descendant id of `target_id` into `out`
/// (target_id itself is NOT inserted — caller is expected to handle it
/// if needed). Used by the merge-tag picker to exclude the source's
/// subtree from the target list.
fn collect_tag_subtree_ids(
    tags: &[db::queries::Tag],
    target_id: i64,
    out: &mut std::collections::HashSet<i64>,
) {
    fn descend(tag: &db::queries::Tag, out: &mut std::collections::HashSet<i64>) {
        out.insert(tag.id);
        for c in &tag.children { descend(c, out); }
    }
    for tag in tags {
        if tag.id == target_id {
            for c in &tag.children { descend(c, out); }
            return;
        }
        collect_tag_subtree_ids(&tag.children, target_id, out);
    }
}

/// Render a flat indented list of tags eligible to be a merge target.
/// Skips entries in `excluded` (source + descendants). When `query` is
/// non-empty, only nodes whose subtree contains a name match are shown
/// — but the matching node itself is the only clickable row (we don't
/// surface non-matching ancestors as click targets, which would feel
/// arbitrary). The flat indented format trades the visual cleanliness
/// of nested popups for at-a-glance scannability.
fn walk_tag_tree_for_merge(
    ui: &mut egui::Ui,
    tags: &[db::queries::Tag],
    excluded: &std::collections::HashSet<i64>,
    query: &str,
    mode: settings::TagSearchMatch,
    depth: usize,
    on_pick: &mut dyn FnMut(i64),
    any: &mut bool,
) {
    fn subtree_has_match(
        tag: &db::queries::Tag,
        excluded: &std::collections::HashSet<i64>,
        query: &str,
        mode: settings::TagSearchMatch,
    ) -> bool {
        if !excluded.contains(&tag.id) && mode.matches(&tag.name, query) {
            return true;
        }
        tag.children.iter().any(|c| subtree_has_match(c, excluded, query, mode))
    }
    for tag in tags {
        let in_subtree = subtree_has_match(tag, excluded, query, mode);
        if !in_subtree { continue; }
        let pickable = !excluded.contains(&tag.id) && mode.matches(&tag.name, query);
        let icon = tag.icon.as_deref().filter(|s| !s.is_empty()).unwrap_or("🏷");
        let label = format!("{} {}", icon, tag.name);
        let indent = depth as f32 * 14.0;
        ui.horizontal(|ui| {
            ui.add_space(indent);
            if pickable {
                if ui.selectable_label(false, label).clicked() {
                    on_pick(tag.id);
                }
                *any = true;
            } else {
                // Non-pickable scaffolding (e.g. excluded ancestor
                // appearing only because a descendant matches).
                ui.label(egui::RichText::new(label).color(theme::TEXT_MUTED));
            }
        });
        if !tag.children.is_empty() {
            walk_tag_tree_for_merge(ui, &tag.children, excluded, query, mode, depth + 1, on_pick, any);
        }
    }
}

/// Status-bar bulk-tags menu. Each tag row shows a tri-state checkbox
/// indicating how the tag relates to the current selection:
///   ☑  every selected item has this tag
///   −  some have it, some don't (mixed)
///   ☐  no selected item has this tag
/// Clicking toggles toward "tagged":  ☐ / − → assign to all, ☑ → remove
/// from all. Recurses into the tag tree so nested hierarchies stay
/// navigable; nested rows render as flat indented entries (rather than
/// nested popups) so the user can see all descendants at once.
fn show_bulk_tags_menu(
    ui: &mut egui::Ui,
    tags: &[db::queries::Tag],
    media_ids: &[i64],
    assigned: &std::collections::HashMap<i64, std::collections::HashSet<i64>>,
    action: &mut Option<app::PendingAction>,
    collapsed: &mut std::collections::HashSet<i64>,
    filter: &str,
) {
    fn walk(
        ui: &mut egui::Ui,
        tags: &[db::queries::Tag],
        media_ids: &[i64],
        assigned: &std::collections::HashMap<i64, std::collections::HashSet<i64>>,
        action: &mut Option<app::PendingAction>,
        collapsed: &mut std::collections::HashSet<i64>,
        depth: usize,
        filter: &str,
    ) {
        fn subtree_matches(tag: &db::queries::Tag, filter: &str) -> bool {
            if filter.is_empty() { return true; }
            if tag.name.to_lowercase().contains(filter) { return true; }
            tag.children.iter().any(|c| subtree_matches(c, filter))
        }
        let total = media_ids.len();
        for tag in tags {
            if !filter.is_empty() && !subtree_matches(tag, filter) { continue; }
            let icon = tag.icon.as_deref().filter(|s| !s.is_empty()).unwrap_or("🏷");
            let count_with = media_ids.iter()
                .filter(|mid| assigned.get(mid).map_or(false, |set| set.contains(&tag.id)))
                .count();
            let glyph = if count_with == 0 { "☐" }
                else if count_with == total { "☑" }
                else { "−" };
            let suffix = if count_with == 0 || count_with == total {
                String::new()
            } else {
                format!("  ({}/{})", count_with, total)
            };
            let label = format!("{}  {}{}", glyph, tag.name, suffix);
            let indent = depth as f32 * 14.0;
            let has_children = !tag.children.is_empty();
            let bg = ui::sidebar::tag_color32(tag);

            let expanded = if !filter.is_empty() { true } else { !collapsed.contains(&tag.id) };

            ui.horizontal(|ui| {
                ui.add_space(indent);
                if has_children && filter.is_empty() {
                    let arrow = if expanded { "▼" } else { "▶" };
                    if ui.small_button(arrow).clicked() {
                        if expanded { collapsed.insert(tag.id); }
                        else { collapsed.remove(&tag.id); }
                    }
                } else {
                    ui.add_space(20.0);
                }
                ui.label(egui::RichText::new(icon).color(bg));
                if ui.selectable_label(count_with == total, label).clicked() {
                    *action = Some(if count_with == total {
                        app::PendingAction::RemoveTags(media_ids.to_vec(), vec![tag.id])
                    } else {
                        app::PendingAction::AssignTags(media_ids.to_vec(), vec![tag.id])
                    });
                }
            });
            if has_children && expanded {
                walk(ui, &tag.children, media_ids, assigned, action, collapsed, depth + 1, filter);
            }
        }
    }
    walk(ui, tags, media_ids, assigned, action, collapsed, 0, filter);
}

/// Recursive tag picker for the status-bar "🏷 Tag…" menu. Each leaf assigns
/// the chosen tag to every currently-selected media item; branches expand
/// into nested submenus so deep tag hierarchies stay navigable.
#[allow(dead_code)]
fn show_status_tags_submenu(
    ui: &mut egui::Ui,
    tags: &[db::queries::Tag],
    media_ids: &[i64],
    action: &mut Option<app::PendingAction>,
) {
    for tag in tags {
        let icon = tag.icon.as_deref().filter(|s| !s.is_empty()).unwrap_or("🏷");
        let label = format!("{} {}", icon, tag.name);
        if tag.children.is_empty() {
            if ui.button(label).clicked() {
                *action = Some(app::PendingAction::AssignTags(
                    media_ids.to_vec(), vec![tag.id],
                ));
                ui.close_menu();
            }
        } else {
            ui.menu_button(label, |ui| {
                if ui.button("Apply this tag").clicked() {
                    *action = Some(app::PendingAction::AssignTags(
                        media_ids.to_vec(), vec![tag.id],
                    ));
                    ui.close_menu();
                }
                ui.separator();
                show_status_tags_submenu(ui, &tag.children, media_ids, action);
            });
        }
    }
}

/// Curated icon presets shown in the edit-tag picker. Kept in lock-step with
/// `PRESET_ICON_NAMES`; both arrays must have the same length.
const PRESET_ICONS: &[&str] = &[
    "👤", "👪", "🐾", "🐦", "🐟",
    "🏠", "🏘", "🏙", "🏫", "🏞",
    "🍽", "🍕", "🍷", "🎂", "📷",
    "🎉", "🎢", "🎵", "🎬", "🎨",
    "🌳", "🌸", "🌊", "⛰", "🏔",
    "☀", "🌙", "🌅", "❄", "🌍",
    "✈", "🚗", "🚂", "🚌", "🛣",
    "⚽", "🏀", "🏃", "🏊", "🚴",
    "🏖", "⛺", "🎯", "📚", "📦",
    "❤", "⭐", "🏷", "🔥", "💡",
];

const PRESET_ICON_NAMES: &[&str] = &[
    "Person", "Family", "Animals", "Bird", "Fish",
    "Home", "Village", "City", "School", "Scenery",
    "Food", "Pizza", "Wine", "Birthday", "Camera",
    "Event", "Amusement", "Music", "Movie", "Art",
    "Nature", "Flower", "Sea", "Mountain", "Mountains",
    "Day", "Night", "Sunset", "Snow", "Travel",
    "Plane", "Car", "Train", "Bus", "Road",
    "Soccer", "Basketball", "Running", "Swimming", "Cycling",
    "Beach", "Camping", "Sport", "Books", "Things",
    "Love", "Star", "Tag", "Fire", "Idea",
];

#[derive(PartialEq)]
enum SettingsTab { Appearance, Toolbar, Shortcuts, Tags, AI, Updates }

/// Everything the self-updater needs across frames. Split out of
/// `PhoTagApp` so the update flow reads as one unit: check → offer →
/// download → install on exit.
#[derive(Default)]
struct UpdateUi {
    /// In-flight version check.
    check_rx: Option<std::sync::mpsc::Receiver<Result<Option<update::UpdateInfo>, String>>>,
    /// In-flight asset download.
    download_rx: Option<std::sync::mpsc::Receiver<Result<std::path::PathBuf, String>>>,
    /// Bytes fetched so far, written by the download thread.
    progress: std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// The newer release we found, if any.
    available: Option<update::UpdateInfo>,
    /// Release notes with markdown flattened, computed once when the check
    /// lands rather than on every frame the offer window is drawn.
    notes_display: String,
    /// Verified download, waiting to be swapped in on exit.
    staged: Option<std::path::PathBuf>,
    /// Last failure, shown in the Updates tab.
    error: Option<String>,
    /// True once the user dismissed the offer this session — suppresses the
    /// popup without disabling the feature.
    dismissed: bool,
    /// Whether the "update available" window is on screen.
    offer_open: bool,
    /// Set when the user explicitly pressed "Check now", so we can report
    /// "you're up to date" instead of staying silent like the startup check.
    manual_check: bool,
}

impl UpdateUi {
    fn checking(&self) -> bool { self.check_rx.is_some() }
    fn downloading(&self) -> bool { self.download_rx.is_some() }
}

struct PhoTagApp {
    state: AppState,
    tex_mgr: TextureManager,
    /// Per-frame caches for the gallery view: row layout, flat tag lookup,
    /// chip text widths. Rebuilt only when their inputs (media_gen /
    /// tag_tree_gen / layout shape) change — avoids the per-frame
    /// allocation churn that was making window-drag jittery.
    gallery_cache: GalleryCache,
    preview: PreviewState,
    video_player: VideoPlayer,
    settings: Settings,
    show_settings: bool,
    settings_tab: SettingsTab,
    /// While true, the next key press in the Shortcuts tab is captured and
    /// stored as `settings.open_key`.
    capturing_open_key: bool,
    /// Which translucent window material (if any) was applied at startup.
    backdrop: winfx::Backdrop,
    /// One-shot guard so the native-window setup (dark title bar + backdrop)
    /// runs once, after the HWND exists.
    window_inited: bool,
    /// Self-update state (check / download / staged install).
    update: UpdateUi,
    /// Set when the user confirmed "Restart & install". The swap itself runs
    /// in `on_exit`, after the session has been persisted.
    pending_install: Option<std::path::PathBuf>,
}

/// Render a key-capture row used in the Settings → Shortcuts tab.
/// Returns true if the bound key was changed this frame.
fn key_capture_row(
    ui: &mut egui::Ui,
    label: &str,
    key_str: &mut String,
    capturing: &mut bool,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(format!("{}:", label));
        let btn_label = if *capturing { "Press a key…".to_string() } else { key_str.clone() };
        if ui.button(btn_label).clicked() { *capturing = !*capturing; }
        if *capturing && ui.button("Cancel").clicked() { *capturing = false; }
    });
    if *capturing {
        let captured = ui.input(|i| {
            for ev in &i.events {
                if let egui::Event::Key { key, pressed: true, .. } = ev {
                    return Some(*key);
                }
            }
            None
        });
        if let Some(k) = captured {
            if k != egui::Key::Escape {
                *key_str = k.name().to_string();
                changed = true;
            }
            *capturing = false;
        }
    }
    changed
}


// ── Self-update ───────────────────────────────────────────────────────
impl PhoTagApp {
    /// Drain the update background channels. Cheap enough to call every
    /// frame — `try_recv` on an empty channel is just an atomic load.
    fn poll_update(&mut self) {
        if let Some(rx) = self.update.check_rx.as_ref() {
            if let Ok(result) = rx.try_recv() {
                self.update.check_rx = None;
                match result {
                    Ok(Some(info)) => {
                        self.update.error = None;
                        // Only steal focus with the popup if the user hasn't
                        // already waved this session away.
                        self.update.offer_open = !self.update.dismissed;
                        self.state.show_toast(format!("PhoTag {} is available", info.version));
                        self.update.notes_display = info.notes_plain();
                        self.update.available = Some(info);
                    }
                    Ok(None) => {
                        self.update.error = None;
                        if self.update.manual_check {
                            self.state.show_toast(format!(
                                "You're on the latest version ({})",
                                update::current_version()
                            ));
                        }
                    }
                    Err(e) => {
                        // A failed startup check is not worth interrupting
                        // anyone over — it surfaces in the Updates tab.
                        if self.update.manual_check {
                            self.state.show_toast(format!("Update check failed: {}", e));
                        }
                        self.update.error = Some(e);
                    }
                }
                self.update.manual_check = false;
            }
        }

        if let Some(rx) = self.update.download_rx.as_ref() {
            if let Ok(result) = rx.try_recv() {
                self.update.download_rx = None;
                match result {
                    Ok(path) => {
                        self.update.error = None;
                        self.update.staged = Some(path);
                    }
                    Err(e) => {
                        self.state.show_toast(format!("Download failed: {}", e));
                        self.update.error = Some(e);
                    }
                }
            }
        }
    }

    fn start_update_check(&mut self, manual: bool) {
        if self.update.checking() { return; }
        self.update.error = None;
        self.update.manual_check = manual;
        self.update.check_rx = Some(update::check_async());
    }

    fn start_update_download(&mut self) {
        let Some(info) = self.update.available.clone() else { return };
        if self.update.downloading() { return; }
        self.update.error = None;
        self.update.progress = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        self.update.download_rx =
            Some(update::download_async(info, self.update.progress.clone()));
    }

    /// The "update available" window: release notes, download progress, and
    /// the restart prompt once the new build is verified on disk.
    fn show_update_offer(&mut self, ctx: &egui::Context) {
        if !self.update.offer_open { return; }
        let Some(info) = self.update.available.clone() else { return };

        let downloading = self.update.downloading();
        let staged = self.update.staged.is_some();
        let mut window_open = true;
        let mut start_download = false;
        let mut restart_now = false;
        let mut dismiss = false;

        egui::Window::new("Update available")
            .collapsible(false)
            .resizable(false)
            .default_width(440.0)
            .default_pos(centered_default_pos(ctx, 440.0, 340.0))
            .open(&mut window_open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("PhoTag {}", info.version))
                        .size(theme::FONT_SIZE_BODY + 3.0)
                        .strong());
                    ui.label(egui::RichText::new(format!("· {:.1} MB", info.size_mb()))
                        .color(theme::TEXT_MUTED));
                });
                ui.label(egui::RichText::new(format!(
                    "You're running {}.", update::current_version()
                )).color(theme::TEXT_SECONDARY));

                if !self.update.notes_display.is_empty() {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .max_height(180.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(self.update.notes_display.as_str())
                                .color(theme::TEXT_SECONDARY));
                        });
                    ui.add_space(4.0);
                    ui.separator();
                }

                ui.add_space(8.0);

                if let Some(ref e) = self.update.error {
                    ui.label(egui::RichText::new(e.as_str()).color(theme::DANGER));
                    ui.add_space(6.0);
                }

                if downloading {
                    let done = self.update.progress.load(std::sync::atomic::Ordering::Relaxed);
                    let frac = if info.asset_size > 0 {
                        (done as f32 / info.asset_size as f32).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    ui.add(egui::ProgressBar::new(frac)
                        .show_percentage()
                        .desired_width(ui.available_width()));
                    ui.add_space(2.0);
                    ui.label(egui::RichText::new(format!(
                        "Downloading… {:.1} of {:.1} MB",
                        done as f64 / (1024.0 * 1024.0),
                        info.size_mb()
                    )).color(theme::TEXT_MUTED).size(theme::FONT_SIZE_SMALL));
                    // Keep repainting so the bar actually animates — nothing
                    // else is generating frames while a download runs.
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                } else if staged {
                    ui.label(egui::RichText::new(
                        "Downloaded and verified. PhoTag will restart to finish installing."
                    ).color(theme::TEXT_SECONDARY));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Restart & install").clicked() { restart_now = true; }
                        if ui.button("Later").clicked() { dismiss = true; }
                    });
                } else {
                    ui.horizontal(|ui| {
                        if ui.button("Update now").clicked() { start_download = true; }
                        if ui.button("Not now").clicked() { dismiss = true; }
                        ui.add_space(4.0);
                        if ui.link("View on GitHub").clicked() {
                            let url = if info.html_url.is_empty() {
                                update::releases_url()
                            } else {
                                info.html_url.clone()
                            };
                            let _ = open::that_detached(&url);
                        }
                    });
                }
            });

        if start_download { self.start_update_download(); }

        if restart_now {
            if let Some(path) = self.update.staged.clone() {
                // Defer the swap to `on_exit` so session state is written
                // first — see the comment there.
                self.pending_install = Some(path);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        // Don't let the window be dismissed mid-download; the request is
        // still in flight and closing would hide its outcome.
        if (dismiss || !window_open) && !downloading {
            self.update.offer_open = false;
            self.update.dismissed = true;
        }
    }
}

impl eframe::App for PhoTagApp {
    /// Background color used to clear the framebuffer before egui paints
    /// the frame. Defaults to black in eframe — that black is what was
    /// leaking through the ~1-pixel seam between the SidePanel and the
    /// CentralPanel as a "thick black line". Matching the panel surface
    /// makes any sub-pixel gap invisible.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // When a Mica/Acrylic backdrop is active, clear to the base surface at
        // *partial* alpha rather than fully transparent. The chrome (command
        // bar, sidebar, status bar) is deliberately frameless — it has no fill
        // of its own — so clearing to transparent would leave it sitting
        // directly on the OS material, and over a bright wallpaper the
        // secondary-weight text loses all contrast. A mostly-opaque base still
        // lets the material tint through (the Mica look) while guaranteeing
        // every label stays readable regardless of what's behind the window.
        if self.backdrop.is_translucent() {
            let b = theme::SURFACE_BASE;
            return egui::Color32::from_rgba_unmultiplied(b.r(), b.g(), b.b(), 224)
                .to_normalized_gamma_f32();
        }
        // Darkest base — shows in the gaps between the floating card panes.
        let c = theme::SURFACE_BASE;
        [
            c.r() as f32 / 255.0,
            c.g() as f32 / 255.0,
            c.b() as f32 / 255.0,
            1.0,
        ]
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // One-time native-window setup: apply the dark title bar + Mica
        // (Win11) / Acrylic (Win10) backdrop once the HWND exists, then flip
        // the theme into translucent mode if a material was actually applied.
        if !self.window_inited {
            self.window_inited = true;
            #[cfg(target_os = "windows")]
            {
                use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                if let Ok(handle) = frame.window_handle() {
                    if let RawWindowHandle::Win32(w) = handle.as_raw() {
                        self.backdrop = winfx::apply_backdrop(w.hwnd.get());
                        if self.backdrop.is_translucent() {
                            theme::set_backdrop_active(ctx, true);
                            ctx.request_repaint();
                        }
                    }
                }
            }
            #[cfg(not(target_os = "windows"))]
            let _ = &frame;
        }

        self.state.process_pending();
        self.state.poll_scan();
        self.state.poll_watcher();
        self.state.ai_mandatory_groups = self.settings.ai.mandatory_groups.clone();
        self.state.ai_auto_analyze = self.settings.ai.auto_analyze;
        self.state.poll_ai();
        self.poll_update();
        self.tex_mgr.poll(ctx);

        // Wake up periodically to check the file system watcher. Use a longer
        // interval (3s) — fs events are rare, and frequent wake-ups make the
        // UI feel laggy because egui treats them like animation frames.
        if self.state.has_watcher() {
            ctx.request_repaint_after(std::time::Duration::from_secs(3));
        }

        if self.state.media_dirty {
            self.state.load_media();
            // Smart invalidation: keep textures for items still in the list
            let valid_ids: std::collections::HashSet<i64> =
                self.state.media.iter().map(|m| m.id).collect();
            self.tex_mgr.retain_only(&valid_ids);
            // Opt #6: Pre-warm first ~60 items on folder/filter switch
            self.tex_mgr.prewarm(&self.state.media);
            // Generate thumbnails for items that are missing them
            self.state.check_missing_thumbnails();
        }

        // Keep polling while any scan is active (not just when progress is visible).
        if self.state.scan_progress.is_some() || self.state.is_scanning() {
            ctx.request_repaint();
        }
        let needs_heartbeat = self.state.scan_progress.is_some()
            || self.state.is_scanning()
            || self.tex_mgr.has_pending()
            || self.state.ai_suggestion_rx.is_some()
            || self.state.ai_downloading;
        if needs_heartbeat {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }

        // ── Unified command bar ──────────────────────────────────────
        // Row 1: wordmark left, centered search, thumb-size + settings
        //        tools right-aligned.
        // Row 2: filter chips (sort, type, ext, date, clear).
        // Row 3: active filter pills (only when filters are active).
        //
        // Visual hierarchy: the chrome (command bar, sidebar, status bar)
        // sits directly on the window base material — no fills, no borders —
        // while the gallery is the single elevated rounded card. This is the
        // Win11 Settings/Store layering: quiet chrome, content pops.
        let chrome_pane = |ix: f32, top: f32, bottom: f32| {
            egui::Frame::none()
                .inner_margin(egui::Margin { left: ix, right: ix, top, bottom })
        };

        egui::TopBottomPanel::top("command_bar")
            .frame(chrome_pane(16.0, 12.0, 8.0))
            .show_separator_line(false)
            .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Wordmark — quiet brand moment anchoring the top-left.
                ui.label(
                    egui::RichText::new("PhoTag")
                        .font(theme::semibold(15.0))
                        .color(theme::TEXT_SECONDARY),
                );
                ui.add_space(4.0);

                // Right-aligned cluster reserves its space first; the
                // remaining width is then filled left-to-right with a
                // centered search bar.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(4.0);
                    if ui.button(theme::icon(theme::ICON_SETTINGS, "⚙", 14.0, theme::TEXT_PRIMARY))
                        .on_hover_text("Settings").clicked()
                    {
                        self.show_settings = !self.show_settings;
                    }
                    ui.add_space(2.0);
                    ui.menu_button("Tools", |ui| {
                        if ui.button("🧹 Bulk actions…  (Ctrl+Shift+D)").clicked() {
                            self.state.bulk_actions = Some(app::BulkActionState::default());
                            ui.close_menu();
                        }
                        if ui.button("\u{1F4CA} Statistics").clicked() {
                            self.state.show_statistics = true;
                            ui.close_menu();
                        }
                        if ui.button("\u{1F4E4} Export metadata\u{2026}").clicked() {
                            self.state.pending_action = Some(app::PendingAction::ExportMetadata);
                            ui.close_menu();
                        }
                    });
                    ui.add_space(10.0);
                    ui::filterbar::value_slider(
                        ui, &mut self.state.thumb_size, 80.0..=300.0, 116.0,
                    );
                    ui.label(theme::icon(theme::ICON_ZOOM, "⊞", 13.0, theme::TEXT_SECONDARY))
                        .on_hover_text("Thumbnail size");
                    ui.add_space(12.0);

                    // Centered search — fills the remaining row width up
                    // to a comfortable max so it stays prominent without
                    // dominating ultra-wide windows.
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let avail = ui.available_width();
                        let search_w = avail.min(540.0).max(220.0);
                        let extra = (avail - search_w).max(0.0);
                        ui.add_space(extra * 0.5);
                        let scopes_before = self.state.search_scopes;
                        ui::filterbar::show_search(ui, &mut self.state, search_w);
                        if self.state.search_scopes != scopes_before {
                            // Persist the user's scope-toggle changes so
                            // they survive across launches.
                            self.settings.search_scopes = self.state.search_scopes;
                            self.settings.save();
                        }
                        ui.add_space(extra * 0.5);
                    });
                });
            });

            ui.add_space(6.0);
            let mut show_dates = self.settings.show_date_dividers;
            ui::filterbar::show_filters(
                ui,
                &mut self.state,
                self.settings.exclude_modifier,
                &mut show_dates,
            );
            if show_dates != self.settings.show_date_dividers {
                self.settings.show_date_dividers = show_dates;
                self.settings.save();
            }
            ui::filterbar::show_active_pills(ui, &mut self.state);
            ui.add_space(6.0);
        });

        // Status bar — three modes:
        //   1. Scan in progress  → spinner + phase + progress bar
        //   2. Items selected    → "N selected" + inline bulk actions
        //   3. Idle              → "N media files" total count
        egui::TopBottomPanel::bottom("status_bar")
            .frame(chrome_pane(16.0, 7.0, 9.0))
            .show_separator_line(false)
            .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(progress) = &self.state.scan_progress {
                    ui.spinner();
                    ui.label(egui::RichText::new(&progress.phase).color(theme::TEXT_SECONDARY));
                    let frac = progress.scanned as f32 / progress.total.max(1) as f32;
                    ui.add(
                        egui::ProgressBar::new(frac)
                            .desired_width(180.0)
                            .desired_height(6.0)
                            .fill(theme::accent())
                            .rounding(theme::rounding(3.0)),
                    );
                } else if !self.state.selected_media_ids.is_empty() {
                    let n = self.state.selected_media_ids.len();
                    ui.label(
                        egui::RichText::new(format!("{} selected", group_digits(n)))
                            .font(theme::semibold(theme::FONT_SIZE_SMALL))
                            .color(theme::accent_hover()),
                    );
                    ui.add_space(8.0);

                    if ui.small_button(theme::icon_label(
                        theme::ICON_VIEW, "👁", "Open",
                        11.0, egui::FontId::proportional(theme::FONT_SIZE_SMALL),
                        theme::TEXT_SECONDARY, theme::TEXT_PRIMARY,
                    )).on_hover_text("Open the first selected item in the preview").clicked() {
                        let id = self.state.selected_media_ids[0];
                        // Use the id-based lookup so the Open button still
                        // works when the selected item has scrolled outside
                        // the in-memory media window.
                        if let Some(item) = self.state.media_by_id(id) {
                            self.state.preview_media = Some(item);
                        }
                    }

                    let tags = self.state.tag_tree.clone();
                    let media_ids = self.state.selected_media_ids.clone();

                    // Tags dropdown — custom popup that opens ABOVE the
                    // button with a small gap so it doesn't cover the
                    // button itself (status bar sits at the bottom of
                    // the window). Toggle on click, close on outside
                    // click or after an assignment fires.
                    let tags_btn = ui.small_button(theme::icon_label(
                        theme::ICON_TAG, "🏷", "Tags",
                        11.0, egui::FontId::proportional(theme::FONT_SIZE_SMALL),
                        theme::TEXT_SECONDARY, theme::TEXT_PRIMARY,
                    ));
                    let popup_id = ui.make_persistent_id("bulk_tags_popup");
                    if tags_btn.clicked() {
                        ui.memory_mut(|m| m.toggle_popup(popup_id));
                    }
                    if ui.memory(|m| m.is_popup_open(popup_id)) {
                        let pre_pending_some = self.state.pending_action.is_some();
                        let gap = 6.0;
                        let pivot_pos = egui::pos2(tags_btn.rect.left(), tags_btn.rect.top() - gap);
                        // Scale the popup with the current window size: ~28%
                        // of the window width for the body, capped to a
                        // reasonable range, and roughly 60% of the window
                        // height for the scrollable tag list.
                        let screen = ctx.screen_rect();
                        let min_w = 240.0_f32;
                        // Leave a margin above the button so the popup never
                        // brushes the very top edge of the window.
                        let space_above = (tags_btn.rect.top() - 16.0).max(200.0);
                        let max_h = (screen.height() * 0.90).clamp(360.0, space_above);
                        let area = egui::Area::new(popup_id)
                            .order(egui::Order::Foreground)
                            .fixed_pos(pivot_pos)
                            .pivot(egui::Align2::LEFT_BOTTOM)
                            .interactable(true)
                            .show(ctx, |ui| {
                                egui::Frame::popup(ui.style()).show(ui, |ui| {
                                    ui.set_height(max_h);
                                    ui.horizontal(|ui| {
                                        ui.label("🔍");
                                        ui.add(egui::TextEdit::singleline(&mut self.state.tag_dropdown_filter)
                                            .hint_text("Filter tags…")
                                            .desired_width(180.0));
                                    });
                                    if tags.is_empty() {
                                        ui.colored_label(theme::TEXT_MUTED, "No tags yet");
                                    } else {
                                        let assigned: std::collections::HashMap<i64, std::collections::HashSet<i64>> = {
                                            let conn = self.state.read_db.lock().unwrap();
                                            let map = db::queries::get_tags_for_media_batch(&conn, &media_ids)
                                                .unwrap_or_default();
                                            map.into_iter()
                                                .map(|(mid, ts)| (mid, ts.into_iter().map(|t| t.id).collect()))
                                                .collect()
                                        };
                                        let filter_lower = self.state.tag_dropdown_filter.trim().to_lowercase();
                                        egui::ScrollArea::vertical()
                                            .max_height(max_h - 30.0)
                                            .auto_shrink([true, false])
                                            .show(ui, |ui| {
                                                show_bulk_tags_menu(
                                                    ui, &tags, &media_ids, &assigned,
                                                    &mut self.state.pending_action,
                                                    &mut self.state.collapsed_dropdown_tag_ids,
                                                    &filter_lower,
                                                );
                                            });
                                    }
                                });
                            });
                        // Keep popup open after tag assignment so the user can
                        // assign/remove multiple tags without re-opening.
                        // Close on click outside the popup AND outside the button.
                        if ui.input(|i| i.pointer.any_click()) {
                            let pos = ctx.pointer_interact_pos();
                            let inside_popup = pos.map(|p| area.response.rect.contains(p)).unwrap_or(false);
                            let inside_btn = pos.map(|p| tags_btn.rect.contains(p)).unwrap_or(false);
                            if !inside_popup && !inside_btn {
                                ui.memory_mut(|m| m.close_popup());
                            }
                        }
                    }

                    // Clear-all-tags button — wipes every tag from each
                    // selected media item. Confirmation is implicit (the
                    // user can re-tag); skip the modal for now.
                    if ui.small_button(theme::icon_label(
                        theme::ICON_CLOSE, "✕", "Clear tags",
                        10.0, egui::FontId::proportional(theme::FONT_SIZE_SMALL),
                        theme::TEXT_SECONDARY, theme::TEXT_PRIMARY,
                    ))
                        .on_hover_text(format!(
                            "Remove every tag from {} item{}",
                            media_ids.len(), if media_ids.len() == 1 { "" } else { "s" },
                        ))
                        .clicked()
                    {
                        self.state.pending_action =
                            Some(app::PendingAction::ClearMediaTags(media_ids.clone()));
                    }

                    if ui.small_button(theme::icon_label(
                        theme::ICON_DELETE, "🗑", "Delete",
                        11.0, egui::FontId::proportional(theme::FONT_SIZE_SMALL),
                        theme::TEXT_SECONDARY, theme::TEXT_PRIMARY,
                    ))
                        .on_hover_text(format!("Send {} file{} to the Recycle Bin", n, if n == 1 { "" } else { "s" }))
                        .clicked()
                    {
                        self.state.confirm_bulk_delete = Some(self.state.selected_media_ids.clone());
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(theme::icon_label(
                            theme::ICON_CLOSE, "✕", "Deselect",
                            10.0, egui::FontId::proportional(theme::FONT_SIZE_SMALL),
                            theme::TEXT_SECONDARY, theme::TEXT_PRIMARY,
                        )).on_hover_text("Clear selection").clicked() {
                            self.state.selected_media_ids.clear();
                            self.state.selection_anchor = None;
                        }
                    });
                } else {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} items in library",
                            group_digits(self.state.media_count as usize),
                        ))
                        .size(theme::FONT_SIZE_SMALL)
                        .color(theme::TEXT_MUTED),
                    );
                }
            });
        });

        // Sidebar — fixed width via `exact_width`, with a custom resize
        // handle painted as an Area on top of the panel's right edge.
        // egui's built-in `.resizable(true)` doesn't persist width
        // updates reliably with our setup, so we manage width ourselves.
        self.state.sidebar_width = self.state.sidebar_width.clamp(160.0, 500.0);
        // `show_separator_line(false)` suppresses egui's built-in panel
        // separator (the thick dark stroke on the right edge) — the sidebar
        // sits directly on the base material; the elevated gallery card
        // provides the visual boundary.
        let panel_frame = egui::Frame::none()
            .inner_margin(egui::Margin { left: 10.0, right: 6.0, top: 2.0, bottom: 8.0 });
        let sidebar_resp = egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(self.state.sidebar_width)
            .show_separator_line(false)
            .frame(panel_frame)
            .show(ctx, |ui| {
                ui::sidebar::show_sidebar(ui, &mut self.state, &self.settings);
            });
        let sidebar_rect = sidebar_resp.response.rect;

        // Custom resize handle: thin foreground Area straddling the panel's
        // right edge, constrained to the sidebar's vertical range so it
        // doesn't overlay (and intercept clicks in) the top command bar
        // or the bottom status bar.
        let handle_x = self.state.sidebar_width;
        let handle_y_top = sidebar_rect.top();
        let handle_h = sidebar_rect.height();
        let mut new_width: Option<f32> = None;
        egui::Area::new(egui::Id::new("sidebar_resize_handle"))
            .fixed_pos(egui::pos2(handle_x - 4.0, handle_y_top))
            .order(egui::Order::Foreground)
            .interactable(true)
            .show(ctx, |ui| {
                let resp = ui.allocate_response(
                    egui::vec2(8.0, handle_h),
                    egui::Sense::click_and_drag(),
                );
                let active = resp.hovered() || resp.dragged();
                if active {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                }
                // The gallery card edge already separates the regions, so
                // the seam stays invisible until the user actually hovers
                // it — then a soft hairline confirms it's grabbable.
                if active {
                    let pad_top = 72.0;
                    let pad_bot = 14.0;
                    let line_x = resp.rect.center().x;
                    let line_top = handle_y_top + pad_top;
                    let line_bot = handle_y_top + handle_h - pad_bot;
                    ui.painter().line_segment(
                        [egui::pos2(line_x, line_top),
                         egui::pos2(line_x, line_bot)],
                        egui::Stroke::new(1.0, egui::Color32::from_white_alpha(26)),
                    );
                }
                if resp.dragged() {
                    let dx = resp.drag_delta().x;
                    if dx != 0.0 {
                        new_width = Some(
                            (handle_x + dx).clamp(160.0, 500.0)
                        );
                    }
                }
            });
        if let Some(w) = new_width {
            self.state.sidebar_width = w;
            ctx.request_repaint();
        }

        // Gallery — the single elevated content card. Rounded, hairline
        // border, sitting above the base chrome.
        let gallery_frame = egui::Frame::default()
            .fill(theme::card_fill())
            .rounding(theme::rounding(theme::ROUND_XL))
            .stroke(egui::Stroke::NONE)
            .inner_margin(egui::Margin { left: 14.0, right: 10.0, top: 12.0, bottom: 10.0 })
            .outer_margin(egui::Margin { left: 0.0, right: 12.0, top: 2.0, bottom: 12.0 });
        egui::CentralPanel::default()
            .frame(gallery_frame)
            .show(ctx, |ui| {
            ui::gallery::show_gallery(
                ui,
                &mut self.state,
                &mut self.tex_mgr,
                &mut self.gallery_cache,
                self.settings.show_date_dividers,
            );
        });

        // ── Ctrl+Shift+D → open bulk action modal ────────────────────────
        // Skipped when any modal/preview/textfield is already open so it
        // doesn't grab the keypress out from under something focused.
        let any_modal_open = self.state.preview_media.is_some()
            || self.state.show_create_tag
            || self.state.confirm_delete.is_some()
            || self.state.confirm_bulk_delete.is_some()
            || self.state.confirm_delete_tag.is_some()
            || self.state.rename_media.is_some()
            || self.state.new_folder_parent.is_some()
            || self.state.edit_tag.is_some()
            || self.state.merge_tag_source.is_some()
            || self.state.bulk_actions.is_some()
            || self.state.show_statistics
            || self.show_settings;
        if !any_modal_open
            && ctx.memory(|m| m.focused().is_none())
            && ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::D))
        {
            self.state.bulk_actions = Some(app::BulkActionState::default());
        }

        // ── Ctrl+Z / Ctrl+Y undo/redo ──────────────────────────────────
        // Active whenever no text field is focused (so it doesn't steal
        // from text editing) and no confirmation modal is blocking.
        if ctx.memory(|m| m.focused().is_none()) {
            if ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Z)) {
                self.state.execute_undo();
            }
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Y)) {
                self.state.execute_redo();
            }
        }

        // ── Ctrl+A select all ───────────────────────────────────────────
        if !any_modal_open
            && self.state.preview_media.is_none()
            && ctx.memory(|m| m.focused().is_none())
            && ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::A))
        {
            self.state.selected_media_ids = self.state.media_skel.iter().map(|s| s.id).collect();
            if let Some(first) = self.state.media_skel.first() {
                self.state.selection_anchor = Some(first.id);
            }
        }

        // ── Tag hotkeys (F1-F9) ─────────────────────────────────────────────
        // When no text field is focused and no modal open, check F1-F9 for
        // tag hotkey assignments. Toggle the tag on the current selection
        // (gallery) or current preview item.
        if !any_modal_open
            && ctx.memory(|m| m.focused().is_none())
        {
            let fkeys = [
                (egui::Key::F1, "F1"), (egui::Key::F2, "F2"), (egui::Key::F3, "F3"),
                (egui::Key::F4, "F4"), (egui::Key::F5, "F5"), (egui::Key::F6, "F6"),
                (egui::Key::F7, "F7"), (egui::Key::F8, "F8"), (egui::Key::F9, "F9"),
            ];
            for (key, key_name) in fkeys {
                if ctx.input(|i| i.key_pressed(key)) {
                    if let Some(hotkey) = self.settings.tag_hotkeys.iter().find(|h| h.key == key_name) {
                        let tag_id = hotkey.tag_id;
                        // Determine the target media IDs
                        let media_ids: Vec<i64> = if let Some(ref pm) = self.state.preview_media {
                            vec![pm.id]
                        } else if !self.state.selected_media_ids.is_empty() {
                            self.state.selected_media_ids.clone()
                        } else {
                            Vec::new()
                        };
                        if !media_ids.is_empty() {
                            // Check if the tag is already on the first media item
                            let already_tagged = if self.state.preview_media.is_some() {
                                self.state.preview_tags.iter().any(|t| t.id == tag_id)
                            } else {
                                let first_id = media_ids[0];
                                self.state.media_tags.get(&first_id)
                                    .map_or(false, |tags| tags.iter().any(|t| t.id == tag_id))
                            };
                            if already_tagged {
                                self.state.pending_action = Some(app::PendingAction::RemoveTags(media_ids, vec![tag_id]));
                            } else {
                                self.state.pending_action = Some(app::PendingAction::AssignTags(media_ids, vec![tag_id]));
                            }
                        }
                    }
                    break; // only handle one F-key per frame
                }
            }
        }

        // ── Open-selected-thumbnail shortcut (configurable, default Space) ───
        // Only active when not already in preview, no modal/text field is
        // focused, and at least one item is selected.
        if self.state.preview_media.is_none()
            && !self.state.show_create_tag
            && !self.show_settings
            && self.state.confirm_delete.is_none()
            && self.state.rename_media.is_none()
            && ctx.memory(|m| m.focused().is_none())
            && !self.state.selected_media_ids.is_empty()
        {
            if let Some(key) = settings::parse_key(&self.settings.open_key) {
                if ctx.input(|i| i.key_pressed(key)) {
                    let target_id = self.state.selected_media_ids[0];
                    if let Some(item) = self.state.media_by_id(target_id) {
                        self.state.preview_media = Some(item);
                    }
                }
            }
        }

        // Create tag modal
        if self.state.show_create_tag {
            let mut window_open = true;
            egui::Window::new("Create Tag")
                .collapsible(false)
                .resizable(false)
                .default_pos(centered_default_pos(ctx, 280.0, 100.0))
                .open(&mut window_open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        let resp = ui.text_edit_singleline(&mut self.state.create_tag_name);
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            let name = self.state.create_tag_name.trim().to_string();
                            if !name.is_empty() {
                                let new_tree = {
                                    let conn = self.state.db.lock().unwrap();
                                    queries::create_tag(&conn, &name, self.state.create_tag_parent).ok();
                                    queries::get_tag_tree(&conn).unwrap_or_default()
                                };
                                self.state.set_tag_tree(new_tree);
                                self.state.show_create_tag = false;
                                self.state.create_tag_name.clear();
                            }
                        }
                        resp.request_focus();
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            let name = self.state.create_tag_name.trim().to_string();
                            if !name.is_empty() {
                                let new_tree = {
                                    let conn = self.state.db.lock().unwrap();
                                    queries::create_tag(&conn, &name, self.state.create_tag_parent).ok();
                                    queries::get_tag_tree(&conn).unwrap_or_default()
                                };
                                self.state.set_tag_tree(new_tree);
                                self.state.show_create_tag = false;
                                self.state.create_tag_name.clear();
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.state.show_create_tag = false;
                        }
                    });
                });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || !window_open {
                self.state.show_create_tag = false;
                self.state.create_tag_name.clear();
            }
        }

        // Edit-tag modal — name + appearance
        if let Some(tag_id) = self.state.edit_tag {
            let mut close = false;
            // (color, icon, display, apply_to_children, exclude_edited)
            let mut save: Option<(Option<String>, Option<String>, bool, bool, bool)> = None;
            let mut rename_to: Option<String> = None;
            // Determine whether the tag has any descendants (controls cascade UI)
            let has_children = ui::sidebar::tag_has_children(&self.state.tag_tree, tag_id);
            // Original name for change-detection on Save.
            let original_name = ui::sidebar::find_tag_name(&self.state.tag_tree, tag_id)
                .unwrap_or_default();
            let mut window_open = true;
            egui::Window::new("Edit tag")
                .collapsible(false)
                .resizable(false)
                .default_width(360.0)
                .default_pos(centered_default_pos(ctx, 360.0, 320.0))
                .open(&mut window_open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.state.edit_tag_name)
                                .desired_width(280.0),
                        );
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.state.edit_tag_color_enabled, "Color");
                        ui.add_enabled_ui(self.state.edit_tag_color_enabled, |ui| {
                            ui.color_edit_button_srgb(&mut self.state.edit_tag_color);
                        });
                    });
                    ui.add_space(4.0);
                    ui.label("Icon:");
                    ui.horizontal(|ui| {
                        let icon_resp = ui.add(egui::TextEdit::singleline(&mut self.state.edit_tag_icon)
                            .char_limit(8)
                            .desired_width(70.0));
                        if ui.button("Clear").clicked() { self.state.edit_tag_icon.clear(); }
                        // Open Windows emoji picker (Win+.) and focus the input
                        if ui.button("😀").on_hover_text("Open emoji picker").clicked() {
                            icon_resp.request_focus();
                            #[cfg(target_os = "windows")]
                            {
                                // Direct FFI call to keybd_event — no child process.
                                extern "system" {
                                    fn keybd_event(bVk: u8, bScan: u8, dwFlags: u32, dwExtraInfo: usize);
                                }
                                unsafe {
                                    keybd_event(0x5B, 0, 0, 0); // Win key down
                                    keybd_event(0xBE, 0, 0, 0); // Period key down
                                    keybd_event(0xBE, 0, 2, 0); // Period key up
                                    keybd_event(0x5B, 0, 2, 0); // Win key up
                                }
                            }
                        }
                        ui.label(egui::RichText::new("(custom emoji)").weak());
                    });
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Pick a preset:").weak());
                    egui::Grid::new("icon_picker").num_columns(8).spacing([4.0, 4.0]).show(ui, |ui| {
                        for (i, icon) in PRESET_ICONS.iter().enumerate() {
                            if ui.add_sized([28.0, 28.0],
                                egui::Button::new(egui::RichText::new(*icon).size(16.0))
                                    .min_size(egui::vec2(28.0, 28.0))
                            ).on_hover_text(PRESET_ICON_NAMES[i]).clicked() {
                                self.state.edit_tag_icon = (*icon).to_string();
                            }
                            if (i + 1) % 8 == 0 { ui.end_row(); }
                        }
                    });
                    ui.add_space(4.0);
                    ui.checkbox(&mut self.state.edit_tag_display, "Display on gallery thumbnails")
                        .on_hover_text("When off, this tag is hidden from the chips drawn on thumbnails. Still shown in the sidebar, info panel, and pickers.");
                    ui.add_space(4.0);
                    ui.label("Associated tags:");
                    ui.label(
                        egui::RichText::new("Assigning this tag will also assign these tags")
                            .size(theme::FONT_SIZE_SMALL).color(theme::TEXT_MUTED),
                    );
                    {
                        fn walk_flat(tags: &[db::queries::Tag], out: &mut Vec<(i64, String)>) {
                            for t in tags {
                                out.push((t.id, t.full_path.replace('>', " > ")));
                                walk_flat(&t.children, out);
                            }
                        }
                        let mut all_tags = Vec::new();
                        walk_flat(&self.state.tag_tree, &mut all_tags);
                        egui::ScrollArea::vertical()
                            .max_height(120.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for (tid, path) in &all_tags {
                                    if *tid == tag_id { continue; }
                                    let mut checked = self.state.edit_tag_associated.contains(tid);
                                    if ui.checkbox(&mut checked, path).changed() {
                                        if checked {
                                            self.state.edit_tag_associated.push(*tid);
                                        } else {
                                            self.state.edit_tag_associated.retain(|&id| id != *tid);
                                        }
                                    }
                                }
                            });
                    }

                    if has_children {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.checkbox(&mut self.state.edit_tag_apply_children, "Apply to children");
                        if self.state.edit_tag_apply_children {
                            ui.indent("excl_indent", |ui| {
                                ui.checkbox(&mut self.state.edit_tag_exclude_edited,
                                    "Exclude previously edited children");
                            });
                        }
                    }
                    ui.add_space(8.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            let color = if self.state.edit_tag_color_enabled {
                                Some(crate::ui::sidebar::format_hex_color(self.state.edit_tag_color))
                            } else { None };
                            let icon = {
                                let s = self.state.edit_tag_icon.trim();
                                if s.is_empty() { None } else { Some(s.to_string()) }
                            };
                            let apply = has_children && self.state.edit_tag_apply_children;
                            let excl = self.state.edit_tag_exclude_edited;
                            let disp = self.state.edit_tag_display;
                            save = Some((color, icon, disp, apply, excl));
                            // Detect rename. Empty name keeps the
                            // current one (no destructive accidents).
                            let new_name = self.state.edit_tag_name.trim().to_string();
                            if !new_name.is_empty() && new_name != original_name {
                                rename_to = Some(new_name);
                            }
                        }
                        if ui.button("Cancel").clicked() { close = true; }
                    });
                });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || !window_open { close = true; }
            if let Some((color, icon, display, apply_to_children, exclude_edited)) = save {
                {
                    let conn = self.state.db.lock().unwrap();
                    db::queries::set_associated_tags(&conn, tag_id, &self.state.edit_tag_associated).ok();
                }
                self.state.pending_action = Some(crate::app::PendingAction::UpdateTagAppearance {
                    id: tag_id, name: rename_to, color, icon, display, apply_to_children, exclude_edited,
                });
                self.state.edit_tag = None;
            } else if close {
                self.state.edit_tag = None;
            }
        }

        // ── Bulk action modal ────────────────────────────────────────────
        if self.state.bulk_actions.is_some() {
            show_bulk_action_modal(ctx, &mut self.state);
        }

        // ── Statistics modal ─────────────────────────────────────────────
        if self.state.show_statistics {
            let mut close_stats = false;
            let mut window_open = true;
            egui::Window::new("\u{1F4CA} Library Statistics")
                .collapsible(false)
                .resizable(true)
                .default_width(480.0)
                .default_pos(centered_default_pos(ctx, 480.0, 500.0))
                .open(&mut window_open)
                .show(ctx, |ui| {
                    let conn = self.state.read_db.lock().unwrap();
                    let stats = queries::get_statistics(&conn);
                    drop(conn);
                    match stats {
                        Ok(stats) => {
                            ui.heading("Overview");
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(format!("Photos: {}", stats.total_photos));
                                ui.add_space(16.0);
                                ui.label(format!("Videos: {}", stats.total_videos));
                                ui.add_space(16.0);
                                ui.label(format!("Total: {}", stats.total_photos + stats.total_videos));
                            });
                            let size_gb = stats.total_size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                            ui.label(format!("Total storage: {:.2} GB", size_gb));

                            ui.add_space(12.0);
                            ui.heading("Rating Distribution");
                            ui.add_space(4.0);
                            let max_rating_count = stats.rating_counts.iter().max().copied().unwrap_or(1).max(1);
                            for i in 0..6 {
                                let count = stats.rating_counts[i];
                                let frac = count as f32 / max_rating_count as f32;
                                let label = if i == 0 { "Unrated".to_string() } else { format!("{} star{}", i, if i == 1 { "" } else { "s" }) };
                                ui.horizontal(|ui| {
                                    ui.label(format!("{:>8}:", label));
                                    let bar_w = 200.0 * frac;
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(200.0, 16.0),
                                        egui::Sense::hover(),
                                    );
                                    let bar_rect = egui::Rect::from_min_size(
                                        rect.min,
                                        egui::vec2(bar_w, 16.0),
                                    );
                                    let color = egui::Color32::from_rgb(
                                        (60 + i as u8 * 35).min(255),
                                        (130 + i as u8 * 20).min(255),
                                        80,
                                    );
                                    ui.painter().rect_filled(bar_rect, egui::Rounding::same(3.0), color);
                                    ui.label(format!("  {}", count));
                                });
                            }

                            ui.add_space(12.0);
                            ui.heading("Top 10 Tags");
                            ui.add_space(4.0);
                            if stats.top_tags.is_empty() {
                                ui.colored_label(egui::Color32::GRAY, "No tags assigned yet.");
                            } else {
                                let max_tag_count = stats.top_tags.first().map(|(_, c)| *c).unwrap_or(1).max(1);
                                for (name, count) in &stats.top_tags {
                                    let frac = *count as f32 / max_tag_count as f32;
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{:>16}:", name));
                                        let bar_w = 180.0 * frac;
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::vec2(180.0, 14.0),
                                            egui::Sense::hover(),
                                        );
                                        let bar_rect = egui::Rect::from_min_size(
                                            rect.min,
                                            egui::vec2(bar_w, 14.0),
                                        );
                                        ui.painter().rect_filled(
                                            bar_rect,
                                            egui::Rounding::same(3.0),
                                            theme::accent(),
                                        );
                                        ui.label(format!("  {}", count));
                                    });
                                }
                            }

                            ui.add_space(12.0);
                            ui.heading("Photos per Month");
                            ui.add_space(4.0);
                            if stats.monthly_counts.is_empty() {
                                ui.colored_label(egui::Color32::GRAY, "No date information available.");
                            } else {
                                let max_monthly = stats.monthly_counts.iter().map(|(_, c)| *c).max().unwrap_or(1).max(1);
                                for (month, count) in &stats.monthly_counts {
                                    let frac = *count as f32 / max_monthly as f32;
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{:>8}:", month));
                                        let bar_w = 180.0 * frac;
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::vec2(180.0, 14.0),
                                            egui::Sense::hover(),
                                        );
                                        let bar_rect = egui::Rect::from_min_size(
                                            rect.min,
                                            egui::vec2(bar_w, 14.0),
                                        );
                                        ui.painter().rect_filled(
                                            bar_rect,
                                            egui::Rounding::same(3.0),
                                            egui::Color32::from_rgb(100, 160, 220),
                                        );
                                        ui.label(format!("  {}", count));
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            ui.colored_label(egui::Color32::RED, format!("Error loading statistics: {}", e));
                        }
                    }
                    ui.add_space(8.0);
                    ui.separator();
                    if ui.button("Close").clicked() { close_stats = true; }
                });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || !window_open { close_stats = true; }
            if close_stats { self.state.show_statistics = false; }
        }

        // Merge-tag picker modal — search-as-you-type list of every tag
        // except the source and its descendants. On click, queues the
        // MergeTag action with the user's preferred children-handling
        // (Re-parent / Delete) from settings.
        if let Some(source_id) = self.state.merge_tag_source {
            let source_name = ui::sidebar::find_tag_name(&self.state.tag_tree, source_id)
                .unwrap_or_else(|| "tag".to_string());
            let mut close = false;
            let mut chosen_target: Option<i64> = None;
            // Collect ids of source + descendants to exclude.
            let mut excluded: std::collections::HashSet<i64> =
                std::collections::HashSet::new();
            excluded.insert(source_id);
            collect_tag_subtree_ids(&self.state.tag_tree, source_id, &mut excluded);

            let match_mode = self.settings.tag_search_match;
            let mut window_open = true;
            egui::Window::new(format!("Merge \"{}\" into…", source_name))
                .collapsible(false)
                .resizable(false)
                .default_width(360.0)
                .default_pos(centered_default_pos(ctx, 360.0, 420.0))
                .open(&mut window_open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("🔍").size(11.0).color(theme::TEXT_SECONDARY));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.state.merge_tag_filter)
                                .hint_text("Filter…")
                                .desired_width(280.0),
                        );
                    });
                    ui.add_space(4.0);
                    let cascade_label = match self.settings.tag_merge_children {
                        settings::TagMergeChildren::Reparent =>
                            "Children of source will be re-parented under target.",
                        settings::TagMergeChildren::Delete =>
                            "Children of source will be deleted.",
                    };
                    ui.colored_label(theme::TEXT_MUTED, cascade_label);
                    ui.add_space(4.0);
                    ui.separator();
                    let q = self.state.merge_tag_filter.trim().to_string();
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            let mut any = false;
                            walk_tag_tree_for_merge(
                                ui,
                                &self.state.tag_tree,
                                &excluded,
                                &q, match_mode,
                                0,
                                &mut |id| { chosen_target = Some(id); },
                                &mut any,
                            );
                            if !any {
                                ui.colored_label(theme::TEXT_MUTED,
                                    "No matching tags. Pick a target by name.");
                            }
                        });
                    ui.add_space(4.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() { close = true; }
                    });
                });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || !window_open { close = true; }
            if let Some(target) = chosen_target {
                let reparent = self.settings.tag_merge_children
                    == settings::TagMergeChildren::Reparent;
                self.state.pending_action = Some(app::PendingAction::MergeTag {
                    source: source_id, target, reparent_children: reparent,
                });
                self.state.merge_tag_source = None;
                self.state.merge_tag_filter.clear();
            } else if close {
                self.state.merge_tag_source = None;
                self.state.merge_tag_filter.clear();
            }
        }

        // New-subfolder modal
        if self.state.new_folder_parent.is_some() {
            let parent_display = self.state.new_folder_parent.clone().unwrap_or_default();
            let mut accept = false;
            let mut cancel = false;
            let mut window_open = true;
            egui::Window::new("New folder")
                .collapsible(false)
                .resizable(false)
                .default_pos(centered_default_pos(ctx, 300.0, 120.0))
                .open(&mut window_open)
                .show(ctx, |ui| {
                    ui.colored_label(egui::Color32::GRAY, format!("In: {parent_display}"));
                    ui.add_space(4.0);
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.state.new_folder_name)
                            .hint_text("folder name")
                            .desired_width(260.0),
                    );
                    resp.request_focus();
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        accept = true;
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() { accept = true; }
                        if ui.button("Cancel").clicked() { cancel = true; }
                    });
                });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || !window_open { cancel = true; }
            if accept {
                let parent = self.state.new_folder_parent.take().unwrap_or_default();
                let name = std::mem::take(&mut self.state.new_folder_name);
                self.state.pending_action = Some(app::PendingAction::CreateFolder { parent, name });
            } else if cancel {
                self.state.new_folder_parent = None;
                self.state.new_folder_name.clear();
            }
        }

        // Bulk-delete confirmation. Keyboard: Enter accepts, Esc
        // cancels — matches the single-delete dialog.
        if let Some(ids) = self.state.confirm_bulk_delete.clone() {
            let n = ids.len();
            let mut accept = false;
            let mut cancel = false;
            let mut window_open = true;
            egui::Window::new("Delete files?")
                .collapsible(false)
                .resizable(false)
                .default_pos(centered_default_pos(ctx, 320.0, 130.0))
                .open(&mut window_open)
                .show(ctx, |ui| {
                    ui.label(format!("Send {} file{} to the Recycle Bin?",
                        n, if n == 1 { "" } else { "s" }));
                    ui.add_space(6.0);
                    ui.colored_label(egui::Color32::GRAY,
                        "They can be restored from the Recycle Bin later.");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(format!("🗑 Delete {}", n)).clicked() { accept = true; }
                        if ui.button("Cancel").clicked() { cancel = true; }
                    });
                });
            if ctx.input(|i| i.key_pressed(egui::Key::Enter))  { accept = true; }
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || !window_open { cancel = true; }
            if accept {
                self.state.confirm_bulk_delete = None;
                self.state.pending_action = Some(app::PendingAction::BulkDeleteMedia { media_ids: ids });
            } else if cancel {
                self.state.confirm_bulk_delete = None;
            }
        }

        // Confirm delete-tag with parent cleanup option.
        if let Some((tag_id, tag_name, parent_ids, count)) =
            self.state.confirm_delete_tag.clone()
        {
            let mut delete_only = false;
            let mut delete_and_clean = false;
            let mut cancel = false;
            let mut window_open = true;
            egui::Window::new("Delete tag?")
                .collapsible(false)
                .resizable(false)
                .default_pos(centered_default_pos(ctx, 380.0, 160.0))
                .open(&mut window_open)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "\"{}\" is assigned to {} image{}.",
                        tag_name, count, if count == 1 { "" } else { "s" },
                    ));
                    ui.add_space(4.0);
                    ui.label("Also remove parent tags from those images?");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Remove parent tags too").clicked() { delete_and_clean = true; }
                        if ui.button("Keep parent tags").clicked() { delete_only = true; }
                        if ui.button("Cancel").clicked() { cancel = true; }
                    });
                });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || !window_open { cancel = true; }
            if delete_and_clean {
                self.state.confirm_delete_tag = None;
                self.state.pending_action = Some(app::PendingAction::DeleteTagAndCleanParents {
                    tag_id, parent_ids,
                });
            } else if delete_only {
                self.state.confirm_delete_tag = None;
                self.state.pending_action = Some(app::PendingAction::DeleteTag(tag_id));
            } else if cancel {
                self.state.confirm_delete_tag = None;
            }
        }

        // Confirm-delete modal — keyboard: Enter accepts, Esc cancels.
        if let Some(media_id) = self.state.confirm_delete {
            let in_preview = self.state.preview_media.is_some();
            // Filename for the confirm-delete dialog: prefer the windowed
            // copy if the item is currently in memory, fall back to the
            // preview state, then to a generic placeholder. Avoids a DB
            // hit on the confirm path.
            let filename = self.state.media.iter()
                .find(|m| m.id == media_id)
                .map(|m| m.filename.clone())
                .or_else(|| self.state.preview_media.as_ref()
                    .filter(|m| m.id == media_id).map(|m| m.filename.clone()))
                .unwrap_or_else(|| format!("media #{media_id}"));

            let mut accept = false;
            let mut cancel = false;
            let mut window_open = true;
            egui::Window::new("Delete file?")
                .collapsible(false)
                .resizable(false)
                .default_pos(centered_default_pos(ctx, 320.0, 130.0))
                .open(&mut window_open)
                .show(ctx, |ui| {
                    ui.label(format!("Send \"{filename}\" to the Recycle Bin?"));
                    ui.add_space(6.0);
                    ui.colored_label(egui::Color32::GRAY, "It can be restored from the Recycle Bin later.");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("🗑 Delete").clicked() { accept = true; }
                        if ui.button("Cancel").clicked() { cancel = true; }
                    });
                });
            if ctx.input(|i| i.key_pressed(egui::Key::Enter)) { accept = true; }
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || !window_open { cancel = true; }

            if accept {
                self.state.confirm_delete = None;
                self.state.pending_action = Some(app::PendingAction::DeleteMedia {
                    media_id,
                    advance_preview: in_preview,
                });
            } else if cancel {
                self.state.confirm_delete = None;
            }
        }

        // Preview overlay (images + video player). Filmstrip-toggle
        // changes are detected after the call so we persist them via
        // the standard Settings::save() path. Toolbar position is
        // persisted on drag-stop only — saving on every drag-delta
        // frame would write to disk dozens of times for one drag.
        let mut show_film = self.settings.show_filmstrip;
        let mut tb_pos = self.settings.toolbar_pos;
        ui::preview::show_preview(
            ctx,
            &mut self.state,
            &mut self.preview,
            &mut self.video_player,
            &mut self.tex_mgr,
            &self.settings.toolbar,
            &mut show_film,
            &mut tb_pos,
        );
        if show_film != self.settings.show_filmstrip {
            self.settings.show_filmstrip = show_film;
            self.settings.save();
        }
        if tb_pos != self.settings.toolbar_pos {
            // Defer save until the user releases the mouse (drag end
            // or context-menu click) so a 30-frame drag isn't 30
            // file writes.
            let mouse_down = ctx.input(|i| i.pointer.primary_down());
            if !mouse_down {
                self.settings.toolbar_pos = tb_pos;
                self.settings.save();
            } else {
                // Mouse still down — keep the in-memory value
                // current so the UI reflects the live drag, but
                // don't persist yet.
                self.settings.toolbar_pos = tb_pos;
            }
        }

        // Settings window
        if self.show_settings {
            let mut changed = false;
            let mut close = false;
            const SETTINGS_W: f32 = 360.0;
            // First open: center on screen. After that the user can drag it
            // anywhere — `default_pos` only takes effect on the very first
            // frame the window exists. `.anchor(...)` locks the window in
            // place (no drag), which is why we deliberately don't use it.
            let screen = ctx.screen_rect();
            let default_pos = egui::pos2(
                screen.center().x - SETTINGS_W / 2.0,
                screen.center().y - 200.0,
            );
            let mut window_open = true;
            egui::Window::new("Settings")
                .collapsible(false)
                .resizable(false)
                .movable(true)
                .fixed_size([SETTINGS_W, 0.0])
                .default_pos(default_pos)
                .open(&mut window_open)
                .show(ctx, |ui| {
                    // ── Tab bar ──────────────────────────────────────────────
                    let tab_h = 28.0;
                    let tab_w = SETTINGS_W / 5.0;
                    ui.horizontal(|ui| {
                        ui.set_height(tab_h);
                        if ui.add_sized(
                            [tab_w, tab_h],
                            egui::SelectableLabel::new(self.settings_tab == SettingsTab::Appearance, "🎨 Appearance"),
                        ).clicked() {
                            self.settings_tab = SettingsTab::Appearance;
                        }
                        if ui.add_sized(
                            [tab_w, tab_h],
                            egui::SelectableLabel::new(self.settings_tab == SettingsTab::Toolbar, "🔧 Toolbar"),
                        ).clicked() {
                            self.settings_tab = SettingsTab::Toolbar;
                        }
                        if ui.add_sized(
                            [tab_w, tab_h],
                            egui::SelectableLabel::new(self.settings_tab == SettingsTab::Shortcuts, "⌨ Shortcuts"),
                        ).clicked() {
                            self.settings_tab = SettingsTab::Shortcuts;
                        }
                        if ui.add_sized(
                            [tab_w, tab_h],
                            egui::SelectableLabel::new(self.settings_tab == SettingsTab::Tags, "🏷 Tags"),
                        ).clicked() {
                            self.settings_tab = SettingsTab::Tags;
                        }
                        if ui.add_sized(
                            [tab_w, tab_h],
                            egui::SelectableLabel::new(self.settings_tab == SettingsTab::AI, "🤖 AI"),
                        ).clicked() {
                            self.settings_tab = SettingsTab::AI;
                        }
                        if ui.add_sized(
                            [tab_w, tab_h],
                            egui::SelectableLabel::new(self.settings_tab == SettingsTab::Updates, "⟳ Updates"),
                        ).clicked() {
                            self.settings_tab = SettingsTab::Updates;
                        }
                    });
                    ui.separator();
                    ui.add_space(4.0);

                    match self.settings_tab {
                        // ── Appearance tab ───────────────────────────────────
                        SettingsTab::Appearance => {
                            ui.label(egui::RichText::new("Accent color").strong());
                            ui.add_space(2.0);
                            ui.label(
                                egui::RichText::new(
                                    "Used for the selection bar and active filter pills."
                                ).color(egui::Color32::GRAY).small(),
                            );
                            ui.add_space(8.0);
                            let current = self.settings.accent_name.clone();
                            // System accent — tracks the live Windows accent color.
                            {
                                let selected = current == theme::SYSTEM_ACCENT_NAME;
                                let sys_color =
                                    theme::resolve_accent_color(theme::SYSTEM_ACCENT_NAME);
                                let row = ui.horizontal(|ui| {
                                    let (rect, swatch_resp) = ui.allocate_exact_size(
                                        egui::vec2(22.0, 22.0),
                                        egui::Sense::click(),
                                    );
                                    ui.painter().rect_filled(
                                        rect, egui::Rounding::same(4.0), sys_color,
                                    );
                                    if selected {
                                        ui.painter().rect_stroke(
                                            rect.expand(2.0),
                                            egui::Rounding::same(6.0),
                                            egui::Stroke::new(1.5, theme::TEXT_PRIMARY),
                                        );
                                    }
                                    ui.add_space(4.0);
                                    let label_resp = ui.selectable_label(
                                        selected, "🪟 System (Windows accent)",
                                    );
                                    swatch_resp.clicked() || label_resp.clicked()
                                });
                                if row.inner && !selected {
                                    self.settings.accent_name =
                                        theme::SYSTEM_ACCENT_NAME.to_string();
                                    theme::set_accent(ui.ctx(), sys_color);
                                    changed = true;
                                }
                            }
                            ui.add_space(2.0);
                            for preset in theme::ACCENT_PRESETS {
                                let selected = preset.name == current;
                                let row = ui.horizontal(|ui| {
                                    // Color swatch — clickable.
                                    let (rect, swatch_resp) = ui.allocate_exact_size(
                                        egui::vec2(22.0, 22.0),
                                        egui::Sense::click(),
                                    );
                                    ui.painter().rect_filled(
                                        rect, egui::Rounding::same(4.0), preset.color,
                                    );
                                    if selected {
                                        ui.painter().rect_stroke(
                                            rect.expand(2.0),
                                            egui::Rounding::same(6.0),
                                            egui::Stroke::new(1.5, theme::TEXT_PRIMARY),
                                        );
                                    }
                                    ui.add_space(4.0);
                                    let label_resp = ui.selectable_label(selected, preset.name);
                                    swatch_resp.clicked() || label_resp.clicked()
                                });
                                if row.inner && !selected {
                                    self.settings.accent_name = preset.name.to_string();
                                    theme::set_accent(ui.ctx(), preset.color);
                                    changed = true;
                                }
                            }
                        }
                        // ── Toolbar tab ──────────────────────────────────────
                        SettingsTab::Toolbar => {
                            ui.label(egui::RichText::new("Zoom controls").strong());
                            ui.add_space(2.0);
                            changed |= ui.checkbox(&mut self.settings.toolbar.zoom_mode_button, "Zoom mode toggle (Center / Cursor)").changed();
                            changed |= ui.checkbox(&mut self.settings.toolbar.zoom_reset_button, "Reset button").changed();
                            changed |= ui.checkbox(&mut self.settings.toolbar.zoom_presets, "Zoom presets dropdown").changed();
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("Photo manipulation").strong());
                            ui.add_space(2.0);
                            changed |= ui.checkbox(&mut self.settings.toolbar.rotate_buttons, "Rotation buttons").changed();
                            changed |= ui.checkbox(&mut self.settings.toolbar.flip_buttons, "Mirror buttons").changed();
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("Tags").strong());
                            ui.add_space(2.0);
                            changed |= ui.checkbox(&mut self.settings.toolbar.tags_button, "Tags dropdown").changed();
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("File").strong());
                            ui.add_space(2.0);
                            changed |= ui.checkbox(&mut self.settings.toolbar.delete_button, "Delete (recycle bin)").changed();
                            changed |= ui.checkbox(&mut self.settings.toolbar.copy_button, "Copy to…").changed();
                            changed |= ui.checkbox(&mut self.settings.toolbar.move_button, "Move to…").changed();
                            changed |= ui.checkbox(&mut self.settings.toolbar.explorer_button, "Show in Explorer (📂)").changed();
                            changed |= ui.checkbox(&mut self.settings.toolbar.open_external_button, "Open in default app (🔗)").changed();
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("View").strong());
                            ui.add_space(2.0);
                            changed |= ui.checkbox(&mut self.settings.toolbar.filmstrip_toggle, "Filmstrip toggle (🎞)").changed();
                        }
                        // ── Shortcuts tab ────────────────────────────────────
                        SettingsTab::Shortcuts => {
                            ui.label(egui::RichText::new("Tag sidebar").strong());
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                ui.label("Modifier to flip select / exclude:");
                            });
                            ui.horizontal(|ui| {
                                let cur = self.settings.exclude_modifier;
                                egui::ComboBox::from_id_salt("exclude_modifier_combo")
                                    .selected_text(cur.label())
                                    .width(80.0)
                                    .show_ui(ui, |ui| {
                                        for opt in [
                                            settings::ExcludeModifier::None,
                                            settings::ExcludeModifier::Ctrl,
                                            settings::ExcludeModifier::Shift,
                                            settings::ExcludeModifier::Alt,
                                        ] {
                                            if ui.selectable_label(cur == opt, opt.label()).clicked()
                                                && self.settings.exclude_modifier != opt
                                            {
                                                self.settings.exclude_modifier = opt;
                                                changed = true;
                                            }
                                        }
                                    });
                                ui.label("+ click  →  toggle the other action");
                            });
                            ui.add_space(4.0);
                            ui.colored_label(
                                egui::Color32::GRAY,
                                "e.g. Ctrl: plain click selects, Ctrl+click excludes (or vice-versa\ndepending on the active mode toggle in the Tags panel).",
                            );

                            ui.add_space(12.0);
                            ui.label(egui::RichText::new("Gallery").strong());
                            ui.add_space(2.0);
                            changed |= key_capture_row(
                                ui, "Open selected media",
                                &mut self.settings.open_key,
                                &mut self.capturing_open_key,
                            );
                            ui.add_space(2.0);
                            ui.colored_label(
                                egui::Color32::GRAY,
                                "Press the assigned key in the gallery to open the\nselected thumbnail in the preview view.",
                            );

                            ui.add_space(12.0);
                            ui.label(egui::RichText::new("Ratings").strong());
                            ui.add_space(2.0);
                            ui.colored_label(
                                egui::Color32::GRAY,
                                "Ratings 0–5 are bound to the number-row 0–5 keys.\nClick a star in the preview chrome to set a rating with the mouse.",
                            );

                            ui.add_space(12.0);
                            ui.label(egui::RichText::new("Tag hotkeys (F1–F9)").strong());
                            ui.add_space(2.0);
                            ui.colored_label(
                                egui::Color32::GRAY,
                                "Press an F-key in gallery or preview to toggle the assigned tag.",
                            );
                            ui.add_space(4.0);
                            // Flatten tag tree for combobox
                            fn flatten_tags(tags: &[db::queries::Tag], out: &mut Vec<(i64, String)>) {
                                for t in tags {
                                    let icon = t.icon.as_deref().filter(|s| !s.is_empty()).unwrap_or("🏷");
                                    out.push((t.id, format!("{} {}", icon, t.full_path.replace('>', " > "))));
                                    flatten_tags(&t.children, out);
                                }
                            }
                            let mut flat_tags = Vec::new();
                            flatten_tags(&self.state.tag_tree, &mut flat_tags);
                            let fkey_names = ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9"];
                            for key_name in fkey_names {
                                let current_tag_id = self.settings.tag_hotkeys.iter()
                                    .find(|h| h.key == key_name)
                                    .map(|h| h.tag_id);
                                let current_label = match current_tag_id {
                                    Some(tid) => flat_tags.iter()
                                        .find(|(id, _)| *id == tid)
                                        .map(|(_, label)| label.as_str())
                                        .unwrap_or("(deleted tag)"),
                                    None => "None",
                                };
                                ui.horizontal(|ui| {
                                    ui.label(format!("{}:", key_name));
                                    egui::ComboBox::from_id_salt(format!("hotkey_{}", key_name))
                                        .selected_text(current_label)
                                        .width(200.0)
                                        .show_ui(ui, |ui| {
                                            if ui.selectable_label(current_tag_id.is_none(), "None").clicked() {
                                                self.settings.tag_hotkeys.retain(|h| h.key != key_name);
                                                changed = true;
                                            }
                                            for (tid, label) in &flat_tags {
                                                let sel = current_tag_id == Some(*tid);
                                                if ui.selectable_label(sel, label).clicked() {
                                                    self.settings.tag_hotkeys.retain(|h| h.key != key_name);
                                                    self.settings.tag_hotkeys.push(settings::TagHotkey {
                                                        key: key_name.to_string(),
                                                        tag_id: *tid,
                                                    });
                                                    changed = true;
                                                }
                                            }
                                        });
                                });
                            }
                        }
                        // ── Tags tab ─────────────────────────────────────────
                        SettingsTab::Tags => {
                            ui.label(egui::RichText::new("Tag tree search").strong());
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                ui.label("Match mode:");
                                for opt in [
                                    settings::TagSearchMatch::Contains,
                                    settings::TagSearchMatch::StartsWith,
                                ] {
                                    if ui.selectable_label(
                                        self.settings.tag_search_match == opt,
                                        opt.label(),
                                    ).clicked() {
                                        if self.settings.tag_search_match != opt {
                                            self.settings.tag_search_match = opt;
                                            changed = true;
                                        }
                                    }
                                }
                            });
                            ui.add_space(2.0);
                            changed |= ui.checkbox(
                                &mut self.settings.tag_search_persist,
                                "Keep search query when switching tabs",
                            ).changed();

                            ui.add_space(12.0);
                            ui.label(egui::RichText::new("Merge").strong());
                            ui.add_space(2.0);
                            ui.label(
                                egui::RichText::new(
                                    "When merging a tag into another, the source's children:",
                                ).color(egui::Color32::GRAY).small(),
                            );
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                for opt in [
                                    settings::TagMergeChildren::Reparent,
                                    settings::TagMergeChildren::Delete,
                                ] {
                                    if ui.selectable_label(
                                        self.settings.tag_merge_children == opt,
                                        opt.label(),
                                    ).clicked() {
                                        if self.settings.tag_merge_children != opt {
                                            self.settings.tag_merge_children = opt;
                                            changed = true;
                                        }
                                    }
                                }
                            });
                        }
                        // ── AI tab ──────────────────────────────────────────
                        SettingsTab::AI => {
                            ui.label(egui::RichText::new("Auto-analyze").strong());
                            ui.add_space(2.0);
                            changed |= ui.checkbox(
                                &mut self.settings.ai.auto_analyze,
                                "Automatically analyze new photos in preview",
                            ).on_hover_text(
                                "When enabled, the AI will automatically suggest tags\nfor every photo you open in preview mode."
                            ).changed();
                            ui.add_space(12.0);
                            ui.label(egui::RichText::new("Mandatory tag groups").strong());
                            ui.add_space(2.0);
                            ui.label(
                                egui::RichText::new(
                                    "For each group, the AI will pick the best-matching tag\nfrom the group when analyzing an image."
                                ).color(egui::Color32::GRAY).small(),
                            );
                            ui.add_space(8.0);

                            // Flatten the tag tree for the dropdown picker.
                            fn flatten_ai_tags(tags: &[db::queries::Tag], out: &mut Vec<(String, String)>) {
                                for t in tags {
                                    let icon = t.icon.as_deref().filter(|s| !s.is_empty()).unwrap_or("🏷");
                                    let display = format!("{} {}", icon, t.full_path.replace('>', " > "));
                                    out.push((t.name.clone(), display));
                                    flatten_ai_tags(&t.children, out);
                                }
                            }
                            let mut all_tags: Vec<(String, String)> = Vec::new();
                            flatten_ai_tags(&self.state.tag_tree, &mut all_tags);

                            let mut remove_group: Option<usize> = None;
                            let mut remove_tag: Option<(usize, usize)> = None;
                            let mut add_tag_to: Option<(usize, String)> = None;
                            for (gi, group) in self.settings.ai.mandatory_groups.iter_mut().enumerate() {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label("Group:");
                                        if ui.text_edit_singleline(&mut group.name).changed() {
                                            changed = true;
                                        }
                                    });
                                    ui.add_space(2.0);
                                    for (ti, tag) in group.tags.iter().enumerate() {
                                        ui.horizontal(|ui| {
                                            ui.label(format!("  🏷 {}", tag));
                                            if ui.small_button("×").on_hover_text("Remove tag").clicked() {
                                                remove_tag = Some((gi, ti));
                                            }
                                        });
                                    }
                                    // Dropdown to pick an existing tag
                                    ui.horizontal(|ui| {
                                        ui.label("  Add:");
                                        let combo_id = egui::Id::new(("ai_group_tag_combo", gi));
                                        egui::ComboBox::from_id_salt(combo_id)
                                            .selected_text("Select tag…")
                                            .width(180.0)
                                            .show_ui(ui, |ui| {
                                                for (name, display) in &all_tags {
                                                    // Skip tags already in this group
                                                    if group.tags.iter().any(|t| t == name) {
                                                        continue;
                                                    }
                                                    if ui.selectable_label(false, display).clicked() {
                                                        add_tag_to = Some((gi, name.clone()));
                                                    }
                                                }
                                                if all_tags.is_empty() {
                                                    ui.colored_label(egui::Color32::GRAY, "No tags in library");
                                                }
                                            });
                                    });
                                    if ui.small_button("Remove group").clicked() {
                                        remove_group = Some(gi);
                                    }
                                });
                                ui.add_space(4.0);
                            }
                            if let Some((gi, name)) = add_tag_to {
                                self.settings.ai.mandatory_groups[gi].tags.push(name);
                                changed = true;
                            }
                            if let Some((gi, ti)) = remove_tag {
                                self.settings.ai.mandatory_groups[gi].tags.remove(ti);
                                changed = true;
                            }
                            if let Some(gi) = remove_group {
                                self.settings.ai.mandatory_groups.remove(gi);
                                changed = true;
                            }
                            if ui.button("+ Add group").clicked() {
                                self.settings.ai.mandatory_groups.push(settings::AiTagGroup {
                                    name: format!("Group {}", self.settings.ai.mandatory_groups.len() + 1),
                                    tags: Vec::new(),
                                });
                                changed = true;
                            }
                        }

                        // ── Updates tab ──────────────────────────────────────
                        SettingsTab::Updates => {
                            let mut do_check = false;
                            let mut do_reopen = false;

                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Version").strong());
                                ui.label(egui::RichText::new(update::current_version())
                                    .color(theme::TEXT_SECONDARY));
                            });
                            ui.add_space(6.0);

                            if ui.checkbox(
                                &mut self.settings.auto_check_updates,
                                "Check for updates on startup",
                            ).changed() {
                                changed = true;
                            }
                            ui.label(
                                egui::RichText::new(
                                    "One background request to GitHub per launch. Nothing is \
                                     downloaded or installed until you confirm."
                                ).color(egui::Color32::GRAY).small(),
                            );

                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(8.0);

                            // Status line reflects whichever phase is live.
                            if self.update.checking() {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label(egui::RichText::new("Checking for updates…")
                                        .color(theme::TEXT_SECONDARY));
                                });
                                ctx.request_repaint_after(std::time::Duration::from_millis(120));
                            } else if self.update.downloading() {
                                ui.label(egui::RichText::new("Downloading update…")
                                    .color(theme::TEXT_SECONDARY));
                            } else if self.update.staged.is_some() {
                                ui.label(egui::RichText::new(
                                    "An update is downloaded and ready to install."
                                ).color(theme::TEXT_SECONDARY));
                            } else if let Some(ref info) = self.update.available {
                                ui.label(egui::RichText::new(
                                    format!("PhoTag {} is available.", info.version)
                                ).color(theme::TEXT_SECONDARY));
                            } else if self.update.error.is_none() {
                                ui.label(egui::RichText::new("You're up to date.")
                                    .color(theme::TEXT_SECONDARY));
                            }

                            if let Some(ref e) = self.update.error {
                                ui.add_space(2.0);
                                ui.label(egui::RichText::new(e.as_str())
                                    .color(theme::DANGER).small());
                            }

                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                let busy = self.update.checking() || self.update.downloading();
                                if ui.add_enabled(!busy, egui::Button::new("Check now")).clicked() {
                                    do_check = true;
                                }
                                if self.update.available.is_some() {
                                    if ui.button("Show update…").clicked() { do_reopen = true; }
                                }
                                if ui.link("Releases on GitHub").clicked() {
                                    let _ = open::that_detached(update::releases_url());
                                }
                            });

                            ui.add_space(10.0);
                            ui.label(
                                egui::RichText::new(
                                    "Updates are downloaded from the project's GitHub releases \
                                     over HTTPS and verified against the release checksum before \
                                     anything is replaced. The previous build is kept alongside \
                                     the new one until the next launch."
                                ).color(egui::Color32::GRAY).small(),
                            );

                            if do_check { self.start_update_check(true); }
                            if do_reopen {
                                self.update.offer_open = true;
                                self.update.dismissed = false;
                            }
                        }
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        if ui.button("Close").clicked() { close = true; }
                    });
                });
            // Esc closes Settings — except while we're capturing the Open key
            // (in that capture mode, Esc is already wired to cancel the
            // capture itself, not the whole window).
            if !self.capturing_open_key && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                close = true;
            }
            if changed { self.settings.save(); }
            if close || !window_open { self.show_settings = false; }
        }

        self.show_update_offer(ctx);

        // ── Toast overlay ───────────────────────────────────────────────
        // Shows a transient message at the bottom-center for 2 seconds.
        if let Some((ref msg, instant)) = self.state.toast_message {
            let elapsed = instant.elapsed().as_secs_f32();
            if elapsed < 2.0 {
                let alpha = if elapsed > 1.5 { ((2.0 - elapsed) / 0.5 * 255.0) as u8 } else { 255 };
                egui::Area::new(egui::Id::new("toast_notification"))
                    .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::new(0.0, -50.0))
                    .order(egui::Order::Tooltip)
                    .interactable(false)
                    .show(ctx, |ui| {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgba_unmultiplied(30, 30, 30, alpha))
                            .rounding(egui::Rounding::same(6.0))
                            .inner_margin(egui::Margin::symmetric(14.0, 8.0))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(msg.as_str())
                                    .color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha)));
                            });
                    });
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            } else {
                self.state.toast_message = None;
            }
        }

    }

    /// Persist "where I left off" state so the next launch lands the user in
    /// the same folder, scroll position, and sidebar tree expansion.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let s = SessionState {
            expanded_subdirs: self.state.expanded_subdirs.clone(),
            expanded_tag_ids: self.state.expanded_tag_ids.clone(),
            selected_folder_id: self.state.selected_folder_id,
            selected_path: self.state.selected_path.clone(),
            sidebar_tab: match self.state.sidebar_tab {
                SidebarTab::Folders => "Folders".into(),
                SidebarTab::Tags => "Tags".into(),
            },
            gallery_scroll: self.state.gallery_scroll,
            sidebar_width: self.state.sidebar_width,
            // Filters
            search_query: self.state.search_query.clone(),
            sort_order: Some(self.state.sort_order),
            media_type_filter: Some(self.state.media_type_filter),
            selected_extensions: self.state.selected_extensions.clone(),
            excluded_extensions: self.state.excluded_extensions.clone(),
            date_from: self.state.date_from.map(|d| d.to_string()),
            date_to: self.state.date_to.map(|d| d.to_string()),
            rating_min: self.state.rating_min,
            rating_max: Some(self.state.rating_max),
            selected_tag_ids: self.state.selected_tag_ids.clone(),
            excluded_tag_ids: self.state.excluded_tag_ids.clone(),
            thumb_size: Some(self.state.thumb_size),
            collapsed_months: self.state.collapsed_months.clone(),
            tag_click_mode: Some(self.state.tag_click_mode),
            ext_click_mode: Some(self.state.ext_click_mode),
            collapsed_dropdown_tag_ids: self.state.collapsed_dropdown_tag_ids.clone(),
        };
        s.save();

        // Install last. The swap relaunches PhoTag, and the new process
        // reads session.json on startup — so it has to happen after the
        // save above, or the relaunched app would restore stale state.
        if let Some(path) = self.pending_install.take() {
            if let Err(e) = update::install(&path) {
                // We're on the way out with no UI left to show this in, so
                // leave a note the user will actually find.
                let log = dirs::data_local_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("PhoTag")
                    .join("update-error.txt");
                let _ = std::fs::write(&log, format!(
                    "PhoTag {} could not install update from {}\n\n{}\n",
                    update::current_version(), path.display(), e
                ));
            }
        }
    }
}
