// Additional stability/freshness checks using the same project's MCP child.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, profile, output, exe }) => {
  const report = { started: new Date().toISOString(), executable:exe,
    executable_sha256: require('node:crypto').createHash('sha256').update(fs.readFileSync(exe)).digest('hex'),
    calls: [], cases: [] };
  const save = () => fs.writeFileSync(path.join(profile, 'idle-followup-report.json'), JSON.stringify(report, null, 2));
  async function ensureWindow(state) {
    const status = state.window_status;
    if (!status) return state;
    assert(status.valid && status.owned, 'the tested native window is unavailable');
    if (status.minimized) {
      (report.window_interruptions ??= []).push({ time: new Date().toISOString(), status });
      await call('control_ui', { action: 'window_restore' }); await delay(4000);
      state = payload(await call('get_ui_state'));
      assert(!state.window_status.minimized, 'native test window remained minimized');
    }
    return state;
  }
  async function invoke(tool, args = {}) {
    report.calls.push({ time: new Date().toISOString(), tool, args });
    if (tool.startsWith('control_')) await ensureWindow(payload(await call('get_ui_state')));
    const result = payload(await call(tool, args));
    if (tool.startsWith('control_')) await delay(300);
    return result;
  }
  const ui = (action, value) => invoke('control_ui', { action, value });
  const gallery = (action, extra = {}) => invoke('control_gallery', { action, ...extra });
  const preview = action => invoke('control_preview', { action });
  async function sample() {
    const view = await invoke('get_view_state');
    const ui = await ensureWindow(await invoke('get_ui_state'));
    const debug = await invoke('get_debug_info', { page_size: 200 });
    return { time: new Date().toISOString(), view, ui, debug };
  }
  async function check(label, action, options = {}) {
    console.log(JSON.stringify({ checking: label }));
    const entry = { label };
    try {
      if (action) await action();
      await delay(options.settle ?? 1500);
      entry.before = await sample(); await delay(options.interval ?? 4000);
      entry.after = await sample();
      entry.new_records = entry.after.debug.logs.records.filter(r => r.id > entry.before.debug.logs.latest_id);
      entry.delta = entry.after.debug.logs.latest_id - entry.before.debug.logs.latest_id;
      entry.render_delta = entry.after.view.preview.image_load.render_count - entry.before.view.preview.image_load.render_count;
      entry.activity_before = entry.before.view.preview.image_load.activity;
      entry.activity_after = entry.after.view.preview.image_load.activity;
      if (entry.delta !== 0 || entry.render_delta !== 0) {
        // Delayed hover tooltips/native transitions can paint a finite burst.
        // Keep that evidence, then require a second quiet observation window.
        entry.retries = [];
        for (let attempt = 0; attempt < 3; attempt++) {
          // Chrome fades for up to 2.8 s after genuine input. A new mouse move
          // during observation invalidates that idle sample, not the product.
          await delay(4000); entry.retry_before = await sample();
          await delay(5000); entry.retry_after = await sample();
          entry.retry_delta = entry.retry_after.debug.logs.latest_id - entry.retry_before.debug.logs.latest_id;
          entry.retry_render_delta = entry.retry_after.view.preview.image_load.render_count - entry.retry_before.view.preview.image_load.render_count;
          entry.retries.push({ before: entry.retry_before, after: entry.retry_after, delta: entry.retry_delta, render_delta: entry.retry_render_delta });
          if (entry.retry_delta === 0 && entry.retry_render_delta === 0) break;
        }
      }
      entry.result = (entry.retry_delta ?? entry.delta) === 0 && (entry.retry_render_delta ?? entry.render_delta) === 0 ? 'settled' : 'sustained';
      if (options.verify) options.verify(entry.after);
      if (options.capture || entry.result === 'sustained') {
        const shot = await call('screenshot_window');
        const image = shot.content.find(item => item.type === 'image');
        assert(image); fs.writeFileSync(path.join(output, label + '.png'), Buffer.from(image.data, 'base64'));
      }
    } catch (error) { entry.result = 'blocked'; entry.error = String(error); }
    report.cases.push(entry); save();
    console.log(JSON.stringify({ checked: label, result: entry.result, delta: entry.delta, retry_delta: entry.retry_delta,
      activity_before: entry.activity_before, activity_after: entry.activity_after, error: entry.error }));
  }
  try {
    await delay(1500);
    report.accessibility = await invoke('control_ui', { action: 'accessibility_activate' });
    assert.equal(report.accessibility.provider_response, true);
    await invoke('control_debug', { action: 'open' });
    await ui('settings_close'); await ui('export_close'); await ui('gallery_dialogs_close');
    await preview('close'); await invoke('control_playback', { action: 'close' });
    await invoke('control_navigation', { action: 'close' });
    await gallery('clear_filters'); await invoke('control_navigation', { action: 'all' });
    await invoke('dev_test_clear_virtual'); await delay(2000);
    const media = await invoke('search_media', { limit: 100 });
    const image = media.items.find(m => m.filename === 'small.png');
    assert(image); report.media_id = image.id;
    await ui('pointer_move', '1,1');
    for (const label of ['preview-still-first', 'preview-still-reopen', 'preview-still-third']) {
      await check(label, () => gallery('preview', { media_id: image.id }), { settle: 5000, interval: 5000, capture: label === 'preview-still-first' });
      await check(label + '-extended', undefined, { settle: 1000, interval: 5000 });
      await preview('close');
    }
    await gallery('preview', { media_id: image.id });
    await check('preview-pointer-inside', () => ui('pointer_move', '650,260'), { settle: 4500, interval: 5000 });
    await check('preview-pointer-unchanged', () => ui('pointer_move', '650,260'), { settle: 4500, interval: 5000 });
    await preview('close'); await ui('pointer_move', '1,1');
    await ui('statistics');
    await check('statistics-live-rating-five', () => invoke('set_rating', { media_ids: [image.id], rating: 5 }), {
      verify: s => assert(s.ui.gallery.dialog_reads.statistics.rating_counts[5] >= 1), capture: true,
    });
    const rated = await invoke('get_ui_state');
    const countFive = rated.gallery.dialog_reads.statistics.rating_counts[5];
    await check('statistics-live-rating-clear', () => invoke('set_rating', { media_ids: [image.id], rating: 0 }), {
      verify: s => assert.equal(s.ui.gallery.dialog_reads.statistics.rating_counts[5], countFive - 1),
    });
    await ui('gallery_dialogs_close');
    await gallery('bulk_configure', { value: 'copy', bulk_source: 'all' });
    await gallery('bulk_field', { field_index: 2, value: '5' });
    await check('bulk-live-rating-added', () => invoke('set_rating', { media_ids: [image.id], rating: 5 }), {
      verify: s => assert(s.ui.gallery.dialog_reads.bulk.count >= 1),
    });
    const bulkBefore = await invoke('get_ui_state');
    await check('bulk-live-rating-removed', () => invoke('set_rating', { media_ids: [image.id], rating: 0 }), {
      verify: s => assert.equal(s.ui.gallery.dialog_reads.bulk.count, bulkBefore.gallery.dialog_reads.bulk.count - 1),
    });
    await gallery('bulk_close');
    await check('hover-animation-settled', () => ui('hover_media', String(image.id)));
    await check('hover-animation-cleared', () => ui('hover_clear'));
    for (const value of ['type', 'tags', 'extensions', 'date', 'rating', 'sort', 'grouping']) {
      await check('populated-filter-' + value, () => gallery('popover', { value }));
      await gallery('popover', { value: 'close' });
    }
    let single, multi1, multi2;
    await check('plugin-single-panel', async () => {
      single = (await ui('panel_open', 'dev-test/status')).instance_id;
      const again = (await ui('panel_open', 'dev-test/status')).instance_id;
      assert.equal(single, again);
    });
    await ui('panel_close', single);
    await check('plugin-multiple-panels', async () => {
      multi1 = (await ui('panel_open', 'dev-test/status-multi')).instance_id;
      multi2 = (await ui('panel_open', 'dev-test/status-multi')).instance_id;
      assert.notEqual(multi1, multi2);
    });
    await ui('panel_close', multi1); await ui('panel_close', multi2);
    await check('plugin-panels-disposed');
    await ui('select_media', String(image.id)); await ui('inspector_tab', 'information');
    for (const value of ['tags', 'exif', 'path'])
      await check('inspector-section-' + value, () => gallery('inspector_section', { value }));
    const another = media.items.find(m => m.media_type === 'image' && m.id !== image.id);
    await check('exif-switch-selection', () => ui('select_media', String(another.id)));
    await check('exif-clear-selection', () => ui('deselect'));
    await ui('select_media', String(image.id));
    await invoke('dev_test_inject_virtual', { count: 1000 });
    for (let i = 0; i < 100; i++) {
      if (!(await invoke('dev_test_status')).running) break;
      await delay(200);
    }
    await check('exif-scroll-selection', () => gallery('scroll', { value: '900' }), { settle: 3000 });
    const virtual = await invoke('search_media', { search: 'virtual-', limit: 10 });
    await check('exif-missing-source-cached', () => ui('select_media', String(virtual.items[0].id)));
    await invoke('dev_test_clear_virtual'); await delay(1500);
    await check('exif-reselect-real', () => ui('select_media', String(image.id)));
    const watcherProbe = path.join(profile, 'fixtures', 'watcher-idle-probe.txt');
    async function waitProbe(count) {
      for (let attempt = 0; attempt < 60; attempt++) {
        const found = await invoke('search_media', { search: 'watcher-idle-probe.txt', limit: 10 });
        if (found.total === count) return;
        await delay(200);
      }
      throw Error('native Watcher did not reconcile the fixture probe');
    }
    await check('watcher-create-settled', async () => {
      fs.writeFileSync(watcherProbe, 'Controlled external fixture change.'); await waitProbe(1);
    });
    await check('watcher-remove-settled', async () => {
      fs.unlinkSync(watcherProbe); await waitProbe(0);
    });
    const invalidDestination = path.join(profile, 'export-directory.json');
    fs.mkdirSync(invalidDestination, { recursive: true });
    await check('export-error-settled', async () => {
      let rejected = false;
      try { await invoke('export_metadata', { path: invalidDestination, confirm: true }); }
      catch { rejected = true; }
      assert(rejected, 'exporting over a directory should report the real filesystem error');
    }, { verify: state => assert.equal(state.ui.metadata_export.state, 'failed') });
    await check('export-error-recovered', () => invoke('export_metadata', {
      path: path.join(profile, 'export-recovered.json'), confirm: true,
    }), { verify: state => assert.equal(state.ui.metadata_export.state, 'succeeded') });
    await ui('export_close');
    for (const [action, value] of [['level', 'ERROR'], ['search', 'with_reader'], ['page_size', '25']]) {
      await check('debug-' + action, () => invoke('control_debug', { action, value }));
      if (action !== 'page_size') await invoke('control_debug', { action, value: '' });
    }
    const records = await invoke('get_debug_info', { page_size: 25 });
    await check('debug-record-details', () => invoke('control_debug', { action: 'select', value: String(records.logs.records[0].id) }));
    await check('debug-clear', () => invoke('control_debug', { action: 'clear' }));
    report.finished = new Date().toISOString();
    report.summary = report.cases.reduce((counts, c) => { counts[c.result] = (counts[c.result] ?? 0) + 1; return counts; }, {});
    save(); console.log(JSON.stringify({ followup_finished: report.summary }));
    assert.equal(report.summary.sustained ?? 0, 0);
    assert.equal(report.summary.blocked ?? 0, 0);
  } finally { save(); }
};
