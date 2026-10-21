# WorkCrate development plan

> 2026-10-20 最新用户澄清：页面固定 A3，只有页内信息条和下方图片排版区。
> 页外尺寸字段是自由标记，不改变实际页面；图片仅移动、等比手柄缩放、可恢复裁剪。
> 原图本地链接 + 独立 PSD 等扩展源文件链接；支持项目一键更新。
> 此决定覆盖下文旧的通用画布、尺寸调整和整页平移描述。
> 结构与行为见 `docs/reference-ui/concepts/workcrate-simple-layout-2026-10-20.md`。


Status: active development on 2026-10-20. This plan turns the user's chosen WorkCrate
scope and board reference into implementation stages. It deliberately maps
canvas, image, master-page, PDF preview and PDF creation behavior to the pinned
FigCraft, DesignCraft and PdfCraft source checkouts. Baseline clarification
(2026-10-20): projects may reference
any file type, but boards accept only supported image files (initially JPEG/JPG
and PNG); preview and export render from source pixels without destructive
downsampling or enlargement of a card thumbnail.

## Implementation rule

Reuse the upstream engine crates directly at their pinned Git revisions when
their public/core crate can build without `egui`/`eframe`. If a WorkCrate data
shape requires a narrow adaptation, keep that adaptation next to the owning
WorkCrate crate, retain upstream notices and name the upstream file/function in
the source comment and PORTS.md. Do not copy any upstream UI. Do not introduce
a second canvas renderer, a custom image-fitting formula or a new PDF writer.

## Implementation progress — 2026-10-20

Later user corrections supersede the initial slice below: the product has three
levels (projects → pages → editor), with the inspected three-screen concepts in
`docs/reference-ui/concepts/workcrate-three-level-2026-10-20.md`. Project cards
show only title/cover, with create/rename/cover/duplicate/archive/restore/delete,
search and sorting. Product page creation and default exports use A3 landscape
420 × 297 mm; first image fills the filename and actual pixel dimensions, not
physical page size. Global project name is inherited with per-page overrides.
The editor has page thumbnails, canonical DesignCraft canvas, and information/
content/images inspector. PNG/JPEG and ordered multi-page PDF now have service,
UI and Full-access MCP routes. Native control and screenshot evidence is recorded
separately from remaining handle/crop/master and native-dialog acceptance.
The latest focused coverage has 13 passing tests. Native screenshots verified
project menu/entry, page selection, global-name draft, persisted DesignCraft drag,
small-window fit and the corrected GPUI BGRA image adapter. All UI modules are
kept below 500 lines after separating canvas, inspector and image adaptation.

- Source pins and three upstream core crate groups compile on Windows. The first
  service slice is implemented: versioned project/reference JSON, DesignCraft
  board placement/render, and single-page PDF export through PdfCraft. Nine
  focused service/contract/UI tests pass. Upstream license aggregation and the
  remaining transitive distribution notices still need review.
- First UI slice is implemented as `fileatlas-plugin-workcrate-ui`: project
  selection/creation, arbitrary file references, three-column board catalog,
  JPEG/JPG/PNG placement, zoom controls, and one-page PDF export. Same-process
  MCP opened the panel and visually confirmed project creation, empty states,
  and a board card with dimensions. Project card/source preview selection is
  not yet surfaced in the UI.
- Canvas fit follows FigCraft `engine::ViewState::fit` (64pt margins, a 50pt
  minimum usable axis, fit capped at 100%), using measured WorkCrate panel
  content size and current GPUI device scale. Same-process MCP screenshots now
  confirm the entire A4 page fits and displays a placed PNG and 1024×768 JPEG.
  A confirmed Full-access MCP PDF export returned 175.54 DPI / 1451×2053 px;
  Poppler reopened the one-page A4 PDF and its rendered page was inspected.
  Native dialogs and camera-resolution sharpness remain unverified.
- Still open: PDF first-page thumbnails through the existing Thumbnail service;
  custom preview UI and PDF preview; board/master model and editor transforms;
  PNG/JPEG export; ordered multi-board PDF; failure/relink handling; and full
  lifecycle/restart/MCP acceptance. Single-page PDF backend does not close the
  ordered multi-board delivery stage.

