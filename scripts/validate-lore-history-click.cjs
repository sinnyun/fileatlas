// Prepare the real FileManager History tab for same-process pointer validation.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');

module.exports = async ({ call, payload, delay, profile, output }) => {
  async function invoke(tool, args = {}) { return payload(await call(tool, args)); }
  async function until(probe, label) {
    let state;
    for (let i = 0; i < 80; i++) {
      state = await invoke('get_ui_state');
      if (probe(state)) return state;
      await delay(250);
    }
    throw Error(`Timed out waiting for ${label}: ${JSON.stringify(state)}`);
  }
  async function capture(name) {
    const result = await call('screenshot_window');
    const image = result.content?.find(block => block.type === 'image');
    assert(image, 'FileAtlas MCP screenshot returned no image');
    const target = path.join(output, `${name}.png`);
    fs.writeFileSync(target, Buffer.from(image.data, 'base64'));
    return target;
  }
  const actualFile = process.env.FILEATLAS_LORE_TEST_FILE;
  const original = actualFile ? fs.readFileSync(actualFile) : null;
  const originalStat = actualFile ? fs.statSync(actualFile) : null;
  try {
  const source = actualFile ? path.dirname(actualFile) : path.join(profile, 'lore-version-fixture');
  fs.mkdirSync(source, { recursive: true });
  const baseImage = path.join(profile, 'click-base.png');
  const newerImage = path.join(profile, 'click-newer.png');
  for (const [color, target] of [['green', baseImage], ['blue', newerImage]]) {
    execFileSync('ffmpeg', ['-y', '-f', 'lavfi', '-i', `color=c=${color}:s=160x90`, '-frames:v', '1', target], { stdio: 'ignore', windowsHide: true });
  }
  const file = actualFile || path.join(source, 'history-fixture.png');
  if (!actualFile) fs.copyFileSync(baseImage, file);
  console.log(`validation fixture: ${file}`);
  const folder = await invoke('add_folder', { path: source, confirm: true });
  console.log('validation folder added');
  assert.equal(folder?.ok, true, JSON.stringify(folder));
  await invoke('control_file_manager', { action: 'navigate', value: source });
  await until(state => state.file_manager?.matched_count > 0, 'fixture FileManager results');
  console.log('validation FileManager results loaded');
  const found = await invoke('search_media', { limit: 100, sort: 'name_asc' });
  const item = found?.items?.find(media => media.path === file || media.filename === path.basename(file));
  assert(item, `restart fixture was not indexed: ${JSON.stringify(found)}`);
  const base = await invoke('create_file_version', { media_id: item.id, title: 'Pointer test base', confirm: true });
  console.log('validation base version created');
  assert.equal(base?.ok, true, JSON.stringify(base));
  fs.copyFileSync(newerImage, file);
  const newer = await invoke('create_file_version', { media_id: item.id, title: 'Pointer test newer', confirm: true });
  console.log('validation newer version created');
  assert.equal(newer?.ok, true, JSON.stringify(newer));
  await invoke('control_ui', { action: 'select_media', value: String(item.id) });
  await invoke('control_ui', { action: 'inspector_open' });
  await invoke('control_ui', { action: 'inspector_tab', value: 'information' });
  const before = await until(state => state.inspector_open && state.inspector_tab === 'information' && state.window_status.geometry && state.inspector_width, 'FileManager Inspector');
  const geometry = before.window_status.geometry;
  // On this view, the three equal-width inspector tabs occupy the top 38 px
  // navigation row below the preview. Target History's middle third using the
  // live client width and saved inspector width from MCP state.
  const x = Math.round(geometry.client_size.width - before.inspector_width / 2);
  const y = 368;
  await invoke('control_ui', { action: 'pointer_move', value: `${x},${y}` });
  await delay(350);
  await invoke('control_ui', { action: 'pointer_down', value: `${x},${y}` });
  await delay(350);
  await invoke('control_ui', { action: 'pointer_up', value: `${x},${y}` });
  const after = await until(
    state => state.inspector_tab === 'history' && state.inspector_history.enabled && state.inspector_history.count > 0,
    'actual History tab click and version ReadModel completion',
  );
  assert(after.inspector_history.count >= 1, `History view loaded without its versions: ${JSON.stringify(after.inspector_history)}`);
  const history = await invoke('list_file_versions', { media_id: item.id });
  console.log('validation History list loaded');
  const oldIndex = history.versions.findIndex(version => version.title === 'Pointer test base');
  assert(oldIndex >= 0, `No prior revision available to click: ${JSON.stringify(history.versions)}`);
  const oldVersion = history.versions[oldIndex];
  const newerIndex = history.versions.findIndex(version => version.title === 'Pointer test newer');
  assert(newerIndex >= 0, `Newer version missing before click validation: ${JSON.stringify(history.versions)}`);
  const newerVersion = history.versions[newerIndex];
  const rowY = 580 + oldIndex * 76;
  await invoke('control_ui', { action: 'pointer_move', value: `${x},${rowY}` });
  await delay(350);
  await invoke('control_ui', { action: 'pointer_down', value: `${x},${rowY}` });
  await delay(350);
  await invoke('control_ui', { action: 'pointer_up', value: `${x},${rowY}` });
  console.log('validation historical row clicked');
  await delay(1500);
  const switchX = Math.round(geometry.client_size.width - before.inspector_width / 4);
  await invoke('control_ui', { action: 'pointer_move', value: `${switchX},420` });
  await delay(350);
  await invoke('control_ui', { action: 'pointer_down', value: `${switchX},420` });
  await delay(350);
  await invoke('control_ui', { action: 'pointer_up', value: `${switchX},420` });
  await delay(1800);
  assert(fs.readFileSync(file).equals(original || fs.readFileSync(baseImage)), 'History button did not restore the selected source bytes');
  console.log('validation older version restored by actual click');
  const retained = await invoke('list_file_versions', { media_id: item.id });
  assert.equal(retained.versions.length, 2, 'Switching backward dropped a saved version');
  assert(retained.versions.some(version => version.versionId === oldVersion.versionId && version.isCurrent), 'Older version is not marked current');
  assert.equal(retained.versions[newerIndex].versionId, newerVersion.versionId, 'Version rows changed order after rewind');
  const rewoundState = await until(state => state.inspector_history.visible && state.file_manager.selected_media_ids.includes(item.id) && state.inspector_history.active === oldVersion.versionId && state.inspector_history.count === 2, 'rewound history UI refresh with retained file selection');
  await capture('history-after-rewind');
  console.log(`validation rewound UI: ${JSON.stringify(rewoundState.inspector_history)}`);
  const newerRowY = 580 + newerIndex * 76;
  await invoke('control_ui', { action: 'pointer_move', value: `${x},${newerRowY}` });
  await delay(350);
  await invoke('control_ui', { action: 'pointer_down', value: `${x},${newerRowY}` });
  await delay(350);
  await invoke('control_ui', { action: 'pointer_up', value: `${x},${newerRowY}` });
  await delay(1500);
  const selectedNewer = await invoke('get_ui_state');
  console.log(`validation newer row click: ${JSON.stringify(selectedNewer.inspector_history)}`);
  await invoke('control_ui', { action: 'pointer_move', value: `${switchX},420` });
  await delay(350);
  await invoke('control_ui', { action: 'pointer_down', value: `${switchX},420` });
  await delay(350);
  await invoke('control_ui', { action: 'pointer_up', value: `${switchX},420` });
  await delay(1800);
  assert(fs.readFileSync(file).equals(fs.readFileSync(newerImage)), 'History button did not restore the newer source bytes after rewind');
  console.log('validation newer version restored by actual click');
  const finalVersions = await invoke('list_file_versions', { media_id: item.id });
  assert.equal(finalVersions.versions.length, 2, 'Switching forward dropped a saved version');
  assert(finalVersions.versions.some(version => version.versionId === newerVersion.versionId && version.isCurrent), 'Newer version is not marked current');
  const finalState = await until(state => state.inspector_history.visible && state.file_manager.selected_media_ids.includes(item.id) && state.inspector_history.active === newerVersion.versionId && state.inspector_history.count === 2, 'history round-trip UI refresh with retained file selection');
  // The reported regression: finish a round trip at 01, edit it, then save 03
  // even though the shared branch's latest revision is still 02.
  await invoke('activate_file_version', { media_id: item.id, version_id: oldVersion.versionId, confirm: true });
  await until(state => state.inspector_history.visible && state.inspector_history.active === oldVersion.versionId, 'leave historical revision active before editing');
  const thirdImage = path.join(profile, 'click-third.png');
  execFileSync('ffmpeg', ['-y', '-f', 'lavfi', '-i', 'color=c=purple:s=160x90', '-frames:v', '1', thirdImage], { stdio: 'ignore', windowsHide: true });
  fs.copyFileSync(thirdImage, file);
  const third = await invoke('create_file_version', { media_id: item.id, title: 'Version 03 after editing 01', confirm: true });
  assert.equal(third.ok, true, 'Creating 03 after historical checkout failed');
  const thirdVersion = third.version;
  assert(thirdVersion.parentRevisionIds.includes(newerVersion.versionId), '03 did not append to the existing branch tip');
  assert(fs.readFileSync(file).equals(fs.readFileSync(thirdImage)), 'Creating 03 replaced the edited source');
  for (const [versionId, bytes] of [
    [oldVersion.versionId, original || fs.readFileSync(baseImage)],
    [newerVersion.versionId, fs.readFileSync(newerImage)],
    [thirdVersion.versionId, fs.readFileSync(thirdImage)],
  ]) {
    await invoke('activate_file_version', { media_id: item.id, version_id: versionId, confirm: true });
    assert(fs.readFileSync(file).equals(bytes), `Saved revision bytes changed: ${versionId}`);
  }
  const afterThird = await invoke('list_file_versions', { media_id: item.id });
  assert.equal(afterThird.versions.length, 3, 'Creating 03 lost an earlier saved revision');
  const thirdState = await until(state => state.inspector_history.visible && state.file_manager.selected_media_ids.includes(item.id) && state.inspector_history.active === thirdVersion.versionId && state.inspector_history.count === 3, 'three saved versions visible after editing historical snapshot');
  console.log('validation edited historical 01, saved 03, and restored all three revisions');
  await delay(1200);
  const quietBefore = await invoke('get_ui_state');
  await delay(1000);
  const quietAfter = await invoke('get_ui_state');
  assert.equal(quietAfter.file_manager.snapshot_revision, quietBefore.file_manager.snapshot_revision, 'Settled history view kept reloading FileManager');
  assert(quietAfter.inspector_history.visible && quietAfter.file_manager.selected_media_ids.includes(item.id), 'Settled watcher refresh lost the file History view');
  const screenshot = await capture('history-version-roundtrip-by-click');
  const report = {
    source_file: file,
    media_id: item.id,
    screenshot,
    edit_after_rewind: { created_version: thirdVersion, versions: afterThird.versions, ui: thirdState.inspector_history },
    quiet: { before_snapshot_revision: quietBefore.file_manager.snapshot_revision, after_snapshot_revision: quietAfter.file_manager.snapshot_revision, selected_media_ids: quietAfter.file_manager.selected_media_ids, visible: quietAfter.inspector_history.visible },
    pointer: {
      history_tab: { x, y },
      old_version_row: { x, y: rowY },
      switch_button: { x: switchX, y: 420 },
      geometry,
      inspector_width: before.inspector_width,
    },
    history: { before_switch: after.inspector_history, listed_versions: history.versions, old_version_clicked: oldVersion.versionId, retained_after_rewind: retained.versions, newer_version_clicked_after_rewind: newerVersion.versionId, after_roundtrip: finalState.inspector_history, final_versions: finalVersions.versions },
    cases: [
      'History tab opened by same-process GPUI pointer down/up; history ReadModel published live version count',
      'A historical revision row and the real Switch version button restored the original image bytes',
      'The newer revision row remained available after rewind, and a second real row/button click restored its bytes',
      'Editing 01 after a 01/02 round trip created 03, preserved 01/02, and all three restored exact bytes',
    ],
  };
  fs.writeFileSync(path.join(profile, 'lore-history-click-setup.json'), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report));
  } finally {
    if (actualFile && original && originalStat) {
      fs.writeFileSync(actualFile, original);
      fs.utimesSync(actualFile, originalStat.atime, originalStat.mtime);
    }
  }
};
