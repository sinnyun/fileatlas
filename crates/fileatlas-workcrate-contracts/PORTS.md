# WorkCrate contracts ports

`fileatlas-workcrate-contracts` is a plain shared crate, not a Cordis plugin. It
publishes no service and opens no file. It owns the stable WorkCrate value types
and the Cordis event types that feature plugins exchange.

## Identity and consumers

- IDs: `WorkCrateProjectId` and `WorkCrateAssetId` are UUID strings assigned by
  `fileatlas-plugin-workcrate-projects`.
- Records: `WorkCrateProject` (name, created_at, archived, optional `main_image`),
  `WorkCrateFileReference` (path, display title, order, size, modified time,
  optional `custom_preview`) and `WorkCratePreview` (relative-cached path, sha256,
  pixel size) are the versioned JSON shapes Projects persists.
- `canvas_image_format` / `CanvasImageFormat` decide which raster sources a board
  may accept. Projects and its `management` module are the only current callers.
- Requests: `WorkCrateAddReferencesRequest { project_id, paths }`.

## Events

- `WorkCrateAddReferencesRequested` — subscribed by
  `fileatlas-plugin-workcrate-projects/src/lib.rs:730`. No crate publishes it yet,
  so the FileManager "add selection to WorkCrate" handoff is still unwired.
- `WorkCrateProjectsChanged` — declared here and currently neither published nor
  subscribed anywhere in the workspace.

## Where to inspect

Type changes surface as compile errors in the WorkCrate plugin crates; there is no
runtime registry. Event delivery problems are inspected through the Cordis event
bus in the emitting and subscribing plugins' logs, and through the WorkCrate UI
operation message. See `fileatlas-plugin-workcrate-projects/PORTS.md` for the
persistence layout and `fileatlas-plugin-workcrate-boards/PORTS.md` for the
official-program boundary.
