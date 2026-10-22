// WorkCrate material header + page-field saves: same-process FileAtlas MCP scenario.
//
// Proves that an inspector field save reuses the explicitly opened official session,
// re-renders only that page, patches only that card, keeps the draft on failure, writes the
// compact header band into the native document, and leaves every other page's cached preview
// file untouched.
//
// node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe \
//   --profile .local/validation/profiles/wc-header-fields --access-level full \
//   --scenario scripts/validate-workcrate-field-header.cjs
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

function walk(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full, out);
    else out.push(full);
  }
  return out;
}

module.exports = async ({ call, step, payload, delay, profile }) => {
  const run = String(Date.now());
  const documents = path.join(profile, "documents", run);
  fs.mkdirSync(documents, { recursive: true });
  const documentFile = () => {
    const names = fs.readdirSync(documents).filter((name) => name.endsWith(".vectorcraft"));
    assert.equal(names.length, 1, `one native document must exist: ${names.join(", ")}`);
    return path.join(documents, names[0]);
  };
  /// The durable page catalogue of the given page, plus the folder its previews live in.
  const cacheFor = (boardId) => {
    const key = `${boardId}@1600`;
    const file = walk(profile)
      .filter((name) => name.endsWith("boards-cache.json"))
      .find((name) => fs.readFileSync(name, "utf8").includes(key));
    assert.ok(file, `the page catalogue of ${boardId} must be cached`);
    return { cache: JSON.parse(fs.readFileSync(file, "utf8")), directory: path.dirname(file) };
  };
  const previewOf = (boardId) => {
    const { cache, directory } = cacheFor(boardId);
    const relative = cache.previews[`${boardId}@1600`];
    assert.ok(relative, `${boardId} must have a cached frame`);
    return { relative, absolute: path.join(directory, relative) };
  };

  const read = async (ids = []) => payload(await call("workcrate-ui.state", { control_ids: ids }));
  const matches = (predicate, view) => {
    try {
      return Boolean(predicate(view));
    } catch {
      // The panel has not rendered this state yet: keep waiting instead of failing the run.
      return false;
    }
  };
  const snapshot = (v) =>
    JSON.stringify({
      busy: v?.ui?.busy,
      canvas_busy: v?.ui?.canvas?.busy,
      editing: v?.ui?.editing,
      field_saves: v?.ui?.field_saves,
      message: v?.ui?.message,
    });
  const until = async (predicate, what, ids = [], attempts = 60) => {
    let last = null;
    for (let attempt = 0; attempt < attempts; attempt += 1) {
      const view = await read(ids);
      last = view;
      if (matches(predicate, view)) return view;
      await delay(300);
    }
    throw Error(`timed out waiting for ${what}; last panel state ${snapshot(last)}`);
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
    const [x, y, w, h] = view.controls[id].split(",").map(Number);
    await pointer("pointer_down", `${x + w / 2},${y + h / 2}`);
    await pointer("pointer_up", `${x + w / 2},${y + h / 2}`);
    await delay(250);
  };
  const idle = () => until((v) => !v.ui.busy && !v.ui.canvas.busy, "an idle panel");
  let markersSeen = 0;
  /// Draft one inspector field with real keyboard input and save it.
  const setField = async (key, text, want) => {
    const before = await idle();
    return {
      revision: before.ui.canvas.state?.revision ?? 0,
      preview: previewOf(board.id).relative,
      done: await draft(key, text, want),
    };
  };
  const draft = async (key, text, want = /^已保存/) => {
    const target = `${board.id}#${key}`;
    await clickId(`field-${key}`);
    // The edit-row geometry is shared by every row, so only the panel state proves which
    // field this draft belongs to.
    await until((v) => v.ui.editing === target, `the draft row of ${key}`);
    await clickId("field-edit-input");
    await pointer("key_press", "ctrl-a");
    // An empty draft is cleared with the keyboard, since typing nothing would leave the
    // selected text in place.
    if (text === "") await pointer("key_press", "backspace");
    else await pointer("input_text", text);
    await clickId("field-edit-save");
    // The row shows its own in-flight marker while the rest of the panel stays usable. A
    // no-op save finishes before the next read, so observing the marker is opportunistic.
    for (let attempt = 0; attempt < 8; attempt += 1) {
      const view = await read(["field-edit-save"]);
      if (view.ui.field_saves.some((entry) => entry.endsWith(`#${key}`))) {
        markersSeen += 1;
        assert.equal(view.ui.busy, false, "a field save must not block the whole panel");
        break;
      }
      if (!view.ui.field_saves.length && want.test(String(view.ui.message || ""))) break;
    }
    const done = await until(
      (v) => !v.ui.field_saves.length && want.test(String(v.ui.message || "")),
      `the ${want} feedback of ${key}`,
      ["field-edit-save"],
      40,
    );
    assert.equal(done.ui.busy, false, `${key}: no whole-panel wait for one field save`);
    return done;
  };

  // 1. A free port forces the silent official session: the user's own GUI is never touched.
  const port = await freePort();
  await step({ tool: "workcrate-ui.configure_vectorcraft", arguments: { executable: EXE, port } });
  const created = await step({
    tool: "workcrate-ui.manage_project",
    arguments: { action: "create", name: `物料信息条${run}`, directory: documents },
  });
  const project = created.result.project;
  const board = created.result.boards[0];
  assert.ok(project?.id && board?.id, "the project and its page must exist");
  assert.equal(created.result.boards.length, 1, "one page at first");

  // 2. A second page is the untouched-cache control.
  const second = await step({ tool: "workcrate-ui.create_page", arguments: { project_id: project.id } });
  const other = second.result.board_id;
  await step({ tool: "workcrate-ui.open_board", arguments: { project_id: project.id, board_id: other } });
  await until((v) => v.ui.preview_pending === 0, "the second page preview");
  const otherPreview = previewOf(other);
  assert.ok(fs.existsSync(otherPreview.absolute), "the second page frame is on disk");

  // 3. Open the official session in edit mode, then step back to preview so the inspector,
  //    rail and scroll of the page are what the user sees.
  await step({ tool: "workcrate-ui.open_board", arguments: { project_id: project.id, board_id: board.id } });
  await until((v) => v.ui.preview_pending === 0, "the first page preview");
  await clickId("workcrate-canvas-edit");
  const open = await until((v) => v.ui.canvas.state && !v.ui.canvas.busy, "the official session");
  assert.equal(open.ui.canvas.state.live, false, "an unused control port must run the headless session");
  const artboard = open.ui.canvas.state.artboard;
  assert.ok(artboard.width > 200, "the page needs room for the compact band");
  await clickId("workcrate-canvas-preview");
  const previewing = await until((v) => v.ui.canvas.mode === "preview" && v.ui.canvas.state, "preview with the session kept");
  assert.equal(previewing.ui.canvas.state.board_id, board.id, "preview keeps the explicit session");
  await step({ tool: "workcrate-ui.state", arguments: {}, capture: "header-band-before-fields" });

  // 4. Consecutive field saves, each through the reused session.
  for (const [key, text, field] of [
    ["material", "瓦楞纸板 A/B", "material"],
    ["quantity", "20套", "quantity"],
    ["size", "300×200", "size"],
  ]) {
    const { revision, preview, done } = await setField(key, text);
    const card = done.ui.boards.find((b) => b.id === board.id);
    assert.ok(done.ui.canvas.state, `${key}: the session must stay open across a field save`);
    assert.equal(done.ui.canvas.state.board_id, board.id, `${key}: the session must stay on this page`);
    assert.ok(done.ui.canvas.state.revision > revision, `${key}: the official session must advance`);
    assert.equal(card.canvas_revision, done.ui.canvas.state.revision, `${key}: the card shows the saved frame`);
    assert.equal(done.ui.preview_pending, 0, `${key}: only the changed page is re-read`);
    assert.equal(done.ui.busy, false, `${key}: no whole-project reload after a field save`);
    assert.equal(done.ui.boards.find((b) => b.id === other).preview_ready, true, `${key}: the other page keeps its image, not a placeholder`);
    assert.equal(card.info[field], text, `${key}: written back into the card`);
    assert.notEqual(previewOf(board.id).relative, preview, `${key}: the changed page got a new frame`);
    assert.equal(previewOf(other).relative, otherPreview.relative, `${key}: the untouched page keeps its preview file`);
    assert.ok(fs.existsSync(otherPreview.absolute), `${key}: the untouched frame is still on disk`);
  }
  await step({ tool: "workcrate-ui.state", arguments: {}, capture: "header-band-after-fields" });

  // 5. Extras: add a field, then fill it.
  await clickId("page-add-field");
  await until((v) => v.ui.editing === `${board.id}#new`, "the new-field draft");
  await clickId("field-edit-input");
  await pointer("input_text", "工艺");
  await clickId("field-edit-save");
  const added = await until(
    (v) => !v.ui.field_saves.length && v.ui.boards.find((b) => b.id === board.id).info.extra["工艺"] === "",
    "the new empty field",
  );
  assert.match(added.ui.message, /^已保存：字段 工艺/);
  const extra = await setField("extra:工艺", "模切");
  assert.equal(extra.done.ui.boards.find((b) => b.id === board.id).info.extra["工艺"], "模切");

  // 6. Inheritance and reset.
  const named = await setField("local", "本页专用名称");
  assert.equal(named.done.ui.boards.find((b) => b.id === board.id).info.project_name_override, "本页专用名称");
  await clickId("reset-project-name");
  const inherited = await until(
    (v) => !v.ui.field_saves.length && v.ui.boards.find((b) => b.id === board.id).info.project_name_override == null,
    "the page name back to inherited",
  );
  assert.match(inherited.ui.message, /^已保存：项目名称/);

  // 7. A no-op draft must not step the official session or re-render the page.
  const quiet = await idle();
  const beforeNoop = quiet.ui.canvas.state.revision;
  const noopPreview = previewOf(board.id).relative;
  const noop = await draft("material", "瓦楞纸板 A/B");
  assert.equal(noop.ui.canvas.state.revision, beforeNoop, "an unchanged field must not create an official step");
  assert.equal(noop.ui.preview_pending, 0, "an unchanged field must not queue a re-render");
  assert.equal(previewOf(board.id).relative, noopPreview, "an unchanged field must not re-render the page");

  // 8. A refused draft keeps the draft and says why in user words. The official session gets
  //    no step, the page keeps its frame, and the card keeps its value.
  const beforeFail = (await idle()).ui.canvas.state.revision;
  const failPreview = previewOf(board.id).relative;
  const failed = await draft("title", "", /^保存失败/);
  assert.equal(failed.ui.editing, `${board.id}#title`, "a failed save must keep its draft row open");
  assert.match(failed.ui.message, /^保存失败：页面名称不能为空/, "the save feedback must read as Chinese guidance");
  assert.ok(
    !/[A-Za-z_]{4,}|material|separator|page_number|extra:|label:/.test(failed.ui.message),
    `the feedback must not name internals: ${failed.ui.message}`,
  );
  assert.ok(failed.ui.canvas.state, "a refused draft must leave the opened session alone");
  assert.equal(failed.ui.canvas.state.revision, beforeFail, "a refused draft must not create an official step");
  assert.equal(previewOf(board.id).relative, failPreview, "a refused draft must not re-render the page");
  assert.equal(failed.ui.boards.find((b) => b.id === board.id).title, board.title, "a refused draft must not change the page");
  assert.ok(markersSeen > 0, "at least one real save must show its own in-flight marker");
  await step({ tool: "workcrate-ui.state", arguments: {}, capture: "header-field-save-failure" });
  await clickId("field-edit-cancel");
  await until((v) => !v.ui.editing, "the draft row to close");

  // 9. The native document holds the compact band and every header role.
  const nativeText = fs.readFileSync(documentFile(), "latin1");
  for (const marker of ["fileatlas.workcrate", "page_number", "project_name", "material", "quantity", "band", "label:extra:"]) {
    assert.ok(nativeText.includes(marker), `the native document must hold \`${marker}\``);
  }

  // 10. Persistence is read back from the official document, not from UI memory.
  const refreshed = await step({ tool: "workcrate-ui.refresh_project", arguments: { project_id: project.id } });
  const reloaded = refreshed.result.boards.find((b) => b.id === board.id);
  assert.equal(reloaded.info.material, "瓦楞纸板 A/B");
  assert.equal(reloaded.info.quantity, "20套");
  assert.equal(reloaded.info.size, "300×200");
  assert.equal(reloaded.info.extra["工艺"], "模切");
  assert.equal(reloaded.info.project_name_override ?? null, null, "an inherited name stays inherited");
  // An explicit page sync is the whole-project path: every page gets a fresh frame there,
  // while the field saves above proved that one save never disturbs the other page's file.
  const refreshedOther = previewOf(other);
  assert.ok(fs.existsSync(refreshedOther.absolute), "the refresh re-rendered the second page");

  // 11. Quiet state: the settled panel stops changing its own snapshot.
  await step({ tool: "workcrate-ui.open_board", arguments: { project_id: project.id, board_id: board.id } });
  await until((v) => v.ui.preview_pending === 0 && !v.ui.busy, "the reloaded page");
  await step({ tool: "workcrate-ui.state", arguments: {}, capture: "header-pages-final" });
  await delay(3000);
  const settled = JSON.stringify((await read()).ui);
  await delay(2500);
  assert.equal(JSON.stringify((await read()).ui), settled, "an idle panel must not repaint its snapshot");

  fs.writeFileSync(
    path.join(profile, "header-fields-result.json"),
    JSON.stringify({ passed: true, port, project: project.id, board: board.id, second: other, savingMarkersSeen: markersSeen, document: documentFile() }, null, 2),
  );
  console.log(JSON.stringify({ headerFields: "passed", project: project.id, board: board.id, savingMarkersSeen: markersSeen }));
};
