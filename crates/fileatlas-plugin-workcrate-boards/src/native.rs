use super::*;
use crate::protocol::NativeProtocol;
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
pub(crate) fn same_path(a: &str, b: &Path) -> bool {
    let normalized = |s: &str| {
        s.trim_start_matches(r"\\?\")
            .replace('\\', "/")
            .to_lowercase()
    };
    normalized(a) == normalized(&b.to_string_lossy())
}
pub(crate) const ACTIVE_DOCUMENT_CHANGED: &str =
    "官方程序当前打开的文档已变化，请回到 WorkCrate 重新关联该工程后再操作";
pub(crate) fn inspect_matches(
    c: &mut impl NativeProtocol,
    path: &Path,
) -> Result<Value, BoardsError> {
    let inspect = c.request("document.inspect", json!({"depth":0}))?;
    if !inspect["path"].as_str().is_some_and(|p| same_path(p, path)) {
        return Err(BoardsError::Invalid(ACTIVE_DOCUMENT_CHANGED.into()));
    }
    Ok(inspect)
}
pub(crate) fn assert_path(c: &mut Client, path: &Path) -> Result<Value, BoardsError> {
    let inspect = inspect_matches(c, path)?;
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
/// Open in the editor GUI unless this exact path has unresolved resource diagnostics on record.
pub(crate) fn open_safely(c: &mut Client, path: &Path) -> Result<(), BoardsError> {
    if let Some(diagnostics) = c
        .diagnostics
        .lock()
        .map_err(|_| BoardsError::Poisoned)?
        .get(path)
    {
        return Err(BoardsError::Invalid(describe_open_report(
            diagnostics,
            false,
        )));
    }
    if let Some(i) = inspect_optional(c)? {
        if i["path"].as_str().is_some_and(|p| same_path(p, path)) {
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
            "官方程序正在等待窗口中的确认，请完成该对话框后再重试".into(),
        ));
    }
    apply_open_report(&opened, path, &c.diagnostics, false)?;
    assert_path(c, path)?;
    Ok(())
}
/// Re-associate an existing native document through the official headless CLI, so
/// attachment never needs the editor GUI.
pub(crate) fn revalidate_open_headless(
    c: &mut mcp::McpSession,
    path: &Path,
    diagnostics: &Mutex<BTreeMap<PathBuf, Value>>,
) -> Result<(), BoardsError> {
    let opened = c.open_file(path)?;
    apply_open_report(&opened, path, diagnostics, true)?;
    inspect_matches(c, path)?;
    Ok(())
}
fn report_links<'a>(report: &'a Value, key: &str) -> &'a [Value] {
    report[key].as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn link_absent(link: &Value) -> bool {
    link["path"]
        .as_str()
        .is_none_or(|raw| !Path::new(raw.trim_start_matches(r"\\?\")).is_file())
}
/// Missing links the user can actually act on: a sandboxed headless session also reports
/// links outside its read root, so a path that exists on disk is not shown as missing.
fn missing_links(report: &Value, sandboxed: bool) -> Vec<&Value> {
    report_links(report, "missingLinks")
        .iter()
        .filter(|link| !sandboxed || link_absent(link))
        .collect()
}
/// The official report names internal link ids and is unreadable on its own. The panel only
/// shows which files failed and the one action that revalidates them.
fn describe_open_report(report: &Value, sandboxed: bool) -> String {
    fn link_label(link: &Value) -> Option<String> {
        if let Some(name) = link["name"]
            .as_str()
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            return Some(name.to_owned());
        }
        let raw = link["path"].as_str()?;
        Path::new(raw.trim_start_matches(r"\\?\"))
            .file_name()?
            .to_str()
            .map(str::to_owned)
    }
    let mut issues = Vec::new();
    let missing = missing_links(report, sandboxed);
    if !missing.is_empty() {
        let names = missing
            .iter()
            .filter_map(|link| link_label(link))
            .take(3)
            .collect::<Vec<_>>();
        let count = missing.len();
        if names.is_empty() {
            issues.push(format!("有 {count} 个资源文件缺失"));
        } else if count > names.len() {
            issues.push(format!("资源文件缺失：{} 等 {count} 个", names.join("、")));
        } else {
            issues.push(format!("资源文件缺失：{}", names.join("、")));
        }
    }
    let modified = report_links(report, "modifiedLinks");
    if !modified.is_empty() {
        issues.push(format!(
            "官方程序改写了 {} 处资源链接，需要确认",
            modified.len()
        ));
    }
    let warnings = report_links(report, "warnings");
    if !warnings.is_empty() {
        issues.push(format!("官方程序报告了 {} 条资源警告", warnings.len()));
    }
    if issues.is_empty() {
        issues.push("官方程序报告资源异常".into());
    }
    let repair = if missing.is_empty() {
        "请先在 VectorCraft 中确认并保存该文档，然后回到 WorkCrate 点击“关联工程”重新选择该文档以重新校验；修复前该文档无法打开或重新关联。"
    } else {
        "请先在 VectorCraft 中恢复或重新链接这些资源并保存文档，然后回到 WorkCrate 点击“关联工程”重新选择该文档以重新校验；修复前该文档无法打开或重新关联。"
    };
    format!("{}。{repair}", issues.join("；"))
}
/// A linked resource reported missing by the headless session is a sandbox artifact when
/// the file is actually present: its automation read root is the native document's folder,
/// so links stored elsewhere cannot resolve even though nothing is broken.
fn open_report_blocking(opened: &Value, sandboxed: bool) -> bool {
    if ["warnings", "modifiedLinks"]
        .iter()
        .any(|key| !report_links(opened, key).is_empty())
    {
        return true;
    }
    !missing_links(opened, sandboxed).is_empty()
}
fn apply_open_report(
    opened: &Value,
    path: &Path,
    diagnostics: &Mutex<BTreeMap<PathBuf, Value>>,
    sandboxed: bool,
) -> Result<(), BoardsError> {
    if !open_report_blocking(opened, sandboxed) {
        diagnostics
            .lock()
            .map_err(|_| BoardsError::Poisoned)?
            .remove(path);
        return Ok(());
    }
    let message = describe_open_report(opened, sandboxed);
    let mut stored = opened.clone();
    if sandboxed {
        stored["missingLinks"] = Value::Array(
            missing_links(opened, true)
                .into_iter()
                .cloned()
                .collect::<Vec<_>>(),
        );
    }
    let mut diagnostics = diagnostics.lock().map_err(|_| BoardsError::Poisoned)?;
    diagnostics.insert(path.into(), stored);
    Err(BoardsError::Invalid(message))
}
pub(crate) fn catalog(
    c: &mut impl NativeProtocol,
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
            for (role, node) in values {
                match role.as_str() {
                    "material" => info.material = node.text.clone(),
                    "size" => info.size = node.text.clone(),
                    "quantity" => info.quantity = node.text.clone(),
                    "size_label" => info.size_label = node.text.clone(),
                    "project_name" => {
                        if !node.inherited {
                            info.project_name_override = Some(node.text.clone())
                        }
                    }
                    key if key.starts_with("extra:") => {
                        info.extra.insert(key[6..].into(), node.text.clone());
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
/// One owned node of a page header: its official id, its current text and, for the printed
/// project name, whether that text came from the project rather than this page.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HeaderNode {
    pub id: u64,
    pub text: String,
    pub inherited: bool,
}
pub(crate) type Fields = BTreeMap<u64, BTreeMap<String, HeaderNode>>;
/// Everything FileAtlas owns on one artboard: the text fields plus the band rectangle and,
/// in documents written before the compact band, the single group that wrapped the block.
#[derive(Clone, Debug, Default)]
pub(crate) struct OwnedNodes {
    pub fields: BTreeMap<String, HeaderNode>,
    pub band: Option<u64>,
    pub chip: Option<u64>,
    pub legacy_group: Option<u64>,
}
pub(crate) fn header_fields(c: &mut impl NativeProtocol) -> Result<Fields, BoardsError> {
    Ok(owned_by_board(c)?
        .into_iter()
        .map(|(board, owned)| (board, owned.fields))
        .collect())
}
pub(crate) fn owned(c: &mut impl NativeProtocol, board: u64) -> Result<OwnedNodes, BoardsError> {
    Ok(owned_by_board(c)?.remove(&board).unwrap_or_default())
}
fn owned_by_board(c: &mut impl NativeProtocol) -> Result<BTreeMap<u64, OwnedNodes>, BoardsError> {
    extract_owned(&c.execute("document.json", json!({}))?)
}
fn note_of(node: &Value) -> Option<Value> {
    let note = node["attrs"]["note"].as_str()?;
    let parsed = serde_json::from_str::<Value>(note).ok()?;
    (parsed["namespace"] == "fileatlas.workcrate" && parsed["version"] == 1).then_some(parsed)
}
fn extract_owned(document: &Value) -> Result<BTreeMap<u64, OwnedNodes>, BoardsError> {
    fn visit(
        node: &Value,
        out: &mut BTreeMap<u64, OwnedNodes>,
        count: &mut usize,
        depth: usize,
    ) -> Result<(), BoardsError> {
        *count += 1;
        if *count > 100000 || depth > 128 {
            return Err(BoardsError::Invalid(
                "native model exceeds traversal bounds".into(),
            ));
        }
        if let Some(note) = note_of(node) {
            let board = note["board"]
                .as_u64()
                .or_else(|| note["artboardId"].as_u64());
            let kind = node["kind"]["type"].as_str().unwrap_or_default();
            if let (Some(board), Some(role), Some(id)) =
                (board, note["role"].as_str(), node["id"].as_u64())
            {
                let owned = out.entry(board).or_default();
                match kind {
                    "text" => {
                        let runs = node["kind"]["runs"]
                            .as_array()
                            .ok_or_else(|| BoardsError::Protocol("text runs missing".into()))?;
                        let text = runs
                            .iter()
                            .filter_map(|r| r["text"].as_str())
                            .collect::<String>();
                        if owned
                            .fields
                            .insert(
                                role.into(),
                                HeaderNode {
                                    id,
                                    text,
                                    inherited: note["inherited"].as_bool().unwrap_or(false),
                                },
                            )
                            .is_some()
                        {
                            return Err(BoardsError::Invalid(
                                "duplicate owned header field".into(),
                            ));
                        }
                    }
                    "path" if role == "band" => owned.band = Some(id),
                    "path" if role == "page_number_chip" => owned.chip = Some(id),
                    "group" if role == "header" => owned.legacy_group = Some(id),
                    _ => {}
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
    let mut owned = BTreeMap::new();
    let mut count = 0;
    for layer in document["layers"]
        .as_array()
        .ok_or_else(|| BoardsError::Protocol("native layers missing".into()))?
    {
        visit(layer, &mut owned, &mut count, 0)?
    }
    Ok(owned)
}
/// The topmost official layer reserved for header nodes, created when a document has none.
pub(crate) fn header_layer(c: &mut impl NativeProtocol) -> Result<u64, BoardsError> {
    const NAME: &str = "WorkCrate header";
    let document = c.execute("document.json", json!({}))?;
    let existing = document["layers"]
        .as_array()
        .ok_or_else(|| BoardsError::Protocol("native layers missing".into()))?
        .iter()
        .filter(|layer| layer["name"].as_str() == Some(NAME))
        .filter_map(|layer| layer["id"].as_u64())
        .next_back();
    if let Some(id) = existing {
        c.execute("layer.setCurrent", json!({"id":id}))?;
        return Ok(id);
    }
    let created = c.execute(
        "layer.new",
        json!({"name":NAME,"top":true,"locked":false,"visible":true}),
    )?;
    let id = created["id"]
        .as_u64()
        .ok_or_else(|| BoardsError::Protocol("header layer ID missing".into()))?;
    c.execute("layer.setCurrent", json!({"id":id}))?;
    Ok(id)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn tagged(id: u64, kind: &str, note: Value, text: Option<&str>) -> Value {
        let mut node = json!({"id":id,"attrs":{"note":note.to_string()},"kind":{"type":kind}});
        if let Some(text) = text {
            node["kind"]["runs"] = json!([{"text":text}]);
        }
        node
    }
    #[test]
    fn actual_text_unknown_versions_and_inheritance() -> Result<(), BoardsError> {
        let value = json!({"layers":[
            tagged(1,"text",json!({"namespace":"fileatlas.workcrate","version":1,"board":7,"role":"material"}),Some("中文123")),
            tagged(2,"text",json!({"namespace":"fileatlas.workcrate","version":2,"board":7,"role":"material"}),Some("ignored")),
            tagged(3,"text",json!({"namespace":"fileatlas.workcrate","version":1,"board":7,"role":"project_name","inherited":true}),Some("全局名")),
        ]});
        let owned = extract_owned(&value)?;
        assert_eq!(owned[&7].fields["material"].text, "中文123");
        assert_eq!(owned[&7].fields["material"].inherited, false);
        assert_eq!(owned[&7].fields["project_name"].text, "全局名");
        assert_eq!(owned[&7].fields["project_name"].inherited, true);
        assert_eq!(owned[&7].band, None);
        Ok(())
    }
    #[test]
    fn band_chip_and_legacy_group_are_reported_separately() -> Result<(), BoardsError> {
        let band = tagged(
            4,
            "path",
            json!({"namespace":"fileatlas.workcrate","version":1,"board":7,"role":"band"}),
            None,
        );
        let chip = tagged(
            6,
            "path",
            json!({"namespace":"fileatlas.workcrate","version":1,"board":7,"role":"page_number_chip"}),
            None,
        );
        let legacy = tagged(
            5,
            "group",
            json!({"namespace":"fileatlas.workcrate","version":1,"board":7,"role":"header"}),
            None,
        );
        let owned = extract_owned(&json!({"layers":[band,chip,legacy]}))?;
        assert_eq!(owned[&7].band, Some(4));
        assert_eq!(owned[&7].chip, Some(6));
        assert_eq!(owned[&7].legacy_group, Some(5));
        assert!(owned[&7].fields.is_empty());
        Ok(())
    }
    #[test]
    fn legacy_group_children_stay_readable() -> Result<(), BoardsError> {
        let text = tagged(
            3,
            "text",
            json!({"namespace":"fileatlas.workcrate","version":1,"board":7,"role":"size"}),
            Some("80 * 180cm"),
        );
        let group = json!({"id":9,"kind":{"type":"group","children":[text]}});
        let owned = extract_owned(&json!({"layers":[group]}))?;
        assert_eq!(owned[&7].fields["size"].id, 3);
        Ok(())
    }
    #[test]
    fn duplicate_roles_refuse_ambiguous_mutation() {
        let node = |id: u64| {
            tagged(
                id,
                "text",
                json!({"namespace":"fileatlas.workcrate","version":1,"board":7,"role":"material"}),
                Some("A"),
            )
        };
        assert!(extract_owned(&json!({"layers":[node(1),node(2)]})).is_err());
    }
    #[test]
    fn resource_diagnostics_summarize_missing_files_and_repair_steps() {
        let report = json!({
            "title":"新建项目 1.vectorcraft",
            "missingLinks":[
                {"ids":[28],"name":"2026-10-21 20-46-04.png","path":r"\\?\F:\AI\geren\pho_cordis\docs\2026-10-21 20-46-04.png"},
                {"path":r"\\?\F:\素材\cover.jpg"},
            ],
            "modifiedLinks":[],
            "warnings":[],
        });
        let message = describe_open_report(&report, false);
        assert!(
            message.contains("资源文件缺失：2026-10-21 20-46-04.png、cover.jpg"),
            "{message}"
        );
        assert!(message.contains("关联工程"), "{message}");
        assert!(
            !message.contains("missingLinks"),
            "raw report leaked: {message}"
        );
        assert!(
            !message.contains(r"\\?\"),
            "internal path form leaked: {message}"
        );
    }
    #[test]
    fn sandboxed_diagnostics_hide_links_that_exist_on_disk() {
        let existing = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
        let report = json!({
            "missingLinks":[
                {"name":"present.png","path":existing},
                {"name":"gone.png","path":r"\\?\F:\absent\gone.png"},
            ],
            "modifiedLinks":[],
            "warnings":[],
        });
        let message = describe_open_report(&report, true);
        assert!(message.contains("资源文件缺失：gone.png"), "{message}");
        assert!(
            !message.contains("present.png"),
            "a sandbox artifact must not be shown as missing: {message}"
        );
        let live = describe_open_report(&report, false);
        assert!(
            live.contains("present.png"),
            "a live report keeps every entry: {live}"
        );
    }
    #[test]
    fn resource_diagnostics_count_link_changes_and_warnings() {
        let report = json!({
            "missingLinks":[],
            "modifiedLinks":[{"path":"a.png"}],
            "warnings":["one","two"],
        });
        let message = describe_open_report(&report, false);
        assert!(message.contains("改写了 1 处资源链接"), "{message}");
        assert!(message.contains("2 条资源警告"), "{message}");
        assert!(message.contains("关联工程"), "{message}");
    }
}
