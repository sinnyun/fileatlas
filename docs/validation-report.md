## 2026-10-18 本地收藏取代置顶

已实现持久收藏、普通删除/清理/淘汰保护、编辑去重保留收藏、旧 pinned 数据兼容，以及独立确认删除收藏。12 项数据库测试、shell check、独立 exe 编译、三次独立进程 MCP 收藏/重启/取消/删除验证及原面板 91 步回归通过。标准 exe 正在运行被锁定，未替换用户窗口；使用 `target/debug/fileatlas-clipboard-favorites.exe` 完成验证。截图、准确命令、进程证据和限制见 [收藏验证](validation-ui/2026-10-18-clipboard-favorites/README.md)。

# Validation report

## 2026-10-18 Clipboard groups and browser-store persistence

The clipboard workbench is user-requested new functionality with no history/group counterpart in the pinned photo-only source. Its backend remains behind DatabaseService. Inspection traced the production path `ClipboardService -> DatabaseService::with_writer -> browser_store::persist`: the old persistence whitelist omitted clipboard categories/history, despite creating them in memory. Added the two existing tables to the durable-store sync and deletion reconciliation; group deletion clears references transactionally and keeps text. No existing media/tag schema or source query semantics changed.

A browser-mode regression first failed with zero groups after reopen, then passed after the fix. All 11 database tests pass, including recapture/group identity, rename, reopen, deletion-to-unclassified and deleted-row restart checks; the text-kind regression also passes. Three independent MCP child processes verify creation, real-process restoration and durable removal through the same production browser service. See [v2 UI/runtime evidence](validation-ui/2026-10-18-clipboard-workbench-v2/README.md) for the 91-step GPUI flow, explicit isolation correction, commands, screenshots and unverified OS clipboard/hotkey paths.

## 2026-10-17 Everything search adapter

Added the official Everything SDK 3 IPC adapter as an optional Cordis provider.
FileManager snapshot and page queries use indexed-file candidates for registered
FileAtlas roots; database substring scopes and other FileAtlas filters remain in
SQLite. MCP `search_media` already routes through FileManagerService, so it shares
the provider and fallback behavior. Search can prepare an already installed
background client. Software checking and installation links remain read-only.
The earlier Session 0 version diagnosis was incorrect: PID 38460 was the
Everything.exe -svc indexing service, not an SDK search client. Starting the
existing 1.5.0.1423 search client made real SDK search work. Browser mode now
materializes actual hits in session-only rows without recursive scanning or
persisting unannotated file inventories. Search/version prompts, exact commands and
remaining runtime boundaries are in `validation-ui/2026-10-17-everything-search/README.md`.

## 2026-10-16 user-directed QuickLook viewer replacement

Original-file viewing now calls the user's installed QuickLook through a
Cordis-owned service. The embedded Preview and MediaPlayback crates and their
controls are removed, per explicit user direction. Upstream PhoTag behavior was
traced in `src/app.rs`, `src/ui/preview.rs` and `src/video.rs`; thumbnail extraction
in `src/thumbs.rs`/`src/video.rs` remains mapped to the existing thumbnail/indexing
services. No SQLite schema, source-file mutation order or persisted JSON migration
was introduced. Inert old viewer settings/session values are retained for compatibility.
The mapping is updated in `source-feature-audit.md`; this supersedes earlier
embedded-viewer completeness claims. Build, regression and same-process MCP
evidence, including unverified external-app boundaries, are recorded in
`validation-ui/2026-10-16-quicklook/README.md`.

**Phase status:** migration-phase record. Interface work from 2026-10-13 onward
is validated against the redesign references and recorded in
[`redesign-validation.md`](redesign-validation.md); source-parity statements
below describe the migration phase, not the current UI target.

## Current status index (2026-10-13)

The latest recorded consolidation and source-parity pass reports
`cargo +1.97.1 fmt --all -- --check`,
`$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2`, and `git diff --check` passed. The prescribed shell check compiles the host dependency graph; it does not establish full plugin behavior, Windows runtime behavior, or complete source parity. Tests were not run in that final pass under the then-applicable implementation-pass instructions. Earlier sections below are dated snapshots and may describe features as absent or checks as blocked before later work; use the latest entry and [`source-feature-audit.md`](source-feature-audit.md) for current migration status.

Outstanding evidence includes executable regression specifications for source
menu/confirmation behavior, settings defaults, database query semantics and
cascades, file-operation rollback, watcher reconciliation, preview toolbar
configuration, and control-level visual behavior. Windows UI/system
integration, representative media coverage, updater replacement/restart, and
real FileAtlas library compatibility also require their respective environments
and fixtures.

## Source-layout specification and first file_manager geometry pass (2026-10-13)

`docs/reference-ui/source-layout-spec.md` was assembled from read-only audits
of the pinned `vendor/photag-source/` renderers, theme, settings model, and
dialogs. Source screenshots remain stored in
`docs/reference-ui/2026-10-13/`; they were inspected only after source values
were recorded. The codebase-memory MCP transport was unavailable during this
pass, so the source fallback was direct file reading. AGENTS.md now requires
source file/line references and prohibits deriving layout values from pixels.

The first FileManager geometry pass changes card hit areas from metadata cards to
source-sized square thumbnail surfaces with a 7-point selection inset, source
corner radius, 12-point row gap, and 40-point month header. Column count now
uses the measured FileManager viewport when available, with the validated session
sidebar width used for the first layout pass. References:
`vendor/photag-source/src/ui/gallery.rs:990-1005,1383-1387,1406-1429` and
`docs/reference-ui/source-layout-spec.md#file_manager-card`. This remains partial:
filter controls still sit inside the FileManager pane instead of the full-width
source command bar, several controls retain migration-specific layouts, and
their GPUI rendering has not reached visual parity.

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 fmt --all -- --check` | Passed | Rust workspace formatting. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Passed | Includes the changed FileManager crate and shell host graph. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 build -p fileatlas-shell --locked -j 2` | Passed | Rebuilt the executable used by the live MCP round trip. |
| `node target/live-ui-roundtrip.cjs` with `FILEATLAS_BIN=target/debug/fileatlas-shell.exe`, screenshot directory `target/validation-screenshots`, `FILEATLAS_DEV_PLUGINS=mcp,dev-test`, isolated `FILEATLAS_DEV_TEST_DIR=target/dev-test`, and `FILEATLAS_MCP_ALLOW_MUTATIONS=1` | Passed after setting required environment | First invocation without `FILEATLAS_BIN` failed before launch. The configured run initialized the local MCP child, read library/media state, closed/opened Preview, toggled Info, set a four-star rating and assigned the isolated `ValidationTag`; screenshot capture returned 1942×1256 images. The generated source-fixture XMP sidecar was removed after the run; profile database/cache stay under `target/dev-test`. |
| Visual comparison | Partial / gaps recorded | Square FileManager tiles improved; source header/filter ownership, controls, native/dark button treatment, navigation rows, details overlays, and other feature panels still need source-driven region passes. Source reference captures are retained under `docs/reference-ui/2026-10-13/`. |

## Historical snapshot: preview, theme, and navigation integration pass (2026-10-13)

| Check | Result | Notes |
|---|---|---|
| `cargo fmt --all -- --check` | Passed with Rust 1.97.1 | Invoked as `cargo +1.97.1 fmt --all -- --check`; the default toolchain is older than the workspace MSRV. |
| `cargo check -p fileatlas-shell --locked` | Passed with Rust 1.97.1 | Invoked as `cargo +1.97.1 check -p fileatlas-shell --locked`, with `CARGO_INCREMENTAL=1` and `CARGO_BUILD_JOBS=2`; shell dependency graph compiled after adding the preview crate to workspace and lockfile. |
| Default-toolchain `cargo check -p fileatlas-shell --locked` | Blocked before compilation | Active rustc 1.94.1 is below workspace MSRV 1.95. |
| Plugin/workspace tests | Not run | Reserved for the user's second-round concentrated debugging as directed by `AGENTS.md`. |
| Windows appearance and media UI | Unverified | Linux/host compilation does not validate HWND effects, Windows shell behavior, real media decoding, or visual parity. |

This pass adds a Cordis image preview service and GPUI surface, wires FileManager
Preview/Enter to selected images, applies saved accent state, adds Windows
appearance APIs and shell HWND integration, and persists navigation
folder/path changes. Zoom controls currently update preview state but do not
scale the rendered image. AI engine code exists in an unregistered crate draft;
its new ONNX/HTTP dependencies are not in the locked workspace and no runtime
integration is claimed.

## Historical snapshot: parallel feature migration pass (2026-10-13)

| Check | Result | Notes |
|---|---|---|
| `cargo fmt --all -- --check` | Passed with Rust 1.97.1 | Invoked as `cargo +1.97.1 fmt --all -- --check` after formatting all workspace Rust sources. |
| `cargo check -p fileatlas-shell --locked` | Passed with Rust 1.97.1 | Invoked as `cargo +1.97.1 check -p fileatlas-shell --locked`, `CARGO_INCREMENTAL=1`, `CARGO_BUILD_JOBS=2`; includes the registered history, media-playback, and drag-drop crates and FileManager bulk actions/settings key capture. |
| Plugin/workspace tests | Not run | Reserved for the user's second-round concentrated debugging as directed by `AGENTS.md`. |
| Windows runtime behavior | Unverified | OLE drag-out, playback process behavior, native dialogs, visual parity, and filesystem interactions still need real Windows validation. |

This pass adds source-derived filter-driven bulk actions, a persisted settings
open-key capture control, a Cordis undo/redo handler service, an FFmpeg video
playback service/view, and a Windows CF_HDROP drag-out service. History mutation
callers and keyboard dispatch, the configured open-key consumer, FileManager video
handoff, and the FileManager drag gesture remain unwired. Audio output is not
migrated. The AI and updater crate drafts remain outside the workspace because
their new external dependencies are not represented in the pinned lockfile;
they are not shell features yet.

## Historical snapshot: session restore migration pass (2026-10-12)

