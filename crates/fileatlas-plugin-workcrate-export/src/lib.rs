//! Export through the official VectorCraft application.
use cordis::{Context, Plugin, Service};
use fileatlas_plugin_workcrate_boards::{BoardsError, WorkCrateBoardsService};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};
use thiserror::Error;
pub struct WorkCrateExportService {
    boards: Arc<WorkCrateBoardsService>,
    progress: RwLock<Option<String>>,
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
        Self {
            boards,
            progress: Default::default(),
        }
    }
    pub fn progress(&self) -> Option<String> {
        self.progress.read().ok().and_then(|v| v.clone())
    }
    fn report(&self, message: String) {
        if let Ok(mut p) = self.progress.write() {
            *p = Some(message)
        }
    }
    pub fn export_pages_pdf(&self, ids: &[String], path: &Path) -> Result<usize, ExportError> {
        if ids.is_empty() {
            return Err(ExportError::EmptySelection);
        }
        self.report("正在导出官方 PDF".into());
        match self.boards.export_native(ids, path, "pdf") {
            Ok(result) => {
                self.report(format!("完成 {} 页 · {}", ids.len(), result));
                Ok(ids.len())
            }
            Err(e) => {
                self.report(format!("导出失败：{e}"));
                Err(e.into())
            }
        }
    }
    pub fn export_pages_images(
        &self,
        ids: &[String],
        paths: &[PathBuf],
    ) -> Result<usize, ExportError> {
        if ids.is_empty() || ids.len() != paths.len() {
            return Err(ExportError::EmptySelection);
        }
        for (id, path) in ids.iter().zip(paths) {
            self.export_page_image(id, path)?;
        }
        Ok(ids.len())
    }
    pub fn export_page_image(&self, id: &str, path: &Path) -> Result<ExportedPage, ExportError> {
        let format = match path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "png" => "png",
            "jpg" | "jpeg" => "jpg",
            _ => return Err(ExportError::UnsupportedFormat),
        };
        self.report("正在导出官方图像".into());
        let b = self.boards.board(id)?;
        let result = self.boards.export_native(&[id.into()], path, format)?;
        self.report(format!("完成 · {result}"));
        Ok(ExportedPage {
            path: path.into(),
            dpi: 300.,
            width_px: (b.width_pt * 300. / 72.).round() as u32,
            height_px: (b.height_pt * 300. / 72.).round() as u32,
        })
    }
    pub fn export_page_pdf(
        &self,
        id: &str,
        page_index: usize,
        path: &Path,
    ) -> Result<ExportedPage, ExportError> {
        if page_index != 0 {
            return Err(ExportError::InvalidPage);
        }
        let b = self.boards.board(id)?;
        self.export_pages_pdf(&[id.into()], path)?;
        Ok(ExportedPage {
            path: path.into(),
            dpi: 72.,
            width_px: b.width_pt.round() as u32,
            height_px: b.height_pt.round() as u32,
        })
    }
}
#[derive(Debug, Error)]
pub enum ExportError {
    #[error(transparent)]
    Boards(#[from] BoardsError),
    #[error("select pages and matching output paths")]
    EmptySelection,
    #[error("export supports PNG/JPEG images")]
    UnsupportedFormat,
    #[error("each native artboard is one page")]
    InvalidPage,
    #[error("plugin: {0}")]
    Plugin(String),
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
    fn prepare(&self, c: ()) -> Result<(), Self::PrepareError> {
        Ok(c)
    }
    async fn apply(&self, context: Context, _: &()) -> Result<(), ExportError> {
        let boards = context
            .try_service::<WorkCrateBoardsService>()
            .map_err(|e| ExportError::Plugin(e.to_string()))?;
        let _publication = context
            .provide(Arc::new(WorkCrateExportService::new(boards)))
            .map_err(|e| ExportError::Plugin(e.to_string()))?;
        Ok(())
    }
}