At the source-integration spike, verify each dependency graph and Windows build
compatibility. Prefer this ownership:

| Need | Primary upstream implementation | How WorkCrate uses it |
|---|---|---|
| Board/page geometry and presentation masters | DesignCraft `designcraft-doc` (`crates/doc/src/page.rs`, `build.rs`) | Reuse its page/spread and `ParentInfo::based_on` inheritance shape, including base-first parent resolution, cycle checks and per-page visibility/override behavior. Keep WorkCrate's stable file-reference IDs in its own manifest. |
| Board interaction and image fit/crop behavior | DesignCraft `designcraft-tools` and `designcraft-render`; compare FigCraft `figcraft-engine` (`crates/engine/src/tools.rs`, `cmd/create.rs`) and `figcraft-render` (`crates/render/src/paint.rs`, `images.rs`) | Drive pointer/key edits through the engine command boundary. Reuse source transforms, fit/crop modes, decode limits, failed-image handling, mip/image sampling and bounded decoded-pixel caches. Board items link only supported image files or selected image previews. Adapt only the GPUI view adapter. |
| Board overview and page raster | DesignCraft `designcraft-render` (`crates/render/src/lib.rs`, `images.rs`) | Render one resolved page model to a requested viewport or output density. Same page model feeds previews and export; display sampling follows zoom/device-pixel scale and export starts from original source pixels. No image IO or decode runs from GPUI render. |
| Existing PDF thumbnail / first-page preview | PdfCraft `pdfcraft-render` (`crates/render/src/raster.rs` and render request/tile types) | Expose first-page thumbnail generation through the existing Thumbnail service, using PdfCraft's request scheduler, output sizing, tile behavior and render cache. Do not create a PDF parser in WorkCrate. |
| PNG/JPEG canvas images and multi-page PDF from those images | PdfCraft `pdfcraft-create` (`crates/create/src/lib.rs::from_images_with_resolution`) | Encode selected boards' rendered full-resolution pixels as PNG/JPEG bytes and feed them in the user's chosen order; each input becomes one PDF page at embedded or specified DPI. Use `pdfcraft-organize::combine_selected` only if a later requirement combines existing PDFs or selects pages from source PDFs. |

All three projects' current desktop UI crates are egui/eframe. FileAtlas's
frontend remains GPUI/GPUI Kit. The desired reference is the Rust document,
canvas interaction, image, rendering and PDF code, not the source applications'
window/panel implementation. If a core crate pulls egui transitively, remove it
from the proposed dependency set before implementation and use the narrowest
source crate or port its engine-side module with attribution.

The checkouts and commits are recorded in
[the architecture proposal](workcrate-architecture-proposal.md). Per-file and
transitive third-party notices must be checked before adding a source crate to
Cargo.lock; the repo root license alone is not sufficient for bundled assets.

## Plugin set and contracts

- `fileatlas-workcrate-contracts` (plain shared crate): stable `ProjectId`,
  `AssetId`, `BoardId`, `TemplateId`; request/result payloads and Cordis event
  types. It contains no GPUI views and opens no files.
- `fileatlas-plugin-workcrate-projects`: project CRUD, persisted file-reference
  records, chosen custom preview, missing/relink state and project overview
  launcher. It registers lifecycle-owned UI through UI Bridge.
- `fileatlas-plugin-workcrate-boards`: board model, template/master inheritance,
  editor commands, canvas view adapter and canonical board renderer service.
- `fileatlas-plugin-workcrate-export`: selected-board export flow, image/PDF
  save dialogs, progress/cancel/errors and atomic output handoff. It consumes the
  resolved model/render service and PdfCraft core crates.
- Existing `fileatlas-plugin-thumbnails`: extend its public thumbnail request
  to PDF first-page previews through PdfCraft. Keep image/video paths and their
  current cache policy intact.

Existing plugin responsibilities:

- FileManager publishes selected source paths or handles a WorkCrate add-reference
  action through a shared request/event. It does not import WorkCrate private
  models.
- File Operations remains responsible for open/reveal/copy/move of original
  files. Removing an asset reference is a JSON/model mutation and never deletes
  the original file.
