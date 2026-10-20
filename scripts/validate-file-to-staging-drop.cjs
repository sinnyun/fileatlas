// Same-process GPUI drag from a FileManager card into the staging panel.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

module.exports = async ({ call, payload, delay, profile, output }) => {
  const root = process.env.FILEATLAS_TEST_LIBRARY;
  assert(root, 'FILEATLAS_TEST_LIBRARY is required');
  const invoke = async (name, args = {}) => payload(await call(name, args));
  let lastState;
  const wait = async test => {
    for (let i = 0; i < 120; i++) {
      const state = await invoke('get_ui_state');
      lastState = state;
      if (test(state)) return state;
      await delay(125);
    }
    throw new Error(`Timed out waiting for FileManager or panel state: ${JSON.stringify({ file_manager: lastState?.file_manager, open_panel_ids: lastState?.open_panel_ids })}`);
  };
  const pointer = async (action, [x, y]) => {
    for (let i = 0; ; i++) {
      try {
        await invoke('control_ui', { action, value: `${x},${y}` });
        break;
      } catch (error) {
        if (i >= 30 || !String(error).includes('pointer request is still pending')) throw error;
        await delay(100);
      }
    }
    await delay(350);
  };
  await invoke('dev-test.scan_fixture');
  for (let i = 0; i < 120 && (await invoke('dev-test.status'))?.running; i++) await delay(200);
  const fixtureDirectory = path.join(process.cwd(), 'test-data/fileatlas-fixtures/01-small');
  const sourceFile = path.join(fixtureDirectory, 'tiny.jpg');
  await invoke('control_file_manager', { action: 'navigate', value: fixtureDirectory });
  await wait(state => state.file_manager?.current_directory?.toLowerCase().endsWith('\\01-small')
    && state.file_manager?.loaded_count > 0);
  await invoke('control_file_manager', { action: 'debug_internal_file_drag', enabled: true });
  await invoke('file-staging.clear');
  await invoke('control_ui', { action: 'panel_open', value: 'file-staging/panel' });
  const state = await wait(current => current.open_panel_ids?.some(id => id.startsWith('file-staging/panel@'))
    && current.file_manager?.debug_internal_file_drag
    && current.panel_instances?.some(value => value.id.startsWith('file-staging/panel@') && value.bounds));
  assert(fs.existsSync(sourceFile), `fixture is missing: ${sourceFile}`);
  const panel = state.panel_instances?.find(value => value.id.startsWith('file-staging/panel@'));
  assert(panel?.bounds, 'staging panel bounds are required');
  const panelBounds = panel.bounds.split(',').map(Number);
  const source = [360, 200];
  const destination = [panelBounds[0] + panelBounds[2] / 2, panelBounds[1] + panelBounds[3] / 2];
  await pointer('pointer_down', source);
  await pointer('pointer_drag', [source[0] + 20, source[1] + 12]);
  await pointer('pointer_drag', destination);
  const screenshot = await call('screenshot_window');
  const image = screenshot.content.find(item => item.type === 'image');
  if (image) fs.writeFileSync(path.join(output, 'file-to-staging-drag-hover.png'), Buffer.from(image.data, 'base64'));
  await pointer('pointer_up', destination);
  let basket;
  for (let i = 0; i < 80; i++) {
    basket = await invoke('file-staging.list');
    if (basket.items.some(item => item.path.toLowerCase().endsWith('tiny.jpg'))) break;
    await delay(150);
  }
  assert(fs.existsSync(sourceFile), 'dragging into staging must preserve the source file');
  assert(basket.items.some(item => item.path.toLowerCase().endsWith('tiny.jpg')),
    'dropping a FileManager file onto staging must add its path');
  console.log(JSON.stringify({ result: 'file-to-staging-passed', sourceFile, stagedCount: basket.items.length, drag: { source, destination } }));
};
