// Region-separation UI evidence: border vs shadow, region colors, group gaps, favorites region.
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, request, profile, output }) => {
  const report = { profile, cases: [], screenshots: [], separations: [] };
  const save = () => fs.writeFileSync(path.join(profile, 'region-report.json'), JSON.stringify(report, null, 2));
  async function invoke(tool, args = {}) {
    const value = payload(await call(tool, args));
    assert(!value?.error, `${tool} failed: ${JSON.stringify(value)}`);
    return value;
  }
  const state = () => invoke('get_ui_state');
  async function until(test, label) {
    let last;
    for (let i = 0; i < 150; i++) { last = await state(); if (test(last)) return last; await delay(100); }
    throw Error(`state timed out: ${label}; last=${JSON.stringify({ groups: last.gallery?.groups, directory: last.gallery?.current_directory, matched: last.gallery?.matched_count, browsing: last.gallery?.browsing })}`);
  }
  async function sidebar(kind, args = {}) {
    await invoke('control_ui', { action: 'sidebar_request', request: { kind, ...args } });
    await delay(150);
  }
  async function capture(name) {
    const result = await call('screenshot_window');
    for (const block of result.content ?? []) if (block.type === 'image') fs.writeFileSync(path.join(output, name + '.png'), Buffer.from(block.data, 'base64'));
    report.screenshots.push(name + '.png'); save();
  }
  async function separation(mode, tag) {
    await invoke('control_ui', { action: 'region_separation', value: mode });
    const s = await until(x => x.region_separation === mode, `region_separation ${mode}`);
    report.separations.push({ mode, disk: JSON.parse(fs.readFileSync(path.join(profile, 'settings.json'), 'utf8')).region_separation, settings_tab: s.settings_tab });
    save();
    // Open and close the Settings panel: the panel toggle repaints the main window, so the
    // captures below show the applied separation instead of a stale paint.
    await invoke('control_ui', { action: 'settings_open' });
    await invoke('control_ui', { action: 'settings_tab', value: 'appearance' });
    await delay(400);
    await capture(`settings-${tag}`);
    await invoke('control_ui', { action: 'settings_close' });
    await delay(500);
    await capture(`sidebar-${tag}`);
    await capture(`gallery-${tag}`);
  }

  await delay(1500);
  const startup = await until(s => s.sidebar_folders, 'sidebar state');
  const expectedStartup = JSON.parse(fs.readFileSync(path.join(profile, 'settings.json'), 'utf8')).region_separation ?? 'border';
  assert.equal(startup.region_separation, expectedStartup, 'startup must apply settings.json');
  report.startup = expectedStartup;
  report.cases.push(`startup applied region_separation="${expectedStartup}" from settings.json into the theme token`);
  save();

  await invoke('resize_window', { width: 1942, height: 1256 });
  await delay(400);

  // Fixture: one browsed directory whose media sit in three different months.
  const root = path.join(profile, 'region-library');
  for (const dir of [root, path.join(root, 'evening'), path.join(root, 'scans')]) fs.mkdirSync(dir, { recursive: true });
  const sources = ['test-data/photag-library/01-small/small.png', 'test-data/photag-library/01-small/tiny.jpg', 'test-data/photag-library/02-mixed-formats/medium.webp'];
  const months = [new Date('2024-06-11T09:00:00Z'), new Date('2025-01-19T09:00:00Z'), new Date('2026-03-03T09:00:00Z')];
  const placed = [];
  for (const [month, time] of months.entries()) {
    sources.forEach((source, k) => {
      const target = path.join(root, `m${month}-${path.basename(source, path.extname(source))}-${k}${path.extname(source)}`);
      fs.copyFileSync(path.resolve(__dirname, '..', source), target);
      fs.utimesSync(target, time, time);
      placed.push(target);
    });
  }
  report.fixture = root;

  // Month bands make the new header gap and the raised group band visible.
  await invoke('control_gallery', { action: 'navigate', value: root });
  await until(s => s.browser.filesystem && s.gallery?.current_directory?.replace(/\\/g, '/').endsWith('region-library'), 'directory open');
  await invoke('control_gallery', { action: 'grouping', value: 'modified' });
  await until(s => (s.gallery?.groups?.length ?? 0) >= 3, 'month groups');
  report.groups = (await state()).gallery.groups;
  report.cases.push(`modified-month grouping produced ${report.groups.length} bands: ${JSON.stringify(report.groups)}`);
  await delay(1200);

  // Favorites region: three custom groups with real folders so tints and separators show.
  const groupNames = ['Work shots', 'Archive set', 'Phone imports'];
  for (const name of groupNames) {
    await sidebar('open_editor', { editor: { target: 'create_group' }, text: name });
    await sidebar('confirm_editor');
    await until(s => s.sidebar_folders.favorite_groups.some(g => g.name === name), `group ${name}`);
  }
  let s = await state();
  for (const [index, group] of s.sidebar_folders.favorite_groups.entries()) {
    await sidebar('add_favorite_folder', { group_id: group.id, path: [root, path.join(root, 'evening'), path.join(root, 'scans')][index % 3] });
    await until(x => x.sidebar_folders.favorite_groups.find(g => g.id === group.id).folders.length === 1, `folder in ${group.name}`);
  }
  await invoke('control_ui', { action: 'navigation_tab', value: 'folders' });
  await sidebar('toggle_section', { key: 'folders' });
  await sidebar('toggle_section', { key: 'drives' });
  await sidebar('toggle_section', { key: 'quick_access' });
  await until(s => s.sidebar_folders.drives_collapsed && s.sidebar_folders.quick_access_collapsed, 'upper sections collapsed');
  await delay(400);
  s = await state();
  assert.equal(s.sidebar_folders.favorite_groups.length, 3, 'three favorite groups persist');
  report.cases.push('sidebar owns one favorites region with three colored, separated groups');
  save();

  await separation('border', 'border');
  await separation('shadow', 'shadow');
  await separation('border', 'border-restored');

  // Collapse one group through the rendered semantic channel, then restore.
  const firstGroup = (await state()).sidebar_folders.favorite_groups[0].id;
  await sidebar('toggle_group', { group_id: firstGroup });
  await until(x => !x.sidebar_folders.favorite_groups.find(g => g.id === firstGroup).expanded, 'group collapsed');
  await capture('favorites-collapsed');
  await sidebar('toggle_group', { group_id: firstGroup });
  await until(x => x.sidebar_folders.favorite_groups.find(g => g.id === firstGroup).expanded, 'group expanded');
  report.cases.push('group collapse/expand still works inside the consolidated favorites region');

  // Quiet state: no repeated IO or repaint records while nothing changes.
  await invoke('control_ui', { action: 'region_separation', value: 'shadow' });
  await until(x => x.region_separation === 'shadow', 'shadow again');
  await invoke('control_debug', { action: 'open' });
  await delay(3000);
  const before = await invoke('get_debug_info', { category: '', search: '', page_size: 200 });
  await delay(3000);
  const after = await invoke('get_debug_info', { category: '', search: '', page_size: 200 });
  await invoke('control_debug', { action: 'close' });
  report.idle = { before: before.logs.latest_id, after: after.logs.latest_id };
  const lastPage = await invoke('get_debug_info', { category: '', search: '', page: Math.max(0, after.logs.pages - 1), page_size: 200 });
  report.idle_rows = lastPage.logs.records.slice(-8);
  assert.equal(after.logs.latest_id, before.logs.latest_id, 'idle must not repeat logs or repaint records');
  report.cases.push('quiet state after a real region-separation change adds no repeated IO/repaint records');

  // Real change through the same Settings write path the view uses: invalid value rejected.
  const invalid = await request('tools/call', { name: 'control_ui', arguments: { action: 'region_separation', value: 'glow' } });
  const invalidText = invalid.content?.find(item => item.type === 'text')?.text ?? '';
  assert(/border or shadow/.test(invalidText), `invalid value must be rejected, got ${invalidText}`);
  report.cases.push(`invalid region_separation value rejected: ${invalidText}`);
  await invoke('control_ui', { action: 'region_separation', value: 'border' });
  const final = await until(x => x.region_separation === 'border', 'final border');
  report.final = { separation: final.region_separation, groups: final.gallery.groups, favorites: final.sidebar_folders.favorite_groups.map(g => ({ name: g.name, folders: g.folders.length, expanded: g.expanded })) };
  save();
};
