# PhoTag Windows file drag-out refactor

## Reference behavior and diagnosis

PhoTag's pinned reference implementation is in
`vendor/photag-source/src/drag.rs` and
`vendor/photag-source/src/ui/gallery.rs`. It creates a Shell `CF_HDROP`
`IDataObject`, allows copy only, and starts `DoDragDrop` synchronously from the
Gallery card's `drag_started` callback. Gallery supplies every selected file
when the grabbed card is already selected; otherwise it supplies only the
grabbed file. This source is the behavioral contract for payload and selection.

The observed PhoTag crash dump resolves through `DoDragDrop` into the GPUI
Windows window procedure, whose panic boundary calls `std::process::abort`.
Calling OLE directly from the card callback leaves that callback in the window
message dispatch stack while `DoDragDrop` runs its nested message loop. A
previous `Window::defer` attempt still ran before the original dispatch had
unwound. A worker-thread experiment avoided that stack but failed to preserve
the active mouse gesture. A first library experiment called `drag-rs`
synchronously and suppressed HWND messages; that compiled but was not verified
as a working drag and is superseded by this refactor.

## Reference project and library research

- GPUI `gpui-pre` 0.3.8 has an `external_drag_payload` path, but its Windows
  `PlatformWindow` uses the default unsupported `can_start_external_drag` and
  `start_external_drag` methods.
- Zed's GPUI discussion documents that Windows external drag support is not
  exposed like other platform backends.
- `drag-rs` (CrabNebula, `Apache-2.0 OR MIT`) supports Windows and documents
  testing Tao, Winit, Wry, and Tauri. Its Windows implementation creates the
  Shell data object and calls `DoDragDrop` synchronously. It supplies the
  platform formats and drag image, but does not solve GPUI callback timing.
- Winit's Windows backend stores a pending drag in event-loop state; the
  framework owns when the platform drag starts. This reinforces that timing
  and message-loop integration must be handled by PhoTag's adapter.
- ZenDesktop documents a separate approach for native Windows drag behavior:
  mouse capture/`WM_MOUSEMOVE` for its own inter-fence movement and OLE for
  Explorer interoperability. It is a useful native-app reference, but not a
  GPUI integration example.
- Windows Shell documentation requires OLE drag/drop and describes the source
  data object, `IDropSource`, `DoDragDrop`, and the target callbacks.

## Refactor

- Gallery retains its existing GPUI card gesture and selected-path behavior.
- `DragDropService` snapshots the source window handle and selected paths,
  then posts a request to a temporary message-only window on the current UI
  thread. The card callback returns before Windows starts OLE.
- The message-only window starts `drag-rs` on the UI thread after the
  originating input callback has returned. It does not block, suppress, or
  subclass the GPUI source window procedure. The normal message pump remains
  available during OLE's modal loop.
- The dispatcher rejects a second drag while the first is queued or active,
  frees the request if posting fails, logs asynchronous native errors, and
  destroys its HWND after the drag returns.
- `drag-rs` remains pinned at 2.1.1. No persisted formats or media records
  change; the native effect is copy-only.

## Drag image sizing and files without previews (2026-10-19)

- The first selected path remains the preview source. If it decodes as an
  image, the drag bitmap is resized to fit within 96×96 pixels before it is
  handed to `drag-rs`; the exported file contents and paths are unchanged.
- If the file cannot be decoded as an image, the drag image becomes a compact
  file silhouette with an extension label and a type color (image, video,
  audio, PDF, or generic). This covers video, document, RAW, and unsupported
  image formats without relying on an installed preview handler.
- The reported `0x80040064` originates at GPUI's Windows drop-target helper
  `DragEnter` call (`gpui-pre-windows-0.3.8/src/window.rs` around line 1206).
  The helper logs an HRESULT from the `IDataObject` while a drag crosses a
  window. It is separate from our preview bitmap decoding, so the fallback
  image change should not be described as fixing that message. Whether it
  affects a target application's import needs separate reproduction.
- Rust 1.97.1 formatting, locked offline checks for drag-drop/Gallery/MCP, and
  the locked offline PhoTag Shell build passed. The user confirmed the prior
  drag implementation successfully starts a drag and transfers files to
  other windows. The new preview-size/fallback appearance still needs visual
  confirmation during an actual Windows drag; MCP synthetic pointer events do
  not expose the native drag image.

## User confirmation and follow-up status (2026-10-19)

The user confirmed the refactored drag starts normally and transfers files to
other windows. This supersedes the earlier pending external-delivery checks
above; the actual rendered size and fallback icon appearance are pending
confirmation after this visual change.

## Verification record

- `cargo +1.97.1 fmt --all -- --check` — passed.
- `cargo +1.97.1 check --locked --offline -j 2 -p photag-plugin-drag-drop
  -p photag-plugin-gallery -p photag-plugin-mcp` — passed.
- `cargo +1.97.1 build --locked --offline -j 2 -p photag-shell` — passed.
- `node scripts/check-ui-refresh.cjs`, `check-architecture.cjs`,
  `check-feature-ledger.cjs`, and `check-ui-theme.cjs` — passed. Architecture
  baseline includes the MCP diagnostic service edge; the checker still prints
  two pre-existing orphan-directory notices for playback and preview.
- Same-process PhoTag MCP launched the rebuilt shell with isolated profile
  `target/mcp-drag-rewrite` and `dev-test`. `get_ui_state` confirmed the two
  fixture images were loaded, the main window was visible and not hung, and
  `drag_drop` began at `idle`. Synthetic MCP `pointer_down` / `pointer_drag`
  left `drag_drop` at `idle`; that input did not enter the native file drag
  path. The window remained visible and responsive. This is not evidence of a
  successful operating-system drag.
- The initial MCP window and loaded fixture gallery are captured in
  [main-window-mcp.png](../../../.local/validation/artifacts/2026-10-19-drag-drop-crash/main-window-mcp.png) and
  [main-window-worker-thread.png](../../../.local/validation/artifacts/2026-10-19-drag-drop-crash/main-window-worker-thread.png).
- Explorer/desktop delivery, target-program import, long hold, cancellation,
  and native source-drag gesture remain unverified. The MCP pointer controls
  cannot establish a real Windows mouse-held OLE drag; the project requires
  same-process MCP for live debugging, so no generic OS automation was used.

## Upstream source links

- [GPUI Windows external drag discussion](https://github.com/zed-industries/zed/discussions/49684)
- [GPUI file manager noting external drag-out gap](https://github.com/situmorang-com/file-explorer)
- [CrabNebula drag-rs Windows implementation](https://github.com/crabnebula-dev/drag-rs/blob/v2/crates/drag/src/platform_impl/windows/mod.rs)
- [Winit Windows event loop](https://github.com/rust-windowing/winit/blob/master/winit-win32/src/event_loop.rs)
- [ZenDesktop Windows drag architecture](https://github.com/jaimitus/ZenDesktop)
- [Microsoft Shell drag-and-drop guide](https://github.com/MicrosoftDocs/win32/blob/docs/desktop-src/shell/dragdrop.md)
