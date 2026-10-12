# Validation report

## Preview, theme, and navigation integration pass (2026-10-13)

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

## Parallel feature migration pass (2026-10-13)

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

## Session restore migration pass (2026-10-12)

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

## Current cloud task pass (2026-10-12)

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

A successful compile is not evidence of feature parity. UI bridge, shell, database, thumbnails, and a partial folder-indexing migration are present; most user-visible PhoTag behavior is still unmigrated. Windows GUI, Windows Shell drag/drop, and file operation
behavior need testing. Real user-library compatibility still needs a test with
an actual library copy.

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

The shell enables `settings`, `session`, and `database` by default, with the UI bridge published by the host. `PHOTAG_DEV_PLUGINS` defaults to `core`. Pass a comma-separated plugin name as the optional argument (or set the environment variable) to enable plugins incrementally. `gallery` brings in file operations; `navigation` brings in thumbnails, indexing, and watcher; `indexing` brings in thumbnails; `watcher` brings in indexing and thumbnails. `all` enables all feature plugins. Supported names: `gallery`, `navigation`, `metadata-export`, `file-operations`, `thumbnails`, `indexing`, `watcher`, and `tagging`.

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
