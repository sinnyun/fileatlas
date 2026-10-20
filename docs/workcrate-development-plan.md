# WorkCrate development plan

Status: planned on 2026-10-20. This plan turns the user's chosen WorkCrate
scope and board reference into implementation stages. It deliberately maps
canvas, image, master-page, PDF preview and PDF creation behavior to the pinned
FigCraft, DesignCraft and PdfCraft source checkouts. No WorkCrate code is
implemented yet.

## Implementation rule

Reuse the upstream engine crates directly at their pinned Git revisions when
their public/core crate can build without `egui`/`eframe`. If a WorkCrate data
shape requires a narrow adaptation, keep that adaptation next to the owning
WorkCrate crate, retain upstream notices and name the upstream file/function in
the source comment and PORTS.md. Do not copy any upstream UI. Do not introduce
a second canvas renderer, a custom image-fitting formula or a new PDF writer.

At the source-integration spike, verify each dependency graph and Windows build
compatibility. Prefer this ownership:

| Need | Primary upstream implementation | How WorkCrate uses it |
|---|---|---|
| Board/page geometry and presentation masters | DesignCraft `designcraft-doc` (`crates/doc/src/page.rs`, `build.rs`) | Reuse its page/spread and `ParentInfo::based_on` inheritance shape, including base-first parent resolution, cycle checks and per-page visibility/override behavior. Keep WorkCrate's stable file-reference IDs in its own manifest. |
| Board interaction and image fit/crop behavior | DesignCraft `designcraft-tools` and `designcraft-render`; compare FigCraft `figcraft-engine` (`crates/engine/src/tools.rs`, `cmd/create.rs`) and `figcraft-render` (`crates/render/src/paint.rs`, `images.rs`) | Drive pointer/key edits through the engine command boundary. Reuse the source's transforms, fit/crop modes, decode limits, failed-image handling and bounded decoded-pixel cache. Adapt only the GPUI view adapter. |
| Board overview and page raster | DesignCraft `designcraft-render` (`crates/render/src/lib.rs`, `images.rs`) | Render one resolved page model to a requested viewport or output density. Same page model feeds previews and export. No image IO or decode runs from GPUI render. |
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

## Stage 1 — project registry and durable file references

Implement `fileatlas-workcrate-contracts` and the projects plugin. Store a
versioned JSON manifest at
`%LOCALAPPDATA%\\FileAtlas\\workcrate\\projects.json` and project board data
in feature-owned project subdirectories. Use stable UUIDs, atomic same-folder
replace, backup recovery and unknown-version rejection. Original files remain
at their existing paths; WorkCrate stores the path and last-known metadata.

Add references from FileManager selection and an arbitrary-files picker. Every
reference supports a user-chosen display title, file number/order, optional
custom preview file and quick open/reveal. Source availability/path changes
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
overview and board canvas; FileManager thumbnails retain existing behavior.

## Stage 3 — board document and source-aligned canvas behavior

Represent each board as one DesignCraft page (or the equivalent smallest
document unit validated at Stage 0), with physical width/height/unit and an
optional parent/master. Parent/master fields include the shared project label,
header/footer, border, metadata placement and page number. Child boards inherit
these fields through the source `based_on` chain; local overrides are stored
separately and survive template edits. Reuse DesignCraft's cycle/missing-parent
validation and base-first draw order.

Represent placed previews as image items with stable item IDs and `AssetId`
links. Reuse the pinned source fit/crop transforms and image-scale semantics;
keep the original file reference separate from the selected preview image.
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
each preview; reopen the project and retain all geometry and order. Check empty,
selected, hover/focus, missing asset, long title and narrow-window states against
the reference image through same-process MCP screenshots.

## Stage 4 — one resolved board renderer for preview and image output

Provide `render_board(board_id, scale_or_dpi, region)` as the shared board
render service. Resolve master fields and asset previews into an immutable
snapshot, then hand that snapshot to the selected upstream renderer away from
GPUI render. Use DesignCraft's source renderer for page/item traversal, image
decode, clip/compositing and cache behavior; use the FigCraft pinned image
transform/canvas viewport behavior where the source audit chooses those exact
algorithms. Add only a GPUI adapter for displaying returned pixels and
pointer-to-page coordinate conversion.

Project overview cards request a small cached viewport. An open board requests
the visible viewport and replaces stale preview pixels when a newer model
revision finishes. PNG/JPEG export requests the board's physical size and
chosen DPI, validates the allocation limits first, and must not upscale the
small overview thumbnail. Report unsupported/corrupt image source paths with
the asset ID and retry/relink action.

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
after render, PDF creation and file flush all succeed. Keep originals intact,
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
