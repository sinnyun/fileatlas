// WorkCrate canvas editing: same-process FileAtlas MCP scenario.
// Drives the real panel with pointer input and the official VectorCraft session, then proves
// that an edit made outside the panel still repaints it.
//
// node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe \
//   --profile .local/validation/profiles/wc-canvas --access-level full \
//   --scenario scripts/workcrate-canvas-scenario.cjs
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
  return { x, y, width, height, cx: x + width / 2, cy: y + height / 2 };
}

/// Window pixels from document points, mirroring the panel's own CanvasMap.
function pixelOf(box, artboard, point) {
  const scale = box.width / artboard.width;
  return [
    box.x + (point[0] - artboard.x) * scale,
    box.y + (point[1] - artboard.y) * scale,
  ];
}

module.exports = async ({ call, payload, step, delay, profile }) => {
  // One fresh project and folder per run: the panel must own exactly one native document.
  const run = String(Date.now());
  const documents = path.join(profile, "documents", run);
  const links = path.join(profile, "links");
  fs.mkdirSync(documents, { recursive: true });
  fs.mkdirSync(links, { recursive: true });
  const first = path.join(links, "placed.jpeg");
  const second = path.join(links, "replacement.jpeg");
  fs.copyFileSync(path.resolve(FIXTURE), first);
  fs.copyFileSync(path.resolve(FIXTURE), second);
  const nativeDocument = () => {
    const names = fs
      .readdirSync(documents)
      .filter((name) => name.endsWith(".vectorcraft"));
    assert.equal(names.length, 1, `one native document must exist: ${names.join(", ")}`);
    return path.join(documents, names[0]);
  };

  const read = async (controls = []) =>
    payload(await call("workcrate-ui.state", { control_ids: controls }));
  // Both the panel snapshot and control geometry only appear once the panel paints, so poll.
  const untilView = async (predicate, what, controls = [], attempts = 30) => {
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
  const click = async (box, capture) => {
    await pointer("pointer_down", `${box.cx},${box.cy}`);
    await pointer("pointer_up", `${box.cx},${box.cy}`);
    await delay(400);
    if (capture) await step({ tool: "workcrate-ui.state", arguments: {}, capture });
  };
  const untilCanvas = () =>
    untilView(
      (view) => view.ui?.canvas?.state && view.controls?.["canvas-artboard"],
      "the official canvas snapshot",
      ["canvas-artboard", "workcrate-canvas-zoom-in", "workcrate-canvas-preview"],
      40,
    );

  // 1. The official port stays unlisted, so the silent headless route must be chosen.
  const port = await freePort();
  await step({ tool: "workcrate-ui.configure_vectorcraft", arguments: { executable: EXE, port } });
  const created = await step({
    tool: "workcrate-ui.manage_project",
    arguments: { action: "create", name: `画布实测${run}`, directory: documents },
  });
  const project = created.result.project;
  const board = created.result.boards[0];
  assert.ok(project?.id && board?.id, "the project and its native page must exist");
  const documentFile = nativeDocument();
  const sizeBefore = fs.statSync(documentFile).size;

  // 2. Open the page, then enter the edit screen with a real pointer click.
  await step({ tool: "workcrate-ui.open_board", arguments: { project_id: project.id, board_id: board.id } });
  const previewScreen = await untilView((view) => view.ui, "the panel snapshot");
  assert.equal(previewScreen.ui.screen, "editor");
  assert.equal(previewScreen.ui.canvas.mode, "preview");
  assert.equal(previewScreen.ui.connection, "not_connected", "browsing must never open the official program");
  const editButton = await readBox("workcrate-canvas-edit");
  await click(editButton.box);
  const view = await untilCanvas();
  const opened = view.ui.canvas.state;
  assert.equal(view.ui.canvas.mode, "edit", "the click must switch the panel to the edit screen");
  assert.equal(opened.live, false, "an unused control port must run the silent headless session");
  assert.ok(opened.artboard.width > 0 && opened.preview, "the snapshot needs official geometry and a bitmap");
  const box = rect(view.controls["canvas-artboard"]);
  const artboard = { x: opened.artboard.x, y: opened.artboard.y, width: opened.artboard.width, height: opened.artboard.height };
  assert.ok(
    Math.abs(box.width / artboard.width - box.height / artboard.height) < 0.01,
    `the overlay box must keep the official artboard ratio: ${JSON.stringify({ box, artboard })}`,
  );
  await step({ tool: "workcrate-ui.state", arguments: {}, capture: "canvas-edit-open" });

  // 3. Reverse sync: an official edit the panel did not cause must repaint the overlay.
  const centre = [artboard.x + artboard.width / 2, artboard.y + artboard.height / 2];
  const createdText = await step({
    tool: "workcrate-ui.canvas_create_text",
    arguments: { x: centre[0], y: centre[1], text: "官方画布文字", size: 28, confirm: true },
  });
  const textSnapshot = createdText.result.canvas;
  assert.ok(textSnapshot.selection.length >= 1, "the official program must select the text it created");
  const textId = textSnapshot.selection[0];
  assert.ok(textSnapshot.revision > opened.revision, "the official revision must advance");
  const synced = await step({
    tool: "workcrate-ui.state",
    arguments: {},
    until: { field: "ui.canvas.state.revision", at_least: textSnapshot.revision, attempts: 40, interval_ms: 400 },
    capture: "canvas-edit-reverse-sync",
  });
  assert.deepEqual(
    synced.result.ui.canvas.state.selection,
    textSnapshot.selection,
    "the panel must show the official selection without any user action",
  );

  // 4. A real pointer drag on the object becomes one official gesture (the official Move).
  const bounds = textSnapshot.selection_bounds;
  assert.ok(bounds, "the official selection box must drive the overlay handles");
  const grab = pixelOf(box, artboard, [bounds.x + bounds.width / 2, bounds.y + bounds.height / 2]);
  const drop = pixelOf(box, artboard, [bounds.x + bounds.width / 2 + 60, bounds.y + bounds.height / 2 + 40]);
  await pointer("pointer_down", `${grab[0]},${grab[1]}`);
  // Real input always emits drag samples up to the release point; the official engine
  // moves the object on those samples, not on the release alone.
  for (let sample = 1; sample <= 4; sample += 1) {
    const t = sample / 4;
    await pointer(
      "pointer_drag",
      `${grab[0] + (drop[0] - grab[0]) * t},${grab[1] + (drop[1] - grab[1]) * t}`,
    );
  }
  await delay(200);
  await pointer("pointer_up", `${drop[0]},${drop[1]}`);
  await delay(2000);
  const dragged = await step({ tool: "workcrate-ui.canvas_state", arguments: {}, capture: "canvas-edit-after-drag" });
  const moved = dragged.result.canvas.selection_bounds;
  assert.ok(
    Math.abs(moved.x - (bounds.x + 60)) < 2 && Math.abs(moved.y - (bounds.y + 40)) < 2,
    `the pointer drag must be the official Move: ${JSON.stringify({ bounds, moved, expected: [bounds.x + 60, bounds.y + 40] })}`,
  );
  assert.ok(dragged.result.canvas.revision > textSnapshot.revision, "the drag must add one official step");

  // 5. Place a link from outside the document folder: the session must widen its one
  //    official access folder and keep the same page open.
  const placed = await step({
    tool: "workcrate-ui.canvas_place",
    arguments: { path: first, at: [centre[0], centre[1] + 60], link: true, confirm: true },
  });
  const imageId = placed.result.canvas.selection[0];
  assert.ok(imageId !== undefined, "the placed image must become the official selection");
  // The panel must follow all the way through: the overlay selection and the object
  // detail (name, kind, geometry) both come from the official session.
  const placedView = await untilView(
    (view) => view.ui?.canvas?.object?.id === imageId,
    `the object panel to show the placed image ${imageId}`,
    [],
    40,
  );
  assert.equal(placedView.ui.canvas.state.revision, placed.result.canvas.revision);
  assert.equal(placedView.ui.canvas.object.kind, "image");
  assert.ok(placedView.ui.canvas.object.linked, "a linked placement stays linked");
  await step({ tool: "workcrate-ui.state", arguments: {}, capture: "canvas-edit-placed-image" });
  fs.appendFileSync(first, Buffer.from([0xff]));
  const updated = await step({
    tool: "workcrate-ui.canvas_links",
    arguments: { action: "update", ids: [imageId], confirm: true },
  });
  assert.ok(
    updated.result.canvas.revision > placed.result.canvas.revision,
    "the official link refresh must step the revision",
  );
  const relinked = await step({
    tool: "workcrate-ui.canvas_links",
    arguments: { action: "relink", ids: [imageId], path: second, confirm: true },
  });
  assert.equal(relinked.result.canvas.board_id, board.id, "relink must stay on the same page");

  // 6. Replace text, then walk the official history.
  const renamed = await step({ tool: "workcrate-ui.canvas_set_text", arguments: { id: textId, text: "第二行文字", confirm: true } });
  const undone = await step({ tool: "workcrate-ui.canvas_step", arguments: { action: "undo", confirm: true } });
  assert.ok(
    undone.result.canvas.revision > renamed.result.canvas.revision,
    "undo must be another official step",
  );
  await step({ tool: "workcrate-ui.canvas_step", arguments: { action: "redo", confirm: true } });

  // 7. Toolbar zoom button, then leave the edit screen (commit + close).
  const zoomIn = await readBox("workcrate-canvas-zoom-in");
  const beforeZoom = zoomIn.view.ui.canvas.zoom;
  await click(zoomIn.box);
  const zoomPreview = await readBox("workcrate-canvas-preview");
  assert.ok(
    zoomPreview.view.ui.canvas.zoom > beforeZoom,
    `the toolbar must zoom the official view: ${beforeZoom} -> ${zoomPreview.view.ui.canvas.zoom}`,
  );
  await click(zoomPreview.box);
  const left = await step({ tool: "workcrate-ui.state", arguments: {}, wait_ms: 1500, capture: "canvas-back-to-preview" });
  assert.equal(left.result.ui.canvas.mode, "preview", "预览 must leave the edit screen");
  assert.equal(left.result.ui.canvas.state, null, "leaving the edit screen closes the official session");

  // 7b. Editing another page must commit and close the page being left.
  const extraPage = await step({ tool: "workcrate-ui.create_page", arguments: { project_id: project.id } });
  await step({
    tool: "workcrate-ui.open_board",
    arguments: { project_id: project.id, board_id: extraPage.result.board_id },
  });
  await click((await readBox("workcrate-canvas-edit")).box);
  const pageTwo = await untilCanvas();
  assert.equal(
    pageTwo.ui.canvas.state.board_id,
    extraPage.result.board_id,
    "the official session must belong to the newly opened page",
  );
  await step({
    tool: "workcrate-ui.open_board",
    arguments: { project_id: project.id, board_id: board.id },
  });
  const switched = await step({
    tool: "workcrate-ui.state",
    arguments: {},
    wait_ms: 1500,
    capture: "canvas-page-switch",
  });
  assert.equal(switched.result.ui.canvas.mode, "preview", "leaving the edited page must exit the edit screen");
  assert.equal(switched.result.ui.canvas.state, null, "the page being left must close its official session");

  // 8. The native document is the single truth: the offline catalogue re-reads the edited page.
  const refreshed = await step({ tool: "workcrate-ui.refresh_project", arguments: { project_id: project.id } });
  assert.equal(refreshed.result.boards.length, 2, "the page list must come from the saved document");
  const sizeAfter = fs.statSync(nativeDocument()).size;
  assert.ok(
    sizeAfter > sizeBefore,
    `the native document must hold the canvas edits: ${sizeBefore} -> ${sizeAfter}`,
  );
  await step({
    tool: "workcrate-ui.open_board",
    arguments: { project_id: project.id, board_id: refreshed.result.boards[0].id },
    wait_ms: 1500,
    capture: "canvas-catalogue-after-edit",
  });
  // 9. Quiet state: a settled panel must not keep changing its own snapshot.
  await delay(3000);
  const settled = JSON.stringify((await read()).ui);
  await delay(2500);
  const again = JSON.stringify((await read()).ui);
  assert.equal(again, settled, "an idle panel must not repaint its snapshot");
  fs.writeFileSync(
    path.join(profile, "canvas-scenario.json"),
    JSON.stringify({ port, project: project.id, board: board.id, textId, imageId, sizeAfter }, null, 2),
  );
  console.log(JSON.stringify({ canvasScenario: "passed", project: project.id, board: board.id, textId, imageId }));
};
