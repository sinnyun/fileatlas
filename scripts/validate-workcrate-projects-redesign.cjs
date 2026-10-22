// Validation: WorkCrate 项目管理 redesign — shared three-tier entry band, unified
// icon buttons (rename/copy/delete/menu, sort/archive toggles), the card anatomy
// (meta line, page info band, preview caption, .vc footer), the create tile, and
// tier navigation to the page list and back. Screenshots back the assertions.
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
    arguments: { action: "create", name: `重设计项目${run}`, directory: documents },
  });
  const project = created.result.project;
  const board1 = created.result.boards[0].id;
  await until(
    (v) => idle(v) && v.ui?.screen === "projects" && v.ui?.page_count === 0,
    "the projects grid after create",
  );

  // 1. The unified controls exist on the redesigned screen.
  const cardIds = [
    "workcrate-add-project-tile",
    "workcrate-sort",
    "workcrate-archived",
    "workcrate-search-input",
    "workcrate-projects-home",
    `project-menu-${project.id}`,
    `project-rename-${project.id}`,
    `project-duplicate-${project.id}`,
    `project-delete-${project.id}`,
    `project-open-${project.id}`,
  ];
  let view = await read(cardIds);
  for (const id of cardIds) {
    if (!view.controls[id]) throw Error(`missing control ${id}: ${JSON.stringify(view.controls)}`);
  }

  // 2. The sort icon toggle flips the order state both ways.
  await clickControl("workcrate-sort");
  await until((v) => idle(v) && v.ui?.sort_by_name === true, "sort by name");
  await clickControl("workcrate-sort");
  await until((v) => idle(v) && v.ui?.sort_by_name === false, "sort by recency");

  // 3. The archive icon toggle shows the empty archived list, then restores.
  await clickControl("workcrate-archived");
  await until((v) => idle(v) && v.ui?.show_archived === true, "archived view");
  view = await read();
  if (view.ui?.screen !== "projects") throw Error(`archived view left the projects screen: ${view.ui?.screen}`);
  await clickControl("workcrate-archived");
  await until((v) => idle(v) && v.ui?.show_archived === false, "active view");

  // 4. The per-card icon row opens the shared menu with cover/archive entries.
  await clickControl(`project-menu-${project.id}`);
  let menuOpen = false;
  for (let attempt = 0; attempt < 30 && !menuOpen; attempt += 1) {
    view = await read([`project-cover-${project.id}-menu`, `project-archive-${project.id}`]);
    menuOpen = Boolean(view.controls[`project-archive-${project.id}`]);
    if (!menuOpen) await delay(100);
  }
  if (!menuOpen) throw Error("the project menu did not open with its archive entry");
  await clickControl(`project-menu-${project.id}`);
  await until((v) => idle(v), "the menu to close");

  await step({ tool: "get_ui_state", capture: "projects-redesign" });

  // 5. Tier navigation: into the page list, back to the projects screen.
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
  await step({ tool: "get_ui_state", capture: "pages-tier-band" });
  const homeBox = await read(["workcrate-projects-home", "workcrate-pages-home"]);
  if (!homeBox.controls["workcrate-projects-home"]) {
    throw Error(`projects-home control missing on pages screen: ${JSON.stringify(homeBox.controls)}`);
  }
  await clickControl("workcrate-projects-home");
  let back = null;
  for (let attempt = 0; attempt < 30; attempt += 1) {
    back = await read(["workcrate-projects-home"]);
    if (back.ui?.screen === "projects") break;
    await delay(100);
  }
  if (back?.ui?.screen !== "projects") {
    throw Error(
      `did not return to projects: screen=${back?.ui?.screen} homeBox=${JSON.stringify(back?.controls)}`,
    );
  }
  await until((v) => idle(v), "the projects screen to settle");

  const result = {
    project: project.id,
    controlsPresent: true,
    sortToggle: true,
    archiveToggle: true,
    cardMenu: true,
    tierNavigation: true,
  };
  fs.writeFileSync(
    path.join(profile, "projects-redesign-result.json"),
    JSON.stringify(result, null, 2),
  );
  return { projectsRedesign: "passed", ...result };
};
