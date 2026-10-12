# PhoTag updater plugin ports

Status: the source updater's release check, numeric version comparison, staged
asset download, SHA-256 verification, executable swap with rollback, relaunch,
and startup cleanup have been ported into a Cordis feature crate. There is no
shell or settings UI integration yet, so users cannot reach these APIs through
the native app in this migration slice.

## Lifecycle and dependencies

- Cordis plugin: `photag.updater`; service: `photag/updater`
  (`UpdaterService`). `UpdaterPlugin` publishes the service and has no runtime
  plugin dependencies.
- `UpdaterService` exposes synchronous and background latest-release checks,
  background download with atomic byte progress, install, and startup cleanup.
- External dependencies: `ureq = "2"` (HTTP; resolves within major 2),
  `sha2 = "0.10"`, `dirs = 5` (workspace), `serde_json = 1` (workspace),
  `cordis-rs = 0.11.1` and `thiserror = 2` (workspace). The existing upstream
  PhoTag manifest also declares `ureq = "2"` and `sha2 = "0.10"`.
- The crate is not registered in the root workspace or host manifest by design
  for this slice. Host integration must add the member/dependency, register
  `UpdaterPlugin`, call `cleanup_previous()` during startup, and present the
  service operations from Settings with explicit user confirmation before
  download/install.

## Inputs, outputs, and behavior

- `check()` requests `OleksandrShabaldas/PhoTag` latest release over HTTPS,
  chooses `-windows-x64.exe` first (then any `.exe`), and returns `None` when
  the release is not newer than this crate's package version.
- `UpdateInfo` includes normalized version, notes, release page, executable
  asset metadata, and optional `SHA256SUMS.txt` URL. `notes_plain()` preserves
  the source's limited Markdown cleanup.
- `download()` stores a `.part` file under `%LOCALAPPDATA%/PhoTag/updates`,
  reports byte progress through `AtomicU64`, verifies the matching SHA-256
  entry when checksums are published, then promotes to `photag-<version>.exe`.
  A release with no checksum manifest remains accepted, matching the source.
- `install()` renames the running executable to `.old`, copies the staged
  executable into place, relaunches, and removes staged content. Copy failure
  attempts to restore the original binary. A relaunch failure leaves the new
  binary installed and reports that manual launch is needed.
- `cleanup_previous()` best-effort removes the `.old` binary and `.part`/`.exe`
  files in the update staging folder; invoke only after successful startup.
- Async operations return standard MPSC receivers. Dropping a receiver ignores
  the result but does not cancel the worker thread; the source implementation
  has no cancellation token.

## Integration and debugging

Compare implementation details against `vendor/photag-source/src/update.rs`
and Settings update flow in `vendor/photag-source/src/main.rs`
(`SettingsTab::Updates`, `start_update_check`, `show_update_offer`, and
`on_exit`). The legacy UI saves session state before installing. Host UI must
preserve that order and surface install failure via a durable log because the
source performs install during exit. This implementation uses the package's
`CARGO_PKG_VERSION`; the host must ensure its package version tracks the
application release version. Windows executable replacement and relaunch need
real Windows validation. No plugin-specific UI contribution is registered in
this crate because the current UI bridge has no updater-specific command/view
contract.