- QuickLook remains the quick viewer. PhotoCraft editing is an explicit
  handoff integration, with no install/update/terminate behavior.
- File Staging remains temporary transfer state. Program Workspaces remains
  startup pane layout state. Neither stores WorkCrate project/board data.
- UI Bridge owns common panels, theme access, commands, settings registration
  and contribution cleanup; Cordis owns startup dependencies and stop cleanup.

## Stage 0 — source pin and engine compatibility spike

Read the pinned source crate manifests and build modules, not just their
READMEs. Resolve the upstream dependency graph for each candidate and record
which crates are core-only. On a scratch crate or isolated local branch, confirm
Cargo can resolve the exact pinned `git` revision without an egui/eframe
dependency path. Check GPUI's image buffer type against the upstream renderer
output and choose the narrowest zero/low-copy conversion path.

Decide one canonical board document model and one renderer in this spike:
DesignCraft is the primary candidate because its page-master model and page
output are oriented around publishing. FigCraft is the exact reference for
canvas manipulation, image placement and viewport behavior; do not put both
complete scene/render engines in the runtime unless the source audit proves
they compose without duplicating image/document state. Use PdfCraft for PDF
source previews and generated PDF pages from the common rendered images.

**Exit:** a short module/dependency map records the exact upstream Rust crates
that build core-only, any source files that need a compatibility adaptation,
crate license/NOTICE files, and a tiny rendered board image plus PDF page using
the pinned APIs. If this spike cannot build without egui, isolate and port the
specific core module under its original license before moving on.

**Progress (2026-10-20):** `cargo +1.97.1 tree --locked -p <crate> -i egui`
confirmed there is no `egui` package in the normal dependency graph for
DesignCraft `designcraft-tools`/`designcraft-render`, FigCraft
`figcraft-engine`/`figcraft-render`, or PdfCraft `pdfcraft-render`,
`pdfcraft-create`, and `pdfcraft-organize`. Locked Windows `cargo check -j 2`
passed for all three selected crate groups. The FigCraft build emits its
upstream warning that optional `craft-fonts` resources are absent from the
adjacent checkout; the selected Rust crates still compile. The tiny output
The WorkCrate boards service now uses DesignCraft document placement and
rendering and PdfCraft page creation/writing. Its focused tests exercise
source-image rendering, PDF output, and replacement of an existing destination.
The GPUI pixel-buffer adapter and same-process MCP UI/export checks are recorded
in the WorkCrate validation report. Stage 0 remains open for upstream license
aggregation and broader UI/source coverage. DesignCraft `DisplayQuality::High` and its `images::mip` choose
the closest cached source mip still above the on-screen device-pixel demand;
PdfCraft `ImageResolution::Dpi` sets PDF page size from the supplied image pixel
dimensions and does not resample those pixels. A first WorkCrate vertical slice
now persists arbitrary file references, enforces real JPEG/PNG decoding at board
placement, renders DesignCraft pages at requested pixels-per-point, and exports
through the separate PdfCraft service. Focused board/project/export tests pass;
the board UI and single-board PDF flow are verified through same-process MCP.
PDF preview integration, image save flow, masters, transforms, and multipage
export remain open.

## Stage 1 — project registry and durable file references

Implement `fileatlas-workcrate-contracts` and the projects plugin. Store a
versioned JSON manifest at
`%LOCALAPPDATA%\\FileAtlas\\workcrate\\projects.json` and project board data
in feature-owned project subdirectories. Use stable UUIDs, atomic same-folder
replace, backup recovery and unknown-version rejection. Original files remain
at their existing paths; WorkCrate stores the path and last-known metadata.

Add references from FileManager selection and an arbitrary-files picker. Every
reference supports a user-chosen display title, file number/order, optional
custom preview file and quick open/reveal. The reference itself may be any file
type, while board placement accepts supported raster images only (initially
JPEG/JPG and PNG). Other file types can be represented on a board through an
attached image preview. Source availability/path changes
surface a missing/relink state while preserving board links. A custom preview
is copied into a feature-owned preview directory because its source may be
external and mutable; record its relative path, hash and dimensions. Removing
the project reference does not remove the copied preview unless the user also
removes that preview record and no board references it.

