# Lore-backed historical version switching — 2026-10-17

## Implementation

Reference: Lore GUI-backed per-entity history in `F:\AI\geren\newworkspace`,
using `lore-vm` revision `35eaf6ed1273397a5b18a283dda560949bb94ca4`.

FileAtlas now has two Cordis-owned plugins:

- `fileatlas-plugin-versions`: creates a private Lore repository/shared store per
  canonical file path, saves titled revisions, lists revisions and restores a
  saved revision to the original path with rollback-aware replacement.
- `fileatlas-plugin-version-info`: typed facade over the shared DatabaseService;
  legacy `version-info.sqlite3` rows are imported idempotently and that source
  file is retained as a rollback copy.

FileManager's History tab uses keyed background `ReadModel`s for versions and the
Lore branch graph. It supports branch creation, switching, merge, archive,
ours/theirs resolution, and abort for a pending conflict. Switch refuses to
replace the source if it differs from the current Lore snapshot or if a merge
is pending. Successful restore/merge refreshes indexed size/time/type and image
thumbnail while preserving rating, tags, capture date and media row identity.
Same-process MCP exposes read, create, activate and branch lifecycle tools;
writes use the explicit mutation guard and `confirm=true` requirement.

## Validation

Static validation used Rust 1.97.1:

- `cargo +1.97.1 fmt --all -- --check` passed.
- `cargo +1.97.1 check --locked -p fileatlas-shell -j 2` passed after restoring production dependencies used by Navigation and File Operations.
- `cargo +1.97.1 test --locked -p fileatlas-plugin-version-info -p fileatlas-plugin-versions -j 2` passed: metadata persistence/current marker plus Lore branch create/switch/conflict/resolve/archive lifecycle.
- Same-process MCP round trip passed with `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-lore-version-mcp-20261017h --plugins dev-test --allow-mutations --scenario scripts/validate-lore-version-mcp.cjs`. The scenario used an isolated profile and indexed PNG fixture to check tool registration, explicit confirmation, create/list/restore, branch create/switch/archive, merge conflict visibility, abort without source-byte changes, and `theirs` resolution. Report: `target/mcp-lore-version-mcp-20261017h/lore-version-report.json`.
- FileManager History interaction passed through actual same-process GPUI pointer events: `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-lore-history-click2-20261017 --plugins dev-test --allow-mutations --scenario scripts/validate-lore-history-click.cjs`. The test clicked the History tab, a non-current revision row and Switch version; MCP UI state showed selection/active revision change and the source image bytes matched the selected revision. Screenshot: `target/mcp-lore-history-click2-20261017/screenshots/history-version-switched-by-click.png`; machine-readable evidence: `target/mcp-lore-history-click2-20261017/lore-history-click-setup.json`.
- Pending merge survived a complete child-process restart. First ran `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-lore-pending-restart-20261017 --plugins dev-test --allow-mutations --scenario scripts/validate-lore-pending-restart-prepare.cjs` to create two divergent branch snapshots and leave the conflict pending. After that child exited, ran `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-lore-pending-restart-20261017 --plugins dev-test --allow-mutations --scenario scripts/validate-lore-pending-restart-resume.cjs`. The second process read back the same source/target/conflict paths, aborted the merge, and confirmed source bytes were unchanged. Report: `target/mcp-lore-pending-restart-20261017/pending-restart-report.json`.
- The MCP round trip exposed Lore millisecond timestamps being interpreted as seconds. `parse_time` now accepts seconds, milliseconds and nanoseconds; a regression test covers the millisecond format. This corrected output now reports current dates in the revision history.
- `node scripts/check-ui-refresh.cjs`, `node scripts/check-feature-ledger.cjs`, and `node scripts/check-architecture.cjs` passed after reviewing and updating the architecture baseline. The architecture checker still reports two pre-existing orphan directories: `crates/fileatlas-plugin-media-playback` and `crates/fileatlas-plugin-preview`.
- `cargo +1.97.1 check --locked -p fileatlas-plugin-file-manager -p fileatlas-plugin-mcp -j 2` passed during the UI/MCP integration check.

The MCP server and GPUI window shared each FileAtlas child process. History-tab
navigation, revision selection, switching, and pending-conflict recovery after
restart were exercised through its MCP tools and actual pointer events. Broader
manual interaction coverage and recovery from abrupt termination during an
in-progress Lore write remain unverified.

## Rewind visibility investigation

Direct source comparison with the working reference UI and both plugins is
recorded in [reference-source-trace.md](reference-source-trace.md). Two defects
were found: ancestry-only listing hid newer saved revisions after rewind, and
the browser's temporary empty refresh cleared file selection and replaced
History with the folder inspector. The current repair uses the complete
VersionInfo catalog, stable revision-ID actions, and complete-result selection
validation. MCP now exposes `inspector_history.visible` so hidden stale fields
cannot stand in for a visible History page.

The actual-file scenario accepts `FILEATLAS_LORE_TEST_FILE`, uses that exact source
path for snapshots/restores, compares bytes with the actual original image,
and restores starting bytes/timestamps in `finally`. It verifies both directions,
retained version order/current flags, visible file selection and settled reload
counts. Earlier allocation errors came from a failing multi-megabyte Buffer
diff against the generated fixture; they do not establish an application or MCP
memory fault.

Final validation passed on Windows / 150% DPI with Rust 1.97.1:

