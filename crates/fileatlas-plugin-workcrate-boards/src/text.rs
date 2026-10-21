//! Native DesignCraft text frames; editor sprites and export share the same document.
use super::*;
use designcraft_geom::{Affine, Rect, shapes};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoardText {
    pub item_id: u64,
    pub story_id: u64,
    pub text: String,
    pub size_pt: f64,
    pub color: String,
}

impl WorkCrateBoard {
    pub fn text_frame(&self, id: &str) -> Result<[f64; 4], BoardsError> {
        let record = self.texts.get(id).ok_or(BoardsError::InvalidBoard)?;
        let r = self
            .source
            .item(designcraft_doc::ItemId(record.item_id))
            .ok_or(BoardsError::InvalidBoard)?
            .bounds();
        Ok([r.x0, r.y0, r.x1, r.y1])
    }
}

impl WorkCrateBoardsService {
    pub fn text_fonts(&self, id: &str) -> Result<serde_json::Value, BoardsError> {
        let board = self.board(id)?;
        let mut session = Session::new();
        session.add_document(DocState::new(board.source, None));
        Ok(session.execute("font.list", &json!({}))?)
    }
    pub fn add_text(&self, id: &str) -> Result<WorkCrateBoard, BoardsError> {
        self.edit_board(id, |board| {
            let offset = (board.texts.len() % 6) as f64 * 24.;
            let mut session = Session::new();
            session.add_document(DocState::new(board.source.clone(), None));
            let frame = session.execute("frame.create", &json!({"rect":[72.+offset,110.+offset,432.+offset,254.+offset],"content":"text","text":"新建文本\n请输入内容","caret":true}))?;
            let item = frame["id"].as_u64().ok_or(BoardsError::InvalidBoard)?;
            let story = frame["story"].as_u64().ok_or(BoardsError::InvalidBoard)?;
            let record = BoardText { item_id:item, story_id:story, text:"新建文本\n请输入内容".into(), size_pt:24., color:"#000000".into() };
            format_text(&mut session, &record)?;
            board.source = session.active().ok_or(BoardsError::NoDocument)?.doc.as_ref().clone();
            board.source.settings.glyph_fallback = true;
            board.texts.insert(format!("{item:020}"), record);
            Ok(())
        })
    }

    pub fn update_text(
        &self,
        id: &str,
        text_id: &str,
        text: &str,
        size: f64,
        color: &str,
        frame: [f64; 4],
    ) -> Result<WorkCrateBoard, BoardsError> {
        validate_frame(frame)?;
        if !size.is_finite()
            || !(0.1..=1296.).contains(&size)
            || text.len() > 100_000
            || color.len() != 7
            || !color.starts_with('#')
            || !color[1..].bytes().all(|c| c.is_ascii_hexdigit())
        {
            return Err(BoardsError::InvalidPlacement);
        }
        self.edit_board(id, |board| {
            let record = board
                .texts
                .get_mut(text_id)
                .ok_or(BoardsError::InvalidBoard)?;
            let mut session = Session::new();
            session.add_document(DocState::new(board.source.clone(), None));
            if record.text != text {
                let (start, end, inserted) = minimal_edit(&record.text, text);
                session.execute("text.select", &json!({"story":record.story_id,"anchor":start,"focus":end}))?;
                session.execute("text.insert", &json!({"text":inserted,"raw":true}))?;
            }
            let mut attrs = serde_json::Map::new();
            if record.size_pt != size { attrs.insert("size".into(), json!(size)); }
            if !record.color.eq_ignore_ascii_case(color) {
                let swatch = session.execute("swatch.create", &json!({"name":format!("WorkCrate text {} {color}",record.item_id),"color":color}))?;
                attrs.insert("fill".into(), swatch["name"].clone());
            }
            if !attrs.is_empty() {
                session.execute("text.select", &json!({"story":record.story_id,"anchor":0,"focus":text.len()}))?;
                session.execute("type.char", &json!({"attrs":attrs}))?;
            }
            record.text = text.into();
            record.size_pt = size;
            record.color = color.to_lowercase();
            let mut doc = session
                .active()
                .ok_or(BoardsError::NoDocument)?
                .doc
                .as_ref()
                .clone();
            let item = doc
                .item_mut(designcraft_doc::ItemId(record.item_id))
                .ok_or(BoardsError::InvalidBoard)?;
            // A text box resizes its frame, never its character transforms.
            item.xf = Affine::IDENTITY;
            item.path = shapes::rectangle(Rect::new(frame[0], frame[1], frame[2], frame[3]));
            board.source = doc;
            Ok(())
        })
    }

