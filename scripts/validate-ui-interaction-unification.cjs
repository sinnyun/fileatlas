// Same-process light-theme interaction review at the user's required 3000×2000 size.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, step, delay, profile, output }) => {
  const invoke = async (tool, arguments_ = {}) => payload(await call(tool, arguments_));
  const save = (name, value) => fs.writeFileSync(path.join(profile, name), JSON.stringify(value, null, 2));
  const settingsPath = path.join(profile, 'settings.json');
  const settings = JSON.parse(fs.readFileSync(settingsPath, 'utf8'));
  assert.equal(settings.color_mode, 'light', 'interaction review must use light mode');

  await delay(1500);
  await step({ tool: 'resize_window', arguments: { width: 3000, height: 2000 }, wait_ms: 800, capture: 'interaction-main-default' });
  const firstShot = fs.readFileSync(path.join(output, 'interaction-main-default.png'));
  const dimensions = { width: firstShot.readUInt32BE(16), height: firstShot.readUInt32BE(20) };
  assert.deepEqual(dimensions, { width: 3000, height: 2000 }, 'screenshot must match the requested test window');
  const state = await invoke('get_ui_state');
  const actualWindow = state.window_status?.geometry;
  save('window-size.json', { requested: { width: 3000, height: 2000 }, screenshot: dimensions, actual: actualWindow, mode: settings.color_mode });
  assert(actualWindow, 'same-process window geometry must be available');

  let geometry = await invoke('debug.ui_geometry');
  save('interaction-main.geometry.json', geometry);
  const entries = geometry.entries ?? [];
  const rails = entries.filter(entry => entry.tier === 'navigation' && entry.height_token >= 53 && entry.height_token <= 55);
  assert(rails.length >= 4, `expected shared rail shortcut geometry, saw ${rails.length}`);
  const pointer = async (action, entry) => {
    const args = {
      action,
      value: `${entry.x + entry.width / 2},${entry.y + entry.height / 2}`,
    };
    for (let attempt = 0; ; attempt++) {
      try { return await invoke('control_ui', args); }
      catch (error) {
        if (attempt >= 30 || !String(error).includes('pointer request is still pending')) throw error;
        await delay(100);
      }
    }
  };
  await pointer('pointer_move', rails[0]);
  await delay(250);
  await step({ tool: 'get_ui_state', capture: 'interaction-rail-hover' });
  await pointer('pointer_down', rails[0]);
  await delay(150);
  await step({ tool: 'get_ui_state', capture: 'interaction-rail-pressed' });
  await pointer('pointer_up', rails[0]);
  await delay(250);
  await step({ tool: 'get_ui_state', capture: 'interaction-rail-restored' });
  await pointer('pointer_down', rails[Math.min(1, rails.length - 1)]);
  await pointer('pointer_up', rails[Math.min(1, rails.length - 1)]);
  await delay(300);
  await step({ tool: 'get_ui_state', capture: 'interaction-rail-selected' });
  geometry = await invoke('debug.ui_geometry');
  save('interaction-selected.geometry.json', geometry);

  await invoke('control_ui', { action: 'settings_open' });
  await delay(500);
  await step({ tool: 'get_ui_state', capture: 'interaction-settings-default' });
  await invoke('control_ui', { action: 'settings_tab', value: 'appearance' });
  await delay(250);
  await step({ tool: 'get_ui_state', capture: 'interaction-settings-appearance' });
  await invoke('control_ui', { action: 'settings_close' });

  for (const [panel, name] of [
    ['clipboard-workbench/panel', 'clipboard'],
    ['program-launcher/panel', 'launcher'],
  ]) {
    const opened = await invoke('control_ui', { action: 'panel_open', value: panel });
    assert(opened?.instance_id, `panel did not open: ${panel}`);
    await delay(400);
    await step({ tool: 'get_ui_state', capture: `interaction-${name}` });
    await invoke('control_ui', { action: 'panel_close', value: opened.instance_id });
    await delay(200);
  }
  save('interaction-coverage.json', {
    result: 'partial',
    checked: ['light mode', '3000×2000 window', 'rail normal/hover/pressed/restored/selected', 'settings', 'clipboard panel', 'launcher panel'],
    pending: ['native button focus and disabled states across all plugin panels', 'popup/menu/checkbox/input focus states', 'WorkCrate panel states requiring VectorCraft fixture process'],
    rail_records: rails.length,
  });
  console.log('UI interaction unification scenario captured in light mode at 3000×2000');
};
