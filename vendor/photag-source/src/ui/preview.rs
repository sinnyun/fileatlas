use crate::app::{AppState, PendingAction};
use crate::db::queries::{MediaItem, Tag};
use crate::settings::{ToolbarPosition, ToolbarSettings};
use crate::theme;
use crate::ui::gallery;
use crate::ui::sidebar::tag_color32;
use crate::video::VideoPlayer;
use egui::{self, Color32, Pos2, Vec2};
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::sync::{mpsc, Arc, Condvar, Mutex};

// ── Decode pipeline (A: neighbour pre-warm + B: progressive decode) ────────
//
// The preview maintains its own small worker pool + result channel + LRU
// cache of decoded `ColorImage`s (keyed on media id). On navigating to a
// new item we kick off a `Fast` low-resolution decode (~1/4 the natural
// size — JPEG decodes this at the /4 DCT scale, ~16× faster than full),
// upload it as the live texture, then immediately queue a `Full` decode
// that replaces the texture once it lands. After the active item reaches
// `Full` quality, we pre-warm `Full` for the prev + next neighbours so
// arrow-key navigation is effectively instant.

const PREVIEW_WORKERS: usize = 2;
/// Max number of decoded images held in the cache. Each can be tens of
/// megabytes at 4K — 8 entries gives us comfortable cover for ±1 prewarm
/// plus rapid back-and-forth without hoarding hundreds of MB indefinitely.
const MAX_IMAGE_CACHE: usize = 8;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum LoadQuality {
    Fast,
    Full,
}

struct LoadRequest {
    media_id: i64,
    path: String,
    max_dim: u32,
    quality: LoadQuality,
    /// Lower number = popped first.
    priority: u32,
}

impl PartialEq for LoadRequest { fn eq(&self, o: &Self) -> bool { self.priority == o.priority } }
impl Eq for LoadRequest {}
impl PartialOrd for LoadRequest {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(o)) }
}
impl Ord for LoadRequest {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        // BinaryHeap is a max-heap — invert so lower priority pops first.
        o.priority.cmp(&self.priority)
    }
}

struct LoadResult {
    media_id: i64,
    quality: LoadQuality,
    image: Arc<egui::ColorImage>,
}

struct WorkQueue {
    heap: Mutex<BinaryHeap<LoadRequest>>,
    cv: Condvar,
}

impl WorkQueue {
    fn new() -> Arc<Self> {
        Arc::new(Self { heap: Mutex::new(BinaryHeap::new()), cv: Condvar::new() })
    }
    fn push(&self, req: LoadRequest) {
        self.heap.lock().unwrap().push(req);
        self.cv.notify_one();
    }
    fn pop(&self) -> LoadRequest {
        let mut heap = self.heap.lock().unwrap();
        loop {
            if let Some(req) = heap.pop() { return req; }
            heap = self.cv.wait(heap).unwrap();
        }
    }
}

fn start_preview_workers(queue: Arc<WorkQueue>, tx: mpsc::Sender<LoadResult>) {
    for _ in 0..PREVIEW_WORKERS {
        let q = queue.clone();
        let tx = tx.clone();
        std::thread::spawn(move || {
            loop {
                let req = q.pop();
                if let Some(image) = gallery::load_full_image_to(&req.path, req.max_dim) {
                    let _ = tx.send(LoadResult {
                        media_id: req.media_id,
                        quality: req.quality,
                        image: Arc::new(image),
                    });
                }
            }
        });
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ZoomMode { Center, Cursor }

pub struct PreviewState {
    pub texture: Option<egui::TextureHandle>,
    /// Thumbnail texture used as an instant blurry placeholder while the
    /// full-resolution image decodes in a worker thread. Populated from
    /// the gallery's `TextureManager` cache the moment the preview opens
    /// — drops the perceived load latency from 3-4s to ~0ms.
    pub placeholder_tex: Option<egui::TextureHandle>,
    pub loading: bool,
    /// Last frame's `rotated_natural` size — used to detect the
    /// placeholder→full-image texture swap so we can re-normalise
    /// `zoom` (which is always relative to the *current* natural size)
    /// without changing the on-screen display size. Without this, a
    /// hand-zoom on the blurry placeholder becomes an extreme zoom-in
    /// the moment the full image swaps in (320-px-relative → 3840-px-
    /// relative same multiplier).
    last_rotated_natural: Vec2,
    /// Multiplier on natural pixel size: 1.0 = 100% (1 image px = 1 screen px).
    /// Only used when `fit_mode` is false.
    pub zoom: f32,
    /// When true, the image is rendered at fit-to-screen scale and `zoom` is
    /// ignored. Reset and the initial open both enter fit_mode.
    pub fit_mode: bool,
    /// Offset of the image center from the screen center, in screen pixels.
    pub pan: Vec2,
    /// Target values that `zoom`/`pan` ease toward each frame for smooth wheel zoom.
    target_zoom: f32,
    target_pan: Vec2,
    /// 0 / 90 / 180 / 270 — clockwise.
    pub rotation: i32,
    pub flip_h: bool,
    pub flip_v: bool,
    pub zoom_mode: ZoomMode,
    /// Whether the right-side info panel is visible. Toggled by the "ℹ"
    /// button on the left edge of the preview.
    pub show_info: bool,
    /// Track the last media id we've credited an "open" for, so navigating
    /// with arrow keys / clicks bumps `open_count` exactly once per item.
    pub last_opened_id: Option<i64>,
    loaded_path: String,
    /// Last instant the user moved the mouse — drives the auto-hiding
    /// chrome (filename, close button, nav arrows, toolbar fade out after
    /// ~2s of stillness).
    pub last_activity: std::time::Instant,

    // ── Decode pipeline state ───────────────────────────────────────
    /// LRU of decoded `ColorImage`s keyed on media id, alongside the
    /// quality the cached entry was decoded at.
    image_cache: HashMap<i64, (Arc<egui::ColorImage>, LoadQuality)>,
    cache_lru: VecDeque<i64>,
    /// Set of media ids with a pending (queued or in-worker) decode.
    /// Cleared on result arrival to allow re-queueing for upgrades.
    pending: HashSet<i64>,
    /// Shared work queue feeding the preview worker pool.
    queue: Arc<WorkQueue>,
    /// Receives finished decodes from worker threads.
    result_rx: mpsc::Receiver<LoadResult>,
    /// Quality of the texture currently being shown for `current_tex_id`.
    /// `None` while no texture is uploaded yet for the active media.
    current_tex_quality: Option<LoadQuality>,
    /// Media id the live `texture` corresponds to. May lag behind
    /// `last_opened_id` when the user navigates faster than decodes
    /// finish — used to gate "should I upload this result to the GPU".
    current_tex_id: Option<i64>,
    /// Sentinel: "we've already kicked off neighbour pre-warm for this
    /// id". Prevents re-queueing prewarms on every frame after `Full`
    /// arrives for the active item.
    prewarm_done_for: Option<i64>,
}

impl PreviewState {
    pub fn new() -> Self {
        let queue = WorkQueue::new();
        let (tx, rx) = mpsc::channel();
        start_preview_workers(queue.clone(), tx);
        Self {
            texture: None,
            placeholder_tex: None,
            loading: false,
            last_rotated_natural: Vec2::ZERO,
            zoom: 1.0,
            fit_mode: true,
            pan: Vec2::ZERO,
            target_zoom: 1.0,
            target_pan: Vec2::ZERO,
            rotation: 0,
            flip_h: false,
            flip_v: false,
            zoom_mode: ZoomMode::Center,
            show_info: false,
            last_opened_id: None,
            loaded_path: String::new(),
            last_activity: std::time::Instant::now(),
            image_cache: HashMap::new(),
            cache_lru: VecDeque::new(),
            pending: HashSet::new(),
            queue,
            result_rx: rx,
            current_tex_quality: None,
            current_tex_id: None,
            prewarm_done_for: None,
        }
    }

    /// Insert (or upgrade) a cached image. Quality-aware: a `Full` image
    /// always replaces a `Fast` one; a `Fast` won't downgrade an existing
    /// `Full`. Touches the LRU and evicts the oldest if cap exceeded.
    fn cache_insert(&mut self, id: i64, img: Arc<egui::ColorImage>, quality: LoadQuality) {
        if let Some((_, existing_q)) = self.image_cache.get(&id) {
            if *existing_q == LoadQuality::Full && quality == LoadQuality::Fast {
                // Don't let a slower-arriving Fast clobber an already-cached Full.
                self.cache_lru.retain(|&x| x != id);
                self.cache_lru.push_back(id);
                return;
            }
            self.cache_lru.retain(|&x| x != id);
        }
        self.image_cache.insert(id, (img, quality));
        self.cache_lru.push_back(id);
        while self.image_cache.len() > MAX_IMAGE_CACHE {
            if let Some(old) = self.cache_lru.pop_front() {
                self.image_cache.remove(&old);
            }
        }
    }

    /// Enqueue a decode request, deduplicating against in-flight loads
    /// and the cache. Returns true if a request was actually queued.
    fn enqueue_load(
        &mut self,
        media_id: i64,
        path: String,
        max_dim: u32,
        quality: LoadQuality,
        priority: u32,
    ) -> bool {
        // Skip if the cache already has the requested-or-better quality.
        if let Some((_, existing_q)) = self.image_cache.get(&media_id) {
            match (*existing_q, quality) {
                (LoadQuality::Full, _) => return false,
                (LoadQuality::Fast, LoadQuality::Fast) => return false,
                (LoadQuality::Fast, LoadQuality::Full) => {} // proceed
            }
        }
        if self.pending.contains(&media_id) { return false; }
        self.pending.insert(media_id);
        self.queue.push(LoadRequest { media_id, path, max_dim, quality, priority });
        true
    }

    /// Compute the chrome opacity (0..1). Full opacity for the first ~2s
    /// after movement, fading to 0 over the next ~0.8s of stillness.
    pub fn chrome_alpha(&self) -> f32 {
        let idle = self.last_activity.elapsed().as_secs_f32();
        if idle < 2.0 { 1.0 }
        else if idle < 2.8 { 1.0 - (idle - 2.0) / 0.8 }
        else { 0.0 }
    }

    fn reset_view(&mut self) {
        self.fit_mode = true;
        self.pan = Vec2::ZERO;
        self.target_pan = Vec2::ZERO;
        self.rotation = 0;
        self.flip_h = false;
        self.flip_v = false;
        // Clear the natural-size baseline; on the first paint after
        // navigation we'll re-seed from whatever texture is current.
        self.last_rotated_natural = Vec2::ZERO;
    }
}

fn navigate(state: &mut AppState, preview: &mut PreviewState, item: MediaItem) {
    state.preview_media = Some(item);
    preview.reset_view();
    preview.texture = None;
    preview.placeholder_tex = None;
    preview.loading = false;
    preview.loaded_path.clear();
}

fn close_preview(state: &mut AppState, preview: &mut PreviewState) {
    state.preview_media = None;
    preview.texture = None;
    preview.placeholder_tex = None;
    preview.loaded_path.clear();
    preview.loading = false;
    preview.reset_view();
}

/// Open the OS file manager with `path` highlighted (Windows: `explorer
/// /select,…`). On non-Windows builds we fall back to opening the parent
/// folder, which is the closest equivalent.
fn show_in_explorer(path: &str) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // explorer.exe expects exactly:  explorer /select,"<path>"
        // — `/select,` is its own switch+comma token followed by a quoted
        // path. `Command::arg()` would wrap the whole `/select,<path>`
        // string in quotes when the path contains spaces, which Explorer
        // misparses as a single folder path. `raw_arg` skips Rust's
        // automatic escaping so we emit the exact command-line text.
        std::process::Command::new("explorer")
            .raw_arg(format!("/select,\"{}\"", path))
            .spawn()
            .ok();
    }
    #[cfg(not(windows))]
    {
        if let Some(parent) = std::path::Path::new(path).parent() {
            open::that_detached(parent).ok();
        }
    }
}

// ── Drawing helpers ────────────────────────────────────────────────────────

/// Paint a subtle radial-style vignette by drawing four edge gradient
/// quads. Each quad fades from semi-opaque black on the screen edge to
/// fully transparent toward the center, focusing attention on the image.
fn paint_vignette(painter: &egui::Painter, screen: egui::Rect) {
    let depth = 90.0;
    let edge = Color32::from_black_alpha(110);
    let inner = Color32::from_black_alpha(0);

    let push = |a: egui::Pos2, b: egui::Pos2, c: egui::Pos2, d: egui::Pos2,
                     ca: Color32, cb: Color32, cc: Color32, cd: Color32| {
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(a, ca);
        mesh.colored_vertex(b, cb);
        mesh.colored_vertex(c, cc);
        mesh.colored_vertex(d, cd);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.add(egui::Shape::Mesh(mesh));
    };

    // Top edge: edge color along top, transparent along bottom of strip.
    push(
        screen.left_top(),
        screen.right_top(),
        egui::pos2(screen.right(), screen.top() + depth),
        egui::pos2(screen.left(),  screen.top() + depth),
        edge, edge, inner, inner,
    );
    // Bottom edge
    push(
        egui::pos2(screen.left(),  screen.bottom() - depth),
        egui::pos2(screen.right(), screen.bottom() - depth),
        screen.right_bottom(),
        screen.left_bottom(),
        inner, inner, edge, edge,
    );
    // Left edge
    push(
        screen.left_top(),
        egui::pos2(screen.left() + depth, screen.top()),
        egui::pos2(screen.left() + depth, screen.bottom()),
        screen.left_bottom(),
        edge, inner, inner, edge,
    );
    // Right edge
    push(
        egui::pos2(screen.right() - depth, screen.top()),
        screen.right_top(),
        screen.right_bottom(),
        egui::pos2(screen.right() - depth, screen.bottom()),
        inner, edge, edge, inner,
    );
}

