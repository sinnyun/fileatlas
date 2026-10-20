//! Source-resolution image and PDF export for WorkCrate boards.
use cordis::{Context, Plugin, Service};
use fileatlas_plugin_workcrate_boards::{BoardsError, WorkCrateBoardsService};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;

pub struct WorkCrateExportService {
    boards: Arc<WorkCrateBoardsService>,
}

impl Service for WorkCrateExportService {
    const NAME: &'static str = "fileatlas/workcrate-export";
}

#[derive(Clone, Debug)]
pub struct ExportedPage {
    pub path: PathBuf,
    pub dpi: f64,
    pub width_px: u32,
    pub height_px: u32,
}

impl WorkCrateExportService {
    pub fn new(boards: Arc<WorkCrateBoardsService>) -> Self {
        Self { boards }
    }

    pub fn export_pages_pdf(&self, ids: &[String], path: &Path) -> Result<usize, ExportError> {
        if ids.is_empty() {
            return Err(ExportError::EmptySelection);
        }
        let mut documents = Vec::new();
        for id in ids {
            let board = self.boards.board(id)?;
            let dpi = source_dpi(&board);
            let page = self.boards.render_page(id, 0, dpi / 72.0)?;
            let document = pdfcraft_create::from_images_with_resolution(
                &[(format!("{}.png", board.title), page.to_png())],
                pdfcraft_create::ImageResolution::Dpi(dpi),
            )?;
            documents.push((board.title, document));
        }
        let sources = documents
            .iter()
            .map(|(name, document)| (name.as_str(), document, None))
            .collect::<Vec<_>>();
        let document = pdfcraft_organize::combine_selected(&sources)?;
        atomic_output(
            path,
            &pdfcraft_cos::write_full(&document, &pdfcraft_cos::SaveOptions::default())?,
        )?;
        Ok(ids.len())
    }

    pub fn export_page_image(&self, id: &str, path: &Path) -> Result<ExportedPage, ExportError> {
        let board = self.boards.board(id)?;
        let dpi = source_dpi(&board);
        let page = self.boards.render_page(id, 0, dpi / 72.0)?;
        let png = page.to_png();
        let bytes = if path
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|x| x.eq_ignore_ascii_case("jpg") || x.eq_ignore_ascii_case("jpeg"))
        {
            let mut output = Vec::new();
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, 100);
            encoder.set_pixel_density(image::codecs::jpeg::PixelDensity {
                density: (
                    (page.width as f64 * 72.0 / board.source.settings.page_width).round() as u16,
                    (page.height as f64 * 72.0 / board.source.settings.page_height).round() as u16,
                ),
                unit: image::codecs::jpeg::PixelDensityUnit::Inches,
            });
            encoder.encode_image(&image::load_from_memory(&png)?.to_rgb8())?;
            output
        } else {
            // Match PdfCraft crates/create/src/tests.rs PNG physical-density encoding;
            // retain the DesignCraft raster unchanged and store the real page print size.
            let pixels = image::load_from_memory(&png)?.into_rgba8();
            let mut output = Vec::new();
            let mut encoder = png::Encoder::new(&mut output, page.width, page.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_pixel_dims(Some(png::PixelDimensions {
                xppu: (page.width as f64 / (board.source.settings.page_width / 72.0 * 0.0254))
                    .round() as u32,
                yppu: (page.height as f64 / (board.source.settings.page_height / 72.0 * 0.0254))
                    .round() as u32,
                unit: png::Unit::Meter,
            }));
            encoder.write_header()?.write_image_data(pixels.as_raw())?;
            output
        };
        atomic_output(path, &bytes)?;
        Ok(ExportedPage {
            path: path.to_path_buf(),
            dpi,
            width_px: page.width,
            height_px: page.height,
        })
    }

    /// Render a board from DesignCraft's hydrated source image bytes, then package that unchanged
    /// raster into PdfCraft. DPI is the highest effective placed-source PPI (300 for an empty
    /// page), with at least 72 DPI so the board raster preserves page geometry, bounded by
    /// PdfCraft's 1200 DPI limit. The PNG is never resampled by PdfCraft.
    pub fn export_page_pdf(
        &self,
        board_id: &str,
        page_index: usize,
        path: &Path,
    ) -> Result<ExportedPage, ExportError> {
        let board = self.boards.board(board_id)?;
        let dpi = board
            .placements
            .values()
            .map(|placed| placed.pixel_width as f64 * 72.0 / placed.placed_width_pt)
            .reduce(f64::max)
            .unwrap_or(300.0)
            .max(72.0)
            .min(1200.0);
        let rendered = self.boards.render_page(board_id, page_index, dpi / 72.0)?;
        let png = rendered.to_png();
        let document = pdfcraft_create::from_images_with_resolution(
            &[(format!("{}.png", board.title), png)],
            pdfcraft_create::ImageResolution::Dpi(dpi),
        )?;
        let bytes = pdfcraft_cos::write_full(&document, &pdfcraft_cos::SaveOptions::default())?;
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        temporary.write_all(&bytes)?;
        temporary.as_file().sync_all()?;
        temporary.persist(path).map_err(|error| error.error)?;
        Ok(ExportedPage {
            path: path.to_path_buf(),
            dpi,
            width_px: rendered.width,
            height_px: rendered.height,
        })
    }
}

