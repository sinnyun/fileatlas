// Same-process FileAtlas MCP verification for the Everything SDK provider.
// All database/settings state is isolated under target/profile.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, profile, root, output }) => {
  const report = { profile, cases: [] };
  const invoke = async (name, args = {}, record = true) => {
    const result = payload(await call(name, args));
    if (record) {
      report.calls ??= [];
      report.calls.push({ name, args, result });
      fs.writeFileSync(path.join(profile, 'everything-search-report.json'), JSON.stringify(report, null, 2));
    }
    assert(!result?.error, JSON.stringify(result));
    return result;
  };

  let initialUi;
  for (let attempt = 0; attempt < 30; attempt++) {
    initialUi = await invoke('get_ui_state', {}, false);
    if (initialUi.window_status?.valid && initialUi.file_manager) break;
    await delay(200);
  }
  assert(initialUi.window_status?.valid && initialUi.file_manager, JSON.stringify(initialUi));
  report.cases.push('same-process FileAtlas window and FileManager view are ready');

  const initialLibrary = await invoke('list_library');
  // One user search must recover automatically when a newly started client
  // finishes its initial index, without sending repeated search requests.
  await invoke('control_file_manager', { action: 'search', value: 'small.png' });
  let indexedUi;
  let bootstrapKey;
  for (let attempt = 0; attempt < 600; attempt++) {
    indexedUi = await invoke('get_ui_state', {}, false);
    const nextKey = JSON.stringify([indexedUi.file_manager?.search_provider, indexedUi.file_manager?.matched_count, indexedUi.file_manager?.everything]);
    if (nextKey !== bootstrapKey) {
      bootstrapKey = nextKey;
      report.bootstrap_state = { provider: indexedUi.file_manager?.search_provider, count: indexedUi.file_manager?.matched_count, everything: indexedUi.file_manager?.everything };
      fs.writeFileSync(path.join(profile, 'everything-search-report.json'), JSON.stringify(report, null, 2));
      console.log(JSON.stringify({ bootstrap: report.bootstrap_state }));
    }
    if (indexedUi.file_manager?.everything?.index_loading) report.bootstrap_loading_observed = true;
    if (indexedUi.file_manager?.search_provider === 'everything' && indexedUi.file_manager.matched_count > 0) break;
    await delay(500);
  }
  assert.equal(indexedUi.file_manager?.search_provider, 'everything', JSON.stringify(indexedUi));
  assert(indexedUi.file_manager.matched_count > 0);
  const bootstrap = await invoke('search_media', { search: 'small.png', limit: 10 });
  await invoke('get_ui_state');
  assert.equal(bootstrap.search_provider, 'everything', JSON.stringify(bootstrap));
  assert(bootstrap.total > 0, JSON.stringify(bootstrap));
  report.cases.push(`Everything IPC bootstrap succeeds; registered roots before fixture=${initialLibrary.folders.length}`);

  const accepted = await invoke('dev_test_scan_fixture');
  assert(accepted.accepted, JSON.stringify(accepted));
  let devTest;
  for (let attempt = 0; attempt < 120; attempt++) {
    devTest = await invoke('dev_test_status', {}, false);
    if (!devTest.running) break;
    await delay(500);
  }
  assert.equal(devTest.running, false, JSON.stringify(devTest));
  assert.match(devTest.status, /扫描完成/, JSON.stringify(devTest));
  report.calls ??= [];
  report.calls.push({ name: 'dev_test_status', result: devTest });
  fs.writeFileSync(path.join(profile, 'everything-search-report.json'), JSON.stringify(report, null, 2));
  report.cases.push(`isolated dev-test fixture scan completed: ${devTest.status}`);

  const search = await invoke('search_media', { search: 'small.png', limit: 10 });
  const items = search.items ?? search.media ?? search.results;
  const state = await invoke('get_ui_state');
  assert.equal(search.search_provider, 'everything', JSON.stringify(search));
  assert(items?.some(item => item.full_path.toLowerCase().endsWith('small.png')), JSON.stringify(search));
  report.cases.push('search_media returned small.png through real Everything SDK IPC');

  // The new files live in a child directory that FileAtlas never browses/scans.
  // A cache-only SQL search cannot discover them.
  const fixture = path.join(profile, 'everything-fixture');
  const unvisited = path.join(fixture, 'unvisited');
  fs.mkdirSync(unvisited, { recursive: true });
  const probe = '!EverythingNative_搜索 [1].png';
  fs.copyFileSync(path.join(root, 'test-data/fileatlas-fixtures/01-small/small.png'), path.join(unvisited, probe));
  for (const suffix of ['alpha', 'beta']) {
    fs.writeFileSync(path.join(unvisited, `!EverythingNative_${suffix}.txt`), suffix);
  }
  const library = await invoke('list_library');
  let folder = library.folders.find(item => path.resolve(item.path).toLowerCase() === fixture.toLowerCase());
  if (!folder) folder = (await invoke('add_folder', { path: fixture, confirm: true })).folder;
  let first;
  for (let attempt = 0; attempt < 40; attempt++) {
    first = await invoke('search_media', { search: '!EverythingNative_', folder_id: folder.id, limit: 1 });
    if (first.total === 3) break;
    await delay(200);
  }
  assert.equal(first.total, 3, JSON.stringify(first));
  assert.equal(first.search_provider, 'everything');
  assert.equal(first.items.length, 1);
  const second = await invoke('search_media', { search: '!EverythingNative_', folder_id: folder.id, offset: 1, limit: 1 });
  assert.equal(second.total, 3);
  assert.equal(second.items.length, 1);
  assert.notEqual(first.items[0].id, second.items[0].id);
  const exact = await invoke('search_media', { search: probe, folder_id: folder.id, limit: 10 });
  assert.equal(exact.total, 1, JSON.stringify(exact));
  assert.equal(exact.search_provider, 'everything');
  assert.equal(exact.items[0].filename, probe);
  assert.equal(exact.items[0].missing, false);
  const textFiles = await invoke('search_media', { search: '!EverythingNative_', folder_id: folder.id, media_type: 'file', limit: 10 });
  assert.equal(textFiles.total, 2);
  const noMatch = await invoke('search_media', { search: '!EverythingNative_absent', folder_id: folder.id });
  assert.equal(noMatch.total, 0);
  report.cases.push('unvisited nested Unicode/operator-like filename discovered; folder scope, pagination, file type and zero results verified');

  await invoke('control_navigation', { action: 'all' });
  await delay(250);
  await invoke('control_file_manager', { action: 'search', value: probe });
  let searchUi;
  for (let attempt = 0; attempt < 60; attempt++) {
    searchUi = await invoke('get_ui_state', {}, false);
    if (searchUi.file_manager?.search === probe && searchUi.file_manager.matched_count === 1 && searchUi.file_manager.search_provider === 'everything') break;
    await delay(100);
  }
  assert.equal(searchUi.file_manager?.search, probe, JSON.stringify(searchUi));
  assert.equal(searchUi.file_manager?.matched_count, 1, JSON.stringify(searchUi));
  assert.equal(searchUi.file_manager?.search_provider, 'everything');
  await delay(1000);
  const settled = await invoke('get_ui_state');
  await delay(1200);
  const quiet = await invoke('get_ui_state');
  assert.equal(quiet.file_manager.snapshot_revision, settled.file_manager.snapshot_revision, 'quiet search must not reload repeatedly');
  report.cases.push('live top search renders real Everything result and remains quiet while idle');

  await invoke('control_file_manager', { action: 'search_scopes_open', enabled: true });
  let file_manager;
  for (let attempt = 0; attempt < 20; attempt++) {
    file_manager = await invoke('get_ui_state', {}, false);
    if (file_manager.file_manager?.search_scopes_open) break;
    await delay(100);
  }
  assert.equal(file_manager.file_manager?.search_scopes_open, true, JSON.stringify(file_manager));
  assert.equal(file_manager.file_manager.search_scopes.time, true, JSON.stringify(file_manager));
  await delay(350);
  const scopeScreenshot = await call('screenshot_window');
  const scopeImage = scopeScreenshot.content.find(item => item.type === 'image');
  assert(scopeImage, 'MCP scope screenshot returned no image');
  fs.writeFileSync(path.join(output, 'everything-search-scopes.png'), Buffer.from(scopeImage.data, 'base64'));
  report.search_scope_screenshot = path.join(output, 'everything-search-scopes.png');
  await invoke('control_file_manager', { action: 'search_scope', value: 'time' });
  for (let attempt = 0; attempt < 20; attempt++) {
    file_manager = await invoke('get_ui_state', {}, false);
    if (file_manager.file_manager?.search_scopes?.time === false) break;
    await delay(100);
  }
  assert.equal(file_manager.file_manager?.search_scopes?.time, false, JSON.stringify(file_manager));
  await invoke('control_file_manager', { action: 'search_scope', value: 'time' });
  report.cases.push('search scope popover opens and time scope can be toggled and restored');

  await invoke('control_file_manager', { action: 'search_scopes_open', enabled: false });
  await invoke('control_ui', { action: 'settings_open' });
  await invoke('control_ui', { action: 'settings_tab', value: 'software' });
  let software;
  for (let attempt = 0; attempt < 40; attempt++) {
    software = await invoke('get_ui_state');
    if (software.settings_software_check?.state === 'ready') break;
    await delay(250);
  }
  assert.equal(software.settings_software_check?.state, 'ready', JSON.stringify(software));
  assert(software.settings_software_check.software.some(item => item.name === 'Everything'), JSON.stringify(software));
  report.cases.push('Settings Software check reports Everything');

  const screenshot = await call('screenshot_window');
  const image = screenshot.content.find(item => item.type === 'image');
  assert(image, 'MCP screenshot returned no image');
  fs.writeFileSync(path.join(output, 'everything-settings-software.png'), Buffer.from(image.data, 'base64'));
  report.screenshot = path.join(output, 'everything-settings-software.png');
  report.final = { search_provider: software.file_manager.search_provider, settings_software_check: software.settings_software_check };
  fs.writeFileSync(path.join(profile, 'everything-search-report.json'), JSON.stringify(report, null, 2));
  return report;
};