/// Multiply the alpha component of `c` by `t` (0..1). Used to fade chrome
/// elements with the auto-hide animation.
fn fade(c: Color32, t: f32) -> Color32 {
    let a = (c.a() as f32 * t.clamp(0.0, 1.0)) as u8;
    Color32::from_rgba_premultiplied(
        ((c.r() as u32 * a as u32) / 255) as u8,
        ((c.g() as u32 * a as u32) / 255) as u8,
        ((c.b() as u32 * a as u32) / 255) as u8,
        a,
    )
}

fn draw_close_button(ui: &egui::Ui, parent_rect: egui::Rect, alpha: f32) -> egui::Rect {
    let center = egui::pos2(parent_rect.max.x - 25.0, parent_rect.min.y + 25.0);
    let size = 10.0;
    let stroke = egui::Stroke::new(2.5, fade(Color32::from_white_alpha(200), alpha));
    ui.painter().line_segment(
        [center + Vec2::new(-size, -size), center + Vec2::new(size, size)],
        stroke,
    );
    ui.painter().line_segment(
        [center + Vec2::new(size, -size), center + Vec2::new(-size, size)],
        stroke,
    );
    egui::Rect::from_center_size(center, Vec2::splat(30.0))
}

/// Draw a circular "ℹ" button on the left edge of the screen, vertically
/// centered along the top (mirror placement of the close button on the right).
/// Toggles `preview.show_info` on click. Drawn as its own foreground Area so
/// it stays clickable even when the main overlay area is consumed by drag.
fn draw_info_toggle(ctx: &egui::Context, preview: &mut PreviewState, alpha: f32) {
    if alpha <= 0.01 { return; }
    let screen = ctx.screen_rect();
    let center = egui::pos2(screen.min.x + 25.0, screen.min.y + 25.0);
    let hit_rect = egui::Rect::from_center_size(center, Vec2::splat(30.0));

    egui::Area::new(egui::Id::new("preview_info_toggle"))
        .fixed_pos(hit_rect.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let resp = ui.allocate_rect(hit_rect, egui::Sense::click());
            let hovered = resp.hovered();
            let active = preview.show_info;
            let (fill, glyph_col) = if active {
                (fade(Color32::from_rgb(60, 130, 240), alpha),
                 fade(Color32::WHITE, alpha))
            } else if hovered {
                (fade(Color32::from_white_alpha(230), alpha),
                 fade(Color32::from_gray(20), alpha))
            } else {
                (fade(Color32::from_white_alpha(180), alpha),
                 fade(Color32::from_gray(20), alpha))
            };

            ui.painter().circle_filled(center, 14.0, fill);
            ui.painter().circle_stroke(
                center,
                14.0,
                egui::Stroke::new(1.5, fade(Color32::from_white_alpha(220), alpha)),
            );
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                "i",
                egui::FontId::proportional(18.0),
                glyph_col,
            );

            if hovered {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() && alpha > 0.5 {
                preview.show_info = !preview.show_info;
            }
            if alpha > 0.5 { resp.on_hover_text("Info"); }
        });
}

fn draw_ai_button(ctx: &egui::Context, state: &mut crate::app::AppState, media_id: i64, alpha: f32) {
    if alpha <= 0.01 { return; }
    let screen = ctx.screen_rect();
    let center = egui::pos2(screen.min.x + 25.0, screen.min.y + 65.0);
    let hit_rect = egui::Rect::from_center_size(center, Vec2::splat(30.0));

    let busy = state.ai_suggestion_rx.is_some() || state.ai_downloading;

    egui::Area::new(egui::Id::new("preview_ai_button"))
        .fixed_pos(hit_rect.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let resp = ui.allocate_rect(hit_rect, egui::Sense::click());
            let hovered = resp.hovered();
            let fill = if busy {
                fade(Color32::from_rgb(240, 180, 60), alpha)
            } else if hovered {
                fade(Color32::from_rgb(60, 180, 120), alpha)
            } else {
                fade(Color32::from_white_alpha(180), alpha)
            };
            let glyph_col = if busy || hovered {
                fade(Color32::WHITE, alpha)
            } else {
                fade(Color32::from_gray(20), alpha)
            };

            ui.painter().circle_filled(center, 14.0, fill);
            ui.painter().circle_stroke(
                center, 14.0,
                egui::Stroke::new(1.5, fade(Color32::from_white_alpha(220), alpha)),
            );
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                "AI",
                egui::FontId::proportional(11.0),
                glyph_col,
            );

            if hovered {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() && alpha > 0.5 && !busy {
                state.request_ai_suggestions(media_id, false);
            }
            let tip = if busy { "Analyzing..." } else { "AI Suggest Tags" };
            if alpha > 0.5 { resp.on_hover_text(tip); }
        });
}

/// Inline AI suggestions panel anchored below the AI button on the
/// left edge of preview. Stays open until the user closes it or
/// navigates away; doesn't block interaction with the rest of the UI.
fn draw_ai_panel(ctx: &egui::Context, state: &mut crate::app::AppState, alpha: f32) {
    // Only show when there are suggestions (or inference is running)
    // for the currently-previewed image.
    let preview_id = state.preview_media.as_ref().map(|m| m.id);
    let suggestions_match = state.ai_suggestions_for == preview_id && preview_id.is_some();
    if !state.show_ai_suggestions && !state.ai_suggestion_rx.is_some() { return; }
    if !suggestions_match && state.ai_suggestion_rx.is_none() { return; }
    if alpha <= 0.01 { return; }

    let screen = ctx.screen_rect();
    let panel_x = screen.min.x + 8.0;
    let panel_y = screen.min.y + 88.0; // below the AI button (at y=65)
    let panel_w = 240.0_f32.min(screen.width() * 0.3);

    let media_id = state.ai_suggestions_for.unwrap_or(0);

    let frame_alpha = (alpha * 220.0) as u8;
    let mut apply_labels: Option<Vec<String>> = None;

    egui::Area::new(egui::Id::new("ai_suggestions_panel"))
        .fixed_pos(egui::pos2(panel_x, panel_y))
        .order(egui::Order::Foreground)
        .interactable(true)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(Color32::from_rgba_unmultiplied(25, 25, 30, frame_alpha))
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::same(10.0))
                .stroke(egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, frame_alpha)))
                .show(ui, |ui| {
                    ui.set_width(panel_w);
                    let text_col = fade(Color32::WHITE, alpha);
                    let muted_col = fade(Color32::from_gray(160), alpha);

                    // ── Header ──
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("AI Suggestions")
                            .color(text_col).strong().size(12.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("×").clicked() {
                                state.show_ai_suggestions = false;
                            }
                        });
                    });
                    ui.add_space(4.0);

                    if state.ai_suggestion_rx.is_some() {
                        // Loading state
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(egui::RichText::new("Analyzing...").color(muted_col).size(11.0));
                        });
                        return;
                    }

                    if let Some(ref suggestions) = state.ai_suggestions {
                        if suggestions.is_empty() {
                            ui.label(egui::RichText::new("No suggestions.").color(muted_col).size(11.0));
                            return;
                        }

                        let mandatory: Vec<_> = suggestions.iter().filter(|s| s.group_name.is_some()).collect();
                        let regular: Vec<_> = suggestions.iter().filter(|s| s.group_name.is_none()).collect();

                        // ── Mandatory groups ──
                        if !mandatory.is_empty() {
                            for s in &mandatory {
                                let group = s.group_name.as_deref().unwrap_or("");
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(format!("{}:", group))
                                        .color(muted_col).size(11.0));
                                    ui.label(egui::RichText::new(&s.label)
                                        .color(text_col).size(11.0));
                                    if ui.small_button("+").on_hover_text("Apply").clicked() {
                                        apply_labels = Some(vec![s.label.clone()]);
                                    }
                                });
                            }
                            ui.add_space(2.0);
                            ui.separator();
                            ui.add_space(2.0);
                        }

                        // ── Regular suggestions ──
                        for s in &regular {
                            let pct = (s.confidence * 100.0) as u32;
                            ui.horizontal(|ui| {
                                let bar_w = (panel_w - 60.0).max(80.0);
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(bar_w, 16.0), egui::Sense::hover(),
                                );
                                // Background bar
                                ui.painter().rect_filled(rect, 3.0,
                                    Color32::from_rgba_unmultiplied(60, 60, 70, frame_alpha));
                                // Fill bar
                                let fill = egui::Rect::from_min_size(
                                    rect.min, egui::vec2(bar_w * s.confidence, 16.0),
                                );
                                ui.painter().rect_filled(fill, 3.0,
                                    Color32::from_rgba_unmultiplied(80, 160, 255, frame_alpha));
                                // Label
                                ui.painter().text(
                                    rect.left_center() + egui::vec2(4.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    &s.label,
                                    egui::FontId::proportional(11.0),
                                    text_col,
                                );
                                // Percentage
                                ui.painter().text(
                                    rect.right_center() - egui::vec2(4.0, 0.0),
                                    egui::Align2::RIGHT_CENTER,
                                    format!("{}%", pct),
                                    egui::FontId::proportional(10.0),
                                    muted_col,
                                );
                                if ui.small_button("+").on_hover_text("Apply").clicked() {
                                    apply_labels = Some(vec![s.label.clone()]);
                                }
                            });
                        }

                        // ── Buttons ──
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui.small_button("Apply all").clicked() {
                                apply_labels = Some(suggestions.iter().map(|s| s.label.clone()).collect());
                            }
                            if suggestions.len() > 3 {
                                if ui.small_button("Top 3").clicked() {
                                    apply_labels = Some(suggestions.iter().take(3).map(|s| s.label.clone()).collect());
                                }
                            }
                        });
                    }
                });
        });

    if let Some(labels) = apply_labels {
        state.pending_action = Some(crate::app::PendingAction::ApplyAiSuggestions {
            media_id, labels,
        });
    }
}

fn draw_nav_arrow(ui: &egui::Ui, rect: egui::Rect, pointing_right: bool, chrome_alpha: f32) -> bool {
    if chrome_alpha <= 0.01 { return false; }
    let center = rect.center();
    let hw = 10.0;
    let hh = 18.0;
    // Chevron geometry: two line segments meeting at a tip. Top → tip → bot
    // gives a ">" or "<" shape rather than a filled triangle.
    let (top, tip, bot) = if pointing_right {
        (egui::pos2(center.x - hw, center.y - hh),
         egui::pos2(center.x + hw, center.y),
         egui::pos2(center.x - hw, center.y + hh))
    } else {
        (egui::pos2(center.x + hw, center.y - hh),
         egui::pos2(center.x - hw, center.y),
         egui::pos2(center.x + hw, center.y + hh))
    };
    let hovered = ui.ctx().input(|i| i.pointer.hover_pos()).map_or(false, |p| rect.contains(p));
    let base_alpha = if hovered { 230 } else { 140 };
    let color = fade(Color32::from_white_alpha(base_alpha), chrome_alpha);
    let stroke = egui::Stroke::new(2.5, color);
    ui.painter().line_segment([top, tip], stroke);
    ui.painter().line_segment([tip, bot], stroke);
    hovered && chrome_alpha > 0.5 && ui.ctx().input(|i| i.pointer.primary_clicked())
}

/// Render the texture as a quad with optional rotation (0/90/180/270) and flips.
fn render_image_quad(
    painter: &egui::Painter,
    tex_id: egui::TextureId,
    center: Pos2,
    visible_size: Vec2,
    rotation_steps: usize,
    flip_h: bool,
    flip_v: bool,
) {
    let half = visible_size * 0.5;
    let corners = [
        center + Vec2::new(-half.x, -half.y),
        center + Vec2::new( half.x, -half.y),
        center + Vec2::new( half.x,  half.y),
        center + Vec2::new(-half.x,  half.y),
    ];

    let mut uvs = [
        egui::pos2(0.0, 0.0),
        egui::pos2(1.0, 0.0),
        egui::pos2(1.0, 1.0),
        egui::pos2(0.0, 1.0),
    ];
    if flip_h { for uv in &mut uvs { uv.x = 1.0 - uv.x; } }
    if flip_v { for uv in &mut uvs { uv.y = 1.0 - uv.y; } }

    // Cyclic shift for rotation: each 90° CW step shifts UVs by 1 position.
    let rotated_uvs: Vec<egui::Pos2> = (0..4)
        .map(|i| uvs[(i + 4 - rotation_steps) % 4])
        .collect();

    let mut mesh = egui::Mesh::with_texture(tex_id);
    for (i, corner) in corners.iter().enumerate() {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: *corner,
            uv: rotated_uvs[i],
            color: Color32::WHITE,
        });
    }
    mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    painter.add(egui::Shape::mesh(mesh));
}

