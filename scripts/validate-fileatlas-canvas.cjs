const assert = require('node:assert/strict');
const path = require('node:path');

module.exports = async ({ call, payload, delay, step, profile }) => {
  const invoke = async (name, args = {}) => payload(await call(name, args));
  const wait = async predicate => {
    for (let i = 0; i < 40; i++) {
      if (await predicate()) return;
      await delay(150);
    }
    throw Error('Canvas scene did not settle');
  };
  const state = () => invoke('whiteboard.state');
  const board = await invoke('whiteboard.create', { title: 'FileAtlas Canvas MCP' });
  await delay(1200);
  await step({ tool: 'screenshot_window', capture: 'canvas-empty' });
  const ui = await invoke('get_ui_state');
  const panel = ui.panel_instances.find(item => item.id.startsWith('whiteboard/panel@'));
  assert(panel, 'FileAtlas canvas panel must be mounted');
  const initial = JSON.parse((await state()).library.boards.find(item => item.id === board.id).scene);
  const fixture = path.join(profile, 'canvas-fixture.png');
  require('node:fs').copyFileSync(path.resolve('test-data/fileatlas-fixtures/01-small/small.png'), fixture);
  await invoke('whiteboard.add_images', { id: board.id, paths: [fixture], at: [640, 420] });
  await wait(async () => {
    const current = JSON.parse((await state()).library.boards.find(item => item.id === board.id).scene);
    return current.elements.some(element => element.kind?.image);
  });
  await step({ tool: 'screenshot_window', capture: 'canvas-shape-layers' });

  const drawn = JSON.parse((await state()).library.boards.find(item => item.id === board.id).scene);
  assert(drawn.elements.length > initial.elements.length, 'host image placement must mutate the engine scene');
  assert(drawn.elements.some(element => element.kind?.image), 'engine scene must contain the placed image');

  const current = await state();
  const saved = current.library.boards.find(item => item.id === board.id);
  assert.equal(current.save.error, null);
  assert.equal(current.save.revision, current.save.saved_revision);
  await invoke('whiteboard.flush');
  await step({ tool: 'screenshot_window', capture: 'canvas-final' });
  const result = {
    passed: true,
    board_id: board.id,
    panel,
    first_element: drawn.elements[0],
    drawn_elements: drawn.elements.length,
    saved_scene_bytes: saved.scene.length,
    profile,
    cases: ['managed canvas panel and live bounds', 'tldraw-informed toolbar and inspector layout visible', 'host image placement and selection in the engine scene', 'versioned library save'],
  };
  require('node:fs').writeFileSync(path.join(profile, 'canvas-result.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result));
};
