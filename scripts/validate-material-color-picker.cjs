const assert = require('node:assert/strict');
const fs = require('node:fs');

module.exports = async ({ step, delay, profile }) => {
  const invoke = async (tool, args = {}, extra = {}) =>
    (await step({ tool, arguments: args, wait_ms: 200, ...extra })).result;
  const ui = (action, value) => invoke('control_ui', { action, ...(value === undefined ? {} : { value }) });
  const click = async (x, y) => {
    for (const action of ['pointer_move', 'pointer_down', 'pointer_up']) await ui(action, `${x},${y}`);
  };
  const shot = name => invoke('get_ui_state', {}, { capture: name });
  const until = async check => {
    for (let i = 0; i < 60; i++) {
      const s = await invoke('get_ui_state');
      if (check(s)) return s;
      await delay(100);
    }
    throw Error('selected Material seed was not applied');
  };
  await delay(1200);
  await invoke('resize_window', { width: 3286, height: 1831 }, { wait_ms: 600 });
  const initial = await invoke('get_ui_state');
  if (initial.theme.mode === 'light') {
    await click(16, initial.window_status.geometry.client_size.height - 116);
    await until(s => s.theme.mode === 'dark');
  }
  await ui('settings_open');
  await ui('settings_plugin_page', 'fileatlas.material-theme');
  await delay(400);
  const page = await shot('picker-page');
  const [x, y, w] = page.panels.find(p => p.id === 'settings').bounds.split(',').map(Number);
  await click(x + w - 130, y + 334);
  await delay(350);
  await shot('picker-palette');
  const original = await invoke('material-theme.state');
  const originalJson = original.generated ? fs.readFileSync(original.output, 'utf8') : null;
  // Native palette, blue 500: measured in the same-process screenshot at 150% DPI.
  await click(x + w - 136, y + 611);
  await delay(250);
  await shot('picker-selected');
  assert.deepEqual((await invoke('material-theme.state')).preferences, original.preferences,
    'choosing a swatch must only edit the draft');
  if (originalJson !== null) assert.equal(fs.readFileSync(original.output, 'utf8'), originalJson);
  await click(x + w - 92, y + 613);
  await until(s => s.theme.id.includes('Material #3b82f6'));
  const generated = await invoke('material-theme.state');
  assert.equal(generated.preferences.seed, '#3b82f6');
  assert.equal(generated.generated, true);
  const set = JSON.parse(fs.readFileSync(generated.output, 'utf8'));
  assert.equal(set.themes.length, 2);
  await shot('picker-generated-dark');
  // Change mode through the real rail button, then inspect the same native picker in light mode.
  await ui('settings_close');
  let s = await invoke('get_ui_state');
  await click(16, s.window_status.geometry.client_size.height - 116);
  await until(s => s.theme.mode === 'light');
  await ui('settings_open');
  await ui('settings_plugin_page', 'fileatlas.material-theme');
  await delay(300);
  await click(x + w - 130, y + 334);
  await shot('picker-palette-light');
  // Preview a different swatch, cancel, then generate: hover must not change the seed.
  await ui('pointer_move', `${x + w - 136},${y + 491}`);
  await ui('key_press', 'escape');
  await click(x + w - 92, y + 613);
  assert.equal((await invoke('material-theme.state')).preferences.seed, '#3b82f6');
  await shot('picker-applied-light');
  await invoke('resize_window', { width: 1920, height: 1200 }, { wait_ms: 500 });
  await shot('picker-small-window');
  fs.writeFileSync(require('node:path').join(profile, 'picker-summary.json'), JSON.stringify({
    passed: true, selected: generated.preferences.seed, draftOnly: true,
    lightDark: true, hoverCancelPreservedSeed: true,
  }, null, 2));
};
