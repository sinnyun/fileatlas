# FileAtlas interface refinement concept

Date: 2026-10-13. Generated with the built-in image_gen tool.

This is a visual concept based on the supplied screenshot, not an implemented UI change or evidence of interactive parity. Existing working-tree code changes were left untouched. No application compilation or runtime validation was performed for this image-only deliverable.

Image: fileatlas-interface-refinement-2026-10-13.png

## Source references reviewed

- docs/reference-ui/source-layout-spec.md: main window, command bar, sidebar, file_manager and status bar.
- docs/source-feature-audit.md: current implementation status and validation limits.
- vendor/photag-source/src/theme.rs:184-209: geometry and font sizes.
- vendor/photag-source/src/main.rs:1020-1097: top toolbar ordering, search sizing and filter rows.
- vendor/photag-source/src/ui/gallery.rs:1383-1429: square tile geometry, hover and selection rules.

The concept retains the screenshot's nine-item library and control inventory. It explores stronger typography, aligned controls, clearer surface hierarchy and restrained emphasis. Image generation is not a source of implementation measurements. Its visible file_manager border/material shading and cyan outlined hover are concept deviations, not changes to the canonical source rules; actual migration work must follow the documented source tokens and distinguish hover from selection. Other screens and hidden interaction states are outside this screenshot concept.

## Final generation prompt

Use case: ui-mockup.
Asset type: complete high-fidelity FileAtlas Windows desktop interface redesign concept, one full application screenshot.
Input image: attached screenshot is the edit target and authoritative functional inventory. Improve the entire visual styling while preserving FileAtlas identity, the same nine media items including the two neutral audio/video placeholder cards and seven colorful abstract test thumbnails. Do not replace the library with scenic photographs.
Create a refined, professionally crafted dark media management application, crisp pixel-sharp UI, straight-on screen capture, no perspective, no laptop frame, no surrounding presentation board. Landscape full desktop window, approximately 1942x1256 aspect ratio. All four window edges and complete status bar visible.
Keep the full functional layout: slim native titlebar with FileAtlas and minimize/maximize/close; first toolbar row FileAtlas wordmark, centered Search your library field, right thumbnail size slider, Tools button and gear Settings; second row in exact order Newest first dropdown, active Months chip, All/Photos/Videos/Audio segmented selector, Extensions control, From and To date controls, star rating dual handle slider with 0–5 label. Left sidebar Folders and Tags tabs, FOLDERS heading with add button, selected All files with count 9, collapsible fileatlas-fixtures with count 9. Main file_manager single elevated charcoal panel with collapsible October 2026 heading and 9 items, nine square tiles in six-column first row and three-column second row, neutral audio and video placeholders then preserve attached colorful abstract image content. Bottom status 9 items in library.
Visual refinement: use the existing source design language carefully: base grayscale RGB25, panel37, raised controls46; white235 primary,160 secondary,112 muted. Slate blue RGB90,127,160 accent and softer selected fill; meaningful clear hierarchy, clean Segoe UI typography, real semibold headings, consistent thin Windows line icons. Subdued surfaces, restrained soft shadows, no bright neon/glass effects. Search pill and filter pills aligned with exact consistent baseline. Source logical sizes: search height32, max width540, filters26, gap6; sidebar nominal220; thumbnail radius7, gap12, file_manager radius10, inner inset14; body13.5, small12, month title19. Scale all logically together to the rendered screen. Sidebar selected row26 logical height, 3-wide accent marker. Preserve only one elevated file_manager container; chrome rests on base, no separate raised sidebar card or fake separator panels. Keep media tiles square and metadata within their image surfaces, no new caption cards underneath. Clear consistent icon alignment; inactive controls remain legible. Nine-item library should still leave substantial calm negative space below, rather than inventing extra content or features.
Show a subtle hover state on one colorful image with a soft shadow and filename overlay inside the lower image area, while keeping library idle count and no selected-media bulk-action row. Keep the nine thumbnails recognizable. Excellent polished actual desktop UI, restrained, practical and implementable. All English labels spelled correctly. No watermark, no explanations or annotations inside the image.

