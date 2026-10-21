# WorkCrate project board and delivery architecture (2026-10-20)

> 2026-10-21：本文的库级 DesignCraft/PdfCraft 集成方案已被用户明确替代。当前直接调用官方 VectorCraft 编译程序，保留 FileAtlas 的 GPUI/Cordis 架构；旧数据无需迁移。以 [当前重构要求](workcrate-vectorcraft-refactor.md) 为准。

Status: requirements and source study recorded from the user's request and the
shared `项目管理插件设计` conversation. This is a proposal for implementation
boundaries, not a claim that the feature is implemented or accepted in every
detail. The attached board overview image is the active layout reference for
the first WorkCrate screen. On 2026-10-20, the user clarified that project
references may point to any file type, while canvas placement is image-only;
canvas rendering and export must preserve quality from original image pixels.

## Product boundary

WorkCrate manages projects, references to original files, presentation boards,
board templates and customer deliverables. It does not copy original files into
project storage, parse or edit every referenced format, or replace PhotoCraft,
Photoshop, Illustrator or other format-specific editors. Users may attach a
custom preview image to any file reference. Removing a reference never deletes
the original file.

The accepted direction is **Option A: project overview plus independent
PhotoCraft file editing**. The project overview shows one project as a visual
board of named and numbered canvas cards. Opening a source file hands it to the
existing OS association/QuickLook path or a future explicit PhotoCraft handoff;
the board itself is for arranging and annotating deliverable content.

## Active screen reference and implementation checklist

Reference: [WorkCrate board overview](reference-ui/concepts/workcrate-board-reference-2026-10-20.png).
This user-supplied image is the chosen structural reference, not product content
or a pixel-perfect size spec. It shows:

- A single project workspace beneath a full-width, narrow top chrome/title bar.
- A broad white canvas area with an airy, responsive three-column board grid.
- Each board entry has a number, a centered editable title and short file/board
  metadata above a strong rectangular frame.
- Board previews preserve the source artwork's proportions and content; a
  collection board may show several related assets together.
- A second grid row begins at the left and leaves unused space open; cards do
  not stretch to fill the row.
- The repeated metadata strip inside each board preview carries project name,
  section/title, physical dimensions and item count. These are real model fields,
  not hard-coded labels.
- The reference's colored title bar and red window controls are sketch cues.
  Actual colors, window chrome and controls use the FileAtlas native GPUI Kit
  theme and host window conventions.

Required states before visual acceptance: empty project, one and many boards,
selected board, hover/focus, long title/path, missing source or preview, narrow
window/reflow, and an export selection. The image does not define exact edit
controls, project navigation, or small-window breakpoints; those must be
resolved in the detailed screen spec before UI implementation.

## Responsibilities and plugin boundaries

Keep existing general-purpose capabilities general. Do not move project-owned
records into a plugin just because that plugin has a similar-looking UI.

| Responsibility | Owner | Contract / notes |
|---|---|---|
| Browse the user's files and select source items | `fileatlas-plugin-file-manager` | Contribute an “add reference to WorkCrate” action through a shared event/request; FileManager must not depend on WorkCrate private types. |
| Open/reveal/copy/move/delete original files | `fileatlas-plugin-file-operations` | WorkCrate calls its public service where appropriate. Removing a project reference never calls a destructive file operation. |
| Standard image thumbnails and reusable cache | `fileatlas-plugin-thumbnails` | Reuse generated source previews when available. Do not assume every format has an installed Windows thumbnail provider. |
| Open an original file for quick viewing | `fileatlas-plugin-quicklook` and existing OS association flow | QuickLook remains user-owned and optional. Editing handoff to PhotoCraft should be a separate explicit integration, not a change to QuickLook semantics. |
| Temporary arbitrary-file transfer basket | `fileatlas-plugin-file-staging` | Remains a short-lived transfer list; it is not the durable asset manifest for a project. |
| Application startup/browser layout | `fileatlas-plugin-workspaces` | Keep its current launch-layout meaning. WorkCrate projects and their boards are independent of the active FileManager pane layout. |
| Common panel chrome, commands, lifecycle cleanup, theme, settings | `fileatlas-ui-bridge` / Cordis | Reuse `PanelCatalog`, typed settings contribution and lifecycle-owned UI entries. |
| Project CRUD, source references, per-reference preview choice | New `fileatlas-plugin-workcrate-projects` | Durable project identity and manifest. Owns relink/missing-source status and user-selected preview assets. |
| Board model, elements, template/master inheritance and editing | New `fileatlas-plugin-workcrate-boards` | Owns canvas geometry and board-local overrides. It stores references to assets, not copied originals. |
| Raster/PDF rendering and delivery assembly | New `fileatlas-plugin-workcrate-export` | Owns export profile, selected board order, progress/cancel/error reporting and temporary output cleanup. Reuses the board model through public contracts. |

The three WorkCrate feature plugins should share a small pure-data
`fileatlas-workcrate-contracts` crate for IDs and event/API payloads. Do not add
cross-plugin private imports or make the shell own business models. This split
allows project/reference work, board editing and export to evolve independently;
the first vertical slice may compile and start them together as one user-facing
WorkCrate feature.

## Persistence and file identity

- Project lists and modest board/template documents are versioned JSON with
  stable UUIDs, serde defaults, atomic same-directory replacement and backup
  recovery. Normal location: `%LOCALAPPDATA%\\FileAtlas\\workcrate\\`;
  isolated runtime profiles use their profile-specific feature directory.
