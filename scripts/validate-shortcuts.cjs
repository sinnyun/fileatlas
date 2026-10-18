// Isolated same-process validation of plugin shortcut discovery and Settings capture.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
module.exports = async ({call, payload, step, delay, profile}) => {
  const settingsPath = path.join(profile, 'settings.json');
  const saved = JSON.parse(fs.readFileSync(settingsPath, 'utf8'));
  const expectedBefore = saved.keybindings?.['program-launcher/shortcut-open'] ?? 'Ctrl+F';
  const commandList = async () => payload(await call('command_list'));
  const initial = await commandList();
  const launcher = initial.commands.find(command => command.id === 'program-launcher/open');
  const clipboard = initial.commands.find(command => command.id === 'clipboard-workbench/open');
  assert(launcher?.keybindings?.some(binding => binding.key === 'Ctrl+F' && binding.scope === 'system'), 'launcher default system binding missing');
  assert(clipboard?.keybindings?.some(binding => binding.key === 'Ctrl+Shift+V' && binding.scope === 'system'), 'clipboard default system binding missing');
  assert.equal(launcher.keybindings.find(binding => binding.id === 'program-launcher/shortcut-open').effective_key, expectedBefore, 'settings override was not restored at startup');

  await delay(1500);
  await step({tool: 'control_ui', arguments: {action: 'settings_open'}, wait_ms: 200});
  await step({tool: 'control_ui', arguments: {action: 'settings_tab', value: 'shortcuts'}, wait_ms: 300});
  if (expectedBefore !== 'Ctrl+Alt+Q') {
    const state = (await step({tool: 'get_ui_state'})).result;
    const bounds = state.panels.find(panel => panel.id === 'settings')?.bounds;
    assert(bounds, 'Settings panel bounds unavailable');
    const [x, y, width] = bounds.split(',').map(Number);
    const rows = initial.commands.flatMap(command => command.keybindings.map(binding => ({command, binding})));
    const rowIndex = rows.findIndex(row => row.binding.id === 'program-launcher/shortcut-open');
    assert(rowIndex >= 0);
    const pointer = `${x + width - 195},${y + 174 + rowIndex * 48}`;
    for (const action of ['pointer_move', 'pointer_down', 'pointer_up']) {
      await step({tool: 'control_ui', arguments: {action, value: pointer}, wait_ms: 200});
    }
    await step({tool: 'control_ui', arguments: {action: 'key_press', value: 'ctrl-alt-q'}, wait_ms: 500});
  }
  const settings = JSON.parse(fs.readFileSync(settingsPath, 'utf8'));
  assert.equal(settings.keybindings?.['program-launcher/shortcut-open'], 'Ctrl+Alt+Q', 'captured system binding was not persisted in the isolated profile');
  const updated = await commandList();
  const updatedLauncher = updated.commands.find(command => command.id === 'program-launcher/open');
  assert.equal(updatedLauncher.keybindings.find(binding => binding.id === 'program-launcher/shortcut-open').effective_key, 'Ctrl+Alt+Q', 'effective system binding did not update');
  const finalState = await step({tool: 'get_ui_state', capture: 'settings-shortcuts-updated'});
  assert.equal(finalState.result.settings_open, true, 'Settings panel did not remain open');
};
