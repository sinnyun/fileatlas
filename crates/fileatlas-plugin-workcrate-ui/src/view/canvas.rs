//! Canvas editing state: the official render, the official selection box and the
//! pixel↔point mapping that turns FileAtlas input into an official gesture.
//!
//! Nothing here edits artwork. Every semantic (hit test, selection, move, scale, text,
//! link) runs inside VectorCraft; this module records pointer events as document points,
//! shows a visual prediction while the pointer is down, and paints the bounds the official
//! program reports back.
use super::*;
use ::gpui::{Bounds, Pixels, Point, point, size};

pub(super) const CANVAS_RAIL_WIDTH: f32 = 190.;
pub(super) const CANVAS_INSPECTOR_WIDTH: f32 = 310.;
pub(super) const HANDLE: f32 = 7.;
pub(super) const GRAB: f32 = 8.;
pub(super) const ZOOM_MIN: f32 = 0.25;
pub(super) const ZOOM_MAX: f32 = 4.;
const MAX_GESTURE_EVENTS: usize = 64;

/// Which official edit the recorded gesture will become. Only affects the prediction box.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum CanvasDragKind {
    Body,
    Handle(u8),
}

/// An in-progress pointer gesture. Coordinates are document points, so the prediction
/// survives scrolling and re-layout until the official program answers with real bounds.
pub(super) struct CanvasDrag {
    pub kind: CanvasDragKind,
    pub origin: CanvasGeometry,
    pub start: (f64, f64),
    pub current: (f64, f64),
    events: Vec<serde_json::Value>,
}

impl CanvasDrag {
    /// Start recording one official gesture. The sequence, its transport cap and the
    /// down/drag/up shape all belong here, so the surface only forwards pointer points.
    pub(super) fn begin(
        kind: CanvasDragKind,
        origin: CanvasGeometry,
        at: (f64, f64),
        down: (f64, f64),
    ) -> Self {
        Self {
            kind,
            origin,
            start: at,
            current: at,
            events: vec![gesture_event("down", down)],
        }
    }

    /// Record a drag sample. Sub-half-point moves change nothing and repaint nothing.
    /// Corner samples are steered onto the proportional path through the opposite
    /// corner: the official gesture transport ignores key modifiers, so the sampled
    /// points are the only way to hand it a shift-style constrained scale.
    pub(super) fn drag_to(&mut self, point: (f64, f64)) -> bool {
        let point = self.constrain(point);
        if (point.0 - self.current.0).abs() < 0.5 && (point.1 - self.current.1).abs() < 0.5 {
            return false;
        }
        self.current = point;
        if self.events.len() < MAX_GESTURE_EVENTS {
            self.events.push(gesture_event("drag", point));
        } else if let Some(last) = self.events.last_mut() {
            *last = gesture_event("drag", point);
        }
        true
    }

    /// Close the sequence with the release point; the whole gesture runs officially.
    pub(super) fn finish(mut self, up: (f64, f64)) -> Vec<serde_json::Value> {
        let up = self.constrain(up);
        if self.events.len() > 1 {
            // The official engine moves on drag, not up. Keep the release position
            // even when pointer sampling was capped or the last move was coalesced.
            if let Some(last) = self.events.last_mut() {
                *last = gesture_event("drag", up);
            }
        }
        self.events.push(gesture_event("up", up));
        self.events
    }

    fn constrain(&self, point: (f64, f64)) -> (f64, f64) {
        match self.kind {
            CanvasDragKind::Handle(handle) => constrain_corner(handle, &self.origin, point),
            CanvasDragKind::Body => point,
        }
    }
}

/// One official frame: the truth about the document plus the decoded bitmap, the
/// detail of a single selected object and the page record the frame belongs to.
pub(super) struct CanvasFrame {
    pub snapshot: CanvasSnapshot,
    pub image: Option<Arc<RenderImage>>,
    pub object: Option<CanvasObject>,
    pub board: Option<WorkCrateBoard>,
}

