# Validation report

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
| `cargo test --workspace --exclude photag-shell --offline` | Passed, 20 tests | 3 database, 1 gallery data, 1 gallery lifecycle, 2 gallery filter, 2 gallery selection, 4 navigation, 1 indexing, 1 thumbnails, 2 watcher, and 3 UI bridge tests |
| `cargo check -p photag-shell --offline` | Passed | Includes the GPUI Kit shell and all registered runtime plugins |
| `cargo test --workspace --offline` | Failed at GPUI archive creation | Rust reports `failed to map object file: memory map must have a non-zero length` when creating a `gpui_component` rlib after the thumbnail dependency was added; package tests and shell check pass separately |
| `cargo test -p photag-plugin-database -p photag-ui-bridge --offline` | Passed, 6 tests | Fresh schema, pre-migration library compatibility, Cordis database lifecycle, and UI owner cleanup tests |
| `cargo test -p photag-plugin-thumbnails --offline` | Passed, 1 test | Original image pipeline creates and reuses a cached thumbnail |
| `cargo test -p photag-plugin-indexing --offline` | Passed, 1 test | Recursive folder scan indexes a still image, ignores a hidden file, pairs a video poster and writes thumbnail cache output |
| `cargo test -p photag-plugin-watcher --offline` | Passed, 2 tests | Extension classification and Linux filesystem create/remove events reconcile to SQLite; Windows-specific notification behavior remains untested |
| `cargo test -p photag-plugin-gallery --offline` | Passed, 6 tests | Confirms window paging, UI lifecycle cleanup, source-equivalent filter mapping, shift range navigation, select-all, rating and delete effects |
| `cargo test -p photag-plugin-navigation --offline` | Passed, 4 tests | Folder/tag counts, source tag mutations, folder add/rescan/remove scan lifecycle, and sidebar contribution cleanup; dot-directory filtering and tag color format covered |
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
