// Same-process layout check at the application's supported minimum window size.
// Run with an isolated validation profile and the dev-test plugin enabled.
const fs = require('node:fs');
const path = require('node:path');

module.exports = async ({ step, delay, call, payload, profile }) => {
  const invoke = async (tool, arguments_ = {}, extra = {}) =>
    (await step({ tool, arguments: arguments_, ...extra })).result;
  const captureGeometry = async name => {
    const geometry = payload(await call('debug.ui_geometry'));
    const directory = path.join(profile, 'screenshots');
    fs.mkdirSync(directory, { recursive: true });
    fs.writeFileSync(path.join(directory, `${name}.geometry.json`), JSON.stringify(geometry, null, 2));
  };

  let state;
  for (let attempt = 0; attempt < 40; attempt++) {
    state = await invoke('get_ui_state');
    if (state.window_status?.geometry?.client_size) break;
    await delay(250);
  }
  if (!state?.window_status?.geometry?.client_size) {
    throw new Error('FileAtlas window geometry unavailable');
  }
  await invoke('resize_window', { width: 1920, height: 1080 }, { wait_ms: 500 });
  await delay(500);
  await invoke('get_ui_state', {}, { capture: 'minimum-window-main' });
  await captureGeometry('minimum-window-main');

  const panel = await invoke(
    'control_ui',
    { action: 'panel_open', value: 'clipboard-workbench/panel' },
    { wait_ms: 500 },
  );
  if (panel?.error) throw new Error(`could not open clipboard panel: ${panel.error}`);
  await delay(500);
  await invoke('get_ui_state', {}, { capture: 'minimum-window-clipboard' });
  await captureGeometry('minimum-window-clipboard');
  await invoke('control_ui', { action: 'panel_close', value: panel.instance_id }, { wait_ms: 300 });

  const launcher = await invoke(
    'control_ui',
    { action: 'panel_open', value: 'program-launcher/panel' },
    { wait_ms: 500 },
  );
  if (launcher?.error) throw new Error(`could not open launcher panel: ${launcher.error}`);
  await delay(500);
  await invoke('get_ui_state', {}, { capture: 'minimum-window-launcher' });
  await captureGeometry('minimum-window-launcher');
  await invoke('control_ui', { action: 'panel_close', value: launcher.instance_id }, { wait_ms: 300 });

  await invoke('control_ui', { action: 'settings_open' }, { wait_ms: 500 });
  await delay(300);
  await invoke('get_ui_state', {}, { capture: 'minimum-window-settings' });
  await captureGeometry('minimum-window-settings');
};

