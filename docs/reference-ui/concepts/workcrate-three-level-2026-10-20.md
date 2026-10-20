> 最新语义更正：用户明确要求固定 A3、顶部页面内容信息条 + 下方图片排版区。
> 页面信息中的尺寸只是标记；不再修改页面尺寸或平移整页。
> 图片仅移动、等比缩放、可恢复裁剪；本地图片链接 + 独立扩展源文件链接。
> 本节下文的物理尺寸编辑、整页平移等旧描述由
> `workcrate-simple-layout-2026-10-20.md` 覆盖。

# WorkCrate three-level reference

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
