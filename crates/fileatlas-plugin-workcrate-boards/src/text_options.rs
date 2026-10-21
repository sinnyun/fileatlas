//! Full native typography attribute catalogue; no duplicated layout engine.
use super::*;
use designcraft_doc::{CharProps, ParaProps, TextFrameOptions};
use serde_json::Value;

impl WorkCrateBoard {
    /// Pure in-memory resolved appearance, including styles set outside the quick inspector.
    pub fn text_appearance(&self, id: &str, byte: usize) -> Value {
        let Some(t) = self.texts.get(id) else {
            return Value::Null;
        };
        let Some(story) = self.source.story(designcraft_doc::StoryId(t.story_id)) else {
            return Value::Null;
        };
        let pos = byte.min(story.text.len());
        let (para, base) = self
            .source
            .styles
            .resolve_para(&story.paras[story.para_at(pos)]);
        let chars = self
            .source
            .styles
            .resolve_char(&base, story.char_format_at(pos));
        json!({"chars":chars,"para":para})
    }
}

fn select(
    session: &mut Session,
    board: &WorkCrateBoard,
    id: &str,
    range: Option<[usize; 2]>,
) -> Result<(), BoardsError> {
    let t = board.texts.get(id).ok_or(BoardsError::InvalidBoard)?;
    let r = range.filter(|r| r[0] != r[1]).unwrap_or([0, t.text.len()]);
    if r[0] > r[1]
        || r[1] > t.text.len()
        || !t.text.is_char_boundary(r[0])
        || !t.text.is_char_boundary(r[1])
    {
        return Err(BoardsError::InvalidPlacement);
    }
    session.execute("selection.set", &json!({"ids":[t.item_id]}))?;
    session.execute(
        "text.select",
        &json!({"story":t.story_id,"anchor":r[0],"focus":r[1]}),
    )?;
    Ok(())
}

impl WorkCrateBoardsService {
    pub fn text_options(
        &self,
        board: &str,
        id: &str,
        range: Option<[usize; 2]>,
    ) -> Result<Value, BoardsError> {
        let board = self.board(board)?;
        let t = board.texts.get(id).ok_or(BoardsError::InvalidBoard)?;
        let mut session = Session::new();
        session.add_document(DocState::new(board.source.clone(), None));
        select(&mut session, &board, id, range)?;
        let attrs = session.execute("type.selectionAttrs", &json!({"mixed":true}))?;
        let ot = session.execute("type.openTypeFeatures", &json!({}))?;
        let frame = board
            .source
            .item(designcraft_doc::ItemId(t.item_id))
            .and_then(|i| i.text_frame())
            .ok_or(BoardsError::InvalidBoard)?;
        Ok(
            json!({"chars":attrs["chars"],"para":attrs["para"],"frame":frame_value(&frame.options)?,"openType":ot,
            "defaults":{"chars":CharProps::default(),"para":ParaProps::default(),"frame":frame_value(&TextFrameOptions::default())?}}),
        )
    }

