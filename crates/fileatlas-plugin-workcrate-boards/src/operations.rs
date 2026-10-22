use super::*;
use crate::protocol::NativeProtocol;
impl WorkCrateBoardsService {
    pub fn create_page(&self, project: &str) -> Result<WorkCrateBoard, BoardsError> {
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let path = self
            .document_path(project)?
            .ok_or(BoardsError::NoDocument)?;
        let binding = self.projects.document_binding(project)?;
        let mut c =
            self.official_session(std::slice::from_ref(&path), std::slice::from_ref(&path))?;
        c.open_document(&path)?;
        let w = 420. * 72. / 25.4;
        let h = 297. * 72. / 25.4;
        let count = native::catalog(&mut c, project, &binding)?.len();
        // Official default placement follows the last *ordered* board, which can
        // overlap another board after reordering. Create to the right of all boards.
        let inspect = c.request("document.inspect", json!({"depth":0}))?;
        let artboards = inspect["artboards"]
            .as_array()
            .ok_or_else(|| BoardsError::Protocol("native artboards missing".into()))?;
        let mut right = f64::NEG_INFINITY;
        let mut top = f64::INFINITY;
        for rect in artboards {
            let number = |key: &str| {
                rect[key]
                    .as_f64()
                    .filter(|n| n.is_finite())
                    .ok_or_else(|| BoardsError::Protocol("native artboard geometry missing".into()))
            };
            right = right.max(number("x")? + number("width")?);
            top = top.min(number("y")?);
        }
        if !right.is_finite() || !top.is_finite() {
            return Err(BoardsError::NoDocument);
        }
        c.execute(
            "artboard.new",
            json!({"x":right+20.,"y":top,"width":w,"height":h,"name":page_title(count)}),
        )?;
        let mut boards = native::catalog(&mut c, project, &binding)?;
        let board = boards.pop().ok_or(BoardsError::NoDocument)?;
        self.write_info(
            &mut c,
            &board,
            &board.title,
            PageInfo {
                order: board.index as u32 + 1,
                ..Default::default()
            },
        )?;
        c.save_file(&path)?;
        let _ = self.refresh_cache_from_client(&mut c, project, binding)?;
        Ok(board)
    }
    /// Save one page's name and material fields. An explicitly opened canvas session that
    /// already holds this page is reused, so the overlay keeps its tool, selection, rail and
    /// scroll instead of being rebuilt; only this page is re-rendered either way.
    pub fn update_info(
        &self,
        id: &str,
        title: &str,
        info: PageInfo,
    ) -> Result<InfoSave, BoardsError> {
        if title != page_title(self.board(id)?.index) {
            return Err(BoardsError::Invalid(
                "页面标题由顺序自动生成，不支持手动修改".into(),
            ));
        }
        let (project, _) = native::split_id(id)?;
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let current = self
            .board_catalog(project)?
            .into_iter()
            .find(|board| board.id == id);
        let path = self
            .document_path(project)?
            .ok_or(BoardsError::NoDocument)?;
        let binding = self.projects.document_binding(project)?;
        if id.split(':').nth(1) != Some(binding.as_str()) {
            return Err(BoardsError::Invalid(
                "本页所属原生工程的关联已变化，请先同步工程与预览后再保存".into(),
            ));
        }
        if let Some(board) = current
            // Nothing to write and nothing to re-render: the stored frame already shows it.
            && board.title == title
            && board.info == info
        {
            return Ok(InfoSave {
                board,
                frame: None,
                changed: false,
            });
        }
        if self.canvas_is_open() && self.canvas_holds(id)? {
            let (board, frame) = self.canvas_write_info(id, title, info, &binding)?;
            return Ok(InfoSave {
                board,
                frame: Some(frame),
                changed: true,
            });
        }
        let mut c =
            self.official_session(std::slice::from_ref(&path), std::slice::from_ref(&path))?;
        c.open_document(&path)?;
        let mut board = native::catalog(&mut c, project, &binding)?
            .into_iter()
            .find(|b| b.id == id)
            .ok_or_else(|| BoardsError::BoardNotFound(id.into()))?;
        self.write_info(&mut c, &board, title, info.clone())?;
        c.save_file(&path)?;
        board.title = title.into();
        board.info = info;
        self.store_page(&mut c, project, binding, &board)?;
        Ok(InfoSave {
            board,
            frame: None,
            changed: true,
        })
    }

