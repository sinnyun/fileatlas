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

## Lazy pages, progressive display and export footer (2026-10-20)

Page catalog loading only decodes cached page thumbnails. Missing previews fill
one page at a time in a bounded background worker; the list becomes interactive
before cache rebuild finishes. Generation/revision checks discard stale UI results.
The selected editor page loads a small proxy first, then the tier required by its
content bounds × fit scale × device scale. Increasing visible size requests a
larger tier; source dimensions are the ceiling. Switching pages releases other
pages' GPUI image textures. Header rendering converts straight RGBA directly,
without a PNG round trip. Native image input/drop still share import_images.

Export uses a finite shared refresh::poll (100ms), updates only on stage changes,
and stops on success/error. Footer shows actual page/total, stage and completion.
Export completion does not reload the project. UI state adds preview_cache_hits,
previews_ready, display_images, display_rgba_bytes, export_status; top-level MCP
state adds export_progress. `export_pages` retains synchronous default behavior;
optional `background:true` queues the same UI export job and returns queued=true,
allowing MCP to exercise/capture the running footer. Background image export accepts
one filename or a directory for an ordered multi-page PNG batch. Full access and
confirm=true are still required. Queued UI export reports a busy-operation conflict
in the visible message; it never reports success for an unstarted job.

Viewport proxy reads use the shared `ReadModel<(generation,revision+tiers,coarse), DisplayImages>`
with lifecycle-owned Wake/listen. It coalesces rapid resize requests, caches
failures and discards stale results; fine resolution starts only after the small
preview completes. An existing larger texture is never replaced by a coarse one.

## Editing additions and inline correction (2026-10-21)

`view/text_editor`, `text_canvas` and `live_text` own native multiline text editing.
Add Text focuses the canvas immediately; double-click resumes an existing frame.
One TextareaState is rendered either on the canvas or in the inspector. Microsoft
YaHei, content color and point size are input data; selection/caret/control chrome
remain native theme roles. Four corner handles change the frame and reflow text;
the small move strip separates dragging from character selection. No glyph transform
is applied. Right-side text, size/color and dimensions subscribe to native Change.

A per-(board,text) pending map coalesces drafts. A finite shared refresh::poll
starts at 100ms, serializes background update_text/render_text work, and stops
when drained. Existing input entities/carets remain alive. Only the edited text
texture/card document is replaced. Save Text uses this same path, never perform
or reload. The existing A3 background and photo texture IDs remain unchanged.
Previews update after a 600ms pause. Stale full-canvas text layers are rejected;
photo ReadModel keys now depend on placements and resolution tiers, not text edits.
Deletion is disabled while text writes are pending. Invalid temporary numeric input
is not committed; explicit Save reports invalid font size/frame dimensions.

Page content retains manual inputs plus native preset menus. Settings contribution
uses the existing SettingsPage contract and Projects FieldPresets service. Page
management has multi-select, select-all, clear and exit (exit clears selection).
MCP adds `text` (add/update/delete/read), `replace_image`, `field_presets`; see the
Rust schemas in mcp/editing.rs for payloads. `state.control_ids` requests measured
controls, including canvas-text-input/select-text-<id>. UI state exposes text_inline,
text_write_active, load_generation, canvas_image_id and display_image_ids for real
interaction/refresh diagnostics. These are observations, not replacement UI handlers.

Live coverage: scripts/validate-workcrate-inline-text.cjs, isolated same-process
profile `.local/validation/profiles/workcrate-inline-final-2026-10-21`.
Chinese text injection is covered; physical IME candidate/composition behavior and
extreme font/box dimensions require separate interactive coverage.

## Selection, native typography browser and preset auto-save (2026-10-21)

Inline mode removes the lower draggable text sprite's mouse target; the native
Textarea handles press/drag selection and its wrapper stops bubbling into image
or object movement. Only the move strip/handles start frame gestures. A gated
observer publishes actual selected UTF-8 byte ranges without notifying on caret
blinks. It permits drag selection/replacement while preserving the frame.

text_options/text_options_view/text_option_labels extend the existing inspector:
compact native bold/italic/underline/strike and alignment actions, plus More Text
Settings with chars/para/frame/OpenType groups, searchable full attribute catalogue,
current values, primitive/enum inputs and native structured JSON values. Labels
are presentation only; keys/types/defaults come from the actual pinned dependency.
Applying formats uses one bounded background action, updates only that text sprite,
and retains background/photo textures. Nonempty selections can receive character
formats independently; frame options always affect the whole frame. The original
size/color quick fields retain whole-frame semantics; advanced size/fill can target
a selection. Default font stays Microsoft YaHei; the advanced fontFamily/fontStyle
attributes allow other fonts. Common font/decorations/leading/alignment also project
into native input appearance. Complex typography, multicolumn/CJK/justified layout
uses DesignCraft when finishing/applying; ordinary input is not a replacement rich
publishing renderer. Reads/changes to advanced current values refresh on opening
More Settings or applying a setting. No egui frontend or parallel text engine added.

Preset Settings observes actual value changes and structural edits. Valid drafts
save automatically via the Projects service; blank unfinished value rows are
excluded and no page values are overwritten. A finite shared refresh::poll drains
coalesced changes and stops after success/error. Manual Save remains available.
The editor's render also reads the existing in-memory preset snapshot, ensuring
menus are current after returning from Settings. No IO is introduced in render.
MCP adds text_options read/update, with project ownership checks and native schemas;
UI state adds text_options/text_format_busy and actual text_draft.selection.

Reproducible flows: validate-workcrate-typography.cjs and
validate-workcrate-field-presets-auto.cjs. Current snapshot has 254 native attribute
entries plus font-provided OpenType features/sets; representative real controls are
verified, not every possible attribute/font/enum combination.

## Inspector refresh and preset menu correction (2026-10-21)

Page-local writes through the shared `perform` path now fetch only the changed board,
replace that board model in the existing catalog, re-render its A3 information/header,
and refresh its disposable page preview. Metadata/text-only writes retain linked-image
texture identities and all other page-card models while replacing the A3 header layer.
If an image source changed, its decoded cache entry is discarded and the selected
page's visible image tiers are requested again. Text deletion also clears
the deleted selection/editor. Page/project navigation, project creation, and page
creation still reload the catalog because they change selection or its membership.
Project-manager rename/archive/delete/duplicate/cover actions update the visible list
directly; only new or changed covers are decoded, deleted cover entries are discarded,
and unrelated cards keep their cached cover textures.
Project-wide linked-image refresh retains its explicit full-catalog refresh because
the command changes sources across pages.

The UI selection wake now compares both project and board IDs before reloading; repeated
navigation to the same page is a no-op. MCP mutations publish an explicit board-refresh
revision, so an external write to the already selected page fetches and updates that
board in place. Project-wide MCP image refresh uses an explicit catalog-refresh revision.
Preset popup items read `FieldPresets` from the existing Projects service when opened,
so a menu closure created before Settings changes does not retain an old values vector.

Same-process coverage: `validate-workcrate-field-presets-auto.cjs` changes Settings while
the editor remains mounted, opens the new field's popup, applies an item, and asserts the
page load generation and linked-image texture IDs stay unchanged while the A3 header
texture updates to show the selected field value. It then exercises same-board MCP
`update_page` and verifies the current page changes without a catalog reload.
`validate-workcrate-inline-text.cjs` rechecks local text save and retained image textures.
Detailed refresh-site audit and limits: `docs/validation-ui/2026-10-21-workcrate-inspector-refresh/README.md`.
