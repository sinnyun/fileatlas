use super::*;

/// WorkCrate presentation data; DesignCraft remains the page and artwork model.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PageInfo {
    pub order: u32,
    pub auto_from_first: bool,
    pub pixel_size: Option<[u32; 2]>,
    pub first_image: Option<String>,
    pub physical_size_cm: Option<[f64; 2]>,
    pub size_label: String,
    pub project_name_override: Option<String>,
    pub material: String,
    pub size: String,
    pub quantity: String,
    pub extra: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_image_sets_source_info_without_changing_a3_and_names_inherit()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let projects = Arc::new(WorkCrateProjectsService::open_at(temp.path().join("data"))?);
        let project = projects.create_project("Global")?;
        let source = temp.path().join("first.png");
        image::RgbImage::new(320, 240).save(&source)?;
        let reference = projects.add_references(&project.id, &[source])?.remove(0);
        let service = WorkCrateBoardsService::new(Arc::clone(&projects));
        let page = service.create_page(&project.id)?;
        let first = service.place_reference(&page.id, &reference.id, 24., 24., 420., false)?;
        assert_eq!(first.title, "first.png");
        assert_eq!(first.info.pixel_size, Some([320, 240]));
        assert_eq!(first.info.physical_size_cm, None);
        assert!((first.source.settings.page_width - 420. * 72. / 25.4).abs() < 0.01);
        let asset = first
            .placements
            .keys()
            .next()
            .ok_or("missing placement")?
            .clone();
        let replacement = temp.path().join("replacement.png");
        image::RgbImage::new(640, 480).save(&replacement)?;
        let refreshed = service.refresh_image(&page.id, &asset, Some(&replacement))?;
        assert_eq!(refreshed.placements[&asset].pixel_width, 640);
        assert_eq!(refreshed.info.pixel_size, Some([640, 480]));
        service.transform_image(&page.id, &asset, 5., 5., 0.9, 15.)?;
        service.place_reference(&page.id, &reference.id, 10., 10., 120., false)?;
        let mut info = service.board(&page.id)?.info;
        info.project_name_override = Some("Local".into());
        service.update_info(&page.id, "Custom page", info)?;
        projects.rename_project(&project.id, "Changed global")?;
        let reopened = service.load_board(&project.id, &page.id)?;
        assert_eq!(service.project_name(&reopened)?, "Local");
        let mut info = reopened.info.clone();
        info.project_name_override = None;
        let reset = service.update_info(&page.id, &reopened.title, info)?;
        assert_eq!(service.project_name(&reset)?, "Changed global");
        assert_eq!(reset.title, "Custom page");
        Ok(())
    }
}

impl WorkCrateBoardsService {
    pub fn create_page(&self, project_id: &str) -> Result<WorkCrateBoard, BoardsError> {
        let order = self.board_catalog(project_id)?.len() as u32 + 1;
        let board = self.create_board(
            project_id,
            "未命名页面",
            420.0 * 72.0 / 25.4,
            297.0 * 72.0 / 25.4,
        )?;
        self.edit_board(&board.id, |board| {
            board.info.auto_from_first = true;
            board.info.order = order;
            board.info.physical_size_cm = None;
            Ok(())
        })
    }

    pub fn update_info(
        &self,
        id: &str,
        title: &str,
        info: PageInfo,
    ) -> Result<WorkCrateBoard, BoardsError> {
        if title.trim().is_empty() {
            return Err(BoardsError::InvalidBoard);
        }
        if let Some([w, h]) = info.physical_size_cm {
            validate_dimensions(w, h)?;
        }
        self.edit_board(id, |board| {
            if info.pixel_size != board.info.pixel_size {
                return Err(BoardsError::InvalidBoard);
            }
            board.title = title.trim().to_owned();
            board.source.title = board.title.clone();
            board.info = info;
            Ok(())
        })
    }

