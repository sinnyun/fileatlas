// WorkCrate page switching inside the editor: same-process FileAtlas MCP scenario.
// Proves that picking another page keeps the edit screen, the preview and the one
// official VectorCraft connection; only leaving the editor closes the session.
//
// node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe \
//   --profile .local/validation/profiles/wc-page-switch --access-level full \
//   --scenario scripts/validate-workcrate-page-switch.cjs
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
  return { x, y, width, height, cx: x + width / 2, cy: y + height / 2 };
}

module.exports = async ({ call, payload, step, delay, profile }) => {
  const run = String(Date.now());
  const documents = path.join(profile, "documents", run);
  fs.mkdirSync(documents, { recursive: true });
  const nativeDocument = () => {
    const names = fs
      .readdirSync(documents)
      .filter((name) => name.endsWith(".vectorcraft"));
    assert.equal(names.length, 1, `one native document must exist: ${names.join(", ")}`);
    return path.join(documents, names[0]);
  };

  const read = async (controls = []) =>
    payload(await call("workcrate-ui.state", { control_ids: controls }));
  const untilView = async (predicate, what, controls = [], attempts = 40) => {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
      const view = await read(controls);
      if (predicate(view)) return view;
      await delay(400);
    }
    throw Error(`timed out waiting for ${what}`);
  };
  const readBox = async (id) => {
    const view = await untilView((v) => v.controls?.[id], `control ${id} geometry`, [id]);
    return { view, box: rect(view.controls[id]) };
  };
  const pointer = async (action, value) => {
    for (let attempt = 0; attempt < 25; attempt += 1) {
      try {
        await step({ tool: "control_ui", arguments: { action, value } });
        return;
      } catch (error) {
        if (!String(error.message).includes("still pending")) throw error;
        await delay(300);
      }
    }
    throw Error(`the MCP pointer queue never drained for ${action}`);
  };
  const click = async (box) => {
    await pointer("pointer_down", `${box.cx},${box.cy}`);
    await pointer("pointer_up", `${box.cx},${box.cy}`);
    await delay(400);
  };
  const untilCanvas = (predicate, what, attempts = 40) =>
    untilView(
      (view) => view.ui?.canvas?.state && !view.ui.canvas.busy && predicate(view.ui.canvas),
      what,
      [],
      attempts,
    );

  // 1. One isolated project with two native pages; no official GUI is touched.
  const port = await freePort();
  await step({ tool: "workcrate-ui.configure_vectorcraft", arguments: { executable: EXE, port } });
  const created = await step({
    tool: "workcrate-ui.manage_project",
    arguments: { action: "create", name: `切页实测${run}`, directory: documents },
  });
  const project = created.result.project;
  const first = { id: created.result.boards[0].id };
  const secondPage = await step({
    tool: "workcrate-ui.create_page",
    arguments: { project_id: project.id },
  });
  const second = { id: secondPage.result.board_id };
  assert.ok(project?.id && first.id && second.id, "both native pages must exist");
  const documentFile = nativeDocument();
  const sizeBefore = fs.statSync(documentFile).size;

  // 2. Enter the page editor with a real pointer click on the first page.
  await step({
    tool: "workcrate-ui.open_board",
    arguments: { project_id: project.id, board_id: first.id },
  });
  const previewScreen = await untilView((view) => view.ui, "the panel snapshot");
  assert.equal(previewScreen.ui.screen, "editor");
  assert.equal(previewScreen.ui.canvas.mode, "preview");
  assert.equal(
    previewScreen.ui.connection,
    "not_connected",
    "browsing must never open the official program",
  );
  await click((await readBox("workcrate-canvas-edit")).box);
  const opened = await untilCanvas(
    (canvas) => canvas.state.board_id === first.id,
    "the first page's official canvas",
  );
  const session = opened.ui.canvas.state.session_id;
  assert.ok(session > 0, "the official connection must have an identity");
  assert.equal(opened.ui.canvas.mode, "edit");
  assert.equal(opened.ui.canvas.state.live, false, "an unused control port runs headless");

  // 3. Pick the second page in the page rail: the edit screen, the preview and the
  //    official connection must all follow without reconnecting.
  await click((await readBox(`rail-${second.id}`)).box);
  const switched = await untilView(
    (view) =>
      view.ui?.canvas?.mode === "edit" &&
      view.ui?.canvas?.state?.board_id === second.id &&
      !view.ui.canvas.busy &&
      view.ui.preview_pending === 0,
    "the second page in the edit screen",
    [],
    50,
  );
  assert.equal(switched.ui.screen, "editor", "a page pick must stay in the page editor");
  assert.equal(
    switched.ui.canvas.state.session_id,
    session,
    "both pages must be served by the same official connection",
  );
  assert.notEqual(
    switched.ui.canvas.state.preview,
    opened.ui.canvas.state.preview,
    "the overlay must render the new page's own frame",
  );
  const secondCard = switched.ui.boards.find((board) => board.id === second.id);
  assert.ok(secondCard?.preview_ready, "the new page's preview must be ready");
  assert.equal(
    secondCard.canvas_revision,
    switched.ui.canvas.state.revision,
    "the rail, the preview and the overlay must share one official revision",
  );

  // 4. An official edit on the newly selected page must reach the panel through the
  //    same connection, still in the edit screen.
  const artboard = switched.ui.canvas.state.artboard;
  const text = await step({
    tool: "workcrate-ui.canvas_create_text",
    arguments: {
      x: artboard.x + artboard.width / 2,
      y: artboard.y + artboard.height / 2,
      text: "切页后的官方文字",
      size: 26,
      confirm: true,
    },
  });
  const authored = text.result.canvas;
  assert.equal(authored.session_id, session, "editing page two must reuse the connection");
  assert.equal(authored.board_id, second.id);
  const synced = await untilCanvas(
    (canvas) => canvas.state.revision === authored.revision,
    "the official edit to reach the overlay",
  );
  assert.equal(synced.ui.canvas.mode, "edit");

  // 5. Switch back: the same session, the same page frame, and the other page's
  //    selection must not leak across the document.
  await click((await readBox(`rail-${first.id}`)).box);
  const back = await untilView(
    (view) =>
      view.ui?.canvas?.mode === "edit" &&
      view.ui?.canvas?.state?.board_id === first.id &&
      !view.ui.canvas.busy &&
      view.ui.preview_pending === 0,
    "the first page again in the edit screen",
    [],
    50,
  );
  assert.equal(back.ui.canvas.state.session_id, session, "returning must not reconnect");
  assert.deepEqual(
    back.ui.canvas.state.selection,
    [],
    "a page's selection must not carry over to another page",
  );
  assert.ok(
    back.ui.canvas.state.revision >= authored.revision,
    "the official document history is shared by every page",
  );
  assert.equal(
    back.ui.boards.find((board) => board.id === first.id).canvas_revision,
    back.ui.canvas.state.revision,
  );

  // 6. Preview mode keeps the same live connection and follows page picks too.
  await click((await readBox("workcrate-canvas-preview")).box);
  const previewMode = await untilView(
    (view) => view.ui?.canvas?.mode === "preview" && view.ui?.canvas?.state?.board_id === first.id,
    "the preview screen",
  );
  assert.equal(previewMode.ui.canvas.state.session_id, session);
  await click((await readBox(`rail-${second.id}`)).box);
  const previewSwitch = await untilView(
    (view) =>
      view.ui?.canvas?.mode === "preview" &&
      view.ui?.canvas?.state?.board_id === second.id &&
      view.ui.preview_pending === 0,
    "the second page while previewing",
    [],
    50,
  );
  assert.equal(previewSwitch.ui.canvas.state.session_id, session);
  assert.ok(
    previewSwitch.ui.canvas.state.revision >= authored.revision,
    "preview must keep the shared official history",
  );
  assert.equal(
    previewSwitch.ui.boards.find((board) => board.id === second.id).canvas_revision,
    previewSwitch.ui.canvas.state.revision,
    "the preview must show the page's newest official frame",
  );

  // 7. Leaving the editor for the page list is the only thing that closes the session.
  await step({ tool: "workcrate-ui.state", arguments: {}, capture: "page-switch-preview" });
  await click((await readBox("workcrate-pages-home")).box);
  const pages = await untilView(
    (view) =>
      view.ui?.screen === "pages" &&
      view.ui?.canvas?.state === null &&
      !view.ui.busy &&
      !view.ui.canvas.busy,
    "the page list after leaving the editor",
    [],
    60,
  );
  assert.equal(pages.ui.canvas.mode, "preview");
  await step({
    tool: "workcrate-ui.canvas_state",
    arguments: {},
    expect_error: "canvas editing is not open",
  });

  // 8. Re-entering the editor starts a fresh connection, proving the close was real.
  await step({
    tool: "workcrate-ui.open_board",
    arguments: { project_id: project.id, board_id: first.id },
  });
  await click((await readBox("workcrate-canvas-edit")).box);
  const reopened = await untilCanvas(
    (canvas) => canvas.state.board_id === first.id,
    "the editor after re-entering",
  );
  assert.ok(
    reopened.ui.canvas.state.session_id > session,
    "leaving the editor must have released the official process",
  );
  await click((await readBox("workcrate-canvas-preview")).box);

  // 9. The document holds both pages and their edits; a settled panel stays quiet.
  const refreshed = await step({
    tool: "workcrate-ui.refresh_project",
    arguments: { project_id: project.id },
  });
  assert.equal(refreshed.result.boards.length, 2, "the saved document must hold both pages");
  assert.ok(
    fs.statSync(nativeDocument()).size > sizeBefore,
    "the native document must hold the page edits",
  );
  await delay(3000);
  const settled = JSON.stringify((await read()).ui);
  await delay(2500);
  assert.equal(JSON.stringify((await read()).ui), settled, "an idle panel must not repaint");

  fs.writeFileSync(
    path.join(profile, "page-switch-scenario.json"),
    JSON.stringify(
      { port, project: project.id, first: first.id, second: second.id, session, revision: authored.revision },
      null,
      2,
    ),
  );
  console.log(
    JSON.stringify({
      pageSwitchScenario: "passed",
      project: project.id,
      pages: [first.id, second.id],
      session,
      revision: authored.revision,
    }),
  );
};
