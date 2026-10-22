//! Write the WorkCrate-owned material header band through official VectorCraft commands.
//!
//! The official program owns every text metric, so this module creates or restyles each
//! field, reads the box the program reports, asks [`crate::header_layout`] where that box
//! belongs, and then moves the node there. Nothing here measures text locally, and no
//! non-header object is created, moved, resized or deleted.
use super::*;
use crate::header_layout as geometry;
use crate::protocol::NativeProtocol;
use std::collections::BTreeSet;

const FONT: &str = "Microsoft YaHei";

/// One header field the page wants printed: its role, text, and the label a detail row uses.
#[derive(Clone)]
struct Requested {
    role: String,
    label: String,
    text: String,
}

/// Where a node is after the official program created or restyled it.
struct Sited {
    id: u64,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
}

/// The header nodes of one page plus the boxes the official program reported for them.
struct Header {
    board: u64,
    origin: (f64, f64),
    owned: native::OwnedNodes,
    sited: BTreeMap<String, Sited>,
}

pub(crate) fn write(
    c: &mut impl NativeProtocol,
    b: &WorkCrateBoard,
    title: &str,
    info: &PageInfo,
    project_name: &str,
) -> Result<(), BoardsError> {
    check_request(b, title, info, project_name)?;
    let inspect = c.request("document.inspect", json!({"depth":0}))?;
    if !inspect["isolation"].is_null() {
        return Err(BoardsError::Invalid(
            "请先退出官方程序的隔离模式，再保存页面内容".into(),
        ));
    }
    let rect = &inspect["artboards"][b.index];
    let origin = (
        rect["x"].as_f64().unwrap_or(0.),
        rect["y"].as_f64().unwrap_or(0.),
    );
    if !origin.0.is_finite() || !origin.1.is_finite() {
        return Err(BoardsError::Protocol(
            "官方程序未返回页面位置，请重新同步页面".into(),
        ));
    }
    c.execute(
        "edit.undoGroup.begin",
        json!({"label":"WorkCrate material header"}),
    )?;
    let outcome = apply(c, b, title, info, project_name, origin);
    match outcome {
        Ok(()) => {
            c.execute("edit.undoGroup.end", json!({}))?;
            Ok(())
        }
        Err(error) => {
            let _ = c.execute("edit.undoGroup.end", json!({"cancel":true}));
            Err(error)
        }
    }
}

fn check_request(
    b: &WorkCrateBoard,
    title: &str,
    info: &PageInfo,
    project_name: &str,
) -> Result<(), BoardsError> {
    if title.is_empty() {
        return Err(BoardsError::Invalid(
            "页面名称不能为空，请填写后再保存".into(),
        ));
    }
    if title.chars().count() > 512 {
        return Err(BoardsError::Invalid("页面名称过长，请缩短后再保存".into()));
    }
    if serde_json::to_vec(info)?.len() > 16384 || info.extra.len() > 24 {
        return Err(BoardsError::Invalid(
            "页面内容过多，请减少字段或缩短内容后再保存".into(),
        ));
    }
    if b.width_pt < 200. || b.height_pt < 100. {
        return Err(BoardsError::Invalid(
            "页面尺寸太小，放不下物料信息条".into(),
        ));
    }
    for text in [
        project_name,
        &info.material,
        &info.size,
        &info.quantity,
        &info.size_label,
    ]
    .into_iter()
    .chain(info.extra.values().map(String::as_str))
    {
        if text.chars().count() > 1024
            || text
                .chars()
                .any(|ch| (ch as u32) < 32 && !matches!(ch, '\n' | '\r' | '\t'))
        {
            return Err(BoardsError::Invalid(
                "字段内容过长或含有控制字符，请缩短后重试".into(),
            ));
        }
    }
    Ok(())
}