// ── Rating chrome ──────────────────────────────────────────────────────────

/// Compact star row + pick/reject glyph row, painted under the filename
/// in the preview's top chrome. Tooltip on hover lists the rating
/// hotkeys (per user request — hardcoded 0–5 keys aren't otherwise
/// surfaced anywhere in the UI).
fn draw_rating_chrome(
    ui: &egui::Ui,
    center_x: f32,
    y_top: f32,
    rating: i32,
    chrome_alpha: f32,
) -> Option<i32> {
    if chrome_alpha <= 0.01 { return None; }
    let star_font = egui::FontId::proportional(16.0);
    let star_filled = "★";
    let star_empty  = "☆";
    let star_w = 16.0;
    let star_gap = 3.0;
    let row_w = 5.0 * star_w + 4.0 * star_gap;
    let mut x = center_x - row_w * 0.5;
    let y = y_top + 9.0;

    let star_color_filled = fade(Color32::from_rgb(240, 200, 80), chrome_alpha);
    let star_color_empty  = fade(Color32::from_white_alpha(110), chrome_alpha);
    let mut clicked_rating: Option<i32> = None;
    for i in 1..=5i32 {
        let center = egui::pos2(x + star_w * 0.5, y);
        // Click target — a generous square around the glyph so users
        // don't have to pixel-hunt the star itself.
        let hit = egui::Rect::from_center_size(center, Vec2::new(star_w + star_gap, 22.0));
        let resp = ui.interact(
            hit,
            ui.id().with(("rating_star", i)),
            egui::Sense::click(),
        );
        if resp.clicked() && chrome_alpha > 0.5 {
            // Click the current rating → clear it; otherwise set to i.
            clicked_rating = Some(if rating == i { 0 } else { i });
        }
        if resp.hovered() && chrome_alpha > 0.5 {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        // Hover preview: pretend the rating is `i` while hovered so the
        // user can see what they'd be setting before clicking.
        let effective_filled = if resp.hovered() { i } else { rating };
        let glyph = if i <= effective_filled { star_filled } else { star_empty };
        let color = if i <= effective_filled { star_color_filled } else { star_color_empty };
        ui.painter().text(center, egui::Align2::CENTER_CENTER, glyph, star_font.clone(), color);
        x += star_w + star_gap;
    }
    // Hover-tooltip with hotkey hint. Allocate an interact rect sized
    // to the star row so users can hover the whole strip.
    let strip_rect = egui::Rect::from_min_size(
        egui::pos2(center_x - row_w * 0.5, y - 10.0),
        Vec2::new(row_w, 20.0),
    );
    let resp = ui.interact(
        strip_rect,
        ui.id().with(("rating_chrome_tip",)),
        egui::Sense::hover(),
    );
    if resp.hovered() && chrome_alpha > 0.5 {
        egui::show_tooltip_text(
            ui.ctx(),
            ui.layer_id(),
            egui::Id::new("rating_chrome_tip_text"),
            "Click a star or press 1-5 to set rating, 0 to clear",
        );
    }
    clicked_rating
}

// ── Filmstrip ──────────────────────────────────────────────────────────────

/// Vertical extent of the filmstrip when shown. The toolbar lifts by
/// this amount + a small gap when the strip is active so they stack
/// without overlapping.
const FILMSTRIP_H: f32 = 80.0;

/// Render a horizontal strip of small thumbnails along the bottom of
/// the screen, centred on the currently-viewed item. Returns the media
/// id of any thumbnail the user clicked on this frame; the caller then
/// navigates to it (matching arrow-key behaviour).
///
/// Thumbnails are sourced from the gallery's TextureManager so we
/// piggyback on the per-frame upload/decode pipeline already in use
/// for the grid view — no extra worker pool here. For items outside
/// the gallery's loaded media window, `state.ensure_media_window` is
/// called first so we have full `MediaItem` rows for `get_or_load`.
fn draw_filmstrip(
    ctx: &egui::Context,
    state: &mut AppState,
    tex_mgr: &mut crate::ui::gallery::TextureManager,
    current_global: Option<usize>,
    chrome_alpha: f32,
) -> Option<i64> {
    let total = state.media_skel.len();
    if total == 0 { return None; }
    let screen = ctx.screen_rect();

    // Layout constants.
    let strip_pad = 8.0;
    let thumb_size = 64.0;
    let thumb_gap = 4.0;
    let item_w = thumb_size + thumb_gap;
    let strip_y_top = screen.max.y - FILMSTRIP_H - strip_pad;

    let cur = current_global.unwrap_or(0);
    let inner_w = screen.width() - strip_pad * 2.0;
    // Spread visible_count items across the inner width, centred on `cur`.
    // +2 of headroom on each side so partial thumbs near the screen edge
    // still have something to draw without abrupt clipping.
    let visible_count = ((inner_w / item_w).floor() as usize).max(1) + 2;
    let half = visible_count / 2;
    let start = cur.saturating_sub(half);
    let end = (start + visible_count + 1).min(total);

    // Make sure those rows are hydrated as MediaItems so we can ask the
    // texture manager for their thumbnails.
    state.ensure_media_window(start, end);

    let mut clicked_id: Option<i64> = None;

    egui::Area::new(egui::Id::new("preview_filmstrip"))
        .fixed_pos(egui::pos2(screen.min.x, strip_y_top))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            // Background plate spanning full width.
            let bg = Color32::from_rgba_unmultiplied(20, 20, 25, (210.0 * chrome_alpha) as u8);
            let bg_rect = egui::Rect::from_min_size(
                egui::pos2(screen.min.x, strip_y_top),
                egui::vec2(screen.width(), FILMSTRIP_H),
            );
            ui.painter().rect_filled(bg_rect, egui::Rounding::ZERO, bg);

            // Place thumbs so the current one lands at screen centre.
            let center_x = screen.center().x;
            let cur_thumb_x = center_x - thumb_size * 0.5;
            let start_x = cur_thumb_x - (cur - start) as f32 * item_w;
            let thumb_y = strip_y_top + (FILMSTRIP_H - thumb_size) * 0.5;

            for idx in start..end {
                let x = start_x + (idx - start) as f32 * item_w;
                let thumb_rect = egui::Rect::from_min_size(
                    egui::pos2(x, thumb_y),
                    egui::Vec2::splat(thumb_size),
                );
                // Don't draw thumbs that fall completely outside the
                // screen — saves a get_or_load + image draw call per
                // frame for off-screen items.
                if thumb_rect.max.x < screen.min.x || thumb_rect.min.x > screen.max.x {
                    continue;
                }

                let item: Option<MediaItem> = state.local_media_idx(idx)
                    .and_then(|l| state.media.get(l))
                    .cloned();
                let item_id = item.as_ref().map(|m| m.id);
                let is_current = idx == cur;

                // Card background — gives a faint outline even when the
                // texture hasn't loaded yet.
                let card_a = (255.0 * chrome_alpha) as u8;
                ui.painter().rect_filled(
                    thumb_rect,
                    egui::Rounding::same(3.0),
                    Color32::from_rgba_unmultiplied(40, 40, 50, card_a / 2),
                );

                // Thumb texture (best-effort — null while loading).
                if let Some(it) = item.as_ref() {
                    if let Some(tex) = tex_mgr.get_or_load(it, 200, ctx) {
                        let tex_size = tex.size_vec2();
                        let scale = (thumb_size / tex_size.x).min(thumb_size / tex_size.y);
                        let dsize = tex_size * scale;
                        let off = (egui::Vec2::splat(thumb_size) - dsize) * 0.5;
                        let img_rect = egui::Rect::from_min_size(thumb_rect.min + off, dsize);
                        ui.painter().image(
                            tex.id(),
                            img_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::from_white_alpha(card_a),
                        );
                    }
                }

                // Selected ring on the current item.
                if is_current {
                    ui.painter().rect_stroke(
                        thumb_rect,
                        egui::Rounding::same(3.0),
                        egui::Stroke::new(2.0, fade(theme::accent(), chrome_alpha)),
                    );
                }

                // Click target.
                let resp = ui.interact(
                    thumb_rect,
                    ui.id().with(("filmstrip_thumb", idx)),
                    egui::Sense::click(),
                );
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    clicked_id = item_id;
                }
            }
        });

    clicked_id
}

// ── Main entry point ───────────────────────────────────────────────────────

