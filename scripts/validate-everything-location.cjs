// Isolated same-process verification of the search location switch.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, profile, output }) => {
  const invoke = async (tool, args = {}) => payload(await call(tool, args));
  const token = `FileAtlasDomain${Date.now()}`;
  const folder = path.join(profile, 'search-location', 'selected');
  const outside = path.join(profile, 'search-location', 'selected-sibling');
  fs.mkdirSync(path.join(folder, 'nested'), { recursive: true });
  fs.mkdirSync(outside, { recursive: true });
  for (const [directory, suffix] of [[folder, 'direct'], [path.join(folder, 'nested'), 'nested'], [outside, 'outside']]) {
    fs.writeFileSync(path.join(directory, `${token}-${suffix}.txt`), suffix);
  }
  const report = { token, cases: [], snapshots: [] };
  const save = () => fs.writeFileSync(path.join(profile, 'search-location-report.json'), JSON.stringify(report, null, 2));
  const wait = async (predicate, description) => {
    let state;
    for (let attempt = 0; attempt < 120; attempt++) {
      state = await invoke('get_ui_state');
      if (predicate(state)) return state;
      await delay(150);
    }
    save();
    assert.fail(`${description}: ${JSON.stringify(state.file_manager)}`);
  };
  const capture = async name => {
    await delay(250);
    const screenshot = await call('screenshot_window');
    const image = screenshot.content.find(item => item.type === 'image');
    assert(image);
    fs.writeFileSync(path.join(output, `${name}.png`), Buffer.from(image.data, 'base64'));
  };
  await wait(state => state.window_status?.valid && state.file_manager, 'window ready');
  await invoke('control_file_manager', { action: 'search', value: '' });
  await delay(200);
  await invoke('control_file_manager', { action: 'search_entire_computer', enabled: false });
  await delay(200);
  await invoke('control_file_manager', { action: 'navigate', value: folder });
  await wait(state => state.file_manager?.current_directory?.replace(/^\\\\\?\\/, '') === folder, 'directory navigation');
  await invoke('control_file_manager', { action: 'show_descendants', enabled: false });
  await delay(1000);
  await invoke('control_file_manager', { action: 'search', value: token });
  const local = await wait(state => state.file_manager?.search === token && state.file_manager.search_provider === 'everything' && state.file_manager.matched_count === 2, 'current folder search includes nested file');
  assert.equal(local.file_manager.search_entire_computer, false);
  report.cases.push('Current folder searches direct and nested files while excluding the sibling folder; Show subfolders is off');
  report.snapshots.push(local.file_manager);
  save();
  await capture('current-folder');
  await invoke('control_file_manager', { action: 'search_entire_computer', enabled: true });
  const global = await wait(state => state.file_manager?.search_entire_computer && state.file_manager.search_provider === 'everything' && state.file_manager.matched_count === 3, 'entire computer discovers unregistered sibling');
  report.cases.push('Entire computer finds all three real indexed files, including an unregistered sibling directory');
  report.snapshots.push(global.file_manager);
  save();
  await capture('entire-computer');
  // The switch center was measured on the preceding 3286x1831 same-process
  // screenshot. Convert physical screenshot pixels using this HWND's geometry.
  const geometry = global.window_status.geometry.screenshot_to_pointer;
  const point = `${(2848 - geometry.offset_x) / geometry.divide_by},${(86 - geometry.offset_y) / geometry.divide_by}`;
  await invoke('control_ui', { action: 'pointer_down', value: point });
  await delay(100);
  await invoke('control_ui', { action: 'pointer_up', value: point });
  await wait(state => !state.file_manager?.search_entire_computer && state.file_manager.matched_count === 2, 'actual switch click restores current folder');
  await delay(900);
  const quiet1 = await invoke('get_ui_state');
  await delay(1200);
  const quiet2 = await invoke('get_ui_state');
  assert.equal(quiet1.file_manager.snapshot_revision, quiet2.file_manager.snapshot_revision);
  report.cases.push('Actual GPUI button click switches back to two local hits; quiet state does not refresh');
  await invoke('control_file_manager', { action: 'search', value: '' });
  await wait(state => state.file_manager?.browsing && state.file_manager.current_directory?.replace(/^\\\\\?\\/, '') === folder, 'clearing search restores directory browser');
  report.cases.push('Clearing search restores original directory browsing');
  await invoke('control_file_manager', { action: 'search_entire_computer', enabled: true });
  await wait(state => state.file_manager?.search_entire_computer, 'save global preference');
  const settings = JSON.parse(fs.readFileSync(path.join(profile, 'settings.json'), 'utf8'));
  assert.equal(settings.search_entire_computer, true);
  report.cases.push('Search location is persisted in isolated settings');
  save();
};
