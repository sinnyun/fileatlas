// Same-process live Gallery validation for unsaved-version indicators.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const { execFileSync } = require('node:child_process');

module.exports = async ({ call, payload, delay, profile, output }) => {
  const invoke = async (tool, args = {}) => payload(await call(tool, args));
  const actual = process.env.PHOTAG_LORE_TEST_FILE;
  const file = actual || path.join(profile, 'fixture', 'version-badge.png');
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const initial = actual ? fs.readFileSync(file) : null;
  const initialStat = actual ? fs.statSync(file) : null;
  const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
  const report = { source: file, cases: [], screenshots: [] };
  async function capture(name) {
    const result = await call('screenshot_window');
    const image = result.content.find(block => block.type === 'image');
    assert(image, 'Missing screenshot');
    const target = path.join(output, `${name}.png`);
    fs.writeFileSync(target, Buffer.from(image.data, 'base64'));
    report.screenshots.push(target);
  }
  async function until(probe, label) {
    let state;
    for (let i = 0; i < 100; i++) {
      state = await invoke('get_ui_state');
      if (probe(state)) return state;
      await delay(250);
    }
    throw Error(`Timeout ${label}: ${JSON.stringify(state.gallery)}`);
  }
  const images = {};
  try {
    for (const color of ['green', 'blue', 'purple']) {
      const target = path.join(profile, `${color}.png`);
      execFileSync('ffmpeg', ['-y', '-f', 'lavfi', '-i', `color=c=${color}:s=160x90`, '-frames:v', '1', target], { stdio: 'ignore', windowsHide: true });
      images[color] = fs.readFileSync(target);
    }
    if (!actual) fs.writeFileSync(file, images.purple);
    const baseline = fs.readFileSync(file);
    await invoke('add_folder', { path: path.dirname(file), confirm: true });
    await invoke('control_gallery', { action: 'navigate', value: path.dirname(file) });
    await until(state => state.gallery?.current_directory?.toLowerCase().endsWith(path.dirname(file).toLowerCase()), 'directory navigation consumed');
    await invoke('control_gallery', { action: 'search', value: path.basename(file) });
    await until(state => state.gallery?.matched_count === 1, 'one searched file');
    const found = await invoke('search_media', { search: path.basename(file), limit: 100 });
    const media = found.items.find(item => item.filename === path.basename(file)
      && path.win32.normalize(item.full_path).toLowerCase().endsWith(path.win32.normalize(file).toLowerCase()));
    assert(media, 'Source not indexed');
    report.media_id = media.id;
    async function status(expected, badge) {
      const direct = await invoke('get_file_version_state', { media_id: media.id });
      assert.equal(direct.state.status, expected, 'Direct immutable-version classification');
      const ui = await until(state => {
        const item = state.gallery?.version_states?.find(item => item.media_id === media.id);
        return !state.gallery.version_states_loading && item?.state?.status === expected && item.badge_visible === badge;
      }, `automatic Gallery ${expected} badge=${badge}`);
      console.log(`version state: ${expected}, badge=${badge}`);
      report.cases.push({ status: expected, badge, current: direct.state.currentVersionId, matching: direct.state.matchingVersionId, snapshot_revision: ui.gallery.snapshot_revision });
      return ui;
    }
    await status('unversioned', false);
    const first = (await invoke('create_file_version', { media_id: media.id, title: '01 original', confirm: true })).version;
    await status('saved', false);
    fs.writeFileSync(file, images.blue);
    const dirty = await status('modified', true);
    await capture('unsaved-grid');
    const listedBefore = await invoke('list_file_versions', { media_id: media.id });
    assert.equal(listedBefore.versions.length, 1, 'Indicator must not create versions automatically');
    const pointer = process.env.PHOTAG_BADGE_POINTER;
    if (pointer) {
      for (const action of ['pointer_move', 'pointer_down', 'pointer_up']) {
        await invoke('control_ui', { action, value: pointer });
        await delay(action === 'pointer_move' ? 1000 : 400);
        if (action === 'pointer_move') await capture('unsaved-dot-tooltip');
      }
      await until(state => state.inspector_history.visible && state.gallery.selected_media_ids.includes(media.id), 'actual amber indicator click opens file History');
      report.badge_clicked = pointer;
      await capture('unsaved-history-opened-by-dot');
    } else {
      await invoke('control_ui', { action: 'select_media', value: String(media.id) });
      await invoke('control_ui', { action: 'inspector_open' });
      await invoke('control_ui', { action: 'inspector_tab', value: 'history' });
    }
    await until(state => state.inspector_history.visible, 'History page');
    await capture('unsaved-history');
    await invoke('control_gallery', { action: 'layout', value: 'list' });
    await until(state => state.gallery.layout === 'list', 'list view');
    await status('modified', true);
    await capture('unsaved-list');
    await invoke('control_gallery', { action: 'layout', value: 'grid' });
    const second = (await invoke('create_file_version', { media_id: media.id, title: '02 blue', confirm: true })).version;
    await status('saved', false);
    await invoke('activate_file_version', { media_id: media.id, version_id: first.versionId, confirm: true });
    assert(fs.readFileSync(file).equals(baseline));
    await status('saved', false); // Old 01 is saved even while branch.latest is 02.
    fs.writeFileSync(file, baseline); // Timestamp change alone must not warn.
    await status('saved', false);
    fs.writeFileSync(file, images.blue);
    const matched = await status('matches_saved_version', false);
    assert.equal(matched.gallery.version_states.find(item => item.media_id === media.id).state.matchingVersionId, second.versionId);
    await capture('matches-other-saved-version');
    fs.writeFileSync(file, images.green);
    assert.equal(images.green.length, images.blue.length, 'Same-size edit fixture');
    await status('modified', true);
    await invoke('create_file_version', { media_id: media.id, title: '03 green edit of 01', confirm: true });
    await status('saved', false);
    await capture('saved-no-dot');
    await delay(1500);
    const quiet1 = await invoke('get_ui_state');
    await delay(1000);
    const quiet2 = await invoke('get_ui_state');
    assert.equal(quiet1.gallery.snapshot_revision, quiet2.gallery.snapshot_revision, 'Idle Gallery reload');
    assert.equal(quiet2.gallery.version_states_loading, false, 'Idle state reader restarted');
    report.quiet = { before: quiet1.gallery.snapshot_revision, after: quiet2.gallery.snapshot_revision };
    report.dirty_geometry = dirty.window_status.geometry;
    const versions = await invoke('list_file_versions', { media_id: media.id });
    assert.equal(versions.versions.length, 3, 'Unexpected automatic or duplicate version');
  } finally {
    if (actual && initial && initialStat) {
      fs.writeFileSync(file, initial);
      fs.utimesSync(file, initialStat.atime, initialStat.mtime);
      report.restoration = { original: hash(initial), restored: hash(fs.readFileSync(file)) };
      assert.equal(report.restoration.original, report.restoration.restored);
    }
    fs.writeFileSync(path.join(profile, 'version-badges-report.json'), JSON.stringify(report, null, 2));
  }
  console.log(JSON.stringify(report));
};
