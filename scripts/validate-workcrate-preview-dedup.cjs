// Verifies one durable frame per page: the page editor renders straight into the cached
// preview file, a re-render replaces it, and no second mirror folder is written or kept.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const net = require('node:net');
const { createHash } = require('node:crypto');

const hash = (file) => createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const listing = (data) => {
  const dir = path.join(data, 'native-previews');
  return fs.existsSync(dir) ? fs.readdirSync(dir).sort() : [];
};
const cache = (data) =>
  JSON.parse(fs.readFileSync(path.join(data, 'boards-cache.json'), 'utf8'));
const referenced = (data) =>
  [...new Set(Object.values(cache(data).previews))]
    .map((relative) => relative.split('/').pop())
    .sort();
// The session reports the frame through the projects service, which hands back a Windows
// verbatim (\\?\) path; compare the same file regardless of that spelling.
const localPath = (value) =>
  path.normalize(String(value).replace(/^([\\/]{2}\?[\\/])/, ''));

module.exports = async ({ step, call, payload, delay, profile }) => {
  const invoke = async (tool, args = {}, extra = {}) =>
    (await step({ tool, arguments: args, wait_ms: 150, ...extra })).result;
  const read = async (ids = []) => payload(await call('workcrate-ui.state', { control_ids: ids }));
  const until = async (test, label, ids = [], attempts = 150) => {
    let s;
    for (let i = 0; i < attempts; i++) {
      s = await read(ids);
      if (s?.ui && test(s)) return s;
      await delay(150);
    }
    throw Error(`${label}: ${JSON.stringify(s)}`);
  };
  // A page pick that lands while the panel is still refreshing its catalogue is dropped by the
  // editor's own guard, so wait for an idle panel before re-entering (recorded as a defect).
  const idle = async () => {
    await until((s) => !s.ui.busy && !s.ui.canvas.busy, 'panel idle', [], 400);
    await delay(600);
  };

  const port = await new Promise((resolve) => {
    const s = net.createServer();
    s.listen(0, '127.0.0.1', () => {
      const p = s.address().port;
      s.close(() => resolve(p));
    });
  });
  await invoke('workcrate-ui.configure_vectorcraft', {
    executable: 'C:\\Program Files\\VectorCraft\\vectorcraft.exe',
    port,
  });

  const folder = path.join(profile, 'documents', '一帧一页');
  fs.mkdirSync(folder, { recursive: true });
  const created = await invoke('workcrate-ui.manage_project', {
    action: 'create',
    name: '一帧一页',
    directory: folder,
  });
  const project_id = created.project.id;
  const A = created.boards[0].id;
  const B = (await invoke('workcrate-ui.create_page', { project_id })).board_id;
  const data = path.join(folder, '.workcrate', project_id);

  // Creation and the second page each wrote one cached frame; nothing duplicated it.
  assert.ok(!fs.existsSync(path.join(data, 'canvas')), 'creating pages must not write a mirror folder');
  assert.deepEqual(
    listing(data),
    referenced(data),
    'the preview folder holds exactly the frames the cache names',
  );
  assert.equal(listing(data).length, 2, 'one frame per page');
  for (const name of listing(data)) {
    assert.match(
      name,
      /^page\d+-[0-9a-f]{8}-1600\.png$/,
      'a frame name is per page and association, not per render',
    );
  }
  const namesAfterCreate = listing(data);
  const frameOf = (boardId) =>
    path.join(
      data,
      'native-previews',
      cache(data).previews[`${boardId}@1600`].split('/').pop(),
    );
  const frameA = frameOf(A);
  const beforeEdit = hash(frameA);

  // The page grid reads the two cached frames without starting a session or rendering more.
  await invoke('workcrate-ui.navigate', { project_id });
  const grid = await until(
    (s) =>
      s.ui?.screen === 'pages' &&
      s.ui.page_count === 2 &&
      s.ui.previews_ready === 2 &&
      !s.ui.busy,
    'cached page grid',
    [],
  );
  assert.deepEqual(grid.ui.boards.map((b) => b.id), [A, B]);
  assert.deepEqual(
    listing(data),
    namesAfterCreate,
    'the page grid reads the cached frames without rendering another one',
  );
  await invoke('workcrate-ui.state', {}, { capture: 'dedup-page-grid' });

  // An editing session renders the page into its own durable file and reports that path.
  const opened = await invoke('workcrate-ui.canvas_open', { board_id: A });
  const reported = (opened.canvas ?? opened).preview;
  assert.ok(reported, 'the editing frame must be reported to the overlay');
  assert.equal(
    localPath(reported),
    path.normalize(frameA),
    'the editor renders the cached frame instead of a second copy',
  );
  await invoke('workcrate-ui.state', {}, { capture: 'dedup-session-frame' });

  // Editing the page re-renders: same file name, new pixels, no extra file, no growth.
  await invoke('workcrate-ui.update_page', {
    project_id,
    board_id: A,
    info: { material_category: '类别甲', material: '第一版物料', quantity: '1', extra: {} },
  });
  const afterFirstSave = hash(frameA);
  assert.notEqual(afterFirstSave, beforeEdit, 'the saved page must be re-rendered into its frame');
  assert.deepEqual(listing(data), namesAfterCreate, 'a re-render replaces its own file');

  await invoke('workcrate-ui.update_page', {
    project_id,
    board_id: A,
    info: { material_category: '类别甲', material: '第二版物料', quantity: '1', extra: {} },
  });
  assert.notEqual(hash(frameA), afterFirstSave, 'the second edit replaces the same frame again');
  assert.deepEqual(listing(data), namesAfterCreate, 'repeated edits never add a file');

  // Browsing pages re-renders each page into its own single file.
  await invoke('workcrate-ui.canvas_open', { board_id: B });
  await invoke('workcrate-ui.canvas_open', { board_id: A });
  assert.deepEqual(listing(data), namesAfterCreate, 'page switching reuses the existing frames');
  await invoke('workcrate-ui.canvas_close', { project_id });

  // A dataset produced by an older release still carries the unreclaimable mirror.
  fs.mkdirSync(path.join(data, 'canvas'), { recursive: true });
  fs.writeFileSync(path.join(data, 'canvas', '1.png'), 'legacy mirror');
  await invoke('workcrate-ui.update_page', {
    project_id,
    board_id: B,
    info: { material_category: '类别乙', material: '回收物料', quantity: '2', extra: {} },
  });
  assert.ok(
    !fs.existsSync(path.join(data, 'canvas')),
    'the first publication after an upgrade removes the duplicate mirror',
  );
  assert.deepEqual(listing(data), namesAfterCreate, 'cleanup leaves exactly one frame per page');
  assert.deepEqual(
    referenced(data),
    namesAfterCreate,
    'every cached reference still names an existing frame',
  );

  await invoke('workcrate-ui.navigate', { project_id });
  const pages = await until(
    (s) =>
      s.ui?.screen === 'pages' &&
      s.ui.page_count === 2 &&
      s.ui.previews_ready === 2 &&
      !s.ui.busy,
    'cached pages',
    [],
    400,
  );
  assert.deepEqual(pages.ui.boards.map((b) => b.id), [A, B]);
  assert.deepEqual(
    listing(data),
    namesAfterCreate,
    'the page grid reads the cached frames without rendering another one',
  );
  await idle();
  await invoke('workcrate-ui.open_board', { project_id, board_id: A });
  const shown = await until(
    (s) => s.ui.board_id === A && !s.ui.busy && !s.ui.canvas.busy && s.ui.canvas?.state?.preview,
    'editor reopened',
    [],
    400,
  );
  await invoke('workcrate-ui.state', {}, { capture: 'dedup-editor-reopened' });
  assert.equal(
    localPath(shown.ui.canvas.state.preview),
    path.normalize(frameA),
    'the reopened editor points at the same single frame',
  );
  assert.deepEqual(listing(data), namesAfterCreate, 'reopen added no frame');

  // An idle panel must not keep rendering: the frame is written when its page changes.
  const quiet = await read();
  await delay(3500);
  const settled = await read();
  assert.equal(settled.ui.reloads, quiet.ui.reloads, 'an idle editor does not re-render');
  assert.deepEqual(listing(data), namesAfterCreate, 'the idle window adds no frame');

  fs.writeFileSync(
    path.join(profile, 'dedup-summary.json'),
    JSON.stringify(
      {
        project_id,
        A,
        B,
        frames: namesAfterCreate,
        checks: [
          'no mirror folder at creation',
          'one frame per page named per page and association',
          'page grid reads the cached frames without rendering',
          'editing session renders the cached file itself',
          'two field saves replace one file with new pixels',
          'page switching adds no file',
          'legacy mirror removed by the first publication',
          'offline card reads and editor reopen',
          'idle panel renders nothing new',
        ],
      },
      null,
      2,
    ),
  );
};
