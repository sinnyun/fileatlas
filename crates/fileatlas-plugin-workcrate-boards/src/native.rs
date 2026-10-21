use super::*;
pub(crate) fn split_id(id: &str) -> Result<(&str, u64), BoardsError> {
    let (project, remainder) = id
        .split_once(':')
        .ok_or_else(|| BoardsError::Invalid("invalid board identity".into()))?;
    let (binding, native) = remainder
        .split_once(':')
        .ok_or_else(|| BoardsError::Invalid("board binding missing".into()))?;
    if binding.is_empty() || native.contains(':') {
        return Err(BoardsError::Invalid("invalid board binding".into()));
    }
    Ok((
        project,
        native
            .parse()
            .map_err(|_| BoardsError::Invalid("invalid native ID".into()))?,
    ))
}
fn same_path(a: &str, b: &Path) -> bool {
    let normalized = |s: &str| {
        s.trim_start_matches(r"\\?\")
            .replace('\\', "/")
            .to_lowercase()
    };
    normalized(a) == normalized(&b.to_string_lossy())
}
pub(crate) fn assert_path(c: &mut Client, path: &Path) -> Result<Value, BoardsError> {
    let inspect = c.request("document.inspect", json!({"depth":0}))?;
    if !inspect["path"].as_str().is_some_and(|p| same_path(p, path)) {
        return Err(BoardsError::Invalid(
            "active native document changed".into(),
        ));
    }
    c.bound_path = Some(path.to_owned());
    Ok(inspect)
}
pub(crate) fn inspect_optional(c: &mut Client) -> Result<Option<Value>, BoardsError> {
    match c.request("document.inspect", json!({"depth":0})) {
        Ok(value) => Ok(Some(value)),
        Err(BoardsError::Remote(message))
            if message.trim_matches('"')
                == "command `document.inspect` is not available right now: no document open" =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}
pub(crate) fn open_safely(c: &mut Client, path: &Path) -> Result<(), BoardsError> {
    open_document(c, path, false)
}
pub(crate) fn revalidate_open(c: &mut Client, path: &Path) -> Result<(), BoardsError> {
    open_document(c, path, true)
}
fn open_document(c: &mut Client, path: &Path, force: bool) -> Result<(), BoardsError> {
    if !force {
        if let Some(diagnostics) = c
            .diagnostics
            .lock()
            .map_err(|_| BoardsError::Poisoned)?
            .get(path)
        {
            return Err(BoardsError::Invalid(format!(
                "native resource diagnostics require explicit reattach/revalidation: {diagnostics}"
            )));
        }
    }
    if let Some(i) = inspect_optional(c)? {
        if !force && i["path"].as_str().is_some_and(|p| same_path(p, path)) {
            c.bound_path = Some(path.to_owned());
            return Ok(());
        }
        if i["dirty"].as_bool() == Some(true) {
            return Err(BoardsError::DirtyDocument);
        }
    }
    let opened = c.request("app.open", json!({"path":path}))?;
    if opened.is_null() {
        return Err(BoardsError::Invalid(
            "official open pending user dialog".into(),
        ));
    }
    if ["warnings", "missingLinks", "modifiedLinks"]
        .iter()
        .any(|key| opened[*key].as_array().is_some_and(|a| !a.is_empty()))
    {
        c.diagnostics
            .lock()
            .map_err(|_| BoardsError::Poisoned)?
            .insert(path.into(), opened.clone());
        return Err(BoardsError::Invalid(format!(
            "native document opened with resource diagnostics: {opened}"
        )));
    }
    c.diagnostics
        .lock()
        .map_err(|_| BoardsError::Poisoned)?
        .remove(path);
    assert_path(c, path)?;
    Ok(())
}
pub(crate) fn catalog(
    c: &mut Client,
    project: &str,
    binding: &str,
) -> Result<Vec<WorkCrateBoard>, BoardsError> {
    let model = c.execute("document.json", json!({}))?;
    let inspect = c.request("document.inspect", json!({"depth":0}))?;
    let boards = model["artboards"]
        .as_array()
        .ok_or_else(|| BoardsError::Protocol("native artboard list missing".into()))?;
    if boards.len() > 10000 {
        return Err(BoardsError::Invalid("too many artboards".into()));
    }
    let fields = header_fields(c)?;
    let mut out = Vec::new();
    for (index, b) in boards.iter().enumerate() {
        let native_id = b["id"]
            .as_u64()
            .ok_or_else(|| BoardsError::Protocol("native artboard ID missing".into()))?;
        let r = &b["rect"];
        let w = r["x1"]
            .as_f64()
            .zip(r["x0"].as_f64())
            .map(|(a, b)| a - b)
            .or_else(|| inspect["artboards"][index]["width"].as_f64())
            .ok_or_else(|| BoardsError::Protocol("width missing".into()))?;
        let h = r["y1"]
            .as_f64()
            .zip(r["y0"].as_f64())
            .map(|(a, b)| a - b)
            .or_else(|| inspect["artboards"][index]["height"].as_f64())
            .ok_or_else(|| BoardsError::Protocol("height missing".into()))?;
        let mut info = PageInfo {
            order: index as u32 + 1,
            ..Default::default()
        };
        if let Some(values) = fields.get(&native_id) {
            for (role, (_, text)) in values {
                match role.as_str() {
                    "material" => info.material = text.clone(),
                    "size" => info.size = text.clone(),
                    "quantity" => info.quantity = text.clone(),
                    "size_label" => info.size_label = text.clone(),
                    "project_name" => info.project_name_override = Some(text.clone()),
                    key if key.starts_with("extra:") => {
                        info.extra.insert(key[6..].into(), text.clone());
                    }
                    _ => {}
                }
            }
        }
        out.push(WorkCrateBoard {
            id: format!("{project}:{binding}:{native_id}"),
            project_id: project.into(),
            native_id,
            index,
            title: b["name"].as_str().unwrap_or("Artboard").into(),
            width_pt: w,
            height_pt: h,
            info,
        })
    }
    Ok(out)
}
pub(crate) type Fields = BTreeMap<u64, BTreeMap<String, (u64, String)>>;
pub(crate) fn header_fields(c: &mut Client) -> Result<Fields, BoardsError> {
    extract_fields(&c.execute("document.json", json!({}))?)
}
pub(crate) fn extract_fields(document: &Value) -> Result<Fields, BoardsError> {
    fn visit(
        node: &Value,
        out: &mut Fields,
        count: &mut usize,
        depth: usize,
    ) -> Result<(), BoardsError> {
        *count += 1;
        if *count > 100000 || depth > 128 {
            return Err(BoardsError::Invalid(
                "native model exceeds traversal bounds".into(),
            ));
        }
        if let Some(note) = node["attrs"]["note"]
            .as_str()
            .and_then(|s| serde_json::from_str::<Value>(s).ok())
        {
            if note["namespace"] == "fileatlas.workcrate"
                && note["version"] == 1
                && node["kind"]["type"] == "text"
            {
                let board = note["board"]
                    .as_u64()
                    .or_else(|| note["artboardId"].as_u64());
                if let (Some(board), Some(role), Some(id)) =
                    (board, note["role"].as_str(), node["id"].as_u64())
                {
                    let runs = node["kind"]["runs"]
                        .as_array()
                        .ok_or_else(|| BoardsError::Protocol("text runs missing".into()))?;
                    let text = runs
                        .iter()
                        .filter_map(|r| r["text"].as_str())
                        .collect::<String>();
                    if out
                        .entry(board)
                        .or_default()
                        .insert(role.into(), (id, text))
                        .is_some()
                    {
                        return Err(BoardsError::Invalid("duplicate owned header field".into()));
                    }
                }
            }
        }
        if let Some(children) = node["kind"]["children"].as_array() {
            for child in children {
                visit(child, out, count, depth + 1)?
            }
        }
        Ok(())
    }
    let mut fields = Fields::new();
    let mut count = 0;
    for layer in document["layers"]
        .as_array()
        .ok_or_else(|| BoardsError::Protocol("native layers missing".into()))?
    {
        visit(layer, &mut fields, &mut count, 0)?
    }
    Ok(fields)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn node(id: u64, version: u64, text: &str) -> Value {
        json!({"id":id,"attrs":{"note":json!({"namespace":"fileatlas.workcrate","version":version,"board":7,"role":"material"}).to_string()},"kind":{"type":"text","runs":[{"text":text}]}})
    }
    #[test]
    fn actual_text_and_unknown_versions() -> Result<(), BoardsError> {
        let value = json!({"layers":[node(1,1,"中文123"),node(2,2,"ignored")]});
        assert_eq!(extract_fields(&value)?[&7]["material"].1, "中文123");
        Ok(())
    }
    #[test]
    fn duplicate_roles_refuse_ambiguous_mutation() {
        assert!(extract_fields(&json!({"layers":[node(1,1,"A"),node(2,1,"B")]})).is_err());
    }
}