    /// Delegates movement/scale/rotation to pinned DesignCraft engine commands.
    pub fn transform_image(
        &self,
        id: &str,
        asset: &str,
        dx: f64,
        dy: f64,
        scale: f64,
        angle: f64,
    ) -> Result<WorkCrateBoard, BoardsError> {
        if ![dx, dy, scale, angle].iter().all(|v| v.is_finite()) || !(0.01..=100.0).contains(&scale)
        {
            return Err(BoardsError::InvalidPlacement);
        }
        self.edit_board(id, |board| {
            let placement = board
                .placements
                .get_mut(asset)
                .ok_or(BoardsError::PlacedAssetMissing)?;
            let item = placement.item_id.ok_or(BoardsError::PlacedAssetMissing)?;
            let mut session = Session::new();
            session.add_document(DocState::new(board.source.clone(), None));
            session.execute("selection.set", &json!({"ids":[item]}))?;
            session.execute("transform.move", &json!({"ids":[item],"dx":dx,"dy":dy}))?;
            session.execute(
                "transform.scale",
                &json!({"ids":[item],"sx":scale,"sy":scale}),
            )?;
            session.execute("transform.rotate", &json!({"ids":[item],"angle":angle}))?;
            board.source = session
                .active()
                .ok_or(BoardsError::NoDocument)?
                .doc
                .as_ref()
                .clone();
            placement.placed_width_pt *= scale;
            Ok(())
        })
    }

    pub fn link_source(
        &self,
        id: &str,
        asset: &str,
        path: Option<String>,
    ) -> Result<WorkCrateBoard, BoardsError> {
        self.edit_board(id, |board| {
            board
                .placements
                .get_mut(asset)
                .ok_or(BoardsError::PlacedAssetMissing)?
                .source_document = path;
            Ok(())
        })
    }

