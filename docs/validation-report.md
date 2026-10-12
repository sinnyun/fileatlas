# Validation report

## Current cloud task pass (2026-10-12)

This pass is in code-generation/documentation mode. Per the current task scope,
plugin tests are deferred to the second-round concentrated debugging phase.
The active interactive tool environment has no `rustc`, `cargo`, or `rustfmt`
on `PATH`. After publication, GitHub Actions on commit
`0940e41024d021f7883be387f9400469d20bff7d` completed the base Shell check
successfully. The settings/session, metadata-export, file-operation, and XMP
writeback changes generated in this pass have not been validated by CI.
Earlier validation below records prior runs and does not validate these changes.

| Check | Result | Notes |
|---|---|---|
| Rust toolchain availability | Blocked | `command -v cargo` and `rustc --version`; neither executable is installed in this active environment. |
| `cargo fmt --all -- --check` | Not run | Requires rustfmt; no Rust toolchain is installed. |
| Local `cargo check -p photag-shell --locked` | Not run | No local Rust toolchain in the active interactive environment. |
| GitHub Actions: latest shell check | Failed at formatting step | Commit `54f266f99866fd351aff602d1561751106181eec`; rustfmt requested standard multiline formatting in the copied settings/session models and two shell calls. Those reported changes are now applied locally; compilation was skipped until formatting passes. |
| GitHub Actions: shell check after formatting corrections | Not created | The formatting correction is published at `44b5199cbb3fd11d2b3fbc631a76c29bbd589525`, but GitHub reports no workflow run or commit status for it. |
| `git diff --check` | Passed | No whitespace errors in the current uncommitted migration changes. |
| Cargo lock dependency-reference audit | Passed | TOML parsed; workspace dependency references resolve uniquely in the edited lockfile. |
| Prior GitHub Actions: `cargo check -p photag-shell --locked` | Passed | Remote commit `0940e41024d021f7883be387f9400469d20bff7d`; validates only that earlier tree. |
| Plugin tests | Deferred | Per task direction, run during second-round concentrated testing. |
| Windows GUI / Shell / file operations | Deferred | Requires the user's Windows environment. |

The requested cloud workflow uses incremental compilation, two build jobs, and
no dev debug symbols. Plugin tests remain deferred. GitHub Actions run 7 on
`54f266f99866fd351aff602d1561751106181eec` failed only in formatting; the reported
formatting was corrected and published in `44b5199cbb3fd11d2b3fbc631a76c29bbd589525`.
GitHub has not created a check for the corrected commit, and there is no local
Rust toolchain, so the current source compilation remains unverified.

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