    /// Patch one page's record and bitmap into the durable catalogue. Every other page keeps
    /// its own identity and preview file, so a field save never redraws another card. A
    /// project with no usable catalogue still needs the whole page list, so it rebuilds.
    fn store_page(
        &self,
        c: &mut mcp::McpSession,
        project: &str,
        binding: String,
        board: &WorkCrateBoard,
    ) -> Result<(), BoardsError> {
        let Some(mut cache) = self.read_cache(project)? else {
            self.refresh_cache_from_client(c, project, binding)?;
            return Ok(());
        };
        if !cache.boards.iter().any(|b| b.id == board.id) {
            self.refresh_cache_from_client(c, project, binding)?;
            return Ok(());
        }
        for (key, relative) in self.make_previews(c, std::slice::from_ref(board))? {
            cache.previews.insert(key, relative);
        }
        if let Some(entry) = cache.boards.iter_mut().find(|b| b.id == board.id) {
            *entry = board.clone();
        }
        cache.source = self.source_stamp(project);
        self.write_cache(project, &cache)
    }
    pub(crate) fn write_info(
        &self,
        c: &mut impl NativeProtocol,
        b: &WorkCrateBoard,
        title: &str,
        info: PageInfo,
    ) -> Result<(), BoardsError> {
        let project_name = info.material_category.clone();
        let report = header::write(c, b, title, &info, &project_name)?;
        *self
            .header_report
            .lock()
            .map_err(|_| BoardsError::Poisoned)? = json!({"board_id":b.id,"report":report});
        Ok(())
    }
    pub fn save_project(&self, project: &str) -> Result<(), BoardsError> {
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let path = self
            .document_path(project)?
            .ok_or(BoardsError::NoDocument)?;
        let binding = self.projects.document_binding(project)?;
        let mut c =
            self.official_session(std::slice::from_ref(&path), std::slice::from_ref(&path))?;
        c.open_document(&path)?;
        c.save_file(&path)?;
        self.refresh_cache_from_client(&mut c, project, binding)?;
        Ok(())
    }
    pub fn open_editor(&self, id: &str) -> Result<(), BoardsError> {
        self.connect()?;
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let mut c = self.client()?;
        let b = self.target(&mut c, id)?;
        c.execute("view.goToArtboard", json!({"index":b.index}))?;
        c.request("ui.focus", json!({}))?;
        Ok(())
    }
    pub fn render_preview(&self, id: &str, max_width: u32) -> Result<PathBuf, BoardsError> {
        let (project, _) = native::split_id(id)?;
        let cache = self
            .read_cache(project)?
            .ok_or_else(|| BoardsError::PreviewUnavailable(id.into()))?;
        let candidates = cache
            .previews
            .iter()
            .filter_map(|(key, relative)| {
                let (board_id, width) = key.rsplit_once('@')?;
                if board_id != id {
                    return None;
                }
                width.parse::<u32>().ok().map(|width| (width, relative))
            })
            .collect::<Vec<_>>();
        let requested = max_width.clamp(64, 2048);
        let (_, relative) = candidates
            .iter()
            .filter(|(width, _)| *width >= requested)
            .min_by_key(|(width, _)| *width)
            .or_else(|| candidates.iter().max_by_key(|(width, _)| *width))
            .ok_or_else(|| BoardsError::PreviewUnavailable(id.into()))?;
        let root = self.projects.project_directory(project)?;
        let root_canonical = std::fs::canonicalize(&root)?;
        let preview = std::fs::canonicalize(root.join(relative))?;
        if !preview.starts_with(root_canonical) {
            return Err(BoardsError::Invalid(
                "cached preview outside project".into(),
            ));
        }
        Ok(preview)
    }
    pub fn place_reference(
        &self,
        id: &str,
        reference: &str,
        x: f64,
        y: f64,
        width: f64,
        use_preview: bool,
    ) -> Result<(), BoardsError> {
        if ![x, y, width].iter().all(|n| n.is_finite()) || width <= 0. {
            return Err(BoardsError::Invalid("invalid placement".into()));
        }
        let (project, _) = native::split_id(id)?;
        let r = self
            .projects
            .references(project)?
            .into_iter()
            .find(|r| r.id == reference)
            .ok_or_else(|| BoardsError::ReferenceNotFound(reference.into()))?;
        let path = if use_preview {
            let preview = r
                .custom_preview
                .ok_or_else(|| BoardsError::Invalid("reference has no preview".into()))?;
            let root = self.projects.project_directory(project)?;
            let p = std::fs::canonicalize(root.join(preview.path))?;
            if !p.starts_with(std::fs::canonicalize(root)?) {
                return Err(BoardsError::Invalid("preview outside project".into()));
            }
            p
        } else {
            PathBuf::from(r.path)
        };
        self.place_at(id, &path, Some((x, y, width)))
    }
    pub fn place_file(&self, id: &str, path: &Path) -> Result<(), BoardsError> {
        self.place_at(id, path, None)
    }
    fn place_at(
        &self,
        id: &str,
        path: &Path,
        placement: Option<(f64, f64, f64)>,
    ) -> Result<(), BoardsError> {
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let (project, _) = native::split_id(id)?;
        let native_path = self
            .document_path(project)?
            .ok_or(BoardsError::NoDocument)?;
        let binding = self.projects.document_binding(project)?;
        if id.split(':').nth(1) != Some(binding.as_str()) {
            return Err(BoardsError::Invalid(
                "本页所属原生工程的关联已变化，请先同步工程与预览后再保存".into(),
            ));
        }
        let source = std::fs::canonicalize(path)?;
        let reads = vec![native_path.clone(), source.clone()];
        let mut c = self.official_session(&reads, std::slice::from_ref(&native_path))?;
        c.open_document(&native_path)?;
        let b = native::catalog(&mut c, project, &binding)?
            .into_iter()
            .find(|b| b.id == id)
            .ok_or_else(|| BoardsError::BoardNotFound(id.into()))?;
        let i = c.request("document.inspect", json!({"depth":0}))?;
        let r = &i["artboards"][b.index];
        let x = r["x"].as_f64().unwrap_or(0.);
        let y = r["y"].as_f64().unwrap_or(0.);
        let (px, py, w) = placement.unwrap_or((24., 128., b.width_pt - 48.));
        let result = c.execute(
            "file.place",
            json!({"path":source,"rect":[x+px,y+py,w,(b.height_pt-py-24.).max(1.)]}),
        )?;
        if result.is_null() {
            return Err(BoardsError::Invalid("placement pending dialog".into()));
        }
        let ids = result["ids"]
            .as_array()
            .map(|items| items.iter().filter_map(Value::as_u64).collect::<Vec<_>>())
            .unwrap_or_default();
        native::group_content(&mut c, b.native_id, &ids)?;
        c.save_file(&native_path)?;
        self.refresh_cache_from_client(&mut c, project, binding)?;
        Ok(())
    }
    pub fn export_native(
        &self,
        ids: &[String],
        path: &Path,
        format: &str,
    ) -> Result<Value, BoardsError> {
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let first = ids
            .first()
            .ok_or_else(|| BoardsError::Invalid("empty selection".into()))?;
        let (project, _) = native::split_id(first)?;
        let native_path = self
            .document_path(project)?
            .ok_or(BoardsError::NoDocument)?;
        let binding = self.projects.document_binding(project)?;
        let mut c = self.official_session(
            std::slice::from_ref(&native_path),
            &[path.parent().unwrap_or(Path::new(".")).to_owned()],
        )?;
        c.open_document(&native_path)?;
        let b = native::catalog(&mut c, project, &binding)?
            .into_iter()
            .find(|b| b.id == *first)
            .ok_or_else(|| BoardsError::BoardNotFound(first.clone()))?;
        let mut indices = Vec::new();
        for id in ids {
            let (other_project, _) = native::split_id(id)?;
            if other_project != project || id.split(':').nth(1) != Some(binding.as_str()) {
                return Err(BoardsError::Invalid(
                    "PDF pages must belong to one associated native document".into(),
                ));
            }
            let target = native::catalog(&mut c, project, &binding)?
                .into_iter()
                .find(|board| board.id == *id)
                .ok_or_else(|| BoardsError::BoardNotFound(id.clone()))?;
            if target.project_id != b.project_id {
                return Err(BoardsError::Invalid(
                    "PDF pages must belong to one native document".into(),
                ));
            }
            indices.push(target.index + 1)
        }
        let parent = path
            .parent()
            .ok_or_else(|| BoardsError::Invalid("output has no directory".into()))?;
        let temporary = tempfile::Builder::new()
            .prefix(".workcrate-export-")
            .suffix(&format!(".{format}"))
            .tempfile_in(parent)?;
        let official_path = temporary.path();
        let params = if format == "pdf" {
            pdf_export_params(official_path, &indices)
        } else {
            if ids.len() != 1 {
                return Err(BoardsError::Invalid("one raster page required".into()));
            }
            raster_export_params(official_path, format, b.index)
        };
        let result = c.request("app.export", params)?;
        let start = std::time::Instant::now();
        while !std::fs::metadata(official_path).is_ok_and(|m| m.is_file() && m.len() > 0) {
            if start.elapsed() > std::time::Duration::from_secs(65) {
                return Err(BoardsError::Protocol(
                    "official export produced no output".into(),
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        temporary.persist(path).map_err(|e| e.error)?;
        Ok(result)
    }
}

/// VectorCraft 0.14.0 `vectorcraft-cli.exe mcp --headless` lists the `export` tool with
/// `additionalProperties:false`, so any key outside this set fails the whole export.
#[cfg(test)]
const OFFICIAL_EXPORT_ARGS: [&str; 8] = [
    "artboard",
    "format",
    "options",
    "outlineText",
    "path",
    "range",
    "scale",
    "selection",
];
/// One PDF page per chosen artboard; the official `range` is 1-based.
/// JPEG-compress color/gray images at print resolution, retaining native text/vectors.
/// The official PDF exporter requires opaque images for JPEG compression.
fn pdf_export_params(path: &Path, indices: &[usize]) -> Value {
    let image_compression = json!({
        "downsample":"bicubic","ppi":300,"abovePpi":300,
        "compression":"jpeg","quality":"high"
    });
    json!({
        "path":path,"format":"pdf",
        "range":indices.iter().map(usize::to_string).collect::<Vec<_>>().join(","),
        "options":{
            "preset":"VectorCraft Default","preserveEditing":false,
            "compression":{
                "color":image_compression,"gray":image_compression,"compressText":true
            }
        }
    })
}
/// A 300 dpi raster of one artboard, selected by native index.
fn raster_export_params(path: &Path, format: &str, artboard: usize) -> Value {
    json!({"path":path,"format":format,"artboard":artboard,"scale":300./72.})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn export_params_only_use_official_tool_arguments() {
        let pdf = pdf_export_params(Path::new("pages.pdf"), &[1, 3]);
        let raster = raster_export_params(Path::new("page.png"), "png", 0);
        for params in [&pdf, &raster] {
            for key in params.as_object().expect("object payload").keys() {
                assert!(
                    OFFICIAL_EXPORT_ARGS.contains(&key.as_str()),
                    "official headless export rejects unknown argument `{key}`"
                );
            }
        }
        assert_eq!(pdf["range"], "1,3");
        assert_eq!(pdf["options"]["preset"], "VectorCraft Default");
        assert_eq!(pdf["options"]["preserveEditing"], false);
        for kind in ["color", "gray"] {
            let compression = &pdf["options"]["compression"][kind];
            assert_eq!(compression["compression"], "jpeg");
            assert_eq!(compression["quality"], "high");
            assert_eq!(compression["ppi"], 300);
            assert_eq!(compression["abovePpi"], 300);
            assert_eq!(compression["downsample"], "bicubic");
        }
        assert_eq!(pdf["options"]["compression"]["compressText"], true);
        assert_eq!(raster["artboard"], 0);
        assert_eq!(raster["scale"], 300. / 72.);
    }
}
