# Material theme generator ports

## Identity, dependencies and lifecycle

- Cordis plugin `fileatlas.material-theme`, service `fileatlas/material-theme` (`MaterialThemeService`).
- Requires public `SettingsService` and `UiBridgeService`; optional host `McpToolRegistry`.
- Compile-time plugin starts after Settings. No timers or workers survive plugin stop.
- Settings view `material-theme/settings` in `settings.pages`; metadata title Material 主题生成.
  Cordis-owned view registration removes it on stop. MCP registration guard removes both tools.
- It only produces native GPUI ThemeSet JSON. It never calls Theme::update/change or maintains
  a separate UI palette. Existing UI Bridge/native GPUI infrastructure resolves every paint.
  Generated JSON may remain selected after generator stop: it is user data read by the host,
  with no dependency on the stopped generator's service or UI.

## Public payloads

`Preferences { schema_version: u32, seed: String, variant: String, contrast: f64 }`.
Version 1; seed is #RRGGBB; variant neutral/tonal-spot/monochrome; finite contrast 0..1.
Default Neutral is low chroma. Zero is Material's standard contrast, not a promise that
arbitrary user-colored media content meets the same theme role pairs.

`MaterialThemeService::snapshot() -> State` is memory-only.
`generate(preferences: Preferences, apply: bool) -> Result<State, String>` validates input,
uses MCU DynamicScheme for both modes, translates roles into native ThemeConfig JSON,
serializes writes under one operation lock and optionally selects the file through Settings.
`apply_saved() -> Result<(), String>` validates the saved ThemeSet and selects it.
`State { preferences, output: PathBuf, error: Option<String>, generated: bool }`.
MCP serializes the domain error as `last_error` so it is not confused with a failed tool call.
The seed uses GPUI Kit's native `ColorSelect` / `ColorPickerState`, not a text input
or a plugin-local swatch table. Its built-in multi-shade palette and HSLA sliders
come from the existing open-source `gpui-component 0.7.2` / `gpui-base 0.7.2`
(through pinned `gpui-kit 0.7.1`, Apache-2.0). No extra GUI dependency is added.
The selection is draft-only until Generate; hover previews do not commit or write.
The picker may expose alpha, but Material's seed uses only its RGB, with opaque output.
`ColorPickerEvent::Change` subscription belongs to the view and drops with it.
The view drafts inputs using the shared declarative SettingsPage; generate/apply are explicit
user actions. Render does not read files, generate colors, or mutate Settings.

MCP `material-theme.state {}` is read-only. `material-theme.generate { seed, variant?,
contrast?, apply? }` defaults to neutral/0/true, is mutating and requires Standard.
Neither is high-impact. The host enforces access, the plugin owns schemas/handlers and cleanup.
Tools do not accept arbitrary output paths or change media data.

## Persistence, compatibility and recovery

JSON is chosen because this is a small whole-document configuration, not query-heavy data.
Root = SettingsService.path().parent()/themes; normal `%LOCALAPPDATA%/FileAtlas/themes`,
MCP profile <profile>/themes. No SQLite, independent storage manager, or binary assets.

- `material-settings.json`: versioned Preferences (schema_version=1).
- `material.json`: native GPUI ThemeSet with named light/dark ThemeConfig entries.
  GPUI's existing schema governs this file, not a private plugin theme schema.
- Existing files are backed up as `.json.bak`; writes stage to `.json.tmp` then rename.
  Each file replace is atomic; the two files are not a transaction. The ThemeSet is authoritative
  for applied colors; a failure is reported and a retained backup can be restored explicitly.
- Settings retains color_mode and adds optional theme_file. Missing field selects native default.
  Old accent_name is retained as metadata, not another source of colors.
- Missing/invalid selected theme falls back to GPUI defaults and logs the exact problem, without
  deleting the user's JSON. Invalid/unknown-version preferences load defaults with a visible
  generator error and preserve the file until the next explicit generation.
- Invalid seed/variant/contrast does not replace the last output. No IO occurs during render.

## Upstream and licensing

`material-colors = 0.4.2`, crate VCS `3ffe880a21dbcfde28504af021600799d0c5882c`,
https://github.com/Aiving/material-colors. Community Rust port of Google's Material Color
Utilities (https://github.com/material-foundation/material-color-utilities).
Pinned 0.4.2 supports the workspace MSRV; 0.5.0 requires Rust 1.97 whereas workspace declares 1.95.
MIT OR Apache-2.0; preserve the included licenses/ notices on distribution. Workspace stays MIT.
The bridge doesn't copy MCU algorithms. Status seed hues come from GPUI's built-in semantic
status colors; the generator creates their role pairs through MCU, with no plugin paint table.

## Debugging and limits

Inspect Settings → Plugin settings → Material 主题生成, `material-theme.state`, generated
JSON, host Debug logs, and get_ui_state.theme (framework/name/mode/resolved role pairs).
Generate reports validation/write errors in State and in its owned view; Settings persistence
errors are returned. GUI apply failures log in UI Bridge; no silent alternate palette.
Tests cover native JSON resolution/contrast across extreme seeds/variants, and failure/backup
preservation. Runtime/visual evidence is tracked under docs/validation-ui; compilation is not
proof of every custom widget state. No dynamic DLL/store/cloud service is introduced.
