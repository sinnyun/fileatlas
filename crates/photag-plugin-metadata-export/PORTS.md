# Metadata export plugin ports / 元数据导出对接

Status: the original all-library database query and JSON serialization are
available behind a Cordis service. A lifecycle-owned GPUI panel now accepts a
JSON destination path, invokes the registered Cordis command, performs the
export on a named worker thread, and displays the record count/path or failure.
The Windows native Save File dialog has not been adapted; users enter the full
destination path in the panel.

## Lifecycle and dependencies

- Cordis plugin: `photag.metadata-export`; service: `photag/metadata-export`
  (`MetadataExportService`).
- Requires `photag/database` and is started after the database plugin.
- Also requires the host-published `photag/ui-bridge` service for lifecycle
  registration of the panel and command.
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
- The GPUI panel `workspace.metadata-export` contains a path input and Export
  button. The button emits `UiCommandInvoked("metadata-export/export")`;
  the Cordis listener starts the query/serialization/write on a named worker
  thread. Existing command/menu contributions use the same command ID.
- Result payload: successful status includes the exact output path and number
  of exported rows; errors preserve database, JSON, filesystem, and state-lock
  messages.

## Debugging

Compare output fields, record order, zero-rating default, and ` > ` tag path
format with `vendor/photag-source/src/db/queries.rs::export_all_metadata`.
Enter a writable absolute path ending in `.json`; inspect the panel result and
`MetadataExportError` text for database, serialization, and filesystem
failures. No native file picker, overwrite confirmation, or Explorer path
browsing has been connected. The export writes directly to the entered path.
