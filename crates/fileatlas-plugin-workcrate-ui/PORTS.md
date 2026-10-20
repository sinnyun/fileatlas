# WorkCrate UI plugin ports

Status: three-level project management, page management and page editor are implemented. It reuses the
projects, boards, export, settings and UI bridge services; persistence remains
owned by the feature services.

## Cordis identity and startup

- Plugin: `fileatlas.workcrate-ui`.
- Contributions: panel `workcrate/overview`, command `workcrate/open`, and
  sidebar launcher view `workcrate/sidebar`.
- Requires `WorkCrateProjectsService`, `WorkCrateBoardsService`,
  `WorkCrateExportService`, `ThumbnailService`, `SettingsService`, and `UiBridgeService`.
- Shell starts the three data services before this UI plugin. Cordis owns panel,
  command, view and event cleanup on stop.
- Optional shared MCP registry tools are `workcrate-ui.state` (read-only,
  project/board metadata only), `workcrate-ui.open_board` (Standard-gated UI
  selection/open), and `workcrate-ui.place_image` (Standard-gated project
  reference plus board placement). The placement tool validates the image via
  the boards service and never writes to the source file. `workcrate-ui.export_pdf`
  requires Full MCP access plus `confirm=true` because it writes the requested
  PDF path. Tool registrations are lifecycle-owned and removed when this plugin
  stops.
- Additional Standard tools: `workcrate-ui.navigate({project_id?,board_id?})`,
  `manage_project({action,project_id?,name?,path?})` for create/rename/cover/
  duplicate/archive/restore; `create_page({project_id})`;
  `update_page({project_id,board_id,title?,info?})` merges editable content fields.
- Full + per-call confirmation tools: `delete_project({project_id})` retains
  project-owned data in trash; `export_pages({project_id,board_ids,path,format})`
  accepts ordered PDF page IDs or one image ID and PNG/JPEG destination.
- `state` includes the painted UI screen, busy status, page count, selections,
  editing field and feedback from the shared `workcrate.ui` surface. Publication
  uses ChangeGate; view drop clears that surface.

## User flows

- Project management shows only title and main image. Search, creation/name
  sorting, creation, rename, cover selection, duplicate, archive/restore and
  confirmed deletion are available. Cover selection uses the existing Thumbnail
  service for validation, retaining the chosen full source bytes separately.
- Page management shows a three-column page grid, independent selection,
  PNG/JPEG selected-page export and ordered all-page PDF export. New pages are
  A3 landscape (420 × 297 mm); existing page dimensions remain compatible.
- Editor has left page thumbnails, center canvas and right information/content/
  image inspector. First image supplies name/pixels, without changing A3.
  Title, physical dimensions and content fields are editable. Project name
  inherits the global project name or stores a local override; reset inherits.
- Canvas hit selection and release-time drag persist DesignCraft transforms;
  selected-image controls scale/rotate via the same engine. Image refresh,
  replacement and arbitrary linked source-document selection are available.
  Image rows retain actual source dimensions; source pixel counts are read-only.
- Native dialogs choose sources and outputs; cancellation leaves data unchanged.
  Image-folder export asks before replacing existing output names.
- Image bytes flow from the selected source path through DesignCraft. Catalog
  previews are generated independently and are never used for canvas or export.
- The GPUI adapter converts RGBA to the native RenderImage BGRA contract exactly
  as GPUI 0.3.8's built-in image loader does. This applies to project covers,
  page previews, source thumbnails and the canvas; export remains canonical RGBA.
- MCP state uses board manifests and does not read or hash linked image files;
  explicit open/placement calls validate and hydrate the selected source.

## Current limits and failure inspection

- Crop handles, interactive resize/rotation handles, master/template editing,
  custom reference-preview selection and linked-source external editing remain
  open. Resize/rotation currently use explicit selected-image controls.
- File picker cancellation is a no-op. Service/render/export errors are shown
  in the panel message area; inspect the relevant projects/boards JSON and
  their stored source paths/hashes, then check FileAtlas logs for renderer or
  PDF writer errors.
- Windows native picker behavior, zoom sharpness with a real high-resolution
  source, PDF save-dialog interaction, and full lifecycle restart still require
  additional same-process FileAtlas MCP coverage.

## Visible image input and concept correction (2026-10-20)

The original picker compared a canonical stored path with an uncanonicalized
picker path; this fails for ordinary Windows paths. `view/input.rs` now owns
canonicalization and multi-image ingestion for both the visible input and drops.
Add Image opens a GPUI path entry (quoted paths, semicolon-separated batch),
Browse opens an owner-parented native JPEG/PNG picker, Add commits through the
same importer. Unsupported extensions and failed reads report errors.
The canvas accepts public `FileManagerFileDragPayload`, `FileStagingDragPayload`
and platform `ExternalPaths`. No FileManager private API or new data subsystem.
The default panel is 1560 × 980 with a 180-wide page rail, 310-wide inspector,
workspace-wide tool strip, responsive three-column project/page cards, same-row
page metadata and subtle inspector image separators. Fit uses the pinned
FigCraft algorithm with a 24-point effective margin.
Actual visible input and normal-mode FileManager drag evidence is recorded in
`docs/validation-ui/2026-10-20-workcrate-input-layout/README.md`. Native Browse
dialog selection and external Explorer drops are separate unverified paths.

The command strip includes working Select and Pan modes. Pan changes only the
visible artboard offset; Fit resets it, without changing DesignCraft document
coordinates or export output. The MCP UI snapshot exposes `canvas_mode` and
`pan_offset` for pointer-driven assertions.

## Superseding fixed-A3 image-layout model (user clarification, 2026-10-20)

The editor is a fixed A3 page with an information band and a clipped image area.
Earlier pan/zoom/document-size editing descriptions are historical. The size
field is a free-form `PageInfo.size_label` annotation; changing it cannot resize
the document. Select by clicking an image. Four visible corner handles resize
uniformly; Crop changes the frame only, Restore returns the original content.
`view/image_edit.rs` keeps a transient GPU-layer gesture, writes once on release,
and refreshes only the changed page preview. `render_template` supplies static
DesignCraft paper/header; source-pixel textures are decoded on data reload, not
on pointer movement. The picture area alone receives file drops.
`linked_images.rs` owns project-wide refresh and source-file open actions.
New dependency: public FileOperationsService, using `open_linked_file(&Path)`
for arbitrary PSD/etc. links without indexing them. No private imports.
MCP state adds painted `controls` bounds and live `ui.layouts`/`ui.gesture`;
`image_links` sets extended links or refreshes a project through the same service.
The prior `canvas_mode`/`pan_offset` snapshot fields were retired with whole-page pan.
