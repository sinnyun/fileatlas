//! WorkCrate boards backed by the pinned DesignCraft document and rendering engine.
use cordis::{Context, Plugin, Service};
use designcraft_compose::Cache;
use designcraft_doc::build::NewDocument;
use designcraft_doc::{Document, Intent};
use designcraft_engine::{DocState, Session};
use designcraft_geom::Unit;
use fileatlas_plugin_workcrate_projects::{WorkCrateProjectsError, WorkCrateProjectsService};
use fileatlas_workcrate_contracts::{WorkCrateFileReference, canvas_image_format};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};
use thiserror::Error;
use uuid::Uuid;

const BOARD_VERSION: u32 = 1;
mod presentation;
pub use presentation::PageInfo;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkCrateBoard {
    pub version: u32,
    pub id: String,
    pub project_id: String,
    pub title: String,
    #[serde(default)]
    pub info: PageInfo,
    pub source: Document,
    /// Maps DesignCraft asset IDs to stable project reference IDs and source/preview choice.
    pub placements: BTreeMap<String, BoardPlacement>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoardPlacement {
    #[serde(default)]
    pub item_id: Option<u64>,
    #[serde(default)]
    pub source_document: Option<String>,
    pub reference_id: String,
    pub uses_preview: bool,
    pub path: String,
    pub sha256: String,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub placed_width_pt: f64,
}

pub struct WorkCrateBoardsService {
    projects: Arc<WorkCrateProjectsService>,
    boards: RwLock<BTreeMap<String, WorkCrateBoard>>,
}

impl Service for WorkCrateBoardsService {
    const NAME: &'static str = "fileatlas/workcrate-boards";
}

impl WorkCrateBoardsService {
    pub fn new(projects: Arc<WorkCrateProjectsService>) -> Self {
        Self {
            projects,
            boards: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn create_board(
        &self,
        project_id: &str,
        title: &str,
        width_pt: f64,
        height_pt: f64,
    ) -> Result<WorkCrateBoard, BoardsError> {
        validate_dimensions(width_pt, height_pt)?;
        let project_dir = self.projects.project_directory(project_id)?;
        let mut nd = NewDocument::default();
        nd.title = title.trim().to_owned();
        nd.intent = Intent::Print;
        nd.width = width_pt;
        nd.height = height_pt;
        nd.units = Unit::Points;
        nd.facing_pages = false;
        nd.pages = 1;
        let board = WorkCrateBoard {
            version: BOARD_VERSION,
            id: Uuid::new_v4().to_string(),
            project_id: project_id.to_owned(),
            title: nd.title.clone(),
            info: PageInfo::default(),
            source: Document::new(&nd),
            placements: BTreeMap::new(),
        };
        write_board(&project_dir, &board)?;
        self.boards
            .write()
            .map_err(|_| BoardsError::Poisoned)?
            .insert(board.id.clone(), board.clone());
        Ok(board)
    }

    /// Place only supported JPEG/PNG bytes. DesignCraft performs image import, sizing and
    /// placement; the board JSON stores links and metadata, never image bytes.
    pub fn place_reference(
        &self,
        board_id: &str,
        reference_id: &str,
        x_pt: f64,
        y_pt: f64,
        width_pt: f64,
        use_preview: bool,
    ) -> Result<WorkCrateBoard, BoardsError> {
        if ![x_pt, y_pt, width_pt].iter().all(|v| v.is_finite()) || width_pt <= 0.0 {
            return Err(BoardsError::InvalidPlacement);
        }
        let mut boards = self.boards.write().map_err(|_| BoardsError::Poisoned)?;
        let board = boards
            .get_mut(board_id)
            .ok_or_else(|| BoardsError::BoardNotFound(board_id.to_owned()))?;
        let refs = self.projects.references(&board.project_id)?;
        let reference = refs
            .iter()
            .find(|r| r.id == reference_id)
            .ok_or_else(|| BoardsError::ReferenceNotFound(reference_id.to_owned()))?;
        let path = self.resolve_placement_path(&board.project_id, reference, use_preview)?;
        let bytes = fs::read(&path)?;
        let (pixel_width, pixel_height) =
            designcraft_render::image_size(&bytes).ok_or(BoardsError::InvalidImage)?;
        if canvas_image_format(&path).is_none() {
            return Err(BoardsError::UnsupportedImageFormat);
        }

        let mut session = Session::new();
        session.add_document(DocState::new(board.source.clone(), None));
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("image.png");
        let (x_pt, y_pt, width_pt) = if board.placements.is_empty() && board.info.auto_from_first {
            board.info.pixel_size = Some([pixel_width, pixel_height]);
            board.title = name.to_owned();
            let page_width = board.source.settings.page_width;
            let page_height = board.source.settings.page_height;
            let header = page_height * 0.06;
            let fit_width = (page_width * 0.9)
                .min((page_height - header) * 0.9 * pixel_width as f64 / pixel_height as f64);
            (
                (page_width - fit_width) * 0.5,
                header
                    + (page_height - header - fit_width * pixel_height as f64 / pixel_width as f64)
                        * 0.5,
                fit_width,
            )
        } else {
            (x_pt, y_pt, width_pt)
        };
        if board.info.pixel_size.is_none() {
            board.info.pixel_size = Some([pixel_width, pixel_height]);
        }
        let result = session.execute("file.place", &json!({"base64": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes), "name": name, "x": x_pt, "y": y_pt, "width": width_pt}))?;
        let placed = session
            .active()
            .ok_or(BoardsError::NoDocument)?
            .doc
            .as_ref()
            .clone();
        let asset = placed
            .assets
            .values()
            .find(|asset| !board.source.assets.contains_key(&asset.id))
            .or_else(|| placed.assets.values().find(|asset| asset.link.is_none()))
            .cloned()
            .ok_or(BoardsError::PlacedAssetMissing)?;
        let mut document = placed;
        let mut fixed = asset.as_ref().clone();
        fixed.link = Some(path.to_string_lossy().into_owned());
        document.assets.insert(asset.id.clone(), Arc::new(fixed));
        let asset_key = asset.id.to_string();
        if board.placements.is_empty() {
            board.info.first_image = Some(asset_key.clone());
        }
        let sha256 = format!("{:x}", Sha256::digest(&bytes));
        board.source = document;
        board.placements.insert(
            asset_key,
            BoardPlacement {
                item_id: result.get("id").and_then(serde_json::Value::as_u64),
                source_document: None,
                reference_id: reference.id.clone(),
                uses_preview: use_preview,
                path: path.to_string_lossy().into_owned(),
                sha256,
                pixel_width,
                pixel_height,
                placed_width_pt: width_pt,
            },
        );
        write_board(&self.projects.project_directory(&board.project_id)?, board)?;
        Ok(board.clone())
    }

    /// Reopen a serialized upstream document and rehydrate linked image bytes before rendering.
    pub fn load_board(
        &self,
        project_id: &str,
        board_id: &str,
    ) -> Result<WorkCrateBoard, BoardsError> {
        let path = self
            .projects
            .project_directory(project_id)?
            .join("boards")
            .join(format!("{board_id}.json"));
        let mut board: WorkCrateBoard = serde_json::from_slice(&fs::read(path)?)?;
        if board.version != BOARD_VERSION || board.id != board_id || board.project_id != project_id
        {
            return Err(BoardsError::InvalidBoard);
        }
        // Product pages have fixed A3 geometry. Old physical-size fields are only
        // labels; do not continue the retired behavior that resized the document.
        if board.info.order > 0
            && ((board.source.settings.page_width - 420. * 72. / 25.4).abs() > 0.01
                || (board.source.settings.page_height - 297. * 72. / 25.4).abs() > 0.01)
        {
            let mut session = Session::new();
            session.add_document(DocState::new(board.source.clone(), None));
            session.execute(
                "layout.documentSetup",
                &json!({"width":420.*72./25.4,"height":297.*72./25.4,"adjustLayout":false}),
            )?;
            board.source = session
                .active()
                .ok_or(BoardsError::NoDocument)?
                .doc
                .as_ref()
                .clone();
        }
        // Retain editable manifest metadata even if a linked source cannot hydrate.
        if board.info.first_image.is_none() {
            board.info.first_image = board
                .placements
                .keys()
                .min_by_key(|id| id.parse::<u64>().unwrap_or(u64::MAX))
                .cloned();
        }
        for (asset, placement) in &mut board.placements {
            if placement.item_id.is_none() {
                placement.item_id = board
                    .source
                    .spreads
                    .iter()
                    .flat_map(|spread| spread.items.iter())
                    .find(|item| {
                        item.graphic()
                            .is_some_and(|graphic| graphic.asset.to_string() == *asset)
                    })
                    .map(|item| item.id.0);
            }
        }
        self.boards
            .write()
            .map_err(|_| BoardsError::Poisoned)?
            .entry(board.id.clone())
            .or_insert_with(|| board.clone());
        for (asset_id, placement) in &board.placements {
            let bytes = fs::read(&placement.path)?;
            if format!("{:x}", Sha256::digest(&bytes)) != placement.sha256 {
                return Err(BoardsError::SourceChanged(placement.path.clone()));
            }
            let (key, asset) = board
                .source
                .assets
                .iter()
                .find(|(key, _)| key.to_string() == *asset_id)
                .ok_or(BoardsError::PlacedAssetMissing)?;
            let mut hydrated = asset.as_ref().clone();
            hydrated.data = Arc::new(bytes);
            board.source.assets.insert(key.clone(), Arc::new(hydrated));
        }
        self.boards
            .write()
            .map_err(|_| BoardsError::Poisoned)?
            .insert(board.id.clone(), board.clone());
        Ok(board)
    }

    /// Render a page at the requested output-pixel density using DesignCraft's source-image
    /// mip selection. The caller supplies physical viewport/output scale in px per point.
    pub fn render_page(
        &self,
        board_id: &str,
        page_index: usize,
        pixels_per_point: f64,
    ) -> Result<designcraft_render::Rendered, BoardsError> {
        if !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
            return Err(BoardsError::InvalidPlacement);
        }
        let boards = self.boards.read().map_err(|_| BoardsError::Poisoned)?;
        let board = boards
            .get(board_id)
            .ok_or_else(|| BoardsError::BoardNotFound(board_id.to_owned()))?;
        let cache = Cache::new();
        if board.placements.keys().any(|id| {
            !board
                .source
                .assets
                .iter()
                .any(|(key, asset)| key.to_string() == *id && !asset.data.is_empty())
        }) {
            return Err(BoardsError::InvalidImage);
        }
        let document = self.presentation_document(board)?;
        let mut renderer = designcraft_render::Renderer::new();
        renderer
            .render_page(
                &document,
                &cache,
                page_index,
                pixels_per_point,
                false,
                &designcraft_render::RenderOptions::default(),
            )
            .ok_or(BoardsError::RenderFailed)
    }

    pub fn board(&self, board_id: &str) -> Result<WorkCrateBoard, BoardsError> {
        self.boards
            .read()
            .map_err(|_| BoardsError::Poisoned)?
            .get(board_id)
            .cloned()
            .ok_or_else(|| BoardsError::BoardNotFound(board_id.to_owned()))
    }

    /// Read board manifests without opening or hashing their linked image files.
    pub fn board_catalog(&self, project_id: &str) -> Result<Vec<WorkCrateBoard>, BoardsError> {
        let directory = self.projects.project_directory(project_id)?.join("boards");
        if !directory.is_dir() {
            return Ok(Vec::new());
        }
        let mut paths = fs::read_dir(directory)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "json")
            })
            .collect::<Vec<_>>();
        paths.sort();
        let mut boards = paths
            .iter()
            .map(|path| {
                let board: WorkCrateBoard = serde_json::from_slice(&fs::read(path)?)?;
                if board.version != BOARD_VERSION || board.project_id != project_id {
                    return Err(BoardsError::InvalidBoard);
                }
                Ok(board)
            })
            .collect::<Result<Vec<_>, BoardsError>>()?;
        boards.sort_by_key(|board| board.info.order);
        Ok(boards)
    }

    /// Read and hydrate a project's saved boards in stable creation/file order.
    pub fn boards_for_project(&self, project_id: &str) -> Result<Vec<WorkCrateBoard>, BoardsError> {
        self.board_catalog(project_id)?
            .into_iter()
            .map(|board| self.load_board(project_id, &board.id))
            .collect()
    }

    fn resolve_placement_path(
        &self,
        project_id: &str,
        reference: &WorkCrateFileReference,
        use_preview: bool,
    ) -> Result<PathBuf, BoardsError> {
        if use_preview {
            let preview = reference
                .custom_preview
                .as_ref()
                .ok_or(BoardsError::PreviewRequired)?;
            let root = self.projects.project_directory(project_id)?;
            let path = root.join(&preview.path);
            if !path.starts_with(&root) {
                return Err(BoardsError::InvalidBoard);
            }
            Ok(path)
        } else {
            let path = PathBuf::from(&reference.path);
            if canvas_image_format(&path).is_none() {
                return Err(BoardsError::PreviewRequired);
            }
            Ok(path)
        }
    }
}

