# FileAtlas balanced modern concept v4

2026-10-13. Generated using the built-in image_gen tool.

User requested a middle ground between dense v2 and minimal v3. This is a static design proposal, not implemented software. Existing code and user data are untouched; no application build or runtime validation is claimed. Sample photos, filenames and metadata are presentation content.

## Intended information hierarchy

Always visible: search, thumbnail size, Tools, Details entry; library title/count, media type selector, sort and advanced Filters entry; Folders/Tags pivot and folder add; month group, image-first tiles with subtle filenames.

Conditional: tags tree only in Tags pivot; compact Tag/More footer only with selection; tile overflow only on hover/focus. Details inspector opens on demand.

Secondary: date/rating/extensions in Filters; month grouping in Tools → View; statistics, metadata export and bulk actions in Tools; ratings and AI suggestions in media context menu or Details; copy/move/delete/reveal in selection More. Source-backed behavior remains accessible in the proposed hierarchy.

## Source and ownership

Read the source layout specification and current feature audit in prior concept passes. Reviewed pinned theme.rs:184-209, main.rs:1020-1097 and gallery.rs:1383-1429 under vendor/photag-source/src/. Preserve source logical square thumbnails/radius7/gap12 and responsive layout as implementation baseline. Source search32/filter26, source body13.5/title19 and sidebar220 are reference rules; v4 placement and styling intentionally differ following the user's redesign direction. Do not measure implementation values from generated pixels.

Sidebar pivot restores source-style Folders/Tags separation. Search/sort/filter/file_manager actions belong to FileManager; navigation to Navigation; composition to Shell; shared style to UI Bridge. Implementing the new popovers and conditional surfaces requires Cordis lifecycle ownership, audit/PORTS/validation updates and appropriate compilation/testing/Windows checks.

## Generated-image review

Output: fileatlas-balanced-modern-v4-2026-10-13.png. The intended middle-ground hierarchy is represented: two columns, common controls visible, advanced filter row absent, Details closed, folder/tag pivots and contextual Tag/More footer. Exactly nine media are present. The generator varies tile aspect ratio and column widths between rows despite the prompt requesting uniform squares; implement the source responsive square grid rather than reproduce this artifact. The full tab underline differs from the source short centered underline. Hidden menus and actual hover/selection transitions remain proposals, not validated runtime behavior.

## Final generation prompt

undefined