pub fn show_preview(
    ctx: &egui::Context,
    state: &mut AppState,
    preview: &mut PreviewState,
    video_player: &mut VideoPlayer,
    tex_mgr: &mut crate::ui::gallery::TextureManager,
    toolbar_settings: &ToolbarSettings,
    show_filmstrip: &mut bool,
    toolbar_pos: &mut ToolbarPosition,
) {
    let Some(media) = state.preview_media.clone() else { return };
    let is_video = media.media_type == "video";
    let is_audio = media.media_type == "audio";

    // Keep `state.preview_tags` in sync with the previewed item.
    state.refresh_preview_tags(false);

    // Bump open count exactly once per (preview session, item). Tracked via
    // `last_opened_id` so navigating with arrows / clicks bumps each new
    // item once, but re-entering the panel for the same media doesn't
    // double-count.
    if preview.last_opened_id != Some(media.id) {
        let conn = state.db.lock().unwrap();
        crate::db::increment_open_count(&conn, media.id);
        drop(conn);
        preview.last_opened_id = Some(media.id);
    }

    // ── Auto-hide chrome — track mouse activity ──────────────────────────────
    // Any pointer movement (or button) keeps the chrome visible for ~2s,
    // then it fades out over the next 0.8s of stillness.
    let pointer_active = ctx.input(|i| {
        i.pointer.delta() != Vec2::ZERO
            || i.pointer.any_pressed()
            || i.pointer.any_down()
            || i.raw_scroll_delta.y != 0.0
    });
    if pointer_active {
        preview.last_activity = std::time::Instant::now();
    }
    let chrome_alpha = preview.chrome_alpha();
    // While fading, request a repaint so the alpha animates to 0 even if
    // the user is perfectly still.
    if chrome_alpha > 0.0 && chrome_alpha < 1.0 {
        ctx.request_repaint();
    } else if chrome_alpha == 1.0 {
        // Schedule a wakeup at the moment the fade should start.
        let until_fade = (2.0 - preview.last_activity.elapsed().as_secs_f32()).max(0.0);
        if until_fade > 0.0 {
            ctx.request_repaint_after(std::time::Duration::from_secs_f32(until_fade + 0.05));
        }
    }

    // ── Info button (left edge) + info panel ─────────────────────────────────
    // Drawn BEFORE the video early-return so it's available in both modes.
    draw_info_toggle(ctx, preview, chrome_alpha);
    draw_ai_button(ctx, state, media.id, chrome_alpha);
    draw_ai_panel(ctx, state, chrome_alpha);
    if preview.show_info {
        let mut open = preview.show_info;
        crate::ui::info::show_info_panel(ctx, state, &mut open);
        preview.show_info = open;
    }

    // ── EXIF Metadata panel (right side overlay, toggled by "I" key) ─────────
    if state.show_metadata_panel && media.media_type == "image" {
        state.load_exif_for(media.id);
        if let Some((_, ref exif_data)) = state.cached_exif {
            let screen = ctx.screen_rect();
            let panel_w = 280.0_f32.min(screen.width() * 0.28);
            let panel_x = screen.max.x - panel_w - 16.0;
            let panel_y = screen.min.y + 50.0;
            let panel_h = (screen.height() - panel_y - 16.0).max(120.0);
            let exif_clone = exif_data.clone();

            egui::Area::new(egui::Id::new("preview_exif_panel"))
                .fixed_pos(egui::pos2(panel_x, panel_y))
                .order(egui::Order::Foreground)
                .interactable(true)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(Color32::from_rgba_premultiplied(20, 22, 28, 220))
                        .rounding(egui::Rounding::same(8.0))
                        .stroke(egui::Stroke::new(1.0, theme::BORDER))
                        .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                        .shadow(egui::epaint::Shadow {
                            offset: Vec2::new(0.0, 4.0),
                            blur: 14.0,
                            spread: 0.0,
                            color: Color32::from_black_alpha(180),
                        })
                        .show(ui, |ui| {
                            ui.set_width(panel_w);
                            ui.set_max_height(panel_h);
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, true])
                                .max_height(panel_h)
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("EXIF Metadata").strong()
                                        .color(Color32::from_rgb(170, 200, 255)));
                                    ui.separator();
                                    ui.add_space(4.0);
                                    let kv = |ui: &mut egui::Ui, k: &str, v: &str| {
                                        if !v.is_empty() {
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new(format!("{}:", k))
                                                    .color(Color32::GRAY));
                                                ui.label(v);
                                            });
                                        }
                                    };
                                    kv(ui, "Camera", &exif_clone.camera_make);
                                    kv(ui, "Model", &exif_clone.camera_model);
                                    kv(ui, "Lens", &exif_clone.lens);
                                    kv(ui, "Focal length", &exif_clone.focal_length);
                                    kv(ui, "Aperture", &exif_clone.aperture);
                                    kv(ui, "Shutter", &exif_clone.shutter_speed);
                                    kv(ui, "ISO", &exif_clone.iso);
                                    kv(ui, "Date taken", &exif_clone.date_taken);
                                    kv(ui, "Color space", &exif_clone.color_space);
                                    kv(ui, "Software", &exif_clone.software);
                                    if let Some((w, h)) = exif_clone.dimensions {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("Dimensions:")
                                                .color(Color32::GRAY));
                                            ui.label(format!("{} x {} px", w, h));
                                        });
                                    }
                                    if exif_clone.file_size > 0 {
                                        let size_str = if exif_clone.file_size < 1024 {
                                            format!("{} B", exif_clone.file_size)
                                        } else if exif_clone.file_size < 1024 * 1024 {
                                            format!("{:.1} KB", exif_clone.file_size as f64 / 1024.0)
                                        } else {
                                            format!("{:.2} MB", exif_clone.file_size as f64 / (1024.0 * 1024.0))
                                        };
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("File size:")
                                                .color(Color32::GRAY));
                                            ui.label(size_str);
                                        });
                                    }
                                    if exif_clone.gps.is_some() {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("GPS:")
                                                .color(Color32::GRAY));
                                            ui.label("Present");
                                        });
                                    }
                                });
                        });
                });
        }
    }

    // Pre-compute prev/next nav targets — used by both the image preview
    // (further down) and the video branch (right below) so video gains
    // the same chevron arrows as image preview.
    let video_current_global = state.global_idx_of(media.id);
    let video_prev_item = video_current_global
        .filter(|&i| i > 0)
        .and_then(|i| state.media_skel.get(i - 1).map(|s| s.id))
        .and_then(|id| state.media_by_id(id));
    let video_next_item = video_current_global
        .and_then(|i| state.media_skel.get(i + 1).map(|s| s.id))
        .and_then(|id| state.media_by_id(id));

    // ── Audio preview ────────────────────────────────────────────────────────
    if is_audio {
        let screen = ctx.screen_rect();

        // Dark background
        let bg_painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background, egui::Id::new("audio_bg"),
        ));
        bg_painter.rect_filled(screen, egui::Rounding::ZERO, Color32::from_gray(24));

        // Centered audio info
        egui::Area::new(egui::Id::new("audio_preview_center"))
            .fixed_pos(screen.center() - Vec2::new(150.0, 80.0))
            .order(egui::Order::Middle)
            .interactable(true)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("\u{1F3B5}")
                            .size(64.0)
                            .color(Color32::from_gray(180)),
                    );
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(&media.filename)
                            .size(16.0)
                            .color(Color32::from_white_alpha(220)),
                    );
                    ui.add_space(16.0);
                    if ui.button(egui::RichText::new("\u{25B6}  Open in default player").size(14.0)).clicked() {
                        let _ = open::that_detached(&media.full_path);
                    }
                });
            });

        // Tag chips, rating, close button, nav arrows
        if !state.preview_tags.is_empty() {
            draw_tag_chips(ctx, state, media.id);
        }

        // Rating
        {
            egui::Area::new(egui::Id::new("audio_rating_area"))
                .fixed_pos(egui::pos2(screen.center().x - 60.0, screen.min.y + 50.0))
                .order(egui::Order::Tooltip)
                .interactable(true)
                .show(ctx, |ui| {
                    let clicked_rating = draw_rating_chrome(ui, 60.0, 0.0, media.rating, 1.0);
                    if let Some(new_rating) = clicked_rating {
                        state.pending_action = Some(PendingAction::SetRating {
                            media_ids: vec![media.id],
                            rating: new_rating,
                        });
                    }
                });
        }

        // Close button
        let mut hit_close = false;
        {
            let close_center = egui::pos2(screen.max.x - 25.0, screen.min.y + 25.0);
            let close_rect = egui::Rect::from_center_size(close_center, Vec2::splat(36.0));
            egui::Area::new(egui::Id::new("audio_close_area"))
                .fixed_pos(close_rect.min)
                .order(egui::Order::Tooltip)
                .interactable(true)
                .show(ctx, |ui| {
                    let (_, resp) = ui.allocate_exact_size(close_rect.size(), egui::Sense::click());
                    if resp.clicked() { hit_close = true; }
                    let alpha = chrome_alpha;
                    if alpha > 0.0 {
                        ui.painter().circle_filled(close_center, 16.0, Color32::from_black_alpha((160.0 * alpha) as u8));
                        ui.painter().text(
                            close_center, egui::Align2::CENTER_CENTER,
                            "\u{2715}", egui::FontId::proportional(18.0),
                            Color32::from_white_alpha((230.0 * alpha) as u8),
                        );
                    }
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                });
        }

        // Nav arrows
        let mut nav_target: Option<MediaItem> = None;
        let arrow_w = 60.0;
        let prev_zone = egui::Rect::from_min_size(
            screen.min + Vec2::new(0.0, screen.height() * 0.25),
            Vec2::new(arrow_w, screen.height() * 0.5),
        );
        let next_zone = egui::Rect::from_min_size(
            egui::pos2(screen.max.x - arrow_w, screen.min.y + screen.height() * 0.25),
            Vec2::new(arrow_w, screen.height() * 0.5),
        );
        let audio_prev = video_prev_item.clone();
        let audio_next = video_next_item.clone();
        egui::Area::new(egui::Id::new("audio_nav_arrows"))
            .fixed_pos(screen.min)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.set_min_size(screen.size());
                if audio_prev.is_some() && draw_nav_arrow(ui, prev_zone, false, chrome_alpha) {
                    nav_target = audio_prev.clone();
                }
                if audio_next.is_some() && draw_nav_arrow(ui, next_zone, true, chrome_alpha) {
                    nav_target = audio_next.clone();
                }
            });

        // Escape / click outside close
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) { hit_close = true; }
        if let Some(item) = nav_target {
            navigate(state, preview, item);
            return;
        }
        if hit_close {
            state.preview_media = None;
        }
        // Keyboard nav
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            if let Some(item) = video_prev_item {
                navigate(state, preview, item);
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            if let Some(item) = video_next_item {
                navigate(state, preview, item);
            }
        }
        return;
    }

    // ── Video preview ────────────────────────────────────────────────────────
    if is_video {
        if !video_player.is_open() {
            video_player.open(&media.full_path);
            if !video_player.is_open() {
                state.preview_media = None;
                return;
            }
        }
        video_player.poll(ctx);
        let video_keep_open = video_player.show_ui(ctx);

        // ── Video chrome: tag chips, rating, filename, toolbar ─────────
        let v_chip_rect = if !state.preview_tags.is_empty() {
            draw_tag_chips(ctx, state, media.id)
        } else {
            egui::Rect::NOTHING
        };
        // Rating + filename — small Area anchored at top-center,
        // sized just to fit its content (not full-screen) so it doesn't
        // steal clicks from the video overlay underneath.
        {
            let v_screen = ctx.screen_rect();
            let v_stars_y = if v_chip_rect != egui::Rect::NOTHING {
                v_chip_rect.max.y + 2.0
            } else {
                v_screen.min.y + 8.0
            };
            egui::Area::new(egui::Id::new("video_rating_area"))
                .fixed_pos(egui::pos2(v_screen.center().x - 60.0, v_stars_y))
                .order(egui::Order::Tooltip)
                .interactable(true)
                .show(ctx, |ui| {
                    let clicked_rating = draw_rating_chrome(
                        ui, 60.0, 0.0, media.rating, 1.0,
                    );
                    if let Some(new_rating) = clicked_rating {
                        state.pending_action = Some(crate::app::PendingAction::SetRating {
                            media_ids: vec![media.id],
                            rating: new_rating,
                        });
                    }
                });
            // Filename painted directly (no interaction needed).
            let v_name_y = v_stars_y + 22.0;
            let fg = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("video_filename_layer"),
            ));
            fg.text(
                egui::pos2(v_screen.center().x, v_name_y),
                egui::Align2::CENTER_CENTER,
                &media.filename,
                egui::FontId::proportional(14.0),
                Color32::from_white_alpha(220),
            );
        }

        // ── Video toolbar (Tags + rating hotkeys + delete) ──────────────
        // Compact toolbar above the video controls row.
        {
            let v_screen = ctx.screen_rect();
            egui::Area::new(egui::Id::new("video_toolbar"))
                .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, -100.0))
                .order(egui::Order::Tooltip)
                .interactable(true)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(Color32::from_black_alpha(160))
                        .rounding(egui::Rounding::same(8.0))
                        .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                // ── Tags button ──
                                let tag_tree = state.tag_tree.clone();
                                let assigned: std::collections::HashSet<i64> =
                                    state.preview_tags.iter().map(|t| t.id).collect();
                                let mut pending: Option<PendingAction> = None;
                                let mut collapsed = state.collapsed_dropdown_tag_ids.clone();
                                let vid_media_id = media.id;
                                let btn = ui.button(format!("🏷 Tags ({}) ▾", assigned.len()));
                                let popup_id = ui.make_persistent_id(("video_tags_popup", vid_media_id));
                                if btn.clicked() {
                                    ui.memory_mut(|m| m.toggle_popup(popup_id));
                                }
                                if ui.memory(|m| m.is_popup_open(popup_id)) {
                                    let gap = 6.0;
                                    let pivot_pos = egui::pos2(btn.rect.left(), btn.rect.top() - gap);
                                    let space_above = (btn.rect.top() - 16.0).max(200.0);
                                    let max_h = (v_screen.height() * 0.70).clamp(200.0, space_above);
                                    let mut filter = state.tag_dropdown_filter.clone();
                                    let recent_ids = state.recent_tag_ids.clone();
                                    let area = egui::Area::new(popup_id)
                                        .order(egui::Order::Tooltip)
                                        .fixed_pos(pivot_pos)
                                        .pivot(egui::Align2::LEFT_BOTTOM)
                                        .interactable(true)
                                        .show(ui.ctx(), |ui| {
                                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                                ui.set_height(max_h);
                                                ui.horizontal(|ui| {
                                                    ui.label("🔍");
                                                    ui.add(egui::TextEdit::singleline(&mut filter)
                                                        .hint_text("Filter tags…")
                                                        .desired_width(150.0));
                                                });
                                                if tag_tree.is_empty() {
                                                    ui.colored_label(Color32::GRAY, "No tags yet");
                                                } else {
                                                    if filter.is_empty() && !recent_ids.is_empty() {
                                                        ui.add_space(2.0);
                                                        ui.colored_label(crate::theme::TEXT_MUTED, "Recent");
                                                        for &tid in recent_ids.iter().take(6) {
                                                            if let Some(tag) = find_tag_in_tree(&tag_tree, tid) {
                                                                let icon = tag.icon.as_deref().filter(|s| !s.is_empty()).unwrap_or("🏷");
                                                                let is_assigned = assigned.contains(&tid);
                                                                let check = if is_assigned { "☑" } else { "☐" };
                                                                let bg = tag_color32(&tag);
                                                                ui.horizontal(|ui| {
                                                                    ui.label(egui::RichText::new(icon).color(bg));
                                                                    let label = format!("{}  {}", check, tag.name);
                                                                    if ui.selectable_label(is_assigned, label).clicked() {
                                                                        pending = Some(if is_assigned {
                                                                            PendingAction::RemoveTags(vec![vid_media_id], vec![tid])
                                                                        } else {
                                                                            PendingAction::AssignTags(vec![vid_media_id], vec![tid])
                                                                        });
                                                                    }
                                                                });
                                                            }
                                                        }
                                                        ui.separator();
                                                    }
                                                    let filter_lower = filter.trim().to_lowercase();
                                                    egui::ScrollArea::vertical()
                                                        .max_height(max_h - 60.0)
                                                        .auto_shrink([true, false])
                                                        .show(ui, |ui| {
                                                            show_tag_assign_menu(
                                                                ui, &tag_tree, vid_media_id,
                                                                &assigned, &mut pending,
                                                                &mut collapsed, 0,
                                                                &filter_lower,
                                                            );
                                                        });
                                                }
                                            });
                                        });
                                    state.tag_dropdown_filter = filter;
                                    let clicked = ui.input(|i| i.pointer.any_click());
                                    if clicked {
                                        let pos = ui.ctx().pointer_interact_pos();
                                        let inside_popup = pos.map(|p| area.response.rect.contains(p)).unwrap_or(false);
                                        let inside_btn = pos.map(|p| btn.rect.contains(p)).unwrap_or(false);
                                        if !inside_popup && !inside_btn {
                                            ui.memory_mut(|m| m.close_popup());
                                        }
                                    }
                                }
                                if pending.is_some() { state.pending_action = pending; }
                                state.collapsed_dropdown_tag_ids = collapsed;

                                // ── Clear tags ──
                                let clear_enabled = !assigned.is_empty();
                                if ui.add_enabled(clear_enabled, egui::Button::new("✕ Clear"))
                                    .on_hover_text("Remove all tags").clicked()
                                {
                                    state.pending_action = Some(
                                        PendingAction::ClearMediaTags(vec![vid_media_id]),
                                    );
                                }

                                ui.label("|");

                                // ── Delete ──
                                if ui.button("🗑").on_hover_text("Delete (Del)").clicked() {
                                    state.confirm_delete = Some(vid_media_id);
                                }
                            });
                        });
                });
        }

        // Prev / next nav arrows (same chevron style as image preview).
        // Drawn in a separate Area at Foreground order so it sits above
        // the video overlay; clicks on the arrow zones fire the
        // navigation BEFORE the video's "click outside → close" handler
        // would have a chance to react.
        let mut nav_target: Option<MediaItem> = None;
        let screen = ctx.screen_rect();
        let arrow_w = 60.0;
        let prev_zone = egui::Rect::from_min_size(
            screen.min + Vec2::new(0.0, screen.height() * 0.25),
            Vec2::new(arrow_w, screen.height() * 0.5),
        );
        let next_zone = egui::Rect::from_min_size(
            egui::pos2(screen.max.x - arrow_w, screen.min.y + screen.height() * 0.25),
            Vec2::new(arrow_w, screen.height() * 0.5),
        );
        let mut hit_prev = false;
        let mut hit_next = false;
        egui::Area::new(egui::Id::new("video_nav_arrows"))
            .fixed_pos(screen.min)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.set_min_size(screen.size());
                if video_prev_item.is_some() && draw_nav_arrow(ui, prev_zone, false, 1.0) {
                    hit_prev = true;
                }
                if video_next_item.is_some() && draw_nav_arrow(ui, next_zone, true, 1.0) {
                    hit_next = true;
                }
            });
        if hit_prev { nav_target = video_prev_item.clone(); }
        if hit_next { nav_target = video_next_item.clone(); }

        // ── Close button (top-right) — own Tooltip-order Area so it
        //    can't be stolen by the video overlay or controls. ──
        let mut hit_close = false;
        {
            let close_center = egui::pos2(screen.max.x - 25.0, screen.min.y + 25.0);
            let close_rect = egui::Rect::from_center_size(close_center, Vec2::splat(36.0));
            egui::Area::new(egui::Id::new("video_close_area"))
                .fixed_pos(close_rect.min)
                .order(egui::Order::Tooltip)
                .interactable(true)
                .show(ctx, |ui| {
                    let (r, resp) = ui.allocate_exact_size(close_rect.size(), egui::Sense::click());
                    if resp.clicked() { hit_close = true; }
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                });
        }

        if let Some(item) = nav_target {
            video_player.close();
            navigate(state, preview, item);
            return;
        }
        if !video_keep_open || hit_close {
            video_player.close();
            state.preview_media = None;
        }
        return;
    }

    // ── Texture loading (progressive + cache) ────────────────────────────────
    //
    // Sequence on a fresh navigation:
    //   1. Set the placeholder (gallery thumb) so the user sees *something*
    //      this frame.
    //   2. If a decoded image is already cached for this media id, upload
    //      it as the texture immediately — done.
    //   3. Otherwise queue a Fast decode (priority 0). When it arrives,
    //      upload it and queue a Full decode (priority 0). When Full
    //      arrives, replace the texture and queue prewarms for prev/next.
    //
    // Cap the decoded resolution at 2× the screen's longest edge so huge
    // originals (50MP+ phone JPEGs) decode at /4 instead of /1. The
    // factor of 2 leaves headroom for moderate zoom-in without visible
    // upscaling artifacts.
    let screen_max = ctx.screen_rect().size().max_elem().max(1024.0);
    let full_max_dim = ((screen_max as u32) * 2).min(3840).max(1024);
    // Fast pass at ~1/4 the full target. JPEG decodes that at /4 DCT
    // scale — typically 8–16× faster than the full decode.
    let fast_max_dim = (full_max_dim / 4).max(512);

    if preview.loaded_path != media.full_path {
        preview.loaded_path = media.full_path.clone();
        preview.placeholder_tex = tex_mgr.peek_thumb(media.id).cloned();
        preview.reset_view();
        preview.prewarm_done_for = None;
        preview.current_tex_id = None;
        preview.current_tex_quality = None;
        preview.texture = None;

        // Cache hit?
        if let Some((img, q)) = preview.image_cache.get(&media.id).cloned() {
            preview.texture = Some(ctx.load_texture(
                "preview_full", (*img).clone(), egui::TextureOptions::LINEAR,
            ));
            preview.current_tex_id = Some(media.id);
            preview.current_tex_quality = Some(q);
            preview.loading = q != LoadQuality::Full;
            preview.cache_lru.retain(|&x| x != media.id);
            preview.cache_lru.push_back(media.id);
            // Upgrade Fast → Full in the background if needed.
            if q == LoadQuality::Fast {
                preview.enqueue_load(
                    media.id, media.full_path.clone(),
                    full_max_dim, LoadQuality::Full, 0,
                );
            }
        } else {
            // Cold open: kick off Fast first; Full is queued once Fast
            // arrives (so the worker isn't competing with itself).
            preview.loading = true;
            preview.enqueue_load(
                media.id, media.full_path.clone(),
                fast_max_dim, LoadQuality::Fast, 0,
            );
        }
    }

    // Drain any decode results that arrived since last frame. We collect
    // first so subsequent mutations (cache_insert, enqueue_load, texture
    // upload) don't conflict with the channel borrow.
    let results: Vec<LoadResult> =
        std::iter::from_fn(|| preview.result_rx.try_recv().ok()).collect();
    for result in results {
        preview.pending.remove(&result.media_id);
        preview.cache_insert(result.media_id, result.image.clone(), result.quality);

        // If this result is for the active preview, upload it.
        if result.media_id == media.id {
            let upgrade = match (preview.current_tex_quality, result.quality) {
                (None, _)                                            => true,
                (Some(LoadQuality::Fast), LoadQuality::Full)         => true,
                (Some(LoadQuality::Fast), LoadQuality::Fast)         => preview.texture.is_none(),
                _                                                    => false,
            };
            if upgrade {
                preview.texture = Some(ctx.load_texture(
                    "preview_full", (*result.image).clone(),
                    egui::TextureOptions::LINEAR,
                ));
                preview.current_tex_id = Some(media.id);
                preview.current_tex_quality = Some(result.quality);
                if result.quality == LoadQuality::Full {
                    preview.loading = false;
                }
            }
            // Got Fast for the active media but Full not yet cached?
            // Queue Full so we eventually upgrade. (No-op if Full is
            // already pending or already cached.)
            if result.quality == LoadQuality::Fast {
                preview.enqueue_load(
                    media.id, media.full_path.clone(),
                    full_max_dim, LoadQuality::Full, 0,
                );
            }
        }
        // Repaint so the new texture appears on the next frame even if
        // no other input event would otherwise wake the UI.
        ctx.request_repaint();
    }

    // ── Navigation targets ───────────────────────────────────────────────────
    // Position is computed against the global skeleton (which spans the
    // entire filtered result set), not against `state.media` (which is
    // just the in-memory window around the gallery viewport — the preview
    // can be on an item far outside that window).
    let current_global = state.global_idx_of(media.id);
    let prev_item = current_global
        .filter(|&i| i > 0)
        .and_then(|i| {
            let prev_id = state.media_skel.get(i - 1).map(|s| s.id)?;
            state.media_by_id(prev_id)
        });
    let next_item = current_global.and_then(|i| {
        let next_id = state.media_skel.get(i + 1).map(|s| s.id)?;
        state.media_by_id(next_id)
    });
    // Used by the bottom-center "12 / 7933" position counter.
    let current_idx = current_global;

    // Neighbour pre-warm — once the active preview reaches Full quality,
    // queue Full decodes for the previous and next items at low priority
    // so arrow-key navigation lands on a cached, already-decoded image.
    // Gated on `prewarm_done_for` so we only enqueue once per active
    // media id (the cache + pending dedup would absorb redundant calls,
    // but skipping the work entirely is cheaper).
    if preview.current_tex_quality == Some(LoadQuality::Full)
        && preview.current_tex_id == Some(media.id)
        && preview.prewarm_done_for != Some(media.id)
    {
        preview.prewarm_done_for = Some(media.id);
        for neighbor in [prev_item.as_ref(), next_item.as_ref()].iter().flatten() {
            preview.enqueue_load(
                neighbor.id, neighbor.full_path.clone(),
                full_max_dim, LoadQuality::Full,
                100, // low priority — never preempts the active preview's load
            );
        }
    }

    // ── Keyboard shortcuts ───────────────────────────────────────────────────
    // Esc closes the preview, EXCEPT when a confirm-delete dialog is open
    // (in that case Esc just dismisses the dialog) or a rename is in progress.
    let confirm_open = state.confirm_delete.is_some();
    let renaming = state.rename_media.is_some();
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !confirm_open && !renaming {
        close_preview(state, preview);
        return;
    }
    // Delete/Backspace → request deletion of the current preview item.
    // Skip when a text field is focused so editing field values still works.
    let typing = ctx.memory(|m| m.focused().is_some());
    if !typing && !confirm_open && !renaming
        && ctx.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
    {
        state.confirm_delete = Some(media.id);
    }
    // F2 → enter rename mode (mirrors the file-manager convention).
    if !typing && !confirm_open && !renaming
        && ctx.input(|i| i.key_pressed(egui::Key::F2))
    {
        state.rename_media = Some((media.id, media.filename.clone()));
    }

    // Rating hotkeys 0-5 — apply to the current preview item.
    if !typing && !confirm_open && !renaming {
        let rating_pressed = [
            (egui::Key::Num0, 0),
            (egui::Key::Num1, 1),
            (egui::Key::Num2, 2),
            (egui::Key::Num3, 3),
            (egui::Key::Num4, 4),
            (egui::Key::Num5, 5),
        ].iter()
            .find(|(k, _)| ctx.input(|i| i.key_pressed(*k)))
            .map(|(_, v)| *v);
        if let Some(r) = rating_pressed {
            state.pending_action = Some(crate::app::PendingAction::SetRating {
                media_ids: vec![media.id], rating: r,
            });
        }
    }
    // "I" key toggles the EXIF metadata panel in preview mode.
    if !typing && !confirm_open && !renaming
        && ctx.input(|i| i.key_pressed(egui::Key::I))
    {
        state.show_metadata_panel = !state.show_metadata_panel;
    }

    if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) && !renaming {
        if let Some(item) = prev_item.clone() { navigate(state, preview, item); return; }
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) && !renaming {
        if let Some(item) = next_item.clone() { navigate(state, preview, item); return; }
    }

    let screen = ctx.screen_rect();
    let filename = media.filename.clone();

    // Pick the texture we'll actually paint: full image if it's loaded,
    // otherwise the placeholder thumbnail. Toolbar / chip / fit logic all
    // key off "do we have *something* to display" rather than "is the
    // full image ready".
    let display_tex_full = preview.texture.is_some();
    let has_display_tex = display_tex_full || preview.placeholder_tex.is_some();

    // ── Filmstrip (drawn first so toolbar can stack above it) ────────────────
    let filmstrip_active = *show_filmstrip && has_display_tex && chrome_alpha > 0.01;
    let mut filmstrip_clicked: Option<i64> = None;
    if filmstrip_active {
        filmstrip_clicked = draw_filmstrip(ctx, state, tex_mgr, current_global, chrome_alpha);
    }

    // ── Toolbar (rendered first to know its rect for hit-testing) ────────────
    // Hidden entirely when chrome has fully faded out. When the filmstrip
    // is visible, the toolbar lifts up by the strip's height + spacing
    // so they don't overlap.
    // 4px gap so the toolbar sits just above the filmstrip rather than
    // floating with extra air between them.
    let toolbar_lift = if filmstrip_active { FILMSTRIP_H + 4.0 } else { 0.0 };
    let toolbar_rect = if toolbar_settings.any_visible() && has_display_tex
        && chrome_alpha > 0.01
    {
        draw_toolbar(ctx, preview, state, media.id, toolbar_settings, show_filmstrip, toolbar_lift, toolbar_pos)
    } else {
        egui::Rect::NOTHING
    };

    // ── Tag chips (top-center, above rating + filename) ────────────────────
    let chip_rect = if has_display_tex && !state.preview_tags.is_empty()
        && chrome_alpha > 0.01
    {
        draw_tag_chips(ctx, state, media.id)
    } else {
        egui::Rect::NOTHING
    };

    let pointer_in_toolbar = ctx
        .input(|i| i.pointer.hover_pos())
        .map_or(false, |p| toolbar_rect.contains(p) || chip_rect.contains(p));

    // ── Compute fit scale ────────────────────────────────────────────────────
    // Prefer the full image's size for accuracy; if only the placeholder
    // (300px thumb) is available, use the media item's stored natural
    // dimensions so the placeholder displays at the correct aspect ratio
    // and at the same final on-screen size as the full image will once
    // it loads — no layout pop when the swap happens.
    let natural_size = if let Some(tex) = &preview.texture {
        tex.size_vec2()
    } else if let (Some(w), Some(h)) = (media.width, media.height) {
        if w > 0 && h > 0 { Vec2::new(w as f32, h as f32) }
        else { preview.placeholder_tex.as_ref().map(|t| t.size_vec2()).unwrap_or(Vec2::ZERO) }
    } else {
        preview.placeholder_tex.as_ref().map(|t| t.size_vec2()).unwrap_or(Vec2::ZERO)
    };
    let (rotated_natural, fit_scale) = if natural_size != Vec2::ZERO {
        let rn = if preview.rotation == 90 || preview.rotation == 270 {
            Vec2::new(natural_size.y, natural_size.x)
        } else {
            natural_size
        };
        let max_size = screen.size() * 0.9;
        let raw = (max_size.x / rn.x).min(max_size.y / rn.y);
        // Cap upscaling at 1× ONLY when we have the real full-res image
        // texture loaded — that's the sane "don't blow past 100% just
        // because the screen is huge" guard. While only the placeholder
        // is on screen, `natural_size` may have fallen back to the
        // ~320-px thumbnail dimensions (when media.width/height aren't
        // populated in the DB), and capping at 1× would pin the
        // placeholder to its tiny native size for the few seconds the
        // full image takes to decode — visible as "image opens
        // extremely zoomed out, then snaps to normal size". Without
        // the cap during placeholder phase, the thumb is upscaled to
        // screen-fit (blurry but the correct size from frame one).
        let fs = if preview.texture.is_some() { raw.min(1.0) } else { raw };
        (rn, fs)
    } else {
        (Vec2::ZERO, 1.0)
    };

    // Detect texture-swap natural-size changes (placeholder ~320 px →
    // full ~3840 px). When the user has hand-zoomed the placeholder
    // (fit_mode == false), `preview.zoom` is implicitly relative to
    // the placeholder's tiny natural size; once the full image swaps
    // in, that same zoom multiplier suddenly applies to a 12× larger
    // texture and the image balloons. Re-normalise the zoom by the
    // ratio so the on-screen display size is preserved across the
    // swap. Skip when only rotation flipped (areas roughly equal —
    // rotation is already accounted for in `rotated_natural` itself).
    let prev = preview.last_rotated_natural;
    preview.last_rotated_natural = rotated_natural;
    if !preview.fit_mode
        && prev != Vec2::ZERO
        && rotated_natural != Vec2::ZERO
    {
        let prev_area = prev.x * prev.y;
        let new_area  = rotated_natural.x * rotated_natural.y;
        if new_area > 0.0 {
            let area_ratio = prev_area / new_area;
            // Threshold catches order-of-magnitude texture swaps but
            // ignores tiny float drift and pure-rotation swaps (which
            // preserve area).
            if (area_ratio - 1.0).abs() > 0.1 {
                let ratio = prev.x / rotated_natural.x;
                preview.zoom *= ratio;
                preview.target_zoom *= ratio;
            }
        }
    }

    // Keep zoom + target synced to fit_scale while in fit_mode so a
    // flip out of fit_mode (e.g. an accidental wheel-tick on entry)
    // starts from the correct value rather than animating away from a
    // stale carry-over.
    if preview.fit_mode {
        preview.zoom = fit_scale;
        preview.target_zoom = fit_scale;
    }

    // ── Scroll zoom (skip if pointer is over toolbar) ────────────────────────
    if !pointer_in_toolbar && has_display_tex {
        let scroll = ctx.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            // When still in fit_mode, seed target/current from fit_scale so the
            // first scroll has something coherent to ease away from.
            if preview.fit_mode {
                preview.zoom = fit_scale;
                preview.target_zoom = fit_scale;
                preview.fit_mode = false;
            }
            let old_target = preview.target_zoom;
            let new_target = (old_target * (1.0 + scroll * 0.0025)).clamp(0.05, 16.0);

            if preview.zoom_mode == ZoomMode::Cursor {
                if let Some(cursor) = ctx.input(|i| i.pointer.hover_pos()) {
                    let scale = new_target / old_target;
                    let delta = cursor - screen.center();
                    preview.target_pan = delta * (1.0 - scale) + preview.target_pan * scale;
                }
            }
            preview.target_zoom = new_target;
        }
    }

    // ── Ease zoom/pan toward their targets ───────────────────────────────────
    if !preview.fit_mode {
        let dt = ctx.input(|i| i.stable_dt).clamp(0.0, 0.1);
        // Exponential smoothing: ~95% closure in ~120 ms.
        let t = 1.0 - (-dt * 25.0).exp();
        preview.zoom += (preview.target_zoom - preview.zoom) * t;
        preview.pan += (preview.target_pan - preview.pan) * t;
        let zoom_done = (preview.target_zoom - preview.zoom).abs() < 0.0005;
        let pan_done = (preview.target_pan - preview.pan).length() < 0.5;
        if zoom_done && pan_done {
            preview.zoom = preview.target_zoom;
            preview.pan = preview.target_pan;
        } else {
            ctx.request_repaint();
        }
    }

    let effective_zoom = if preview.fit_mode { fit_scale } else { preview.zoom };
    let display_size = rotated_natural * effective_zoom;

    // ── Fullscreen overlay ───────────────────────────────────────────────────
    let mut close = false;
    let mut nav_prev = false;
    let mut nav_next = false;

    egui::Area::new(egui::Id::new("preview_overlay"))
        .fixed_pos(screen.min)
        // Background — the toolbar / info button / tag chips all live on
        // higher layers, so this big screen-spanning click_and_drag widget
        // can't accidentally cover them. (It would also be promoted to the
        // top of its layer on click otherwise, hiding the toolbar.)
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            let (rect, resp) =
                ui.allocate_exact_size(screen.size(), egui::Sense::click_and_drag());

            ui.painter().rect_filled(rect, egui::Rounding::ZERO, Color32::from_black_alpha(230));

            // Vignette painted on top of the dark backdrop but before the
            // image so the image stays at full brightness.
            paint_vignette(ui.painter(), rect);

            // Drag-to-pan (within the image area, not over toolbar)
            if resp.dragged() {
                let drag_start = resp.interact_pointer_pos().unwrap_or_default();
                if !toolbar_rect.contains(drag_start) && !chip_rect.contains(drag_start) {
                    preview.pan += resp.drag_delta();
                    preview.target_pan = preview.pan;
                }
            }

            // Right-click context menu on the preview image
            let ctx_media_id = media.id;
            resp.context_menu(|ui| {
                if ui.button("🔗 Open in default app").clicked() {
                    state.pending_action = Some(PendingAction::OpenExternal { media_id: ctx_media_id });
                    ui.close_menu();
                }
                if ui.button("📂 Show in Explorer").clicked() {
                    state.pending_action = Some(PendingAction::ShowInExplorer { media_id: ctx_media_id });
                    ui.close_menu();
                }
                if ui.button("🤖 AI Suggest Tags").clicked() {
                    state.ai_request_media_id = Some(ctx_media_id);
                    ui.close_menu();
                }
                ui.separator();
                ui.menu_button("⭐ Rate", |ui| {
                    for r in 0..=5 {
                        let label = if r == 0 { "☆ No rating".to_string() } else { "★".repeat(r as usize) };
                        if ui.button(label).clicked() {
                            state.pending_action = Some(PendingAction::SetRating { media_ids: vec![ctx_media_id], rating: r });
                            ui.close_menu();
                        }
                    }
                });
                ui.separator();
                if ui.button("🗑 Delete").clicked() {
                    state.confirm_delete = Some(ctx_media_id);
                    ui.close_menu();
                }
            });

            // Pick which texture to paint this frame: full image once
            // ready, else the placeholder thumbnail. Both are drawn at
            // the same on-screen size so the swap is a clean crossfade
            // (no jump in geometry).
            let render_tex = preview.texture.as_ref().or(preview.placeholder_tex.as_ref());
            if let Some(tex) = render_tex {
                let center = rect.center() + preview.pan;
                let rotation_steps = (preview.rotation / 90).rem_euclid(4) as usize;
                render_image_quad(
                    ui.painter(),
                    tex.id(),
                    center,
                    display_size,
                    rotation_steps,
                    preview.flip_h,
                    preview.flip_v,
                );

                let display_pct = if preview.fit_mode {
                    (fit_scale * 100.0) as i32
                } else {
                    (preview.zoom * 100.0) as i32
                };
                let suffix = format!("  —  {}%", display_pct);
                let font = egui::FontId::proportional(14.0);
                let title_color = fade(Color32::from_white_alpha(220), chrome_alpha);
                let suffix_color = fade(Color32::from_white_alpha(180), chrome_alpha);
                // Render the filename + percentage. Filename half is clickable
                // (single-click → rename). When a rename is in progress, replace
                // the filename half with an inline TextEdit at the same spot.
                let pct_galley = ui.fonts(|f| f.layout_no_wrap(suffix.clone(), font.clone(), suffix_color));
                let title_y = rect.min.y + 25.0;
                let renaming_this = matches!(&state.rename_media, Some((id, _)) if *id == media.id);
                if renaming_this {
                    // Inline editor — anchor to top-center of the screen so it
                    // sits where the filename normally is.
                    let mut commit = false;
                    let mut cancel = false;
                    // Esc cancels even though the TextEdit holds focus —
                    // egui's singleline editor doesn't blur on Esc, and the
                    // preview's outer Esc handler is gated on `!renaming`,
                    // so without this dedicated check Esc would do nothing
                    // while the rename field is open.
                    if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
                        cancel = true;
                    }
                    let area_resp = egui::Area::new(egui::Id::new("rename_filename"))
                        .anchor(egui::Align2::CENTER_TOP, Vec2::new(0.0, 16.0))
                        .order(egui::Order::Foreground)
                        .show(ui.ctx(), |ui| {
                            ui.horizontal(|ui| {
                                let buf = match &mut state.rename_media {
                                    Some((_, b)) => b,
                                    None => return,
                                };
                                let r = ui.add(
                                    egui::TextEdit::singleline(buf)
                                        .desired_width(360.0)
                                        .font(egui::TextStyle::Body),
                                );
                                // Order matters: `lost_focus()` must be
                                // observed BEFORE `request_focus()` is
                                // called this frame — otherwise the
                                // re-grab makes egui report "still
                                // focused" and Enter never commits.
                                if r.lost_focus() {
                                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) { commit = true; }
                                    else { cancel = true; }
                                }
                                r.request_focus();
                                if ui.small_button("✓").on_hover_text("Save (Enter)").clicked() { commit = true; }
                                if ui.small_button("✕").on_hover_text("Cancel (Esc)").clicked() { cancel = true; }
                            });
                        });
                    let _ = area_resp;
                    if commit {
                        if let Some((id, name)) = state.rename_media.take() {
                            state.pending_action = Some(crate::app::PendingAction::RenameMedia {
                                media_id: id, new_filename: name,
                            });
                        }
                    } else if cancel {
                        state.rename_media = None;
                    }
                } else {
                    // Order top-to-bottom: tag chips → stars → filename
                    // (chips are drawn earlier; stars go right under them).
                    let stars_y = if chip_rect != egui::Rect::NOTHING {
                        chip_rect.max.y + 2.0
                    } else {
                        rect.min.y + 8.0
                    };
                    let clicked_rating = draw_rating_chrome(
                        ui,
                        rect.center().x,
                        stars_y,
                        media.rating,
                        chrome_alpha,
                    );
                    if let Some(new_rating) = clicked_rating {
                        state.pending_action = Some(crate::app::PendingAction::SetRating {
                            media_ids: vec![media.id],
                            rating: new_rating,
                        });
                    }
                    // Filename sits right under the stars row (star row is ~20px tall).
                    let name_y = stars_y + 22.0;
                    let name_galley = ui.fonts(|f| f.layout_no_wrap(filename.clone(), font.clone(), title_color));
                    let total_w = name_galley.size().x + pct_galley.size().x;
                    let left_x = rect.center().x - total_w * 0.5;
                    let name_rect = egui::Rect::from_min_size(
                        egui::pos2(left_x, name_y - name_galley.size().y * 0.5),
                        name_galley.size(),
                    );
                    ui.painter().galley(name_rect.min, name_galley, title_color);
                    ui.painter().text(
                        egui::pos2(name_rect.max.x, name_y),
                        egui::Align2::LEFT_CENTER,
                        suffix, font, suffix_color,
                    );
                    let click_rect = name_rect.expand2(Vec2::new(4.0, 4.0));
                    let click_resp = ui.interact(
                        click_rect,
                        ui.id().with(("rename_filename_hit", media.id)),
                        egui::Sense::click(),
                    );
                    if click_resp.hovered() && chrome_alpha > 0.5 {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        ui.painter().line_segment(
                            [egui::pos2(name_rect.min.x, name_rect.max.y + 1.0),
                             egui::pos2(name_rect.max.x, name_rect.max.y + 1.0)],
                            egui::Stroke::new(1.0, fade(Color32::from_white_alpha(180), chrome_alpha)),
                        );
                        click_resp.clone().on_hover_text(
                            "Click: show in Explorer  ·  Double-click or F2: rename",
                        );
                    }
                    // Filename click handling.
                    //   single-click → reveal the file in Explorer
                    //   double-click → enter rename mode
                    // egui fires `clicked()` on release-1 of a double-click and
                    // `double_clicked()` on release-2, so we defer the
                    // single-click action by ~400 ms to see whether a second
                    // click is coming. Without this, every intentional rename
                    // would also pop an Explorer window first.
                    if chrome_alpha > 0.5 {
                        let mem_id = egui::Id::new(("filename_click_pending", media.id));
                        let now = ctx.input(|i| i.time);
                        let pending: Option<f64> = ctx.memory(|m| m.data.get_temp(mem_id));
                        const DOUBLE_CLICK_THRESHOLD: f64 = 0.4;

                        if click_resp.double_clicked() {
                            // Cancel any pending single-click and start rename.
                            ctx.memory_mut(|m| m.data.remove::<f64>(mem_id));
                            state.rename_media = Some((media.id, filename.clone()));
                        } else if click_resp.clicked() {
                            ctx.memory_mut(|m| m.data.insert_temp(mem_id, now));
                            // Wake up after the threshold so the deferred fire
                            // happens even if the user doesn't move the mouse.
                            ctx.request_repaint_after(
                                std::time::Duration::from_secs_f32(
                                    DOUBLE_CLICK_THRESHOLD as f32 + 0.05,
                                ),
                            );
                        } else if let Some(t) = pending {
                            let age = now - t;
                            if age >= DOUBLE_CLICK_THRESHOLD && age < 1.5 {
                                // Confirmed single click — fire the deferred action.
                                ctx.memory_mut(|m| m.data.remove::<f64>(mem_id));
                                show_in_explorer(&media.full_path);
                            } else if age >= 1.5 {
                                // Stale (e.g. user navigated away and came back).
                                // Drop it without firing.
                                ctx.memory_mut(|m| m.data.remove::<f64>(mem_id));
                            }
                        }
                    }
                }
            } else {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Loading...",
                    egui::FontId::proportional(18.0),
                    Color32::from_white_alpha(150),
                );
            }

            let close_hit = draw_close_button(ui, rect, chrome_alpha);

            let arrow_w = 60.0;
            let prev_zone = egui::Rect::from_min_size(
                rect.min + Vec2::new(0.0, rect.height() * 0.25),
                Vec2::new(arrow_w, rect.height() * 0.5),
            );
            let next_zone = egui::Rect::from_min_size(
                egui::pos2(rect.max.x - arrow_w, rect.min.y + rect.height() * 0.25),
                Vec2::new(arrow_w, rect.height() * 0.5),
            );
            if prev_item.is_some() { nav_prev = draw_nav_arrow(ui, prev_zone, false, chrome_alpha); }
            if next_item.is_some() { nav_next = draw_nav_arrow(ui, next_zone, true,  chrome_alpha); }

            // Position counter at bottom-center: "12 / 7933". Fades with the
            // rest of the chrome.
            if let Some(idx) = current_idx {
                let pos_text = format!("{} / {}", idx + 1, state.media_skel.len());
                ui.painter().text(
                    egui::pos2(rect.center().x, rect.max.y - 16.0),
                    egui::Align2::CENTER_CENTER,
                    pos_text,
                    egui::FontId::proportional(12.0),
                    fade(Color32::from_white_alpha(160), chrome_alpha),
                );
            }

            if resp.clicked() && chrome_alpha > 0.5 {
                if let Some(p) = resp.interact_pointer_pos() {
                    if close_hit.contains(p) {
                        close = true;
                    } else if prev_item.is_some() && prev_zone.contains(p) {
                        nav_prev = true;
                    } else if next_item.is_some() && next_zone.contains(p) {
                        nav_next = true;
                    }
                }
            }
        });

    if close {
        close_preview(state, preview);
    } else if nav_prev {
        if let Some(item) = prev_item { navigate(state, preview, item); }
    } else if nav_next {
        if let Some(item) = next_item { navigate(state, preview, item); }
    } else if let Some(id) = filmstrip_clicked {
        // Filmstrip click → jump to that media id. Use the id-based
        // lookup so it works for items outside the gallery's loaded
        // media window (we just primed it inside draw_filmstrip).
        if let Some(item) = state.media_by_id(id) {
            navigate(state, preview, item);
        }
    }
}

