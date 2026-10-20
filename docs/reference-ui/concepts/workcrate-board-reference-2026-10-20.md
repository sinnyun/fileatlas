# WorkCrate board overview reference checklist

Reference image: [workcrate-board-reference-2026-10-20.png](workcrate-board-reference-2026-10-20.png).
The image is the user's visual direction for the project overview and board
catalog. It is not pixel data or sample project content.

## Layout hierarchy

- Use the host window's narrow full-width title/chrome row. The sketch's purple
  strip and red rectangles only mark a title bar and window controls; use the
  native FileAtlas window and GPUI Kit theme.
- Keep a broad, light main work area beneath the title row. Place board cards in
  a responsive three-column grid at normal desktop width. Start each incomplete
  row at the left and leave remaining space open; cards keep a consistent size.
- Above every board frame, align its ordinal at the left and its editable board
  title centered. Put the short source / physical-size / image-count summary
  between or below them as shown in the reference hierarchy.
- Give each board preview a strong rectangular frame and a compact metadata
  strip along its top edge. Show project or section title, physical dimensions,
  and placed-image count from the actual model.
- Preserve the image's aspect ratio within the page preview and clip to the
  frame. Keep the metadata strip outside the artwork region so it never covers
  placed image content.
- Use shared GPUI Kit spacing, typography, borders and theme roles. Do not copy
  hand-drawn border thickness, placeholder text, or the colored window controls.

## Content and placement rules

- Project references can link any file type and appear in project management
  with a file-kind preview or a user-selected image preview.
- Direct board placement accepts supported raster images only; the first
  implementation supports JPEG/JPG and PNG. A non-image project file can be
  shown on a board only through its attached image preview.
- Canvas image transforms are non-destructive. Source files are never modified
  by fit, crop, resize, display sampling, or export.
- Canvas display uses the pinned DesignCraft renderer at High quality and its
  viewport-derived mip selection. FigCraft `engine::ViewState::fit` supplies
  the fit scale (64pt margins, a 50pt minimum usable axis, capped at 100%);
  current zoom and device scale determine render density. Overview thumbnails
  never become the source for canvas or export pixels.
- Export re-renders from original source images and saved transforms. Its
  default output pixel dimensions follow board physical size and the actual
  placed images' effective resolution, within PdfCraft's limits. An explicit
  higher DPI can be selected with a warning that the source cannot add detail.

## Required states

- Empty project and empty board; one board; several boards; partial final row.
- Selected board, hover, keyboard focus, and multi-board export selection.
- Long project, board and source names; missing source; missing custom preview;
  unsupported or corrupt image; relink and retry states.
- Narrow window: reflow the grid while preserving card order and keeping each
  board's metadata strip and image frame visible.
- Adding a direct non-image reference to a board must be rejected with a clear
  explanation and an action to choose an image preview instead.
- Zoom in and out on a high-resolution photo and verify that visible detail
  remains sharp. Export the board and verify output dimensions and sharpness
  follow the actual image pixels rather than a small thumbnail.

The reference does not define project navigation, editing controls, exact
breakpoints, or export-dialog layout. Resolve those from the active FileAtlas
0.2 host patterns and GPUI Kit controls before implementing those screens.

## First implementation slice checklist (2026-10-20)

- [x] WorkCrate opens as a host floating panel with native FileAtlas chrome.
- [x] Project selector and create-project action.
- [x] Responsive wrapping board catalog; cards show ordinal, title, preview,
  placed-image count and page dimensions.
- [x] Empty project/board states and a selected board's editor view.
- [x] Canvas toolbar offers zoom, JPEG/JPG/PNG placement and PDF export.
- [x] Image placement and export use the board service's original-resolution
  data path; board thumbnails are only for catalog previews.
- [x] Same-process overview and editor screenshots; fixture image placement and
  source-resolution PDF export were visually inspected through MCP and Poppler.
- [ ] Camera-resolution detail, native image/save dialogs, small-window reflow,
  zoom interaction, and source relink/recovery checks.
- [ ] Editable title/placement transforms, source relinking, preview selection,
  keyboard focus/hover review, narrow-window reflow, and multi-board export.

The source geometry and title are shown from the model, but this first UI slice
does not yet expose editing controls for them. The canvas currently places a new
image at its default position and width; drag/resize handles are a follow-up.