pub(super) fn canvas_frame(
    boards: &WorkCrateBoardsService,
    snapshot: CanvasSnapshot,
) -> CanvasFrame {
    CanvasFrame {
        image: snapshot
            .preview
            .as_deref()
            .and_then(|path| image::open(path).ok())
            .map(|image| render_image(image.into_rgba8())),
        object: match snapshot.selection.as_slice() {
            [id] => boards.canvas_object(*id).ok(),
            _ => None,
        },
        board: boards.board(&snapshot.board_id).ok(),
        snapshot,
    }
}

fn gesture_event(kind: &str, (x, y): (f64, f64)) -> serde_json::Value {
    serde_json::json!({
        "kind": kind,
        "x": (x * 100.).round() / 100.,
        "y": (y * 100.).round() / 100.,
    })
}

/// The single owner of overlay pixel ↔ document point mapping. The official bitmap covers
/// exactly one artboard, so a document point is that artboard's origin plus its local
/// offset, scaled by the displayed size.
#[derive(Clone)]
pub(super) struct CanvasMap {
    pub origin: Point<Pixels>,
    pub artboard: CanvasGeometry,
    scale: f32,
}

impl CanvasMap {
    pub(super) fn new(rect: Bounds<Pixels>, artboard: CanvasGeometry) -> Option<Self> {
        let scale = f32::from(rect.size.width) / artboard.width as f32;
        (scale.is_finite() && scale > 0.).then_some(Self {
            origin: rect.origin,
            artboard,
            scale,
        })
    }
    pub(super) fn to_document(&self, position: Point<Pixels>) -> (f64, f64) {
        (
            self.artboard.x + f64::from(f32::from(position.x - self.origin.x)) / self.scale as f64,
            self.artboard.y + f64::from(f32::from(position.y - self.origin.y)) / self.scale as f64,
        )
    }
    /// A rectangle inside the artboard, in window pixels.
    pub(super) fn rect_of(&self, geometry: &CanvasGeometry) -> Bounds<Pixels> {
        Bounds {
            origin: self.point_of(geometry.x, geometry.y),
            size: size(
                px(geometry.width as f32 * self.scale),
                px(geometry.height as f32 * self.scale),
            ),
        }
    }
    pub(super) fn point_of(&self, x: f64, y: f64) -> Point<Pixels> {
        point(
            self.origin.x + px((x - self.artboard.x) as f32 * self.scale),
            self.origin.y + px((y - self.artboard.y) as f32 * self.scale),
        )
    }
}

pub(super) fn handle_point(geometry: &CanvasGeometry, handle: u8) -> (f64, f64) {
    let (left, top) = (geometry.x, geometry.y);
    let (right, bottom) = (geometry.x + geometry.width, geometry.y + geometry.height);
    let (middle_x, middle_y) = (left + geometry.width / 2., top + geometry.height / 2.);
    match handle {
        0 => (left, top),
        1 => (middle_x, top),
        2 => (right, top),
        3 => (left, middle_y),
        4 => (right, middle_y),
        5 => (left, bottom),
        6 => (middle_x, bottom),
        _ => (right, bottom),
    }
}

/// Corner drags keep the box ratio through the fixed opposite corner. Negative extents
/// flip the box through the anchor, matching the official shift-scale behavior.
fn constrain_corner(handle: u8, origin: &CanvasGeometry, point: (f64, f64)) -> (f64, f64) {
    let (w0, h0) = (origin.width, origin.height);
    if !matches!(handle, 0 | 2 | 5 | 7) || !(w0.is_finite() && h0.is_finite() && w0 > 0. && h0 > 0.)
    {
        return point;
    }
    let (left, top) = (origin.x, origin.y);
    let (right, bottom) = (origin.x + w0, origin.y + h0);
    let (anchor, sign) = match handle {
        0 => ((right, bottom), (-1., -1.)),
        2 => ((left, bottom), (1., -1.)),
        5 => ((right, top), (-1., 1.)),
        _ => ((left, top), (1., 1.)),
    };
    let extent_x = sign.0 * (point.0 - anchor.0);
    let extent_y = sign.1 * (point.1 - anchor.1);
    let s = (extent_x / w0).max(extent_y / h0);
    (anchor.0 + sign.0 * s * w0, anchor.1 + sign.1 * s * h0)
}

