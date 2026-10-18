const assert = require('node:assert/strict');
const fs = require('node:fs');

module.exports = async ({ call, payload, step, delay, output }) => {
  const control = async (action, value) => payload(await call('control_ui', { action, ...(value === undefined ? {} : { value }) }));
  const state = async () => payload(await call('get_ui_state'));
  const command = async (command_id) => payload(await call('command_run', { command_id }));
  const waitForPlugin = async (id, enabled) => {
    for (let index = 0; index < 40; index++) {
      const current = await state();
      const plugin = current.settings_plugins?.runtime?.find(item => item.id === id);
      if (plugin && plugin.enabled === enabled && !plugin.busy) return current;
      await delay(250);
    }
    throw new Error(`Plugin ${id} did not become ${enabled ? 'enabled' : 'disabled'}`);
  };

  await delay(1500);
  await control('settings_open');
  await delay(400);
  await control('settings_tab', 'plugins');
  await delay(300);
  let current = await state();
  assert.equal(current.settings_tab, 'plugins');
  assert.equal(current.settings_plugins.runtime.length, 3);
  const ids = current.settings_plugins.runtime.map(item => item.id).sort();
  assert.deepEqual(ids, ['photag.clipboard', 'photag.launcher', 'photag.screenshot']);
  await step({ tool: 'get_ui_state', capture: 'plugins-overview' });

  await control('settings_plugin_page', 'photag.launcher');
  await delay(250);
  current = await state();
  assert.equal(current.settings_tab, 'plugin');
  assert.equal(current.settings_plugin_id, 'photag.launcher');
  await step({ tool: 'get_ui_state', capture: 'launcher-settings' });

  await control('plugin_launch_mode', 'photag.launcher:panel');
  current = await state();
  assert.equal(current.settings_plugins.launch_modes['photag.launcher'], 'PanelOnly');
  await command('program-launcher/open');
  await delay(350);
  current = await state();
  assert.equal(current.panel_presentation, 'panel-only');
  assert.ok(current.open_panel_ids.some(id => id.startsWith('program-launcher/panel@')));
  await step({ tool: 'get_ui_state', capture: 'launcher-panel-only' });
  const launcherPanel = current.open_panel_ids.find(id => id.startsWith('program-launcher/panel@'));
  await control('panel_close', launcherPanel);
  await control('plugin_launch_mode', 'photag.launcher:workspace');

  await control('plugin_enabled', 'photag.clipboard:off');
  current = await waitForPlugin('photag.clipboard', false);
  let commands = payload(await call('command_list'));
  assert.ok(!commands.commands.some(item => item.id === 'clipboard-workbench/open'));
  assert.ok(!current.sidebar_plugin_buttons.some(item => item.id === 'clipboard-workbench/sidebar-button-view'));
  const disabledSidebarButtons = current.sidebar_plugin_buttons;
  await control('settings_close');
  await delay(1500);
  await step({ tool: 'get_ui_state', capture: 'clipboard-disabled-sidebar' });
  await control('settings_open');
  await delay(350);
  await control('plugin_enabled', 'photag.clipboard:on');
  current = await waitForPlugin('photag.clipboard', true);
  commands = payload(await call('command_list'));
  assert.ok(commands.commands.some(item => item.id === 'clipboard-workbench/open'));
  assert.ok(current.sidebar_plugin_buttons.some(item => item.id === 'clipboard-workbench/sidebar-button-view'));
  await step({ tool: 'get_ui_state', capture: 'plugins-restored' });

  await control('settings_plugin_page', 'photag.screenshot');
  await delay(900);
  current = await state();
  assert.equal(current.settings_plugin_id, 'photag.screenshot');
  assert.equal(current.settings_plugins.launch_modes['photag.screenshot'] ?? 'Direct', 'Direct');
  await step({ tool: 'get_ui_state', capture: 'screenshot-settings' });

  await control('settings_close');
  current = await state();
  assert.equal(current.settings_open, false);
  await control('plugin_enabled', 'photag.clipboard:off');
  current = await waitForPlugin('photag.clipboard', false);
  await delay(1500);
  current = await state();
  assert.ok(!current.sidebar_folders.plugin_buttons.includes('clipboard-workbench/sidebar-button-view'));
  await step({ tool: 'get_ui_state', capture: 'clipboard-disabled-sidebar-final' });
  fs.writeFileSync(`${output}/plugin-settings-result.json`, JSON.stringify({
    plugin_ids: ids,
    launcher_panel_presentation: 'panel-only',
    clipboard_stop_start: 'passed',
    sidebar_buttons_while_clipboard_stopped: disabledSidebarButtons,
    final_clipboard_enabled: current.settings_plugins.runtime.find(item => item.id === 'photag.clipboard').enabled,
    screenshot_default: current.settings_plugins.launch_modes['photag.screenshot'],
    final_runtime: current.settings_plugins.runtime,
  }, null, 2));
};
