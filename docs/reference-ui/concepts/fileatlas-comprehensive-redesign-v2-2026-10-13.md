# FileAtlas comprehensive redesign concept v2

Date: 2026-10-13. Tool: built-in image_gen.

The user explicitly requested a more thorough redesign including layout, styling, graphics and effects. This concept therefore deliberately departs from the source layout. It is an image-only design deliverable, not an implementation, feature-parity claim or interactive validation. Existing code changes are untouched.

## Design decisions

- Replace the source two-column composition with navigation, media workspace and a dismissible selected-item inspector.
- Move the Folders/Tags tabs into separate navigation sections; retain folder actions and tag search/mode controls.
- Separate library context from search commands and filtering; retain source sort, month grouping, media types, extensions, date bounds and rating range.
- Unify typography, line icons, borders, spacing and interaction emphasis with a graphite/periwinkle palette.
- Preserve square image surfaces and use in-image filename/media overlays; preserve footer selection actions.
- Use nine conceptual sample media to communicate photographic readability. These do not represent changes to the user's database or test fixtures. Metadata, tags and file names in the image are demonstration data.
- Recompose Preview and Settings using the same visual language in a companion design sheet.

## Source review and deliberate differences

Read docs/reference-ui/source-layout-spec.md in full across this and the preceding concept pass, plus current status in docs/source-feature-audit.md. Source inspected in the preceding pass: theme.rs:184-209, main.rs:1020-1097, gallery.rs:1383-1429 under vendor/photag-source/src/.

Source rules: default sidebar220, search32 high/220–540 wide, filters26 high/gaps6, thumbnail radius7/gap12, file_manager radius10, body13.5/title19; source composition has one elevated file_manager with two sidebar tabs. V2 proposes new spacing, palette, three columns and sidebar sections by explicit user request; generated pixels are not implementation measurements. Source preview is full-window, its filmstrip80 high with64 thumbnails, toolbar floats near the bottom (preview.rs:797-1002,2370-2419). Source settings is movable width360, six tabs with72 source tab width (main.rs:2107-2175); v2 proposes a larger modal with vertical category navigation to address density and the documented tab-overflow risk. These are design proposals, not compatibility fixes already shipped.

Ownership on implementation: composition remains fileatlas-shell; search, filters, file_manager and selected details belong to fileatlas-plugin-file-manager; navigation to fileatlas-plugin-navigation; preview/playback to their feature plugins; settings to fileatlas-plugin-settings; shared styling to fileatlas-ui-bridge. Preserve Cordis lifecycle ownership and public contracts.

Only screenshot-visible representative states are drawn. Hidden menus, error/empty/loading states and every settings tab are not fully specified by these images. No build or runtime checks apply to this image-only deliverable; application changes require the source audit, PORTS alignment, locked compilation, formatting, relevant tests and Windows interaction checks.

## Generated-image review

Companion image: fileatlas-comprehensive-redesign-v2-preview-settings-2026-10-13.png. Preview, the Appearance settings category and representative interaction states are visible. Other settings categories are navigation entries, not completed content specifications. The Loading sample was rendered with a slider-like thumb; use the source progress-bar behavior in implementation. Palette swatches and the aperture brand mark are design proposals, not existing source assets. Raster typography contains minor imperfect sample text in the small accent preview. Effects are static depictions rather than runtime animations.

Main image: fileatlas-comprehensive-redesign-v2-main-2026-10-13.png. The generator rendered landscape tiles rather than the requested squares, added a grid/list switch outside the source inventory, and changed tag mode order to All/Select/Exclude. Treat these as concept discrepancies: a grid/list switch needs separately implemented behavior or removal; source-backed tag modes and square tiles remain the implementation baseline unless deliberately redesigned. A static concept does not demonstrate actual hover, animation, focus or keyboard behavior.

## Companion design sheet prompt

undefined

## Main screenshot prompt

undefined
