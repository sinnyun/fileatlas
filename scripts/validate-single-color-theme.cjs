// Same-process visual audit with every native theme role forced to one exact color.
const fs = require('node:fs');
const path = require('node:path');

module.exports = async ({ call, payload, step, delay, root, profile }) => {
  const state = async () => payload(await call('get_ui_state'));
  let ready = false;
  for (let i = 0; i < 30; i++) {
    const current = await state();
    if (current?.window_status?.valid && current.window_status.owned) { ready = true; break; }
    await delay(1000);
  }
  if (!ready) throw Error('Same-process MCP window did not become ready');
  const capture = async name => {
    const result = await call('screenshot_window');
    const image = result.content.find(item => item.type === 'image');
    if (!image) throw Error(`No screenshot returned for ${name}`);
    fs.writeFileSync(path.join(profile, 'screenshots', `${name}.png`), Buffer.from(image.data, 'base64'));
  };
  await capture('single-color-main');
  await step({ tool: 'dev-test.scan_fixture', wait_ms: 150 });
  let finished = false;
  for (let i = 0; i < 60; i++) {
    const status = payload(await call('dev-test.status'));
    if (status && !status.running && status.status?.task?.running !== true) { finished = true; break; }
    await delay(500);
  }
  if (!finished) throw Error('Fixture scan did not finish');
  const fixture = path.join(root, 'test-data', 'fileatlas-fixtures');
  await step({ tool: 'control_file_manager', arguments: { action: 'navigate', value: fixture }, wait_ms: 1200 });
  await step({ tool: 'control_file_manager', arguments: { action: 'show_descendants', enabled: true }, wait_ms: 800 });
  await step({ tool: 'control_file_manager', arguments: { action: 'grouping', value: 'none' }, wait_ms: 700 });
  await capture('single-color-populated-grid');
  await step({ tool: 'control_ui', arguments: { action: 'settings_open' }, wait_ms: 400 });
  await step({ tool: 'control_ui', arguments: { action: 'settings_plugin_page', value: 'fileatlas.material-theme' }, wait_ms: 700 });
  await capture('single-color-settings');
  const current = await state();
  fs.writeFileSync(path.join(profile, 'single-color-state.json'), JSON.stringify(current, null, 2));
};
