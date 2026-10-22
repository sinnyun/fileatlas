// Validation: WorkCrate 页面管理 (Tier 2) and 装配工作台 (Tier 3) redesign —
// the Tier 2 hero header (statistics, multi-select toggle, unified actions),
// the page card anatomy (checkbox, spec line, material band, preview, footer),
// the export bar, and the Tier 3 unified icon toolbar (zoom/undo/redo/handoff),
// numbered rail cards and inspector cards. Screenshots back the assertions.
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

module.exports = async ({ call, payload, step, delay, profile }) => {
  const run = String(Date.now());
  const documents = path.join(profile, "documents", run);
  fs.mkdirSync(documents, { recursive: true });

  const read = async (controlIds) =>
    payload(
      await call(
        "workcrate-ui.state",
        controlIds ? { control_ids: controlIds } : {},
      ),
    );
  const until = async (predicate, what, attempts = 300) => {
    let last = null;
    for (let attempt = 0; attempt < attempts; attempt += 1) {
      const view = await read();
      last = view;
      if (predicate(view)) return view;
      await delay(100);
    }
    throw Error(`timeout: ${what}; ${JSON.stringify(last?.ui?.message)}`);
  };
  const idle = (v) => !v.ui?.busy && !v.ui?.canvas?.busy && !v.ui?.field_saves?.length;
  const windowClick = async (x, y) => {
    const at = `${x},${y}`;
    for (const action of ["pointer_down", "pointer_up"]) {
      for (let attempt = 0; attempt < 30; attempt += 1) {
        try {
          await step({ tool: "control_ui", arguments: { action, value: at } });
          break;
        } catch (error) {
          if (!String(error.message).includes("still pending")) throw error;
          await delay(250);
        }
      }
    }
  };
  const clickControl = async (id, controlIds) => {
    const view = await read(controlIds ?? [id]);
    const box = String(view.controls[id] ?? "")
      .split(",")
      .map(Number);
    if (box.length !== 4 || box[2] <= 0) throw Error(`control ${id} not visible: ${view.controls[id]}`);
    await windowClick(box[0] + box[2] / 2, box[1] + box[3] / 2);
    await delay(300);
  };

  const port = await freePort();
  await step({ tool: "workcrate-ui.configure_vectorcraft", arguments: { executable: EXE, port } });
  await step({ tool: "workcrate-ui.navigate", arguments: {} });
  await until((v) => idle(v) && v.ui?.screen === "projects", "the projects screen");

  const created = await step({
    tool: "workcrate-ui.manage_project",
    arguments: { action: "create", name: `页面重设计${run}`, directory: documents },
  });
  const project = created.result.project;
  const board1 = created.result.boards[0].id;
  await until((v) => idle(v) && v.ui?.screen === "projects", "the projects grid after create");

  // Enter the page matrix through the editor entry, then the Tier 2 pill.
  await step({
    tool: "workcrate-ui.open_board",
    arguments: { project_id: project.id, board_id: board1 },
  });
  await until(
    (v) => idle(v) && v.ui?.screen === "editor" && v.ui?.canvas?.state?.revision > 0,
    "the canvas session",
  );
  await clickControl("workcrate-pages-home");
  await until((v) => idle(v) && v.ui?.screen === "pages", "the page list");

  // 1. The redesigned Tier 2 controls all exist.
  const pageIds = [
    "pages-multi-select",
    "pages-select-all",
    "pages-clear-selection",
    "pages-exit-multi-select",
    "attach-document",
    "refresh-native",
    "workcrate-new-page",
    "export-image-format",
    "export-selected-images",
    "export-all-pages",
    `select-page-${board1}`,
    `open-page-${board1}`,
    `page-preview-${board1}`,
  ];
  let view = await read(pageIds);
  for (const id of pageIds) {
    if (!view.controls[id]) throw Error(`missing control ${id}: ${JSON.stringify(view.controls)}`);
  }

  // 2. The multi-select icon toggle round-trips.
  await clickControl("pages-multi-select");
  await until((v) => idle(v) && v.ui?.multi_select === true, "multi-select on");
  await clickControl("pages-multi-select");
  await until(
    (v) => idle(v) && v.ui?.multi_select === false && v.ui?.selected_pages.length === 0,
    "multi-select off clears the selection",
  );

  // 3. Selecting a page through its checkbox turns multi-select on.
  await clickControl(`select-page-${board1}`);
  await until(
    (v) => idle(v) && v.ui?.multi_select === true && v.ui?.selected_pages.includes(board1),
    "the page checkbox selects the page",
  );
  await clickControl("pages-clear-selection");
  await until((v) => idle(v) && v.ui?.selected_pages.length === 0, "clear selection");

  await step({ tool: "get_ui_state", capture: "pages-redesign" });

  // 4. The editor: unified icon toolbar and numbered rail controls exist.
  await clickControl(`open-page-${board1}`);
  await until(
    (v) => idle(v) && v.ui?.screen === "editor" && v.ui?.canvas?.state?.revision > 0,
    "the editor canvas",
  );
  const editorIds = [
    "workcrate-canvas-add-image",
    "workcrate-canvas-create-text",
    "workcrate-canvas-zoom-out",
    "workcrate-canvas-zoom-in",
    "workcrate-canvas-undo",
    "workcrate-canvas-redo",
    "place-design-file",
    "export-current-image",
    "workcrate-canvas-handoff",
    `rail-${board1}`,
  ];
  view = await read(editorIds);
  for (const id of editorIds) {
    if (!view.controls[id]) throw Error(`missing control ${id}: ${JSON.stringify(view.controls)}`);
  }

  // 5. The zoom icon buttons change the displayed zoom.
  await clickControl("workcrate-canvas-zoom-out");
  await until((v) => idle(v) && Math.abs(v.ui?.canvas?.zoom - 0.8) < 1e-6, "zoom out to 80%");
  await clickControl("workcrate-canvas-zoom-in");
  await until((v) => idle(v) && Math.abs(v.ui?.canvas?.zoom - 1) < 1e-6, "zoom back to 100%");

  await step({ tool: "get_ui_state", capture: "studio-redesign" });

  // 6. Tier navigation still works from the editor: pages, then projects.
  await clickControl("workcrate-pages-home");
  await until((v) => idle(v) && v.ui?.screen === "pages", "back to the page list");
  await clickControl("workcrate-projects-home");
  await until((v) => idle(v) && v.ui?.screen === "projects", "back to the projects screen");

  const result = {
    project: project.id,
    board: board1,
    pagesControlsPresent: true,
    multiSelectToggle: true,
    checkboxSelection: true,
    editorControlsPresent: true,
    zoomButtons: true,
    tierNavigation: true,
  };
  fs.writeFileSync(
    path.join(profile, "pages-redesign-result.json"),
    JSON.stringify(result, null, 2),
  );
  return { pagesRedesign: "passed", ...result };
};