- `cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-versions -p fileatlas-plugin-version-info -p fileatlas-plugin-file-manager`: 27 tests passed (23 FileManager, 1 VersionInfo, 3 Lore versions), including older→newer activation and persistent metadata/current-marker coverage.
- `cargo +1.97.1 build --locked -j 2 -p fileatlas-shell`: passed, retaining the pre-existing LNK4098 runtime-library warning.
- `cargo +1.97.1 fmt --all -- --check`, Node syntax and all three governance checks passed. The architecture baseline was reviewed: its only delta is the FileManager test fixture's public VersionInfo dependency, edges 82→83; debt remains 711. Test fixtures now use public DatabaseService methods and include the real Versions plugin dependency.
- With `FILEATLAS_LORE_TEST_FILE=D:\桌面文件夹\测试\临时文件\00.png`, ran `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-lore-real-file-complete-20261017 --plugins dev-test --allow-mutations --scenario scripts/validate-lore-history-click.cjs`.
- This actual source path was used throughout, without relocating it. Two versions were created; real GPUI pointer events selected the older version and Switch, then the newer row and Switch. Both content comparisons passed, both records stayed visible and ordered, the active marker changed correctly, file selection survived refresh, and History remained visible.
- After settling, FileManager snapshot revision stayed 9→9 across the 1-second quiet sample. Final source SHA-256 matched the starting value: `AC42FDB1715623D1EFFF456EAD8D56DF3F5137CBF94A71065FFC4C1C84E327F5`; original bytes and timestamp were restored.
- Evidence: [round-trip report](../../../.local/validation/artifacts/2026-10-17-lore-version-history/real-file-roundtrip.json), [source restoration](../../../.local/validation/artifacts/2026-10-17-lore-version-history/source-restoration.json), [older version visible](../../../.local/validation/artifacts/2026-10-17-lore-version-history/real-file-after-rewind.png), [newer version visible again](../../../.local/validation/artifacts/2026-10-17-lore-version-history/real-file-roundtrip.png). The accepted Inspector structure is preserved; this is a behavior correction.

## Save after a historical checkout — follow-up

The user sequence 01→02→01→edit→03 exposed a separate shared-store
branch-tip precondition. [Official API analysis](official-api-rewind-save.md)
documents the upstream recovery sequence and the offline explicit-local-tip
adaptation. `prepare_snapshot_commit` now repairs a stale private checkout and
its failed content staging before re-copying/staging the untouched source.

Four Versions/VersionInfo tests passed, including a regression that failed with
the exact user error before the fix, 03 creation, all saved-byte restoration,
and recovery from a deliberately rejected old-style staged commit. Locked Shell
build, fmt, Node syntax and all three governance checks passed.

With `FILEATLAS_LORE_TEST_FILE` set to the original `00.png` path, reran the same
MCP command using profile `target/mcp-lore-edit-after-rewind-20261017`. Actual
01/02 pointer round trip plus 01-edit-create03 and restoration of all three
versions passed; History remained visible and quiet snapshot stayed 22→22.
Original bytes/timestamp were restored, with starting/final SHA-256 equal.
Evidence: [report](../../../.local/validation/artifacts/2026-10-17-lore-version-history/edit-after-rewind.json), [source restoration](../../../.local/validation/artifacts/2026-10-17-lore-version-history/edit-after-rewind-restoration.json),
[three-version screenshot](../../../.local/validation/artifacts/2026-10-17-lore-version-history/edit-after-rewind.png).

## Review follow-up — shared metadata service and bounded state reads

Version metadata now uses typed `DatabaseService` operations in the existing
`fileatlas-tags.db` durable store. A legacy sibling `version-info.sqlite3` is
imported idempotently and kept as a rollback copy. Browser persistence clears
the saved current marker for affected entities before syncing version rows, so
branch activation cannot transiently violate the one-current-revision index.
FileManager limits visible-file Lore state reads to eight concurrent operations;
per-file locks are reclaimed after the last operation/waiter leaves. If a Lore
commit succeeds but its metadata write fails, FileManager state recovers the Lore
HEAD and does not report the file as unversioned.

Validation on Windows with Rust 1.97.1:

- `cargo +1.97.1 test --locked -p fileatlas-plugin-database -p fileatlas-plugin-version-info -p fileatlas-plugin-versions -j 2`: 15 tests passed, including browser-mode current-marker persistence and lost-metadata Lore HEAD recovery.
- `cargo +1.97.1 check --locked -p fileatlas-shell -p fileatlas-plugin-file-manager -p fileatlas-plugin-versions -p fileatlas-plugin-version-info -j 2` and `cargo +1.97.1 build --locked -j 2 -p fileatlas-shell` passed. The existing LNK4098 warning remains.
- `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-lore-version-review-fix2-20261017 --plugins dev-test --allow-mutations --scenario scripts/validate-lore-version-mcp.cjs` passed all four scenarios, including branch switches, merge conflict abort/resolution, and version activation.
- The same-process GPUI click scenario passed using profile `target/mcp-lore-history-review-fix-20261017`; it exercised History selection, restore, edit-after-rewind and three-version byte checks. The settled FileManager snapshot stayed at revision 25→25, with the selected file and History panel retained. Report and screenshot are in that profile's output directory.
- `cargo +1.97.1 fmt --all -- --check`, `node scripts/check-layering.cjs`, `node scripts/check-architecture.cjs`, `node scripts/check-feature-ledger.cjs`, and `node scripts/check-ui-refresh.cjs` passed. The reviewed architecture baseline adds the intentional `fileatlas-plugin-version-info -> fileatlas-plugin-database` edge; existing orphan warnings for `media-playback` and `preview` remain.

## Current limits

- Version identity is based on the canonical absolute path; renaming/moving the
  file starts a different history.
- Branch merge conflicts use whole-file `ours`/`theirs` selection, matching
  this file-scoped history model; manual line-level editing is not exposed.
- Directory snapshots and external edit merge are not supported. Unsaved source
  edits must be saved as a version before switching or merging.
- Atomic replacement and Windows watcher/index refresh require the live MCP
  run above; no Windows behavior is claimed based on compilation alone.