**Exit:** create/open/duplicate/rename/archive project, add/remove/relink
arbitrary files, assign/replace/remove a custom preview, restart persistence,
missing source handling, and original-file preservation all work through
Cordis/UI Bridge and isolated FileAtlas MCP.

## Stage 2 — thumbnail and preview pipeline

Extend the existing Thumbnail service with a typed PDF first-page request. Use
PdfCraft `pdfcraft-render`'s `PageRenderer`/`RenderPool` and `RenderRequest`
types (including their size caps, bounded work queue, tile and preview request
semantics) behind FileAtlas's existing worker/read-model and cache facilities;
do not copy the egui canvas scheduler. Return a cache path plus dimensions. Add
stale-key invalidation on file size/modified-time change and a cached failure
result so a corrupt PDF is not parsed each frame.

For each WorkCrate file reference, choose its overview preview in this order:
user custom preview, existing FileAtlas thumbnail, PdfCraft first-page preview
for PDF, then the existing file-kind icon/empty preview. Do not decode unknown
formats in WorkCrate. Board placement always refers to the stable `AssetId`,
then resolves to the chosen image source at render time.

**Exit:** image, PDF, custom preview, unavailable/corrupt PDF, missing custom
preview and generic non-image references all display consistently in project
overview. Board placement accepts JPEG/JPG and PNG, and non-image references
appear on a board only through a chosen image preview. FileManager thumbnails
retain existing behavior.

## Stage 3 — board document and source-aligned canvas behavior

Represent each board as one DesignCraft page (or the equivalent smallest
document unit validated at Stage 0), with physical width/height/unit and an
optional parent/master. Parent/master fields include the shared project label,
header/footer, border, metadata placement and page number. Child boards inherit
these fields through the source `based_on` chain; local overrides are stored
separately and survive template edits. Reuse DesignCraft's cycle/missing-parent
validation and base-first draw order.

Represent placed images as items with stable item IDs and `AssetId` links.
Validate that a direct placement is a supported raster image (initially
JPEG/JPG or PNG); a non-image reference may be placed only through an attached
image preview. Reuse the pinned source fit/crop transforms and image-scale
semantics; keep the original file reference separate from the selected preview
image.
Text/label items carry only the WorkCrate fields required by the board design.
Do not recreate a full vector editor, masks or PSD layer model.

Build the overview screen to the user reference: slim full-width top chrome;
spacious three-column board-card grid; number and centered title above each
frame; a summary row with source name/dimensions/count; inside the frame, a
metadata strip and a proportional, clipped page preview. Rows remain
start-aligned with open space when incomplete. Open a board in an editor view
with canvas centered and zoom/pan controls; keep toolbars and property controls
in GPUI Kit patterns and FileAtlas's shared theme.

Use source command operations for add/place, move, resize, duplicate, reorder,
delete and master assignment. GPUI pointer events adapt into these commands;
they do not mutate document fields from a paint callback. Keep an undoable edit
as one command transaction, following the selected source engine's history
boundary.

**Exit:** create/duplicate/reorder/delete boards; set physical dimensions;
assign a master; edit an inherited field and observe all inheriting boards
update; override one board; add several different file types; crop/fit/position
each supported image or chosen preview; reject direct placement of non-image
files; reopen the project and retain all geometry and order. Check empty,
selected, hover/focus, missing asset, long title and narrow-window states against
the reference image through same-process MCP screenshots.

## Stage 4 — one resolved board renderer for preview and image output

Provide `render_board(board_id, scale_or_dpi, region)` as the shared board
render service. Resolve master fields and image assets into an immutable
snapshot, then hand that snapshot to the selected upstream renderer away from
GPUI render. Use DesignCraft's source renderer for page/item traversal, image
decode, clip/compositing and cache behavior; use the FigCraft pinned image
transform/canvas viewport behavior where the source audit chooses those exact
algorithms. Editor sampling follows the visible zoom and device-pixel scale,
selecting sufficient source resolution for sharp visible detail while keeping
the stored original untouched. Add only a GPUI adapter for displaying returned
pixels and pointer-to-page coordinate conversion.

