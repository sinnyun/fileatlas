# WorkCrate Projects plugin ports

Status: project/file-reference JSON service and attached JPEG/PNG preview
storage are implemented. Board documents and source-pixel rendering now live in
`fileatlas-plugin-workcrate-boards`; the WorkCrate overview/editor UI and MCP
interactions remain in development.

## Cordis identity and lifecycle

- Plugin: `fileatlas.workcrate-projects`.
- Service: `fileatlas/workcrate-projects` (`WorkCrateProjectsService`).
- Requires only Cordis. Publishes the service for WorkCrate UI and board
  features. Listens for `WorkCrateAddReferencesRequested` from the shared
  `fileatlas-workcrate-contracts` crate and resolves paths on a blocking worker.
- Shell supplies an isolated root `<dev-test-profile>/workcrate` during MCP
  validation. Normal startup uses `%LOCALAPPDATA%\FileAtlas\workcrate`.
- Cordis owns service publication and event-listener cleanup.

## Stored data

- `projects.json` is a version 1 JSON index with project ID, name, creation
  timestamp, archive status, and optional `main_image` (relative preview path,
  hash and dimensions). The additive optional field defaults to None for old v1
  files; there is no destructive migration.
- `projects/<project-uuid>/project.json` contains that project's stable asset
  IDs and original source paths. The directory also reserves a `previews/`
  location for durable user-selected preview copies.
- JSON writes use a same-directory temporary file, flush the file, retain a
  backup during replacement, and restore a backup if the primary file is absent
  at startup. Unknown versions stop startup without rewriting user data.
- Source files are never copied or modified by add, relink, rename, archive, or
  remove-reference operations. Relinking updates the path on the existing asset
  ID; removing a reference does not delete its source or preview file.
- References can point to any file type. The board contracts currently permit
  direct placement of JPEG/JPG and PNG only; other references need an attached
  image preview to appear on a board.

## Public service operations

- `projects(include_archived)` returns project records.
- `create_project`, `rename_project`, and `set_archived` persist project
  lifecycle changes.
- `set_main_image(id,path,thumbnails)` validates JPEG/PNG with the shared
  Thumbnail service, copies source bytes into feature-owned previews, and updates
  the project index. Old covers remain recoverable.
- `duplicate_project(id,name)` copies reference metadata and retained previews;
  the Boards service separately clones page documents with new page IDs.
- `delete_project(id)` moves the feature-owned directory to
  `workcrate/trash/<uuid>-<operation-uuid>` and removes its index entry, rolling
  back the directory move if index persistence fails. Linked originals stay
  intact. Trash restoration is currently manual; archive restoration is in UI.
- `references(project_id)` returns stable file references.
- `add_references(project_id, paths)` canonicalizes existing files, ignores
  duplicate Windows paths, stores file metadata, and permits arbitrary formats.
- `relink_reference(project_id, asset_id, path)` retains the asset ID.
- `set_custom_preview(project_id, asset_id, path, thumbnails)` accepts JPEG/PNG,
  validates it through FileAtlas's shared Thumbnail service, copies the actual
  image bytes into the project `previews/` directory, and records dimensions
  plus SHA-256. It never substitutes the low-resolution thumbnail cache file.
- `remove_custom_preview(project_id, asset_id)` clears the selected reference
  but retains the copied image to preserve recovery and any board placement.
- `remove_reference(project_id, asset_id)` removes only the reference and
  reorders the remaining entries.
- `revision()` and `wake()` support passive UI refresh. Rendering must not call
  these operations or read files.

The board service consumes references and writes each DesignCraft-backed board
under the same project directory. See the boards plugin `PORTS.md` for image
acceptance, source-byte hydration, viewport render, and PDF output behavior.

## Current limits and diagnostics

- The three-level project/editor UI is owned by WorkCrate UI. Custom reference
  preview attach/replace/remove controls remain open. Multi-service duplication
  failure/recovery and concurrent project operations need additional coverage.
- Same-process FileAtlas MCP startup and native project creation were verified;
  the project selector, arbitrary file picker, custom-preview UI and restart
  persistence still need runtime coverage.
- For persistence failures inspect the Cordis startup error, then
  `projects.json` and the matching `projects/<id>/project.json` plus backup.

## Verification

- `cargo +1.97.1 test --offline -j 2 -p fileatlas-workcrate-contracts -p fileatlas-plugin-workcrate-projects`
  covers arbitrary-file references, reopen, duplicate path handling, relinking
  with stable identity, original-file preservation, and image-extension
  eligibility contracts.
- Runtime validation of native picker and custom-preview UI remains pending;
  project/reference services are connected to the WorkCrate UI and MCP state.

## Field presets (2026-10-21)

FieldPreset { key,label,values } and FieldPresets { version,fields } use a modest
whole-document JSON under the existing WorkCrate root: field-presets.json, version1.
field_presets returns the cached snapshot; save_field_presets validates and writes
with existing atomic JSON/recovery helpers, then emits the existing Projects wake.
Built-ins local/material/size/quantity remain present; custom keys use extra:<label>.
Multiple unique presets per field are supported (64 fields, 128 values/field;
nonempty labels <=60 characters and values <=500). Unknown schema versions or
invalid/duplicate records fail explicitly. Missing file uses default built-ins;
existing backup reading/recovery behavior is reused. Large image assets remain
separate feature-owned files. Preset edits/removals never rewrite stored page values.

2026-10-21 UI synchronization: the existing FieldPresets storage/service signatures
are unchanged. The Settings contribution now auto-saves valid field/value edits,
filtering unfinished blank value rows. Existing wake/revision and cached reads
supply editor menus; adding choices does not replace a page's saved content.
