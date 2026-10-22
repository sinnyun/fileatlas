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
                    "project_name" => {
                        if !node.inherited {
                            info.material_category = node
                                .note
                                .get("value")
                                .and_then(Value::as_str)
                                .unwrap_or(&node.text)
                                .to_owned()
                        }
                    }
                    key if key.starts_with("extra:") => {
                        info.extra.insert(key[6..].into(), node.text.clone());
                    }
                    _ => {}
                }
            }
        }
        if let Some(details) = fields
            .get(&native_id)
            .and_then(|fields| fields.get("details"))
        {
            crate::header::Details::read(&details.note, &mut info)?;
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
    pub note: Value,
}
pub(crate) type Fields = BTreeMap<u64, BTreeMap<String, HeaderNode>>;
/// Everything FileAtlas owns on one artboard: the text fields plus the band rectangle and
/// the groups around them. `page_group` wraps this page's whole material band inside the
/// document-wide master; `content_group` holds the artwork the user added to this page.
#[derive(Clone, Debug, Default)]
pub(crate) struct OwnedNodes {
    pub fields: BTreeMap<String, HeaderNode>,
    pub band: Option<u64>,
    pub chip: Option<u64>,
    pub legacy_group: Option<u64>,
    pub page_group: Option<u64>,
    pub layout_version: u32,
    pub content_group: Option<u64>,
}
/// One document-wide scan: every artboard's owned nodes plus the master group that wraps
/// all pages' material bands (locked so only FileAtlas can edit it).
#[derive(Clone, Debug, Default)]
pub(crate) struct OwnedLayout {
    pub boards: BTreeMap<u64, OwnedNodes>,
    pub header_root: Option<u64>,
}
pub(crate) fn header_fields(c: &mut impl NativeProtocol) -> Result<Fields, BoardsError> {
    Ok(owned_layout(c)?
        .boards
        .into_iter()
        .map(|(board, owned)| (board, owned.fields))
        .collect())
}
pub(crate) fn owned(c: &mut impl NativeProtocol, board: u64) -> Result<OwnedNodes, BoardsError> {
    Ok(owned_layout(c)?.boards.remove(&board).unwrap_or_default())
}
/// Mark one node as FileAtlas-owned. `board` scopes the node to one artboard; the
/// document-wide master group carries no board. `inherited` only annotates the printed
/// project name.
pub(crate) fn tag_node(
    c: &mut impl NativeProtocol,
    id: u64,
    role: &str,
    board: Option<u64>,
    inherited: bool,
) -> Result<(), BoardsError> {
    let mut note = json!({"namespace":"fileatlas.workcrate","version":1,"role":role});
    if let Some(board) = board {
        note["board"] = json!(board);
    }
    if inherited {
        note["inherited"] = json!(true);
    }
    let written = c.execute("attributes.set", json!({"id":id,"note":note.to_string()}))?;
    if written["changed"].as_u64() != Some(1) {
        return Err(BoardsError::Protocol(
            "官方程序未能记录该节点的归属，请重新同步页面后再保存".into(),
        ));
    }
    Ok(())
}
/// Move freshly created content (a placed image, a new text) into the page's content
/// group, creating that group from the first node the page receives.
pub(crate) fn group_content(
    c: &mut impl NativeProtocol,
    board: u64,
    created: &[u64],
) -> Result<(), BoardsError> {
    if created.is_empty() {
        return Ok(());
    }
    let group = owned(c, board)?.content_group;
    match group {
        Some(group) => {
            for node in created {
                c.execute("node.move", json!({"id":node,"parent":group}))?;
            }
        }
        None => {
            let created_group = c.execute("object.group", json!({"ids":created}))?;
            let id = created_group["id"]
                .as_u64()
                .ok_or_else(|| BoardsError::Protocol("官方程序未创建页面内容组，请重试".into()))?;
            tag_node(c, id, "content_root", Some(board), false)?;
        }
    }
    // Grouping and moving deselect everything, so restore the created ids: the overlay's
    // object panel must keep showing the node the user just made.
    c.execute("select.set", json!({"ids":created}))?;
    Ok(())
}
/// Re-point an official click selection at the layer under the pointer. The official hit
/// test reports the outermost group, so a click on grouped content would select and drag
/// the whole page's content as one piece; FileAtlas walks the group's children instead and
/// selects the deepest one under the click. A click that landed inside a locked group —
/// the material band's master and its subgroups — selects nothing: the panel owns those.
pub(crate) fn drill_click(
    c: &mut impl NativeProtocol,
    point: (f64, f64),
) -> Result<(), BoardsError> {
    // `document.json` carries no bounds; the inspect layer tree does, together with the
    // selection this click produced.
    let inspect = c.request("document.inspect", json!({"depth":16}))?;
    let selected = inspect["selection"]
        .as_array()
        .and_then(|ids| match ids.as_slice() {
            [one] => one.as_u64(),
            _ => None,
        });
    let Some(id) = selected else {
        return Ok(());
    };
    let layers = inspect["layers"]
        .as_array()
        .ok_or_else(|| BoardsError::Protocol("native layer tree missing".into()))?;
    let Some((node, enclosed)) = locate(layers, id, false) else {
        return Ok(());
    };
    if enclosed {
        c.execute("select.none", json!({}))?;
        return Ok(());
    }
    if !node["kind"]
        .as_str()
        .is_some_and(|kind| kind.eq_ignore_ascii_case("group"))
    {
        return Ok(());
    }
    if let Some(child) = deepest_at(node, point)
        && child != id
    {
        c.execute("select.set", json!({"ids":[child]}))?;
    }
    Ok(())
}
/// Execute one drag whose press lands at `down` and whose pointer ends `delta` away,
/// with `original` as the raw event batch. The official hit test re-selects the
/// outermost group at the press, so forwarding the batch would move the whole page's
/// content group; FileAtlas moves the child the overlay shows instead, picks the child
/// under the press when the drag starts elsewhere, and forwards only the batches the
/// official engine already handles right (a marquee from empty paper, a shown group).
pub(crate) fn run_drag(
    c: &mut impl NativeProtocol,
    down: (f64, f64),
    delta: (f64, f64),
    original: &Value,
) -> Result<(), BoardsError> {
    let inspect = c.request("document.inspect", json!({"depth":16}))?;
    let layers = inspect["layers"]
        .as_array()
        .ok_or_else(|| BoardsError::Protocol("native layer tree missing".into()))?;
    let selection = selection_ids(&inspect);
    let shown = bounds_contains(&inspect["selectionBounds"], down);
    if shown && !selection.is_empty() {
        let whole = match selection.as_slice() {
            [one] => locate(layers, *one, false).is_some_and(|(node, locked)| {
                !locked
                    && node["kind"]
                        .as_str()
                        .is_some_and(|k| k.eq_ignore_ascii_case("group"))
            }),
            _ => true,
        };
        if whole {
            c.request("pointer_gesture", json!({"events":original}))?;
            return Ok(());
        }
        if let [one] = selection.as_slice() {
            let Some((_, locked)) = locate(layers, *one, false) else {
                return Ok(());
            };
            if !locked {
                move_node(c, *one, delta)?;
            }
        }
        return Ok(());
    }
    // The drag starts elsewhere: pick what the press lands on with a synthetic click.
    c.request(
        "pointer_gesture",
        json!({"events":[{"kind":"down","x":down.0,"y":down.1},{"kind":"up","x":down.0,"y":down.1}]}),
    )?;
    let picked = c.request("document.inspect", json!({"depth":16}))?;
    let picked_layers = picked["layers"]
        .as_array()
        .ok_or_else(|| BoardsError::Protocol("native layer tree missing".into()))?;
    match selection_ids(&picked).as_slice() {
        // Empty paper: the drag was a marquee, which the official engine runs itself.
        [] => {
            c.request("pointer_gesture", json!({"events":original}))?;
        }
        [one] => {
            let Some((node, locked)) = locate(picked_layers, *one, false) else {
                return Ok(());
            };
            if locked {
                // The press landed in the material band: the panel owns it.
                return Ok(());
            }
            let target = match node["kind"]
                .as_str()
                .is_some_and(|k| k.eq_ignore_ascii_case("group"))
            {
                true => deepest_at(node, down).unwrap_or(*one),
                false => *one,
            };
            c.execute(
                "object.move",
                json!({"ids":[target],"dx":delta.0,"dy":delta.1}),
            )?;
            c.execute("select.set", json!({"ids":[target]}))?;
        }
        _ => {}
    }
    Ok(())
}
/// Move one node by a pointer delta, keeping it selected for the overlay's object panel.
fn move_node(c: &mut impl NativeProtocol, id: u64, delta: (f64, f64)) -> Result<(), BoardsError> {
    c.execute("object.move", json!({"ids":[id],"dx":delta.0,"dy":delta.1}))?;
    c.execute("select.set", json!({"ids":[id]}))?;
    Ok(())
}
fn selection_ids(inspect: &Value) -> Vec<u64> {
    inspect["selection"]
        .as_array()
        .map(|ids| ids.iter().filter_map(Value::as_u64).collect())
        .unwrap_or_default()
}
fn bounds_contains(bounds: &Value, point: (f64, f64)) -> bool {
    let (Some(x), Some(y), Some(width), Some(height)) = (
        bounds["x"].as_f64(),
        bounds["y"].as_f64(),
        bounds["width"].as_f64(),
        bounds["height"].as_f64(),
    ) else {
        return false;
    };
    point.0 >= x && point.0 <= x + width && point.1 >= y && point.1 <= y + height
}
/// Find `id` in the official inspect layer tree, reporting whether a locked group
/// encloses it.
fn locate(layers: &[Value], id: u64, locked: bool) -> Option<(&Value, bool)> {
    for layer in layers {
        let here = locked || layer["locked"].as_bool().unwrap_or(false);
        if layer["id"].as_u64() == Some(id) {
            return Some((layer, here));
        }
        if let Some(found) = locate(
            layer["children"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            id,
            here,
        ) {
            return Some(found);
        }
    }
    None
}
/// The deepest child of `node` whose bounds contain `point`, topmost first; document
/// order is z-order, so later children sit above earlier ones.
fn deepest_at(node: &Value, point: (f64, f64)) -> Option<u64> {
    let children = node["children"].as_array()?;
    children.iter().rev().find_map(|child| {
        let bounds = &child["bounds"];
        let (x, y, width, height) = (
            bounds["x"].as_f64()?,
            bounds["y"].as_f64()?,
            bounds["width"].as_f64()?,
            bounds["height"].as_f64()?,
        );
        let inside = point.0 >= x && point.0 <= x + width && point.1 >= y && point.1 <= y + height;
        if !inside {
            return None;
        }
        child["id"].as_u64().map(|id| {
            match child["kind"]
                .as_str()
                .is_some_and(|k| k.eq_ignore_ascii_case("group"))
            {
                true => deepest_at(child, point).unwrap_or(id),
                false => id,
            }
        })
    })
}
pub(crate) fn owned_layout(c: &mut impl NativeProtocol) -> Result<OwnedLayout, BoardsError> {
    extract_owned(&c.execute("document.json", json!({}))?)
}
fn note_of(node: &Value) -> Option<Value> {
    let note = node["attrs"]["note"].as_str()?;
    let parsed = serde_json::from_str::<Value>(note).ok()?;
    (parsed["namespace"] == "fileatlas.workcrate" && parsed["version"] == 1).then_some(parsed)
}
fn extract_owned(document: &Value) -> Result<OwnedLayout, BoardsError> {
    fn visit(
        node: &Value,
        out: &mut OwnedLayout,
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
                let owned = out.boards.entry(board).or_default();
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
                                    note: note.clone(),
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
                    "group" if role == "header_page" => {
                        owned.page_group = Some(id);
                        owned.layout_version = note["layout"].as_u64().unwrap_or(0) as u32;
                    }
                    "group" if role == "content_root" => owned.content_group = Some(id),
                    _ => {}
                }
            }
            // The master group wraps every page's band, so it carries no board id.
            if note["role"] == "header_root"
                && let Some(id) = node["id"].as_u64()
            {
                out.header_root = Some(id);
            }
        }
        if let Some(children) = node["kind"]["children"].as_array() {
            for child in children {
                visit(child, out, count, depth + 1)?
            }
        }
        Ok(())
    }
    let mut out = OwnedLayout::default();
    let mut count = 0;
    for layer in document["layers"]
        .as_array()
        .ok_or_else(|| BoardsError::Protocol("native layers missing".into()))?
    {
        visit(layer, &mut out, &mut count, 0)?
    }
    Ok(out)
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
mod tests;