| Check | Result | Notes |
|---|---|---|
| `cargo fmt --all -- --check` | Passed | Workspace formatting check after file_manager and navigation session wiring. |
| Initial `cargo check -p fileatlas-shell --locked -j 2` | Failed before compilation | Cargo required lockfile updates for the new local file_manager/navigation dependencies; their workspace package edges were added to `Cargo.lock`. |
| `cargo check -p fileatlas-shell --locked -j 2` after lock update | Blocked by toolchain | Active `rustc 1.94.1` is below workspace MSRV 1.95. |
| `cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Passed | Windows x86_64 MSVC host; compiled shell dependency graph. The first compile exposed and fixed the Windows `Command::creation_flags` mutable binding in indexing. Existing warnings remain in session/settings publication handling and metadata-export import. |
| Plugin/workspace tests | Deferred | Per repository verification instructions, tests remain reserved for the user's second-round concentrated debugging. |

The session migration restores/persists file_manager query filters and restores
validated navigation selection and directory expansion. This compile does not
verify behavior in a running Windows GUI or against an actual FileAtlas library.

## Historical snapshot: cloud task pass (2026-10-12)

This pass is in code-generation/documentation mode. Per the current task scope,
plugin tests are deferred to the second-round concentrated debugging phase.
The active interactive tool environment has no `rustc`, `cargo`, or `rustfmt`
on `PATH`. GitHub Actions run 20 on commit
`49771e11c9cd5f9cd056284cf5266fbd7eaf8d39` passed formatting and
`cargo check -p fileatlas-shell --locked`. That build compiles the configured
Cordis plugins as Shell dependencies, including the file-operation and tagging
crates. Plugin tests remain deferred; this Linux compile does not validate
Windows runtime behavior or feature parity.

GitHub Actions run 24 passed formatting and `cargo check -p fileatlas-shell
--locked` for the selected-media details implementation. The new EXIF parser
and lazy field display have now passed formatting and `cargo check -p
fileatlas-shell --locked` in run 27. Plugin tests remain deferred; Windows runtime
behavior, ffprobe video details and the preview info toggle remain unfinished.

The navigation/settings/export and selected-file action batch was committed as
`0a0f32d7367e311de7b84003d57c74597e435df8`. GitHub Actions run 30 stopped at
formatting before compiling; its rustfmt output has been applied to the Rust
sources. The formatting-correction commit is awaiting a fresh base-shell check.
Run 31 on formatting commit `df9889e1517ce9bff4bc2311b4a483e4fc99284b`
found one remaining rustfmt difference at the end of the settings view; it has
been applied. Run 32 on `fa1312eede8f02c701b3d647c2a16145a87a65e8` passed
formatting but compilation found missing `StyledExt` import and a reusable
settings callback issue, plus a borrowed EXIF error string escaping the render
scope. Those compiler findings are fixed. Run 33 on
`5a3bc105d1bef1f514cbb708ee3d7a6ff64c50df` passed formatting and reported one
borrowed settings section label; it now requires a static string. A fresh
check is pending. The run also reported a pre-existing ignored publication
guard, now bound explicitly.
Run 34 on `e7376885c903b511a328620f127f63219ddf80d6` passed formatting; the
compiler found that GPUI input mutations require the render window outside the
entity-update closure. The tag editor and parent selector now forward that
window and use the current two-argument update API. Run 35 on
`631a1fb3b0962b429ced77910f8132fbaaecf5a1` passed both formatting and
`cargo check -p fileatlas-shell --locked`. Plugin tests remain deferred by task
direction; Windows UI, filesystem, Shell, drag/drop, and Recycle Bin behavior
remain unverified.

| Check | Result | Notes |
|---|---|---|
| Rust toolchain availability | Blocked | `command -v cargo` and `rustc --version`; neither executable is installed in this active environment. |
| `cargo fmt --all -- --check` | Not run | Requires rustfmt; no Rust toolchain is installed. |
| Local `cargo check -p fileatlas-shell --locked` | Not run | No local Rust toolchain in the active interactive environment. |
| GitHub Actions: latest base shell check | Passed | Commit `49771e11c9cd5f9cd056284cf5266fbd7eaf8d39`, run 20; formatting and `cargo check -p fileatlas-shell --locked` both passed. Plugins are compiled as dependencies, not tested. |
| GitHub Actions: selected-media detail migration | Formatting failed; compile skipped | Commit `976851250f7d276141bb4db87583c38564a252a9`, run 21; three rustfmt diffs in `view/card.rs` were corrected locally. A fresh CI run is pending. |
| GitHub Actions: selected-media detail formatting correction | Formatting passed; compile failed | Commit `050328d1be3e4c4528c0c29fec3c813815886e7f`, run 22; `open_count` reader closure needed an `Ok(...)` wrapper. The compiler-reported fix is applied locally; rerun pending. |
| GitHub Actions: open-count reader fix | Formatting failed; compile skipped | Commit `3bae0c2005d9cc700a636dee9b5569217099b9c0`, run 23; rustfmt requested line wrapping for the new `Ok(...)`. Corrected locally; rerun pending. |
| GitHub Actions: selected-media metadata detail | Passed | Commit `34a9386ec8ee1a4a249a745987f8066a5c253451`, run 24; formatting and `cargo check -p fileatlas-shell --locked` passed. Plugin tests were not run. |
| GitHub Actions: lazy EXIF field display | Formatting failed; compile skipped | Commit `2a929d79a8c68aa42bfb564533389a8932335ef8`, run 26; one line-wrap difference in the error display arm is corrected locally. The EXIF dependency and lockfile have not yet reached compilation. |
| GitHub Actions: lazy EXIF field display correction | Passed | Commit `490b9dfee985831ea2f67e3b8f8616c29affd692`, run 27; formatting and `cargo check -p fileatlas-shell --locked` passed with the new EXIF dependency. Plugin tests were not run. |
| GitHub Actions: latest shell check | Failed at formatting step | Commit `54f266f99866fd351aff602d1561751106181eec`; rustfmt requested standard multiline formatting in the copied settings/session models and two shell calls. Those reported changes are now applied locally; compilation was skipped until formatting passes. |
| GitHub Actions: shell check after formatting corrections | Not created | The formatting correction is published at `44b5199cbb3fd11d2b3fbc631a76c29bbd589525`, but GitHub reports no workflow run or commit status for it. |
| GitHub Actions: file-operation/XMP migration check | Failed at formatting step | Commit `4f893eb2dbce0768eb4ade07aaca4a539fb04313`; rustfmt reported formatting in file operations, tagging, and reused XMP source, plus three settings field attributes. Base shell compilation was skipped. Corrections are being applied from the logged rustfmt diff; the next push will rerun formatting and the base shell compile. |
| GitHub Actions: formatting correction and bulk-copy check | Failed at formatting step | Commit `8be10866d175a5e5cda46892572eacc3593193bc`; only the new bulk-copy `std::fs::copy` call remained different from rustfmt output. Base shell compilation was skipped; that formatting correction is now applied locally. |
| GitHub Actions: corrected bulk-copy commit | Cancelled | Commit `083162fd54226bb47b7172dc841f10c34ab81c72`; run 13 was cancelled, and a duplicate run for the prior `8be10866d175a5e5cda46892572eacc3593193bc` still reported the old formatting diff. No current shell compile result exists yet. |
| GitHub Actions: formatting and base shell compile | Formatting passed; compile failed | Commit `fae2b05126256324111e67128d5bb853c457127f`; Cargo could not resolve `trash 5.2.5`'s lock reference to `chrono 0.4.44` alongside the workspace's `chrono 0.4.45`. The lockfile reference was updated to the compatible 0.4.45 package and re-parsed locally; compilation remains to be rerun. |
| GitHub Actions: first lockfile conflict correction | Formatting passed; compile failed | Commit `22d245eb7042c2f66b64d4724f0630abcaf735b3`; Cargo then exposed the same stale-lock conflict for `libc` 0.2.184 vs 0.2.190. The `trash` references to `libc`, `log`, and `objc2` are now aligned to compatible versions already selected by the workspace; compilation remains to be rerun. |
| GitHub Actions: second lockfile conflict correction | Formatting passed; compile failed | Commit `aa2df38b92663bed3f0ed8fc9e54a04e21f38fcd`; Cargo exposed stale `iana-time-zone` references to older `js-sys`, `wasm-bindgen`, `log`, and Android system-properties packages. These references now match the workspace's compatible package selections; compilation remains to be rerun. |
| GitHub Actions: compatibility-line lock normalization | Formatting passed; compile failed | Commit `bae09af5b434f0712747b9a38988eac931cf6b05`; Cargo next reported `cfg-if` 1.0.4 vs 1.0.5. The lockfile now aligns every dependency edge to the newest already-locked version on its SemVer compatibility line, removes 47 unreachable package records, and passes TOML/reference checks locally. The basic shell compile must rerun before any build claim. |
| GitHub Actions: normalized lock and base shell compilation | Formatting passed; compile reached source and failed | Commit `d3afbec3b634d2b6a9c9ec63e8875632b3fbd642`; lock resolution succeeded. Compiler reported ambiguous result inference in `open_external` and an extra `?` in the media query helper. Both source fixes are applied locally; a fresh base shell compile is pending. |
| `git diff --check` | Passed | No whitespace errors in the current uncommitted migration changes. |
| Cargo lock dependency-reference audit | Passed | TOML parsed; workspace dependency references resolve uniquely in the edited lockfile. |
| Prior GitHub Actions: `cargo check -p fileatlas-shell --locked` | Passed | Remote commit `0940e41024d021f7883be387f9400469d20bff7d`; validates only that earlier tree. |
| Plugin tests | Deferred | Per task direction, run during second-round concentrated testing. |
| Windows GUI / Shell / file operations | Deferred | Requires the user's Windows environment. |

The requested cloud workflow uses incremental compilation, two build jobs, and
no dev debug symbols. Plugin tests remain deferred. Earlier failed CI attempts
are retained in the table above; run 20 is the latest check and passes the
formatting and base shell compilation steps.

---

## Cloud revalidation (2026-10-12)

| Check | Result | Notes |
|---|---|---|
| Rust toolchain | Passed | Rust stable 1.99.0 and rustfmt installed for this session |
| `cargo fmt --all -- --check` | Passed | After formatting the file_manager active-filter row |
| `git diff --check` | Passed | No whitespace errors |
| `cargo test -p fileatlas-plugin-file-manager --locked --offline` | Passed, 13 tests | Includes per-extension clearing from both include/exclude filter states |
| `cargo test --workspace --exclude fileatlas-shell --locked --offline` | Passed, 28 tests | Linux x86_64; includes database, file_manager, indexing, navigation, tagging, thumbnails, watcher, and UI bridge tests |
| `cargo check -p fileatlas-shell --locked --offline` | Passed | Includes GPUI Shell and file_manager view compilation |
| `cargo +1.95.0 check -p fileatlas-shell --locked --offline` | Passed | Confirms declared minimum Rust toolchain compiles the shell |
| `cargo +1.95.0 test --workspace --exclude fileatlas-shell --locked --offline` | Passed, 28 tests | Confirms plugin tests run on the declared minimum Rust toolchain |

The checked-in `Cargo.lock` at starting commit `29d3343` began with a Cargo
truncation warning and could not be parsed. It was regenerated from the
workspace manifests with Cargo 1.99.0 before these checks. A separate
`cargo +1.88.0 check -p fileatlas-shell --locked --offline` failed because the
resolved `cosmic-text 0.19.0`, `oo7 0.6.0`, `serde-saphyr 1.3.0`, and
`smol_str 0.3.6` require Rust 1.89 or newer (and `oo7` requires 1.92). The
Rust 1.92 failed in `gpui-pre-util 0.3.8` on `<[T]>::as_array`; Rust 1.93
then failed on `core::hint::cold_path` in `gpui-pre 0.3.8`. The latter API
stabilized in Rust 1.95, so the workspace MSRV is declared as 1.95 and both
the shell check and plugin tests pass on that toolchain; see the
[official Rust release notes](https://doc.rust-lang.org/stable/releases.html).
Initial Linux linking also identified missing `libfontconfig1-dev` and
`libxkbcommon-x11-dev`; both were installed for this validation run and added
to the cloud environment guide.

## Environment

- Cloud host: Linux x86_64, 2026-10-12.
- Rust stable: `rustc 1.99.0`.
- The target application is Windows-only in material areas (native shell,
  drag-out, Recycle Bin, Mica/Acrylic); no Windows GUI is available here.

## Results

| Check | Result | Notes |
|---|---|---|
| FileAtlas clone and pinned revision | Passed | `36cee242a78c0a8cd9ac6b0d63b64265aa0add90` |
| `cargo check -p fileatlas-shell --offline` | Passed | Linux x86_64; compiles the GPUI Kit host and Cordis runtime composition |
| `cargo fmt --all` | Passed | Stable toolchain with rustfmt |
| `cargo test --workspace --exclude fileatlas-shell --offline` | Passed, 22 tests | 3 database, 7 file_manager data/filter/selection/lifecycle, 4 navigation, 1 tagging, 1 indexing, 1 thumbnails, 2 watcher, and 3 UI bridge tests |
| `cargo check -p fileatlas-shell --offline` | Passed | Includes GPUI view factories, slot mounting, and all registered runtime plugins |
| `cargo check -p fileatlas-plugin-file-manager --offline` | Passed | Includes GPUI cached-thumbnail image element integration |
| `cargo test --workspace --offline` | Failed at GPUI archive creation | Rust reports `failed to map object file: memory map must have a non-zero length` when creating a `gpui_component` rlib after the thumbnail dependency was added; package tests and shell check pass separately |
| `cargo test -p fileatlas-plugin-database -p fileatlas-ui-bridge --offline` | Passed, 6 tests | Fresh schema, pre-migration library compatibility, Cordis database lifecycle, and UI owner cleanup tests |
| `cargo test -p fileatlas-plugin-thumbnails --offline` | Passed, 1 test | Original image pipeline creates and reuses a cached thumbnail |
| `cargo test -p fileatlas-plugin-indexing --offline` | Passed, 1 test | Recursive folder scan indexes a still image, ignores a hidden file, pairs a video poster and writes thumbnail cache output |
| `cargo test -p fileatlas-plugin-watcher --offline` | Passed, 2 tests | Extension classification and Linux filesystem create/remove events reconcile to SQLite and dispatch Cordis media-index change events; Windows-specific notification behavior remains untested |
| `cargo test -p fileatlas-plugin-file-manager --offline` | Passed, 12 tests | Confirms cached-window shifts, 200-row page loading, UI lifecycle cleanup including Cordis folder/tag/path selection and index events, source-equivalent filter mapping, search-scope toggling, date input parsing, inclusive rating-range bounds, include/exclude extension cycling, pointer plain/Ctrl/Shift selection, keyboard navigation/select-all/rating/delete state effects and persisted rating clamping; GPUI media-type, sort, rating, date, search-scope, extension, clear-filter and selected-item detail rendering compile through shell check |
| `cargo test -p fileatlas-plugin-navigation --offline` | Passed, 4 tests | Folder/tag counts, source tag mutations, folder add/rescan/remove scan lifecycle, and sidebar contribution cleanup; dot-directory filtering and tag color format covered |
| `cargo test -p fileatlas-plugin-tagging --offline` | Passed, 1 test | Assignment expands associations and ancestors with source manual/automatic flags; remove and clear paths covered |
| Git bundle clone | Passed | Cloned delivery bundle to a clean directory and inspected its root commit |
| GPUI/GPUI Kit integration compile | Passed on Linux | Windows native GUI execution is not available here |
| FileAtlas regression tests | Partial | No end-to-end UI parity test suite; existing tests cover plugin lifecycle and basic DB setup |
| Windows GUI / Shell / file operation tests | Not run | Requires the user's Windows environment |

## Outstanding validation issues

- The file_manager test covers synthetic database rows only and does not render GPUI. The scanner and folder-action tests cover synthetic fixtures only. The watcher tests cover classification and Linux native create/remove notifications, but do not exercise Windows notification delivery. Additional regression tests are needed for large batches, supported media formats, FFmpeg video extraction, real filesystem events, rename/delete reconciliation, and error handling.
- Windows GUI, Windows Shell, drag/drop, Recycle Bin, and real-file operations
  remain unverified. A synthetic pre-migration SQLite fixture passes; a copy of
  an actual FileAtlas user library has not been tested.
- The first Linux workspace test attempt failed to link because `libxcb` and
  `libxkbcommon` development files were absent. After installing those system
  packages, the complete workspace test command passed.
- After the thumbnail crate added image decoding dependencies, a fresh full
  workspace test attempt hit the GPUI archive mapping error listed above.
  `cargo test --workspace --exclude fileatlas-shell --offline` and
  `cargo check -p fileatlas-shell --offline` both pass independently.

This paragraph records the validation report's initial baseline assessment,
before the later feature migrations documented below. It is superseded as a
current feature inventory by the 2026-10-13 source-parity/consolidation entry
and `docs/source-feature-audit.md`. The underlying caution still applies:
successful compilation is not evidence of complete parity, and Windows
behavior plus real user-library compatibility need dedicated validation.

## Windows build performance baseline (2026-10-12)

This optimization pass measured the existing configuration before changing
build behavior. No tests were run.

| Command | Result | Notes |
|---|---|---|
| `cargo fmt --all -- --check` | Passed | Installed rustfmt; no formatting differences. |
| `cargo check -p fileatlas-shell --locked --timings` | Not run to compilation | Default `stable` is Rust 1.94.1; Cargo rejected all workspace crates because they declare Rust 1.95. |
| `cargo +1.96.0 fmt --all -- --check` | Passed | Windows MSVC host toolchain. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; $env:CARGO_PROFILE_DEV_DEBUG='0'; cargo +1.96.0 check -p fileatlas-shell --locked --timings` | Passed, 2m 54s | Windows MSVC, first complete dependency build for this toolchain; 681 dirty units, GPUI and registered Shell plugin dependency graph. Timing report: `target/cargo-timings/cargo-timing-20261012T155111622Z-0de67308caf8ada2.html`. |
| Same `cargo +1.96.0 check` with unchanged sources | Passed, 1.26s Cargo / 1.42s wall time | Warm-check baseline; timing report also written under `target/cargo-timings/`. |
| Final executable link timing | Not measured | This pass followed the permitted `cargo check` verification; no `cargo build` was run. |