    pub fn move_text_frame(
        &self,
        id: &str,
        text_id: &str,
        frame: [f64; 4],
    ) -> Result<WorkCrateBoard, BoardsError> {
        validate_frame(frame)?;
        self.edit_board(id, |board| {
            let text = board.texts.get(text_id).ok_or(BoardsError::InvalidBoard)?;
            let item = board
                .source
                .item_mut(designcraft_doc::ItemId(text.item_id))
                .ok_or(BoardsError::InvalidBoard)?;
            item.xf = Affine::IDENTITY;
            item.path = shapes::rectangle(Rect::new(frame[0], frame[1], frame[2], frame[3]));
            Ok(())
        })
    }

    pub fn delete_text(&self, id: &str, text_id: &str) -> Result<WorkCrateBoard, BoardsError> {
        self.edit_board(id, |board| {
            let text = board
                .texts
                .remove(text_id)
                .ok_or(BoardsError::InvalidBoard)?;
            let mut session = Session::new();
            session.add_document(DocState::new(board.source.clone(), None));
            session.execute("selection.set", &json!({"ids":[text.item_id]}))?;
            session.execute("edit.clear", &json!({}))?;
            board.source = session
                .active()
                .ok_or(BoardsError::NoDocument)?
                .doc
                .as_ref()
                .clone();
            Ok(())
        })
    }

    pub fn render_text(
        &self,
        id: &str,
        text_id: &str,
        scale: f64,
    ) -> Result<designcraft_render::Rendered, BoardsError> {
        if !scale.is_finite() || scale <= 0. {
            return Err(BoardsError::InvalidPlacement);
        }
        let board = self.board(id)?;
        let f = board.text_frame(text_id)?;
        let item = board
            .texts
            .get(text_id)
            .ok_or(BoardsError::InvalidBoard)?
            .item_id;
        let mut doc = board.source;
        for spread in &mut doc.spreads {
            Arc::make_mut(spread).items.retain(|i| i.id.0 == item);
        }
        let w = ((f[2] - f[0]) * scale).round().max(1.) as u32;
        let h = ((f[3] - f[1]) * scale).round().max(1.) as u32;
        if w > designcraft_render::MAX_SIDE
            || h > designcraft_render::MAX_SIDE
            || w as u64 * h as u64 > designcraft_render::MAX_PAGE_PIXELS
        {
            return Err(BoardsError::InvalidDimensions);
        }
        Ok(designcraft_render::Renderer::new().render(
            &doc,
            &Cache::new(),
            &[designcraft_render::Placed {
                spread: designcraft_doc::SpreadRef::Doc(0),
                xf: Affine::IDENTITY,
            }],
            w,
            h,
            Affine::scale(scale) * Affine::translate((-f[0], -f[1])),
            &designcraft_render::RenderOptions {
                paper: false,
                background: None,
                ..Default::default()
            },
        ))
    }

    pub fn update_content_field(
        &self,
        id: &str,
        key: &str,
        value: &str,
    ) -> Result<WorkCrateBoard, BoardsError> {
        self.edit_board(id, |board| {
            match key {
                "local" => {
                    board.info.project_name_override = (!value.is_empty()).then(|| value.into())
                }
                "material" => board.info.material = value.into(),
                "size" => board.info.size = value.into(),
                "quantity" => board.info.quantity = value.into(),
                _ if key.starts_with("extra:") && key.len() > 6 => {
                    board.info.extra.insert(key[6..].into(), value.into());
                }
                _ => return Err(BoardsError::InvalidBoard),
            }
            Ok(())
        })
    }
}

fn format_text(session: &mut Session, text: &BoardText) -> Result<(), BoardsError> {
    let swatch = session.execute(
        "swatch.create",
        &json!({"name":format!("WorkCrate text {} {}",text.item_id,text.color),"color":text.color}),
    )?;
    session.execute(
        "text.select",
        &json!({"story":text.story_id,"anchor":0,"focus":text.text.len()}),
    )?;
    session.execute("type.char",&json!({"attrs":{"fontFamily":"Microsoft YaHei","fontStyle":"Regular","size":text.size_pt,"fill":swatch["name"]}}))?;
    Ok(())
}

fn validate_frame(f: [f64; 4]) -> Result<(), BoardsError> {
    if !f.iter().all(|v| v.is_finite() && v.abs() <= 100_000.)
        || f[2] - f[0] < 4.
        || f[3] - f[1] < 4.
    {
        return Err(BoardsError::InvalidPlacement);
    }
    Ok(())
}

// Replace only the changed UTF-8 interval so native per-character formats survive typing.
fn minimal_edit<'a>(old: &str, new: &'a str) -> (usize, usize, &'a str) {
    let start = old
        .chars()
        .zip(new.chars())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum::<usize>();
    let suffix = old[start..]
        .chars()
        .rev()
        .zip(new[start..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum::<usize>();
    (start, old.len() - suffix, &new[start..new.len() - suffix])
}
