// Real GPUI interaction through same-process FileAtlas MCP. Isolated dev-test only.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
module.exports = async ({step, delay, profile}) => {
  const invoke = async (tool, args = {}, extra = {}) =>
    (await step({tool, arguments: args, ...extra})).result;
  const state = () => invoke('get_ui_state');
  const settings = () => JSON.parse(fs.readFileSync(path.join(profile, 'settings.json'), 'utf8'));
  const key = value => invoke('control_ui', {action: 'key_press', value}, {wait_ms: 160});
  const click = async (x, y) => {
    for (const action of ['pointer_move', 'pointer_down', 'pointer_up']) {
      for (let retry = 0; ; retry++) {
        try {
          await invoke('control_ui', {action, value: `${x},${y}`}, {wait_ms: 180});
          break;
        } catch (error) {
          if (retry >= 5 || !String(error).includes('pointer request is still pending')) throw error;
          await delay(250);
        }
      }
    }
  };
  const shot = name => invoke('get_ui_state', {}, {capture: name});
  await delay(1200);
  await invoke('control_ui', {action: 'settings_open'}, {wait_ms: 250});
  await invoke('control_ui', {action: 'settings_tab', value: 'shortcuts'}, {wait_ms: 250});
  let ui;
  for (let i = 0; i < 50; i++) {
    ui = await state();
    if (ui.settings_tag_choices?.state === 'ready') break;
    await delay(200);
  }
  assert.equal(ui.settings_tag_choices?.state, 'ready', 'tag selectors must finish loading');
  const commands = (await invoke('command_list')).commands;
  const rows = commands.flatMap(command => command.keybindings.map(binding => ({command, binding})));
  const launcher = rows.findIndex(row => row.command.id === 'program-launcher/open');
  const clipboard = rows.findIndex(row => row.command.id === 'clipboard-workbench/open');
  assert(launcher >= 0 && clipboard >= 0, 'both real plugin contributions must render');
  const launcherId = rows[launcher].binding.id;
  const clipboardId = rows[clipboard].binding.id;
  const [x, y, width] = ui.panels.find(panel => panel.id === 'settings').bounds.split(',').map(Number);
  const right = x + width - 21;
  // Header 36, page padding 20, title/description 66, group header 28, row center 24.
  const rowY = index => y + 174 + index * 48;
  const capture = async index => {
    await click(right - 174, rowY(index));
    const observed = (await state()).settings_shortcuts;
    assert.equal(observed.binding_id, rows[index].binding.id, 'visible Change button must start this binding');
    assert.equal(observed.capturing_key, true);
  };
  await shot('normal');
  await capture(clipboard);
  await shot('recording');
  const beforeConflict = settings().keybindings ?? {};
  await key('ctrl-f');
  ui = await shot('conflict');
  assert(ui.settings_shortcuts.error.includes('程序启动台'), 'inline conflict must name occupying command');
  assert.equal(ui.settings_shortcuts.capturing_key, true, 'conflict must allow retry');
  assert.deepEqual(settings().keybindings ?? {}, beforeConflict, 'conflict must not save');
  await key('escape');
  ui = await state();
  assert(ui.settings_open, 'Escape cancels recording without closing Settings');
  assert.equal(ui.settings_shortcuts.capturing_key, false);
  assert.equal(ui.settings_shortcuts.error, '');
  await capture(launcher);
  await key('space');
  assert((await state()).settings_shortcuts.error.includes('修饰键'), 'invalid system chord must show inline error');
  await key('ctrl-alt-q');
  assert.equal(settings().keybindings[launcherId], 'Ctrl+Alt+Q');
  assert.equal((await state()).settings_shortcuts.capturing_key, false);
  await shot('saved');
  await click(right - 28, rowY(clipboard));
  assert.equal(settings().keybindings[clipboardId], '');
  await shot('disabled');
  await click(right - 92, rowY(clipboard));
  await click(right - 92, rowY(launcher));
  assert.equal(settings().keybindings[clipboardId], undefined);
  assert.equal(settings().keybindings[launcherId], undefined);
  await capture(clipboard);
  await click(right - 48, rowY(clipboard));
  assert.equal((await state()).settings_shortcuts.capturing_key, false, 'inline cancel button must work');
  await capture(clipboard);
  await invoke('control_ui', {action: 'settings_tab', value: 'appearance'}, {wait_ms: 250});
  assert.equal((await state()).settings_shortcuts.capturing_key, false, 'tab change must release capture');
  await invoke('control_ui', {action: 'settings_tab', value: 'shortcuts'}, {wait_ms: 250});
  await capture(clipboard);
  await invoke('control_ui', {action: 'settings_close'}, {wait_ms: 250});
  assert.equal((await state()).settings_shortcuts.capturing_key, false, 'close must release capture');
  await invoke('control_ui', {action: 'settings_open'}, {wait_ms: 250});
  const quietBefore = await state();
  await delay(700);
  const quietAfter = await state();
  assert.equal(quietBefore.file_manager.snapshot_revision, quietAfter.file_manager.snapshot_revision);
  assert.deepEqual(quietBefore.settings_tag_choices, quietAfter.settings_tag_choices, 'settled tag ReadModel must stay stable');
  await shot('final');
  // Shrink the native window through the project plugin, without OS input injection.
  await invoke('resize_window', {width: 1510, height: 1000}, {wait_ms: 400});
  ui = await shot('compact');
  const [sx, sy, sw, sh] = ui.panels.find(panel => panel.id === 'settings').bounds.split(',').map(Number);
  for (const value of ['10', '-1,10,-80', '10,10,5000', '10,10,NaN'])
    await step({tool: 'control_ui', arguments: {action: 'pointer_scroll', value}, expect_error: 'bounded'});
  await invoke('control_ui', {action: 'pointer_scroll', value: `${sx + sw - 100},${sy + sh - 100},-900`}, {wait_ms: 350});
  await shot('compact-scrolled');
  await click(sx + sw - 36, sy + sh - 84);
  await shot('compact-f9-menu');
  const summary = {passed: true, executableProcess: 'same-process MCP',
    checks: ['registered rows', 'recording', 'conflict preserves value', 'Escape',
      'modifier validation and retry', 'saved chord', 'disable', 'restore', 'cancel button',
      'tab/close cleanup', 'quiet ReadModel', 'compact window and bounded wheel scroll'],
    scale: ui.window_status.geometry.scale_factor};
  fs.writeFileSync(path.join(profile, 'layout-summary.json'), JSON.stringify(summary, null, 2));
};
