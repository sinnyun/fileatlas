> 最新语义更正：用户明确要求固定 A3、顶部页面内容信息条 + 下方图片排版区。
> 页面信息中的尺寸只是标记；不再修改页面尺寸或平移整页。
> 图片仅移动、等比缩放、可恢复裁剪；本地图片链接 + 独立扩展源文件链接。
> 本节下文的物理尺寸编辑、整页平移等旧描述由
> `workcrate-simple-layout-2026-10-20.md` 覆盖。

# WorkCrate three-level reference

## 2026-10-22 interaction optimization checklist

### Material header / page fields follow-up

The inspected October 21 studio user sketch is the material-header target:
compact pale horizontal band, left page number and prominent project name,
right material / dimensions / quantity on the same baseline. Additional fields
flow into subordinate value + label rows below the primary row. The
inspector concept confirms horizontal field labels and values with aligned inputs.
Size annotation remains page metadata, independent of the actual artboard size.
Existing artwork and dimensions stay in place when the owned header is upgraded.

Page-field saves reuse this page's warm native session; browsing still opens no
official process, and a cold save silently warms the session itself, keeping the
page in preview. Render only the changed page, decode the official frame off the
UI thread, patch only its card/frame, retain the inspector/rail/scroll and allow
the next field draft while saving. Keep failure drafts and actionable save
feedback. No whole-project reload, placeholder frame or field-row reflow after
each save.

The right inspector groups fields into two cards matching the sketch: 页面信息 ·
画板规格 and 页面内容 · 同步物料栏, with fixed-height value rows, an inline draft
row, a fixed status footer and a full-width 新增物料参数行 action. Validate
consecutive edits, native persistence, inheritance/reset, extras, no-op edits,
unaffected page cache identity, stable row geometry and quiet-state using
same-process MCP.

**Status 2026-10-22: implemented and validated.** The band sites every text box
from the official `document.node` metrics, field saves warm/reuse that page's
session, touch only that page's card, frame and cache file, and decode the frame
before the UI applies it; the inspector keeps its rows in place with Chinese save
feedback. The 11-stage header run, the inspector-layout scenarios, the two-pass
geometry contract and the remaining uncovered states are in
[the header validation record](../../validation-ui/2026-10-22-workcrate-header/README.md)
and [the inspector-layout record](../../validation-ui/2026-10-22-workcrate-inspector-layout/README.md).

Inspected the active October 21 projects/pages/studio images, inspector image and
October 22 canvas concept. Preserve the three-level layout, horizontal toolbar,
left page rail, flexible official bitmap and independently scrolling right panel.
This is a local interaction correction, not a new visual design.

- Preview/edit share the latest official frame after an explicit editing session;
  update the rail and durable cached page preview after edits. Passive initial
  browsing still opens no official process. Close sessions on page/project exit.
- Keep existing page images visible during background reload, prioritize the
  selected page, and avoid decoding every project cover on each canvas operation.
- Preserve the final position of long gestures; gate edits while an operation is
  pending and discard stale probe/dialog results after navigation.
- Right inspector: multiline text, contextual enabled states, finite numeric
  bounds, selection-aware field reset, readable linked/embedded status. Preserve
  paired X/Y and width/height rows, real error messages and scroll access.
- Validate edit → preview → edit, rail/matrix refresh, text/image edits, undo/redo,
  navigation, quiet-state and small-window behavior through same-process MCP.

The latest user sketches supersede the earlier two-level WorkCrate slice.
Authoritative references: `workcrate-projects-user-2026-10-20.png`,
`workcrate-pages-user-2026-10-20.png`, `workcrate-editor-user-2026-10-20.png`.
The rectangles identify regions; they are not required product borders.
`workcrate-editor-study-2026-10-20.png` is an inspected imagegen layout study,
not a replacement acceptance target. Its image-generation prompt requested a
central editable page and fixed right inspector with page information, content,
and image rows. The latest sketch also requires a left page-thumbnail column.

## Implementation checklist

