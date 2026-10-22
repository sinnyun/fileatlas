//! Persistent three-text material bar. Official point text owns right alignment and metrics.
use super::*;
use crate::{header_layout as g, protocol::NativeProtocol};
mod payload;
pub(crate) use payload::Details;
use payload::single_line;
const LAYOUT: u32 = 2;

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct WriteReport {
    initialized: bool,
    commands: Vec<Value>,
}
impl WriteReport {
    pub(crate) fn changed(&self) -> bool {
        !self.commands.is_empty()
    }
    pub(crate) fn installed(&self) -> bool {
        self.initialized
    }
}
struct Tracked<'a, C> {
    inner: &'a mut C,
    report: WriteReport,
}
impl<C: NativeProtocol> NativeProtocol for Tracked<'_, C> {
    fn request(&mut self, method: &str, params: Value) -> Result<Value, BoardsError> {
        self.inner.request(method, params)
    }
    fn execute(&mut self, command: &str, params: Value) -> Result<Value, BoardsError> {
        let result = self.inner.execute(command, params.clone())?;
        if !command.starts_with("document.") {
            self.report
                .commands
                .push(json!({"command":command,"id":params.get("id"),"ids":params.get("ids")}));
        }
        Ok(result)
    }
}

pub(crate) fn write(
    c: &mut impl NativeProtocol,
    b: &WorkCrateBoard,
    title: &str,
    info: &PageInfo,
    category: &str,
) -> Result<WriteReport, BoardsError> {
    write_impl(c, b, title, info, category, true, None)
}

/// The caller owns the surrounding native undo group (page order transaction).
pub(crate) fn write_in_group(
    c: &mut impl NativeProtocol,
    b: &WorkCrateBoard,
    title: &str,
    info: &PageInfo,
    layout: &native::OwnedLayout,
) -> Result<WriteReport, BoardsError> {
    write_impl(
        c,
        b,
        title,
        info,
        &info.material_category,
        false,
        Some(layout),
    )
}