// ── Floating toolbar ───────────────────────────────────────────────────────

/// Recursive tag-tree renderer used inside the toolbar's "Tags" menu.
/// Renders a flat indented list — one row per tag, no nested popups.
/// Each row toggles assignment for `media_id` on click.
/// Parent tags have a ▶/▼ toggle to expand/collapse children.
/// Check if a tag or any of its descendants match the filter.
fn tag_subtree_matches(tag: &Tag, filter: &str) -> bool {
    if filter.is_empty() { return true; }
    if tag.name.to_lowercase().contains(filter) { return true; }
    tag.children.iter().any(|c| tag_subtree_matches(c, filter))
}

/// Find a tag by ID anywhere in the tree (for recent tags section).
fn find_tag_in_tree(tags: &[Tag], id: i64) -> Option<Tag> {
    for tag in tags {
        if tag.id == id { return Some(tag.clone()); }
        if let Some(found) = find_tag_in_tree(&tag.children, id) {
            return Some(found);
        }
    }
    None
}

fn show_tag_assign_menu(
    ui: &mut egui::Ui,
    tags: &[Tag],
    media_id: i64,
    assigned: &std::collections::HashSet<i64>,
    pending: &mut Option<PendingAction>,
    collapsed: &mut std::collections::HashSet<i64>,
    depth: usize,
    filter: &str,
) {
    for tag in tags {
        // Skip tags (and their subtrees) that don't match the filter.
        if !filter.is_empty() && !tag_subtree_matches(tag, filter) {
            continue;
        }
        let icon = tag.icon.as_deref().filter(|s| !s.is_empty()).unwrap_or("🏷");
        let is_assigned = assigned.contains(&tag.id);
        let check = if is_assigned { "☑" } else { "☐" };
        let indent = depth as f32 * 14.0;
        let has_children = !tag.children.is_empty();

        // When filter is active, force-expand so matches are visible.
        let expanded = if !filter.is_empty() { true } else { !collapsed.contains(&tag.id) };

        ui.horizontal(|ui| {
            ui.add_space(indent);
            // Expand/collapse toggle for parents (hidden when filtering).
            if has_children && filter.is_empty() {
                let arrow = if expanded { "▼" } else { "▶" };
                if ui.small_button(arrow).clicked() {
                    if expanded { collapsed.insert(tag.id); }
                    else { collapsed.remove(&tag.id); }
                }
            } else if has_children {
                ui.add_space(20.0);
            } else {
                ui.add_space(20.0);
            }
            // Colored icon label + checkbox + name
            let bg = tag_color32(tag);
            ui.label(egui::RichText::new(icon).color(bg));
            let label = format!("{}  {}", check, tag.name);
            if ui.selectable_label(is_assigned, label).clicked() {
                *pending = Some(if is_assigned {
                    PendingAction::RemoveTags(vec![media_id], vec![tag.id])
                } else {
                    PendingAction::AssignTags(vec![media_id], vec![tag.id])
                });
            }
        });
        if has_children && expanded {
            show_tag_assign_menu(ui, &tag.children, media_id, assigned, pending, collapsed, depth + 1, filter);
        }
    }
}

