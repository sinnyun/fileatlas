// Read actual user images via same-process MCP; only isolated profile data is changed.
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const assert = require('node:assert/strict');
module.exports = async ({ call, payload, delay, profile, output, exe }) => {
  const folder = process.env.FILEATLAS_REAL_FOLDER;
  assert(folder && fs.statSync(folder).isDirectory());
  const report = { started: new Date().toISOString(), folder, profile, exe,
    exe_sha256: crypto.createHash('sha256').update(fs.readFileSync(exe)).digest('hex'),
    scope: 'Empty application cache; OS file cache is not flushed', cases: [], calls: [], progress: [] };
  const save = () => fs.writeFileSync(path.join(profile, 'real-folder-report.json'), JSON.stringify(report, null, 2));
  async function invoke(tool, args = {}) {
    const started = Date.now();
    try { const result = payload(await call(tool, args)); report.calls.push({ tool, args, elapsed_ms: Date.now() - started }); return result; }
    catch (error) { report.calls.push({ tool, args, error: String(error) }); save(); throw error; }
  }
  function cacheFiles(directory) {
    if (!fs.existsSync(directory)) return [];
    return fs.readdirSync(directory, { withFileTypes: true }).flatMap(e =>
      e.isDirectory() ? cacheFiles(path.join(directory, e.name)) : [path.join(directory, e.name)]);
  }
  async function fingerprint() {
    const list = [];
    for (const entry of fs.readdirSync(folder, { withFileTypes: true })) {
      if (!entry.isFile()) continue;
      const file = path.join(folder, entry.name), before = fs.statSync(file), hash = crypto.createHash('sha256');
      for await (const bytes of fs.createReadStream(file)) hash.update(bytes);
      list.push({ file, bytes: before.size, mtime_ms: before.mtimeMs, sha256: hash.digest('hex') });
    }
    return list;
  }
  const names = ['2026-08-10 222-26-13.png', '01 (10).jpg', '01 (3).png', '01 (4).png', '01 (5).png', '01 (6).jpg', '01 (7).png'];
  const logs = () => invoke('get_debug_info', { category: 'preview', page_size: 200 });
  function parseJob(record) {
    const parsed = { id: record.id, message: record.message, level: record.level };
    for (const key of ['job_id', 'max_dim', 'file_bytes', 'queue_ms', 'cache_check_ms', 'read_ms', 'header_ms', 'decode_ms', 'orientation_ms', 'resize_ms', 'write_ms', 'duration_ms', 'total_ms']) {
      const match = record.message.match(new RegExp('(?:^| )' + key + '=([-+0-9.eE]+)(?: |$)'));
      if (match) parsed[key] = Number(match[1]);
    }
    parsed.cached = record.message.includes('cached=true');
    parsed.reused = record.message.includes('reused=true');
    parsed.status = record.message.match(/(?:^| )status="([^"]+)"/)?.[1];
    return parsed;
  }
  async function ensureWindow() {
    const state = await invoke('get_ui_state');
    assert(state.window_status?.valid && state.window_status?.owned);
    if (state.window_status.minimized) {
      (report.window_interruptions ??= []).push(state.window_status);
      await invoke('control_ui', { action: 'window_restore' }); await delay(3000);
    }
    return state;
  }
  async function test(name, pass) {
    const entry = { name, pass, started: new Date().toISOString() };
    console.log(JSON.stringify({ opening: name, pass }));
    try {
      await ensureWindow();
      const media = report.media.items.find(m => m.filename === name); assert(media);
      entry.media = media; entry.sequence_before = (await logs()).logs.latest_id;
      const started = Date.now();
      await invoke('control_file_manager', { action: 'preview', media_id: media.id });
      let view;
      for (let i = 0; i < 240; i++) {
        view = await invoke('get_view_state');
        const load = view.preview.image_load;
        if (load.source?.toLowerCase() === media.full_path.toLowerCase() && (load.full_frame_ms != null || load.error)) break;
        await delay(250);
      }
      entry.paint = view.preview.image_load; entry.wall_to_observation_ms = Date.now() - started;
      assert.equal(entry.paint.source.toLowerCase(), media.full_path.toLowerCase());
      entry.result = entry.paint.full_frame_ms != null ? 'displayed' : entry.paint.error ? 'image-error' : 'paint-timeout';
      // Let real neighboring prefetch finish before measuring the next image.
      let last = -1, quiet = 0, latest;
      for (let i = 0; i < 120; i++) {
        latest = await logs(); quiet = latest.logs.total === last ? quiet + 1 : 0; last = latest.logs.total;
        const records = latest.logs.records.filter(r => r.id > entry.sequence_before);
        if (quiet >= 3 && records.filter(r => r.message.includes('preview decode started')).length === records.filter(r => r.message.includes('preview decode finished')).length) break;
        await delay(500);
      }
      entry.logs = latest;
      entry.jobs = latest.logs.records.filter(r => r.id > entry.sequence_before && r.message.includes('preview decode finished')).map(parseJob);
      entry.target_jobs = entry.jobs.filter(j => j.message.includes('source=' + media.full_path + ' '));
      if (entry.paint.cache_policy) {
        for (const job of entry.jobs.filter(j => j.max_dim === 96)) {
          assert(job.reused, 'indexed filmstrip must reuse FileManager thumbnail');
          assert.equal(job.decode_ms, 0); assert.equal(job.read_ms, 0); assert.equal(job.write_ms, 0);
        }
        assert(entry.target_jobs.filter(j => j.max_dim === 1600).length <= 1);
        if (name.endsWith('.png')) assert(!entry.target_jobs.some(j => j.max_dim === 1600));
      }
      entry.current_ui = await ensureWindow();
      if (name === names[0]) {
        const shot = await call('screenshot_window');
        fs.writeFileSync(path.join(output, pass + '-target.png'), Buffer.from(shot.content.find(c => c.type === 'image').data, 'base64'));
      }
      await invoke('control_preview', { action: 'close' }); await delay(300);
    } catch (error) { entry.result = 'blocked'; entry.error = String(error); }
    report.cases.push(entry); save();
    console.log(JSON.stringify({ tested: name, pass, result: entry.result, first_ms: entry.paint?.first_frame_ms,
      full_ms: entry.paint?.full_frame_ms, jobs: entry.target_jobs?.map(j => ({ max_dim: j.max_dim, cached: j.cached,
        queue: j.queue_ms, read: j.read_ms, decode: j.decode_ms, resize: j.resize_ms, write: j.write_ms, total: j.total_ms })), error: entry.error }));
  }
  try {
    report.sources_before = await fingerprint();
    report.cache_before = { thumbs: cacheFiles(path.join(profile, 'thumbs')), local: cacheFiles(path.join(profile, 'local', 'FileAtlas')) };
    assert.equal(report.cache_before.thumbs.length, 0); assert.equal(report.cache_before.local.length, 0);
    await delay(1500); await ensureWindow();
    report.accessibility = await invoke('control_ui', { action: 'accessibility_activate' });
    assert.equal(report.accessibility.provider_response, true);
    await invoke('control_debug', { action: 'open' });
    const started = Date.now();
    report.folder_result = await invoke('add_folder', { path: folder, confirm: true });
    for (let i = 0; i < 240; i++) {
      const state = await invoke('get_ui_state');
      report.progress.push({ elapsed_ms: Date.now() - started, scan_progress: state.scan_progress, file_manager: state.file_manager?.matched_count });
      if (state.scan_progress?.error) throw Error(state.scan_progress.error);
      if (!state.scan_progress && state.file_manager?.matched_count >= report.sources_before.length) break;
      await delay(500);
    }
    report.scan_ms = Date.now() - started; report.scan_final = await invoke('get_ui_state');
    assert.equal(report.scan_final.scan_progress, null);
    report.media = await invoke('search_media', { limit: 100 });
    assert.equal(report.media.total, report.sources_before.length);
    assert(names.every(name => report.media.items.some(m => m.filename === name)));
    report.pre_preview_cache = { preview: cacheFiles(path.join(profile, 'local', 'FileAtlas', 'preview')), thumbs: cacheFiles(path.join(profile, 'thumbs')).length };
    assert.equal(report.pre_preview_cache.preview.length, 0);
    for (const name of names) await test(name, 'first');
    for (const name of names) await test(name, 'reopen');
    const before = await logs(); await delay(5000); const after = await logs();
    report.idle = { before: before.logs.total, after: after.logs.total }; assert.equal(before.logs.total, after.logs.total);
    report.sources_after = await fingerprint(); assert.deepEqual(report.sources_after, report.sources_before);
    report.summary = report.cases.reduce((counts, entry) => (counts[entry.result] = (counts[entry.result] ?? 0) + 1, counts), {});
    report.finished = new Date().toISOString(); save();
    assert.equal(report.summary.blocked ?? 0, 0); assert.equal(report.summary['paint-timeout'] ?? 0, 0);
    console.log(JSON.stringify({ finished: report.summary, scan_ms: report.scan_ms, sources_unchanged: true }));
  } finally { save(); }
};
