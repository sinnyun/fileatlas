const fs = require('node:fs');
const path = require('node:path');

module.exports = async ({ call, delay, payload, root, profile }) => {
  // Wait for the GPUI main window; MCP initializes before the window is mounted.
  await delay(10000);
  const initial = payload(await call('get_ui_state'));
  if (!initial.window_status?.valid || !initial.file_manager) {
    throw new Error(`FileManager window is not ready: ${JSON.stringify(initial.window_status)}`);
  }

  const scan = payload(await call('dev-test.scan_fixture'));
  if (!scan.accepted) throw new Error(`Fixture scan was not accepted: ${JSON.stringify(scan)}`);
  let scanStatus;
  for (let attempt = 0; attempt < 60; attempt++) {
    await delay(500);
    scanStatus = payload(await call('dev-test.status'));
    if (!scanStatus.running) break;
  }
  if (scanStatus?.running) throw new Error(`Fixture scan did not finish: ${JSON.stringify(scanStatus)}`);

  const modes = ['grid', 'waterfall', 'justified', 'list'];
  const folders = [
    '01-small',
    '02-mixed-formats',
    path.join('03-nested', 'level-one', 'level-two'),
  ];
  const results = { scan: scanStatus.status, folders: {} };
  for (const folder of folders) {
    const folderPath = path.join(root, 'test-data', 'fileatlas-fixtures', folder);
    const navigation = payload(await call('control_navigation', {
      action: 'select_path', value: folderPath,
    }));
    await delay(1200);
    results.folders[folder] = { navigation, layouts: {} };
    for (const mode of modes) {
      const changed = payload(await call('control_file_manager', {
        action: 'layout', value: mode,
      }));
      await delay(1000);
      const state = payload(await call('get_ui_state'));
      if (state.file_manager?.layout !== mode) {
        throw new Error(`Expected ${mode}, got ${state.file_manager?.layout}`);
      }
      const screenshot = await call('screenshot_window');
      const image = screenshot.content.find((item) => item.type === 'image');
      if (!image) throw new Error(`MCP returned no screenshot for ${folder}/${mode}`);
      const screenshotName = folder.replace(/[\\/]+/g, '-');
      fs.writeFileSync(
        path.join(profile, 'screenshots', `${screenshotName}-${mode}.png`),
        Buffer.from(image.data, 'base64'),
      );
      results.folders[folder].layouts[mode] = {
        changed,
        matched_count: state.file_manager.matched_count,
        card_states: state.file_manager_cards,
        gallery_options: state.file_manager.gallery_options,
      };
    }
  }
  fs.writeFileSync(path.join(profile, 'gallery-layouts-result.json'), JSON.stringify(results, null, 2));
};