    pub fn refresh_image(
        &self,
        id: &str,
        asset: &str,
        replacement: Option<&Path>,
    ) -> Result<WorkCrateBoard, BoardsError> {
        self.edit_board(id, |board| {
            let placement = board
                .placements
                .get_mut(asset)
                .ok_or(BoardsError::PlacedAssetMissing)?;
            let path = replacement
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from(&placement.path));
            if canvas_image_format(&path).is_none() {
                return Err(BoardsError::UnsupportedImageFormat);
            }
            let bytes = fs::read(&path)?;
            let (w, h) = designcraft_render::image_size(&bytes).ok_or(BoardsError::InvalidImage)?;
            let key = board
                .source
                .assets
                .keys()
                .find(|id| id.to_string() == asset)
                .copied()
                .ok_or(BoardsError::PlacedAssetMissing)?;
            let mut session = Session::new();
            session.add_document(DocState::new(board.source.clone(), None));
            session.execute(
                "links.relink",
                &json!({"asset":key.0,"path":path.to_string_lossy()}),
            )?;
            if (w as f64 / h as f64 - placement.pixel_width as f64 / placement.pixel_height as f64)
                .abs()
                > 1e-6
            {
                let mut doc = session
                    .active()
                    .ok_or(BoardsError::NoDocument)?
                    .doc
                    .as_ref()
                    .clone();
                let item = placement.item_id.ok_or(BoardsError::PlacedAssetMissing)?;
                if let Some(item) = doc.item_mut(designcraft_doc::ItemId(item)) {
                    if let designcraft_doc::Content::Graphic(graphic) = &mut item.content {
                        graphic.size = (w as f64, h as f64);
                    }
                }
                session = Session::new();
                session.add_document(DocState::new(doc, None));
                session.execute("selection.set", &json!({"ids":[item]}))?;
                session.execute(
                    "object.fit",
                    &json!({"ids":[item],"mode":"fillProportionally"}),
                )?;
            }
            board.source = session
                .active()
                .ok_or(BoardsError::NoDocument)?
                .doc
                .as_ref()
                .clone();
            placement.path = path.to_string_lossy().into_owned();
            placement.sha256 = format!("{:x}", Sha256::digest(&bytes));
            placement.pixel_width = w;
            placement.pixel_height = h;
            if board.info.first_image.as_deref() == Some(asset) {
                board.info.pixel_size = Some([w, h]);
                if board.info.auto_from_first {
                    board.title = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                }
            }
            Ok(())
        })
    }

    pub(crate) fn edit_board(
        &self,
        id: &str,
        edit: impl FnOnce(&mut WorkCrateBoard) -> Result<(), BoardsError>,
    ) -> Result<WorkCrateBoard, BoardsError> {
        let mut boards = self.boards.write().map_err(|_| BoardsError::Poisoned)?;
        let mut next = boards
            .get(id)
            .cloned()
            .ok_or_else(|| BoardsError::BoardNotFound(id.into()))?;
        edit(&mut next)?;
        write_board(&self.projects.project_directory(&next.project_id)?, &next)?;
        boards.insert(id.into(), next.clone());
        Ok(next)
    }

    pub fn project_name(&self, board: &WorkCrateBoard) -> Result<String, BoardsError> {
        Ok(board.info.project_name_override.clone().unwrap_or(
            self.projects
                .projects(true)?
                .into_iter()
                .find(|p| p.id == board.project_id)
                .ok_or(BoardsError::InvalidBoard)?
                .name,
        ))
    }

    pub fn duplicate_project_pages(&self, original: &str, target: &str) -> Result<(), BoardsError> {
        let source_root = self.projects.project_directory(original)?;
        let target_root = self.projects.project_directory(target)?;
        for mut board in self.board_catalog(original)? {
            board.id = Uuid::new_v4().to_string();
            board.project_id = target.to_owned();
            for placement in board.placements.values_mut() {
                if let Ok(relative) = Path::new(&placement.path).strip_prefix(&source_root) {
                    placement.path = target_root.join(relative).to_string_lossy().into_owned();
                }
            }
            write_board(&target_root, &board)?;
        }
        Ok(())
    }

    /// The exported information band is real DesignCraft text, not a UI overlay.
    pub(super) fn presentation_document(
        &self,
        board: &WorkCrateBoard,
    ) -> Result<Document, BoardsError> {
        let mut source = board.source.clone();
        source.settings.glyph_fallback = true;
        let width = source.settings.page_width;
        let height = source.settings.page_height * 0.06;
        let left = format!(
            "{:02}  {}",
            board.info.order.max(1),
            self.project_name(board)?
        );
        let mut right = format!(
            "{}  {}  {}",
            board.info.material, board.info.size, board.info.quantity
        );
        for (key, value) in &board.info.extra {
            right.push_str(&format!("  {key}: {value}"));
        }
        let mut session = Session::new();
        session.add_document(DocState::new(source, None));
        // This is printable document content, independent of the application's theme.
        let swatch = session.execute(
            "swatch.create",
            &json!({"name":"WorkCrate information band", "color":{"model":"gray","k":0.06}}),
        )?;
        let band = session.execute("frame.create", &json!({"rect":[0.,0.,width,height]}))?;
        session.execute(
            "object.fill",
            &json!({"ids":[band["id"]],"swatch":swatch["name"]}),
        )?;
        for (text, x0, x1) in [
            (left, width * 0.01, width * 0.54),
            (right, width * 0.55, width * 0.99),
        ] {
            let result = session.execute(
                "frame.create",
                &json!({"rect":[x0,0.0,x1,height],"content":"text","text":text,"caret":true}),
            )?;
            session.execute(
                "text.select",
                &json!({"story":result["story"],"anchor":0,"focus":text.len()}),
            )?;
            session.execute(
                "type.char",
                &json!({"attrs":{"size":(height*0.44).clamp(0.5,36.0)}}),
            )?;
        }
        Ok(session
            .active()
            .ok_or(BoardsError::NoDocument)?
            .doc
            .as_ref()
            .clone())
    }
}
