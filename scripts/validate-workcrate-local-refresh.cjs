// WorkCrate localized refresh: same-process FileAtlas MCP scenario.
//
// Proves that ordinary work done while the page editor is open patches only what it
// changed. A whole-catalogue reload moves `ui.reloads`; every step below (field save,
// project-index wake, linked placement, relink, page switch) must leave that counter,
// the official `session_id`, the card list and the untouched page's preview alone.
// Creating a page is a genuine structure change and must still reload once.
//
// node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe \
//   --profile .local/validation/profiles/wc-local-refresh --access-level full \
//   --scenario scripts/validate-workcrate-local-refresh.cjs
const assert = require("node:assert/strict");
const fs = require("node:fs");
const net = require("node:net");
const path = require("node:path");

const EXE = "C:\\Program Files\\VectorCraft\\vectorcraft.exe";
const FIXTURE = path.join(
  "test-data",
  "fileatlas-fixtures",
  "03-nested",
  "level-one",
  "level-two",
  "nested-photo.jpeg",
);

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

module.exports = async ({ call, payload, step, delay, profile }) => {
  const run = String(Date.now());
  const documents = path.join(profile, "documents", run);
  // Placed files live inside the document folder, so the warm official session keeps its
  // access roots and a placement never restarts it.
  const assets = path.join(documents, "assets");
  fs.mkdirSync(assets, { recursive: true });
  const first = path.join(assets, "placed.jpeg");
  const second = path.join(assets, "replacement.jpeg");
  fs.copyFileSync(path.resolve(FIXTURE), first);
  fs.copyFileSync(path.resolve(FIXTURE), second);

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
      `timed out waiting for ${what}; last panel state ${JSON.stringify({
        screen: last?.ui?.screen,
        busy: last?.ui?.busy,
        page_count: last?.ui?.page_count,
        previews_ready: last?.ui?.previews_ready,
        preview_pending: last?.ui?.preview_pending,
        mode: last?.ui?.canvas?.mode,
        canvas_state: Boolean(last?.ui?.canvas?.state),
        reloads: last?.ui?.reloads,
        saves: last?.ui?.field_saves,
        message: last?.ui?.message,
        controls: Object.keys(last?.controls || {}),
      })}`,
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
    const view = await until(
      (v) => v.controls?.[id] && !v.ui?.busy && !v.ui?.canvas?.busy,
      `the clickable geometry of ${id}`,
      [id],
    );
    const box = rect(view.controls[id]);
    await pointer("pointer_down", `${box.x + box.width / 2},${box.y + box.height / 2}`);
    await pointer("pointer_up", `${box.x + box.width / 2},${box.y + box.height / 2}`);
    await delay(250);
  };
  const idle = (v) => !v.ui?.busy && !v.ui?.canvas?.busy && !v.ui?.field_saves?.length;
  const cardOf = (view, id) => view.ui.boards.find((b) => b.id === id);
  // A localized update: no catalogue reload, same official connection, same card list.
  const localized = (view, base, what) => {
    assert.equal(view.ui.reloads, base.reloads, `${what} must not reload the catalogue`);
    assert.equal(
      view.ui.canvas?.state?.session_id,
      base.session,
      `${what} must keep the official connection`,
    );
    assert.deepEqual(
      view.ui.boards.map((b) => b.id),
      base.ids,
      `${what} must keep every page card`,
    );
    assert.equal(
      cardOf(view, base.other).preview_ready,
      true,
      `${what} must keep the untouched page's preview`,
    );
  };
  const draft = async (key, text) => {
    await clickId(`field-${key}`);
    await until((v) => v.ui.editing === `${board}#${key}`, `the ${key} draft`);
    await clickId("field-edit-input");
    await pointer("key_press", "ctrl-a");
    await pointer("input_text", text);
    await pointer("key_press", "enter");
    return until(
      (v) =>
        idle(v) &&
        !v.ui.editing &&
        /^已保存/.test(String(v.ui.message || "")),
      `the ${key} save`,
      ["field-material"],
    );
  };

  // 1. One fresh project with a native document and a second page as the control card.
  const port = await freePort();
  await step({ tool: "workcrate-ui.configure_vectorcraft", arguments: { executable: EXE, port } });
  const created = await step({
    tool: "workcrate-ui.manage_project",
    arguments: { action: "create", name: `局部刷新${run}`, directory: documents },
  });
  const project = created.result.project;
  const board = created.result.boards[0].id;
  assert.ok(project?.id && board, "the project and its native page must exist");
  const extra = await step({ tool: "workcrate-ui.create_page", arguments: { project_id: project.id } });
  const other = extra.result.board_id;
  await step({ tool: "workcrate-ui.open_board", arguments: { project_id: project.id, board_id: other } });
  await until((v) => v.ui?.preview_pending === 0, "the control page preview");
  await step({ tool: "workcrate-ui.open_board", arguments: { project_id: project.id, board_id: board } });
  await until(
    (v) => idle(v) && v.ui?.screen === "editor" && v.ui?.preview_pending === 0,
    "the editor preview screen",
    ["field-material", "workcrate-canvas-edit"],
  );

  // 2. A cold field save warms the official session in preview mode without a reload.
  const beforeSave = await read();
  const saved = await draft("material", "瓦楞纸板");
  assert.equal(cardOf(saved, board).info.material, "瓦楞纸板");
  assert.equal(saved.ui.reloads, beforeSave.ui.reloads, "a field save must not reload the catalogue");
  assert.ok(saved.ui.canvas?.state, "the cold save must warm the official session");
  assert.equal(saved.ui.canvas.mode, "preview", "a save must not switch the screen into editing");
  const session = saved.ui.canvas.state.session_id;

  // 3. The edit screen reuses that same official connection.
  await clickId("workcrate-canvas-edit");
  const editing = await until(
    (v) => v.ui?.canvas?.mode === "edit" && v.ui?.canvas?.state && idle(v),
    "the edit screen over the warm session",
  );
  assert.equal(editing.ui.canvas.state.session_id, session, "enter editing must reuse the session");
  assert.equal(editing.ui.canvas.state.board_id, board);
  const added = await step({
    tool: "workcrate-ui.canvas_create_text",
    arguments: {
      x: editing.ui.canvas.state.artboard.x + 120,
      y: editing.ui.canvas.state.artboard.y + 90,
      text: "局部刷新验证",
      size: 24,
      confirm: true,
    },
  });
  const textId = added.result.canvas.selection[0];
  const base = await until(
    (v) => idle(v) && v.ui?.canvas?.state?.revision === added.result.canvas.revision,
    "the panel to adopt the official text",
  );
  const start = {
    reloads: base.ui.reloads,
    session,
    ids: base.ui.boards.map((b) => b.id),
    other,
  };
  await step({ tool: "workcrate-ui.state", arguments: {}, capture: "local-refresh-editor" });

  // 4. A project-index wake (rename) patches the list; it must not rebuild the editor.
  await step({
    tool: "workcrate-ui.manage_project",
    arguments: { action: "rename", project_id: project.id, name: `局部刷新改名${run}` },
  });
  await delay(900);
  localized(await until((v) => idle(v), "the settled panel after a rename"), start, "a project rename");

  // 5. A linked placement adds a project reference, which wakes the project index too.
  const centre = [
    base.ui.canvas.state.artboard.x + base.ui.canvas.state.artboard.width / 2,
    base.ui.canvas.state.artboard.y + base.ui.canvas.state.artboard.height / 2,
  ];
  const placed = await step({
    tool: "workcrate-ui.canvas_place",
    arguments: { path: first, at: [centre[0], centre[1] + 120], link: true, confirm: true },
  });
  const imageId = placed.result.canvas.selection[0];
  const afterPlace = await until(
    (v) => idle(v) && v.ui?.canvas?.state?.revision === placed.result.canvas.revision,
    "the panel to adopt the placed image",
  );
  assert.equal(afterPlace.ui.canvas.object?.id, imageId);
  localized(afterPlace, start, "a linked placement");

  // 6. Relinking registers another reference and still patches the same frame.
  const relinked = await step({
    tool: "workcrate-ui.canvas_links",
    arguments: { action: "relink", ids: [imageId], path: second, confirm: true },
  });
  const afterRelink = await until(
    (v) => idle(v) && v.ui?.canvas?.state?.revision === relinked.result.canvas.revision,
    "the panel to adopt the relinked image",
  );
  localized(afterRelink, start, "a relink");
  await step({ tool: "workcrate-ui.state", arguments: {}, capture: "local-refresh-after-link" });

  // 7. Switching pages inside the editor moves the artboard, not the panel.
  await step({ tool: "workcrate-ui.open_board", arguments: { project_id: project.id, board_id: other } });
  const pageTwo = await until(
    (v) => idle(v) && v.ui?.canvas?.state?.board_id === other,
    "the editor on the control page",
  );
  localized(pageTwo, start, "a page switch");
  assert.equal(pageTwo.ui.canvas.mode, "edit", "a page switch keeps the edit screen");
  await step({ tool: "workcrate-ui.open_board", arguments: { project_id: project.id, board_id: board } });
  const pageOne = await until(
    (v) => idle(v) && v.ui?.canvas?.state?.board_id === board,
    "the editor back on the first page",
  );
  localized(pageOne, start, "a switch back");

  // 8. A warm save from the preview screen patches one card and keeps the session.
  await clickId("workcrate-canvas-preview");
  await until((v) => idle(v) && v.ui?.canvas?.mode === "preview", "the preview screen");
  await step({ tool: "workcrate-ui.state", arguments: {}, capture: "local-refresh-preview" });
  await until(
    (v) => idle(v) && v.controls?.["field-material"] && v.ui?.canvas?.mode === "preview",
    "the page-info inspector",
    ["field-material"],
  );
  const warmSaved = await draft("material", "灰板纸");
  assert.equal(cardOf(warmSaved, board).info.material, "灰板纸");
  localized(warmSaved, start, "a warm field save");
  assert.equal(warmSaved.ui.canvas.state.session_id, session, "a warm save keeps the connection");

  // 9. Sanity: a real structure change (create page) still reloads the catalogue once.
  const beforeCreate = warmSaved.ui.reloads;
  await step({ tool: "workcrate-ui.create_page", arguments: { project_id: project.id } });
  const grown = await until(
    (v) => idle(v) && v.ui?.canvas?.state === null && v.ui?.page_count === 3,
    "the catalogue reload after a page was created",
  );
  assert.ok(grown.ui.reloads > beforeCreate, "creating a page must reload the catalogue");
  assert.equal(grown.ui.canvas.state, null, "a structure change closes the stale session");

  // 10. Quiet state: the settled panel does not keep repainting its own snapshot.
  await delay(3000);
  const settled = JSON.stringify((await read()).ui);
  await delay(2500);
  assert.equal(JSON.stringify((await read()).ui), settled, "an idle panel must not repaint its snapshot");

  const result = {
    passed: true,
    project: project.id,
    board,
    other,
    textId,
    imageId,
    reloadsBeforeCreate: beforeCreate,
    reloadsAfterCreate: grown.ui.reloads,
  };
  fs.writeFileSync(path.join(profile, "local-refresh-result.json"), JSON.stringify(result, null, 2));
  console.log(JSON.stringify({ localRefresh: "passed", ...result }));
};
