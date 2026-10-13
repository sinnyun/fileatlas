# Validation report

## Current status index (2026-10-13)

The latest recorded consolidation and source-parity pass reports
`cargo +1.97.1 fmt --all -- --check`,
`$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p photag-shell --locked -j 2`, and `git diff --check` passed. The prescribed shell check compiles the host dependency graph; it does not establish full plugin behavior, Windows runtime behavior, or complete source parity. Tests were not run in that final pass under the then-applicable implementation-pass instructions. Earlier sections below are dated snapshots and may describe features as absent or checks as blocked before later work; use the latest entry and [`source-feature-audit.md`](source-feature-audit.md) for current migration status.

Outstanding evidence includes executable regression specifications for source
menu/confirmation behavior, settings defaults, database query semantics and
cascades, file-operation rollback, watcher reconciliation, preview toolbar
configuration, and control-level visual behavior. Windows UI/system
integration, representative media coverage, updater replacement/restart, and
real PhoTag library compatibility also require their respective environments
and fixtures.

## Historical snapshot: preview, theme, and navigation integration pass (2026-10-13)

| Check | Result | Notes |
|---|---|---|
| `cargo fmt --all -- --check` | Passed with Rust 1.97.1 | Invoked as `cargo +1.97.1 fmt --all -- --check`; the default toolchain is older than the workspace MSRV. |
| `cargo check -p photag-shell --locked` | Passed with Rust 1.97.1 | Invoked as `cargo +1.97.1 check -p photag-shell --locked`, with `CARGO_INCREMENTAL=1` and `CARGO_BUILD_JOBS=2`; shell dependency graph compiled after adding the preview crate to workspace and lockfile. |
| Default-toolchain `cargo check -p photag-shell --locked` | Blocked before compilation | Active rustc 1.94.1 is below workspace MSRV 1.95. |
| Plugin/workspace tests | Not run | Reserved for the user's second-round concentrated debugging as directed by `AGENTS.md`. |
| Windows appearance and media UI | Unverified | Linux/host compilation does not validate HWND effects, Windows shell behavior, real media decoding, or visual parity. |

This pass adds a Cordis image preview service and GPUI surface, wires Gallery
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
| `cargo check -p photag-shell --locked` | Passed with Rust 1.97.1 | Invoked as `cargo +1.97.1 check -p photag-shell --locked`, `CARGO_INCREMENTAL=1`, `CARGO_BUILD_JOBS=2`; includes the registered history, media-playback, and drag-drop crates and Gallery bulk actions/settings key capture. |
| Plugin/workspace tests | Not run | Reserved for the user's second-round concentrated debugging as directed by `AGENTS.md`. |
| Windows runtime behavior | Unverified | OLE drag-out, playback process behavior, native dialogs, visual parity, and filesystem interactions still need real Windows validation. |

This pass adds source-derived filter-driven bulk actions, a persisted settings
open-key capture control, a Cordis undo/redo handler service, an FFmpeg video
playback service/view, and a Windows CF_HDROP drag-out service. History mutation
callers and keyboard dispatch, the configured open-key consumer, Gallery video
handoff, and the Gallery drag gesture remain unwired. Audio output is not
migrated. The AI and updater crate drafts remain outside the workspace because
their new external dependencies are not represented in the pinned lockfile;
they are not shell features yet.

## Historical snapshot: session restore migration pass (2026-10-12)

| Check | Result | Notes |
|---|---|---|
| `cargo fmt --all -- --check` | Passed | Workspace formatting check after gallery and navigation session wiring. |
| Initial `cargo check -p photag-shell --locked -j 2` | Failed before compilation | Cargo required lockfile updates for the new local gallery/navigation dependencies; their workspace package edges were added to `Cargo.lock`. |
| `cargo check -p photag-shell --locked -j 2` after lock update | Blocked by toolchain | Active `rustc 1.94.1` is below workspace MSRV 1.95. |
| `cargo +1.97.1 check -p photag-shell --locked -j 2` | Passed | Windows x86_64 MSVC host; compiled shell dependency graph. The first compile exposed and fixed the Windows `Command::creation_flags` mutable binding in indexing. Existing warnings remain in session/settings publication handling and metadata-export import. |
| Plugin/workspace tests | Deferred | Per repository verification instructions, tests remain reserved for the user's second-round concentrated debugging. |

