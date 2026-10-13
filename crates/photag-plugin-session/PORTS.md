# Session plugin ports / 会话数据对接

Status: the pinned PhoTag `session.json` schema and Cordis service are migrated. Gallery filters, selected media IDs, scroll offset, thumbnail size, and month-group setting restore on view creation and save on selection/filter changes, scroll changes, and view teardown. Navigation restores folder/path selection, expanded directories, and expanded tag IDs. The shell restores and saves the sidebar width. Preview reopens the last image path when it still exists; transform state is not persisted.

## Lifecycle and storage

- Cordis plugin: `photag.session`; service name: `photag/session` (`SessionService`).
- No service dependencies. Default file is `%LOCALAPPDATA%\PhoTag\session.json`; the path can be overridden. Gallery depends on this service.
- `SessionState` preserves PhoTag field names, serde defaults, sort order, media type, and tag/extension click-mode enum strings. The model covers selected folder/path/media IDs, preview path, sidebar tab/width, expanded nodes, gallery scroll, filters, thumbnail size, month grouping, collapsed months, and dropdown expansion. `selected_media_ids`, `preview_path`, and `group_by_month` are backward-compatible PhoTag additions; the original file does not contain these newer fields.

## Data interface

- `snapshot()` returns a clone of current state. `replace(session)` updates the in-memory state and writes pretty JSON to the original path. Missing or invalid files load as the default session, matching source behavior.
- Database `SortOrder` is reused so serialized order names stay aligned with the gallery query contract.

## Debugging

Check `session.json` keys and values against `vendor/photag-source/src/session.rs`. Gallery persists its current selection and pixel scroll offset after scroll changes and view teardown. Shell persists width at the end of a sidebar drag. Preview only persists its source path; zoom, pan, rotation, flips, info visibility, selected tag, and active sidebar tab are not currently restored. Abrupt process termination can still lose the latest gallery or sidebar change before its next save point.
