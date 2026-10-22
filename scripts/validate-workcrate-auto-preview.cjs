// WorkCrate automatic preview acquisition: same-process FileAtlas MCP scenario.
//
// Proves that a saved change made by an independent official VectorCraft session reaches
// the WorkCrate preview without switching to the editor or pressing "同步工程与预览".
// The idle watcher samples the saved document stamp, starts the silent official session
// only after a real change, patches the catalogue and re-decodes the page bitmap.
//
// node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe \
//   --profile .local/validation/profiles/wc-auto-preview --access-level full \
//   --scenario scripts/validate-workcrate-auto-preview.cjs
const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const fs = require("node:fs");
const net = require("node:net");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

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

function digest(file) {
  return crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
}

module.exports = async ({ call, step, payload, delay, profile, root }) => {
  const run = String(Date.now());
  const documents = path.join(profile, "documents", run);
  fs.mkdirSync(documents, { recursive: true });

  const read = async () => payload(await call("workcrate-ui.state", {}));
  const until = async (predicate, what, attempts = 120) => {
    let last = null;
    for (let attempt = 0; attempt < attempts; attempt += 1) {
      const view = await read();
      last = view;
      try {
        if (view?.ui && predicate(view)) return view;
      } catch {
        /* an incomplete panel snapshot is retried */
      }
      await delay(500);
    }
    throw Error(
      `timed out waiting for ${what}; last=${JSON.stringify({
        screen: last?.ui?.screen,
        busy: last?.ui?.busy,
        pending: last?.ui?.preview_pending,
        auto: last?.ui?.auto_sync,
        message: last?.ui?.message,
      })}`,
    );
  };

  // 1. A free port keeps every WorkCrate session on the silent headless transport.
  const port = await freePort();
  await step({ tool: "workcrate-ui.configure_vectorcraft", arguments: { executable: EXE, port } });
  const created = await step({
    tool: "workcrate-ui.manage_project",
    arguments: { action: "create", name: `自动同步${run}`, directory: documents },
  });
  const project = created.result.project;
  const board = created.result.boards[0];
  assert.ok(project?.id && board?.id, "the project and its page must exist");

  await step({
    tool: "workcrate-ui.open_board",
    arguments: { project_id: project.id, board_id: board.id },
  });
  const initial = await until(
    (v) =>
      v.ui.board_id === board.id &&
      v.ui.preview_pending === 0 &&
      v.ui.previews_ready >= 1 &&
      v.ui.busy === false &&
      v.ui.auto_sync,
    "the initial cached preview",
  );
  assert.equal(initial.ui.auto_sync.syncs, 0, "a freshly built cache must not auto sync");
  assert.equal(initial.ui.canvas.state, null, "browsing must not open an official session");

  const document = path.join(documents, `${project.name}.vectorcraft`);
  assert.ok(fs.existsSync(document), `native document missing: ${document}`);
  const cacheFile = path.join(profile, "workcrate", "projects", project.id, "boards-cache.json");
  const cache = () => JSON.parse(fs.readFileSync(cacheFile, "utf8"));
  // Nanosecond stamps exceed Number's safe integer range, so compare their raw digits.
  const sourceStamp = () => {
    const raw = fs.readFileSync(cacheFile, "utf8");
    const match = /"source"\s*:\s*\{\s*"modified_ns"\s*:\s*(\d+)\s*,\s*"len"\s*:\s*(\d+)/.exec(raw);
    assert.ok(match, "the catalogue records the saved document stamp");
    return { modified_ns: BigInt(match[1]), len: BigInt(match[2]) };
  };
  const initialCache = cache();
  const initialStamp = sourceStamp();
  const key = `${board.id}@1600`;
  const initialPreview = path.join(path.dirname(cacheFile), initialCache.previews[key]);
  const initialHash = digest(initialPreview);

  // 2. A fresh cache stays quiet and never spawns an official process on its own.
  await delay(4000);
  const quiet = await read();
  assert.equal(quiet.ui.auto_sync.syncs, 0, "an unchanged document must not trigger a sync");
  assert.equal(quiet.ui.auto_sync.pending, false);

  // 3. An independent official headless session saves a visible edit. This is the user's
  //    VectorCraft program from WorkCrate's point of view: it only writes the native file.
  const vcMcp = path.join(root, ".agents", "skills", "vectorcraft", "scripts", "vc-mcp.cjs");
  const external = spawnSync(
    process.execPath,
    [
      vcMcp,
      "--headless",
      "--read-root",
      documents,
      "--write-root",
      documents,
      "--call",
      "open_file",
      JSON.stringify({ path: document }),
      "--call",
      "add_text",
      JSON.stringify({ text: "AUTO-SYNC 自动同步", x: 48, y: 130, size: 48, color: "#d32f2f" }),
      "--call",
      "save_file",
      JSON.stringify({ path: document }),
      "--inspect",
    ],
    {
      cwd: root,
      encoding: "utf8",
      timeout: 120000,
      env: { ...process.env, VECTORCRAFT_CONFIG_DIR: path.join(profile, "external-cli-config") },
    },
  );
  assert.equal(
    external.status,
    0,
    `the independent official edit failed:\n${external.stdout}\n${external.stderr}`,
  );
  const editedStamp = fs.statSync(document);

  // 4. No manual sync: the idle watcher must notice the save and rebuild the catalogue.
  const synced = await until(
    (v) =>
      v.ui.auto_sync.syncs >= 1 &&
      v.ui.auto_sync.pending === false &&
      v.ui.busy === false &&
      v.ui.preview_pending === 0 &&
      v.ui.previews_ready >= 1,
    "the automatic catalogue refresh",
  );
  assert.equal(synced.ui.auto_sync.error, null, "the automatic refresh must succeed");
  assert.match(String(synced.ui.message), /^已自动同步/);
  assert.equal(synced.ui.canvas.state, null, "an automatic refresh must not open an editor session");
  assert.equal(synced.ui.board_id, board.id, "the selected page must stay selected");

  const refreshedCache = cache();
  const refreshedStamp = sourceStamp();
  assert.notEqual(
    refreshedStamp.modified_ns,
    initialStamp.modified_ns,
    "the catalogue records the newly saved stamp",
  );
  assert.ok(refreshedStamp.len > 0n, "the stamp must describe the edited file");
  assert.notEqual(refreshedCache.previews[key], initialCache.previews[key], "a new frame is published");
  const refreshedPreview = path.join(path.dirname(cacheFile), refreshedCache.previews[key]);
  assert.notEqual(digest(refreshedPreview), initialHash, "the page bitmap must show the official edit");
  assert.equal(fs.existsSync(initialPreview), false, "the superseded preview file is removed");
  const storedMs = Number(refreshedStamp.modified_ns / 1_000_000n);
  assert.ok(
    editedStamp.mtimeMs <= storedMs + 2000,
    "the stored stamp must not lag behind the saved file",
  );

  await step({
    tool: "workcrate-ui.state",
    arguments: {},
    capture: "auto-preview-after-external-save",
  });

  // 5. The watcher settles: no repeated sync, no repaint of its own snapshot.
  await delay(3000);
  const settled = JSON.stringify((await read()).ui);
  await delay(2500);
  assert.equal(JSON.stringify((await read()).ui), settled, "an idle panel must stay quiet");
  assert.equal((await read()).ui.auto_sync.syncs, synced.ui.auto_sync.syncs);

  const result = {
    passed: true,
    port,
    project: project.id,
    board: board.id,
    document,
    autoSyncs: synced.ui.auto_sync.syncs,
    initialPreview,
    refreshedPreview: refreshedCache.previews[key],
    initialHash,
    refreshedHash: digest(refreshedPreview),
  };
  fs.writeFileSync(
    path.join(profile, "auto-preview-result.json"),
    JSON.stringify(result, null, 2),
  );
  console.log(JSON.stringify({ autoPreview: "passed", ...result }));
};