The session migration restores/persists gallery query filters and restores
validated navigation selection and directory expansion. This compile does not
verify behavior in a running Windows GUI or against an actual PhoTag library.

## Historical snapshot: cloud task pass (2026-10-12)

This pass is in code-generation/documentation mode. Per the current task scope,
plugin tests are deferred to the second-round concentrated debugging phase.
The active interactive tool environment has no `rustc`, `cargo`, or `rustfmt`
on `PATH`. GitHub Actions run 20 on commit
`49771e11c9cd5f9cd056284cf5266fbd7eaf8d39` passed formatting and
`cargo check -p photag-shell --locked`. That build compiles the configured
Cordis plugins as Shell dependencies, including the file-operation and tagging
crates. Plugin tests remain deferred; this Linux compile does not validate
Windows runtime behavior or feature parity.

GitHub Actions run 24 passed formatting and `cargo check -p photag-shell
--locked` for the selected-media details implementation. The new EXIF parser
and lazy field display have now passed formatting and `cargo check -p
photag-shell --locked` in run 27. Plugin tests remain deferred; Windows runtime
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
`cargo check -p photag-shell --locked`. Plugin tests remain deferred by task
direction; Windows UI, filesystem, Shell, drag/drop, and Recycle Bin behavior
remain unverified.

| Check | Result | Notes |
|---|---|---|
| Rust toolchain availability | Blocked | `command -v cargo` and `rustc --version`; neither executable is installed in this active environment. |
| `cargo fmt --all -- --check` | Not run | Requires rustfmt; no Rust toolchain is installed. |
| Local `cargo check -p photag-shell --locked` | Not run | No local Rust toolchain in the active interactive environment. |
| GitHub Actions: latest base shell check | Passed | Commit `49771e11c9cd5f9cd056284cf5266fbd7eaf8d39`, run 20; formatting and `cargo check -p photag-shell --locked` both passed. Plugins are compiled as dependencies, not tested. |
| GitHub Actions: selected-media detail migration | Formatting failed; compile skipped | Commit `976851250f7d276141bb4db87583c38564a252a9`, run 21; three rustfmt diffs in `view/card.rs` were corrected locally. A fresh CI run is pending. |
| GitHub Actions: selected-media detail formatting correction | Formatting passed; compile failed | Commit `050328d1be3e4c4528c0c29fec3c813815886e7f`, run 22; `open_count` reader closure needed an `Ok(...)` wrapper. The compiler-reported fix is applied locally; rerun pending. |
| GitHub Actions: open-count reader fix | Formatting failed; compile skipped | Commit `3bae0c2005d9cc700a636dee9b5569217099b9c0`, run 23; rustfmt requested line wrapping for the new `Ok(...)`. Corrected locally; rerun pending. |
| GitHub Actions: selected-media metadata detail | Passed | Commit `34a9386ec8ee1a4a249a745987f8066a5c253451`, run 24; formatting and `cargo check -p photag-shell --locked` passed. Plugin tests were not run. |
| GitHub Actions: lazy EXIF field display | Formatting failed; compile skipped | Commit `2a929d79a8c68aa42bfb564533389a8932335ef8`, run 26; one line-wrap difference in the error display arm is corrected locally. The EXIF dependency and lockfile have not yet reached compilation. |
| GitHub Actions: lazy EXIF field display correction | Passed | Commit `490b9dfee985831ea2f67e3b8f8616c29affd692`, run 27; formatting and `cargo check -p photag-shell --locked` passed with the new EXIF dependency. Plugin tests were not run. |
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
| Prior GitHub Actions: `cargo check -p photag-shell --locked` | Passed | Remote commit `0940e41024d021f7883be387f9400469d20bff7d`; validates only that earlier tree. |
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
| `cargo fmt --all -- --check` | Passed | After formatting the gallery active-filter row |
| `git diff --check` | Passed | No whitespace errors |
| `cargo test -p photag-plugin-gallery --locked --offline` | Passed, 13 tests | Includes per-extension clearing from both include/exclude filter states |
| `cargo test --workspace --exclude photag-shell --locked --offline` | Passed, 28 tests | Linux x86_64; includes database, gallery, indexing, navigation, tagging, thumbnails, watcher, and UI bridge tests |
| `cargo check -p photag-shell --locked --offline` | Passed | Includes GPUI Shell and gallery view compilation |
| `cargo +1.95.0 check -p photag-shell --locked --offline` | Passed | Confirms declared minimum Rust toolchain compiles the shell |
| `cargo +1.95.0 test --workspace --exclude photag-shell --locked --offline` | Passed, 28 tests | Confirms plugin tests run on the declared minimum Rust toolchain |