The build emitted warnings for unused `ServicePublication` values in session
and settings, and an unused `FluentBuilder` import in metadata-export. These
warnings did not fail the check and are recorded for follow-up in the relevant
code changes. The measured configuration already has dev incremental
compilation, no dev debug info, bounded parallelism, and CI artifact caching;
the data does not yet justify changing the linker or replacing the CI cache.

The longest unit in the cold timing report was `windows 0.62.2 (check)` at
39.43 seconds, followed by `gpui-pre 0.3.8` at 9.58 seconds and
`gpui-component 0.7.2` at 8.76 seconds. `cargo tree -e features -i
windows@0.62.2 -p fileatlas-shell --locked` traces the Windows API feature set to
the GPUI Windows backend and GPUI Kit dependency path. Application-level
feature trimming is not established as safe; validate upstream feature
requirements and Windows UI coverage before attempting it.

## Windows development launcher and staged plugin startup (2026-10-13)

`start-dev.bat [plugin-list]` checks `fileatlas-shell`, then runs the native GPUI development window. It takes no port argument because this desktop application has no development HTTP port.

The shell enables `settings`, `session`, and `database` by default, with the UI bridge published by the host. `FILEATLAS_DEV_PLUGINS` defaults to `core`. Pass a comma-separated plugin name as the optional argument (or set the environment variable) to enable plugins incrementally. `file_manager` brings in file operations; `navigation` brings in thumbnails, indexing, and watcher; `indexing` brings in thumbnails; `watcher` brings in indexing and thumbnails. `all` enables all feature plugins. Supported names: `file_manager`, `navigation`, `metadata-export`, `file-operations`, `thumbnails`, `indexing`, `watcher`, `tagging`, `history`, `media-playback`, and `drag-drop`. Preview starts with FileManager; Settings and Session start in core.

| Check | Result | Notes |
|---|---|---|
| `cargo fmt --all -- --check` | Passed | Windows MSVC toolchain 1.97.1. |
| `cargo check -p fileatlas-shell --locked -j 2` | Passed | Windows MSVC; incremental shell dependency graph. Existing warnings: session `ServicePublication` and metadata-export unused import. |
| `start-dev.bat` | Started | No port prompt; shell compile check passed; current `fileatlas-shell.exe` is responsive with a nonzero main-window handle. CUA did not expose native apps for visual inspection. |
| Plugin tests | Deferred | Per repository verification instructions. |

To progressively enable a feature, pass its name, for example `start-dev.bat file_manager` or `start-dev.bat navigation`.

Some initial runs exited with a GPUI debug panic (`Duplicate a11y node id`) during first render; this can recur and is still unresolved. Other direct and batch-file runs remained responsive with a nonzero window handle, including the current no-argument batch-file launch. Capture a full backtrace and isolate the view as plugins are enabled incrementally.

## Parallel migration follow-up (2026-10-13)

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 fmt --all -- --check` | Passed | Windows MSVC toolchain. |
| `cargo +1.97.1 check -p fileatlas-shell --locked` | Failed during lock resolution | Newly added native dialogs and EXIF/date dependencies bring a transitive graph that conflicts with pinned versions (currently `futures-util` 0.3.32 vs 0.3.34). No Rust compilation was reached. Lock graph requires a coherent regeneration/reconciliation before this migration is validated. |
| Tests | Not run | Repository instructions reserve tests for the concentrated second-round debugging pass. |
| Windows UI/native dialogs/OLE | Unverified | Requires real Windows runtime validation. |

## Lockfile reconciliation and focused second-round debugging (2026-10-13)

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 update` | Passed | Reconciled the workspace lock graph with current manifests and the workspace Rust 1.95 MSRV. Removed orphaned lock records from the imported dialog dependency closure and unified duplicate transitive versions. |
| `cargo +1.97.1 fmt --all -- --check` | Passed | Windows MSVC toolchain. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked` | Passed | Windows MSVC; full shell dependency graph compiled after fixing missing GPUI traits, the settings navigation weak-reference type, metadata path input conversion, and GPUI click-count access. |
| Focused tests for preview, settings, file_manager, navigation, metadata-export, history, media-playback, and drag-drop | Passed on rerun | 17 unit tests passed (13 FileManager, 4 Navigation); remaining selected crates have no unit tests. The initial Navigation lifecycle test exposed a missing SessionService test fixture, which was added before rerunning. All selected doc-tests passed (zero defined). |
| `FILEATLAS_DEV_PLUGINS=file_manager cargo +1.97.1 run -p fileatlas-shell --locked` | Passed twice after UI ID fix | After moving FileManager after its providers and prefixing Preview/MediaPlayback button IDs, two consecutive starts kept a responsive main window with a nonzero HWND for over 40 seconds each and closed cleanly. Earlier launches reproduced a duplicate `Close` accessibility node and `0xc0000409`; plugin-specific button IDs removed that failure in the two follow-up launches. Test data used a temporary `LOCALAPPDATA` directory. |
| Windows UI/OLE/dialog/watcher interactions | Partially verified | Native UI automation is unavailable in this environment. The FileManager window starts and responds, but visual inspection, Explorer drag, native dialog interaction, and filesystem notification exercise were not performed. |

The shell provider-order audit found that FileManager was started before MediaPlayback, Tagging, and History despite requiring or optionally consuming those services. Startup now starts FileManager after its providers, and shutdown stops FileManager and History before those dependencies. The duplicate accessibility node came from both Preview and MediaPlayback using the same `Close` button ID; plugin-specific button IDs fixed it in two consecutive FileManager profile launches.

## MCP integration (2026-10-13)

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 update --offline` | Passed | Reconciled the lockfile from the cached registry for the official `rmcp` 3.5.1 SDK and current workspace manifests. |
| Initial `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Failed, then fixed | The first compile found the Windows `PrintWindow` API under the XPS module and bulk outcome types without serialization. The adapter now calls the correct Windows API and maps operation outcomes into explicit JSON fields. |
| `cargo +1.97.1 fmt --all -- --check` | Passed | Final formatting check. |
| First MCP-integrated `cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Passed before later MCP/workspace edits | Windows MSVC shell dependency graph, including the MCP tools and native screenshot code at that point. The `export_metadata` tool and later workspace edits were added afterward, so they are not covered by this passing result. |
| `cargo +1.97.1 fmt --manifest-path crates/fileatlas-plugin-mcp/Cargo.toml -- --check` | Passed | MCP crate formatting. |
| Concurrent-edit `cargo +1.97.1 fmt --all -- --check` (intermediate) | Blocked, later resolved | FileManager referenced a module before it was created and Preview had unformatted changes. The final formatting check after integration and repair passed in the section below. |
| Earlier `cargo +1.97.1 fmt --all -- --check` | Passed | Passed before the latest concurrent workspace edits. |
| MCP-era `cargo +1.97.1 check -p fileatlas-shell --locked -j 2` (intermediate) | Failed, later resolved | At that point FileManager lacked the scroll-tracking API and Updater emitted warnings. The API integration was corrected; final shell check passed in the section below. |
| Tests | Not run | Followed the repository code-generation pass restriction. |
| MCP client launch and live screenshot fidelity | Unverified | The `--mcp` stdio path and `PrintWindow(PW_RENDERFULLCONTENT)` implementation compile. A live MCP client round trip and visual comparison of the returned image against the GPUI window still require Windows runtime validation. |

