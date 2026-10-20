//! Disposable v1 previews. Source documents and linked files remain authoritative.
use super::*;
use std::io::{Cursor, Write};

impl WorkCrateBoardsService {
    /// Includes geometry, labels, source digests and the inherited project name.
    pub fn preview_revision(&self, board: &WorkCrateBoard) -> Result<String, BoardsError> {
        let bytes = serde_json::to_vec(&(1, board, self.project_name(board)?))?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }

    fn cache_directory(&self, project: &str) -> Result<PathBuf, BoardsError> {
        Ok(self.projects.project_directory(project)?.join("cache-v1"))
    }

    /// Cache hits never open, hash or decode linked originals.
    pub fn cached_preview(&self, board: &WorkCrateBoard) -> Option<image::RgbaImage> {
        let root = self.cache_directory(&board.project_id).ok()?;
        let revision = self.preview_revision(board).ok()?;
        image::open(root.join(format!("{}-{revision}.png", board.id)))
            .ok()
            .map(|i| i.into_rgba8())
    }

    /// Lossless display proxy, rounded up to a resolution tier; never upscales a source.
    /// Export never consumes these files. Missing/corrupt caches rebuild from verified links.
    pub fn display_image(
        &self,
        board: &WorkCrateBoard,
        asset: &str,
        requested_side: u32,
    ) -> Result<image::RgbaImage, BoardsError> {
        let placed = board
            .placements
            .get(asset)
            .ok_or(BoardsError::PlacedAssetMissing)?;
        let side = requested_side
            .max(128)
            .checked_next_power_of_two()
            .unwrap_or(u32::MAX)
            .min(placed.pixel_width.max(placed.pixel_height));
        let root = self.cache_directory(&board.project_id)?;
        let path = root.join(format!("{}-{side}.png", placed.sha256));
        if let Ok(image) = image::open(&path) {
            return Ok(image.into_rgba8());
        }
        // Reuse an existing larger tier instead of decoding the original again.
        let mut larger = side;
        while larger < placed.pixel_width.max(placed.pixel_height) {
            larger = larger
                .saturating_mul(2)
                .min(placed.pixel_width.max(placed.pixel_height));
            if let Ok(image) = image::open(root.join(format!("{}-{larger}.png", placed.sha256))) {
                let image = image.resize(side, side, image::imageops::FilterType::Triangle);
                let mut encoded = Cursor::new(Vec::new());
                image
                    .write_to(&mut encoded, image::ImageFormat::Png)
                    .map_err(|_| BoardsError::InvalidImage)?;
                write_cache(&path, encoded.get_ref())?;
                return Ok(image.into_rgba8());
            }
        }
        let bytes = board
            .source
            .assets
            .iter()
            .find(|(id, _)| id.to_string() == asset)
            .map(|(_, a)| a.data.clone())
            .filter(|data| !data.is_empty())
            .map(Ok)
            .unwrap_or_else(|| fs::read(&placed.path).map(Arc::new))?;
        if format!("{:x}", Sha256::digest(bytes.as_slice())) != placed.sha256 {
            return Err(BoardsError::SourceChanged(placed.path.clone()));
        }
        let image = image::load_from_memory(&bytes).map_err(|_| BoardsError::InvalidImage)?;
        let image = if image.width().max(image.height()) > side {
            image.resize(side, side, image::imageops::FilterType::Triangle)
        } else {
            image
        };
        let mut encoded = Cursor::new(Vec::new());
        image
            .write_to(&mut encoded, image::ImageFormat::Png)
            .map_err(|_| BoardsError::InvalidImage)?;
        write_cache(&path, encoded.get_ref())?;
        Ok(image.into_rgba8())
    }