fn write_impl(
    c: &mut impl NativeProtocol,
    b: &WorkCrateBoard,
    title: &str,
    info: &PageInfo,
    category: &str,
    transaction: bool,
    layout: Option<&native::OwnedLayout>,
) -> Result<WriteReport, BoardsError> {
    validate(b, title, info)?;
    let loaded;
    let layout = match layout {
        Some(layout) => layout,
        None => {
            loaded = native::owned_layout(c)?;
            &loaded
        }
    };
    let owned = layout.boards.get(&b.native_id).cloned().unwrap_or_default();
    let details = Details::from_info(info);
    let note = note(b.native_id, g::ROLE_DETAILS, json!({"details":details}));
    let category_note = note_value(b.native_id, category);
    let initialized = owned.layout_version == LAYOUT
        && owned.page_group.is_some()
        && layout.header_root.is_some()
        && owned.band.is_some()
        && owned.chip.is_some()
        && [g::ROLE_PAGE_NUMBER, g::ROLE_PROJECT_NAME, g::ROLE_DETAILS]
            .iter()
            .all(|key| owned.fields.contains_key(*key));
    let number = format!("{:02}", b.index + 1);
    let mut recolor = Vec::new();
    if initialized {
        for role in [g::ROLE_PAGE_NUMBER, g::ROLE_PROJECT_NAME, g::ROLE_DETAILS] {
            let id = owned.fields[role].id;
            let summary = c.execute("document.node", json!({"id":id,"summary":true}))?;
            if summary["fill"].as_str() != Some(g::COLOR_TEXT) {
                recolor.push(id);
            }
        }
    }
    let category_text = visible(category);
    let detail_text = visible(&details.text());
    let changed = !initialized
        || !recolor.is_empty()
        || b.title != title
        || [
            (g::ROLE_PAGE_NUMBER, number.as_str(), None),
            (
                g::ROLE_PROJECT_NAME,
                category_text.as_str(),
                Some(&category_note),
            ),
            (g::ROLE_DETAILS, detail_text.as_str(), Some(&note)),
        ]
        .into_iter()
        .any(|(role, text, note)| {
            owned
                .fields
                .get(role)
                .is_none_or(|node| node.text != text || note.is_some_and(|note| note != &node.note))
        });
    if !changed {
        return Ok(WriteReport::default());
    }
    let inspect = c.request("document.inspect", json!({"depth":0}))?;
    if !inspect["isolation"].is_null() {
        return Err(BoardsError::Invalid(
            "请先退出官方程序的隔离模式，再保存页面内容".into(),
        ));
    }
    let rect = &inspect["artboards"][b.index];
    let geometry = g::Geometry {
        x: finite(rect, "x")?,
        y: finite(rect, "y")?,
        width: b.width_pt,
    };
    let mut c = Tracked {
        inner: c,
        report: WriteReport {
            initialized: !initialized,
            commands: vec![],
        },
    };
    if transaction {
        c.execute(
            "edit.undoGroup.begin",
            json!({"label":"WorkCrate material header"}),
        )?;
    }
    let result = (|| {
        if b.title != title {
            c.execute("artboard.setProps", json!({"index":b.index,"name":title}))?;
        }
        if initialized {
            // Existing documents adopt the requested ink without replacing text nodes.
            if !recolor.is_empty() {
                c.execute("text.setStyle", json!({"ids":recolor,"fill":g::COLOR_TEXT}))?;
            }
            patch(&mut c, &owned, g::ROLE_PAGE_NUMBER, &number, None, geometry)?;
            patch(
                &mut c,
                &owned,
                g::ROLE_PROJECT_NAME,
                &category_text,
                Some(category_note),
                geometry,
            )?;
            patch(
                &mut c,
                &owned,
                g::ROLE_DETAILS,
                &detail_text,
                Some(note),
                geometry,
            )?;
        } else {
            install(
                &mut c,
                b,
                &owned,
                layout.header_root,
                geometry,
                &number,
                &category_text,
                &detail_text,
                category_note,
                note,
            )?;
        }
        Ok::<_, BoardsError>(())
    })();
    match result {
        Ok(()) => {
            if transaction {
                c.execute("edit.undoGroup.end", json!({}))?;
            }
            Ok(c.report)
        }
        Err(error) => {
            if transaction {
                c.execute("edit.undoGroup.end", json!({"cancel":true}))?;
            }
            Err(error)
        }
    }
}