The checked-in `Cargo.lock` at starting commit `29d3343` began with a Cargo
truncation warning and could not be parsed. It was regenerated from the
workspace manifests with Cargo 1.99.0 before these checks. A separate
`cargo +1.88.0 check -p photag-shell --locked --offline` failed because the
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
| PhoTag clone and pinned revision | Passed | `36cee242a78c0a8cd9ac6b0d63b64265aa0add90` |
| `cargo check -p photag-shell --offline` | Passed | Linux x86_64; compiles the GPUI Kit host and Cordis runtime composition |
| `cargo fmt --all` | Passed | Stable toolchain with rustfmt |
| `cargo test --workspace --exclude photag-shell --offline` | Passed, 22 tests | 3 database, 7 gallery data/filter/selection/lifecycle, 4 navigation, 1 tagging, 1 indexing, 1 thumbnails, 2 watcher, and 3 UI bridge tests |
| `cargo check -p photag-shell --offline` | Passed | Includes GPUI view factories, slot mounting, and all registered runtime plugins |
| `cargo check -p photag-plugin-gallery --offline` | Passed | Includes GPUI cached-thumbnail image element integration |
| `cargo test --workspace --offline` | Failed at GPUI archive creation | Rust reports `failed to map object file: memory map must have a non-zero length` when creating a `gpui_component` rlib after the thumbnail dependency was added; package tests and shell check pass separately |
| `cargo test -p photag-plugin-database -p photag-ui-bridge --offline` | Passed, 6 tests | Fresh schema, pre-migration library compatibility, Cordis database lifecycle, and UI owner cleanup tests |
| `cargo test -p photag-plugin-thumbnails --offline` | Passed, 1 test | Original image pipeline creates and reuses a cached thumbnail |
| `cargo test -p photag-plugin-indexing --offline` | Passed, 1 test | Recursive folder scan indexes a still image, ignores a hidden file, pairs a video poster and writes thumbnail cache output |
| `cargo test -p photag-plugin-watcher --offline` | Passed, 2 tests | Extension classification and Linux filesystem create/remove events reconcile to SQLite and dispatch Cordis media-index change events; Windows-specific notification behavior remains untested |
| `cargo test -p photag-plugin-gallery --offline` | Passed, 12 tests | Confirms cached-window shifts, 200-row page loading, UI lifecycle cleanup including Cordis folder/tag/path selection and index events, source-equivalent filter mapping, search-scope toggling, date input parsing, inclusive rating-range bounds, include/exclude extension cycling, pointer plain/Ctrl/Shift selection, keyboard navigation/select-all/rating/delete state effects and persisted rating clamping; GPUI media-type, sort, rating, date, search-scope, extension, clear-filter and selected-item detail rendering compile through shell check |
| `cargo test -p photag-plugin-navigation --offline` | Passed, 4 tests | Folder/tag counts, source tag mutations, folder add/rescan/remove scan lifecycle, and sidebar contribution cleanup; dot-directory filtering and tag color format covered |
| `cargo test -p photag-plugin-tagging --offline` | Passed, 1 test | Assignment expands associations and ancestors with source manual/automatic flags; remove and clear paths covered |
| Git bundle clone | Passed | Cloned delivery bundle to a clean directory and inspected its root commit |
| GPUI/GPUI Kit integration compile | Passed on Linux | Windows native GUI execution is not available here |
| PhoTag regression tests | Partial | No end-to-end UI parity test suite; existing tests cover plugin lifecycle and basic DB setup |
| Windows GUI / Shell / file operation tests | Not run | Requires the user's Windows environment |

## Outstanding validation issues

- The gallery test covers synthetic database rows only and does not render GPUI. The scanner and folder-action tests cover synthetic fixtures only. The watcher tests cover classification and Linux native create/remove notifications, but do not exercise Windows notification delivery. Additional regression tests are needed for large batches, supported media formats, FFmpeg video extraction, real filesystem events, rename/delete reconciliation, and error handling.
- Windows GUI, Windows Shell, drag/drop, Recycle Bin, and real-file operations
  remain unverified. A synthetic pre-migration SQLite fixture passes; a copy of
  an actual PhoTag user library has not been tested.