## Parallel migration and repair (2026-10-13)

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 update --offline` | Passed | Reconciled the lockfile for updater, Rodio, native dialogs, and the workspace-registered AI crate using the cached registry. |
| `cargo +1.97.1 fmt --all -- --check` | Passed | Final workspace formatting check after the parallel implementation and repair rounds. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Passed | Final Windows MSVC shell dependency graph compiled with AI, updater, playback, native dialogs, and UI changes. Earlier runs exposed and then fixed Navigation menu closure ownership, Settings install-path ownership, History event argument inference, FileManager imports/dialog APIs/snapshot borrowing, and Shell Cordis context ownership. |
| Tests | Not run | Per the repository's code-generation pass restriction. |
| Windows user-interface, model download/inference, updater replacement/restart, audio device, Explorer and Recycle Bin behavior | Unverified | These require interactive Windows validation and representative media/user-library fixtures. |


## Parallel remaining-feature migration and consolidation (2026-10-13)

Ten feature work packages were developed by subagents, followed by a separate
consolidation and repair pass. The pass advances FileManager viewport virtualization,
micro-thumbnail rendering, Preview transforms/session state and host actions,
AI suggestion application/history, tag drag/drop and writeback error feedback,
Settings search-scope and toolbar persistence, export status handling, theme
tokens, and bulk-action behavior. Source audit entries were updated to retain
the remaining parity limits.

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 update --offline` | Passed | Reconciled workspace package dependency edges after Preview began depending on AI, Settings, Tagging, History, and File Operations. No packages changed versions. |
| Initial `cargo +1.97.1 fmt --all -- --check` | Failed, then corrected | Reported formatting in the concurrent feature edits. Formatting was corrected by the owning agents. |
| Final `cargo +1.97.1 fmt --all -- --check` | Passed | Workspace formatting check after the consolidation pass. |
| Initial `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Failed, then corrected | First required lockfile reconciliation. Subsequent compiler diagnostics found GPUI drag/drop trait/stateful-ID/type annotations, Preview fluent-builder imports and asynchronous callback captures, FileManager uniform-list scrolling, disabled-button trait scope, and render-image frame construction. Owning agents fixed them. |
| Final `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Passed | Windows MSVC shell dependency graph compiled successfully with the integrated feature changes. |
| Plugin/workspace tests | Not run | Per repository instructions, tests remain reserved for a separate concentrated debugging pass. |
| Windows visual, shell, media, and filesystem behavior | Unverified | The compile does not validate UI interactions, model download/runtime inference, native dialogs, OLE, Recycle Bin, real-media metadata, or filesystem races. |

The integrated FileManager supports rubber-band selection, AI Suggest Tags card
actions, sidebar-tag drop assignment, and card/Preview action menus. Preview
now renders audio with filename, tags, rating and default-player action; its
filmstrip follows image/video/audio query order, and video opens through
MediaPlayback. Windows visual/native behavior—including pointer interactions,
dialogs, OLE, Recycle Bin, media devices, real-media metadata and filesystem
reconciliation—remains unverified.

## Migration audit and documentation reconciliation (2026-10-13)

The feature-to-source table in `docs/source-feature-audit.md` is the canonical
migration status. The older duplicated implementation-slice summary was removed
because it no longer reflected the latest FileManager, Navigation, AI, updater,
Preview, and Settings work. Crate `PORTS.md` coverage was checked for all current
workspace crates.

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 fmt --all -- --check` | Passed | Ran against the current workspace after the latest migration commits. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Passed | Ran against the current workspace; no compiler warnings were emitted. |
| Tests | Not run | This documentation reconciliation does not change implementation code. The latest feature migration test pass remains deferred as described above. |
| Windows/MCP runtime validation | Still required | MCP client round trips, screenshot fidelity, model inference/download, updater replacement, media-device behavior, Explorer/OLE, Recycle Bin, native dialogs, and watcher race behavior require focused Windows validation. |

MCP mutations are disabled by default and require the environment opt-in.
Selected high-impact tools also require `confirm: true`; this acknowledges the
call but does not prove a human reviewed it. Configure a trusted MCP client to
request user confirmation before mutation, and do not treat tool descriptions
as an application-enforced safety boundary.

## Consolidation repair and source parity review (2026-10-13)

This pass compared changed FileManager, Navigation, Preview and Settings behavior
with the pinned source and repaired stale parity notes. It added a shared typed
`TagDragPayload` so dropping a Navigation tag onto a FileManager card assigns that
tag to the card or its current selection. FileManager and Navigation view code was
split into behavioral/rendering submodules to keep each Rust source file below
800 lines. The source feature audit now reflects the implemented rubber-band
selection, AI Suggest Tags, context actions, drag assignment and calendar bulk
date controls. Source behavior that does not exist, such as a pick/reject action
handler, remains identified as absent in the source.

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 update --offline` | Passed | Reconciled the local workspace dependency edges without changing package versions. This was required before the prescribed `--locked` check would run. |
| Initial `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Failed, then repaired | First rejected the stale lock file; after lock reconciliation, found missing Navigation event type/FluentBuilder declarations, Settings/Preview GPUI async closure arity, Preview `ImageDecoder` trait import/mutability, and FileManager event payload annotations. |
| Final `cargo +1.97.1 fmt --all -- --check` | Passed | Ran after FileManager/Navigation module splits and source parity changes. |
| Final `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Passed | Windows MSVC host shell dependency graph compiled successfully after the repairs. |
| Tests | Not run | Per the repository code-generation pass restriction. |
| Windows UI/media/filesystem behavior | Unverified | Build success does not verify native drag/drop, selection geometry, menus, playback devices, filesystem operations or full-format media behavior. |


### Audio Preview and full media sequence (2026-10-13)

Source review of `vendor/photag-source/src/ui/preview.rs` confirmed the audio
stage, filename, default-player action, tag chips, 0–5 rating, close and arrow
controls. Preview now provides the audio stage and routes the default-player
action through File Operations. FileManager supplies the complete ordered
image/video/audio sequence; arrows navigate across media kinds, and video
entries hand off to Media Playback. Audio/video image-only transforms are not
shown as content behavior. Windows media-device and visual interaction behavior
remains unverified.

### Final post-audio verification (2026-10-13)

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 fmt --all -- --check` | Passed | Ran after adding Audio Preview and splitting Preview toolbar support. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Passed | Incremental shell dependency check completed successfully after the final media-sequence changes. |
| Tests | Not run | Prohibited during this migration pass by the workspace instructions. |
| Windows UI/media/filesystem behavior | Unverified | This compile does not validate native focus/navigation, default-player launch, playback devices, dialogs, or visual parity. |

### Preview final split compile repair (2026-10-13)

| Check | Result | Notes |
|---|---|---|
| Initial `cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Failed, then repaired | The Preview module split left `DatabaseService` and `SessionService` imports out of scope and omitted `StyledExt` for `font_medium`; added the imports to the owning modules. |
| `cargo +1.97.1 fmt --all -- --check` | Passed | Ran after import fixes. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Passed | The shell dependency graph compiled successfully after Preview repair. |
| Tests / Windows validation | Not run / unverified | Tests remain prohibited in this pass; no Windows UI or media behavior is claimed as verified. |

### Full source-parity and consolidation pass (2026-10-13)

Ten subagents reviewed separate feature groups in parallel batches, followed by
a distinct full-table source review and repair pass. The pass migrated the
remaining code-backed interactions found in source comparison, including
Statistics, tag associations/merge/delete flows, recent and collapsed tag
picker state, filter modes, date grouping persistence, Settings modal/update
offers, video controls/fallback, theme alpha consumption, and service shutdown
ordering. It also synchronized the crate `PORTS.md` guides and this feature map.

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 update --offline` | Passed | Reconciled the added local crate edge and `open` dependency without changing third-party package versions. |
| `cargo +1.97.1 fmt --all` | Passed | Applied rustfmt after the final module splits and UI integration changes. |
| `cargo +1.97.1 fmt --all -- --check` | Passed | Final workspace formatting check after the UI integration repairs. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Passed | Final Windows host shell dependency graph compiled after this check exposed and the pass repaired type/import/ownership errors in the contributed UI paths. |
| Modified Rust source file size | Passed | All modified Rust files are below 800 lines; Navigation, Preview and Settings views were split by behavior. |
| `git diff --check` | Passed | No whitespace errors in the consolidated changes. |
| Tests | Not run | Reserved for the user's second-round concentrated debugging per workspace instructions. |
| Windows UI/media/filesystem behavior | Unverified | Native modal/keyboard behavior, update replacement, codecs/audio devices, FFmpeg corpus, OLE, shell dialogs, watcher delivery and visual parity require real Windows validation. |

### Graph-guided source code parity review (2026-10-13)

The reference mirror and migrated workspace were indexed separately with the
codebase-memory graph. The reference index covered all 22 Rust files (1,228
nodes / 5,195 edges, no reported parse gaps); the final post-test target index
contained 2,807 nodes / 10,539 edges (vendor, docs and build output excluded).
Traces from
`AppState::process_pending`, `show_file_manager`, `show_tag_node`, and `show_preview`
were used to navigate source call paths, then relevant destination code was
read directly. The detailed feature-family mapping and remaining caveats are in
`docs/source-feature-audit.md`.

Direct source review found two FileManager parity defects: Enter stopped previewing
when Settings was available, and multi-selection had no visible Preview-first
action despite the source exposing Open for a non-empty selection. Both were
fixed in `crates/fileatlas-plugin-file-manager/src/view/keyboard.rs` and `view/render.rs`.
The same review recorded one intentional difference: after AI automatic
inference fails or disconnects, the migrated Preview marks the image analyzed
to avoid a tight automatic retry loop; the source leaves it eligible for retry.
Manual Analyze remains available. This is a behavior difference, not an absent
feature.

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 fmt --all -- --check` | Passed | Run after the FileManager parity repairs. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Passed | Incremental shell dependency graph check after the FileManager parity repairs. |
| `git diff --check` | Passed | No whitespace errors at the time of the final review; line-ending normalization warnings may appear on Windows for edited Markdown files. |
| Tests | Not run | Per repository instruction, reserve tests for the user's second-round concentrated debugging. |
| Equivalence / runtime | Not established | Graph correspondence and successful compilation do not prove every source branch or Windows behavior. Full SQL, native UI, media corpus and system integration still need focused validation. |

### Second-round concentrated debugging (2026-10-13)