Project overview cards request a small cached viewport. An open board requests
the visible viewport and replaces stale preview pixels when a newer model
revision finishes. Keep DesignCraft's High display quality; use its mip choice
for the current viewport so the selected source level is not below visible
device-pixel demand. PNG/JPEG export re-renders from original source pixels and
board transforms, never enlarging the small overview thumbnail. Derive the
default export pixel dimensions from the board size and the actual placed
images' effective source resolution, bounded by PdfCraft's page/pixel limits;
allow an explicit DPI override only with a clear warning when it exceeds the
source images' effective resolution. Report unsupported/corrupt image source
paths with the asset ID and retry/relink action.

**Exit:** the same resolved board produces matching content/order/crop in the
overview, editor and full-size PNG/JPEG output. Compare generated pixels to
upstream golden/fixture expectations for fit, crop, orientation, alpha,
high-resolution sampling, missing/invalid images and page clipping. Confirm
quiet-state rendering does not perform repeated file reads or notifications.

## Stage 5 — PNG/JPEG and ordered PDF delivery

Add single-board PNG/JPEG export using the Stage 4 renderer and native save
dialog. For multiple selected boards, preserve the explicit order shown by the
selection/reorder UI, render each through the same Stage 4 renderer, then call
PdfCraft `pdfcraft-create::from_images_with_resolution` on those encoded image
buffers with the chosen output DPI. This gives one image-derived PDF page per board and
preserves exact board appearance. It avoids a second layout implementation in
the PDF path. A later request to combine existing PDFs can use
`pdfcraft-organize::combine_selected`; it is outside this initial delivery
flow.

Write output to a same-directory temporary file and atomically replace only
after render, PDF creation and file flush all succeed. The first PDF service
slice now follows this handoff and tests replacement of an existing destination.
Keep originals intact,
surface any missing input before export where possible, and report render/PDF/
save failures with a retry path. Long batches support cancel between pages and
discard the incomplete temporary output.

**Exit:** one board exported to PNG/JPEG; selected boards exported to an
ordered, multi-page PDF; verify page count, page dimensions/DPI, order, output
reopen/render, source preservation, cancellation, overwrite confirmation,
existing destination and failed-write cleanup through the FileAtlas MCP and
real Windows save dialog.

## Stage 6 — lifecycle, recovery and acceptance

Register every project panel, board editor, command, event handler and
background renderer under Cordis/UI Bridge lifecycle ownership. Disable each
WorkCrate plugin and verify its registrations, tasks and open surface clean up
without affecting FileAtlas core. Update each new plugin's PORTS.md with
identity, dependencies, payloads, storage, migration/recovery and current
limits; update source behavior mappings and feature ledger.

Run project governance checks and required Rust fmt/locked checks for changed
crates. Then use the same-process FileAtlas MCP with an isolated profile to
exercise the whole project → add files → choose custom previews → edit boards
→ export PNG/PDF → restart/reopen flow. Record exact commands, profile,
screenshots, interaction states and remaining Windows boundaries. A successful
build alone is not UI acceptance.

## Scope after the first usable release

Only after the first export vertical slice: reusable customer template library,
automatic gallery/grid layout, batch numbering, optional delivery package with
source-file list and richer board object controls. Keep the initial feature
focused on referenced files, page-sized boards, inherited presentation
metadata and dependable image/PDF output.


## 2026-10-20 性能实现补充

| 源码映射 | 本次复用与调整 |
|---|---|
| DesignCraft `render/src/lib.rs` Rendered/Renderer | 继续同一页面/裁剪渲染；直接 to_straight 交给图片编码，去掉 PNG 往返 |
| DesignCraft `render/src/images.rs` decoded/mip | 保留导出的原图 mip 路径；编辑器单页显示代理独立于原始 Asset.data |
| DesignCraft `engine/src/cmd/file.rs` file.place | 原图链接及原始像素元数据保留，不将显示代理写回文档 |
| PdfCraft `create/src/lib.rs` png_image/from_images_with_resolution | 保留无损图像建页、页面组织和原子输出；优化其 CPU 构建配置及 zlib-rs 压缩后端 |
| FileAtlas UI Bridge read_model/refresh | 当前页代理按文档修订和显示分辨率合并读取；有限导出阶段采样停止后无空转 |

