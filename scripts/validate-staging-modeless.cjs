// Same-process check that the modeless staging panel leaves FileManager input active.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, output }) => {
  const invoke = async (name, args = {}) => payload(await call(name, args));
  const pointer = async (action, point) => {
    for (let attempt = 0; ; attempt++) {
      try {
        await invoke('control_ui', { action, value: `${point[0]},${point[1]}` });
        return;
      } catch (error) {
        if (attempt >= 30 || !String(error).includes('pointer request is still pending')) throw error;
        await delay(100);
      }
    }
  };
  const root = process.env.FILEATLAS_TEST_LIBRARY;
  assert(root && fs.existsSync(path.join(root, 'alpha')));
  await invoke('dev-test.scan_fixture');
  await invoke('control_file_manager', { action: 'show_descendants', enabled: true });
  await invoke('control_file_manager', { action: 'navigate', value: root });
  for (let i = 0; i < 100; i++) {
    const state = await invoke('get_ui_state');
    if (state.file_manager?.current_directory?.toLowerCase().endsWith('library')) break;
    await delay(100);
  }
  await invoke('control_ui', { action: 'panel_open', value: 'file-staging/panel' });
  await delay(500);
  const open = await invoke('get_ui_state');
  assert(open.open_panel_ids?.some(id => id.startsWith('file-staging/panel@')));
  const image = await call('screenshot_window');
  const screenshot = image.content.find(item => item.type === 'image');
  assert(screenshot, 'window screenshot unavailable');
  fs.writeFileSync(path.join(output, 'staging-modeless-open.png'), Buffer.from(screenshot.data, 'base64'));
  // Click the alpha folder card to the left of the floating panel. This must
  // select it while the panel remains open, proving the background is not occluded.
  const geometry = open.window_status.geometry.screenshot_to_pointer;
  const point = [330, 170];
  await pointer('pointer_down', point);
  await pointer('pointer_up', point);
  for (let i = 0; i < 60; i++) {
    const state = await invoke('get_ui_state');
    if (state.file_manager?.selected_directory?.toLowerCase().endsWith('\\alpha')) {
      console.log(JSON.stringify({ result: 'modeless-interaction-passed', selected_directory: state.file_manager.selected_directory, open_panel_ids: state.open_panel_ids }));
      return;
    }
    await delay(100);
  }
  throw new Error(`FileManager did not select the folder with staging panel open (pointer geometry ${JSON.stringify(geometry)})`);
};