    pub fn set_text_options(
        &self,
        board: &str,
        id: &str,
        range: Option<[usize; 2]>,
        scope: &str,
        attrs: Value,
    ) -> Result<WorkCrateBoard, BoardsError> {
        let map = attrs.as_object().ok_or(BoardsError::InvalidPlacement)?;
        if map.is_empty() || serde_json::to_vec(&attrs)?.len() > 100_000 {
            return Err(BoardsError::InvalidPlacement);
        }
        if scope == "chars"
            && let Some(size) = attrs.get("size")
        {
            if !size
                .as_f64()
                .is_some_and(|v| v.is_finite() && (0.1..=1296.).contains(&v))
            {
                return Err(BoardsError::InvalidPlacement);
            }
        }
        self.edit_board(board, |board| {
            let record = board
                .texts
                .get(id)
                .cloned()
                .ok_or(BoardsError::InvalidBoard)?;
            let mut session = Session::new();
            session.add_document(DocState::new(board.source.clone(), None));
            select(&mut session, board, id, range)?;
            match scope {
                "chars" | "para" => {
                    let mut attrs = attrs.clone();
                    if scope == "chars" {
                        for (key, value) in
                            attrs.as_object_mut().ok_or(BoardsError::InvalidPlacement)?
                        {
                            if (key == "fill" || key == "stroke" || key.ends_with("Color"))
                                && value.as_str().is_some_and(|v| v.starts_with('#'))
                            {
                                let hex = value
                                    .as_str()
                                    .ok_or(BoardsError::InvalidPlacement)?
                                    .to_owned();
                                let swatch = session.execute(
                                    "swatch.create",
                                    &json!({"name":format!("WorkCrate {key} {hex}"),"color":hex}),
                                )?;
                                *value = swatch["name"].clone();
                            }
                        }
                    }
                    session.execute(
                        if scope == "chars" {
                            "type.char"
                        } else {
                            "type.para"
                        },
                        &json!({"attrs":attrs}),
                    )?;
                }
                "action" => {
                    let command = attrs["command"]
                        .as_str()
                        .ok_or(BoardsError::InvalidPlacement)?;
                    if !["type.bold", "type.italic"].contains(&command) {
                        return Err(BoardsError::InvalidPlacement);
                    }
                    session.execute(command, &json!({}))?;
                }
                "openType" => {
                    let mut features = attrs.clone();
                    if attrs["liga"].is_boolean() {
                        features["liga"] = json!(true);
                    }
                    session.execute("type.feature", &json!({"features":features}))?;
                    if let Some(on) = attrs["liga"].as_bool() {
                        session.execute("type.char", &json!({"attrs":{"ligatures":on}}))?;
                    }
                }
                "frame" => {
                    let current = board
                        .source
                        .item(designcraft_doc::ItemId(record.item_id))
                        .and_then(|i| i.text_frame())
                        .ok_or(BoardsError::InvalidBoard)?;
                    let mut value = frame_value(&current.options)?;
                    for (key, val) in map {
                        if value.get(key).is_none() {
                            return Err(BoardsError::InvalidPlacement);
                        }
                        value[key] = val.clone();
                    }
                    // Deserialize every option with the pinned native types; reject misspelled enums.
                    let options: TextFrameOptions = serde_json::from_value(value)?;
                    validate_options(&options)?;
                    let mut command = attrs.clone();
                    command["ids"] = json!([record.item_id]);
                    for (from, to) in [
                        ("autoSizeUseMinHeight", "useMinimumHeight"),
                        ("autoSizeMinHeight", "minimumHeight"),
                        ("autoSizeUseMinWidth", "useMinimumWidth"),
                        ("autoSizeMinWidth", "minimumWidth"),
                        ("autoSizeNoLineBreaks", "noLineBreaks"),
                    ] {
                        if let Some(v) = command.get(from).cloned() {
                            command[to] = v;
                        }
                    }
                    session.execute("object.textFrameOptions", &command)?;
                    // Includes native structured options (baselineGrid/path) not in the quick command.
                    let doc = session.active_mut().ok_or(BoardsError::NoDocument)?;
                    Arc::make_mut(&mut doc.doc)
                        .item_mut(designcraft_doc::ItemId(record.item_id))
                        .and_then(|i| i.text_frame_mut())
                        .ok_or(BoardsError::InvalidBoard)?
                        .options = options;
                }
                _ => return Err(BoardsError::InvalidPlacement),
            }
            board.source = session
                .active()
                .ok_or(BoardsError::NoDocument)?
                .doc
                .as_ref()
                .clone();
            // Keep quick controls consistent for a whole-frame point-size change.
            if range.is_none_or(|r| r[0] == r[1] || r == [0, record.text.len()]) && scope == "chars"
            {
                if let Some(v) = attrs["size"].as_f64().filter(|v| v.is_finite() && *v > 0.) {
                    board
                        .texts
                        .get_mut(id)
                        .ok_or(BoardsError::InvalidBoard)?
                        .size_pt = v;
                }
                if attrs.get("fill").is_some() || attrs.get("fillTint").is_some() {
                    let a = board.text_appearance(id, 0);
                    if let Some(color) = a["chars"]["fill"].as_str().and_then(|fill| {
                        board.source.resolve_color(
                            fill,
                            a["chars"]["fillTint"].as_f64().unwrap_or(1.) as f32,
                        )
                    }) {
                        let [r, g, b, _] = color.to_rgba8(1.);
                        board
                            .texts
                            .get_mut(id)
                            .ok_or(BoardsError::InvalidBoard)?
                            .color = format!("#{r:02x}{g:02x}{b:02x}");
                    }
                }
            }
            Ok(())
        })
    }
}

fn validate_options(o: &TextFrameOptions) -> Result<(), BoardsError> {
    let values = [
        o.gutter,
        o.column_width,
        o.first_baseline_min,
        o.auto_size_min_height,
        o.auto_size_min_width,
        o.column_rule_weight,
        o.column_rule_offset,
        o.column_rule_top_inset,
        o.column_rule_bottom_inset,
    ];
    if !(1..=40).contains(&o.columns)
        || values
            .iter()
            .chain(o.inset.iter())
            .any(|v| !v.is_finite() || v.abs() > 100_000.)
        || o.gutter < 0.
        || o.inset.iter().any(|v| *v < 0.)
        || o.auto_size_ref > 8
    {
        return Err(BoardsError::InvalidPlacement);
    }
    Ok(())
}

// Native serde skips optional/default frame fields on disk; the settings catalogue must still expose them.
fn frame_value(o: &TextFrameOptions) -> Result<Value, BoardsError> {
    let mut v = serde_json::to_value(o)?;
    for (key, value) in [
        ("autoSizeUseMinHeight", json!(o.auto_size_use_min_height)),
        ("autoSizeMinHeight", json!(o.auto_size_min_height)),
        ("autoSizeUseMinWidth", json!(o.auto_size_use_min_width)),
        ("autoSizeMinWidth", json!(o.auto_size_min_width)),
        ("autoSizeNoLineBreaks", json!(o.auto_size_no_line_breaks)),
        ("baselineGrid", json!(o.baseline_grid)),
        ("path", json!(o.path)),
    ] {
        v[key] = value;
    }
    Ok(v)
}
