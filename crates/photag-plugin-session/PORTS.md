# Session plugin ports / 会话数据对接

Status: the pinned PhoTag `session.json` schema and Cordis service are migrated. Gallery search/sort/media type/extension/date/rating/tag filters restore on view creation and persist when the gallery query reloads. Navigation folder/path selection and expanded directories restore; the gallery consumer saves selection and the navigation consumer saves expansion changes. Gallery scroll/selection, sidebar dimensions, and preview state remain pending.

## Lifecycle and storage

- Cordis plugin: `photag.session`; service name: `photag/session` (`SessionService`).
- No service dependencies. Default file is `%LOCALAPPDATA%\PhoTag\session.json`; the path can be overridden. Gallery depends on this service.
- `SessionState` preserves PhoTag field names, serde defaults, sort order, media type, and tag/extension click-mode enum strings. The model covers selected folder/path, sidebar tab/width, expanded nodes, gallery scroll, filters, collapsed months, and dropdown expansion.

## Data interface

- `snapshot()` returns a clone of current state. `replace(session)` updates the in-memory state and writes pretty JSON to the original path. Missing or invalid files load as the default session, matching source behavior.
- Database `SortOrder` is reused so serialized order names stay aligned with the gallery query contract.

## Debugging

Check `session.json` keys and values against `vendor/photag-source/src/session.rs`; inspect startup restore and shutdown save integration separately. Current shell starts the service, but does not yet apply the saved state to views.