This is the second-round test pass requested after the implementation/audit
round. It adds FileManager keyboard regression coverage, legacy-database reopen and
query behavior checks, common still-image decoder coverage, and a separately
run FFmpeg-generated video thumbnail smoke test.

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 test --workspace --locked -j 2` | Passed | 42 passed, 0 failed, 3 ignored. Ignored tests are two live network checksum tests and an FFmpeg smoke test; the FFmpeg smoke test was run separately and passed below. |
| `cargo +1.97.1 test -p fileatlas-plugin-indexing --locked extracts_a_thumbnail_from_a_valid_generated_video -- --ignored --nocapture` | Passed | On Windows MSVC with FFmpeg 8.1 available, generated a valid short MP4 and extracted a JPEG thumbnail through the migrated indexing FFmpeg helper. |
| FileManager regression tests | Passed | 15 tests, including Enter preview while a configured open key is active and Enter opening the first selected item in selection order. |
| Database tests | Passed | 4 tests, including pre-migration library preservation/defaults after upgrade and reopen, plus representative source filter/sort behavior against temporary SQLite data. |
| Thumbnail / indexing tests | Passed | Synthetic JPEG, PNG, WebP, TIFF, BMP thumbnail generation; image/video-poster pairing and hidden-file filtering. These are generated fixtures, not a representative real-media corpus. |
| `cargo +1.97.1 test -p fileatlas-plugin-file-operations --locked` | Passed | 3 Windows temporary-directory tests verify rename/index update, move within a registered library, move outside the library while retaining the file and removing the row, and collision leaves both source and index unchanged. |
| Windows watcher test | Passed | The workspace run executed native file create/remove reconciliation against the Windows filesystem. Rename delivery and harder race cases remain unverified. |
| Native GUI, shell operations and real library corpus | Unverified | This headless test run did not exercise live GPUI focus/dialogs, Explorer OLE, Recycle Bin, audio devices, updater replacement, real FileAtlas libraries, or diverse camera EXIF files. |

### Continued source-parity regression checks (2026-10-13)

Direct inspection compared `vendor/photag-source/src/xmp.rs` with the migrated
`crates/fileatlas-plugin-tagging/src/xmp.rs`; the writer logic remains source
compatible. Added isolated file tests for JPEG APP13/IPTC insertion and removal,
mtime restoration, XMP escaping and sidecar removal, non-JPEG byte
preservation, and visibility/clearing of a background worker write failure. A
temporary SQLite test verifies the actual folder/media, parent/child tag,
media-tag, and tag-association cascade behavior. Deleting a
tag parent removes its child and association edge while retaining the unrelated
associated tag and its media assignment; deleting the folder then removes the
media row and its assignment.

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 test -p fileatlas-plugin-tagging --locked xmp::tests -- --nocapture` | Passed | 2 isolated temporary-file tests. |
| `cargo +1.97.1 test -p fileatlas-plugin-database --locked folder_and_tag_foreign_keys_cascade_through_related_rows -- --nocapture` | Passed | Exercises foreign-key cascades on a disposable database. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 test --workspace --locked -j 2` | Passed | 47 passed, 0 failed, 3 ignored (two live network checksum tests and the optional FFmpeg smoke test, which passed separately in the previous section). |
| `cargo +1.97.1 test -p fileatlas-plugin-tagging --locked failed_background_write_is_visible_and_can_be_cleared -- --nocapture` | Passed | A missing temporary media file drives the actual worker error branch; the failure is observable and acknowledgeable. |
| `cargo +1.97.1 test -p fileatlas-plugin-tagging --locked queued_writes_for_a_path_coalesce_to_the_latest_tags -- --nocapture` | Passed | A channel gate holds the first write while two newer states are scheduled; the queued middle state is discarded and the last state is written. |
| `cargo +1.97.1 fmt --all -- --check` | Passed | Workspace formatting check after the deterministic worker test. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Passed | Host shell dependency graph compiles after the writer seam and test changes. |
| `git diff --check` | Passed | No whitespace errors. |
| Real camera/editor interoperability | Not tested in this slice | Requires representative photos and Windows applications; the deterministic queue test is not a substitute for metadata interoperability. |

### Development media library and scale-test plugin (2026-10-13)

Added `test-data/fileatlas-fixtures/` with 11 generated files (about 6.32 MB):
JPEG, PNG, WebP, GIF, TIFF, BMP, WAV, an intentionally invalid JPEG, nested
directories, and a hidden support thumbnail. The opt-in
`fileatlas-plugin-dev-test` panel uses the production indexer to scan the checked-in
folder and transactionally injects/replaces/cleans 1K, 10K, or 100K virtual
media rows for FileManager and SQLite load checks. Virtual rows do not create files.
The dev-test profile redirects database, settings, session and thumbnail cache
to `target/dev-test/` by default; it does not touch the regular FileAtlas database.

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 run -p fileatlas-plugin-dev-test --example generate_test_media --locked` | Passed | Generated the reproducible local fixture set. |
| `cargo +1.97.1 test -p fileatlas-plugin-dev-test --locked -- --nocapture` | Passed | 2 tests: production scanner indexes nested real fixtures and sees both generated and missing thumbnails; temporary SQLite database replaces 100,000 rows, validates type/thumbnail distribution, then clears all virtual rows without creating physical files. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 test --workspace --locked -j 2` | Passed | 49 passed, 0 failed, 3 ignored (two live network checksum tests and the optional FFmpeg smoke test, separately passed above). |
| `cargo +1.97.1 fmt --all -- --check` | Passed | Workspace formatting check. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Passed | Host and opt-in plugin compile together. |
| `git diff --check` | Passed | No whitespace errors. |
| Interactive Windows profile and 100K FileManager scrolling | Not run | Launch with `start-dev.bat dev-test` to inspect real GPUI rendering, database response and scrolling; automated tests do not measure user-visible frame rate. |

### MCP control of the development child process (2026-10-13)

The MCP child process is the FileAtlas application instance: its GPUI window,
Cordis services, and stdio MCP server share one process. With
`FILEATLAS_DEV_PLUGINS=dev-test`, MCP now exposes fixture scan, virtual 1K/10K/100K
injection, virtual cleanup, and task status. The agent can invoke these tools,
poll completion, query FileManager data, and request a screenshot from the same
window. Attaching MCP to a separately launched FileAtlas main process remains a
future IPC/transport change.

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 check -p fileatlas-plugin-mcp --locked -j 2` | Passed | MCP and optional development-service integration compile together. |
| Live MCP client launching a GPUI window and calling dev-test tools | Not run | Requires an MCP client configured to spawn the child and a Windows interactive desktop. |
| Screenshot image fidelity on the launched GPUI window | Not run | `screenshot_window` is implemented; compare its returned image with a Windows desktop capture during runtime validation. |

### Local Windows validation phase (2026-10-13)

Updated `AGENTS.md` to reflect the current local validation phase. Compared the
local verification targets with the source-backed feature families in
`docs/source-feature-audit.md` and the pinned PhoTag v0.2.0 sources under
`vendor/photag-source/`. Existing targeted tests exercise schema/query
compatibility, FileManager filters and selection, file operation ordering,
tagging/XMP writeback, image thumbnails, watcher reconciliation, and the
development scanner/virtual-row tool. This is a code/test comparison, not a
manual one-to-one interaction walkthrough of every source feature.

The first real Windows launch of the `file_manager` and `dev-test` profiles failed
with `STATUS_STACK_OVERFLOW`; the empty `core` profile did start. Added an
MSVC-only 8 MiB executable stack in `crates/fileatlas-shell/build.rs`, retaining
the GPUI Windows main thread. After that change, both profiles started and
created a native window handle. They were run with `LOCALAPPDATA` redirected to
`target/local-validation` and dev-test data under `target/dev-test`, then
closed after confirming startup. This validates process/plugin startup, not
interactive rendering or feature workflows.

For the next live interaction/debug pass, configure a local MCP client to spawn
`fileatlas-shell --mcp` and use the same-process window/library/media tools; enable
`dev-test` to scan the generated fixture library and exercise 1K/10K/100K
virtual FileManager rows. The server cannot attach to the separately launched
processes used for the startup checks above. Keep mutations disabled for
read-only inspection. See `crates/fileatlas-plugin-mcp/PORTS.md` for client
startup, available tools, and mutation gates.

| Check | Result | Notes |
|---|---|---|
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 fmt --all -- --check` | Passed | Run after the Windows stack configuration change. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Passed | Windows MSVC shell dependency graph compiled. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 test --workspace --locked -j 2` | Passed | 49 passed, 0 failed, 3 ignored. Two ignored cases require live network checksum downloads; the ignored FFmpeg test passed when run separately below. |
| `cargo +1.97.1 test -p fileatlas-plugin-indexing --locked extracts_a_thumbnail_from_a_valid_generated_video -- --ignored --nocapture` | Passed | FFmpeg 8 generated a short video and the migrated helper extracted its JPEG poster. |
| `$env:FILEATLAS_DEV_PLUGINS='file_manager'; $env:LOCALAPPDATA='F:\AI\geren\pho_cordis\target\local-validation'; cargo +1.97.1 run -p fileatlas-shell --locked -j 2` | Passed | Launched with `Start-Process`; native process/window handle observed, then closed after startup check. |
| `$env:FILEATLAS_DEV_PLUGINS='dev-test'; $env:FILEATLAS_DEV_TEST_DIR='F:\AI\geren\pho_cordis\target\dev-test'; $env:LOCALAPPDATA='F:\AI\geren\pho_cordis\target\local-validation'; cargo +1.97.1 run -p fileatlas-shell --locked -j 2` | Passed | Launched with `Start-Process`; isolated dev-test profile created a native window handle, then closed after startup check. |
| Interactive FileManager/Navigation/Preview workflows and visual/source parity | In progress | The isolated reference and migrated apps were both launched. FileManager, tag creation, media selection, preview, and info-panel screenshots were captured. Live comparison found and fixed Preview layout/centering. Settings tabs were separately captured in the reference app; migrated Settings, Navigation input/dialogs, drag/drop, and remaining Preview controls still need one-to-one interaction checks. |
| MCP child round trips | Passed | The local MCP stdio client initialized, listed 27 tools, scanned the fixture library to 9 indexed media, queried library/media state, opened a PNG in the actual window, toggled preview info, set a 4-star rating, assigned a tag, and verified the resulting media/tag state. `screenshot_window` returned inspected 2326 × 1699 PNG captures from the same GPUI process. |
| Original reference launch isolation | Passed with one initial caveat | The original v0.2.0 app was built from a temporary source copy. Its first launch touched the default FileAtlas runtime profile before the path override was added; no UI mutations were made there. The reference was stopped, patched only in the temporary copy to use `FILEATLAS_REFERENCE_DATA_DIR`, then relaunched with its database and settings isolated under `target/reference-local`. All subsequent reference operations used the isolated profile. |
| Native dialogs, OLE drag-out, updater replacement, real camera/editor metadata, codecs/audio devices | Not tested | The source audit records these as remaining runtime/corpus validation items. The temporary reference fixture is synthetic and does not prove codec or metadata interoperability. |

### Follow-up live UI debugging (2026-10-13)

Continued the reference comparison in the isolated Windows profiles above. The
first migrated FileManager capture showed nine records in MCP but no visible media
cards. Runtime layout tracing confirmed 9 skeletons, 9 loaded media records,
and 3 visual rows; the list viewport was shorter than a card row. The FileManager
now starts with advanced type/sort/rating/date/extension filters collapsed,
allows six columns on the validated window size, reserves enough viewport for
two rows, and clips thumbnails to each card. AI suggestions and playback
controls return zero-size content when no image/video is active. The resulting
MCP screenshot shows all nine fixture records in two rows, including generated
thumbnails and audio/broken-image placeholders. Advanced filters remain
available from “More filters”.

The live Settings tab pass had also exposed duplicate GPUI accessibility IDs in
repeated option controls and a content card with no bounded height. Option IDs
are now unique, and the Settings card has an explicit viewport-derived height;
the migrated Appearance, Toolbar, Shortcuts, Tags, AI, and Updates tabs were
opened through UI Automation after the fixes. Navigation tags and expanded
folder hierarchy were also exercised against the same 9-item fixture.

These fixes improve live functionality but do not establish full visual parity:
the migrated top-level layout and filter controls still differ from the
reference, the validated file_manager uses generated synthetic media, and native
Windows dialogs, drag/drop, file operations, codec coverage, and external
metadata interoperability remain to be exercised individually. Screenshots
are local ignored artifacts under `target/validation-screenshots/`, including
`reference-file-manager-window.png`, `migrated-gallery-clean.png`, and the six
`migrated-settings-*.png` captures.

