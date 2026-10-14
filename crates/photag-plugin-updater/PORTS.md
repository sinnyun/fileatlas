# PhoTag updater plugin ports

Status: the source updater's release check, numeric version comparison, staged
asset download, SHA-256 verification, executable swap with rollback, relaunch,
and startup cleanup are exposed by a Cordis feature crate. Shell registration
and the Settings-owned UI for checking, automatic release offer, downloading,
progress, and install are wired. Windows executable replacement and relaunch
remain unverified.

## Lifecycle and dependencies

- Cordis plugin: `photag.updater`; service: `photag/updater`
  (`UpdaterService`). `UpdaterPlugin` publishes the service and has no runtime
  plugin dependencies.
- `UpdaterService` exposes synchronous and background latest-release checks,
  background download with atomic byte progress, direct install, shell-scheduled
  install (`schedule_install`/`take_pending_install`), and startup cleanup.
- External dependencies: `ureq = "2"` (HTTP; resolves within major 2),
  `sha2 = "0.10"`, `dirs = 5` (workspace), `serde_json = 1` (workspace),
  `cordis-rs = 0.11.1` and `thiserror = 2` (workspace). The existing upstream
  PhoTag manifest also declares `ureq = "2"` and `sha2 = "0.10"`.
- The shell starts the updater plugin before Settings. Applying the updater
  plugin runs best-effort startup cleanup and publishes the service. Settings
  receives that service through Cordis and owns the update UI, including the
  source-style floating offer and session-scoped “Not now” dismissal.

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
`on_exit`). The legacy UI saves session state before installing and installs
during exit. The current GPUI Settings action uses an explicit two-step
confirmation and flushes the latest `SessionService` snapshot, then schedules
the staged path and quits the GPUI application. After `application().run`
returns, the shell installs the update, so the prior process exits before the
executable is replaced. Install failures are written to
`%LOCALAPPDATA%/PhoTag/update-error.txt`. This implementation uses the package's
`CARGO_PKG_VERSION`; the host package version must track the application
release version. Windows replacement and relaunch need real Windows validation.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics: plugin startup and existing error/diagnostic output now emit tracing records captured by photag.debug; instrumented public operations report wall-clock span duration and errors. Domain service contracts, event payloads and source operation ordering are unchanged. See photag-plugin-debug/PORTS.md for capture scope and limits.