- The first Linux workspace test attempt failed to link because `libxcb` and
  `libxkbcommon` development files were absent. After installing those system
  packages, the complete workspace test command passed.
- After the thumbnail crate added image decoding dependencies, a fresh full
  workspace test attempt hit the GPUI archive mapping error listed above.
  `cargo test --workspace --exclude photag-shell --offline` and
  `cargo check -p photag-shell --offline` both pass independently.

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
| `cargo check -p photag-shell --locked --timings` | Not run to compilation | Default `stable` is Rust 1.94.1; Cargo rejected all workspace crates because they declare Rust 1.95. |
| `cargo +1.96.0 fmt --all -- --check` | Passed | Windows MSVC host toolchain. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; $env:CARGO_PROFILE_DEV_DEBUG='0'; cargo +1.96.0 check -p photag-shell --locked --timings` | Passed, 2m 54s | Windows MSVC, first complete dependency build for this toolchain; 681 dirty units, GPUI and registered Shell plugin dependency graph. Timing report: `target/cargo-timings/cargo-timing-20261012T155111622Z-0de67308caf8ada2.html`. |
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
windows@0.62.2 -p photag-shell --locked` traces the Windows API feature set to
the GPUI Windows backend and GPUI Kit dependency path. Application-level
feature trimming is not established as safe; validate upstream feature
requirements and Windows UI coverage before attempting it.

## Windows development launcher and staged plugin startup (2026-10-13)

`start-dev.bat [plugin-list]` checks `photag-shell`, then runs the native GPUI development window. It takes no port argument because this desktop application has no development HTTP port.

The shell enables `settings`, `session`, and `database` by default, with the UI bridge published by the host. `PHOTAG_DEV_PLUGINS` defaults to `core`. Pass a comma-separated plugin name as the optional argument (or set the environment variable) to enable plugins incrementally. `gallery` brings in file operations; `navigation` brings in thumbnails, indexing, and watcher; `indexing` brings in thumbnails; `watcher` brings in indexing and thumbnails. `all` enables all feature plugins. Supported names: `gallery`, `navigation`, `metadata-export`, `file-operations`, `thumbnails`, `indexing`, `watcher`, `tagging`, `history`, `media-playback`, and `drag-drop`. Preview starts with Gallery; Settings and Session start in core.

| Check | Result | Notes |
|---|---|---|
| `cargo fmt --all -- --check` | Passed | Windows MSVC toolchain 1.97.1. |
| `cargo check -p photag-shell --locked -j 2` | Passed | Windows MSVC; incremental shell dependency graph. Existing warnings: session `ServicePublication` and metadata-export unused import. |
| `start-dev.bat` | Started | No port prompt; shell compile check passed; current `photag-shell.exe` is responsive with a nonzero main-window handle. CUA did not expose native apps for visual inspection. |
| Plugin tests | Deferred | Per repository verification instructions. |

To progressively enable a feature, pass its name, for example `start-dev.bat gallery` or `start-dev.bat navigation`.

Some initial runs exited with a GPUI debug panic (`Duplicate a11y node id`) during first render; this can recur and is still unresolved. Other direct and batch-file runs remained responsive with a nonzero window handle, including the current no-argument batch-file launch. Capture a full backtrace and isolate the view as plugins are enabled incrementally.

