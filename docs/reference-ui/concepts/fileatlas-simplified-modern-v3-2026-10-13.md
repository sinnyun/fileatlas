# FileAtlas simplified modern interface concept v3

Date: 2026-10-13. Built-in image_gen tool.

User direction: simplify the dense v2 interface; show only necessary information and buttons; move secondary controls into collapsible regions or second-level menus; adopt modern software styling. This supersedes v2's permanently open inspector and filter ribbon.

## Visible interface

- Two columns: compact navigation and a large photo workspace.
- Navigation: FileAtlas, All files, folder section/add action, collapsed Tags, Settings.
- One command row: Search, Filters, More.
- Content: library name/count, collapsible month heading, nine image-first tiles.
- Tile overlays appear on hover; video/audio retain their essential media cues.
- No permanent inspector, EXIF fields, filenames on every tile, rating controls or idle bulk-action toolbar.

## Secondary-control mapping

| Entry point | Existing behavior retained in proposal |
|---|---|
| Filters popover | All/Photos/Videos/Audio, included/excluded extensions, From/To dates, 0–5 rating bounds, clear filters |
| More menu → View | Sort order, month grouping when applicable, continuous thumbnail size80–300 |
| More menu → Tools | Bulk actions, Statistics, Export metadata |
| Collapsed Tags section | Search, Select/Exclude/All modes, hierarchy, tag actions |
| Media hover/context menu | Preview, Details, ratings, tags, AI suggestions, open/reveal, copy/move/delete |
| Media selection | Source-backed selection actions appear only while selected |
| Settings | Existing Appearance/Toolbar/Shortcuts/Tags/AI/Updates categories |

This mapping preserves the prior feature inventory as a design intention. The static image does not implement or exercise these menus. Active filters should show a compact count on Filters and removable summaries inside its popover. Keyboard focus and context menus must provide equivalent access to hover-only actions during implementation.

## Source review and implementation boundary

Based on the source-layout specification and feature audit read in the prior concept passes, plus pinned source main.rs:1020-1097, gallery.rs:1383-1429, theme.rs:184-209 under vendor/photag-source/src/. Source has two toolbar rows and Folders/Tags tabs; v3 intentionally moves filters into a popover and uses a collapsed tag section, following the user's explicit redesign request. Source square thumbnails/radius7/gap12 and responsive column calculation remain the implementation baseline; generated pixels are not a measurement source. New neutral styling is a proposal and must use shared theme ownership if implemented.

Feature ownership remains unchanged: composition fileatlas-shell; file_manager/filters/actions fileatlas-plugin-file-manager; navigation fileatlas-plugin-navigation; shared tokens fileatlas-ui-bridge. No application code, database, user settings or sample fixtures were changed. No compilation or runtime validation was performed for this image-only deliverable. Example nature media are synthetic presentation content.

## Generated-image review

Output: fileatlas-simplified-modern-v3-2026-10-13.png. Two columns, Search/Filters/More, collapsed Tags, closed inspector and exactly nine tiles are represented. The generator added a thin blue outline to the hovered tile; keep hover, focus and selection distinguishable in implementation. Hidden menus are mapped above but are not depicted or implemented. Source square-card proportions are broadly maintained.

## Final prompt

undefined

