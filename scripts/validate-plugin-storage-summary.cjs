const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, step, profile }) => {
  const invoke = async (name, args = {}) => payload(await call(name, args));
  const control = async (action, value) =>
    invoke('control_ui', { action, ...(value === undefined ? {} : { value }) });

  await delay(1200);
  const before = await invoke('get_ui_state');
  assert(before.settings_plugins.runtime.length > 0, 'runtime plugin catalog should be available');
  await control('settings_open');
  await delay(250);
  await control('settings_tab', 'storage');
  await delay(1800);
  const page = await invoke('get_ui_state');
  assert.equal(page.settings_open, true);
  assert.equal(page.settings_tab, 'storage');
  await control('settings_storage_refresh');
  await delay(500);
  const refreshed = await invoke('get_ui_state');
  assert.equal(refreshed.settings_tab, 'storage', 'refresh should keep the aggregate page selected');
  await step({ tool: 'screenshot_window', capture: 'plugin-storage-summary' });
  console.log(JSON.stringify({
    passed: true,
    runtime_plugins: before.settings_plugins.runtime.length,
    settings_tab: page.settings_tab,
    profile,
    capture: 'plugin-storage-summary',
  }));
};