## Parallel migration follow-up (2026-10-13)

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 fmt --all -- --check` | Passed | Windows MSVC toolchain. |
| `cargo +1.97.1 check -p photag-shell --locked` | Failed during lock resolution | Newly added native dialogs and EXIF/date dependencies bring a transitive graph that conflicts with pinned versions (currently `futures-util` 0.3.32 vs 0.3.34). No Rust compilation was reached. Lock graph requires a coherent regeneration/reconciliation before this migration is validated. |
| Tests | Not run | Repository instructions reserve tests for the concentrated second-round debugging pass. |
| Windows UI/native dialogs/OLE | Unverified | Requires real Windows runtime validation. |

## Lockfile reconciliation and focused second-round debugging (2026-10-13)

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 update` | Passed | Reconciled the workspace lock graph with current manifests and the workspace Rust 1.95 MSRV. Removed orphaned lock records from the imported dialog dependency closure and unified duplicate transitive versions. |
| `cargo +1.97.1 fmt --all -- --check` | Passed | Windows MSVC toolchain. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p photag-shell --locked` | Passed | Windows MSVC; full shell dependency graph compiled after fixing missing GPUI traits, the settings navigation weak-reference type, metadata path input conversion, and GPUI click-count access. |
| Focused tests for preview, settings, gallery, navigation, metadata-export, history, media-playback, and drag-drop | Passed on rerun | 17 unit tests passed (13 Gallery, 4 Navigation); remaining selected crates have no unit tests. The initial Navigation lifecycle test exposed a missing SessionService test fixture, which was added before rerunning. All selected doc-tests passed (zero defined). |
| `PHOTAG_DEV_PLUGINS=gallery cargo +1.97.1 run -p photag-shell --locked` | Passed twice after UI ID fix | After moving Gallery after its providers and prefixing Preview/MediaPlayback button IDs, two consecutive starts kept a responsive main window with a nonzero HWND for over 40 seconds each and closed cleanly. Earlier launches reproduced a duplicate `Close` accessibility node and `0xc0000409`; plugin-specific button IDs removed that failure in the two follow-up launches. Test data used a temporary `LOCALAPPDATA` directory. |
| Windows UI/OLE/dialog/watcher interactions | Partially verified | Native UI automation is unavailable in this environment. The Gallery window starts and responds, but visual inspection, Explorer drag, native dialog interaction, and filesystem notification exercise were not performed. |

The shell provider-order audit found that Gallery was started before MediaPlayback, Tagging, and History despite requiring or optionally consuming those services. Startup now starts Gallery after its providers, and shutdown stops Gallery and History before those dependencies. The duplicate accessibility node came from both Preview and MediaPlayback using the same `Close` button ID; plugin-specific button IDs fixed it in two consecutive Gallery profile launches.

## MCP integration (2026-10-13)

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 update --offline` | Passed | Reconciled the lockfile from the cached registry for the official `rmcp` 3.5.1 SDK and current workspace manifests. |
| Initial `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p photag-shell --locked -j 2` | Failed, then fixed | The first compile found the Windows `PrintWindow` API under the XPS module and bulk outcome types without serialization. The adapter now calls the correct Windows API and maps operation outcomes into explicit JSON fields. |
| `cargo +1.97.1 fmt --all -- --check` | Passed | Final formatting check. |
| First MCP-integrated `cargo +1.97.1 check -p photag-shell --locked -j 2` | Passed before later MCP/workspace edits | Windows MSVC shell dependency graph, including the MCP tools and native screenshot code at that point. The `export_metadata` tool and later workspace edits were added afterward, so they are not covered by this passing result. |
| `cargo +1.97.1 fmt --manifest-path crates/photag-plugin-mcp/Cargo.toml -- --check` | Passed | MCP crate formatting. |
| Concurrent-edit `cargo +1.97.1 fmt --all -- --check` (intermediate) | Blocked, later resolved | Gallery referenced a module before it was created and Preview had unformatted changes. The final formatting check after integration and repair passed in the section below. |
| Earlier `cargo +1.97.1 fmt --all -- --check` | Passed | Passed before the latest concurrent workspace edits. |
| MCP-era `cargo +1.97.1 check -p photag-shell --locked -j 2` (intermediate) | Failed, later resolved | At that point Gallery lacked the scroll-tracking API and Updater emitted warnings. The API integration was corrected; final shell check passed in the section below. |
| Tests | Not run | Followed the repository code-generation pass restriction. |
| MCP client launch and live screenshot fidelity | Unverified | The `--mcp` stdio path and `PrintWindow(PW_RENDERFULLCONTENT)` implementation compile. A live MCP client round trip and visual comparison of the returned image against the GPUI window still require Windows runtime validation. |

## Parallel migration and repair (2026-10-13)

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 update --offline` | Passed | Reconciled the lockfile for updater, Rodio, native dialogs, and the workspace-registered AI crate using the cached registry. |
| `cargo +1.97.1 fmt --all -- --check` | Passed | Final workspace formatting check after the parallel implementation and repair rounds. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p photag-shell --locked -j 2` | Passed | Final Windows MSVC shell dependency graph compiled with AI, updater, playback, native dialogs, and UI changes. Earlier runs exposed and then fixed Navigation menu closure ownership, Settings install-path ownership, History event argument inference, Gallery imports/dialog APIs/snapshot borrowing, and Shell Cordis context ownership. |
| Tests | Not run | Per the repository's code-generation pass restriction. |
| Windows user-interface, model download/inference, updater replacement/restart, audio device, Explorer and Recycle Bin behavior | Unverified | These require interactive Windows validation and representative media/user-library fixtures. |


