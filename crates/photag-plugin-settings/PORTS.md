# Settings plugin ports / 设置数据对接

Status: the pinned PhoTag settings JSON model and Cordis service are migrated. GPUI settings controls and wiring each setting into its feature remain pending.

## Lifecycle and storage

- Cordis plugin: `photag.settings`; service name: `photag/settings` (`SettingsService`).
- No service dependencies. Default file is `%LOCALAPPDATA%\PhoTag\settings.json`; tests/hosts may override the path. Missing or invalid JSON loads PhoTag defaults, matching source behavior.
- The `Settings`, toolbar, AI, search-scope, tag-search/merge, hotkey, accent and update preference fields preserve the source JSON names, serde defaults, and enum spellings.

## Data interface

- `snapshot()` returns a clone of current settings. `replace(settings)` replaces and persists; `update(edit)` applies one edit, persists, and returns the updated clone.
- `ExcludeModifier::matches_modifiers(ctrl, shift, alt)` retains source matching semantics without egui types. `open_key` and hotkey key names remain strings for a GPUI adapter.
- Persistence errors are returned to callers; load retains the source fallback-to-default behavior.

## Debugging

Compare `settings.json` against the source model in `vendor/photag-source/src/settings.rs`. For a value that resets, inspect JSON validity and serde defaults; for a failed save, inspect the resolved LocalAppData path and filesystem permissions.