| Check | Result | Notes |
|---|---|---|
| UI Automation: open migrated Settings and switch through all six tabs | Passed | Repeated after fixing duplicate a11y IDs and card height. |
| UI Automation: switch Navigation to Tags and expand fixture folders | Passed | Tag row and nested folder counts correspond to the indexed test library. |
| Same-process MCP scan/query/screenshot and tag/rating round trip | Passed | 9 fixture media; screenshot displays all 9 FileManager cards; queried `small.png` returned 4-star rating and `ValidationTag`. |
| Shell structure: hide development/metadata panels by default and expose from Tools | Passed | UI Automation invoked Tools and found the metadata export and development controls; FileManager cards stay visible with Tools closed. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 fmt --all -- --check` | Passed | After live UI fixes. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Passed | Windows shell and plugin dependency graph. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 test --workspace --locked -j 2` | Passed | 49 passed, 0 failed, 3 ignored. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 build -p fileatlas-shell --locked -j 2` | Passed | Executable used for the final MCP screenshot pass. |

Original runtime screenshots are now committed under
`docs/reference-ui/2026-10-13/`; the accompanying README maps captures to
their UI state and records the isolated reference build and fixture context.
The shell now treats the header, navigation/file_manager workspace, contextual
views, and optional Tools panel as separate regions. The Tools panel starts
closed so metadata-export and dev-test surfaces do not consume the FileManager's
workspace by default. This is the first structural alignment pass; detailed
toolbar placement and remaining source-layout differences are still follow-up
work. The current capture still shows empty gray bands at the top and bottom
of the content pane, so exact workspace boundary alignment is not yet accepted
as matching the reference. Navigation's folder picker is now the compact `+`
action; manual path entry is behind `Path` so the folder tree stays visible.
UI Automation confirmed the editor is absent by default and appears when `Path`
is invoked; its close/reopen sequence still needs a clean follow-up check.

### Live reference-to-migration UI comparison (2026-10-13)

Built and launched PhoTag v0.2.0 from `vendor/photag-source/` using a temporary
copy, then ran the migrated `target/debug/fileatlas-shell.exe --mcp` with the
opt-in `dev-test` plugin. Both profiles used generated test media; the
migrated database/settings/cache live under `target/dev-test/` and its
`LOCALAPPDATA` redirect, while reference data lives under
`target/reference-local/`. This round validates only the interactions listed
below; it is not full source-feature parity.

The reference was operated through Windows UI Automation and mouse input:
open folder picker, add and scan the fixture folder (9 visible items), switch
to Tags, create `ValidationTag`, select `tiny.jpg`, show the selection action
bar, and open its preview. Reference Settings tabs for Appearance, Shortcuts,
Tags, AI, and Updates were also captured. Migrated interactions used MCP tools
against the same process that owned the GPUI HWND: scan fixture, query 9 media,
create/read `ValidationTag`, open `small.png`, toggle info, set rating 4, assign
the tag, and query the final record. `get_view_state` matched the selected path
and info toggle; `get_media` returned rating 4 and `ValidationTag`.

The comparison caught two actual Preview defects. The view was registered in
`workspace.main`, so it appeared as a short panel below FileManager instead of
FileAtlas's full-window preview. The preview image row also used alignment styles
without flex display, pinning the image to the stage's upper-left. The view now
uses `workspace.modal`, returns a zero-size element when no source is open,
draws a dim full-window backdrop, and uses flex containers to center the
image. Rebuilt and inspected the resulting MCP PNG: `small.png` is centered,
the FileManager is dimmed, and the file/image/EXIF panel appears when info is
enabled. The original Reference screenshot and migrated screenshot artifacts
are in the ignored local directory `target/validation-screenshots/` (for
example `reference-settings.png`, `reference-selected-media.png`,
`reference-preview.png`, `migrated-gallery-clean.png`, `migrated-preview.png`,
and `migrated-preview-info.png`).

| Check | Result | Notes |
|---|---|---|
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Passed | Repeated after the modal-layer and image-centering fix. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 build -p fileatlas-shell --locked -j 2` | Passed | Rebuilt the actual Windows executable used by the MCP UI walkthrough. |
| MCP fixture scan and state round trip | Passed | 9 media rows; small PNG opened; info became true; rating 4 and tag association were returned by `get_media`. |
| Preview full-window screenshot comparison | Passed for tested state | MCP `PrintWindow` output visually inspected; image centered and FileManager dimmed. High-DPI/window-size parity and every toolbar action are not yet checked. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 test --workspace --locked -j 2` after this UI fix | Passed | 49 passed, 0 failed, 3 ignored (two live network checksum downloads and the optional FFmpeg smoke test). |
| `cargo +1.97.1 fmt --all -- --check` | Passed | Workspace formatting check after all local validation changes. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p fileatlas-shell --locked -j 2` | Passed | Includes the MCP close action and Preview modal/wrapping changes. |


### Window bands, Fluent localization and MCP-only UI validation (2026-10-13)

This change continues the interrupted window-framework work, then adds the
requested independent English/Chinese plugin. The user's latest instruction
requires MCP testing/screenshots and excludes direct computer control. The
final validation below uses only the same-process stdio MCP endpoint; no OS
mouse/keyboard injection or Computer Use is involved. Earlier reference-window
observations are retained as earlier evidence, not repeated pointer tests.

**Environment:** Windows/MSVC; Rust +1.97.1; Cargo jobs2/incremental enabled;
source viewport1280×800, sidebar220 and initial thumbs150; generated 9-media
fixture. The observed PrintWindow PNG size is1942×1256 physical pixels including
native frame/DPI scaling. Reference and migration use the same fixture files;
identical modified-date ties can yield different item order because independent
scans assign different IDs. No pixel-perfect or whole-source acceptance is claimed.

**Source rules inspected:** main.rs:1007-1097 full-width commands/search/filters;
1099-1277 status modes;1282-1367 sidebar/resizer and single elevated FileManager;
filterbar.rs:15-155 search/scope and600-821 sliders/dates; sidebar.rs:181-193,
228-275 pivot tabs; main.rs:2111-2175 Settings geometry/tabs and2187-2246 swatches;
gallery.rs:1383-1590 square cards/hover/selection/overlays; preview.rs:940-1002
and chrome state rules. Source dimensions come from code, not PNG measurement.

Changes and actual observations:

- FileManager contributes command.search/size/filters/tools, workspace.status/modal
  and file_manager.content as projections of one entity. Shell composes the full-width
  command band, two-column body, one panel card and full-width status. Duplicate
  FileManager heading/options/search/action rows are gone. Selecting small.png via
  MCP produces the real bottom action bar and selected inset card. Existing
  ratings/details/rename/copy/move remain under extra selected actions.
- Thumbnail/rating controls use the source continuous80–300 and nearest-thumb
  range interaction. Their rendering was inspected; pointer drag/edge clamping
  was not exercised by MCP. Statistics and Bulk actions now open as overlays,
  not permanent rows. No destructive or copy/move operation was executed.
- Sidebar pivot tabs use equal halves, height32 and the source underline.
  Appearance has22×22 palette swatches. Six Settings tabs do not wrap; the width
  uses source min72 plus intrinsic text/button spacing. A live auto-height
  attempt collapsed GPUI scroll content to zero; fixed bounded scroll height
  was restored and all six pages were captured again. The520-point cap is still
  a source difference, as are checkbox presentation and section grouping.
- fileatlas.i18n uses fluent-bundle0.16, embedded en/zh-CN resources, English default
  and English fallback. Language is saved in a separate language.json; normal
  source settings/session/SQLite are not migrated. Live language changes update
  labels, month/count/Statistics text and existing input placeholders without
  clearing typed values. A separate isolated child restart returned zh-CN after
  it had been selected; validation resets its own profile to English afterward.
- MCP adds get_ui_state and control_ui for Settings tabs, language, FileManager
  Statistics/Bulk/selection/deselection. Requests use public services and the
  existing GPUI actions, and are asynchronous. Four i18n tests cover all Fluent
  messages and arguments/plurals, persistence, failed writes, and Cordis cleanup.

Exact reproducible commands:

```powershell
$env:CARGO_BUILD_JOBS='2'
$env:CARGO_INCREMENTAL='1'
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check -p fileatlas-shell --locked -j 2
cargo +1.97.1 build -p fileatlas-shell --locked -j 2
cargo +1.97.1 test --workspace --locked -j 2
node scripts/validate-ui-mcp.cjs
```

The Node client launches target/debug/fileatlas-shell.exe --mcp with
FILEATLAS_DEV_PLUGINS=dev-test, FILEATLAS_DEV_TEST_DIR=target/mcp-ui-validation,
LOCALAPPDATA=target/mcp-ui-validation/local and FILEATLAS_MCP_ALLOW_MUTATIONS=1.
Mutation opt-in is needed for fixture scan, language persistence and UI state
requests; the script never executes delete/copy/move/export/update tools.
Updates are disabled only in its isolated settings. The client initializes the
protocol, scans through the production service, verifies9 records, requests
screenshots, writes its report and terminates its child. All23 PNGs and the
protocol report are under target/mcp-ui-screenshots; selected captures are
retained under docs/validation-ui/2026-10-13.

| Verification | Outcome / limit |
|---|---|
| Locked formatting/check/build | Passed on the Windows host. |
| Workspace tests | 53 passed,0 failed,3 ignored; ignored network/FFmpeg cases remain as documented earlier. |
| MCP FileManager and selection | Passed for9 synthetic records, both languages; no pointer selection test. |
| MCP Settings | Captured Appearance,Toolbar,Shortcuts,Tags,AI,Updates in English and Chinese; language service/refresh observed. Mouse activation and save of every individual option were not tested. |
| MCP Statistics/Bulk | Both overlays captured in both languages; count query rendered. File execution/native confirmations were not invoked. Source Statistics photo/video total excludes audio, so its8 total differs from9 indexed library rows. |
| MCP Preview Info | small.png decoded/centered, left Info fields rendered and translated; get_view_state show_info true. Remaining toolbar position/order/filmstrip/EXIF and media-device differences persist. |
| Preview time fade | Real service sampled near0,2.05,2.35,2.9s: opacity1,about0.93,about0.56,0. Hidden-chrome PNG captured. This checks idle fade, not mouse-triggered reappearance or GPU frame cadence. |
| i18n persistence/cleanup/failure | Four targeted tests passed; live isolated restart confirmed saved Chinese; default missing preference is English. |

**Remaining differences:** FileManager filename/tag hover overlays and the120ms
hover scale/shadow/brightness,140ms selection transition and220ms decode fade
are not established as matching the reference. Cards retain always-visible
migration action affordances and placeholder differences; month typography,
sidebar folder/tag rows/context menus, source scan-progress status priority,
Settings default tab/checkbox/group layout, Preview tool ordering/position, and dialogs
still require restoration. Some generated status/error text, backend/native
messages and external release notes may remain English. No pointer hover,
tooltip activation, native dialogs, drag/drop, Recycle Bin, Explorer actions,
real codec/audio device or metadata interoperability were tested in this pass.
The complete UI migration remains incomplete; these screenshots and checks
only establish the listed states.

## 2026-10-13 GPUI Kit integration audit

Read-only code/API review on Windows/PowerShell in F:\AI\geren\pho_cordis.
Application code was not changed. See
[integration audit](gpui-kit-integration-audit-2026-10-13.md) for source references,
dependency versions, findings and verification limits.

- `cargo fmt --all -- --check`: passed.
- `cargo check --locked -p fileatlas-shell -j 2`: stopped because default rustc1.94.1
  is below workspace requirement1.95; this is not a Kit API compilation result.
- `cargo +1.97.1 check --locked -p fileatlas-shell -j 2`: passed using an installed toolchain.
- `cargo +1.97.1 test --locked -p fileatlas-ui-bridge --lib -j 2`: passed,3 tests.
- `cargo +1.97.1 tree --locked -p gpui-kit --depth 1 --prefix none`: Kit0.7.1,
  Base/Component/Assets0.7.2, GPUI/platform0.3.8.

Static findings: missing application asset-source registration and runtime accent
updates not synchronized to Kit Theme. Custom controls' focus/accessibility and UI
interaction test coverage remain incomplete. No GUI launch or mouse/keyboard/native
workflow was exercised in this audit; build/ownership tests do not prove visual parity.

## 2026-10-13 GPUI Kit resource and runtime accent fixes

The two audit findings are resolved in the working tree: Shell registers
`gpui_kit::assets::Assets` before run, and successful Settings edits call
`theme::apply_accent_components(&mut gpui_kit::App)` after updating the source
accent token. The runtime helper uses Kit `Theme::update` to reconcile colors,
renderable tokens and Base projections and refresh windows; it changes only
primary/accent/selection, retaining the existing 64-alpha selection rule. No
control placement, feature workflow or settings schema was redesigned.
Source references: vendor/photag-source/src/theme.rs:408-534 and
src/main.rs:2187-2246. Existing parallel working-tree changes were preserved.

Environment: Windows PowerShell, F:/AI/geren/pho_cordis; explicitly installed
Rust1.97.1, bounded build/test jobs2, existing incremental profiles.

- `cargo +1.97.1 fmt --all -- --check`: passed.
- `cargo +1.97.1 check --locked -p fileatlas-shell -j 2`: passed.
- `cargo +1.97.1 test --locked -p fileatlas-ui-bridge -p fileatlas-plugin-settings --lib -j 2`:
  passed (UI Bridge3 tests; Settings has0 lib tests).
- `cargo +1.97.1 test --locked -p fileatlas-ui-bridge -p fileatlas-plugin-settings -j 2`:
  passed (same3 tests, doc-tests0).
- `git diff --check` on relevant code and integration docs: passed.

These are compilation and existing lifecycle tests, not runtime theme or SVG
rendering tests. This fix pass did not exercise native windows, accent swatch
clicks, menu/calendar icons, keyboard, codecs or file operations. Windows
interaction and visual parity remain unverified by this pass.

### 2026-10-13 v5 S2 existing backend paths

The inspector reuses TaggingService removal and RemovedTags history (source
FileManager RemoveTags + queries::remove_tags); empty-library add-folder dispatch
reuses Navigation's native picker and production scan/watch services. No
schema or persisted file format change. Existing FileManager/Navigation tests
passed (16/4); MCP scanned 9 synthetic fixture media in an isolated profile.
Inspector tag removal, native folder picker, and real-media interoperability
were not exercised. User-directed quiet startup updates retain source check,
download and install semantics, but suppress automatic offer opening. Live
MCP observed update_available=true/update_offer_open=false after startup and
update_offer_open=true after a manual check; no download/install performed.