#[derive(Debug, Error)]
pub enum ExportError {
    #[error(transparent)]
    Png(#[from] png::EncodingError),
    #[error("select at least one page")]
    EmptySelection,
    #[error(transparent)]
    Image(#[from] image::ImageError),
    #[error(transparent)]
    Organize(#[from] pdfcraft_organize::OrganizeError),
    #[error(transparent)]
    Boards(#[from] BoardsError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Create(#[from] pdfcraft_create::CreateError),
    #[error(transparent)]
    Write(#[from] pdfcraft_cos::CosError),
    #[error("Cordis dependency or service publication failed: {0}")]
    Plugin(String),
}

fn source_dpi(board: &fileatlas_plugin_workcrate_boards::WorkCrateBoard) -> f64 {
    board
        .placements
        .values()
        .map(|p| p.pixel_width as f64 * 72.0 / p.placed_width_pt)
        .reduce(f64::max)
        .unwrap_or(72.0)
        .clamp(72.0, 1200.0)
}

fn atomic_output(path: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

pub struct WorkCrateExportPlugin;
impl Plugin for WorkCrateExportPlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = ExportError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "fileatlas.workcrate-export".into()
    }
    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }
    async fn apply(&self, context: Context, _input: &Self::Input) -> Result<(), Self::ApplyError> {
        let boards = context
            .try_service::<WorkCrateBoardsService>()
            .map_err(|error| ExportError::Plugin(error.to_string()))?;
        let _publication = context
            .provide(Arc::new(WorkCrateExportService::new(boards)))
            .map_err(|error| ExportError::Plugin(error.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fileatlas_plugin_workcrate_boards::WorkCrateBoardsService;
    use fileatlas_plugin_workcrate_projects::WorkCrateProjectsService;
    use image::{ImageBuffer, Rgb};
    use std::fs;

    #[test]
    fn image_export_embeds_a3_print_dimensions() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let projects = Arc::new(WorkCrateProjectsService::open_at(
            temp.path().join("store"),
        )?);
        let project = projects.create_project("A3 output")?;
        let boards = Arc::new(WorkCrateBoardsService::new(projects));
        let page = boards.create_page(&project.id)?;
        let export = WorkCrateExportService::new(boards);
        let path = temp.path().join("page.png");
        let result = export.export_page_image(&page.id, &path)?;
        let decoded =
            png::Decoder::new(std::io::BufReader::new(fs::File::open(path)?)).read_info()?;
        let dimensions = decoded
            .info()
            .pixel_dims
            .ok_or("missing PNG physical dimensions")?;
        assert!((result.width_px as f64 / dimensions.xppu as f64 * 1000. - 420.).abs() < 0.1);
        assert!((result.height_px as f64 / dimensions.yppu as f64 * 1000. - 297.).abs() < 0.1);
        let path = temp.path().join("page.jpg");
        export.export_page_image(&page.id, &path)?;
        let image = image::open(path)?;
        assert_eq!(
            (image.width(), image.height()),
            (result.width_px, result.height_px)
        );
        Ok(())
    }

    #[test]
    fn pdf_export_preserves_page_pixels_and_uses_source_effective_ppi() {
        let temp = tempfile::tempdir().unwrap();
        let projects =
            Arc::new(WorkCrateProjectsService::open_at(temp.path().join("store")).unwrap());
        let project = projects.create_project("Print proof").unwrap();
        let source = temp.path().join("source.png");
        ImageBuffer::from_fn(32, 24, |x, y| Rgb([(x * 7) as u8, (y * 9) as u8, 140]))
            .save(&source)
            .unwrap();
        let reference = projects
            .add_references(&project.id, std::slice::from_ref(&source))
            .unwrap()
            .remove(0);
        let boards = Arc::new(WorkCrateBoardsService::new(projects));
        let board = boards
            .create_board(&project.id, "Artwork", 360.0, 240.0)
            .unwrap();
        boards
            .place_reference(&board.id, &reference.id, 0.0, 0.0, 144.0, false)
            .unwrap();
        let export = WorkCrateExportService::new(boards);
        let output = temp.path().join("artwork.pdf");
        let result = export.export_page_pdf(&board.id, 0, &output).unwrap();
        assert_eq!(result.dpi, 72.0);
        assert_eq!((result.width_px, result.height_px), (360, 240));
        assert_eq!(&fs::read(output).unwrap()[..5], b"%PDF-");
    }

    #[test]
    fn pdf_export_replaces_existing_output_only_after_serialization()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let projects = Arc::new(WorkCrateProjectsService::open_at(
            temp.path().join("store"),
        )?);
        let project = projects.create_project("Replace proof")?;
        let boards = Arc::new(WorkCrateBoardsService::new(projects));
        let board = boards.create_board(&project.id, "Artwork", 120.0, 80.0)?;
        let output = temp.path().join("artwork.pdf");
        fs::write(&output, b"previous output")?;

        WorkCrateExportService::new(boards).export_page_pdf(&board.id, 0, &output)?;

        assert_eq!(&fs::read(output)?[..5], b"%PDF-");
        assert_eq!(
            fs::read_dir(temp.path())?
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().contains("tmp"))
                .count(),
            0
        );
        Ok(())
    }
}
