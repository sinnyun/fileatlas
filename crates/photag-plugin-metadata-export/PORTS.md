# Metadata export plugin ports / 元数据导出对接

Status: the original all-library database query and JSON serialization are
available behind a Cordis service. A lifecycle-owned GPUI panel now offers the
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
  selection. This crate-local dependency is not yet represented in the root
  Cargo.lock; workspace integration must resolve/pin it before a locked build.
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
- The GPUI panel `workspace.metadata-export` contains a path input, Browse…,
  and Export buttons. Browse opens `rfd::FileDialog` with the source title,
  JSON filter, and `photag_backup.json` default. On Windows, the native Save
  File dialog handles existing-file overwrite confirmation. Cancel leaves the
  current path intact. The button emits `UiCommandInvoked("metadata-export/export")`;
  the Cordis listener starts the query/serialization/write on a named worker
  thread. Existing command/menu contributions use the same command ID.
- Result payload: successful status includes the exact output path and number
  of exported rows; errors preserve database, JSON, filesystem, and state-lock
  messages.

## Debugging

Compare output fields, record order, zero-rating default, and ` > ` tag path
format with `vendor/photag-source/src/db/queries.rs::export_all_metadata`.
Inspect the panel result and `MetadataExportError` text for database,
serialization, and filesystem failures. The Save File dialog is platform
native through `rfd`; Windows overwrite behavior follows the native dialog.
Other desktop-platform behavior depends on the corresponding `rfd` backend and
has not been validated. The worker writes directly to the selected path after
the dialog returns.