/// Render a row of chip buttons for the currently-assigned tags. Right-click
/// a chip → "Remove tag". Returns the bounding rect for hit-testing.
fn draw_tag_chips(ctx: &egui::Context, state: &mut AppState, media_id: i64) -> egui::Rect {
    let tags = state.preview_tags.clone();
    let mut pending: Option<PendingAction> = None;

    let resp = egui::Area::new(egui::Id::new("preview_tag_chips"))
        .anchor(egui::Align2::CENTER_TOP, Vec2::new(0.0, 8.0))
        .order(egui::Order::Foreground)
        .sense(egui::Sense::hover())
        .show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for tag in &tags {
                    let icon = tag.icon.as_deref().filter(|s| !s.is_empty()).unwrap_or("🏷");
                    let bg = tag_color32(tag);
                    let text_col = if (bg.r() as u32 + bg.g() as u32 + bg.b() as u32) > 380 {
                        Color32::BLACK
                    } else {
                        Color32::WHITE
                    };

                    // One pill per tag, implemented as a rounded Frame so it
                    // can grow horizontally to fit a fading-in × child when
                    // hovered. The shadow replaces the previous white outline.
                    let hover_id = ui.id().with(("preview_chip", tag.id));
                    let was_hovered = ui.memory(|m| m.data.get_temp::<bool>(hover_id).unwrap_or(false));
                    // Smooth 0..1 interpolation for the × widget's size + alpha.
                    let t = ctx.animate_bool_with_time(hover_id, was_hovered, 0.18);

                    let frame_resp = egui::Frame::none()
                        .fill(bg)
                        .rounding(10.0)
                        .shadow(egui::epaint::Shadow {
                            offset: Vec2::new(0.0, 2.0),
                            blur: 6.0,
                            spread: 0.0,
                            color: Color32::from_black_alpha(110),
                        })
                        .inner_margin(egui::Margin { left: 8.0, right: 8.0, top: 2.0, bottom: 2.0 })
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0 * t.max(0.0);
                                ui.label(
                                    egui::RichText::new(format!("{} {}", icon, tag.name)).color(text_col),
                                );
                                if t > 0.001 {
                                    let alpha = (t * 255.0) as u8;
                                    let mut col = text_col;
                                    col = Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), alpha);
                                    let (rect, _) = ui.allocate_exact_size(
                                        Vec2::new(14.0 * t, 14.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "×",
                                        egui::FontId::proportional(14.0),
                                        col,
                                    );
                                }
                            });
                        });

                    frame_resp.response.clone().on_hover_text(&tag.full_path);
                    let tag_id_for_menu = tag.id;
                    frame_resp.response.context_menu(|ui| {
                        if ui.button("🗑 Remove tag from this photo").clicked() {
                            pending = Some(PendingAction::RemoveTags(vec![media_id], vec![tag_id_for_menu]));
                            ui.close_menu();
                        }
                    });

                    // Geometric hover check — doesn't get stolen by child widgets.
                    let now_hovered = ctx
                        .input(|i| i.pointer.hover_pos())
                        .map_or(false, |p| frame_resp.response.rect.contains(p));
                    if now_hovered != was_hovered {
                        ui.memory_mut(|m| m.data.insert_temp(hover_id, now_hovered));
                        ctx.request_repaint();
                    }

                    // Detect left-click on the chip via raw pointer state.
                    // Frame responses have no click sense, and adding one
                    // via ui.interact would fight with context_menu, so we
                    // bypass the widget system entirely.
                    // Two modes: Ctrl+click removes immediately (no need to
                    // wait for × to appear), regular click when × is visible
                    // also removes.
                    if now_hovered {
                        let primary_clicked = ctx.input(|i| i.pointer.primary_clicked());
                        let ctrl_held = ctx.input(|i| i.modifiers.ctrl);
                        if primary_clicked && (ctrl_held || t > 0.3) {
                            pending = Some(PendingAction::RemoveTags(
                                vec![media_id], vec![tag.id],
                            ));
                        }
                        if ctrl_held {
                            ctx.set_cursor_icon(egui::CursorIcon::NotAllowed);
                        }
                    }
                }
            });
        });

    if pending.is_some() { state.pending_action = pending; }
    resp.response.rect
}

