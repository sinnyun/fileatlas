# Runtime debug plugin

- Cordis identity: `photag.debug`; public service: `photag/debug` (`Arc<DebugService>`).
- Startup dependency: `UiBridgeService`. Shell creates the buffer and calls `attach` before runtime/plugin startup, then starts DebugPlugin after publishing the bridge. All other plugins only emit `tracing`/`log`; they do not depend on this UI plugin.
- `DebugPlugin::Config/Input = Arc<DebugService>`. Default capacity 10,000 records, individual target/action/message capped at 4096 UTF-8 bytes plus truncation marker. Oldest records are evicted; sequence numbers remain monotonic after clear. State is memory only and clears on restart.
- `attach(&Arc<DebugService>) -> Result<(), String>` installs the process tracing subscriber, log adapter and panic hook once. The capture sink is weak and detached on plugin disposal; reattachment is supported. The panic hook preserves the previous handler. A different preinstalled subscriber/logger is an explicit startup error, not silent missing capture.
- `Record`: id, local ISO timestamp with milliseconds and UTC offset, monotonic elapsed_ms, level, category, target, action, message, optional duration_ms, span_id, parent_span_id, thread. Category derives from the first module path segment. All emitted application tracing/log levels are retained; rmcp transport DEBUG/TRACE payloads are excluded to prevent recursively recording queried log responses. MCP operations and rejected requests have separate application records. Spans produce start and close records; close uses monotonic wall-clock duration (includes waits), with error status if the span received ERROR. Nested spans correlate via IDs. Explicit worker duration_ms fields are retained in the duration column.
- `query(&Filter) -> Page`: exact category and case-insensitive exact level; case-insensitive substring across message/action/target/ID, newest first, zero-based page, default 50 rows, page size clamped to 1..200, page clamped to available results. Returns total/retained/evicted counts, categories and latest ID. `through_id` freezes an inspection view while collection continues; capacity eviction still applies. `record_by_id` retrieves retained details exactly.
- Collection revision and view invalidation are separate. `view_revision(panel: bool) -> (u64,u64)` returns UI/control and live-content clocks; closed drawers, the status button and paused views do not repaint for incoming logs. GPUI's DEBUG/TRACE `Sending a11y tree update` records remain in the full buffer/filter/paging API but do not advance the content clock, because the drawer's own paint emits them. Other application records and framework warnings/errors still drive live updates. This breaks the observer feedback loop without dropping accessibility diagnostics.
- `control(action, value) -> Result<(), String>` drives both real UI and MCP: open/close/toggle, category/level/search, page, page_size (25/50/100/200), pause/follow, select and clear. Historical paging automatically freezes at current sequence. UI controls/filter state are ephemeral, no session/schema migration. `clear` resets view and eviction count, not sequence numbers. `ui_state` returns the shared drawer state.
- UI contributions: bottom status button in `workspace.debug-status`, expandable dock in `workspace.bottom`, metadata `debug/panel`. Cordis removes views, catalog metadata, public service, sink and event observers on stop. Drawer is 46% of client height, constrained to 220..380 logical pixels, with scrollable toolbar/list/details, exact level/category menus (category list scrolls within 320 logical pixels), keyword field, pause/resume, clear, previous/next page, page size and close. Clicking a row shows retained details, thread and correlation IDs.
- Events observed: UiCommandInvoked, FolderSelectionChanged, DirectoryPathSelected, TagSelectionChanged, TagFilterSelectionChanged and MediaIndexChanged. No product mutation is emitted by the debug plugin.
- Producers: all application startup apply spans and previous stderr diagnostic paths, DB serialized read/write spans with caller location and SQL errors, Gallery load/page/reload/filter/group/selection/preview/actions, Navigation requests/selections, file and tag operations, indexing stream phase/error/finish and worker elapsed, thumbnail generation, media playback commands, history and metadata export. File/data behavior and original error returns are preserved.
- Limits: this is application logging, not instruction-level tracing or a native crash dump. Non-emitting third-party native components/OS actions are not invented. Process abort/termination may prevent UI inspection, and cancelled/dropped spans report their lifetime rather than certifying business success. No persistent log export, SQL statement/body capture, CPU profiler or durable per-file audit history. Worker spans do not automatically propagate parent context through every spawn_blocking/thread boundary; thread IDs and explicit scan IDs help correlate.
- Failures: compilation/test logs under target/debug-plugin-*.log; isolated same-process MCP logs/screenshots and validation evidence are documented in docs/validation-ui/2026-10-14-debug-panel/README.md. MCP get_debug_info reads the buffer without logging itself; control_debug uses the normal mutation guard.

## Shared refresh ownership (2026-10-15)

Both drawer and footer views use UI Bridge refresh::poll and ChangeGate over existing view revisions. Recording remains independent: all captured records are retained by the existing store; only actual UI-relevant revision changes request a repaint. No new filtering or suppression is introduced.

## Drawer repaint-noise display rule (2026-10-15)

`Filter.hide_ui_refresh: bool` defaults false for raw service queries. The actual
UiState defaults it to true, so the drawer excludes only DEBUG/TRACE
`gpui::window` records containing `Sending a11y tree update:` before search and
pagination. The category menu also excludes categories made solely of those
hidden records. WARN/ERROR and all other operation/DB records remain visible.
Raw retention, eviction, sequence clocks and record_by_id are unchanged; retained
still counts the full buffer. The classifier is shared with the existing
content-clock feedback guard. No new controls or persisted settings.
The original no-suppression statement above describes collection, which remains
unchanged; the user explicitly requested this presentation filter afterwards.
Evidence: docs/validation-ui/2026-10-15-debug-hover/README.md.

## Image timing detail records (2026-10-15)

Preview now emits per-job source/cache paths, sizes/dimensions, role, decoder,
cache status, phase timings and failures. The existing message/detail drawer
shows these fields when selecting a row; duration_ms still populates the duration
column. No new render-time queries, display filters or persisted state. Routine
GPUI repaint hiding does not hide these application DEBUG/ERROR records.

## Unified theme (2026-10-18)

View paints use UI Bridge semantic theme tokens instead of local color literals.
Gallery inspector/statistics cards and headings consume shared components; compact
navigation preserves its layout with common selection/hover paints. Status, branch
and media-overlay colors remain semantically distinct in the central theme.
No product service/data/event contract changes. See `docs/ui-theme.md`.