## Parallel remaining-feature migration and consolidation (2026-10-13)

Ten feature work packages were developed by subagents, followed by a separate
consolidation and repair pass. The pass advances Gallery viewport virtualization,
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
| Initial `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p photag-shell --locked -j 2` | Failed, then corrected | First required lockfile reconciliation. Subsequent compiler diagnostics found GPUI drag/drop trait/stateful-ID/type annotations, Preview fluent-builder imports and asynchronous callback captures, Gallery uniform-list scrolling, disabled-button trait scope, and render-image frame construction. Owning agents fixed them. |
| Final `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p photag-shell --locked -j 2` | Passed | Windows MSVC shell dependency graph compiled successfully with the integrated feature changes. |
| Plugin/workspace tests | Not run | Per repository instructions, tests remain reserved for a separate concentrated debugging pass. |
| Windows visual, shell, media, and filesystem behavior | Unverified | The compile does not validate UI interactions, model download/runtime inference, native dialogs, OLE, Recycle Bin, real-media metadata, or filesystem races. |

The integrated Gallery supports rubber-band selection, AI Suggest Tags card
actions, sidebar-tag drop assignment, and card/Preview action menus. Preview
now renders audio with filename, tags, rating and default-player action; its
filmstrip follows image/video/audio query order, and video opens through
MediaPlayback. Windows visual/native behavior—including pointer interactions,
dialogs, OLE, Recycle Bin, media devices, real-media metadata and filesystem
reconciliation—remains unverified.

## Migration audit and documentation reconciliation (2026-10-13)

The feature-to-source table in `docs/source-feature-audit.md` is the canonical
migration status. The older duplicated implementation-slice summary was removed
because it no longer reflected the latest Gallery, Navigation, AI, updater,
Preview, and Settings work. Crate `PORTS.md` coverage was checked for all current
workspace crates.

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 fmt --all -- --check` | Passed | Ran against the current workspace after the latest migration commits. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p photag-shell --locked -j 2` | Passed | Ran against the current workspace; no compiler warnings were emitted. |
| Tests | Not run | This documentation reconciliation does not change implementation code. The latest feature migration test pass remains deferred as described above. |
| Windows/MCP runtime validation | Still required | MCP client round trips, screenshot fidelity, model inference/download, updater replacement, media-device behavior, Explorer/OLE, Recycle Bin, native dialogs, and watcher race behavior require focused Windows validation. |

MCP mutations are disabled by default and require the environment opt-in.
Selected high-impact tools also require `confirm: true`; this acknowledges the
call but does not prove a human reviewed it. Configure a trusted MCP client to
request user confirmation before mutation, and do not treat tool descriptions
as an application-enforced safety boundary.

## Consolidation repair and source parity review (2026-10-13)

