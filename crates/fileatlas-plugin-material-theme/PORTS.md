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

`Preferences { schema_version: u32, seed: String, variant: String, contrast: f64,
theme_name: String, manual_colors: BTreeMap<String,String> }`.
Version 3. The editable palette has eight semantic families: the primary seed, neutral
surface, text, secondary, success, warning, danger and info. `manual_colors` accepts only
`background`, `foreground`, `secondary.background`, and the four status `*.background` roles.
v1/v2 settings migrate seed/style/contrast and fold compatible advanced role overrides into
their nearest family; old border overrides become the generated neutral outline. Theme files
already saved remain untouched. Seed and manual values are #RRGGBB; variant
neutral/tonal-spot/monochrome; finite contrast 0..1.
Default Neutral is low chroma. Zero is Material's standard contrast, not a promise that
arbitrary user-colored media content meets the same theme role pairs.

`MaterialThemeService::snapshot() -> State` is memory-only.
`generate(preferences: Preferences, apply: bool) -> Result<State, String>` writes
`material.json`; `generate_named(preferences, name, apply)` writes `<safe-name>.json`.
Both replace an existing theme path when the chosen name matches. Before replacement, the old
theme is copied to `<name>.json.bak`; the new JSON is staged to `.json.tmp` and installed with a
write-through replacement on Windows (atomic rename elsewhere). The settings UI exposes one
`生成并应用主题` action; its target row explains the backup behavior and the action selects the
resulting theme.
Both validate the seven editable family role names and hex values, apply each family consistently
to light and dark ThemeConfig entries, use MCU DynamicScheme for each family's tonal variants, and
optionally select through Settings. The primary seed drives primary button, link, selection, focus,
and active roles. `accent.background`, all general button hover roles, `list.hover.background` and
`table.hover.background` share the same primary-family container tone, aligning toolbar, tree, card,
and table hover states. Secondary and status swatches cover their respective fills and chart roles;
surface, text, outline, chart and GPUI base color roles resolve only from the eight families.
Per-role arbitrary overrides are no longer accepted. Each ThemeConfig name
includes the requested theme name and a per-generation ID so GPUI Kit's name-keyed ThemeRegistry
cannot reuse a previous palette after same-name replacement or when another named theme uses the
same seed/style/contrast.
Writes are serialized under one operation lock.
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
contrast?, manual_colors?, name?, apply? }` defaults to neutral/0/true; a name selects a separate
output, otherwise it writes material.json. It is mutating and requires Standard.
Neither is high-impact. The host enforces access, the plugin owns schemas/handlers and cleanup.
Tools do not accept arbitrary output paths or change media data.

## Persistence, compatibility and recovery

JSON is chosen because this is a small whole-document configuration, not query-heavy data.
Root = SettingsService.path().parent()/themes; normal `%LOCALAPPDATA%/FileAtlas/themes`,
MCP profile <profile>/themes. No SQLite, independent storage manager, or binary assets.

- `material-settings.json`: versioned Preferences (schema_version=3); v1/v2 files remain readable
  and upgrade when the user next saves. Compatible old role edits fold into the eight families;
  unrepresentable per-role distinctions are removed from generator preferences when saved.
- `material.json` or `<safe-name>.json`: native GPUI ThemeSet with light/dark ThemeConfig entries.
  GPUI's existing schema governs this file, not a private plugin theme schema.
- `<safe-name>.json` may be overwritten by explicit generation. Before replacement, the last
  version is copied to `<safe-name>.json.bak`; failed input validation leaves the existing theme
  untouched. Writes stage to `.json.tmp`, then replace the destination. Preferences are updated
  separately and retain a `.json.bak`; the two files are not a transaction. The ThemeSet is
  authoritative for applied colors.
- Settings retains color_mode and adds optional theme_file. Missing field selects native default.
  Old accent_name is retained as metadata, not another source of colors.
- Missing/invalid selected theme falls back to GPUI defaults and logs the exact problem, without
  deleting the user's JSON. Invalid/unknown-version preferences load defaults with a visible
  generator error and preserve the file until the next explicit generation.
- Theme names allow ASCII letters, digits, `-`, `_` (maximum 64 characters). Invalid
  seed/variant/role/color does not replace the last output. No IO occurs during render.

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
Tests cover native JSON resolution/contrast across extreme seeds/variants, same-name replacement
with a `.json.bak` containing the prior theme, and preservation of the replacement when input is
invalid.
Runtime/visual evidence is tracked under docs/validation-ui; compilation is not
proof of every custom widget state. No dynamic DLL/store/cloud service is introduced.