页面缩略图增量保存到项目 cache-v1，启动先读缩略图，缺失逐页后台补齐；
无需在退出时重新加载全部原图。详细缓存格式、失效与恢复见 Boards PORTS。
性能、重启缓存和导出进度证据见本次性能验证记录。

## 2026-10-21 文本直接编辑与局部刷新

本次用户纠正：文本不能只在右侧输入、点击保存后整页重载。实现改为新增直接聚焦画布、
双击已有文本继续输入；原生多行输入与右侧表单共享状态，字号/颜色/宽高实时保存。
四角只改变文本框，按原生排版换行；拖动输入框上沿移动，保持字形比例。

| 上游/已有入口 | 对应实现 |
|---|---|
| DesignCraft engine frame.create、text.select/text.insert、type.char、swatch.create | Boards text.rs 保持原生故事/字号/颜色和真实文本框，固定 Microsoft YaHei |
| DesignCraft engine object.fit/fittingOptions + links.relink | 替换/刷新均中心 fillProportionally，原有框/裁剪不变 |
| DesignCraft render Renderer::render(paper=false) | 单文本透明图层，保存仅替换该层；A3 模板和照片纹理复用 |
| GPUI Kit TextareaState + InputEvent::Change | 原生光标、选区、多行输入；画布/检查器仅一个实体参与渲染 |
| UI Bridge refresh::poll + ReadModel | 单后台写入队列合并草稿、600ms 安静后缩略图更新，空闲停止；照片缓存键排除文本变化 |

FieldPresets 和批量选页继续复用已有 Projects/Settings/UI 服务，未新增基础设施。
来源归属、持久化兼容及限制见各 WorkCrate PORTS；[实际窗口记录](validation-ui/2026-10-21-workcrate-inline-text/README.md)。

## 2026-10-21 选区、完整原生属性目录、默认信息自动同步

本轮保留三栏/A3，只在原检查器内增加常用格式和可折叠的“更多文本设置”。
输入态去掉底层框的鼠标命中，原生输入负责拖选与替换，上沿/手柄保留对象操作。

| 参考源码入口 | 原生复用与本轮结果 |
|---|---|
| DesignCraft doc/attrs.rs、doc/item.rs | 自动读取110字符、117段落、27文本框属性；补出serde默认省略的7个可选框字段 |
| DesignCraft ui-egui/panels/properties.rs、typography.rs | 常用格式/对齐、字体样式/行距/字距/缩进/分栏；四组属性搜索与字体实际提供的OpenType特性/样式集 |
| DesignCraft engine/cmd/text.rs | type.selectionAttrs/type.char/type.para/type.feature、原生bold/italic；有效UTF-8选区可独立格式化 |
| DesignCraft engine/cmd/object.rs | object.textFrameOptions保留原生几何副作用，补充选项仍由原生TextFrameOptions承载 |
| DesignCraft doc/story.rs | 输入只替换变化字节区间，保留未改动字符run/段落格式；不再全量重建故事 |
| FigCraft ui-egui/style_picker.rs、font_preview.rs | 核对字体/样式/字号、行距/字距和文本装饰；由DesignCraft原生属性覆盖 |
| PdfCraft ui-egui/bulk_fields.rs、comment_props.rs | 核对PDF文本外观；PDF特有表单/批注编辑未引入A3排版页 |
| FileAtlas SettingsPage + Projects FieldPresets | 字段/默认信息有效修改自动保存，空白编辑行不入库；编辑菜单读服务内存快照，连续添加/删除不受后台保存禁用 |

更多设置支持原生复合值JSON，快访问字号/颜色继续按整框应用；复杂发布排版由DesignCraft呈现，
输入态不声称复制全部多栏/CJK排版细节。属性覆盖与逐项交互覆盖分开记录。
[本轮实际窗口及完整原生属性验证](validation-ui/2026-10-21-workcrate-typography/README.md)。
