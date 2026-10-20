// Validate grouped-subfolder drop routing and real per-file movement in an isolated profile.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, profile, output }) => {
  const root = process.env.FILEATLAS_TEST_LIBRARY;
  assert(root && fs.existsSync(path.join(root, 'alpha', 'drag-source.jpg')));
  const report = { root, profile, output, started: new Date().toISOString() };
  const invoke = async (name, args = {}) => payload(await call(name, args));
  const pathKey = value => value.replace(/^\\\\\?\\/, '').toLowerCase();
  await invoke('dev-test.scan_fixture');
  let status;
  for (let i = 0; i < 120; i++) {
    status = await invoke('dev-test.status');
    if (!status?.running) break;
    await delay(500);
  }
  report.scan = status;
  let query;
  for (let i = 0; i < 30; i++) {
    await delay(250);
    query = await invoke('file_manager.search_media', { search: 'drag-source.jpg', limit: 10 });
    if (query.items?.some(item => item.filename === 'drag-source.jpg')) break;
  }
  report.before = query;
  const media = query.items?.find(item => item.filename === 'drag-source.jpg');
  assert(media, 'fixture image was not indexed');
  const state = await invoke('get_ui_state');
  report.initial_grouping = state.file_manager?.grouping_mode;
  await invoke('control_file_manager', { action: 'show_descendants', enabled: true });
  let grouped;
  for (let i = 0; i < 30; i++) {
    await delay(250);
    grouped = await invoke('get_ui_state');
    if (grouped.file_manager?.grouping_mode === 'subfolder') break;
  }
  assert.equal(grouped.file_manager?.grouping_mode, 'subfolder', 'subfolder grouping request was not applied');
  await invoke('control_file_manager', { action: 'navigate', value: root });
  for (let i = 0; i < 30; i++) {
    await delay(250);
    grouped = await invoke('get_ui_state');
    if (grouped.file_manager?.groups?.some(([group]) => pathKey(group).endsWith('\\beta'))) break;
  }
  report.grouped = grouped.file_manager;
  assert.equal(grouped.file_manager?.grouping_mode, 'subfolder');
  const groupPath = grouped.file_manager?.groups?.map(([group]) => group).find(group => pathKey(group).endsWith('\\beta'));
  assert(groupPath,
    `destination was not exposed as a subfolder group: ${JSON.stringify({current: grouped.file_manager?.current_directory, groups: grouped.file_manager?.groups})}`);
  const destination = groupPath;
  const diskDestination = path.join(root, 'beta');
  const groupedShot = await call('screenshot_window');
  const groupedImage = groupedShot.content.find(item => item.type === 'image');
  assert(groupedImage, 'same-process FileManager screenshot was unavailable');
  fs.writeFileSync(path.join(output, 'subfolder-groups.png'), Buffer.from(groupedImage.data, 'base64'));
  report.move = await invoke('control_file_manager', {
    action: 'move_files_to_subfolder', media_ids: [media.id], destination,
  });
  for (let i = 0; i < 80 && !fs.existsSync(path.join(diskDestination, 'drag-source.jpg')); i++) await delay(250);
  assert(fs.existsSync(path.join(diskDestination, 'drag-source.jpg')), 'destination file was not created');
  assert(!fs.existsSync(path.join(root, 'alpha', 'drag-source.jpg')), 'source file still exists');
  let moved;
  for (let i = 0; i < 40; i++) {
    moved = await invoke('file_manager.search_media', { search: 'drag-source.jpg', limit: 10 });
    if (moved.items?.some(item => pathKey(item.full_path) === pathKey(path.join(diskDestination, 'drag-source.jpg')))) break;
    await delay(250);
  }
  report.after = moved;
  assert(moved.items?.some(item => pathKey(item.full_path) === pathKey(path.join(diskDestination, 'drag-source.jpg'))),
    `FileManager index did not report the moved file path: ${JSON.stringify(moved)}`);
  report.destination_files = fs.readdirSync(diskDestination);
  report.finished = new Date().toISOString();
  fs.writeFileSync(path.join(profile, 'subfolder-drop-report.json'), JSON.stringify(report, null, 2));
  console.log(JSON.stringify({ result: 'passed', source_removed: true, destination_indexed: true, report: path.join(profile, 'subfolder-drop-report.json') }));
};
