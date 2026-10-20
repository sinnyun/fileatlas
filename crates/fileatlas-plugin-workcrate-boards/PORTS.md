# WorkCrate Boards plugin ports

Status: DesignCraft-backed board data, image-only placement, source-pixel rendering,
and PDF output through the separate export service are implemented. The first
WorkCrate UI supplies three-level navigation, image placement, DesignCraft drag,
scale/rotation, metadata editing and export. Template/master and crop/handle
editing remain open.

## Cordis identity and startup

- Plugin: `fileatlas.workcrate-boards`.
- Service: `fileatlas/workcrate-boards` (`WorkCrateBoardsService`).
- Requires `fileatlas/workcrate-projects`; it resolves project references and
  uses the project-owned `boards/` directory.
- Shell may start it with the projects service in normal, MCP, or explicit
  `workcrate-boards` development profiles. Cordis owns service publication and
  stop cleanup.

## Upstream implementation pins

- DesignCraft `designcraft-engine`, `designcraft-doc`, `designcraft-compose`,
  `designcraft-geom`, and `designcraft-render` at
  `443ea4640ca368fcf0241b04813bd3817acd5df8` (MIT OR Apache-2.0). Board
  creation uses `Document::new`; image placement executes upstream `file.place`;
  page raster uses `Renderer::render_page` with `DisplayQuality::High` defaults.
- The root Cargo manifest patches Krilla to the matching Craft-Libs revision
  expected by DesignCraft's PDF integration.

## Persistence and image rules

- Boards are version 1 JSON under
  `projects/<project-id>/boards/<board-id>.json`, with same-directory temporary
  writes and a previous JSON backup. Large image binaries are never embedded.
- DesignCraft serializes its `Asset.data` as skipped. Each placement stores the
  stable project reference ID, selected original/preview path, SHA-256, source
  pixel dimensions and placed width. Reopening verifies the hash and rehydrates
  image bytes before the renderer is called.
- The board service accepts JPEG/JPG and PNG only, checks the extension and then
  verifies actual image decoding through `designcraft_render::image_size`.
  Other project references require an attached JPEG/PNG preview.
- `render_page` takes pixels-per-point, so callers request density from the
  visible zoom and device scale. The UI fit calculation mirrors FigCraft
  `engine::ViewState::fit` in `vendor/workcrate-references/FigCraft/crates/engine/src/lib.rs`:
  64pt margins, a 50pt minimum usable viewport axis, and fit capped at 100%.
  It uses DesignCraft's high quality image path and source mip selection;
  overview thumbnails are never passed as board data.
- Additive v1 `info` uses serde defaults for older manifests. It stores page
  order, first-source pixels, physical cm size, optional local project-name
  override, material/size/quantity and extra content fields. Placement records
  add an optional DesignCraft item ID and optional arbitrary source-document link.
  Legacy item IDs are inferred from DesignCraft graphic references on load and
  persisted on the next edit; no stored documents are discarded.

## Public operations and failure inspection

- `create_board(project_id, title, width_pt, height_pt)` validates page bounds,
  builds a DesignCraft print document, and persists it.
- `create_page(project_id)` is the product entry point: A3 landscape, ordered,
  first-image name/pixel defaults. First image is fitted inside the page below
  the information band; it does not change physical dimensions.
- `update_info(id,title,PageInfo)` saves content; physical edits use DesignCraft
  `layout.documentSetup` with Adjust Layout. Actual source pixels cannot be
  overwritten through this method.
- `transform_image(id,asset,dx,dy,scale,angle)` executes DesignCraft
  selection/move/scale/rotate. `refresh_image(id,asset,replacement?)` replaces or
  revalidates raster bytes through DesignCraft `links.relink`;
  `link_source(id,asset,path?)` saves a document link.
- `duplicate_project_pages(original,target)` copies page documents with new IDs.
- `board_catalog` reads manifests without linked binary IO. UI hydrates pages
  separately so one missing source does not hide all page metadata.
- `place_reference(board_id, reference_id, x_pt, y_pt, width_pt, use_preview)`
  validates and places source or chosen preview image, then persists the updated
  document and link metadata.
- `load_board(project_id, board_id)` reads the JSON and rehydrates verified
  source bytes. A changed file hash returns `SourceChanged` and requires relink
  or replacement before a trusted render.
- `render_page(board_id, page_index, pixels_per_point)` is the rendering entry
  point for a viewport or export image.
- Its presentation snapshot adds the real page-content information band with
  DesignCraft `frame.create`, `text.select`, and `type.char`. Global/local name
  and page content therefore match previews and exports. Outside page-info UI
  is excluded. Unhydrated source assets cause an error, not an incomplete export.
- Failures are returned as `BoardsError`. Inspect the project board JSON and
  `placements` paths/hashes first; then check DesignCraft decode/render errors
  or PdfCraft create/write errors.

## Coverage

The crate tests cover JPEG/PNG eligibility, non-image reference rejection,
board JSON's omission of raw image data, reopen/re-hydration and viewport-scale
page rendering. Same-process WorkCrate MCP placed a PNG and a 1024×768 JPEG,
rendered both at the measured panel/device density, and reopened the board.
Native image picker interaction, relink states and camera-resolution detail
remain unverified. PDF export lives in the separate WorkCrate Export plugin.

The information header is generated through upstream `swatch.create`,
`frame.create`, `object.fill`, `text.select`, and `type.char`. Its pale neutral
background is printable document content and stays the same in canvas and
exports; it is not a plugin-local application theme.

Rehydration keeps an existing fully loaded board available until all new source
bytes are read, then publishes the hydrated document. Registering an unloaded
manifest must not transiently overwrite a live loaded board and produce a
spurious invalid-image error during zoom/fit rendering. The manifest is still
registered for editing when no cached model exists and a source cannot hydrate.

## Fixed A3, uniform frames and linked refresh (supersedes size-edit semantics)

`PageInfo.size_label: String` is annotation only. `update_info` never calls
documentSetup. Product pages (order>0) load as A3, retaining image coordinates
and legacy size annotations; the next ordinary write preserves the .bak path.
`WorkCrateBoard::image_layout(asset)` returns frame/content rectangles.
`set_image_frame(id,asset,[x0,y0,x1,y1],crop)` delegates to transform.resize:
non-crop rejects nonuniform ratios; crop changes the frame without source-byte
mutation. `restore_image_crop` uses object.fit/fitFrameToContent.
`refresh_project_images(project)` returns `{updated,unchanged,failures}` per
placement across all pages; failures retain valid records and original files.
Refreshing a source with a changed aspect ratio uses proportional fitting.
`render_template(id,scale)` renders paper and information with placed-image items
removed for the cached editor background. Full render_page remains the export
and page-preview source, including frame clipping.