Inspected implementation concepts (imagegen built-in):
`workcrate-projects-concept-2026-10-20.png`,
`workcrate-pages-concept-2026-10-20.png`,
`workcrate-editor-concept-2026-10-20.png`.
The generation prompt set is archived in `workcrate-concept-prompts-2026-10-20.md`.
Prompts specify the three-level navigation, native neutral theme, restrained
selection states, project-only title/cover cards, A3 landscape pages, selection
and export footer, left page rail and right metadata/image inspector. Source
pixel dimensions are factual metadata; the editor concept's input appearance
must not permit falsifying them. Artwork/text in concepts are illustrative.

- Project management: responsive three-column project cards show only project
  name and main-image preview; no page metadata. Top search/sort/archive controls
  and add-project action; per-project menu opens, renames, changes cover,
  duplicates, archives/restores, or deletes the project.
- Page management: project title, responsive three-column page cards in stable
  order, page info above each preview. Select pages independently from opening
  them; footer exports selected pages as images and all pages to ordered PDF.
- Page editor: left scrollable page thumbnail rail, central page with the name,
  physical size and source pixel size above it, right scrollable inspector.
- Inspector order: page information, page content, page images. Fields remain
  editable; image rows pair a preview with source name, linked file, physical
  size, pixel size, update/replace/edit actions.
- Page information defaults from the first added raster image: filename and
  real pixel dimensions. Physical dimensions are separate editable values.
  The user confirmed pixel dimensions, not DPI/PPI, on 2026-10-20. Page physical
  size and default output use A3 landscape 420 × 297 mm independently of images;
  the first image does not resize the page.
- Page content inherits the project's global name unless a page stores a local
  override. Reset restores inheritance. Material, size, quantity and added fields
  are page data. They appear in the exported page header.
- Only the page with its content header exports; page information above it and
  surrounding navigation/inspectors do not export.
- Canvas edits use DesignCraft page/image/transform commands and renderer.
  Source images remain the rendering/export input at viewport/output density.
- Canvas viewport measurements resolve the actual host panel instance and the
  center viewport bounds, including after resize. The public panel definition
  ID alone is not an instance geometry key. Reuse FigCraft fit on those real
  bounds; do not keep a fixed desktop-fit scale in a smaller window.
- Use GPUI Kit native theme roles, shared controls, subtle separators, and
  selection cues rather than the sketch's heavy colored outlines.
- Empty, selected, editable, missing/error, long-name and narrow-window states
  need same-process MCP verification; no visual pass from compilation alone.

## 2026-10-20 user-reported rejection and correction checklist

The previous service-level proof did not establish that the visible add-image
control or FileManager drop worked. The user rejected its visual fidelity.
Keep the original three concepts as targets; do not replace them.

- Increase the default panel to 1560 × 980 logical pixels, preserving host
  floating-panel chrome, maximizing and resizing. Default is three cards across.
- Projects: subtle shared card, strong title and adjacent menu, tall cover,
  a full-width add-project tile. No page metadata on project cards.
- Pages: filename, physical size, real source pixels and open action on one
  row; selection frames the whole card; page image below; export footer fixed.
- Editor: breadcrumb above all columns; command strip above the left/center
  workspace; inspector starts immediately below breadcrumb. Left rail 180,
  right inspector 310 logical pixels, flex center. Add-page action in rail.
- Reduce fit padding from 64 to 24 logical pixels per side by supplying an
  adjusted viewport to the unchanged FigCraft fit function. Name/size/pixels
  stay outside the exported A3 artboard; printable pale information band inside.
- Inspector image entries use separators, not large bordered cards. Inline
  fields stay horizontal; only inspector/rail scroll at narrow sizes.
- Add image opens an application-owned path entry with native Browse, Add and
  Cancel. Input supports quoted paths and semicolon-separated multiple files.
  Picker and drop use the same canonicalized original-image importer.
- Canvas accepts shared FileManagerFileDragPayload, FileStagingDragPayload and
  GPUI ExternalPaths. Actual FileManager-card pointer drag must persist a new
  DesignCraft placement; a backend place_image call is not this proof.
