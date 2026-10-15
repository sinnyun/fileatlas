# Metadata export plugin ports / 元数据导出对接

Status: the original all-library database query and JSON serialization are
available behind a Cordis service. A lifecycle-owned GPUI modal now offers the
source-compatible native Save File dialog and overwrite confirmation, invokes
the registered Cordis command, performs the export on a named worker thread,
and displays the record count/path or failure.

## Lifecycle and dependencies

- Cordis plugin: `photag.metadata-export`; service: `photag/metadata-export`
  (`MetadataExportService`).
- Requires `photag/database` and is started after the database plugin.
- Also requires the host-published `photag/ui-bridge` service for lifecycle
  registration of the panel and command.
- Uses the source project's `rfd` 0.15 native file-dialog library for Save File
  selection. It is included in the workspace lockfile with default portal
  features disabled; Windows uses the native Windows dialog backend.
- Plugin ID: `photag.metadata-export`; command ID:
  `metadata-export/export`; panel view ID: `metadata-export/panel-view`.
- `photag.metadata-export` removes its catalog entries, view, and command
  listener when its Cordis context is disposed.

## Input / output

- `records()` calls the source-compatible `export_all_metadata` query through
  the database reader connection. It exports every media row ordered by full
  path with `filename`, `full_path`, `rating`, and hierarchical `tags`.
- `to_pretty_json()` returns the source pretty-printed top-level JSON array.
- `write_to(path)` writes those bytes at the selected path synchronously. The
  GPUI command performs the equivalent steps on a dedicated worker thread.
- `set_destination(path)` stores the path used by the command and
  `status()` exposes `Idle`, `Exporting`, `Succeeded { path, records }`, or
  `Failed(message)` to the panel.
- The GPUI dialog contributes to workspace.modal and opens through UiBridge surface
  metadata-export. The command opens preparation; Choose destination opens rfd
  with JSON filter/default photag_backup.json and invokes request_export after
  a destination is returned. It shows true Idle/Exporting/Succeeded/Failed states,
  record count/path and error, with indeterminate progress and bounded footer.
  Native cancellation leaves the destination unchanged; overwrite is OS-owned.
- Result payload: successful status includes the exact output path and number
  of exported rows; errors preserve database, JSON, filesystem, and state-lock
  messages. The panel renders idle guidance, an in-progress message, and a
  persistent terminal success or failure message (including path and record
  count on success); service failures and unavailable status use the danger
  color, while input and command-dispatch errors have a separate visible error
  line. Ready and active states include concise guidance about the current
  operation.
- The Cordis command is single-flight: another invocation while an export is
  running is rejected without replacing the active status. Thread-start errors
  transition the status to `Failed`, so the view cannot remain stuck at
  `Exporting`; worker panics are caught and reported as failures as well.
  Completion is stored only by the worker associated with that active run;
  the panel observes it through a weak entity update, starts at most one local
  status poll per invocation, and stops polling when work ends or the view is
  disposed. Database and file I/O run without holding the status mutex.
- Selecting or submitting a new destination clears the prior terminal result
  (unless a run is active), so success text always identifies the file actually
  written and is not left attached to a newly selected destination.

## Debugging

Compare output fields, record order, zero-rating default, and ` > ` tag path
format with `vendor/photag-source/src/db/queries.rs::export_all_metadata`.
Inspect the panel result and `MetadataExportError` text for database,
serialization, and filesystem failures. The Save File dialog is platform
native through `rfd`; Windows overwrite behavior follows the native dialog.
Other desktop-platform behavior depends on the corresponding `rfd` backend and
has not been validated. The worker writes directly to the selected path after
the dialog returns.

## Shared visual tokens

The export panel uses photag-ui-bridge::theme for source secondary/danger text and its subtle edge color. Layout spacing and typography still use GPUI Kit presets; Windows visual parity is unverified.


## Localization integration (2026-10-13)

Visible control labels consume the public UI Bridge translator supplied by
photag.i18n. Stable commands, source settings/schema values, media paths and
user-created tag data are unchanged. Missing translations fall back to English;
OS/backend errors and some generated diagnostics are not fully localized.
The 07 companion sheet shows percentage progress, but the public export status
has only `Exporting` and carries no progress count, so the panel reports the
actual active state without an invented percentage. Live MCP visual review is
reserved for the parent redesign task; Windows native dialog interaction is
still unverified here.

## v5 design 07 command and visible status (2026-10-14)

The `metadata-export/export` command opens the preparation dialog through UiBridgeService
surface `metadata-export`. Choose destination invokes the source-compatible native JSON save
dialog (`photag_backup.json`, OS overwrite confirmation), then sets the destination and calls
`MetadataExportService::request_export`. The modal polls the real shared status and displays
idle/exporting/succeeded/failed, record count and destination. Progress is indeterminate because
the service has no per-record progress contract. MCP `export_metadata` also drives this status
modal. Closing does not cancel a worker and is disabled while exporting. Schema/query/writer,
service identity and worker lifecycle are unchanged; the UI contribution now owns workspace.modal.

## 2026-10-14 current implementation and MCP evidence

Reference07 real success12records and filesystemdirectory failure(os5) rendered through PhoTag MCP. Query/schema/order/writer unchanged. Native save/overwrite clicks remain unverified; see docs/redesign-validation.md.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics: plugin startup and existing error/diagnostic output now emit tracing records captured by photag.debug; instrumented public operations report wall-clock span duration and errors. Domain service contracts, event payloads and source operation ordering are unchanged. See photag-plugin-debug/PORTS.md for capture scope and limits.

## Shared refresh ownership (2026-10-15)

Export panel reports status/visibility changes to UI Bridge refresh::poll, preserving existing loading/success/error and opening behavior. No per-tick notify while status is unchanged.