/// The visual-only box shown while the pointer is down. The released gesture is executed by
/// VectorCraft, and its reported bounds replace this.
pub(super) fn predict(drag: &CanvasDrag, pointer: (f64, f64)) -> CanvasGeometry {
    let origin = &drag.origin;
    match drag.kind {
        CanvasDragKind::Body => CanvasGeometry {
            x: origin.x + pointer.0 - drag.start.0,
            y: origin.y + pointer.1 - drag.start.1,
            ..*origin
        },
        CanvasDragKind::Handle(handle) => {
            let (mut left, mut top) = (origin.x, origin.y);
            let (mut right, mut bottom) = (origin.x + origin.width, origin.y + origin.height);
            match handle {
                0 => {
                    left = pointer.0;
                    top = pointer.1;
                }
                1 => top = pointer.1,
                2 => {
                    right = pointer.0;
                    top = pointer.1;
                }
                3 => left = pointer.0,
                4 => right = pointer.0,
                5 => {
                    left = pointer.0;
                    bottom = pointer.1;
                }
                6 => bottom = pointer.1,
                _ => {
                    right = pointer.0;
                    bottom = pointer.1;
                }
            }
            CanvasGeometry {
                x: left.min(right),
                y: top.min(bottom),
                width: (right - left).abs(),
                height: (bottom - top).abs(),
            }
        }
    }
}

impl WorkCratePanel {
    /// The last painted viewport of the canvas area; the panel bounds keep the first
    /// editing frame usable instead of collapsing.
    pub(super) fn canvas_viewport(&self) -> Bounds<Pixels> {
        self.control_bounds("canvas-viewport").unwrap_or_else(|| {
            let (width, height) = self.panel_content_size();
            Bounds {
                origin: point(px(0.), px(0.)),
                size: size(
                    px(
                        (width - f64::from(CANVAS_RAIL_WIDTH + CANVAS_INSPECTOR_WIDTH + 60.))
                            .max(360.) as f32,
                    ),
                    px((height as f32 - 190.).max(260.)),
                ),
            }
        })
    }

