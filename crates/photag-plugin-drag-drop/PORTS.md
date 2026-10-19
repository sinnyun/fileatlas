# Drag-out plugin ports

Status: Gallery keeps GPUI's card gesture and selected-file resolution. Its
callback posts a request to a temporary message-only window on the same UI
thread and returns. Windows dispatches that request after the originating
input callback unwinds; the dispatcher then starts the native session with the
pinned `drag` crate 2.1.1. The dispatch window is destroyed when the native
session returns. The user confirmed successful delivery to other windows on
Windows; the 2026-10-19 record covers current preview-image behavior and the
remaining visual confirmation.

## Cordis identity and lifecycle

- Plugin: `photag.drag-drop` (`DragDropPlugin`, `DragDropConfig`).
- Service: `photag/drag-drop` (`DragDropService`). It has no plugin
  dependencies and publishes its service while active. Cordis owns the
  publication and removes it when the plugin stops; the service registers no
  UI contributions. Its dispatch window is transient and owned by one queued
  drag request.
- Shell includes this crate in the Cargo workspace and starts
  `DragDropPlugin` before Gallery. Gallery resolves the service during setup
  and calls it from the card drag gesture.

## Public operation

- `DragDropService::start_file_drag(window, &[PathBuf]) -> Result<(),
  DragDropError>` queues a copy-only native drag request with the source window
  handle. The call returns before OLE begins; the posted dispatcher callback
  remains on the GPUI UI thread and returns after drop or cancel. Empty input
  is a no-op. The first path supplies the drag image; all paths are delivered
  in order. Decodable image previews are downscaled to fit within 96×96 pixels.
  Files without a decodable image preview use a compact file silhouette with
  an extension label and type color. A second request is rejected while one is
  queued or active.
- The `drag` crate canonicalizes paths and constructs the Windows Shell data
  object. Its allowed effect defaults to copy, so the original files are not
  moved by this feature. Queue/window creation failures are returned
  synchronously as `DragDropError::Platform`; errors from the later native
  session are logged from the dispatcher window procedure.
- Gallery owns path selection and passes the selected media paths (matching
  PhoTag: dragging a selected card exports the full selection; dragging an
  unselected card exports that card). It releases model borrows before
  queuing the drag.

## Events and failure inspection

- This service produces and consumes no Cordis events. Inspect
  `src/dispatch.rs` for deferred same-thread dispatch and transient-window
  cleanup, `DragDropError` for queue failures, and the pinned `drag` 2.1.1
  implementation for OLE/data-object behavior.
- The dependency is licensed `Apache-2.0 OR MIT`, pinned in `Cargo.toml` and
  `Cargo.lock`. It is used only for runtime drag sessions and introduces no
  persisted state or migration.

## Source mapping and implementation references

- `vendor/photag-source/src/drag.rs` contains PhoTag's original hand-written
  `IDataObject`/`IDropSource` and synchronous `DoDragDrop` call. Its caller in
  `vendor/photag-source/src/ui/gallery.rs` resolves selected IDs and starts
  the drag inside the `egui` card's `drag_started` block. It defines the
  payload and copy-only behavior, but that synchronous call site is not a safe
  GPUI template.
- GPUI `gpui-pre` 0.3.8 exposes `external_drag_payload` and promotes the
  payload to `PlatformWindow::start_external_drag`; its Windows platform
  implementation inherits the unsupported default. This feature therefore
  retains Gallery's existing `on_drag` gesture and supplies a Windows adapter
  without changing the UI framework.
- `drag-rs` documents Windows support and tests Tao, Winit, Wry and Tauri. Its
  Windows implementation initializes OLE, creates the Shell data object, and
  calls `DoDragDrop` synchronously. The message-only dispatcher supplies the
  event-loop timing that a standalone library cannot provide for GPUI.
- A separate Winit implementation keeps pending native drag data in its
  Windows event-loop state. This confirms that the drag session's event-loop
  integration is part of the platform adapter rather than only data-object
  construction.
- User confirmed cross-window Windows delivery. Long-hold behavior,
  cancellation, and visual confirmation of the updated drag image remain to be
  checked on Windows; compilation alone does not verify those interactions.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics: plugin startup and existing error/diagnostic output now
emit tracing records captured by photag.debug; instrumented public operations
report wall-clock span duration and errors. Domain service contracts, event
payloads and source operation ordering are unchanged. See
`photag-plugin-debug/PORTS.md` for capture scope and limits.

## Native theme paints (2026-10-19)

`start_file_drag(window, paths: &[PathBuf], style: DragImageStyle)` now requires
`DragImageStyle { background, foreground, border, accent: [u8;3] }` from the caller's
resolved native theme. Gallery supplies current UI Bridge/native roles. Fallback document
icons no longer contain a private pixel palette or file-format color table. Real image
preview pixels and OS behavior are unchanged; native OLE drag execution was not revalidated
by the theme migration. No UI-stack dependency was added to this low-level service.
