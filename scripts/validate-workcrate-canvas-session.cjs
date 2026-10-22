// WorkCrate page editor: the background official session opens without a running GUI.
//
// Creates a project and its native document through the official headless CLI, then opens
// the page canvas. Proves the session is silent (live=false) and renders the page even
// though no VectorCraft window or control port exists.
//
// node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe \
//   --profile .local/validation/profiles/wc-canvas-session --access-level full \
//   --scenario scripts/validate-workcrate-canvas-session.cjs
const assert = require("node:assert/strict");
const fs = require("node:fs");
const net = require("node:net");
const path = require("node:path");

const EXE = "C:\\Program Files\\VectorCraft\\vectorcraft.exe";
const CLI = "C:\\Program Files\\VectorCraft\\vectorcraft-cli.exe";

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

module.exports = async ({ call, payload, profile }) => {
  const directory = path.join(profile, "canvas-session");
  fs.mkdirSync(directory, { recursive: true });
  const port = await freePort();
  await call("workcrate-ui.configure_vectorcraft", {
    executable: EXE,
    cli_executable: CLI,
    port,
  });
  const created = payload(
    await call("workcrate-ui.manage_project", {
      action: "create",
      name: `画布会话验证-${Date.now()}`,
      directory,
    }),
  );
  const board = created.boards[0];
  assert.ok(board, "the created project must have one native page");
  const opened = payload(await call("workcrate-ui.canvas_open", { board_id: board.id }));
  const canvas = opened.canvas;
  assert.equal(
    canvas.live,
    false,
    `no GUI is running, so the silent session must serve the canvas: ${JSON.stringify(canvas)}`,
  );
  assert.equal(canvas.board_id, board.id);
  assert.ok(canvas.preview, "the canvas must render a preview");
  assert.ok(fs.existsSync(canvas.preview), `preview must exist on disk: ${canvas.preview}`);
  const state = payload(await call("workcrate-ui.canvas_state", {}));
  assert.equal(state.canvas.board_id, board.id);
  console.log(
    JSON.stringify(
      {
        project: created.project.id,
        board: board.id,
        canvas: {
          live: canvas.live,
          revision: canvas.revision,
          preview: canvas.preview,
          preview_size: canvas.preview_size,
        },
      },
      null,
      2,
    ),
  );
};