- Keep large user-supplied preview images and generated exports as separate
  files. JSON records relative paths inside the WorkCrate feature directory,
  plus content hash/dimensions where useful. Do not treat the thumbnail cache as
  durable user artwork.
- References inside a declared project root may be relative to that root.
  External files use an absolute path and a stable asset ID. Re-linking changes
  the path on that existing ID, preserving all board placements. Avoid names as
  keys because they are not unique or stable.
- Record source path, current availability and last-known metadata separately.
  Missing or moved files remain visible and relinkable; they are not silently
  dropped from a project.
- If measured project/board volume or querying needs later exceed whole-file
  JSON, evaluate the existing `fileatlas-plugin-database` SQLite service then;
  do not introduce a private database layer preemptively.

## Unified board and export model

Use one explicit-coordinate board model for interactive presentation and
export. It contains physical page size/units, safe margins, template identity,
elements with stable IDs and coordinates, asset references, editable title and
description text, optional numbering and per-element appearance overrides.
Shared template fields (header, border, project label, metadata positions and
page number) are inherited; a board can override permitted fields without
destroying its template relationship. This is a presentation master, not a
Photoshop/PPT layer synchronization system.

PNG/JPEG and PDF export consume the same resolved board snapshot. Render canvas
placements from original image pixels using the same non-destructive transforms
as the editor. For screen display, choose source sampling/mip level from the
actual zoom and device-pixel scale so visible detail stays sharp; never mutate
or down-res the stored source image to create a canvas placement. Export
re-renders from source pixels at the requested output dimensions/resolution,
not by enlarging an overview thumbnail. PDF assembly preserves chosen order and page dimensions, reports a
missing asset before or during rendering, writes to a temporary file and
atomically replaces only after successful completion. Keep PNG/JPEG output and
PDF page composition as separate user choices over the same renderer.

## FigCraft, DesignCraft and PdfCraft source study

Shallow local source checkouts live under the ignored
`vendor/workcrate-references/` directory. They are reference material only and
are not Cargo dependencies. Exact main-branch snapshots cloned on 2026-10-20:

| Repository | Commit | License in upstream | Best WorkCrate reference |
|---|---|---|---|
| [FigCraft](https://github.com/storytold/figcraft) | `dd6bd3fe1c11d81b04e075c8a363f8a31fd7fb8d` | MIT OR Apache-2.0; assets retain their own notices | Persistent document model, command registry, undo boundaries, canvas tools, vector/image rendering and command-driven interaction. Its full interface uses egui and its engine is a full design editor; do not transplant it wholesale into GPUI. |
| [DesignCraft](https://github.com/storytold/designcraft) | `443ea4640ca368fcf0241b04813bd3817acd5df8` | MIT OR Apache-2.0; assets retain their own notices | Closest conceptual reference for parent/master pages, frames, inherited styles, text/image layout, page composition and a shared renderer for screen and PDF. Its full interface is egui; its publishing-grade typesetting/render stack is much broader than the initial WorkCrate scope. |
| [PdfCraft](https://github.com/storytold/pdfcraft) | `ec0a965074f376a5bf610435948e4e5184bf3299` | MIT OR Apache-2.0; vendored subcrates/assets may have separate notices | PDF page organization, ordering, thumbnails, combine/insert/extract, robust save/undo and renderer isolation. It is a PDF workbench, not the source of the visual board/editor model. |

Source-level design observations:

- FigCraft separates geometry, document, layout, text, rendering, format and
  engine from the egui front end. Its document stores nodes in an immutable
  ordered map; the engine owns the central command catalogue used across the
  UI/CLI/MCP surfaces. Study `crates/doc/src/lib.rs`,
  `crates/engine/src/cmd/`, `crates/engine/src/` history/command tests and
  `crates/render` for stable-ID edits and undo grouping.
- DesignCraft separates document structures, composition, rendering, PDF and
  export formats from its engine/UI. Its document model explicitly handles
  spreads, pages, parent pages, layers, frames, stories and styles. Study
  `crates/doc`, `crates/compose`, `crates/render`, `crates/pdf` and `crates/engine`
  for template inheritance and matching screen/PDF line/layout output.
- PdfCraft separates PDF object/model, rendering, page organization, creation,
  editing and its engine façade; its `export` crate covers export formats such
  as PPTX rather than board-to-PDF composition. Study
  `crates/organize/src/import.rs` (`combine`, selected combine, insert/extract),
  `crates/create`, `crates/render` and engine save/undo tests for safe page
  assembly. Use DesignCraft's PDF writer as the closer board-to-PDF reference.
- All three currently use egui/eframe UI layers. FileAtlas uses GPUI/GPUI Kit;
  port domain boundaries and algorithms selectively, never add egui to FileAtlas.
- Before adapting any code or crates, inspect file-level license/NOTICE and
  transitive dependency notices, pin the source commit, preserve attribution,
  and document distribution obligations in the receiving plugin's PORTS.md.
  The repository-level dual license alone does not automatically cover every
  bundled asset or third-party crate.

## Delivery stages

See the detailed, source-mapped [WorkCrate development plan](workcrate-development-plan.md).
In short: pin/validate the upstream core crates; implement durable project/file
references; extend existing thumbnails for PDFs through PdfCraft; build a
DesignCraft-aligned board/master model and reference-matched overview; expose a
single renderer for preview and full-size images; use PdfCraft's image-to-PDF
path for ordered canvas pages; then validate the full live workflow.

Each stage needs a focused PORTS.md contract, compatibility/recovery behavior
and same-process FileAtlas MCP validation before being described as complete.