fn visible(text: &str) -> String {
    let value = single_line(text);
    // A space keeps an empty point-text object and its anchor alive without printing a value.
    if value.is_empty() { " ".into() } else { value }
}
fn note(board: u64, role: &str, extras: Value) -> Value {
    let mut note = json!({"namespace":"fileatlas.workcrate","version":1,"layout":LAYOUT,"board":board,"role":role});
    if let Some(extras) = extras.as_object() {
        for (key, value) in extras {
            note[key] = value.clone();
        }
    }
    note
}
fn note_value(board: u64, value: &str) -> Value {
    note(board, g::ROLE_PROJECT_NAME, json!({"value":value}))
}
fn tag(c: &mut impl NativeProtocol, id: u64, note: Value) -> Result<(), BoardsError> {
    let result = c.execute("attributes.set", json!({"id":id,"note":note.to_string()}))?;
    if result["changed"].as_u64() != Some(1) {
        return Err(BoardsError::Protocol(
            "官方程序未能记录物料栏字段，请重新同步后再保存".into(),
        ));
    }
    Ok(())
}
fn patch(
    c: &mut impl NativeProtocol,
    owned: &native::OwnedNodes,
    role: &str,
    text: &str,
    note: Option<Value>,
    geometry: g::Geometry,
) -> Result<(), BoardsError> {
    let node = &owned.fields[role];
    if node.text != text {
        c.execute("text.setText", json!({"id":node.id,"text":text}))?;
        check_bounds(c, node.id, role, geometry)?;
    }
    if let Some(note) = note
        && note != node.note
    {
        tag(c, node.id, note)?;
    }
    Ok(())
}
fn text(
    c: &mut impl NativeProtocol,
    role: &str,
    value: &str,
    note: Value,
    geometry: g::Geometry,
) -> Result<u64, BoardsError> {
    let (x, size, color, justify, style) = match role {
        g::ROLE_PAGE_NUMBER => (
            geometry.x + 36.,
            g::PAGE_NUMBER_SIZE,
            g::COLOR_TEXT,
            "center",
            "bold",
        ),
        g::ROLE_PROJECT_NAME => (
            geometry.category_x(),
            g::PROJECT_NAME_SIZE,
            g::COLOR_TEXT,
            "left",
            "bold",
        ),
        _ => (
            geometry.right(),
            g::GROUP_SIZE,
            g::COLOR_TEXT,
            "right",
            "regular",
        ),
    };
    let created = c.execute("text.create", json!({"x":x,"y":geometry.centre_y(),"text":value,"size":size,"font":g::TEXT_FONT,"style":style,"color":color,"justify":justify}))?;
    let id = created["id"]
        .as_u64()
        .ok_or_else(|| BoardsError::Protocol("官方程序未创建物料栏文字，请重试".into()))?;
    // Official metrics are used once to vertically centre each independent text object.
    let bounds = bounds(c, id)?;
    let delta = geometry.centre_y() - finite(&bounds, "y")? - finite(&bounds, "height")? / 2.;
    if delta.abs() > 0.01 {
        c.execute(
            "object.transform",
            json!({"ids":[id],"matrix":[1,0,0,1,0,delta]}),
        )?;
    }
    check_bounds(c, id, role, geometry)?;
    tag(c, id, note)?;
    Ok(id)
}
fn rectangle(
    c: &mut impl NativeProtocol,
    board: u64,
    role: &str,
    rect: Value,
    fill: &str,
    stroke: Option<&str>,
) -> Result<u64, BoardsError> {
    let result = c.execute("shape.rectangle", rect)?;
    let id = result["id"]
        .as_u64()
        .ok_or_else(|| BoardsError::Protocol("官方程序未创建物料栏底色，请重试".into()))?;
    c.execute("paint.setFill", json!({"id":id,"color":fill}))?;
    c.execute(
        "paint.setStroke",
        stroke.map_or_else(
            || json!({"id":id,"none":true}),
            |color| json!({"id":id,"color":color}),
        ),
    )?;
    tag(c, id, note(board, role, json!({})))?;
    Ok(id)
}
#[allow(clippy::too_many_arguments)]
fn install(
    c: &mut impl NativeProtocol,
    b: &WorkCrateBoard,
    old: &native::OwnedNodes,
    root: Option<u64>,
    geometry: g::Geometry,
    number: &str,
    category: &str,
    details: &str,
    category_note: Value,
    detail_note: Value,
) -> Result<(), BoardsError> {
    if let Some(root) = root {
        c.execute("object.setProps", json!({"id":root,"locked":false}))?;
    }
    native::header_layer(c)?;
    let band = rectangle(
        c,
        b.native_id,
        "band",
        json!({"x":geometry.x+8.,"y":geometry.y+8.,"width":b.width_pt-16.,"height":52.,"radius":g::BAND_RADIUS}),
        g::COLOR_BAND,
        None,
    )?;
    let chip = rectangle(
        c,
        b.native_id,
        "page_number_chip",
        json!({"x":geometry.x+16.,"y":geometry.y+22.,"width":40.,"height":24.,"radius":g::CHIP_RADIUS}),
        g::COLOR_CHIP,
        Some(g::COLOR_CHIP_EDGE),
    )?;
    let page = text(
        c,
        g::ROLE_PAGE_NUMBER,
        number,
        note(b.native_id, g::ROLE_PAGE_NUMBER, json!({})),
        geometry,
    )?;
    let category = text(c, g::ROLE_PROJECT_NAME, category, category_note, geometry)?;
    let details = text(c, g::ROLE_DETAILS, details, detail_note, geometry)?;
    let order = [band, chip, page, category, details];
    let grouped = c.execute("object.group", json!({"ids":order}))?;
    let group = grouped["id"]
        .as_u64()
        .ok_or_else(|| BoardsError::Protocol("官方程序未创建物料栏分组".into()))?;
    tag(c, group, note(b.native_id, "header_page", json!({})))?;
    for (index, id) in order.into_iter().enumerate() {
        c.execute("node.move", json!({"id":id,"parent":group,"index":index}))?;
    }
    let root = match root {
        Some(root) => {
            c.execute("node.move", json!({"id":group,"parent":root}))?;
            root
        }
        None => {
            let grouped = c.execute("object.group", json!({"ids":[group]}))?;
            let id = grouped["id"]
                .as_u64()
                .ok_or_else(|| BoardsError::Protocol("官方程序未创建物料栏主组".into()))?;
            native::tag_node(c, id, "header_root", None, false)?;
            id
        }
    };
    // Install first, then retire the old subtree, so its shared root never becomes empty.
    if let Some(id) = old.page_group.or(old.legacy_group) {
        c.execute("edit.clear", json!({"ids":[id]}))?;
    } else {
        let ids = old
            .fields
            .values()
            .map(|n| n.id)
            .chain(old.band)
            .chain(old.chip)
            .collect::<Vec<_>>();
        if !ids.is_empty() {
            c.execute("edit.clear", json!({"ids":ids}))?;
        }
    }
    c.execute("object.setProps", json!({"id":root,"locked":true}))?;
    Ok(())
}
fn bounds(c: &mut impl NativeProtocol, id: u64) -> Result<Value, BoardsError> {
    Ok(c.execute("document.node", json!({"id":id,"summary":true}))?["bounds"].clone())
}
fn check_bounds(
    c: &mut impl NativeProtocol,
    id: u64,
    role: &str,
    geometry: g::Geometry,
) -> Result<(), BoardsError> {
    let bounds = bounds(c, id)?;
    let left = finite(&bounds, "x")?;
    let right = left + finite(&bounds, "width")?;
    let fits = match role {
        g::ROLE_DETAILS => left >= geometry.split() && right <= geometry.right() + 0.1,
        g::ROLE_PROJECT_NAME => {
            left >= geometry.category_x() - 0.1 && right <= geometry.split() - 12.
        }
        _ => left >= geometry.x + 16. && right <= geometry.x + 56.,
    };
    if !fits {
        return Err(BoardsError::Invalid(
            if role == g::ROLE_DETAILS {
                "物料信息过长，右侧单行放不下，请缩短内容后重试"
            } else {
                "编号或物料类别过长，左侧单行放不下，请缩短后重试"
            }
            .into(),
        ));
    }
    Ok(())
}
fn finite(value: &Value, key: &str) -> Result<f64, BoardsError> {
    value[key]
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| {
            BoardsError::Protocol("官方程序未返回物料栏文字边界，请重新同步后重试".into())
        })
}
fn validate(b: &WorkCrateBoard, title: &str, info: &PageInfo) -> Result<(), BoardsError> {
    if title.trim().is_empty() || title.chars().count() > 512 {
        return Err(BoardsError::Invalid("页面名称不能为空或过长".into()));
    }
    if b.width_pt < 200. || b.height_pt < 100. {
        return Err(BoardsError::Invalid("页面尺寸太小，放不下物料栏".into()));
    }
    if serde_json::to_vec(info)?.len() > 16384 || info.extra.len() > 24 {
        return Err(BoardsError::Invalid(
            "页面内容过多，请减少字段或缩短内容后再保存".into(),
        ));
    }
    for text in [
        &info.material_category,
        &info.material,
        &info.size,
        &info.quantity,
    ]
    .into_iter()
    .chain(info.extra.keys())
    .chain(info.extra.values())
    {
        if text.chars().count() > 1024
            || text
                .chars()
                .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
        {
            return Err(BoardsError::Invalid(
                "字段内容过长或含控制字符，请缩短后重试".into(),
            ));
        }
    }
    Ok(())
}