/// The layout's reason names an internal role id, so it is replaced here: the panel only ever
/// shows which page field failed and what the user should do.
fn describe(error: geometry::HeaderLayoutError) -> BoardsError {
    BoardsError::Invalid(match error {
        geometry::HeaderLayoutError::Unmeasured(_) => {
            "官方程序未返回该字段的排版尺寸，请重新同步页面后再保存".into()
        }
        geometry::HeaderLayoutError::TooWide(_) => {
            "字段内容过宽，这一页放不下物料信息条，请缩短后重试".into()
        }
        geometry::HeaderLayoutError::TooManyRows(rows) => {
            format!("附加字段过多，物料信息条最多容纳 {rows} 行")
        }
    })
}

fn apply(
    c: &mut impl NativeProtocol,
    b: &WorkCrateBoard,
    title: &str,
    info: &PageInfo,
    project_name: &str,
    origin: (f64, f64),
) -> Result<(), BoardsError> {
    c.execute("artboard.setProps", json!({"index":b.index,"name":title}))?;
    let mut owned = native::owned(c, b.native_id)?;
    if let Some(group) = owned.legacy_group {
        // Documents written before the compact band wrapped the whole block in one group.
        // Releasing it deletes only FileAtlas-owned nodes, never the page artwork.
        c.execute("edit.clear", json!({"ids":[group]}))?;
        owned = native::OwnedNodes::default();
    }
    native::header_layer(c)?;
    let mut header = Header {
        board: b.native_id,
        origin,
        owned,
        sited: BTreeMap::new(),
    };

    let page_number = format!("{:02}", b.index + 1);
    let group = [
        requested("material", "物料", &info.material),
        requested("size", "尺寸", &info.size),
        requested("quantity", "数量", &info.quantity),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let mut rows = [requested("size_label", "规格", &info.size_label)]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    for (key, value) in &info.extra {
        if let Some(field) = requested(&format!("extra:{key}"), key, value) {
            rows.push(field);
        }
    }

    // Pass one: the primary row and the separators it would need, so the official metrics
    // decide whether the material run still shares that row.
    header.row(
        c,
        geometry::ROLE_PAGE_NUMBER,
        &page_number,
        geometry::PAGE_NUMBER_SIZE,
        geometry::COLOR_ACCENT,
        true,
        false,
    )?;
    header.row(
        c,
        geometry::ROLE_PROJECT_NAME,
        project_name,
        geometry::PROJECT_NAME_SIZE,
        geometry::COLOR_PRIMARY,
        true,
        info.project_name_override.is_none(),
    )?;
    for field in &group {
        header.row(
            c,
            &field.role,
            &field.text,
            geometry::GROUP_SIZE,
            geometry::COLOR_ACCENT,
            false,
            false,
        )?;
    }
    for index in 0..group.len().saturating_sub(1) {
        header.row(
            c,
            &geometry::separator_role(index),
            "·",
            geometry::GROUP_SIZE,
            geometry::COLOR_MUTED,
            false,
            false,
        )?;
    }
    let measured = metrics(&header.sited);
    let group_fields = fields(&group);
    let group_in_primary = geometry::group_fits(
        origin.0,
        b.width_pt,
        &page_number,
        project_name,
        &group_fields,
        &measured,
    )
    .map_err(describe)?;

    // Pass two: the rows below the primary one. Demoted fields are restyled to detail size,
    // so every metric handed to the layout describes the size that text prints at. Each
    // detail row needs both its value and its label measured by the official program.
    if !group_in_primary {
        rows.extend(group.iter().cloned());
    }
    for field in &rows {
        header.row(
            c,
            &field.role,
            &field.text,
            geometry::SUB_SIZE,
            geometry::COLOR_PRIMARY,
            false,
            false,
        )?;
        header.row(
            c,
            &geometry::label_role(&field.role),
            &field.label,
            geometry::SUB_SIZE,
            geometry::COLOR_MUTED,
            false,
            false,
        )?;
    }
    let row_fields = fields(&rows);
    let layout = geometry::build(
        origin.0,
        origin.1,
        b.width_pt,
        &page_number,
        project_name,
        &group_fields,
        &row_fields,
        group_in_primary,
        &metrics(&header.sited),
    )
    .map_err(describe)?;

    for placement in &layout.rows {
        let site = header.sited.get(placement.role.as_str()).ok_or_else(|| {
            BoardsError::Protocol("官方程序未返回该字段的排版位置，请重新同步页面".into())
        })?;
        let (dx, dy) = (placement.left - site.left, placement.top - site.top);
        if dx.abs() > 0.01 || dy.abs() > 0.01 {
            c.execute(
                "object.transform",
                json!({"ids":[site.id],"matrix":[1,0,0,1,dx,dy]}),
            )?;
        }
    }

    let wanted = layout
        .rows
        .iter()
        .map(|placement| placement.role.as_str())
        .collect::<BTreeSet<_>>();
    let stale = header
        .owned
        .fields
        .iter()
        .filter(|(role, _)| !wanted.contains(role.as_str()))
        .map(|(_, node)| node.id)
        .collect::<Vec<_>>();
    for id in stale {
        c.execute("edit.clear", json!({"ids":[id]}))?;
    }

    // The band is page furniture behind its own text: rebuilt at the measured height with
    // the rounded corners from the sketch. The page-number chip is created after it and
    // both are ordered behind the text, never resized through the selection.
    if let Some(id) = header.owned.band {
        c.execute("edit.clear", json!({"ids":[id]}))?;
    }
    if let Some(id) = header.owned.chip {
        c.execute("edit.clear", json!({"ids":[id]}))?;
    }
    let rectangle = c.execute(
        "shape.rectangle",
        json!({"x":layout.band.x,"y":layout.band.y,"width":layout.band.width,"height":layout.band.height,"radius":geometry::BAND_RADIUS}),
    )?;
    let band = rectangle["id"].as_u64().ok_or_else(|| {
        BoardsError::Protocol("官方程序未创建信息条底色，请重新同步页面后再保存".into())
    })?;
    c.execute(
        "paint.setFill",
        json!({"id":band,"color":geometry::COLOR_BAND}),
    )?;
    c.execute("paint.setStroke", json!({"id":band,"none":true}))?;
    tag(c, band, geometry::ROLE_BAND, b.native_id, false)?;
    if let Some(chip) = &layout.chip {
        let rectangle = c.execute(
            "shape.rectangle",
            json!({"x":chip.x,"y":chip.y,"width":chip.width,"height":chip.height,"radius":chip.radius}),
        )?;
        let id = rectangle["id"].as_u64().ok_or_else(|| {
            BoardsError::Protocol("官方程序未创建页码底色，请重新同步页面后再保存".into())
        })?;
        c.execute(
            "paint.setFill",
            json!({"id":id,"color":geometry::COLOR_CHIP}),
        )?;
        c.execute(
            "paint.setStroke",
            json!({"id":id,"color":geometry::COLOR_CHIP_EDGE}),
        )?;
        tag(c, id, geometry::ROLE_PAGE_CHIP, b.native_id, false)?;
    }
    c.execute("object.arrange.sendToBack", json!({"ids":[band]}))?;
    let text = header
        .sited
        .values()
        .map(|site| site.id)
        .collect::<Vec<_>>();
    if !text.is_empty() {
        c.execute("object.arrange.bringToFront", json!({"ids":text}))?;
    }
    Ok(())
}

fn requested(role: &str, label: &str, text: &str) -> Option<Requested> {
    (!text.is_empty()).then(|| Requested {
        role: role.into(),
        label: label.into(),
        text: text.into(),
    })
}

fn fields(rows: &[Requested]) -> Vec<geometry::Field<'_>> {
    rows.iter()
        .map(|row| geometry::Field {
            role: &row.role,
            label: &row.label,
            text: &row.text,
        })
        .collect()
}

