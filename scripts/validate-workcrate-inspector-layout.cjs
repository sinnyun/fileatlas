// WorkCrate inspector cards and field-save latency: same-process FileAtlas MCP scenario.
//
// Proves that the 页面信息/页面内容 cards render in place, that opening and committing a
// field draft never reflows the panel, and that cold field saves warm the official session
// silently (the page stays in preview) while later saves reuse it with no whole-project
// reload. Latencies are recorded for the validation note.
//
// node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe \
//   --profile .local/validation/profiles/wc-inspector-layout --access-level full \
//   --scenario scripts/validate-workcrate-inspector-layout.cjs
const assert = require("node:assert/strict");
const fs = require("node:fs");
const net = require("node:net");
const path = require("node:path");

const EXE = "C:\\Program Files\\VectorCraft\\vectorcraft.exe";

function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const port = server.address().port;
      server.close(() => resolve(port));
    });
  });
}

function rect(value) {
  const [x, y, width, height] = String(value).split(",").map(Number);
  assert.ok(
    [x, y, width, height].every((n) => Number.isFinite(n) && n >= 0),
    `bad control rectangle: ${value}`,
  );
  return { x, y, width, height };
}

module.exports = async ({ call, step, payload, delay, profile }) => {
  const run = String(Date.now());
  const documents = path.join(profile, "documents", run);
  fs.mkdirSync(documents, { recursive: true });

  const read = async (ids = []) => payload(await call("workcrate-ui.state", { control_ids: ids }));
  const matches = (predicate, view) => {
    try {
      return Boolean(predicate(view));
    } catch {
      return false;
    }
  };
  const until = async (predicate, what, ids = [], attempts = 80) => {
    let last = null;
    for (let attempt = 0; attempt < attempts; attempt += 1) {
      const view = await read(ids);
      last = view;
      if (matches(predicate, view)) return view;
      await delay(250);
    }
    throw Error(
      `timed out waiting for ${what}; last panel state ${JSON.stringify({ busy: last?.ui?.busy, editing: last?.ui?.editing, saves: last?.ui?.field_saves, message: last?.ui?.message })}`,
    );
  };
  const pointer = async (action, value) => {
    for (let attempt = 0; attempt < 30; attempt += 1) {
      try {
        await step({ tool: "control_ui", arguments: { action, value } });
        return;
      } catch (error) {
        if (!String(error.message).includes("still pending")) throw error;
        await delay(250);
      }
    }
    throw Error(`the MCP pointer queue never drained for ${action}`);
  };
  const clickId = async (id) => {
    const view = await until((v) => v.controls?.[id], `the geometry of ${id}`, [id]);
    const box = rect(view.controls[id]);
    await pointer("pointer_down", `${box.x + box.width / 2},${box.y + box.height / 2}`);
    await pointer("pointer_up", `${box.x + box.width / 2},${box.y + box.height / 2}`);
    await delay(200);
    return box;
  };
  const cardOf = (view, id) => view.ui.boards.find((b) => b.id === id);
  const idle = () => until((v) => !v.ui.busy && !v.ui.canvas.busy, "an idle panel");
  const draft = async (key, text, submit) => {
    const target = `${board.id}#${key}`;
    await clickId(`field-${key}`);
    await until((v) => v.ui.editing === target, `the draft row of ${key}`);
    await clickId("field-edit-input");
    await pointer("key_press", "ctrl-a");
    await pointer("input_text", text);
    const started = Date.now();
    if (submit === "enter") await pointer("key_press", "enter");
    else await clickId("field-edit-save");
    const done = await until(
      (v) => !v.ui.field_saves.length && !v.ui.editing && /^已保存/.test(String(v.ui.message || "")),
      `the save feedback of ${key}`,
      ["field-size", "page-add-field"],
      80,
    );
    return { view: done, ms: Date.now() - started };
  };

  // 1. A free port forces the silent official session: the user's own GUI is never touched.
  const port = await freePort();
  await step({ tool: "workcrate-ui.configure_vectorcraft", arguments: { executable: EXE, port } });
  const created = await step({
    tool: "workcrate-ui.manage_project",
    arguments: { action: "create", name: `检查器布局${run}`, directory: documents },
  });
  const project = created.result.project;
  const board = created.result.boards[0];
  assert.ok(project?.id && board?.id, "the project and its page must exist");

  // A second page is the untouched control for the one-card patch.
  const second = await step({ tool: "workcrate-ui.create_page", arguments: { project_id: project.id } });
  const other = second.result.board_id;
  await step({ tool: "workcrate-ui.open_board", arguments: { project_id: project.id, board_id: other } });
  await until((v) => v.ui.preview_pending === 0, "the second page preview");
  await step({ tool: "workcrate-ui.open_board", arguments: { project_id: project.id, board_id: board.id } });
  const ready = await until((v) => v.ui.preview_pending === 0, "the first page preview");
  assert.equal(ready.ui.canvas.state, null, "browsing must not open an official session");

  // 2. Both cards paint, with the fixed inspector width.
  const controls = [
    "inspector",
    "field-title",
    "field-physical",
    "field-local",
    "field-material",
    "field-size",
    "field-quantity",
    "page-add-field",
  ];
  const layout = await until((v) => controls.every((id) => v.controls?.[id]), "the inspector cards", controls);
  const inspector = rect(layout.controls["inspector"]);
  assert.ok(inspector.width > 250, `the inspector must keep its fixed width: ${inspector.width}`);
  const rows = ["field-title", "field-physical", "field-local", "field-material", "field-size", "field-quantity"].map(
    (id) => rect(layout.controls[id]),
  );
  for (const row of rows) {
    assert.ok(row.x > inspector.x && row.x + row.width <= inspector.x + inspector.width, "a field row stays inside the inspector");
  }
  const materialRow = rect(layout.controls["field-material"]);
  await step({ tool: "workcrate-ui.state", arguments: {}, capture: "inspector-cards-before" });

  // 3. Editing opens in the same row: no stacked label, no height jump.
  await clickId("field-material");
  const editing = await until(
    (v) => v.ui.editing === `${board.id}#material`,
    "the material draft",
    ["field-edit-input", "field-edit-save", "inspector", "field-size"],
  );
  const input = rect(editing.controls["field-edit-input"]);
  assert.ok(Math.abs(input.y - materialRow.y) <= 2, `the draft keeps the value row's y (${input.y} vs ${materialRow.y})`);
  assert.ok(Math.abs(input.height - materialRow.height) <= 8, "the draft keeps one row height");
  assert.equal(rect(editing.controls["inspector"]).height, inspector.height, "opening a draft must not resize the inspector");
  assert.equal(rect(editing.controls["field-size"]).y, rows[4].y, "opening a draft must not move the rows below");
  await clickId("field-edit-input");
  await pointer("key_press", "ctrl-a");
  await pointer("input_text", "瓦楞纸板 A/B");
  await step({ tool: "workcrate-ui.state", arguments: {}, capture: "inspector-draft-inline" });

  // 4. Enter commits while the panel stays usable. This is a cold save: no official session
  //    exists yet, so it opens the silent one and hands its frame straight to the page.
  const coldStart = Date.now();
  await pointer("key_press", "enter");
  let markerSeen = false;
  for (let attempt = 0; attempt < 10; attempt += 1) {
    const view = await read();
    if (view.ui.field_saves.some((entry) => entry.endsWith("#material"))) {
      markerSeen = true;
      assert.equal(view.ui.busy, false, "a field save must not block the whole panel");
    }
    if (!view.ui.field_saves.length && /^已保存/.test(String(view.ui.message || ""))) break;
    await delay(100);
  }
  const cold = await until(
    (v) =>
      !v.ui.field_saves.length &&
      !v.ui.editing &&
      /^已保存：物料名称/.test(String(v.ui.message || "")),
    "the cold material save",
    ["field-material", "field-size", "inspector", "page-add-field"],
  );
  const coldMs = Date.now() - coldStart;
  const coldCard = cardOf(cold, board.id);
  assert.equal(coldCard.info.material, "瓦楞纸板 A/B", "the cold save must reach the card");
  assert.equal(cold.ui.busy, false, "a cold save must not perform a whole-project reload");
  assert.equal(cardOf(cold, other).preview_ready, true, "the other page keeps its preview");
  assert.ok(cold.ui.canvas.state, "the cold save must warm the official session");
  assert.equal(cold.ui.canvas.state.board_id, board.id, "the warmed session must stay on this page");
  assert.equal(cold.ui.canvas.mode, "preview", "warming a save must not switch the page into editing");
  assert.equal(cold.ui.canvas.state.live, false, "an unused control port must run the headless session");
  assert.equal(coldCard.canvas_revision, cold.ui.canvas.state.revision, "the card shows the frame the save rendered");
  assert.equal(cold.ui.preview_pending, 0, "a saved page must not be queued for a second render");
  assert.equal(rect(cold.controls["field-size"]).y, rows[4].y, "committing a draft must not move the rows below");
  assert.equal(rect(cold.controls["inspector"]).height, inspector.height, "committing a draft must not resize the inspector");
  await step({ tool: "workcrate-ui.state", arguments: {}, capture: "inspector-cards-after-first-save" });

  // 5. The next saves reuse the warm session and are measurably faster.
  const quantity = await draft("quantity", "20套", "click");
  assert.equal(cardOf(quantity.view, board.id).info.quantity, "20套");
  assert.equal(quantity.view.ui.canvas.state.board_id, board.id, "a warm save keeps the session");
  assert.equal(quantity.view.ui.busy, false, "a warm save must not perform a whole-project reload");

  const size = await draft("size", "300×200", "enter");
  assert.equal(cardOf(size.view, board.id).info.size, "300×200");

  // 6. The card's own add row still creates an extra parameter, then fills it.
  await clickId("page-add-field");
  await until((v) => v.ui.editing === `${board.id}#new`, "the new-parameter draft");
  await clickId("field-edit-input");
  await pointer("key_press", "ctrl-a");
  await pointer("input_text", "工艺");
  await pointer("key_press", "enter");
  const added = await until(
    (v) => !v.ui.field_saves.length && !v.ui.editing && "工艺" in cardOf(v, board.id).info.extra,
    "the new empty parameter",
  );
  assert.match(added.ui.message, /^已保存：字段 工艺/);
  const extra = await draft("extra:工艺", "模切", "enter");
  assert.equal(cardOf(extra.view, board.id).info.extra["工艺"], "模切");

  const final = await until(
    (v) => !v.ui.busy && !v.ui.canvas.busy && !v.ui.field_saves.length,
    "the settled inspector",
    ["inspector", "field-material", "field-size"],
  );
  assert.equal(rect(final.controls["inspector"]).height, inspector.height, "saved extras must not resize the inspector");
  await step({ tool: "workcrate-ui.state", arguments: {}, capture: "inspector-cards-final" });

  // 7. Quiet state: the hidden warm session must not repaint its own snapshot.
  await delay(3000);
  const settled = JSON.stringify((await read()).ui);
  await delay(2500);
  assert.equal(JSON.stringify((await read()).ui), settled, "an idle panel must not repaint its snapshot");

  const result = {
    passed: true,
    port,
    project: project.id,
    board: board.id,
    other,
    inspectorWidth: inspector.width,
    coldSaveMs: coldMs,
    quantitySaveMs: quantity.ms,
    sizeSaveMs: size.ms,
    savingMarkerSeen: markerSeen,
  };
  fs.writeFileSync(path.join(profile, "inspector-layout-result.json"), JSON.stringify(result, null, 2));
  console.log(JSON.stringify({ inspectorLayout: "passed", ...result }));
};