/// Render an icon-only button with a guaranteed-visible tooltip.
///
/// `Response::on_hover_text` was unreliable here — possibly due to the
/// button's parent Area sitting near the bottom of the screen, possibly
/// due to layer ordering with other overlays — so we drive the tooltip
/// ourselves via `egui::show_tooltip_text`. As long as the response is
/// hovered, the tooltip is shown immediately at the default position
/// near the cursor, on the dedicated Tooltip layer.
fn icon_btn(ui: &mut egui::Ui, icon: &str, tip: &str) -> egui::Response {
    let resp = ui.button(icon);
    if resp.hovered() {
        egui::show_tooltip_text(
            ui.ctx(),
            ui.layer_id(),
            egui::Id::new(("preview_toolbar_tip", tip)),
            tip,
        );
    }
    resp
}

fn draw_toolbar(
    ctx: &egui::Context,
    preview: &mut PreviewState,
    state: &mut AppState,
    media_id: i64,
    s: &ToolbarSettings,
    show_filmstrip: &mut bool,
    bottom_lift: f32,
    toolbar_pos: &mut ToolbarPosition,
) -> egui::Rect {
    // The toolbar is anchored at the screen's center-bottom by default.
    // User drags shift it by `toolbar_pos.offset_x` / `offset_y`, which
    // we fold into the anchor's offset so the existing layout flow
    // stays intact.
    //   x: positive offset = move right (anchor x is screen-relative)
    //   y: anchor uses screen-coords (down is positive). Default is
    //      `-50 - bottom_lift` (50 px above the bottom, plus filmstrip
    //      lift). Adding offset_y shifts further; positive offset_y
    //      moves the toolbar *down* toward the bottom edge.
    let anchor_offset = Vec2::new(
        toolbar_pos.offset_x,
        -50.0 - bottom_lift + toolbar_pos.offset_y,
    );
    let resp = egui::Area::new(egui::Id::new("preview_toolbar"))
        .anchor(egui::Align2::CENTER_BOTTOM, anchor_offset)
        // Foreground (not Tooltip) — egui's hover-tooltips draw at Tooltip
        // order, so anything sharing that layer hides them.
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            // Tooltips fire immediately, even if the cursor is still
            // moving — important on a tightly-packed toolbar.
            ui.style_mut().interaction.tooltip_delay = 0.0;
            ui.style_mut().interaction.show_tooltips_only_when_still = false;
            egui::Frame::none()
                .fill(Color32::from_rgba_premultiplied(20, 20, 25, 230))
                .rounding(8.0)
                .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                .shadow(egui::epaint::Shadow {
                    offset: Vec2::new(0.0, 4.0),
                    blur: 14.0,
                    spread: 0.0,
                    color: Color32::from_black_alpha(160),
                })
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Drag handle (always rendered — dimmed when locked
                        // so the user can right-click to unlock without
                        // having to know any keyboard shortcut).
                        draw_toolbar_handle(ui, toolbar_pos);
                    ui.horizontal(|ui| {
                        let mut anything_in_zoom = false;

                        if s.zoom_mode_button {
                            let label = match preview.zoom_mode {
                                ZoomMode::Center => "Zoom: Center",
                                ZoomMode::Cursor => "Zoom: Cursor",
                            };
                            if ui.button(label).clicked() {
                                preview.zoom_mode = match preview.zoom_mode {
                                    ZoomMode::Center => ZoomMode::Cursor,
                                    ZoomMode::Cursor => ZoomMode::Center,
                                };
                            }
                            anything_in_zoom = true;
                        }

                        if s.zoom_reset_button {
                            if ui.button("Reset").clicked() {
                                preview.fit_mode = true;
                                preview.pan = Vec2::ZERO;
                                preview.target_pan = Vec2::ZERO;
                            }
                            anything_in_zoom = true;
                        }

                        if s.zoom_presets {
                            let current_pct = (preview.zoom * 100.0).round() as i32;
                            let label = if preview.fit_mode {
                                "Fit".to_string()
                            } else {
                                format!("{}%", current_pct)
                            };
                            egui::ComboBox::from_id_salt("zoom_preset")
                                .selected_text(label)
                                .width(70.0)
                                .show_ui(ui, |ui| {
                                    for p in [25, 50, 100, 125, 150, 200] {
                                        if ui.selectable_label(false, format!("{}%", p)).clicked() {
                                            preview.target_zoom = p as f32 / 100.0;
                                            preview.fit_mode = false;
                                        }
                                    }
                                });
                            anything_in_zoom = true;
                        }

                        if anything_in_zoom && s.manip_category_visible() {
                            ui.label("|");
                        }

                        if s.rotate_buttons {
                            if icon_btn(ui, "↺", "Rotate counter-clockwise").clicked() {
                                preview.rotation = (preview.rotation + 270) % 360;
                            }
                            if icon_btn(ui, "↻", "Rotate clockwise").clicked() {
                                preview.rotation = (preview.rotation + 90) % 360;
                            }
                        }

                        if s.flip_buttons {
                            if icon_btn(ui, "↔", "Mirror horizontally").clicked() {
                                preview.flip_h = !preview.flip_h;
                            }
                            if icon_btn(ui, "↕", "Mirror vertically").clicked() {
                                preview.flip_v = !preview.flip_v;
                            }
                        }

                        if (s.manip_category_visible() || anything_in_zoom) && s.tags_category_visible() {
                            ui.label("|");
                        }

                        if s.tags_button {
                            let tag_tree = state.tag_tree.clone();
                            let assigned: std::collections::HashSet<i64> =
                                state.preview_tags.iter().map(|t| t.id).collect();
                            let mut pending: Option<PendingAction> = None;
                            let mut collapsed = state.collapsed_dropdown_tag_ids.clone();

                            // Custom popup that opens ABOVE the button with a
                            // small gap, so the popup never covers the button
                            // itself when the toolbar lives at the bottom of
                            // the preview area.
                            let btn = ui.button(format!("🏷 Tags ({}) ▾", assigned.len()));
                            let popup_id = ui.make_persistent_id(("tags_popup", media_id));
                            if btn.clicked() {
                                ui.memory_mut(|m| m.toggle_popup(popup_id));
                            }
                            if ui.memory(|m| m.is_popup_open(popup_id)) {
                                let gap = 6.0;
                                let pivot_pos = egui::pos2(btn.rect.left(), btn.rect.top() - gap);
                                let screen = ui.ctx().screen_rect();
                                let min_w = 240.0_f32;
                                let space_above = (btn.rect.top() - 16.0).max(200.0);
                                let max_h = (screen.height() * 0.90).clamp(360.0, space_above);
                                let mut filter = state.tag_dropdown_filter.clone();
                                let recent_ids = state.recent_tag_ids.clone();
                                let area = egui::Area::new(popup_id)
                                    .order(egui::Order::Foreground)
                                    .fixed_pos(pivot_pos)
                                    .pivot(egui::Align2::LEFT_BOTTOM)
                                    .interactable(true)
                                    .show(ui.ctx(), |ui| {
                                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                                            ui.set_height(max_h);
                                            // Search filter
                                            ui.horizontal(|ui| {
                                                ui.label("🔍");
                                                ui.add(egui::TextEdit::singleline(&mut filter)
                                                    .hint_text("Filter tags…")
                                                    .desired_width(180.0));
                                            });
                                            if tag_tree.is_empty() {
                                                ui.colored_label(Color32::GRAY, "No tags yet");
                                            } else {
                                                // Recent tags section
                                                if filter.is_empty() && !recent_ids.is_empty() {
                                                    ui.add_space(2.0);
                                                    ui.colored_label(crate::theme::TEXT_MUTED, "Recent");
                                                    for &tid in recent_ids.iter().take(6) {
                                                        if let Some(tag) = find_tag_in_tree(&tag_tree, tid) {
                                                            let icon = tag.icon.as_deref().filter(|s| !s.is_empty()).unwrap_or("🏷");
                                                            let is_assigned = assigned.contains(&tid);
                                                            let check = if is_assigned { "☑" } else { "☐" };
                                                            let bg = tag_color32(&tag);
                                                            ui.horizontal(|ui| {
                                                                ui.label(egui::RichText::new(icon).color(bg));
                                                                let label = format!("{}  {}", check, tag.name);
                                                                if ui.selectable_label(is_assigned, label).clicked() {
                                                                    pending = Some(if is_assigned {
                                                                        PendingAction::RemoveTags(vec![media_id], vec![tid])
                                                                    } else {
                                                                        PendingAction::AssignTags(vec![media_id], vec![tid])
                                                                    });
                                                                }
                                                            });
                                                        }
                                                    }
                                                    ui.separator();
                                                }
                                                let filter_lower = filter.trim().to_lowercase();
                                                egui::ScrollArea::vertical()
                                                    .max_height(max_h - 60.0)
                                                    .auto_shrink([true, false])
                                                    .show(ui, |ui| {
                                                        show_tag_assign_menu(
                                                            ui, &tag_tree, media_id,
                                                            &assigned, &mut pending,
                                                            &mut collapsed, 0,
                                                            &filter_lower,
                                                        );
                                                    });
                                            }
                                        });
                                    });
                                state.tag_dropdown_filter = filter;
                                // Close on click outside (covers both the
                                // popup body and the button to avoid a
                                // re-toggle race on the same click).
                                let clicked = ui.input(|i| i.pointer.any_click());
                                if clicked {
                                    let pos = ui.ctx().pointer_interact_pos();
                                    let inside_popup = pos
                                        .map(|p| area.response.rect.contains(p))
                                        .unwrap_or(false);
                                    let inside_btn = pos
                                        .map(|p| btn.rect.contains(p))
                                        .unwrap_or(false);
                                    if !inside_popup && !inside_btn {
                                        ui.memory_mut(|m| m.close_popup());
                                    }
                                }
                                // Keep popup open after tag assignment so
                                // the user can assign/remove multiple tags
                                // without re-opening.
                                // Consume scroll inside the popup so it
                                // doesn't zoom the background image.
                                let over_popup = ui.input(|i| i.pointer.hover_pos())
                                    .map_or(false, |p| area.response.rect.contains(p));
                                if over_popup {
                                    ui.ctx().input_mut(|i| { i.raw_scroll_delta = Vec2::ZERO; });
                                }
                            }
                            if pending.is_some() { state.pending_action = pending; }
                            state.collapsed_dropdown_tag_ids = collapsed;

                            // "Clear all tags" — wipes every tag assignment
                            // from the current media item. Disabled when
                            // nothing is assigned.
                            let clear_enabled = !assigned.is_empty();
                            let clear_resp = ui.add_enabled(
                                clear_enabled,
                                egui::Button::new("✕ Clear tags"),
                            ).on_hover_text("Remove every tag from this item");
                            if clear_resp.clicked() {
                                state.pending_action = Some(
                                    PendingAction::ClearMediaTags(vec![media_id])
                                );
                            }
                        }

                        if (s.tags_category_visible() || s.manip_category_visible() || anything_in_zoom)
                            && s.file_category_visible()
                        {
                            ui.label("|");
                        }

                        if s.delete_button && icon_btn(ui, "🗑", "Delete (Del/Backspace)").clicked() {
                            state.confirm_delete = Some(media_id);
                        }
                        if s.copy_button && icon_btn(ui, "📋", "Copy to…").clicked() {
                            state.pending_action = Some(PendingAction::CopyMedia { media_id });
                        }
                        if s.move_button && icon_btn(ui, "➡", "Move to…").clicked() {
                            state.pending_action = Some(PendingAction::MoveMedia { media_id, advance_preview: true });
                        }
                        if s.explorer_button && icon_btn(ui, "📂", "Show in Explorer").clicked() {
                            state.pending_action = Some(PendingAction::ShowInExplorer { media_id });
                        }
                        if s.open_external_button && icon_btn(ui, "🔗", "Open in default app").clicked() {
                            state.pending_action = Some(PendingAction::OpenExternal { media_id });
                        }

                        // ── View ──────────────────────────────────────
                        if (s.file_category_visible() || s.tags_category_visible()
                            || s.manip_category_visible()
                            || s.zoom_category_visible())
                           && s.view_category_visible()
                        {
                            ui.label("|");
                        }
                        if s.filmstrip_toggle {
                            // Toggle button: on/off state shown via the
                            // selectable-label highlight. Tooltip names
                            // the action that the click would take.
                            let tip = if *show_filmstrip { "Hide filmstrip" } else { "Show filmstrip" };
                            let resp = ui.selectable_label(*show_filmstrip, "🎞");
                            if resp.hovered() {
                                egui::show_tooltip_text(
                                    ui.ctx(),
                                    ui.layer_id(),
                                    egui::Id::new(("filmstrip_tip", *show_filmstrip)),
                                    tip,
                                );
                            }
                            if resp.clicked() {
                                *show_filmstrip = !*show_filmstrip;
                            }
                        }
                    });
                    });
                });
        });

    resp.response.rect
}