fn metrics(sited: &BTreeMap<String, Sited>) -> BTreeMap<String, geometry::Measured> {
    sited
        .iter()
        .map(|(role, site)| {
            (
                role.clone(),
                geometry::Measured {
                    width: site.width,
                    height: site.height,
                },
            )
        })
        .collect()
}

impl Header {
    /// Make one owned text node hold `text` at `size` and remember the box the official
    /// program reports for it.
    fn row(
        &mut self,
        c: &mut impl NativeProtocol,
        role: &str,
        text: &str,
        size: f64,
        color: &'static str,
        bold: bool,
        inherited: bool,
    ) -> Result<(), BoardsError> {
        let style = if bold { "bold" } else { "regular" };
        let existing = self
            .owned
            .fields
            .get(role)
            .map(|node| (node.id, node.text.clone(), node.inherited));
        let site = match existing {
            Some((id, current, current_inherited)) => {
                if current != text {
                    c.execute("text.setText", json!({"id":id,"text":text}))?;
                }
                c.execute(
                    "text.setStyle",
                    json!({"id":id,"size":size,"fill":color,"style":style}),
                )?;
                if current_inherited != inherited {
                    tag(c, id, role, self.board, inherited)?;
                }
                let (left, top, width, height) = measured_box(c, id)?;
                Sited {
                    id,
                    left,
                    top,
                    width,
                    height,
                }
            }
            None => {
                let created = c.execute(
                    "text.create",
                    json!({"x":self.origin.0 + geometry::TEXT_INSET,"y":self.origin.1 + geometry::PRIMARY_CENTER,"text":text,"size":size,"font":FONT,"style":style,"color":color}),
                )?;
                let id = created["id"].as_u64().ok_or_else(|| {
                    BoardsError::Protocol("官方程序未创建信息条文字，请重新同步页面后再保存".into())
                })?;
                tag(c, id, role, self.board, inherited)?;
                let bounds = &created["bounds"];
                let site = Sited {
                    id,
                    left: finite(bounds, "x")?,
                    top: finite(bounds, "y")?,
                    width: finite(bounds, "width")?,
                    height: finite(bounds, "height")?,
                };
                self.owned.fields.insert(
                    role.into(),
                    native::HeaderNode {
                        id,
                        text: text.into(),
                        inherited,
                    },
                );
                site
            }
        };
        if let Some(node) = self.owned.fields.get_mut(role) {
            node.text = text.into();
            node.inherited = inherited;
        }
        self.sited.insert(role.into(), site);
        Ok(())
    }
}

