// Same-process check: one dead project root must not blank another project's tree.
// Run twice with FILEATLAS_PROJECT_TREE_ORDER=live-first then gone-first so both
// the surviving tree and the dead root's own note are captured expanded.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, profile, output }) => {
  const invoke = async (tool, args = {}) => payload(await call(tool, args));
  const report = { profile, checks: [] };
  const verbatim = value => `\\\\?\\${value}`;
  const order = process.env.FILEATLAS_PROJECT_TREE_ORDER || 'live-first';
  assert(['live-first', 'gone-first'].includes(order), 'order is live-first or gone-first');

  const live = path.join(profile, 'tree-live');
  const gone = path.join(profile, 'tree-gone');
  const manifest = path.join(profile, 'project-tree', 'project-tree.json');
  const liveRoot = { path: verbatim(live), name: 'tree-live' };
  const goneRoot = { path: verbatim(gone), name: 'tree-gone' };
  const seeded = {
    version: 1,
    roots: order === 'live-first' ? [liveRoot, goneRoot] : [goneRoot, liveRoot],
  };

  // The debug client starts the app before a scenario runs, so the list has to
  // exist on disk in a separate first pass for the service to load it.
  if (process.env.FILEATLAS_SEED_PROJECT_TREE === '1') {
    fs.rmSync(gone, { recursive: true, force: true });
    fs.mkdirSync(path.join(live, 'docs', 'notes'), { recursive: true });
    fs.mkdirSync(path.join(live, 'src'), { recursive: true });
    fs.mkdirSync(path.dirname(manifest), { recursive: true });
    fs.writeFileSync(manifest, JSON.stringify(seeded, null, 2));
    console.log(JSON.stringify({ seeded: true, order, manifest }, null, 2));
    return;
  }

  assert.deepEqual(
    JSON.parse(fs.readFileSync(manifest, 'utf8')),
    seeded,
    `run the ${order} seed pass before driving the page`,
  );
  assert(!fs.existsSync(gone), 'the missing root must stay missing for this scenario');

  await invoke('control_ui', { action: 'sidebar_open' });
  await invoke('control_ui', { action: 'navigation_tab', value: 'project-tree' });
  let tab = '';
  for (let attempt = 0; attempt < 80 && tab !== 'project-tree'; attempt += 1) {
    tab = (await invoke('get_ui_state')).navigation_tab;
    if (tab !== 'project-tree') await delay(150);
  }
  assert.equal(tab, 'project-tree', 'the project tree page did not open');
  await delay(1500);

  const image = (await call('screenshot_window')).content.find(item => item.type === 'image');
  assert(image, 'FileAtlas MCP screenshot did not return an image');
  const screenshot = path.join(output, `project-tree-${order}.png`);
  fs.writeFileSync(screenshot, Buffer.from(image.data, 'base64'));

  assert.deepEqual(
    JSON.parse(fs.readFileSync(manifest, 'utf8')),
    seeded,
    'the scan must not rewrite the project list',
  );
  report.checks.push({
    page: 'project-tree',
    order,
    roots: seeded.roots.map(root => root.path),
    screenshot,
    expected: order === 'live-first'
      ? 'tree-live expanded shows docs/notes/src; tree-gone row carries 不可用 only'
      : 'tree-gone expanded shows one plain-path note and no os error text',
  });
  console.log(JSON.stringify(report, null, 2));
};