    pub(super) fn control_bounds(&self, id: &str) -> Option<Bounds<Pixels>> {
        let values = self.surfaces.value(&format!("workcrate.control.{id}"))?;
        let n = values
            .split(',')
            .map(str::parse::<f32>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        (n.len() == 4).then(|| Bounds {
            origin: point(px(n[0]), px(n[1])),
            size: size(px(n[2]), px(n[3])),
        })
    }

    /// Mapping in window pixels, used to turn pointer events into document points.
    pub(super) fn canvas_window_map(&self) -> Option<CanvasMap> {
        let snapshot = self.canvas.as_ref()?;
        CanvasMap::new(
            self.control_bounds("canvas-artboard")?,
            snapshot.artboard.clone(),
        )
    }

    pub(super) fn clear_canvas_state(&mut self) {
        self.canvas_mode = false;
        self.canvas = None;
        self.canvas_image = None;
        self.canvas_object = None;
        self.canvas_drag = None;
        self.canvas_poll += 1;
    }

    /// Sample the official session on a timer while the canvas is open. The timer is only
    /// a sampling opportunity: an unchanged probe neither wakes nor repaints, and leaving
    /// the canvas stops the loop by invalidating its generation.
    pub(super) fn start_canvas_poller(&mut self, cx: &mut ViewContext<Self>) {
        self.canvas_poll += 1;
        let generation = self.canvas_poll;
        refresh::poll(cx, std::time::Duration::from_millis(800), move |view, _| {
            if view.canvas_poll != generation {
                return refresh::PollResult::until(false, false);
            }
            view.request_canvas_probe()
        });
    }

    pub(super) fn request_canvas_probe(&mut self) -> refresh::PollResult {
        if self.canvas.is_none() && !self.canvas_mode && !self.canvas_busy {
            return refresh::PollResult::until(false, false);
        }
        // The open snapshot arrives through canvas_job; probing before it exists would
        // report a change on every tick.
        let Some(known) = self.canvas.as_ref().map(|snapshot| snapshot.revision) else {
            return refresh::PollResult::until(false, true);
        };
        if self.canvas_busy || self.canvas_drag.is_some() || self.canvas_read.is_pending() {
            return refresh::PollResult::until(false, true);
        }
        self.canvas_probe_tick += 1;
        let boards = self.boards.clone();
        self.canvas_read.request(
            (self.canvas_poll, known, self.canvas_probe_tick),
            move || boards.canvas_probe(known).map_err(|e| e.to_string()),
        );
        refresh::PollResult::until(false, true)
    }

    /// An official revision FileAtlas did not cause: show it at once, then ask the
    /// official program to re-render the page so the bitmap matches too.
    pub(super) fn deliver_canvas_probe(&mut self, cx: &mut ViewContext<Self>) -> bool {
        if !self.canvas_read.poll() {
            return false;
        }
        if self.canvas.is_none() || self.canvas_busy || self.canvas_drag.is_some() {
            return false;
        }
        if self.canvas_read.key().is_none_or(|(generation, known, _)| {
            *generation != self.canvas_poll
                || self.canvas.as_ref().is_none_or(|s| s.revision != *known)
        }) {
            return false;
        }
        match self.canvas_read.value().cloned() {
            Some(Ok(Some(snapshot))) => {
                if self.canvas.as_ref().is_none_or(|current| {
                    current.board_id != snapshot.board_id || current.revision == snapshot.revision
                }) {
                    return false;
                }
                self.canvas_job(
                    |boards| boards.canvas_refresh().map_err(|e| e.to_string()),
                    cx,
                );
                true
            }
            Some(Err(error)) => {
                self.clear_canvas_state();
                let boards = self.boards.clone();
                self.perform(
                    move || {
                        boards.canvas_close().map_err(|e| e.to_string())?;
                        Err(error)
                    },
                    cx,
                );
                true
            }
            _ => false,
        }
    }

    pub(super) fn toggle_canvas_mode(&mut self, editing: bool, cx: &mut ViewContext<Self>) {
        if editing == self.canvas_mode
            || self.busy
            || self.canvas_busy
            || !self.field_saves.is_empty()
        {
            return;
        }
        self.canvas_mode = editing;
        if self.canvas.is_some() {
            // Preview hides editing affordances, not the live official frame.
            self.canvas_drag = None;
            if editing {
                self.switch_canvas_page(cx);
            }
            cx.notify();
            return;
        }
        if editing {
            let Some(id) = self.selected_board.clone() else {
                return;
            };
            self.start_canvas_poller(cx);
            self.canvas_job(
                move |boards| boards.canvas_open(&id).map_err(|e| e.to_string()),
                cx,
            );
            return;
        }
        cx.notify();
    }

    /// Show another page of the same document in the already open editor. The official
    /// session keeps its process, history and access folders; only the artboard it renders
    /// moves. A switch requested while another frame is in flight converges when that
    /// frame lands (see `apply_canvas_frame`).
    pub(super) fn switch_canvas_page(&mut self, cx: &mut ViewContext<Self>) {
        let Some(id) = self.selected_board.clone() else {
            return;
        };
        if self.canvas.is_none()
            || self
                .canvas
                .as_ref()
                .is_some_and(|state| state.board_id == id)
        {
            return;
        }
        if self.busy || self.canvas_busy || !self.field_saves.is_empty() {
            return;
        }
        self.canvas_drag = None;
        self.start_canvas_poller(cx);
        self.canvas_job(
            move |boards| boards.canvas_open(&id).map_err(|e| e.to_string()),
            cx,
        );
    }

    /// Another client re-read the visible page. Reuse or reopen the session and repaint
    /// that one card; the catalogue, the other cards and their previews stay untouched.
    pub(super) fn refresh_visible_board(&mut self, cx: &mut ViewContext<Self>) {
        let Some(id) = self.selected_board.clone() else {
            return;
        };
        self.canvas_job(
            move |boards| boards.canvas_open(&id).map_err(|e| e.to_string()),
            cx,
        );
    }

    /// Close the official session and refresh the cached catalogue, so the page matrix shows
    /// the same document the edits just wrote.
    pub(super) fn leave_canvas(&mut self, cx: &mut ViewContext<Self>) {
        // A field save owns the official session until it returns; closing it here would
        // hand the UI a frame from a session that no longer exists.
        if !self.field_saves.is_empty() {
            return;
        }
        self.clear_canvas_state();
        let Some(project) = self.selected_project.clone() else {
            return;
        };
        let boards = self.boards.clone();
        self.perform(
            move || {
                if boards.canvas_is_open() {
                    boards.canvas_commit().map_err(|e| e.to_string())?;
                    boards.canvas_close().map_err(|e| e.to_string())?;
                }
                boards
                    .refresh_project(&project)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }

    /// Hand the document to the official GUI: the canvas session is committed and closed
    /// first so only one program holds the file.
    pub(super) fn canvas_handoff_editor(&mut self, cx: &mut ViewContext<Self>) {
        let Some(board) = self.selected_board.clone() else {
            return;
        };
        self.clear_canvas_state();
        self.start_canvas_poller(cx);
        self.canvas_job(
            move |boards| {
                if boards.canvas_is_open() {
                    boards.canvas_commit().map_err(|e| e.to_string())?;
                    boards.canvas_close().map_err(|e| e.to_string())?;
                }
                boards.open_editor(&board).map_err(|e| e.to_string())?;
                boards.canvas_open(&board).map_err(|e| e.to_string())
            },
            cx,
        );
    }

    /// Run one official canvas operation off the UI thread and repaint from its result.
    pub(super) fn canvas_job(
        &mut self,
        run: impl FnOnce(&Arc<WorkCrateBoardsService>) -> Result<CanvasSnapshot, String>
        + Send
        + 'static,
        cx: &mut ViewContext<Self>,
    ) {
        if self.canvas_busy || self.busy || !self.field_saves.is_empty() {
            return;
        }
        self.canvas_busy = true;
        self.message = None;
        cx.notify();
        let generation = self.canvas_poll;
        let boards = self.boards.clone();
        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_executor()
                .spawn(async move {
                    let snapshot = run(&boards)?;
                    Ok::<_, String>(canvas_frame(&boards, snapshot))
                })
                .await;
            let _ = this.update_in(cx, |view: &mut WorkCratePanel, window, cx| {
                view.canvas_busy = false;
                if view.canvas_poll != generation {
                    // Leaving the canvas, or opening another page, invalidated this frame:
                    // the session it describes is already closed.
                    cx.notify();
                    return;
                }
                match outcome {
                    Ok(frame) => view.apply_canvas_frame(frame, window, cx),
                    Err(error) => view.message = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn apply_canvas_frame(
        &mut self,
        frame: CanvasFrame,
        window: &mut Window,
        cx: &mut ViewContext<Self>,
    ) {
        let CanvasFrame {
            snapshot,
            image,
            object,
            board,
        } = frame;
        let stale_page = self.selected_board.as_deref() != Some(snapshot.board_id.as_str());
        if let Some(card) = self
            .boards_view
            .iter_mut()
            .find(|c| c.board.id == snapshot.board_id)
        {
            if let Some(board) = board {
                card.board = board;
            }
            card.image = image.clone();
            card.preview_checked = true;
            card.error = None;
            card.canvas_revision = Some(snapshot.revision);
        }
        self.canvas = Some(snapshot);
        self.canvas_image = image;
        self.on_canvas_object(object, window, cx);
        if stale_page {
            // The user picked another page while this frame was rendering; the session
            // already holds the document, so move it to the selected page now.
            self.switch_canvas_page(cx);
        }
    }

    /// Hook for the object panel: keeps its fields in step with the official selection.
    fn on_canvas_object(
        &mut self,
        object: Option<CanvasObject>,
        window: &mut Window,
        cx: &mut ViewContext<Self>,
    ) {
        let changed_selection =
            self.canvas_object.as_ref().map(|o| o.id) != object.as_ref().map(|o| o.id);
        self.canvas_object = object;
        if changed_selection {
            for input in &self.canvas_numbers {
                input.update(cx, |state, cx| state.set_value("", window, cx));
            }
            self.canvas_text_input
                .update(cx, |state, cx| state.set_value("", window, cx));
        }
        self.sync_canvas_inputs(changed_selection, window, cx);
    }

    /// Push the official bounds and text into the panel fields. A field the user is
    /// typing into keeps its content; the next selection change refreshes it.
    fn sync_canvas_inputs(&mut self, force: bool, window: &mut Window, cx: &mut ViewContext<Self>) {
        let bounds = self
            .canvas_object
            .as_ref()
            .and_then(|object| object.bounds.clone())
            .or_else(|| {
                self.canvas
                    .as_ref()
                    .and_then(|snapshot| snapshot.selection_bounds.clone())
            });
        let numbers = match bounds {
            Some(bounds) => [
                Some(bounds.x),
                Some(bounds.y),
                Some(bounds.width),
                Some(bounds.height),
            ],
            None => [None; 4],
        };
        let inputs = self.canvas_numbers.clone();
        for (index, input) in inputs.iter().enumerate() {
            let value = numbers[index].map(|number| format!("{number:.1}"));
            self.sync_input(input, value, force, window, cx);
        }
        let text = self
            .canvas_object
            .as_ref()
            .filter(|object| object.kind.eq_ignore_ascii_case("type"))
            .map(|object| object.text.clone());
        if let Some(text) = text {
            let input = &self.canvas_text_input;
            if (force || !input.read(cx).focus_handle(cx).is_focused(window))
                && input.read(cx).value() != text
            {
                input.update(cx, |state, cx| state.set_value(text, window, cx));
            }
        }
    }

    fn sync_input(
        &mut self,
        input: &Entity<gpui_kit::component::input::InputState>,
        value: Option<String>,
        force: bool,
        window: &mut Window,
        cx: &mut ViewContext<Self>,
    ) {
        let Some(value) = value else {
            return;
        };
        if (!force && input.read(cx).focus_handle(cx).is_focused(window))
            || input.read(cx).value() == value
        {
            return;
        }
        input.update(cx, |state, cx| state.set_value(value, window, cx));
    }

    pub(super) fn set_canvas_zoom(&mut self, factor: f32, cx: &mut ViewContext<Self>) {
        self.canvas_zoom = (self.canvas_zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
        cx.notify();
    }

    pub(super) fn canvas_undo_redo(&mut self, redo: bool, cx: &mut ViewContext<Self>) {
        self.canvas_job(
            move |boards| {
                if redo {
                    boards.canvas_redo()
                } else {
                    boards.canvas_undo()
                }
                .map_err(|e| e.to_string())
            },
            cx,
        );
    }
}

pub(super) fn canvas_status(snapshot: Option<&CanvasSnapshot>) -> String {
    let Some(snapshot) = snapshot else {
        return "正在连接官方画布…".into();
    };
    format!(
        "{} · 第 {} 步{}",
        if snapshot.live {
            "已连接 VectorCraft 窗口"
        } else {
            "后台官方会话"
        },
        snapshot.revision,
        if snapshot.dirty { " · 未保存" } else { "" }
    )
}

#[cfg(test)]
mod tests {
    use super::{
        CanvasDrag, CanvasDragKind, CanvasGeometry, CanvasMap, MAX_GESTURE_EVENTS, handle_point,
        predict,
    };
    use ::gpui::{Bounds, point, px, size};

    /// The artboard occupies 200 × 400 pt starting at (-40, 20) and is displayed at
    /// 600 × 1200 px in the window, i.e. 3 px per pt with a (120, 60) px offset.
    fn map() -> CanvasMap {
        match CanvasMap::new(
            Bounds {
                origin: point(px(120.), px(60.)),
                size: size(px(600.), px(1200.)),
            },
            CanvasGeometry {
                x: -40.,
                y: 20.,
                width: 200.,
                height: 400.,
            },
        ) {
            Some(map) => map,
            None => panic!("a finite artboard must map onto the window"),
        }
    }

    #[test]
    fn window_pixels_and_document_points_map_both_ways() {
        let map = map();
        assert_eq!(map.to_document(point(px(120.), px(60.))), (-40., 20.));
        assert_eq!(map.to_document(point(px(420.), px(360.))), (60., 120.));
        let back = map.point_of(60., 120.);
        assert_eq!(
            (f32::from(back.x), f32::from(back.y)),
            (420., 360.),
            "mapping must return to the same window pixel"
        );
        let rect = map.rect_of(&CanvasGeometry {
            x: 10.,
            y: 40.,
            width: 50.,
            height: 25.,
        });
        assert_eq!(
            (f32::from(rect.origin.x), f32::from(rect.origin.y)),
            (270., 120.)
        );
        assert_eq!(
            (f32::from(rect.size.width), f32::from(rect.size.height)),
            (150., 75.)
        );
    }

    #[test]
    fn handle_points_cover_the_box_edges() {
        let bounds = CanvasGeometry {
            x: 10.,
            y: 10.,
            width: 100.,
            height: 50.,
        };
        assert_eq!(handle_point(&bounds, 0), (10., 10.));
        assert_eq!(handle_point(&bounds, 4), (110., 35.));
        assert_eq!(handle_point(&bounds, 7), (110., 60.));
    }

    #[test]
    fn body_drag_translates_and_handle_drag_anchors_the_opposite_corner() {
        let origin = CanvasGeometry {
            x: 10.,
            y: 10.,
            width: 100.,
            height: 50.,
        };
        let body = CanvasDrag::begin(CanvasDragKind::Body, origin.clone(), (20., 20.), (20., 20.));
        let moved = predict(&body, (25., 30.));
        assert_eq!(
            (moved.x, moved.y, moved.width, moved.height),
            (15., 20., 100., 50.)
        );
        let corner = CanvasDrag::begin(CanvasDragKind::Handle(7), origin, (110., 60.), (110., 60.));
        let scaled = predict(&corner, (130., 40.));
        assert_eq!(
            (scaled.x, scaled.y),
            (10., 10.),
            "top-left must stay anchored"
        );
        assert_eq!((scaled.width, scaled.height), (120., 30.));
        let flipped = predict(&corner, (0., 100.));
        assert_eq!(
            (flipped.x, flipped.y, flipped.width, flipped.height),
            (0., 10., 10., 90.),
            "dragging the corner past the left edge keeps the box positive"
        );
    }

    #[test]
    fn tiny_moves_are_ignored_and_the_gesture_sequence_stays_bounded() {
        let mut drag =
            CanvasDrag::begin(CanvasDragKind::Body, Default::default(), (0., 0.), (0., 0.));
        assert!(
            !drag.drag_to((0.4, 0.4)),
            "sub-half-point moves repaint nothing"
        );
        assert!(drag.drag_to((6., 6.)));
        for step in 0..200 {
            drag.drag_to((6. + step as f64, 6.));
        }
        let events = drag.finish((100., 100.));
        assert_eq!(
            events.len(),
            MAX_GESTURE_EVENTS + 1,
            "one down, {MAX_GESTURE_EVENTS} recorded samples and the release"
        );
        let sample = |index: usize| events.get(index).cloned().unwrap_or_default();
        assert_eq!(sample(0)["kind"], "down");
        assert_eq!(sample(events.len() - 1)["kind"], "up");
        assert_eq!(sample(events.len() - 1)["x"], 100.);
        assert_eq!(sample(events.len() - 2)["kind"], "drag");
        assert_eq!(sample(events.len() - 2)["x"], 100.);
        assert_eq!(sample(events.len() - 2)["y"], 100.);
    }

    #[test]
    fn corner_drags_sample_the_proportional_path_and_side_handles_stay_free() {
        let origin = CanvasGeometry {
            x: 10.,
            y: 10.,
            width: 100.,
            height: 50.,
        };
        let mut corner = CanvasDrag::begin(
            CanvasDragKind::Handle(7),
            origin.clone(),
            (110., 60.),
            (110., 60.),
        );
        corner.drag_to((130., 40.));
        let shown = predict(&corner, corner.current);
        assert_eq!(
            (shown.width, shown.height),
            (120., 60.),
            "the 2:1 box ratio survives the dominant axis"
        );
        let events = corner.finish((150., 30.));
        let Some(last) = events.last() else {
            panic!("finish always appends the release");
        };
        // s = max(140/100, 20/50) = 1.4 -> the release point is steered onto the diagonal.
        assert_eq!(
            (last["x"].as_f64(), last["y"].as_f64()),
            (Some(150.), Some(80.))
        );

        let mut side =
            CanvasDrag::begin(CanvasDragKind::Handle(4), origin, (110., 35.), (110., 35.));
        side.drag_to((130., 70.));
        let freed = predict(&side, side.current);
        assert_eq!(
            (freed.width, freed.height),
            (120., 50.),
            "edge handles resize one axis only"
        );
    }
}
