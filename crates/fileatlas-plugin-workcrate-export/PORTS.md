# WorkCrate Export plugin ports

Status: core single-board PDF service, atomic output handoff, UI export command,
and Full-access MCP export flow are implemented, including ordered multi-page
PDF and PNG/JPEG. Progress/cancel and native Windows save-dialog validation remain
open.

## Cordis identity and dependencies

- Plugin: `fileatlas.workcrate-export`.
- Service: `fileatlas/workcrate-export` (`WorkCrateExportService`).
- Requires the public `fileatlas/workcrate-boards` service. Cordis owns
  publication and plugin stop cleanup.
- PdfCraft `pdfcraft-create`, `pdfcraft-cos`, and `pdfcraft-organize` are pinned at
  `ec0a965074f376a5bf610435948e4e5184bf3299` (MIT OR Apache-2.0).

## Export behavior

- `export_page_pdf(board_id, page_index, path)` reads the board placement model,
  derives the output density from the highest source-image effective PPI (300
  DPI for an empty board; at least 72 DPI for accurate board page dimensions;
  capped at PdfCraft's 1200 DPI), requests a
  DesignCraft High-quality page render at that pixel density, then writes the
  resulting PNG into a one-page PDF with PdfCraft.
- PdfCraft's `ImageResolution::Dpi` sets page size from pixel dimensions and
  does not resample the raster. Image input is rehydrated from original/selected
  preview bytes by the Boards service, never sourced from overview thumbnails.
- `export_pages_pdf(ids,path)` renders ordered page IDs independently at source
  density, creates their PDF pages, and combines through PdfCraft
  `combine_selected`. A3 product pages remain A3; dimensions can differ only when
  explicitly edited. Pixel rounding introduces at most one output pixel of
  physical-size rounding per axis, without dropping source image data.
- `export_page_image(id,path)` writes PNG or JPEG. PNG pHYs encoding follows
  PdfCraft create's existing `set_pixel_dims` usage; JPEG uses the image crate's
  native JFIF density at quality 100. Both retain nominal page print dimensions
  separately from actual source-pixel metadata; PNG is lossless for the composed
  raster, JPEG remains a lossy format despite its highest quality setting.
- Result reports output path, dpi, and raster width/height. Errors distinguish
  board/render failures, file IO, PDF creation and PDF serialization.
- The PDF is fully serialized before a same-directory temporary file is flushed
  and atomically persisted over the requested destination. On write failure the
  prior destination remains available and the temporary file is cleaned up.
- Raster page caps or PdfCraft's 1200 DPI bound can reject or limit extreme
  print sizes; the editor must surface that result and offer an explicit lower
  density export choice rather than silently downsampling.

## Verification and current limits

The export test verifies derived DPI, full page pixel dimensions and a valid PDF
header. Same-process MCP export produced a one-page A4 PDF from a board with PNG
and 1024×768 JPEG sources; the MCP result reported 175.54 DPI and 1451×2053 px.
Poppler reopened the PDF and rendered the page at 110 DPI for visual inspection.
Native PDF save-dialog interaction and camera-level resolution remain
unverified. Inspect the returned error and target file when diagnosing export
failures.