fn write_board(project_dir: &Path, board: &WorkCrateBoard) -> Result<(), BoardsError> {
    let dir = project_dir.join("boards");
    fs::create_dir_all(&dir)?;
    let target = dir.join(format!("{}.json", board.id));
    let temp = dir.join(format!(".{}.tmp", board.id));
    fs::write(&temp, serde_json::to_vec_pretty(board)?)?;
    if target.exists() {
        fs::copy(&target, target.with_extension("json.bak"))?;
    }
    fs::rename(temp, target)?;
    Ok(())
}

fn validate_dimensions(width: f64, height: f64) -> Result<(), BoardsError> {
    if width.is_finite()
        && height.is_finite()
        && width > 0.0
        && height > 0.0
        && width <= 100_000.0
        && height <= 100_000.0
    {
        Ok(())
    } else {
        Err(BoardsError::InvalidDimensions)
    }
}

#[derive(Debug, Error)]
pub enum BoardsError {
    #[error(transparent)]
    Projects(#[from] WorkCrateProjectsError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    DesignCraft(#[from] designcraft_engine::EngineError),
    #[error("board was not found: {0}")]
    BoardNotFound(String),
    #[error("file reference was not found: {0}")]
    ReferenceNotFound(String),
    #[error("board dimensions are invalid")]
    InvalidDimensions,
    #[error("placement coordinates or width are invalid")]
    InvalidPlacement,
    #[error("only valid JPEG/JPG or PNG image files can be placed on a board")]
    UnsupportedImageFormat,
    #[error("the image is corrupt or cannot be decoded")]
    InvalidImage,
    #[error("this reference needs a JPEG/PNG preview before it can be placed")]
    PreviewRequired,
    #[error("upstream placement did not produce an image asset")]
    PlacedAssetMissing,
    #[error("board document is invalid or mismatched")]
    InvalidBoard,
    #[error("source image changed since placement: {0}")]
    SourceChanged(String),
    #[error("DesignCraft session has no active document")]
    NoDocument,
    #[error("service lock was poisoned")]
    Poisoned,
    #[error("Cordis dependency or service publication failed: {0}")]
    Plugin(String),
    #[error("DesignCraft could not render the requested board page at this scale")]
    RenderFailed,
}

pub struct WorkCrateBoardsPlugin;
impl Plugin for WorkCrateBoardsPlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = BoardsError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "fileatlas.workcrate-boards".into()
    }
    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }
    async fn apply(&self, context: Context, _input: &Self::Input) -> Result<(), Self::ApplyError> {
        let projects = context
            .try_service::<WorkCrateProjectsService>()
            .map_err(|error| BoardsError::Plugin(error.to_string()))?;
        let _publication = context
            .provide(Arc::new(WorkCrateBoardsService::new(projects)))
            .map_err(|error| BoardsError::Plugin(error.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    #[test]
    fn board_places_only_decodable_jpeg_png_and_serializes_links_not_pixels() {
        let temp = tempfile::tempdir().unwrap();
        let projects =
            Arc::new(WorkCrateProjectsService::open_at(temp.path().join("data")).unwrap());
        let project = projects.create_project("Board test").unwrap();
        let source = temp.path().join("source.png");
        let image = ImageBuffer::from_fn(32, 24, |x, y| Rgb([(x * 7) as u8, (y * 9) as u8, 140]));
        image.save(&source).unwrap();
        let reference = projects
            .add_references(&project.id, std::slice::from_ref(&source))
            .unwrap()
            .remove(0);
        let service = WorkCrateBoardsService::new(Arc::clone(&projects));
        let board = service
            .create_board(&project.id, "Source fidelity", 360.0, 240.0)
            .unwrap();
        let placed = service
            .place_reference(&board.id, &reference.id, 12.0, 18.0, 144.0, false)
            .unwrap();
        let placement = placed.placements.values().next().unwrap();
        assert_eq!((placement.pixel_width, placement.pixel_height), (32, 24));

        let serialized = serde_json::to_string(&placed).unwrap();
        assert!(!serialized.contains("data:image"));
        assert!(!serialized.contains("[0,0,140"));

        let reopened = service.load_board(&project.id, &board.id).unwrap();
        assert!(
            reopened
                .source
                .assets
                .values()
                .any(|asset| asset.data.len() > 0)
        );
        let rendered = service.render_page(&board.id, 0, 2.0).unwrap();
        assert_eq!((rendered.width, rendered.height), (720, 480));
    }

    #[test]
    fn non_image_references_need_an_attached_preview_to_be_placed() {
        let temp = tempfile::tempdir().unwrap();
        let projects =
            Arc::new(WorkCrateProjectsService::open_at(temp.path().join("data")).unwrap());
        let project = projects.create_project("Preview only").unwrap();
        let source = temp.path().join("layout.psd");
        fs::write(&source, b"not an image").unwrap();
        let reference = projects
            .add_references(&project.id, std::slice::from_ref(&source))
            .unwrap()
            .remove(0);
        let service = WorkCrateBoardsService::new(projects);
        let board = service
            .create_board(&project.id, "Board", 300.0, 200.0)
            .unwrap();
        assert!(matches!(
            service.place_reference(&board.id, &reference.id, 0.0, 0.0, 80.0, false),
            Err(BoardsError::PreviewRequired)
        ));
    }
}

mod editing;
pub use editing::{ImageLayout, RefreshProjectImagesReport};
