use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageLayout {
    pub frame: [f64; 4],
    pub content: [f64; 4],
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_a3_labels_uniform_scale_crop_restore_and_project_refresh()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let projects = Arc::new(WorkCrateProjectsService::open_at(temp.path().join("data"))?);
        let project = projects.create_project("Linked layouts")?;
        let source = temp.path().join("linked.png");
        image::RgbImage::new(640, 480).save(&source)?;
        let source_bytes = fs::read(&source)?;
        let reference = projects
            .add_references(&project.id, std::slice::from_ref(&source))?
            .remove(0);
        let service = WorkCrateBoardsService::new(Arc::clone(&projects));
        let page = service.create_page(&project.id)?;
        let page = service.place_reference(&page.id, &reference.id, 24., 80., 420., false)?;
        let asset = page.placements.keys().next().ok_or("no image")?.clone();
        let initial = page.image_layout(&asset)?;
        let mut info = page.info.clone();
        info.size_label = "210cm × 100cm（制作尺寸）".into();
        info.physical_size_cm = Some([210., 100.]);
        let page = service.update_info(&page.id, &page.title, info)?;
        assert!((page.source.settings.page_width - 420. * 72. / 25.4).abs() < 0.01);
        assert!((page.source.settings.page_height - 297. * 72. / 25.4).abs() < 0.01);
        let f = initial.frame;
        assert!(
            service
                .set_image_frame(
                    &page.id,
                    &asset,
                    [f[0], f[1], f[0] + 100., f[1] + 200.],
                    false
                )
                .is_err()
        );
        let scaled = [
            f[0],
            f[1],
            f[0] + (f[2] - f[0]) * 0.6,
            f[1] + (f[3] - f[1]) * 0.6,
        ];
        let page = service.set_image_frame(&page.id, &asset, scaled, false)?;
        let full = page.image_layout(&asset)?;
        let cropped = [
            scaled[0] + 20.,
            scaled[1] + 15.,
            scaled[2] - 10.,
            scaled[3] - 10.,
        ];
        let page = service.set_image_frame(&page.id, &asset, cropped, true)?;
        assert_eq!(page.image_layout(&asset)?.content, full.content);
        assert_eq!(fs::read(&source)?, source_bytes);
        let restored = service
            .restore_image_crop(&page.id, &asset)?
            .image_layout(&asset)?;
        for (actual, expected) in restored.frame.into_iter().zip(full.content) {
            assert!((actual - expected).abs() < 0.001);
        }
        let psd = temp.path().join("source.psd");
        fs::write(&psd, b"linked PSD fixture")?;
        service.link_source(&page.id, &asset, Some(psd.to_string_lossy().into_owned()))?;
        let second = service.create_page(&project.id)?;
        service.place_reference(&second.id, &reference.id, 24., 80., 420., false)?;
        image::RgbImage::new(900, 300).save(&source)?;
        let result = service.refresh_project_images(&project.id)?;
        assert_eq!(result.updated, 2);
        assert!(result.failures.is_empty());
        let page = service.load_board(&project.id, &page.id)?;
        assert_eq!(
            page.placements[&asset].source_document.as_deref(),
            psd.to_str()
        );
        let layout = page.image_layout(&asset)?;
        assert!(
            ((layout.content[2] - layout.content[0]) / (layout.content[3] - layout.content[1])
                - 3.)
                .abs()
                < 1e-6
        );
        assert_eq!(service.refresh_project_images(&project.id)?.unchanged, 2);
        fs::remove_file(&source)?;
        assert_eq!(
            service.refresh_project_images(&project.id)?.failures.len(),
            2
        );
        assert_eq!(fs::read(&psd)?, b"linked PSD fixture");
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RefreshProjectImagesReport {
    pub updated: usize,
    pub unchanged: usize,
    pub failures: Vec<String>,
}

impl WorkCrateBoard {
    pub fn image_layout(&self, asset: &str) -> Result<ImageLayout, BoardsError> {
        let placement = self
            .placements
            .get(asset)
            .ok_or(BoardsError::PlacedAssetMissing)?;
        let item = self
            .source
            .spreads
            .iter()
            .flat_map(|s| s.items.iter())
            .find(|item| Some(item.id.0) == placement.item_id)
            .ok_or(BoardsError::PlacedAssetMissing)?;
        let graphic = item.graphic().ok_or(BoardsError::PlacedAssetMissing)?;
        let frame = item.bounds();
        let content = (item.xf * graphic.xf).transform_rect_bbox(designcraft_geom::Rect::new(
            0.,
            0.,
            graphic.size.0,
            graphic.size.1,
        ));
        Ok(ImageLayout {
            frame: [frame.x0, frame.y0, frame.x1, frame.y1],
            content: [content.x0, content.y0, content.x1, content.y1],
        })
    }
}

impl WorkCrateBoardsService {
    /// Static A3 paper and information header for the editor. Images are separate
    /// cached GPU layers while dragging; normal page rendering still owns exports.
    pub fn render_template(
        &self,
        id: &str,
        scale: f64,
    ) -> Result<designcraft_render::Rendered, BoardsError> {
        let board = self.board(id)?;
        let mut document = self.presentation_document(&board)?;
        for spread in &mut document.spreads {
            Arc::make_mut(spread).items.retain(|item| {
                !board.texts.values().any(|t| t.item_id == item.id.0)
                    && !board
                        .placements
                        .values()
                        .any(|p| p.item_id == Some(item.id.0))
            });
        }
        designcraft_render::Renderer::new()
            .render_page(
                &document,
                &Cache::new(),
                0,
                scale,
                false,
                &designcraft_render::RenderOptions::default(),
            )
            .ok_or(BoardsError::RenderFailed)
    }

    /// Resize both frame and content uniformly, or resize just the clipping frame.
    /// Source image bytes are never cropped or rewritten.
    pub fn set_image_frame(
        &self,
        id: &str,
        asset: &str,
        to: [f64; 4],
        crop: bool,
    ) -> Result<WorkCrateBoard, BoardsError> {
        if !to.iter().all(|n| n.is_finite()) || to[2] - to[0] < 1. || to[3] - to[1] < 1. {
            return Err(BoardsError::InvalidPlacement);
        }
        self.edit_board(id, |board| {
            let before = board.image_layout(asset)?.frame;
            let sx = (to[2] - to[0]) / (before[2] - before[0]);
            let sy = (to[3] - to[1]) / (before[3] - before[1]);
            if !crop && (sx - sy).abs() > 1e-5 {
                return Err(BoardsError::InvalidPlacement);
            }
            let placement = board
                .placements
                .get_mut(asset)
                .ok_or(BoardsError::PlacedAssetMissing)?;
            let item = placement.item_id.ok_or(BoardsError::PlacedAssetMissing)?;
            let mut session = Session::new();
            session.add_document(DocState::new(board.source.clone(), None));
            session.execute("selection.set", &json!({"ids":[item]}))?;
            session.execute(
                "transform.resize",
                &json!({"ids":[item],"from":before,"to":to,"content":!crop}),
            )?;
            board.source = session
                .active()
                .ok_or(BoardsError::NoDocument)?
                .doc
                .as_ref()
                .clone();
            if !crop {
                placement.placed_width_pt *= sx;
            }
            Ok(())
        })
    }

    pub fn restore_image_crop(&self, id: &str, asset: &str) -> Result<WorkCrateBoard, BoardsError> {
        self.edit_board(id, |board| {
            let item = board
                .placements
                .get(asset)
                .and_then(|p| p.item_id)
                .ok_or(BoardsError::PlacedAssetMissing)?;
            let mut session = Session::new();
            session.add_document(DocState::new(board.source.clone(), None));
            session.execute("selection.set", &json!({"ids":[item]}))?;
            session.execute(
                "object.fit",
                &json!({"ids":[item],"mode":"fitFrameToContent"}),
            )?;
            board.source = session
                .active()
                .ok_or(BoardsError::NoDocument)?
                .doc
                .as_ref()
                .clone();
            Ok(())
        })
    }

    pub fn refresh_project_images(
        &self,
        project: &str,
    ) -> Result<RefreshProjectImagesReport, BoardsError> {
        let mut report = RefreshProjectImagesReport::default();
        for board in self.board_catalog(project)? {
            // Registers editable metadata even when a source changed since the last save.
            let _ = self.load_board(project, &board.id);
            for (asset, placement) in board.placements {
                match fs::read(&placement.path) {
                    Ok(bytes) if format!("{:x}", Sha256::digest(&bytes)) == placement.sha256 => {
                        report.unchanged += 1
                    }
                    Ok(_) => match self.refresh_image(&board.id, &asset, None) {
                        Ok(_) => report.updated += 1,
                        Err(error) => report.failures.push(format!("{}: {error}", placement.path)),
                    },
                    Err(error) => report.failures.push(format!("{}: {error}", placement.path)),
                }
            }
        }
        Ok(report)
    }
}