fn tag(
    c: &mut impl NativeProtocol,
    id: u64,
    role: &str,
    board: u64,
    inherited: bool,
) -> Result<(), BoardsError> {
    let mut note = json!({"namespace":"fileatlas.workcrate","version":1,"board":board,"role":role});
    if inherited {
        note["inherited"] = json!(true);
    }
    let written = c.execute("attributes.set", json!({"id":id,"note":note.to_string()}))?;
    if written["changed"].as_u64() != Some(1) {
        return Err(BoardsError::Protocol(
            "官方程序未能记录该字段的归属，请重新同步页面后再保存".into(),
        ));
    }
    Ok(())
}

fn measured_box(c: &mut impl NativeProtocol, id: u64) -> Result<(f64, f64, f64, f64), BoardsError> {
    let node = c.execute("document.node", json!({"id":id,"summary":true}))?;
    let bounds = &node["bounds"];
    Ok((
        finite(bounds, "x")?,
        finite(bounds, "y")?,
        finite(bounds, "width")?,
        finite(bounds, "height")?,
    ))
}

fn finite(value: &Value, key: &str) -> Result<f64, BoardsError> {
    value[key]
        .as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| {
            BoardsError::Protocol("官方程序未返回文字排版尺寸，请重新同步页面后再保存".into())
        })
}