## 2026-10-14 backend-preserving redesign validation

Shell1.97.1 locked build/check pass; relevant11crate tests36 pass/1 ignored, separate FFmpeg ignored test1pass. See redesign-validation latest section for exact commands. JSON export12real rows passed and writing to directory returned actual os5 failure. Source-compatible paired video poster validation.png is not a separate indexed image (12total, not13). Invalid JPEG metadata write skips according to source semantics, so actual writeback retry/dismiss was tested using FileShare.None exclusive lock to produce os32 on isolated corrupt.jpg. Production query/schema/JSON writer/tagging order preserved; mapping in source-feature-audit updated. MediaIndexChanged scan completion observations refresh the sidebar; virtual dev-test inject/cleanup selects fixture folder by its existing contract. 100000virtual+12real stress scroll page offset99812 loaded200; crosspage selectionbulk2; virtualrows cleaned and All files12 reverified. No destructive file actions or ordinary userprofile mutation. Native Windows file dialogs/overwrite/recycle/drag and complete codec interoperability remain unverified.

## 2026-10-14 基线后的通用弹出面板／列表／菜单优化

用户指定的四项优化已实现：设置及媒体预览共用UI Bridge浮动框架、主界面点击隔离、Cordis-owned Single/Multiple PanelCatalog与各定义默认尺寸、Grid/List按钮及会话偏好、保持卡片按钮菜单触发实体。设计对照final-v5/01/02/03/05与baseline sidebar-pages-preview-v2；新的窗口几何与列表按本轮用户指令。完整141步MCP参数／结果／截图、准确命令、样本及未覆盖边界见[本轮运行证据](validation-ui/2026-10-14-panels/README.md)。

Rust1.97.1 MSVC fmt/check/标准locked shell build通过；相关26项测试通过，最后UI Bridge5项清理回归通过。真实GPUI设置拖动100×50、缩放900×680→800×580、tab点击与外层隔离；菜单进入和预览ID6、单实例复用、多实例独立移动／关闭；真实图片/WAV/MP4解码及面板max/restore；List实际选择ID9与下一进程恢复；100009项列表末页offset99809/cache200；只读拒绝。正常用户数据未用于测试。

SQLite与既有后端动作语义不改；源Session serde-default与FileManager grid/context-action语义已追踪，映射追加于source-feature-audit.md；file_manager_list_layout字段为兼容的新增偏好。新组件使用说明见[floating-panels](floating-panels.md)。七个未逐项实测resize把手、窗口外捕获、不同DPI、列表修饰键/框选/OS拖出、全codec及听感、现有服务多媒体并行仍未覆盖，不将这些记为通过。

## 2026-10-14 Native AddFolder reentry fix

Source PendingAction::AddFolder optional selection/cancel/register/watch/scan semantics are unchanged. Navigation now awaits rfd::AsyncFileDialog before acquiring GPUI view state; the former synchronous click reproducibly panicked with RefCell already borrowed in gpui-pre async_context.rs:66. Native picker state/capture/cancellation via FileAtlas MCP showed two actual open/cancel/reopen cycles without process exit or library mutation. Navigation tests5 passed, locked shell build/check and fmt passed. Full evidence and remaining native-accept boundary: docs/validation-ui/2026-10-14-folder-picker/README.md.

## 全文件收录及增量记录保留（2026-10-14）

追踪source scan.rs的扩展名/隐藏/封面排除和db/queries.rs的INSERT OR REPLACE，按用户新需求扩大为所有普通文件；`file`复用已有text列。Watcher共享Indexing公开分类器，保留800ms及4000/200限制，排除目录行。为补入旧漏文件，scanner改用数据库公开scan_upsert_media，不替换ID、不清空评分/标签/日期；原insert_media其他消费者不变。自动回归验证两次扫描间新增文件和已有用户数据保留；27测试通过1ignored，Windows同进程MCP实际右键评分后增量扫描ID/评分保留、目录无重复，新增/删除未知文件通知生效。显式Rebuild/Rescan的历史清空语义仍在，增量补扫采用Add同目录。详见[全文件验证](validation-ui/2026-10-14-all-files/README.md)；未扩充codec支持或宣称全平台覆盖。

最终补验：缓存命中增量扫描保留图片尺寸/micro，Indexing回归再次通过；重编译后32步FileAtlas MCP重测保留ID/评分及24×16尺寸，见同证据目录cache-retention-report.json。

## 2026-10-14 启动数据未刷新的原因与修复

用户通过`start-dev.bat`启动仍只看到旧内容。确认脚本会执行最新源码的`cargo run`，但默认载入的是持久化隔离档案`target/dev-test`。旧版索引创建后，启动流程只读出SQLite并注册文件监听，没有调用扫描器；扫描器新增全文件支持后，已有SQLite记录不会自行迁移。Watcher此前仅收集运行期间的创建/删除/重命名通知，所以文件夹中的旧漏项在退出重开后仍留在旧索引。

Watcher现于启动时逐目录调用Indexing生产扫描器；索引写入后发事件使FileManager自动查询新结果。Scanner更新文件大小和修改时间、保留ID/评分/日期/标签；同路径图片内容变化时清除旧路径缓存并重建尺寸和缩略图。完整遍历后移除离线删除行；文件夹不可用时保留数据库行并报告失败。FileManager刷新保留有效选择和滚动位置。使用旧档案副本、`FILEATLAS_DEV_PLUGINS=all`及同进程MCP验证：两文件夹由40/9条刷新为79/12条，文件管理窗口选中路径实时显示60条，总计91条，刷新状态`running=false`、2/2目录完成、0失败；原40条路径ID及评分保持，标签样例也保持原媒体ID和评分。第二次启动数量仍为79/12，未重复插入。未改写`target/dev-test`原数据库及用户文件。

`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked -j 2 -p fileatlas-shell`和`cargo +1.97.1 build --locked -j 2 -p fileatlas-shell`通过。Indexing测试3通过/1原FFmpeg测试ignored；Watcher2通过；FileManager16通过。单元覆盖离线删除和不可用根目录保留；Watcher集成覆盖运行中创建/删除。详见[启动刷新证据](validation-ui/2026-10-14-startup-refresh/README.md)。大型/网络库冷启动耗时、Windows原生通知竞态仍待用户规模验证。

## Grouping extraction/extensions (2026-10-14)

Source trace: vendor/photag-source/src/ui/gallery.rs build_file_manager_layout/format_month_label and caller around1047–1063; original skeleton/full-row queries and session collapse/settings booleans. New fileatlas-plugin-grouping owns stable ordering/section calculation, retaining effective date fallback; database adds a filtered compact input query without schema changes. Root/parent path, extension, indexed mtime and real filesystem creation month are user-requested additions. FileManager fetches arranged IDs across cache boundaries. 26 automated tests plus57 same-process Windows MCP steps with248 files confirm documented behavior, not all filesystems/large-library performance. See docs/validation-ui/2026-10-14-grouping/README.md for commands, compatibility and remaining limits.

## Subfolder current-directory priority (2026-10-14)

This refines the user-requested grouping extension, beyond upstream date grouping. Scoped FileManager passes the current directory to GroupingService::arrange_in_scope; direct files precede descendants and retain query sort. Unscoped direct registered-root groups are ranked first. No DB rewrite or source query semantic change.28 related tests and19 same-process MCP steps passed (root4, nested2, filtered descendant1). MCP Navigation requests retain supplied indexed path spelling after existing canonical containment validation. Evidence/limits: docs/validation-ui/2026-10-14-root-first/README.md.

## Immediate directory browsing extension (2026-10-14)

Source trace: db/queries.rs build_filters/path_prefix, scan.rs list_subdirectories:466 and app.rs get_subdirs:819. None now uses additive direct_directory filtering plus checked existing directory listing. Original recursive load/query APIs, other grouping modes, global search/tag views, SQLite and persisted paths remain compatible. Folder cards route through Navigation public requests and carry no fabricated media IDs; current-folder bulk scope is immediate.36 related tests pass (1 existing FFmpeg ignored);84 same-process MCP steps include actual GPUI double-click, mixed folder/file rubber-band, preview, empty/ordinary restart and200-record last-page cache. Evidence/limits: docs/validation-ui/2026-10-14-folder-browser/README.md.
## 2026-10-15 Preview idle/latency follow-up

Pinned `src/ui/preview.rs` Fast→Full behavior and
`src/ui/gallery.rs::decode_jpeg_scaled/load_full_image_to` were traced for
immediate thumbnail display, scaled JPEG and EXIF handling. The new native
stage retains these semantics while using GPUI's existing full-asset loader;
ordinary images no longer generate redundant oriented PNG files. User
transform composition remains after EXIF and only materializes cache files.
No media files, SQLite schema or normal profile paths changed. Source mapping
and Preview/Debug/MCP PORTS were updated together. Related tests (30) and
locked shell build/check/fmt passed. Same-process Windows pointer/open,
accessibility and idle timing evidence is recorded in
[preview-feedback](validation-ui/2026-10-15-preview-feedback/README.md).
This evidence does not establish full codec/camera/editor interoperability.

## 2026-10-15 Shared UI refresh and full idle audit

Pinned Settings tag-tree, Statistics metrics, bulk predicates, selected media,
EXIF, scan progress and preview/transport semantics were traced and retained.
Render-time read feedback found by actual MCP tests is replaced by UI Bridge
ReadModel; all GPUI timers share refresh::poll and passive change gates. Source
mapping and affected PORTS were updated together. No SQLite schema, persisted
normal profile or command-time target validation changed. Conservative writer
revision invalidation includes partial-write errors without changing SQL order.

Final rebuilt binary SHA-256 is recorded in the evidence. Two same-process
Windows MCP suites exercised136 states (134 settled,2 actually-playing), with
zero sustained/blocked outcomes. Real writes update open Statistics/Bulk, native
Watcher reconciles an isolated fixture, and missing/cleared/off-window selections
settle without repeating reads. Original failures and all quiet rechecks are
retained.89 workspace tests pass,3 existing tests remain ignored; locked shell
check/build/format and architecture guard pass. Runtime scope, inputs, exact
commands, native paint timings and unverified destructive/codec/network-install
boundaries are in [idle audit](validation-ui/2026-10-15-idle-audit/README.md).

## 2026-10-15 Preview per-file phase timing

Instrumented queue wait, metadata/cache check, compressed read, header, decode,
orientation, resize and encoded cache publication without changing source media
or SQLite. Source DCT scaling and generic orientation/pixels are retained; generic
input buffering is the intentional IO-measurement change. Related14 tests pass,
including PNG/BMP/TIFF/WebP equivalence, JPEG dimensions and read/header failures.
Same-process MCP covers cold/warm/corrupt images and actual detail rendering;
see the decode-details validation evidence. Cache-write errors have explicit
failure diagnostics but a forced OS write-denial scenario is not part of that run.

## 2026-10-15 First three preview optimizations

Dev PNG/DEFLATE dependencies are optimized; indexed FileManager thumbnails now
travel through public FilmstripMedia to Preview, with worker freshness checks
and existing missing/stale fallback. JPEG-only progressive work is deferred
until pending >=150ms, at most once per open; full paint/switch/close cancel
unneeded work and stale completions cannot publish. Source mapping is in
source-feature-audit.md and Preview/FileManager/MCP PORTS.

Final same-process MCP actual-folder run:14/14 complete, screenshot PNG
663.3→78.9ms and72MP PNG8045.4→422.1ms; no PNG1600jobs,19 thumbnail
reuses without decoding, one JPEG progressive cancelled before encoding.
All11 source fingerprints unchanged. Synthetic navigation/zoom/rotation/
rapid close/error/details tests pass; corrupt-image render12→12 and raw idle
latest_id701→701. Workspace94 passed/3 ignored before final claim guard;
final Preview11 passed, locked shell build/format/refresh guard pass.
See validation-ui/2026-10-15-preview-optimization/README.md for exact commands,
single-run/cache limits, test harness corrections and remaining boundaries.


