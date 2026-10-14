# Session plugin ports / 会话数据对接

Status: the pinned PhoTag `session.json` schema and Cordis service are migrated. Gallery filters, selected media IDs, scroll offset, thumbnail size, and month-group setting restore on view creation and save on selection/filter changes, scroll changes, and view teardown. Navigation restores folder/path selection, expanded directories, and expanded tag IDs. The shell restores and saves the sidebar width. Preview reopens the last image path and restores its transform, zoom, pan, zoom anchor, and information visibility when the path still exists.

## Lifecycle and storage

- Cordis plugin: `photag.session`; service name: `photag/session` (`SessionService`).
- No service dependencies. Default file is `%LOCALAPPDATA%\PhoTag\session.json`; the path can be overridden. Gallery depends on this service.
- `SessionState` preserves PhoTag field names, serde defaults, sort order, media type, and tag/extension click-mode enum strings. The model covers selected folder/path/media IDs, preview path and optional preview transform fields, sidebar tab/width, expanded nodes, gallery scroll, filters, thumbnail size, month grouping, collapsed months, and dropdown expansion. New fields are serde-defaulted for compatibility with existing session files.

## Data interface

- `snapshot()` returns a clone of current state. `replace(session)` updates the in-memory state and writes pretty JSON to the original path. Missing or invalid files load as the default session, matching source behavior.
- Database `SortOrder` is reused so serialized order names stay aligned with the gallery query contract.

## Debugging

Check `session.json` keys and values against `vendor/photag-source/src/session.rs`. Gallery persists its current selection and pixel scroll offset after scroll changes and view teardown. Shell persists width at the end of a sidebar drag. Preview persists/restores its view state for the matching source path; Navigation restores the active sidebar page from the free-form sidebar_tab field, accepting old Folders/Tags values. Abrupt process termination can still lose the latest gallery or sidebar change before its next save point.

## Inspector width compatibility (2026-10-14)

SessionState.inspector_width:f32 is serde(default); zero uses300pt. Existing sidebar_width, selection and all source fields retain their names/semantics. Shell saves both widths at splitter mouse-up; no user file migration/reset is performed. Compatibility test loads old JSON without inspector_width, preserves folder7/media12/Tags and round-trips a350pt right width. MCP restart restored actual dragged widths304/340 in an isolated profile.
