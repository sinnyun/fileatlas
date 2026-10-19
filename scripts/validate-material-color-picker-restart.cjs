const assert = require('node:assert/strict');
module.exports = async ({ step, delay }) => {
  const invoke = async (tool, args = {}, extra = {}) =>
    (await step({ tool, arguments: args, wait_ms: 200, ...extra })).result;
  const ui = (action, value) => invoke('control_ui', { action, ...(value === undefined ? {} : { value }) });
  const click = async (x, y) => {
    for (const action of ['pointer_move', 'pointer_down', 'pointer_up']) await ui(action, `${x},${y}`);
  };
  await delay(1200);
  const state = await invoke('get_ui_state');
  assert(state.theme.id.includes('Material #3b82f6'));
  assert.equal(state.theme.mode, 'light');
  assert.equal((await invoke('material-theme.state')).preferences.seed, '#3b82f6');
  await invoke('resize_window', { width: 1920, height: 1200 }, { wait_ms: 500 });
  await ui('settings_open');
  await ui('settings_plugin_page', 'photag.material-theme');
  const page = await invoke('get_ui_state', {}, { capture: 'picker-restart' });
  const [x, y, w] = page.panels.find(p => p.id === 'settings').bounds.split(',').map(Number);
  await click(x + w - 130, y + 334);
  await delay(300);
  await invoke('get_ui_state', {}, { capture: 'picker-small-palette' });
};