## 2026-10-16 浏览与侧栏审查回归

沿用户确认的浏览语义审查其他 AI 的 Explorer 侧栏。设置改为带修订事件与串行持久化，成功才提交内存；会话同值不写、失败不污染快照。浏览任意存在目录可重启恢复，旧资料库成员限制仅留在旧模式。目录快照在途期间合并刷新，完成的部分结果先交付，避免分块反复使其失效。后台源语义对照 vendor/photag-source/src/session.rs、原 main.rs/sidebar.rs 选择与展开流程；新增快速访问和分组收藏按当前用户设计实现。

13 相关 crate 65 测试通过，locked Shell check/build、fmt 和 refresh guard 通过；静态日志 target/sidebar-review-tests-final.log。真实同进程 MCP 的失败写盘/重试、稳定重启、原生监听、十万文件首批/末页/取消/快速切换、标签与缺失重链通过。可追溯证据与未覆盖边界见 validation-ui/2026-10-16-sidebar-review/README.md；历史浏览证据缺失的 summary/PNG 已补齐，不把历史坐标当作当前侧栏点击证据。


## 2026-10-16 进入目录及恢复窗口重复刷新

已移除 FileManager 焦点恢复无条件重扫和注册后根伪事件；后台等待监听注册后再枚举，背景快照保留现有列表/选择/滚动与 rich window。23 相关测试、locked check、独立输出构建、fmt 和 UI refresh guard 通过。同进程 MCP 实际三次最小化/恢复：目录 generation 1→1、查询 revision 2→2、窗口 offset 250→250；真实外部新增/删除仍生效。基本 8 项和十万文件扩展 11 项回归通过。常用 exe 因正在运行被占用，验证输出为 target/debug/fileatlas-shell-refresh.exe，未终止用户窗口。命令、截图、范围和剩余边界见 [重复刷新记录](validation-ui/2026-10-16-refresh/README.md)。


## 2026-10-16 用户指定顶部面包屑地址栏

后续目录性能：UI Bridge Wake/listen连接目录批次、范围事件和ReadModel完成，
使用原采样器按变化刷新，保留bounded4/取消/generation。Yazi精确源码及FileAtlas来源
映射已更新。真实D盘两个目录10次，最终debug构建中位内容发布时间722.0→67.4ms，
源目录名字/大小/mtime不变，持久media/media_tags/tag_previews各0。42测试与同进程
地址11项、刷新4项、自身监听2项通过。十万文件11项扩展通过，首批159ms、全量17775ms，
保留全量处理限制。最终常用exe已更新。原始报告、命令和计时范围见
validation-ui/2026-10-16-navigation-latency/README.md。

完整根至当前目录面包屑、路径编辑、后退/前进/上一级已集成 Navigation 既有贡献。历史统一目录/Home/标签范围，窗口内最多200项，真实路径在动作时验证；当前层级不重载，非法路径保留状态。窗口启动只恢复一个对应范围事件，避免空路径清空标签/收藏。34项测试、locked check、独立输出构建、fmt、UI refresh guard通过；同进程MCP 11项语义/实际Enter-Escape-Ctrl+L、4项真实pointer、文件夹及标签只读重启验证通过，重复刷新回归通过。设计来源为用户圈定截图；截图、命令、接口与未覆盖网络/输入系统边界见 [地址栏证据](validation-ui/2026-10-16-address/README.md)。

## 2026-10-16 DatabaseService 查询边界迁移

生产消费端的 SQLite 回调已收口到具名 `DatabaseService` 操作。复合 FileManager
快照、索引批次、Watcher 协调、Tagging 记录与虚拟数据写入均在 Database 内
持锁/事务；源查询 SQL、schema 和批次顺序保持。守卫现在阻止业务 crate 在生产
代码中调用原始 reader/writer/cache-writer 回调或运行时依赖 `rusqlite`。Dev Test
只通过 Cordis 发布的 `DatabaseService`、`IndexingService` 注入/扫描，不持有连接；
其生产路径不导入 SQL 查询函数，`rusqlite` 仅在单元测试夹具中使用。

验证：`cargo +1.97.1 check --locked --workspace -j 2`、`cargo +1.97.1 build
--locked --bin fileatlas-shell -j 2`、格式检查和四项治理脚本通过。相关 crate
测试累计 58 项通过，Indexing 另 1 项 FFmpeg 依赖测试忽略。同进程 MCP 使用
隔离 profile `target/mcp-db-boundary-validation`。首次重启回归暴露旧版浏览器
模式曾将带评分的虚拟行写入持久库，已增加会话缓存写入、历史保留前缀清理及
跨重启单测。重建可执行文件后，同进程 MCP 注入 1,000 行（6.04 ms）并清理
1,000 行；最终 FileManager 库计数为 0，持久库保留前缀行数为 0。两次只读调试状态
采样无数据库错误且 view revision 稳定。未验证视频 FFmpeg
扩展测试及其他 Windows 原生输入/文件系统竞态。
# Lore-backed historical version switching — 2026-10-17

FileAtlas now separates Lore revision storage (`fileatlas-plugin-versions`) from
durable SQLite display/current-revision metadata (`fileatlas-plugin-version-info`).
The FileManager History inspector and same-process MCP use the version service for
revision create/restore and the Lore branch create/switch/archive/merge lifecycle.
Pending conflicts persist across service restarts and support whole-file ours,
theirs, or abort. Branch switching verifies a clean snapshot unless an MCP
caller explicitly confirms overwrite; file-changing operations refresh the
existing indexed media row. Locked Shell compilation, the metadata test, and
the Lore branch conflict/archive lifecycle test pass. Full validation evidence,
single-file limits, and live Windows/MCP blockers are recorded in
`docs/validation-ui/2026-10-17-lore-version-history/README.md`.


## 2026-10-17 Lore 历史回退显示回归

原始参考源码对照确认：VersionInfo 完整目录必须保留 Lore 当前祖先链未返回的 revision。VersionsService 列表已修正，按持久化 revision number 稳定排列；激活只改当前标记，既有版本行不删除。FileManager 选择与刷新修复见 redesign-validation.md。

27 项 FileManager/VersionInfo/Versions 测试、locked Shell 构建和静态守卫通过。原路径 D:\桌面文件夹\测试\临时文件\00.png 在隔离 MCP profile 中实际创建两版本并通过 GPUI 指针往返切换；两个版本、当前标记、字节和页面选择均保持正确。参考源码映射与证据见 validation-ui/2026-10-17-lore-version-history/reference-source-trace.md 和同目录 README.md。

## 2026-10-17 从旧版编辑后创建03的 BranchAdvanced 修复

修复前服务回归复现用户的完全相同错误。对照 Lore 官方 shared-store 文档和固定 lore-vm 测试，确认 checkout01 不等于分支 tip02；创建快照前需在私有工作树同步最新本地 tip、重新暂存。已处理失败暂存的下一次创建恢复，并保留源文件、01/02及无修改拒绝语义。

cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-versions -p fileatlas-plugin-version-info 四项通过；包含01→02→01修改创建03、三个版本恢复、人工失败暂存后创建04、无修改拒绝、原有分支冲突/解决/归档。locked Shell 构建、fmt、Node语法和三项治理守卫通过；既有 LNK4098 与两个 orphan 目录警告仍保留。

FILEATLAS_LORE_TEST_FILE 指向原路径 D:\桌面文件夹\测试\临时文件\00.png，运行同进程 MCP 场景，profile 为 target/mcp-lore-edit-after-rewind-20261017。已在实际源文件上完成01/02实点往返、停01修改创建03、恢复三份版本字节及可见页验证。quiet snapshot 22→22，文件选中项保留。源文件恢复开始时内容和时间戳，SHA-256 为 3D246F8B6BD16F794EFAA92060439A6CF1B6EE2CC85D3C5250E2579528B253AF，前后一致。证据：validation-ui/2026-10-17-lore-version-history/edit-after-rewind.json 和 edit-after-rewind-restoration.json。

## 2026-10-17 未保存版本内容状态

用户新增文件管理窗口修改提示。沿用参考项目VersionInfo稳定ID和当前标记，固定Lore的
file.hash/file.info按immutable revision比对hash/size，先当前基准再所有已存版本；
不以mtime、branch.latest或私有暂存副本判断。服务测试覆盖同大小内容改变、旧版
未变、时间变化、匹配另一版本、失败暂存与缺失文件。Versions4、FileManager23、
i18n4通过；实际00.png同进程MCP验证监听更新、版本创建/回退/再提交、提示点击，
没有因读取/点击增加版本。检查失败暴露未确认状态，不谎报未保存。
详见 validation-ui/2026-10-17-version-status/README.md。

## 2026-10-17 标签数据库优先与 Inspector 内联添加

按用户要求关闭常规标签赋值/移除/清空/合并/重挂时的自动 IPTC/XMP 写回，
避免在媒体目录创建 XMP 侧车文件；标签关系仅由 SQLite 管理。显式集成 API
仍保留为 opt-in。文件信息右侧 Tags 区域将原加号打开的独立标签面板替换为
原位搜索、多选层级下拉和“添加所选”操作；标签移除 chips 保留。FileManager 批量
标签操作与 Preview 标签选择器不在本次范围内。

参考实现：`vendor/photag-source/src/xmp.rs`（差异：源程序会自动写文件元数据，
当前 FileAtlas 根据用户决定只在明确调用集成 API 时写回）。实现记录和验证边界：
`validation-ui/2026-10-17-tagging-inspector/README.md`。

## 2026-10-17 程序启动台安装信息兼容

此为用户新增能力，vendor/photag-source没有程序启动台对应项；映射记于source-feature-audit.md末尾。现有DatabaseService创建新增app_launcher_metadata表并同步浏览模式持久库；无既有媒体/标签列变更。`launcher_metadata_upgrades_old_database_and_preserves_user_edits_across_restart`通过：无metadata表的前版启动台库兼容打开，自动发现保留自定义名称/分类/隐藏，metadata和信息编辑跨重启保留，删除关联清理。Database10项回归通过；UI/native图标/排序与同进程样例启动证据见validation-ui/2026-10-17-program-launcher/README.md。安装时间/大小是Windows注册值，缺失不伪造。
# 2026-10-18 收藏分组兼容性

## 2026-10-18 普通浏览版本检查的无关写入失效

用户静置刷Lore统计日志后，追踪发现可见卡片的未保存提示会被动查询版本；缓存键又误用
全库写入修订。修复前，无History操作的剪贴板写入+重绘令日志95→127；移除无关修订
后3次同类写入均95不变，真实文件监听仍触发Modified和127，创建版本后Saved并再次静止。
未屏蔽统计日志。单文件/82文件静置本身没有复现无限增长，不宣称已排除用户现场所有
背景触发。Rust1.97.1 locked build、fmt、FileManager24/Versions5、治理守卫、diff检查通过。
完整同进程MCP命令/范围见[运行记录](validation-ui/2026-10-18-version-idle/README.md)。

源 folders 表及 add/list/remove_folder 查询已追踪，本轮不修改媒体或登记库表。
旧登记根作为快捷方式导入 Settings 的收藏分组；删除收藏不调用会删除媒体索引的
remove_folder。新增 imported_library_paths 为 serde-default/空值跳过字段，以规范化路径
避免 SQLite ID 重用误判。旧 JSON、重复与别名、移除后恢复、新路径复用旧 DB ID 的回归
通过，同进程 MCP 验证移除快捷方式前后库登记相同及重启不复活。完整命令/证据/边界见
[收藏分组记录](validation-ui/2026-10-18-favorite-groups/README.md)。

# 2026-10-18 统一主题：迁移语义边界

强调色名仍沿用源 `settings.rs` 的字符串序列化和默认函数模式；用户要求的新默认值由
UI Bridge 提供工作台蓝，已保存 System/旧预设保留。UI-only 变更不改数据库 schema、
服务 payload、事件顺序、标签颜色或媒体文件。映射见 `docs/source-feature-audit.md`。
当前界面验收见 `docs/validation-ui/2026-10-18-unified-theme/README.md`，不以旧 egui
视觉为验收标准，不把静态主题守卫当作全部运行状态的证明。
