# Drag-out plugin ports

Status: the Windows CF_HDROP/OLE drag service is registered in the workspace and shell startup graph. Gallery card drag-start calls it with the selected paths or the unselected card path. Windows OLE behavior remains unverified on a Windows desktop.

## Cordis identity and lifecycle

- Plugin: `photag.drag-drop` (`DragDropPlugin`, `DragDropConfig`).
- Service: `photag/drag-drop` (`DragDropService`). It has no plugin dependencies and publishes its service while active. Cordis owns the publication and removes it when the plugin stops; it registers no UI contributions or long-lived native resources.
- Shell includes this crate in the Cargo workspace and starts `DragDropPlugin` before Gallery. Gallery resolves the service during setup and calls it from the card drag gesture.

## Public operation

- `DragDropService::start_file_drag(&[PathBuf]) -> Result<bool, DragDropError>` is synchronous and must run on the UI thread in response to a pointer drag gesture. Empty input returns `Ok(false)`.
- Input paths are exposed as Unicode wide strings in a `CF_HDROP` `DROPFILES` block. The advertised allowed effect is copy only; the original files are never moved or modified by this service.
- `Ok(true)` means OLE returned `DRAGDROP_S_DROP`; `Ok(false)` means the operation was cancelled or the shell did not report a completed drop. Initialization/allocation failures and non-Windows use return typed errors.
- Gallery owns path selection and passes the selected media paths (matching PhoTag: dragging a selected card exports the full selection; dragging an unselected card exports that card). Do not hold database locks across the blocking OLE call.

## Events and failure inspection

- This service produces and consumes no Cordis events. It performs the shell interaction directly on the invoking UI thread.
- Inspect HRESULTs in `DragDropError`, then `src/win32.rs` for COM format negotiation, HGLOBAL construction, ref-counting, and OLE cleanup. The CF_HDROP payload is copied into each requested `STGMEDIUM`, as required by the data object ownership contract.

## Source mapping and remaining integration

- Source: `vendor/photag-source/src/drag.rs`; its only source caller is the gallery card drag-start handler in `vendor/photag-source/src/ui/gallery.rs` (around line 1666). It constructs paths from selected IDs and calls `start_file_drag` on the UI thread.
- This crate preserves that call's copy-only `DROPEFFECT_COPY` contract and selected-path payload. Gallery invokes the service from the GPUI card drag gesture. The migration is wired end to end; verify pointer-drag timing while the OLE modal loop is active and exercise Explorer, desktop, and a `CF_HDROP` consumer on Windows.
- Current implementation adds explicit OLE initialization error handling and balances successful `OleInitialize` calls with `OleUninitialize`. Shell application lifecycle and actual native drag delivery are not yet verified.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics: plugin startup and existing error/diagnostic output now emit tracing records captured by photag.debug; instrumented public operations report wall-clock span duration and errors. Domain service contracts, event payloads and source operation ordering are unchanged. See photag-plugin-debug/PORTS.md for capture scope and limits.
