const assert = require('node:assert/strict');

module.exports = async ({ call, payload, step, delay, profile }) => {
  const invoke = async (name, args = {}) => payload(await call(name, args));
  const control = async (action, value) =>
    invoke('control_ui', { action, ...(value === undefined ? {} : { value }) });

  await delay(1500);
  const plugins = await invoke('get_ui_state');
  assert(
    plugins.settings_plugins.runtime.some(plugin => plugin.id === 'fileatlas.whiteboard'),
    'canvas plugin should be registered in Settings',
  );
  await control('settings_open');
  await delay(300);
  for (const plugin of plugins.settings_plugins.runtime) {
    await control('settings_plugin_page', plugin.id);
    await delay(150);
    const selected = await invoke('get_ui_state');
    assert.equal(selected.settings_tab, 'plugin');
    assert.equal(selected.settings_plugin_id, plugin.id, `plugin settings page should open: ${plugin.id}`);
  }
  await control('settings_plugin_page', 'fileatlas.whiteboard');
  // Let the asynchronous, cached filesystem usage request settle before capture.
  await delay(1500);
  const page = await invoke('get_ui_state');
  assert.equal(page.settings_tab, 'plugin');
  assert.equal(page.settings_plugin_id, 'fileatlas.whiteboard');
  await step({ tool: 'screenshot_window', capture: 'whiteboard-storage-settings' });
  console.log(JSON.stringify({
    passed: true,
    plugin_id: page.settings_plugin_id,
    profile,
    capture: 'whiteboard-storage-settings',
  }));
};
