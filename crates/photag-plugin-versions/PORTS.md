# Lore Versions plugin ports / Lore 历史版本对接

## Lifecycle and dependencies

- Cordis plugin: `photag.versions`; service: `photag/versions` (`VersionsService`).
- Requires `VersionInfoService`, PhoTag `ThumbnailService`, `DatabaseService`
  (used by MCP media-ID routes to resolve the indexed file path), and
  `IndexingService` (refreshes filesystem metadata and thumbnails after an
  MCP restore or content-changing branch action). Shell starts Indexing before
  Versions.
- Lore GUI `lore-vm` is pinned to commit `35eaf6ed1273397a5b18a283dda560949bb94ca4`, matching `F:\AI\geren\newworkspace`.
- Data lives below the active database directory: `versions/worktrees/<entity>/`, `versions/stores/<entity>/shared-store/`, and immutable preview copies in `versions/previews/<entity>/`.
- `VersionsService::storage_path() -> &Path` exposes the active Lore storage root for status and diagnostics; revision metadata itself is stored in the shared PhoTag database service.
- Each regular file has a Lore repository and private shared object store. Lore never receives a remote URL or a user's library root as a repository root.

## Operations

- `file_state(absolute_path) -> FileVersionState` compares source content with immutable saved revisions. States: `unversioned`, `saved` (selected revision), `matches_saved_version` (another stored revision), `modified` (none). It never moves the checkout or current marker. Unstable/missing/unreadable content returns errors. No versions means no Lore open/hash.

- `list(absolute_path) -> Vec<VersionEntry>` uses VersionInfo's complete saved-version catalog in descending revision-number order. Activating an older revision never removes later saved versions from this list. Lore supplies the active HEAD and supplements current-ancestry revisions missing metadata after a partial write.
- `create_version(absolute_path, title) -> VersionEntry` validates a regular file and title, snapshots the file into the Lore worktree, stages/commits it with the title, flushes Lore, then writes metadata and a stable thumbnail copy.
- Saving edits after activating an older revision first reads the current branch's local `latest`. If the checkout is behind it, `prepare_snapshot_commit` unstages only a rejected private `content` payload, syncs the private worktree to that explicit local tip with `reset=true`, then snapshots/stages the original source. This appends a chronological version without deleting 01/02 or overwriting the edited source. A clean unchanged historical source still returns no changes. Conflicts, pending merges and unrelated staged paths are not discarded. An empty sync target is not used because this offline store has no remote.
- `activate(absolute_path, version_id) -> ()` verifies the requested revision and requires both the indexed source and Lore worktree to match the currently saved state. It syncs the requested Lore revision, atomically replaces the original file with rollback on install failure, then updates VersionInfo's current marker.
- `branch_graph(absolute_path, include_archived) -> VersionGraph` projects Lore branches, ancestry stacks, revision histories and any saved pending merge into VersionInfo.
- `create_branch`, `switch_branch`, and `archive_branch` use Lore branch operations. Switching restores the selected branch snapshot and updates active revision metadata; archiving the current branch is rejected.
- `merge_branch` merges into the current branch. Conflicts persist in VersionInfo and leave the library source untouched. `resolve_merge(path, "ours"|"theirs")` resolves the single-file conflict set and restores the merge result; `abort_merge` invokes Lore's merge abort and clears the pending state.
- No network synchronization, shelling out to Git, or directory snapshots are supported. The plugin accepts regular files, matching PhoTag History's media-file scope.
- If a VersionInfo write fails after Lore commit, listing and Gallery status recover the Lore HEAD from the repository, so a missing metadata row does not make a saved file appear unversioned. The next successful snapshot writes normal catalog metadata.
- Per-file async operation locks are removed when no operation or waiter retains them; the service does not accumulate one lock entry for every path ever viewed.
- Gallery requests visible file states with at most eight concurrent Lore reads. Work remains proportional to visible files and saved revisions; it runs outside render and does not form one serial chain across the whole viewport.

## Consumers and event flow

- Gallery reads through a keyed `ReadModel`; explicit create/switch actions run asynchronously outside render.
- After activation or a content-changing branch action, `IndexingService`
  refreshes filesystem-derived size/time/type and image thumbnail while
  preserving rating/tags/capture date. MCP emits `MediaIndexChanged` and
  `MediaVersionHistoryChanged(media_id)`; Gallery reloads the current query
  and invalidates the selected file's History panel.
- MCP `list_file_versions` is read-only. `create_file_version` and `activate_file_version` are high-impact: they require Full in Settings → MCP → AI control and per-call `confirm=true`.

## Plugin-owned MCP tools

`VersionsPlugin` contributes namespaced tools through the host's
`McpToolRegistry`; the plugin retains the Cordis registration guard so stopping
Versions removes the tools from `tools/list` and `tools/call`.

- `versions.list {media_id}` and `versions.file_state {media_id}` read the
  indexed media row first, then call `VersionsService` using its canonical
  path. `versions.list_branches {media_id, include_archived?}` reads the
  Lore/VersionInfo branch graph.
- `versions.create {media_id, title}` and `versions.activate
  {media_id, version_id}` are mutating tools requiring `confirm=true`.
- `versions.manage_branch {media_id, action, ...}` supports `create`,
  `switch`, `archive`, `merge`, `resolve`, and `abort`; it requires
  `confirm=true`. Successful actions that change file contents emit
  `MediaIndexChanged` so Gallery and database-derived state refresh.
- The old unnamespaced MCP aliases are retired; the feature-owned namespaced
  routes are the current client API.

## Limits and diagnostics

- Normal Gallery browsing may call `file_state` for visible unsaved-change badges
  without opening History. Lore store initialization emits DEBUG
  `MeterProvider.ExistingMeterReturned`; it is meter reuse, not a restore/commit.
  A finite initial burst is possible. Gallery caches passive reads by file/snapshot/
  history identity; unrelated shared-database writes must not trigger them. See
  `docs/validation-ui/2026-10-18-version-idle/README.md` for the regression evidence.

- Identity currently hashes the canonical absolute path. A file rename/move starts a separate history; external edits must be saved as a version before switching. Symlinks/special files are rejected.
- Content snapshots are deduplicated by Lore's shared store. Historical previews are separate JPEG copies; clear them only together with their Lore entity.
- Errors are surfaced as `VersionsError` and displayed in Inspector. Inspect `versions/worktrees/<entity>/.lore`, the corresponding shared store, version-info SQLite metadata and `photag.debug` spans. Do not delete a worktree/store to repair display metadata.