/// Pill-shaped drag handle painted at the top of the floating preview
/// toolbar. Drag to reposition; right-click for the lock + reset menu.
/// When locked, the pill is dimmed and dragging is disabled — but the
/// context menu still works so the user can unlock without a keybind.
fn draw_toolbar_handle(ui: &mut egui::Ui, toolbar_pos: &mut ToolbarPosition) {
    let pill_w = 36.0;
    let pill_h = 4.0;
    let row_h = 10.0;
    let avail = ui.available_width().max(pill_w + 20.0);
    let (row_rect, _) = ui.allocate_exact_size(
        Vec2::new(avail, row_h),
        egui::Sense::hover(),
    );
    // Pill positioned center-top of the row (just above the buttons).
    let pill_rect = egui::Rect::from_center_size(
        egui::pos2(row_rect.center().x, row_rect.center().y),
        Vec2::new(pill_w, pill_h),
    );
    // Generous click target — easier to grab than the visible pill.
    let hit_rect = pill_rect.expand2(Vec2::new(8.0, 6.0));
    let resp = ui.interact(
        hit_rect,
        ui.id().with("toolbar_handle"),
        egui::Sense::click_and_drag(),
    );

    if !toolbar_pos.locked && resp.dragged() {
        let delta = resp.drag_delta();
        toolbar_pos.offset_x += delta.x;
        toolbar_pos.offset_y += delta.y;
    }
    resp.context_menu(|ui| {
        let lock_label = if toolbar_pos.locked {
            "🔓 Unlock position"
        } else {
            "🔒 Lock position"
        };
        if ui.button(lock_label).clicked() {
            toolbar_pos.locked = !toolbar_pos.locked;
            ui.close_menu();
        }
        if ui.button("↺ Reset to default").clicked() {
            toolbar_pos.offset_x = 0.0;
            toolbar_pos.offset_y = 0.0;
            ui.close_menu();
        }
    });

    if !toolbar_pos.locked && resp.hovered() {
        ui.ctx().set_cursor_icon(if resp.dragged() {
            egui::CursorIcon::Grabbing
        } else {
            egui::CursorIcon::Grab
        });
    }

    // Paint the pill: dim when locked, normal when idle, brightest on
    // hover/drag. RGB stays neutral so it reads as chrome rather than
    // a coloured affordance.
    let alpha: u8 = if toolbar_pos.locked {
        70
    } else if resp.dragged() || resp.hovered() {
        220
    } else {
        150
    };
    ui.painter().rect_filled(
        pill_rect,
        egui::Rounding::same(pill_h * 0.5),
        Color32::from_rgba_unmultiplied(190, 190, 200, alpha),
    );
}