This pass compared changed Gallery, Navigation, Preview and Settings behavior
with the pinned source and repaired stale parity notes. It added a shared typed
`TagDragPayload` so dropping a Navigation tag onto a Gallery card assigns that
tag to the card or its current selection. Gallery and Navigation view code was
split into behavioral/rendering submodules to keep each Rust source file below
800 lines. The source feature audit now reflects the implemented rubber-band
selection, AI Suggest Tags, context actions, drag assignment and calendar bulk
date controls. Source behavior that does not exist, such as a pick/reject action
handler, remains identified as absent in the source.

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 update --offline` | Passed | Reconciled the local workspace dependency edges without changing package versions. This was required before the prescribed `--locked` check would run. |
| Initial `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p photag-shell --locked -j 2` | Failed, then repaired | First rejected the stale lock file; after lock reconciliation, found missing Navigation event type/FluentBuilder declarations, Settings/Preview GPUI async closure arity, Preview `ImageDecoder` trait import/mutability, and Gallery event payload annotations. |
| Final `cargo +1.97.1 fmt --all -- --check` | Passed | Ran after Gallery/Navigation module splits and source parity changes. |
| Final `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p photag-shell --locked -j 2` | Passed | Windows MSVC host shell dependency graph compiled successfully after the repairs. |
| Tests | Not run | Per the repository code-generation pass restriction. |
| Windows UI/media/filesystem behavior | Unverified | Build success does not verify native drag/drop, selection geometry, menus, playback devices, filesystem operations or full-format media behavior. |


### Audio Preview and full media sequence (2026-10-13)

Source review of `vendor/photag-source/src/ui/preview.rs` confirmed the audio
stage, filename, default-player action, tag chips, 0–5 rating, close and arrow
controls. Preview now provides the audio stage and routes the default-player
action through File Operations. Gallery supplies the complete ordered
image/video/audio sequence; arrows navigate across media kinds, and video
entries hand off to Media Playback. Audio/video image-only transforms are not
shown as content behavior. Windows media-device and visual interaction behavior
remains unverified.

### Final post-audio verification (2026-10-13)

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 fmt --all -- --check` | Passed | Ran after adding Audio Preview and splitting Preview toolbar support. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p photag-shell --locked -j 2` | Passed | Incremental shell dependency check completed successfully after the final media-sequence changes. |
| Tests | Not run | Prohibited during this migration pass by the workspace instructions. |
| Windows UI/media/filesystem behavior | Unverified | This compile does not validate native focus/navigation, default-player launch, playback devices, dialogs, or visual parity. |

### Preview final split compile repair (2026-10-13)

| Check | Result | Notes |
|---|---|---|
| Initial `cargo +1.97.1 check -p photag-shell --locked -j 2` | Failed, then repaired | The Preview module split left `DatabaseService` and `SessionService` imports out of scope and omitted `StyledExt` for `font_medium`; added the imports to the owning modules. |
| `cargo +1.97.1 fmt --all -- --check` | Passed | Ran after import fixes. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p photag-shell --locked -j 2` | Passed | The shell dependency graph compiled successfully after Preview repair. |
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
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p photag-shell --locked -j 2` | Passed | Final Windows host shell dependency graph compiled after this check exposed and the pass repaired type/import/ownership errors in the contributed UI paths. |
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
`AppState::process_pending`, `show_gallery`, `show_tag_node`, and `show_preview`
were used to navigate source call paths, then relevant destination code was
read directly. The detailed feature-family mapping and remaining caveats are in
`docs/source-feature-audit.md`.

Direct source review found two Gallery parity defects: Enter stopped previewing
when Settings was available, and multi-selection had no visible Preview-first
action despite the source exposing Open for a non-empty selection. Both were
fixed in `crates/photag-plugin-gallery/src/view/keyboard.rs` and `view/render.rs`.
The same review recorded one intentional difference: after AI automatic
inference fails or disconnects, the migrated Preview marks the image analyzed
to avoid a tight automatic retry loop; the source leaves it eligible for retry.
Manual Analyze remains available. This is a behavior difference, not an absent
feature.

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 fmt --all -- --check` | Passed | Run after the Gallery parity repairs. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p photag-shell --locked -j 2` | Passed | Incremental shell dependency graph check after the Gallery parity repairs. |
| `git diff --check` | Passed | No whitespace errors at the time of the final review; line-ending normalization warnings may appear on Windows for edited Markdown files. |
| Tests | Not run | Per repository instruction, reserve tests for the user's second-round concentrated debugging. |
| Equivalence / runtime | Not established | Graph correspondence and successful compilation do not prove every source branch or Windows behavior. Full SQL, native UI, media corpus and system integration still need focused validation. |

### Second-round concentrated debugging (2026-10-13)

This is the second-round test pass requested after the implementation/audit
round. It adds Gallery keyboard regression coverage, legacy-database reopen and
query behavior checks, common still-image decoder coverage, and a separately
run FFmpeg-generated video thumbnail smoke test.

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 test --workspace --locked -j 2` | Passed | 42 passed, 0 failed, 3 ignored. Ignored tests are two live network checksum tests and an FFmpeg smoke test; the FFmpeg smoke test was run separately and passed below. |
| `cargo +1.97.1 test -p photag-plugin-indexing --locked extracts_a_thumbnail_from_a_valid_generated_video -- --ignored --nocapture` | Passed | On Windows MSVC with FFmpeg 8.1 available, generated a valid short MP4 and extracted a JPEG thumbnail through the migrated indexing FFmpeg helper. |
| Gallery regression tests | Passed | 15 tests, including Enter preview while a configured open key is active and Enter opening the first selected item in selection order. |
| Database tests | Passed | 4 tests, including pre-migration library preservation/defaults after upgrade and reopen, plus representative source filter/sort behavior against temporary SQLite data. |
| Thumbnail / indexing tests | Passed | Synthetic JPEG, PNG, WebP, TIFF, BMP thumbnail generation; image/video-poster pairing and hidden-file filtering. These are generated fixtures, not a representative real-media corpus. |
| `cargo +1.97.1 test -p photag-plugin-file-operations --locked` | Passed | 3 Windows temporary-directory tests verify rename/index update, move within a registered library, move outside the library while retaining the file and removing the row, and collision leaves both source and index unchanged. |
| Windows watcher test | Passed | The workspace run executed native file create/remove reconciliation against the Windows filesystem. Rename delivery and harder race cases remain unverified. |
| Native GUI, shell operations and real library corpus | Unverified | This headless test run did not exercise live GPUI focus/dialogs, Explorer OLE, Recycle Bin, audio devices, updater replacement, real PhoTag libraries, or diverse camera EXIF files. |

