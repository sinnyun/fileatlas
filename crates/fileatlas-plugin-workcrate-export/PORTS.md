# WorkCrate Export ports

Cordis plugin `fileatlas.workcrate-export` publishes `fileatlas/workcrate-export`, requires Boards service. Public constructor `new(Arc<WorkCrateBoardsService>)`, `progress() -> Option<String>`, export_pages_pdf, export_pages_images, export_page_image, export_page_pdf retain shell/service integration.

All file generation uses official VectorCraft app.export through Boards. PDF selection uses current native indices; all selected pages must belong to one associated native document. Images support PNG/JPG at300dpi. ExportedPage retains existing path,dpi,width_px,height_px fields. Native output is generated into a fresh temporary file, native background status/file completion verified, then atomically handed to requested path. Count results indicate completed output; native warnings are retained in progress on success. Failure progress persists for PDF; image failures return ExportError.

No PdfCraft, image encoder, own renderer or raster PDF generation. No process ownership/cleanup beyond underlying client lifetime. Native errors and progress provide diagnostics; no event producers/consumers. Integration runtime remains separately verified with the ignored official_native_round_trip test.
