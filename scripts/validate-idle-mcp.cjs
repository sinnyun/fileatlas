// Run via photag-mcp-debug.cjs --scenario scripts/validate-idle-mcp.cjs.
// All application interaction is same-process PhoTag MCP. Fixtures/profile
// writes stay under target; source test-data and normal user data stay intact.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, root, profile, output, exe }) => {
  const report = { started: new Date().toISOString(), profile, executable:exe,
    executable_sha256: require('node:crypto').createHash('sha256').update(fs.readFileSync(exe)).digest('hex'),
    calls: [], cases: [], blockers: [] };
  const destination = path.join(profile, 'idle-report.json');
  const save = () => fs.writeFileSync(destination, JSON.stringify(report, null, 2));
  async function ensureWindow(state) {
    const status = state.window_status;
    if (!status) return state;
    assert(status.valid && status.owned, 'the tested native window is unavailable');
    if (status.minimized) {
      (report.window_interruptions ??= []).push({ time: new Date().toISOString(), status });
      await call('control_ui', { action: 'window_restore' });
      await delay(4000);
      state = payload(await call('get_ui_state'));
      assert(!state.window_status.minimized, 'native test window remained minimized');
    }
    return state;
  }
  async function invoke(tool, args = {}) {
    const started = Date.now();
    try {
      if (tool.startsWith('control_')) await ensureWindow(payload(await call('get_ui_state')));
      const result = payload(await call(tool, args));
      report.calls.push({ time: new Date().toISOString(), tool, args, duration_ms: Date.now() - started });
      // Feature request queues are single-slot and consumed by 100 ms pollers.
      if (tool.startsWith('control_')) await delay(250);
      return result;
    } catch (error) {
      report.calls.push({ time: new Date().toISOString(), tool, args, error: String(error) });
      save(); throw error;
    }
  }
  async function ui(action, value) { await invoke('control_ui', { action, value }); }
  async function gallery(action, more = {}) { await invoke('control_gallery', { action, ...more }); }
  async function preview(action, more = {}) { await invoke('control_preview', { action, ...more }); }
  async function navigation(action, more = {}) { await invoke('control_navigation', { action, ...more }); }
  async function debug(action, value) { await invoke('control_debug', { action, value }); }
  async function waitFor(tool, field, predicate, count = 120) {
    for (let i = 0; i < count; i++) {
      const state = await invoke(tool);
      const value = field.split('.').reduce((v, key) => v?.[key], state);
      if (predicate(value)) return state;
      await delay(200);
    }
    throw Error(`Timed out waiting for ${tool}.${field}`);
  }
  async function capture(label) {
    const result = await call('screenshot_window');
    const image = result.content.find(item => item.type === 'image');
    assert(image, 'MCP screenshot missing');
    fs.writeFileSync(path.join(output, label + '.png'), Buffer.from(image.data, 'base64'));
  }
  async function snapshot() {
    const uiState = await ensureWindow(await invoke('get_ui_state'));
    const view = await invoke('get_view_state');
    const debugState = await invoke('get_debug_info', { page_size: 200 });
    return { time: new Date().toISOString(), ui: uiState, view,
      debug: { ui: debugState.ui, view_revision: debugState.view_revision,
        latest_id: debugState.logs.latest_id, total: debugState.logs.total },
      records: debugState.logs.records };
  }
  function difference(a, b) {
    const records = b.records.filter(record => record.id > a.debug.latest_id);
    return { logs: b.debug.latest_id - a.debug.latest_id,
      a11y: records.filter(r => r.target === 'gpui::window' && r.message.includes('Sending a11y tree update:')).length,
      database: records.filter(r => r.category === 'database').length,
      errors: records.filter(r => r.level === 'ERROR' || r.level === 'WARN'),
      preview_renders: b.view.preview.image_load.render_count - a.view.preview.image_load.render_count,
      records };
  }
  async function check(label, action, options = {}) {
    console.log(JSON.stringify({ checking: label, completed: report.cases.length }));
    const entry = { label, started: new Date().toISOString(), expected_activity: !!options.active };
    try {
      if (action) await action();
      await delay(options.settle ?? 1400);
      entry.before = await snapshot();
      await delay(options.interval ?? 3000);
      entry.after = await snapshot();
      entry.delta = difference(entry.before, entry.after);
      // A final async completion can arrive during the first sample. Recheck
      // once; never hide an ongoing sequence of redraws behind a single count.
      if (entry.delta.logs && !options.active) {
        entry.retries = [];
        for (let attempt = 0; attempt < 3; attempt++) {
          await delay(4000);
          entry.retry_before = await snapshot();
          await delay(4000);
          entry.retry_after = await snapshot();
          entry.retry_delta = difference(entry.retry_before, entry.retry_after);
          entry.retries.push({ before:entry.retry_before, after:entry.retry_after, delta:entry.retry_delta });
          if (!entry.retry_delta.logs) break;
        }
      }
      const final = entry.retry_delta ?? entry.delta;
      entry.result = options.active ? 'active-observed' : final.logs === 0 && final.preview_renders === 0 ? 'settled' : 'sustained';
      if (options.verify) options.verify(entry.after);
      if (options.active) {
        assert(entry.before.view.playback.playing && entry.after.view.playback.playing,
          'active playback sample must actually stay playing');
        assert(entry.after.view.playback.position_seconds > entry.before.view.playback.position_seconds,
          'active playback sample did not advance');
        assert(final.database === 0, 'playback paints must not repeatedly query the database');
      }
      if (options.capture || entry.result === 'sustained') {
        await capture(label); entry.screenshot = label + '.png';
      }
    } catch (error) {
      entry.result = 'blocked'; entry.error = String(error);
      report.blockers.push({ label, error: entry.error });
    }
    report.cases.push(entry); save();
    console.log(JSON.stringify({ checked: label, result: entry.result,
      delta: entry.delta && { logs: entry.delta.logs, a11y: entry.delta.a11y, database: entry.delta.database },
      final_delta: entry.retry_delta && { logs: entry.retry_delta.logs, a11y: entry.retry_delta.a11y, database: entry.retry_delta.database } }));
  }

  try {
    await waitFor('get_ui_state', 'gallery.loaded_count', n => typeof n === 'number');
    await delay(600);
    report.accessibility = await invoke('control_ui', { action: 'accessibility_activate' });
    assert.equal(report.accessibility.provider_response, true);
    await debug('open');
    // Reset transient surfaces when reusing an isolated suite profile.
    await preview('close'); await invoke('control_playback', { action: 'close' });
    await ui('settings_close'); await ui('export_close'); await ui('gallery_dialogs_close');
    await navigation('close'); await gallery('popover', { value: 'close' });
    await gallery('clear_filters'); await navigation('all'); await ui('deselect');
    await check('library-idle', undefined, { capture: true });
    for (const page of ['folders', 'tags', 'extensions'])
      await check('navigation-' + page, () => ui('navigation_tab', page));
    await ui('settings_open');
    for (const tab of ['appearance', 'toolbar', 'shortcuts', 'tags', 'ai', 'updates'])
      await check('settings-' + tab, () => ui('settings_tab', tab), { capture: tab === 'ai' });
    await ui('settings_close');
    await check('statistics', () => ui('statistics'), { capture: true });
    await ui('gallery_dialogs_close');
    await check('bulk-empty', () => gallery('bulk_open'));
    await gallery('bulk_close');
    await check('export-idle', () => ui('export_open'));
    await ui('export_close');
    for (const value of ['type', 'tags', 'extensions', 'date', 'rating', 'sort', 'grouping']) {
      await check('filter-' + value, () => gallery('popover', { value }));
      await gallery('popover', { value: 'close' });
    }
    await check('tools-menu', () => gallery('tools_open'));
    await gallery('tools_close');

    const fixtures = path.join(profile, 'fixtures');
    fs.cpSync(path.join(root, 'test-data', 'photag-library'), fixtures, { recursive: true });
    // Optional generated 24MP image and test video prepared before launch.
    for (const [from, to] of [
      ['target/preview-latency-fixtures/cold-24mp.jpg', 'large-preview.jpg'],
      ['target/idle-suite-test.mp4', 'test-video.mp4'],
      ['target/idle-suite-test.wav', 'long-audio.wav'],
    ]) if (fs.existsSync(path.join(root, from))) fs.copyFileSync(path.join(root, from), path.join(fixtures, to));
    report.folder = await invoke('add_folder', { path: fixtures, confirm: true });
    await waitFor('get_ui_state', 'gallery.matched_count', n => n >= 12);
    await delay(2500);
    const media = await invoke('search_media', { limit: 100 });
    report.media = media.items;
    const image = media.items.find(m => m.filename === 'small.png');
    const broken = media.items.find(m => m.filename === 'broken-image.jpg');
    const animated = media.items.find(m => m.filename === 'medium.gif');
    const large = media.items.find(m => m.filename === 'large-preview.jpg');
    const audio = media.items.find(m => m.filename === 'long-audio.wav') ?? media.items.find(m => m.filename === 'sample.wav');
    const video = media.items.find(m => m.filename === 'test-video.mp4');
    assert(image && broken && audio, 'fixture scan did not expose required media');
    report.tag = await invoke('create_tag', { name: 'Idle audit tag' });
    await check('populated-library', undefined, { settle: 2500, capture: true });
    // Exercise cache freshness while Settings stays open, not just after a
    // reopen. These labels and counts are published by the rendered view.
    await ui('settings_open'); await ui('settings_tab', 'shortcuts');
    await waitFor('get_ui_state', 'settings_tag_choices.state', value => value === 'ready');
    report.liveTag = await invoke('create_tag', { name: 'Live choice refresh' });
    await check('settings-live-tag-created', undefined, { verify: state => {
      assert(state.ui.settings_tag_choices.choices.some(tag => tag.id === report.liveTag.id));
    }});
    await invoke('rename_tag', { tag_id: report.liveTag.id, name: 'Renamed choice refresh' });
    await check('settings-live-tag-renamed', undefined, { verify: state => {
      assert(state.ui.settings_tag_choices.choices.some(tag => tag.id === report.liveTag.id && tag.name === 'Renamed choice refresh'));
    }});
    await invoke('delete_tag', { tag_id: report.liveTag.id, confirm: true });
    await check('settings-live-tag-deleted', undefined, { verify: state => {
      assert(!state.ui.settings_tag_choices.choices.some(tag => tag.id === report.liveTag.id));
    }});
    await check('settings-ai-populated', () => ui('settings_tab', 'ai'));
    await ui('settings_close');
    await check('inspector-selected', () => ui('select_media', String(image.id)));
    for (const tab of ['information', 'history', 'extensions'])
      await check('inspector-' + tab, () => ui('inspector_tab', tab));
    await ui('inspector_tab', 'information');
    await check('multiple-selection', () => gallery('select_many', { media_ids: [image.id, broken.id, audio.id] }));
    await check('gallery-tag-picker', () => gallery('tag_picker', { enabled: true }));
    await gallery('tag_picker', { enabled: false });
    await check('media-actions', () => gallery('open_media_menu', { media_id: image.id }));
    await gallery('close_media_menu');
    await check('bulk-populated', () => gallery('bulk_open'));
    for (const value of ['copy', 'move', 'delete'])
      await check('bulk-' + value, () => gallery('bulk_configure', { value, bulk_source: 'selection' }));
    await gallery('bulk_configure', { value: 'copy', bulk_source: 'all' });
    await check('bulk-name-filter', () => gallery('bulk_field', { field_index: 4, value: 'small.png' }), { verify: state => {
      assert.equal(state.ui.gallery.dialog_reads.bulk.count, 1);
    }});
    await check('bulk-filter-error', () => gallery('bulk_field', { field_index: 2, value: 'invalid' }), { verify: state => {
      assert.equal(state.ui.gallery.dialog_reads.bulk.state, 'failed');
    }});
    await check('bulk-filter-error-recovered', () => gallery('bulk_field', { field_index: 2, value: '' }), { verify: state => {
      assert.equal(state.ui.gallery.dialog_reads.bulk.count, 1);
    }});
    await gallery('bulk_close');
    for (const value of ['none', 'month', 'file_type', 'modified', 'created', 'folder', 'subfolder'])
      await check('grouping-' + value, () => gallery('grouping', { value }));
    await check('gallery-list', () => gallery('layout', { value: 'list' }));
    await gallery('layout', { value: 'grid' });
    await check('empty-search', () => gallery('search', { value: 'no-result-idle-audit' }));
    await gallery('clear_filters'); await gallery('grouping', { value: 'none' });
    await check('folder-root', () => navigation('select_path', { value: fixtures }));
    await check('folder-nested', () => navigation('select_path', { value: path.join(fixtures, '03-nested', 'level-one') }));
    await navigation('all'); await gallery('grouping', { value: 'month' });
    for (const action of ['create', 'edit', 'delete', 'merge', 'move']) {
      await check('tag-dialog-' + action, () => navigation(action, { tag_id: report.tag.id }));
      await navigation('close');
    }
    await check('preview-image', () => gallery('preview', { media_id: image.id }), { settle: 4500, capture: true });
    await check('preview-info', () => preview('toggle_info'), { settle: 4000 });
    await check('preview-tags', () => preview('tags_open'), { settle: 4000 });
    await preview('tags_close');
    await check('preview-more', () => preview('more_open'), { settle: 4000 });
    await preview('more_close');
    await check('preview-zoom', () => preview('zoom', { zoom_percent: 150 }), { settle: 4500 });
    await check('preview-rotate', () => preview('rotate_clockwise'), { settle: 4500 });
    await check('preview-flip', () => preview('flip_horizontal'), { settle: 4500 });
    await check('preview-filmstrip', () => preview('filmstrip_show'), { settle: 4500 });
    await preview('close');
    if (large) {
      await check('preview-large', () => gallery('preview', { media_id: large.id }), { settle: 5000 });
      await preview('close');
    }
    await check('preview-broken', () => gallery('preview', { media_id: broken.id }), { settle: 4500, capture: true });
    await preview('close');
    if (animated) {
      await check('preview-gif', () => gallery('preview', { media_id: animated.id }), { settle: 4000 });
      await preview('close');
    }
    for (const item of [audio, video].filter(Boolean)) {
      const prefix = item === audio ? 'audio' : 'video';
      await invoke('open_media', { media_id: item.id });
      await delay(600);
      let state = await invoke('get_view_state');
      if (state.playback.playing) await invoke('control_playback', { action: 'play_pause' });
      await check(prefix + '-paused', undefined, { capture: true });
      for (const panel of ['tags', 'more']) {
        await check(prefix + '-' + panel, () => invoke('control_playback', { action: panel + '_open' }));
        await invoke('control_playback', { action: panel + '_close' });
      }
      await check(prefix + '-seek-paused', () => invoke('control_playback', { action: 'seek', seconds: 0.2 }));
      await check(prefix + '-playing', () => invoke('control_playback', { action: 'play_pause' }), { active: true, settle: 200 });
      await check(prefix + '-paused-after-playing', () => invoke('control_playback', { action: 'play_pause' }));
      await invoke('control_playback', { action: 'close' });
      await check(prefix + '-closed');
    }
    await check('export-success', () => invoke('export_metadata', { path: path.join(profile, 'export.json'), confirm: true }));
    await ui('export_close');
    await check('debug-paused', () => debug('pause'));
    await check('debug-follow', () => debug('follow'));
    await check('debug-category', () => debug('category', 'database'));
    await debug('category', '');
    await check('debug-page', () => debug('page', '1'));
    await debug('follow');
    await check('debug-closed', () => debug('close'));
    await debug('open');
    // Scale transitions expose selection outside the 200-row media window.
    await invoke('dev_test_inject_virtual', { count: 1000 });
    await waitFor('dev_test_status', 'running', value => value === false);
    await navigation('all'); await gallery('clear_filters');
    await check('scale-library', undefined, { settle: 2500 });
    const scaled = await invoke('search_media', { search: 'virtual-', limit: 100, sort: 'oldest' });
    if (scaled.items.length) {
      await ui('select_media', String(scaled.items[0].id));
      await check('scale-off-window-selection', () => gallery('scroll', { value: '900' }));
    }
    await ui('deselect');
    await check('scale-no-selection');
    await check('scale-statistics', () => ui('statistics'), { verify: state => {
      assert.equal(state.ui.gallery.dialog_reads.statistics.state, 'ready');
    }});
    await ui('gallery_dialogs_close');
    await check('scale-bulk', () => gallery('bulk_open'), { settle: 2500, verify: state => {
      assert(state.ui.gallery.dialog_reads.bulk.count >= 1000);
    }});
    await gallery('bulk_close');
    // Native picker uses the real Sidebar + through the existing GPUI queue.
    await ui('navigation_tab', 'folders');
    await ui('pointer_down', '228,72'); await delay(300);
    await ui('pointer_up', '228,72');
    await waitFor('get_ui_state', 'folder_picker.native_open', value => value === true, 40);
    await check('native-folder-picker-open');
    await navigation('folder_picker_cancel');
    await waitFor('get_ui_state', 'folder_picker.pending', value => value === false, 40);
    await check('native-folder-picker-closed');
    await ui('settings_open'); await ui('settings_tab', 'updates');
    await ui('settings_check_updates');
    await waitFor('get_ui_state', 'update_checking', value => value === false, 200);
    await check('manual-update-result', undefined, { capture: true });
    report.finished = new Date().toISOString();
    report.summary = report.cases.reduce((counts, entry) => {
      counts[entry.result] = (counts[entry.result] ?? 0) + 1; return counts;
    }, {});
    save(); console.log(JSON.stringify({ idle_suite_finished: report.summary, report: destination }));
    assert.equal(report.summary.sustained ?? 0, 0, 'idle suite found sustained work; inspect idle-report.json');
    assert.equal(report.summary.blocked ?? 0, 0, 'idle suite has unverified/blocked cases; inspect idle-report.json');
  } finally { save(); }
};