    /// Render only a small page using proxies and DesignCraft's original frame/clip geometry.
    pub fn cache_preview(&self, board: &WorkCrateBoard) -> Result<image::RgbaImage, BoardsError> {
        let revision = self.preview_revision(board)?;
        let mut preview = board.clone();
        for id in board.placements.keys() {
            let image = self.display_image(board, id, 1024)?;
            let mut encoded = Cursor::new(Vec::new());
            image
                .write_to(&mut encoded, image::ImageFormat::Png)
                .map_err(|_| BoardsError::InvalidImage)?;
            if let Some((key, asset)) = preview
                .source
                .assets
                .iter()
                .find(|(key, _)| key.to_string() == *id)
            {
                let mut replacement = asset.as_ref().clone();
                replacement.data = Arc::new(encoded.into_inner());
                preview.source.assets.insert(*key, Arc::new(replacement));
            }
        }
        let document = self.presentation_document(&preview)?;
        let page = designcraft_render::Renderer::new()
            .render_page(
                &document,
                &Cache::new(),
                0,
                0.6,
                false,
                &designcraft_render::RenderOptions::default(),
            )
            .ok_or(BoardsError::RenderFailed)?;
        let pixels = image::RgbaImage::from_raw(page.width, page.height, page.to_straight())
            .ok_or(BoardsError::RenderFailed)?;
        let root = self.cache_directory(&board.project_id)?;
        if self.preview_revision(board)? != revision {
            // An inherited header changed while this page was rendering.
            return Err(BoardsError::InvalidBoard);
        }
        let mut encoded = Cursor::new(Vec::new());
        pixels
            .write_to(&mut encoded, image::ImageFormat::Png)
            .map_err(|_| BoardsError::InvalidImage)?;
        // The revision is part of the atomic payload filename: interrupted or
        // out-of-order workers cannot pair a new revision with old page pixels.
        write_cache(
            &root.join(format!("{}-{revision}.png", board.id)),
            encoded.get_ref(),
        )?;
        prune_page_previews(&root, &board.id);
        Ok(pixels)
    }

    /// Action-time original hydration, independent of viewport proxy resolution.
    pub fn prepare_export(&self, id: &str) -> Result<WorkCrateBoard, BoardsError> {
        let board = self.board(id)?;
        self.load_board(&board.project_id, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_previews_survive_reopen_without_originals_and_invalidate_on_edits()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let projects = Arc::new(WorkCrateProjectsService::open_at(
            temp.path().join("store"),
        )?);
        let project = projects.create_project("Cache proof")?;
        let path = temp.path().join("linked.png");
        image::RgbaImage::from_pixel(800, 400, image::Rgba([19, 111, 239, 255])).save(&path)?;
        let reference = projects
            .add_references(&project.id, &[path.clone()])?
            .remove(0);
        let service = WorkCrateBoardsService::new(projects.clone());
        let board = service.create_page(&project.id)?;
        let board = service.place_reference(&board.id, &reference.id, 0., 80., 420., false)?;
        let asset = board.placements.keys().next().ok_or("no placement")?;
        let proxy = service.display_image(&board, asset, 200)?;
        assert_eq!(proxy.dimensions(), (256, 128));
        assert_eq!(
            fs::read(&path)?,
            board
                .source
                .assets
                .values()
                .next()
                .ok_or("no asset")?
                .data
                .as_ref()
                .clone()
        );
        let revision = service.preview_revision(&board)?;
        let preview = service.cache_preview(&board)?;
        assert!(preview.width() < 800);
        drop(service);
        fs::rename(&path, path.with_extension("offline"))?;
        let reopened = WorkCrateBoardsService::new(projects.clone());
        let board = reopened.board_catalog(&project.id)?.remove(0);
        assert!(board.source.assets.values().all(|a| a.data.is_empty()));
        assert!(reopened.cached_preview(&board).is_some());
        assert_eq!(
            reopened.display_image(&board, asset, 200)?.dimensions(),
            (256, 128)
        );
        assert!(
            reopened.prepare_export(&board.id).is_err(),
            "exports must require originals"
        );
        projects.rename_project(&project.id, "Renamed")?;
        assert_ne!(revision, reopened.preview_revision(&board)?);
        assert!(reopened.cached_preview(&board).is_none());
        let mut info = board.info.clone();
        info.material = "Updated header".into();
        let edited = reopened.update_info(&board.id, &board.title, info)?;
        assert!(reopened.cached_preview(&edited).is_none());
        Ok(())
    }
}

fn prune_page_previews(root: &Path, id: &str) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let prefix = format!("{id}-");
    let mut pages = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.starts_with(&prefix)
                && name.ends_with(".png")
                && name.len() == prefix.len() + 64 + 4
        })
        .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
        .collect::<Vec<_>>();
    pages.sort_by_key(|(time, _)| std::cmp::Reverse(*time));
    for (_, path) in pages.into_iter().skip(2) {
        let _ = fs::remove_file(path);
    }
}

fn write_cache(path: &Path, bytes: &[u8]) -> Result<(), BoardsError> {
    let parent = path.parent().ok_or(BoardsError::InvalidBoard)?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.persist(path).map_err(|e| e.error)?;
    Ok(())
}