### Continued source-parity regression checks (2026-10-13)

Direct inspection compared `vendor/photag-source/src/xmp.rs` with the migrated
`crates/photag-plugin-tagging/src/xmp.rs`; the writer logic remains source
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
| `cargo +1.97.1 test -p photag-plugin-tagging --locked xmp::tests -- --nocapture` | Passed | 2 isolated temporary-file tests. |
| `cargo +1.97.1 test -p photag-plugin-database --locked folder_and_tag_foreign_keys_cascade_through_related_rows -- --nocapture` | Passed | Exercises foreign-key cascades on a disposable database. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 test --workspace --locked -j 2` | Passed | 47 passed, 0 failed, 3 ignored (two live network checksum tests and the optional FFmpeg smoke test, which passed separately in the previous section). |
| `cargo +1.97.1 test -p photag-plugin-tagging --locked failed_background_write_is_visible_and_can_be_cleared -- --nocapture` | Passed | A missing temporary media file drives the actual worker error branch; the failure is observable and acknowledgeable. |
| `cargo +1.97.1 test -p photag-plugin-tagging --locked queued_writes_for_a_path_coalesce_to_the_latest_tags -- --nocapture` | Passed | A channel gate holds the first write while two newer states are scheduled; the queued middle state is discarded and the last state is written. |
| `cargo +1.97.1 fmt --all -- --check` | Passed | Workspace formatting check after the deterministic worker test. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p photag-shell --locked -j 2` | Passed | Host shell dependency graph compiles after the writer seam and test changes. |
| `git diff --check` | Passed | No whitespace errors. |
| Real camera/editor interoperability | Not tested in this slice | Requires representative photos and Windows applications; the deterministic queue test is not a substitute for metadata interoperability. |

### Development media library and scale-test plugin (2026-10-13)

Added `test-data/photag-library/` with 11 generated files (about 6.32 MB):
JPEG, PNG, WebP, GIF, TIFF, BMP, WAV, an intentionally invalid JPEG, nested
directories, and a hidden support thumbnail. The opt-in
`photag-plugin-dev-test` panel uses the production indexer to scan the checked-in
folder and transactionally injects/replaces/cleans 1K, 10K, or 100K virtual
media rows for Gallery and SQLite load checks. Virtual rows do not create files.
The dev-test profile redirects database, settings, session and thumbnail cache
to `target/dev-test/` by default; it does not touch the regular PhoTag database.

| Check | Result | Notes |
|---|---|---|
| `cargo +1.97.1 run -p photag-plugin-dev-test --example generate_test_media --locked` | Passed | Generated the reproducible local fixture set. |
| `cargo +1.97.1 test -p photag-plugin-dev-test --locked -- --nocapture` | Passed | 2 tests: production scanner indexes nested real fixtures and sees both generated and missing thumbnails; temporary SQLite database replaces 100,000 rows, validates type/thumbnail distribution, then clears all virtual rows without creating physical files. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 test --workspace --locked -j 2` | Passed | 49 passed, 0 failed, 3 ignored (two live network checksum tests and the optional FFmpeg smoke test, separately passed above). |
| `cargo +1.97.1 fmt --all -- --check` | Passed | Workspace formatting check. |
| `$env:CARGO_INCREMENTAL='1'; $env:CARGO_BUILD_JOBS='2'; cargo +1.97.1 check -p photag-shell --locked -j 2` | Passed | Host and opt-in plugin compile together. |
| `git diff --check` | Passed | No whitespace errors. |
| Interactive Windows profile and 100K Gallery scrolling | Not run | Launch with `start-dev.bat dev-test` to inspect real GPUI rendering, database response and scrolling; automated tests do not measure user-visible frame rate. |
