use super::*;
impl WorkCrateBoardsService {
    pub fn create_page(&self, project: &str) -> Result<WorkCrateBoard, BoardsError> {
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let mut c = self.client()?;
        let w = 420. * 72. / 25.4;
        let h = 297. * 72. / 25.4;
        if let Some(path) = self.document_path(project)? {
            native::open_safely(&mut c, &path)?;
            native::assert_path(&mut c, &path)?;
            c.execute(
                "artboard.new",
                json!({"width":w,"height":h,"name":"未命名页面"}),
            )?;
        } else {
            let directory = self.projects.project_directory(project)?;
            let inspect = native::inspect_optional(&mut c)?;
            if inspect.as_ref().is_some_and(|i| i["dirty"] == true) {
                return Err(BoardsError::DirtyDocument);
            }
            c.execute(
                "file.new",
                json!({"name":"WorkCrate","width":w,"height":h,"units":"Points","artboards":1,"backgroundContents":"white"}),
            )?;
            let path = directory.join("document.vectorcraft");
            c.request("app.save", json!({"path":path}))?;
            wait_output(&mut c, &path)?;
            self.projects.set_document_path(project, &path)?;
        }
        native::assert_path(
            &mut c,
            &self
                .document_path(project)?
                .ok_or(BoardsError::NoDocument)?,
        )?;
        let board = native::catalog(&mut c, project, &self.projects.document_binding(project)?)?
            .pop()
            .ok_or(BoardsError::NoDocument)?;
        self.write_info(
            &mut c,
            &board,
            &board.title,
            PageInfo {
                order: board.index as u32 + 1,
                ..Default::default()
            },
        )?;
        let path = self
            .document_path(project)?
            .ok_or(BoardsError::NoDocument)?;
        c.request("app.save", json!({"path":path}))?;
        wait_output(&mut c, &path)?;
        Ok(board)
    }
    pub fn update_info(&self, id: &str, title: &str, info: PageInfo) -> Result<(), BoardsError> {
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let mut c = self.client()?;
        let b = self.target(&mut c, id)?;
        self.write_info(&mut c, &b, title, info)?;
        let path = self
            .document_path(&b.project_id)?
            .ok_or(BoardsError::NoDocument)?;
        native::assert_path(&mut c, &path)?;
        c.request("app.save", json!({"path":path}))?;
        wait_output(&mut c, &path)
    }
    fn write_info(
        &self,
        c: &mut Client,
        b: &WorkCrateBoard,
        title: &str,
        info: PageInfo,
    ) -> Result<(), BoardsError> {
        if title.is_empty()
            || title.chars().count() > 512
            || serde_json::to_vec(&info)?.len() > 16384
            || info.extra.len() > 24
        {
            return Err(BoardsError::Invalid("header exceeds bounds".into()));
        }
        let fields = native::header_fields(c)?
            .remove(&b.native_id)
            .unwrap_or_default();
        let project_name = info.project_name_override.clone().unwrap_or(
            self.projects
                .projects(true)?
                .into_iter()
                .find(|p| p.id == b.project_id)
                .ok_or_else(|| BoardsError::Invalid("project missing".into()))?
                .name,
        );
        let mut values = BTreeMap::from([
            ("project_name".to_owned(), project_name),
            ("material".to_owned(), info.material),
            ("size".to_owned(), info.size),
            ("quantity".to_owned(), info.quantity),
            ("size_label".to_owned(), info.size_label),
        ]);
        for (key, value) in info.extra {
            values.insert(format!("extra:{key}"), value);
        }
        let field_order = values.keys().cloned().collect::<Vec<_>>();
        for role in &field_order {
            let label = match role.as_str() {
                "project_name" => "项目",
                "material" => "物料",
                "size" => "尺寸",
                "quantity" => "数量",
                "size_label" => "规格",
                other => other.strip_prefix("extra:").unwrap_or(other),
            };
            values.insert(format!("label:{role}"), label.to_owned());
        }
        let inspect = c.request("document.inspect", json!({"depth":0}))?;
        if !inspect["isolation"].is_null() {
            return Err(BoardsError::Invalid(
                "leave native isolation mode before editing the header".into(),
            ));
        }
        let rect = &inspect["artboards"][b.index];
        let x = rect["x"].as_f64().unwrap_or(0.);
        let y = rect["y"].as_f64().unwrap_or(0.);
        if b.width_pt < 200. || b.height_pt < 100. {
            return Err(BoardsError::Invalid("board too small for header".into()));
        }
        c.execute(
            "edit.undoGroup.begin",
            json!({"label":"WorkCrate material header"}),
        )?;
        let result = (|| {
            c.execute("artboard.setProps", json!({"index":b.index,"name":title}))?;
            let mut layer = None;
            let mut created_ids = Vec::new();
            for (role, (id, _)) in &fields {
                if !values.contains_key(role) {
                    c.execute("edit.clear", json!({"ids":[id]}))?;
                }
            }
            if fields.is_empty() {
                let new_layer = c.execute(
                    "layer.new",
                    json!({"name":"WorkCrate header","top":true,"locked":false,"visible":true}),
                )?;
                let layer_id = new_layer["id"]
                    .as_u64()
                    .ok_or_else(|| BoardsError::Protocol("layer ID missing".into()))?;
                c.execute("layer.setCurrent", json!({"id":layer_id}))?;
                layer = Some(layer_id);
                let background=c.execute("shape.rectangle",json!({"x":x+6.,"y":y+6.,"width":b.width_pt-12.,"height":(b.height_pt-12.).min(110.)}))?;
                let id = background["id"]
                    .as_u64()
                    .ok_or_else(|| BoardsError::Protocol("background ID missing".into()))?;
                c.execute("paint.setFill", json!({"id":id,"color":"#eef2f6"}))?;
                c.execute("paint.setStroke", json!({"id":id,"none":true}))?;
                created_ids.push(id);
            }

            for (role, value) in &values {
                let base_role = role.strip_prefix("label:").unwrap_or(role);
                let i = field_order
                    .iter()
                    .position(|r| r == base_role)
                    .ok_or_else(|| BoardsError::Invalid("unknown header role".into()))?;
                let label = role.starts_with("label:");
                let text = value.clone();
                if text.chars().count() > 1024 {
                    return Err(BoardsError::Invalid("field too long".into()));
                }
                let row = i % 8;
                let column = i / 8;
                let target_x = x
                    + 12.
                    + column as f64 * (b.width_pt / 4.)
                    + if label { 0. } else { b.width_pt / 10. };
                let target_y = y + 18. + row as f64 * ((b.height_pt - 24.).min(96.) / 8.);
                if let Some((id, _)) = fields.get(role) {
                    let node = c.execute("document.node", json!({"id":id}))?;
                    let transform = &node["kind"]["xf"];
                    let current_x = transform[4].as_f64().ok_or_else(|| {
                        BoardsError::Protocol("native text origin missing".into())
                    })?;
                    let current_y = transform[5].as_f64().ok_or_else(|| {
                        BoardsError::Protocol("native text origin missing".into())
                    })?;
                    c.execute("object.transform",json!({"ids":[id],"matrix":[1,0,0,1,target_x-current_x,target_y-current_y]}))?;
                    c.execute("text.setText", json!({"id":id,"text":text}))?;
                } else {
                    if layer.is_none() {
                        let l=c.execute("layer.new",json!({"name":"WorkCrate header","top":true,"locked":false,"visible":true}))?;
                        let id = l["id"]
                            .as_u64()
                            .ok_or_else(|| BoardsError::Protocol("layer ID absent".into()))?;
                        c.execute("layer.setCurrent", json!({"id":id}))?;
                        layer = Some(id);
                    }
                    let created=c.execute("text.create",json!({"x":target_x,"y":target_y,"text":text,"size":8,"font":"Microsoft YaHei","area":{"width":if label{b.width_pt/10.-4.}else{b.width_pt*0.15-16.},"height":10.,"fit":"shrinkText","fitMinPercent":30},"color":"#18212f"}))?;
                    let id = created["id"]
                        .as_u64()
                        .ok_or_else(|| BoardsError::Protocol("text ID absent".into()))?;
                    let note=json!({"namespace":"fileatlas.workcrate","version":1,"board":b.native_id,"role":role}).to_string();
                    c.execute("attributes.set", json!({"id":id,"note":note}))?;
                    created_ids.push(id);
                }
            }
            if fields.is_empty() && !created_ids.is_empty() {
                let grouped = c.execute("object.group", json!({"ids":created_ids}))?;
                let id = grouped["id"]
                    .as_u64()
                    .ok_or_else(|| BoardsError::Protocol("header group ID missing".into()))?;
                c.execute(
                    "layer.setProps",
                    json!({"id":id,"name":"WorkCrate material header"}),
                )?;
                c.execute("attributes.set",json!({"id":id,"note":json!({"namespace":"fileatlas.workcrate","version":1,"board":b.native_id,"role":"header"}).to_string()}))?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                c.execute("edit.undoGroup.end", json!({}))?;
                Ok(())
            }
            Err(e) => {
                let _ = c.execute("edit.undoGroup.end", json!({"cancel":true}));
                Err(e)
            }
        }
    }
    pub fn save_project(&self, project: &str) -> Result<(), BoardsError> {
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let path = self
            .document_path(project)?
            .ok_or(BoardsError::NoDocument)?;
        let mut c = self.client()?;
        native::assert_path(&mut c, &path)?;
        c.request("app.save", json!({"path":path}))?;
        wait_output(&mut c, &path)
    }
    pub fn open_editor(&self, id: &str) -> Result<(), BoardsError> {
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let mut c = self.client()?;
        let b = self.target(&mut c, id)?;
        c.execute("view.goToArtboard", json!({"index":b.index}))?;
        c.request("ui.focus", json!({}))?;
        Ok(())
    }
    pub fn render_preview(&self, id: &str, max_width: u32) -> Result<PathBuf, BoardsError> {
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let mut c = self.client()?;
        let b = self.target(&mut c, id)?;
        let inspect = c.request("document.inspect", json!({"depth":0}))?;
        let rev = inspect["revision"]
            .as_u64()
            .ok_or_else(|| BoardsError::Protocol("revision missing".into()))?;
        let pixels = max_width.clamp(64, 2048);
        let scale = (pixels as f64 / b.width_pt).min(2048. / b.height_pt);
        let dir = self
            .projects
            .project_directory(&b.project_id)?
            .join("native-previews");
        std::fs::create_dir_all(&dir)?;
        use std::hash::{Hash, Hasher};
        let source = self
            .document_path(&b.project_id)?
            .ok_or(BoardsError::NoDocument)?;
        let metadata = std::fs::metadata(&source)?;
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hash);
        metadata.len().hash(&mut hash);
        metadata.modified()?.hash(&mut hash);
        let path = dir.join(format!(
            "{}-{}-{rev}-{pixels}.png",
            b.native_id,
            hash.finish()
        ));
        if !path.is_file() {
            c.request(
                "ui.render",
                json!({"artboard":b.index,"scale":scale,"path":path}),
            )?;
            wait_output(&mut c, &path)?;
        }
        Ok(path)
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
        let mut c = self.client()?;
        let b = self.target(&mut c, id)?;
        let i = c.request("document.inspect", json!({"depth":0}))?;
        let r = &i["artboards"][b.index];
        let x = r["x"].as_f64().unwrap_or(0.);
        let y = r["y"].as_f64().unwrap_or(0.);
        let (px, py, w) = placement.unwrap_or((24., 128., b.width_pt - 48.));
        let result = c.execute(
            "file.place",
            json!({"path":path,"rect":[x+px,y+py,w,(b.height_pt-py-24.).max(1.)]}),
        )?;
        if result.is_null() {
            return Err(BoardsError::Invalid("placement pending dialog".into()));
        }
        let path = self
            .document_path(&b.project_id)?
            .ok_or(BoardsError::NoDocument)?;
        native::assert_path(&mut c, &path)?;
        c.request("app.save", json!({"path":path}))?;
        wait_output(&mut c, &path)
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
        let mut c = self.client()?;
        let b = self.target(&mut c, first)?;
        let mut indices = Vec::new();
        for id in ids {
            let target = self.target(&mut c, id)?;
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
            json!({"path":official_path,"format":"pdf","useArtboards":true,"range":indices.iter().map(usize::to_string).collect::<Vec<_>>().join(",")})
        } else {
            if ids.len() != 1 {
                return Err(BoardsError::Invalid("one raster page required".into()));
            }
            json!({"path":official_path,"format":format,"artboard":b.index,"scale":300./72.})
        };
        let result = c.request("app.export", params)?;
        wait_output(&mut c, official_path)?;
        temporary.persist(path).map_err(|e| e.error)?;
        Ok(result)
    }
}
pub(crate) fn wait_output(c: &mut Client, path: &Path) -> Result<(), BoardsError> {
    let start = std::time::Instant::now();
    loop {
        let ui = c.request("ui.inspect", json!({}))?;
        let busy = ui.get("background").is_some_and(|v| {
            !v.is_null() && v != &json!([]) && v != &json!({}) && v != &json!(false)
        });
        if !busy {
            let status = ui["ui"]["status"]
                .as_str()
                .or_else(|| ui["status"].as_str())
                .unwrap_or("");
            let lower = status.to_lowercase();
            if lower.contains("failed") || lower.contains("error") {
                return Err(BoardsError::Remote(status.into()));
            }
            if path
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("vectorcraft"))
            {
                let state = native::assert_path(c, path)?;
                if state["dirty"] == true {
                    return Err(BoardsError::Invalid(
                        "official save did not settle document changes".into(),
                    ));
                }
            }

            if std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() > 0) {
                return Ok(());
            }
            return Err(BoardsError::Invalid(format!(
                "official operation produced no output: {}",
                path.display()
            )));
        }
        if start.elapsed().as_secs() > 65 {
            return Err(BoardsError::Protocol("background outcome uncertain".into()));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
