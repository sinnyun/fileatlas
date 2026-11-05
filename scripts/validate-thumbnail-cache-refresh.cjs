const fs = require('node:fs');
const path = require('node:path');

module.exports = async ({ call, delay, payload, root, profile }) => {
  await delay(10000);
  let state = payload(await call('get_ui_state'));
  if (!state.window_status?.valid || !state.file_manager) {
    throw new Error(`FileManager window is not ready: ${JSON.stringify(state.window_status)}`);
  }
  const scan = payload(await call('dev-test.scan_fixture'));
  if (!scan.accepted) throw new Error(`Fixture scan failed: ${JSON.stringify(scan)}`);
  let status;
  for (let attempt = 0; attempt < 60; attempt++) {
    await delay(500);
    status = payload(await call('dev-test.status'));
    if (!status.running) break;
  }
  if (status?.running) throw new Error('Fixture scan did not finish.');

  const folder = path.join(root, 'test-data', 'fileatlas-fixtures', '01-small');
  payload(await call('control_navigation', { action: 'select_path', value: folder }));
  await delay(1200);
  payload(await call('control_file_manager', { action: 'tools_open' }));
  await delay(400);
  const screenshots = {};
  for (const [name, action] of [
    ['current-view', 'refresh_current_thumbnails'],
    ['all-system-cache', 'refresh_all_thumbnails'],
  ]) {
    const accepted = payload(await call('control_file_manager', { action }));
    if (!accepted.accepted) throw new Error(`${action} was not accepted: ${JSON.stringify(accepted)}`);
    await delay(1200);
    state = payload(await call('get_ui_state'));
    if (!state.window_status?.valid) throw new Error(`Window closed after ${action}`);
    const screenshot = await call('screenshot_window');
    const image = screenshot.content.find((item) => item.type === 'image');
    if (!image) throw new Error(`No screenshot returned for ${action}`);
    const file = path.join(profile, 'screenshots', `${name}.png`);
    fs.writeFileSync(file, Buffer.from(image.data, 'base64'));
    screenshots[name] = { accepted, matched_count: state.file_manager?.matched_count, file };
  }
  fs.writeFileSync(
    path.join(profile, 'thumbnail-cache-refresh-result.json'),
    JSON.stringify({ status, screenshots }, null, 2),
  );
};
