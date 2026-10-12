# Metadata export plugin ports / 元数据导出对接

Status: the original all-library database query and JSON serialization are
available behind a Cordis service. The source Save File dialog, export command
execution, success/error toast, and end-to-end UI wiring remain pending.

## Lifecycle and dependencies

- Cordis plugin: `photag.metadata-export`; service: `photag/metadata-export`
  (`MetadataExportService`).
- Requires `photag/database` and is started after the database plugin.
- No filesystem path is assumed by the service. The calling UI supplies the
  destination so the Windows save dialog can retain PhoTag behavior.

## Input / output

- `records()` calls the source-compatible `export_all_metadata` query through
  the database reader connection. It exports every media row ordered by full
  path with `filename`, `full_path`, `rating`, and hierarchical `tags`.
- `to_pretty_json()` returns the source pretty-printed top-level JSON array.
- `write_to(path)` writes those bytes at the selected path. It is synchronous;
  invoke it from a background task for a large library and surface errors.

## Debugging

Compare output fields, record order, zero-rating default, and ` > ` tag path
format with `vendor/photag-source/src/db/queries.rs::export_all_metadata`.
Check that the selected destination is writable and inspect
`MetadataExportError` for database, JSON, and filesystem failures. The menu
action and save-dialog bridge have not yet been connected.
