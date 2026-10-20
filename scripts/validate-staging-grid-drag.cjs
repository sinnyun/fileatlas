// Same-process view setup and screenshot probe for staging-card cross-window drag.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, profile, output }) => {
  const root = process.env.FILEATLAS_TEST_LIBRARY;
  assert(root && fs.existsSync(path.join(root, 'alpha', 'staged-photo.jpg')));
  const invoke = async (name, args = {}) => payload(await call(name, args));
  const report = { root, profile, output, started: new Date().toISOString() };
  const until = async (test, label) => {
    for (let attempt = 0; attempt < 100; attempt++) {
      const state = await invoke('get_ui_state');
      if (test(state)) return state;
      await delay(150);
    }
    throw Error(`Timed out waiting for ${label}`);
  };
  const screenshot = async (name) => {
    const result = await call('screenshot_window');
    const image = result.content.find(item => item.type === 'image');
    assert(image, `${name}: screenshot unavailable`);
    fs.writeFileSync(path.join(output, `${name}.png`), Buffer.from(image.data, 'base64'));
    return image;
  };

  await invoke('dev-test.scan_fixture');
  for (let attempt = 0; attempt < 120; attempt++) {
    if (!(await invoke('dev-test.status'))?.running) break;
    await delay(250);
  }
  await invoke('control_file_manager', { action: 'show_descendants', enabled: true });
  await until(state => state.file_manager?.grouping_mode === 'subfolder', 'subfolder grouping');
  await invoke('control_file_manager', { action: 'navigate', value: root });
  const grouped = await until(state => state.file_manager?.grouping_mode === 'subfolder'
    && Object.values(state.file_manager?.group_frames ?? {}).some(frame => frame.path.toLowerCase().endsWith('\\beta')),
  'visible destination folder group');
  report.file_manager = grouped.file_manager;

  const staged = path.join(root, 'alpha', 'staged-photo.jpg');
  await invoke('file-staging.clear');
  report.staging = await invoke('file-staging.add', {
    paths: [
      staged,
      path.join(process.cwd(), 'test-data/fileatlas-fixtures/01-small/small.png'),
      path.join(process.cwd(), 'test-data/fileatlas-fixtures/02-mixed-formats/medium.webp'),
      path.join(process.cwd(), 'test-data/fileatlas-fixtures/03-nested/level-one/level-two/nested-photo.jpeg'),
    ],
  });
  const opened = await invoke('control_ui', { action: 'panel_open', value: 'file-staging/panel' });
  report.panel_open = opened;
  await delay(700);
  const state = await invoke('get_ui_state');
  report.live_file_manager = state.file_manager;
  report.native_panels = state.native_panels;
  report.window_status = state.window_status;
  const panelImage = await screenshot('staging-grid-main');
  const imageBuffer = Buffer.from(panelImage.data, 'base64');
  report.image_dimensions = [imageBuffer.readUInt32BE(16), imageBuffer.readUInt32BE(20)];
  const frames = Object.values(state.file_manager.group_frames);
  const destinationFrame = frames.find(frame => frame.path.toLowerCase().endsWith('\\beta'));
  assert(destinationFrame, 'destination folder frame disappeared before drag');
  const geometry = report.window_status.geometry;
  const [width, height] = report.image_dimensions;
  const source = [
    (width * 0.39 - geometry.screenshot_to_pointer.offset_x) / geometry.screenshot_to_pointer.divide_by,
    (height * 0.52 - geometry.screenshot_to_pointer.offset_y) / geometry.screenshot_to_pointer.divide_by,
  ];
  const [x, y, frameWidth, frameHeight] = destinationFrame.bounds;
  // Land well inside the visible group frame (the frame shares its lower
  // edge with the next group and the viewport's content clip).
  const destination = [x + 80, y + Math.min(100, frameHeight - 20)];
  const pointer = async (action, point) => {
    for (let attempt = 0; ; attempt++) {
      try {
        await invoke('control_ui', { action, value: `${point[0]},${point[1]}` });
        break;
      } catch (error) {
        if (attempt >= 30 || !String(error).includes('pointer request is still pending')) throw error;
        await delay(100);
      }
    }
    await delay(350);
  };
  report.drag = { source, destination, destinationFrame };
  await pointer('pointer_down', source);
  await pointer('pointer_drag', [source[0] + 18, source[1] + 12]);
  await pointer('pointer_drag', destination);
  await screenshot('staging-grid-drop-hover');
  report.during_drag = await invoke('get_ui_state');
  await pointer('pointer_up', destination);
  for (let attempt = 0; attempt < 80 && fs.existsSync(staged); attempt++) await delay(150);
  report.after_drop_state = await invoke('get_ui_state');
  report.after_drop_basket = await invoke('file-staging.list');
  fs.writeFileSync(path.join(profile, 'staging-grid-probe.json'), JSON.stringify(report, null, 2));
  assert(!fs.existsSync(staged), 'release over the subfolder must move the staged file');
  assert(fs.existsSync(path.join(root, 'beta', 'staged-photo.jpg')), 'moved file was not created in the target folder');
  const remaining = await invoke('file-staging.list');
  assert(!remaining.items.some(item => item.path.toLowerCase().endsWith('staged-photo.jpg')),
    'successfully moved path must leave the staging basket');
  assert.equal(remaining.items.length, 3, 'other staged cards remain in the basket');
  report.after = { remaining: remaining.items, destination_files: fs.readdirSync(path.join(root, 'beta')) };
  await screenshot('staging-grid-after-drop');
  fs.writeFileSync(path.join(profile, 'staging-grid-probe.json'), JSON.stringify(report, null, 2));
  console.log(JSON.stringify({ result: 'drag-passed', drag: report.drag, report: path.join(profile, 'staging-grid-probe.json') }));
};
