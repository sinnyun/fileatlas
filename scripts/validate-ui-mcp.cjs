// Local, isolated UI validation through PhoTag's stdio MCP only.
// Run after `cargo build -p photag-shell --locked -j 2`.
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const root = path.resolve(__dirname, '..');
const profile = path.join(root, '.local', 'validation', 'profiles', 'mcp-ui-validation');
const output = path.join(root, '.local', 'validation', 'artifacts', 'ui-mcp-screenshots');
fs.mkdirSync(profile, { recursive: true });
fs.mkdirSync(output, { recursive: true });
// Only the isolated profile is configured; regular PhoTag data is untouched.
// `settings.rs:331-334` resolves through %LOCALAPPDATA%, which the child below
// remaps into this profile. show_filmstrip stays off by default in PhoTag
// (`settings.rs:232`), so the profile turns it on to make the band observable.
const localAppData = path.join(profile, 'local');
fs.mkdirSync(path.join(localAppData, 'PhoTag'), { recursive: true });
fs.writeFileSync(path.join(localAppData, 'PhoTag', 'settings.json'),
  JSON.stringify({ auto_check_updates: false, show_filmstrip: true }));
const child = spawn(path.join(root, 'target/debug/photag-shell.exe'), ['--mcp'], {
  cwd: root,
  env: { ...process.env, PHOTAG_DEV_PLUGINS: 'dev-test', PHOTAG_DEV_TEST_DIR: profile,
    LOCALAPPDATA: path.join(profile, 'local'), PHOTAG_MCP_ALLOW_MUTATIONS: '1' },
  stdio: ['pipe', 'pipe', 'pipe'],
});
const diagnostics = fs.createWriteStream(path.join(output, 'stderr.log'));
child.stderr.pipe(diagnostics);
let buffer = '', nextId = 1;
const pending = new Map();
child.stdout.setEncoding('utf8');
child.stdout.on('data', data => {
  buffer += data;
  while (buffer.includes('\n')) {
    const end = buffer.indexOf('\n');
    const line = buffer.slice(0, end); buffer = buffer.slice(end + 1);
    if (!line.trim()) continue;
    const message = JSON.parse(line);
    const waiter = pending.get(message.id);
    if (waiter) { pending.delete(message.id); waiter(message); }
  }
});
function request(method, params) {
  return new Promise((resolve, reject) => {
    const id = nextId++;
    const timer = setTimeout(() => { pending.delete(id); reject(Error(`MCP timeout: ${method}`)); }, 30000);
    pending.set(id, message => { clearTimeout(timer); message.error ? reject(Error(JSON.stringify(message.error))) : resolve(message.result); });
    child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
  });
}
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
async function call(name, args = {}) {
  const result = await request('tools/call', { name, arguments: args });
  assert(!result.isError, JSON.stringify(result.content?.filter(c => c.type === 'text')));
  return result;
}
function json(result) { return JSON.parse(result.content.find(c => c.type === 'text').text); }
async function ui(action, value) {
  const state = json(await call('control_ui', { action, value }));
  assert(!state.error, state.error);
  await delay(450); // Gallery and Settings consume requests on their own GPUI pollers.
  return state;
}
const report = { profile, screenshots: [], states: [] };
async function animation(action, value, id, field, target) {
  const start = Date.now(), samples = [];
  await call('control_ui', { action, value });
  for (let i = 0; i < 45; i++) {
    const state = json(await call('get_ui_state')).gallery_cards.find(card => card.media_id === id);
    if (state) samples.push({ elapsedMs: Date.now() - start, value: state[field] });
    if (samples.length > 1 && samples.at(-1).value === target) break;
    await delay(16);
  }
  const frames = json(await call('get_ui_state')).gallery_animation_frames.filter(frame => frame.media_id === id && frame.timestamp_ms >= start);
  assert(frames.some(frame => frame[field] > 0 && frame[field] < 1), `missing rendered intermediate ${field}: ${JSON.stringify(frames)}`);
  assert.equal(samples.at(-1).value, target);
  (report.animations ??= []).push({ action, id, field, target, samples, frames });
}
async function screenshot(label) {
  const shot = await call('screenshot_window');
  const image = shot.content.find(c => c.type === 'image');
  assert(image, 'missing MCP PNG');
  const filename = path.join(output, `${label}.png`);
  fs.writeFileSync(filename, Buffer.from(image.data, 'base64'));
  report.screenshots.push({ label, path: filename, metadata: shot.content.filter(c => c.type === 'text') });
}
(async () => {
  try {
    await request('initialize', { protocolVersion: '2024-11-05', capabilities: {}, clientInfo: { name: 'photag-ui-validation', version: '1' } });
    child.stdin.write(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n');
    report.tools = (await request('tools/list', {})).tools.map(tool => tool.name);
    report.initial = json(await call('get_ui_state'));
    await call('dev_test_scan_fixture');
    for (let i = 0; i < 100; i++) {
      if (!json(await call('dev_test_status')).running) break;
      await delay(200);
    }
    report.library = json(await call('list_library'));
    const media = json(await call('search_media', { limit: 100 }));
    assert.equal(media.total, 9);
    const sample = media.items.find(item => item.filename === 'small.png');
    assert(sample);
    for (const language of ['en', 'zh-CN']) {
      await ui('language', language);
      await ui('settings_close');
      await ui('deselect');
      await call('control_preview', { action: 'close' });
      await delay(150);
      await screenshot(`${language}-gallery`);
      await animation('hover_media', String(sample.id), sample.id, 'hover', 1);
      await screenshot(`${language}-gallery-hover`);
      await animation('hover_clear', undefined, sample.id, 'hover', 0);
      await ui('settings_open');
      for (const tab of ['appearance', 'toolbar', 'shortcuts', 'tags', 'ai', 'updates']) {
        await ui('settings_tab', tab);
        await screenshot(`${language}-settings-${tab}`);
      }
      await ui('settings_close');
      await ui('statistics'); await screenshot(`${language}-statistics`);
      await ui('gallery_dialogs_close');
      await ui('bulk_actions'); await screenshot(`${language}-bulk`);
      await ui('gallery_dialogs_close');
      await animation('select_media', String(sample.id), sample.id, 'selection', 1);
      await screenshot(`${language}-selected`);
      await call('open_media', { media_id: sample.id });
      await delay(200);
      const opened = json(await call('get_view_state')).preview;
      assert.equal(opened.media_id, sample.id, 'preview opened without its media id');
      assert.equal(opened.filmstrip.length, media.total, 'filmstrip not seeded from the library page');
      await screenshot(`${language}-preview-bands`); // toolbar, filmstrip, corner chrome
      if (!opened.show_info) await call('control_preview', { action: 'toggle_info' });
      await delay(200);
      await screenshot(`${language}-preview-info`);
      const transformed = json(await call('control_preview', { action: 'zoom', zoom_percent: 200 }));
      await call('control_preview', { action: 'rotate_clockwise' });
      await call('control_preview', { action: 'flip_horizontal' });
      assert.equal(transformed.preview.zoom_percent, 200);
      assert.equal(json(await call('get_view_state')).preview.quarter_turns, 1);
      assert.equal(json(await call('get_view_state')).preview.flip_horizontal, true);
      await screenshot(`${language}-preview-transformed`);
      await call('control_preview', { action: 'fit' });
      await screenshot(`${language}-preview-filmstrip-click`);
      report.states.push(json(await call('get_view_state')));
      await call('control_preview', { action: 'close' });
    }
    // Observe the real service's fade curve without synthetic pointer input.
    await call('open_media', { media_id: sample.id });
    const start = Date.now();
    report.fade = [];
    for (const target of [0, 2050, 2350, 2900]) {
      await delay(Math.max(0, start + target - Date.now()));
      report.fade.push({ elapsedMs: Date.now() - start, ...json(await call('get_ui_state')) });
    }
    assert(report.fade[0].preview_chrome_alpha > 0.9);
    assert.equal(report.fade.at(-1).preview_chrome_alpha, 0);
    await screenshot('preview-chrome-hidden');
    await call('control_preview', { action: 'close' });
    await ui('language', 'en'); // User requested English as the default.
    if (process.argv.includes('--scan-status')) {
      // Hard links to a synthetic TIFF make decoding last long enough to observe
      // the production progress band; they never modify the source fixture.
      const corpus = path.join(profile, `scan-corpus-${Date.now()}`);
      fs.mkdirSync(corpus);
      const source = path.join(root, 'test-data/photag-library/02-mixed-formats/large.tiff');
      for (let i = 0; i < 120; i++) fs.linkSync(source, path.join(corpus, `scan-${i}.tiff`));
      const added = json(await call('add_folder', { path: corpus, confirm: true }));
      assert(added.ok, JSON.stringify(added));
      report.scan = { folder: added.folder, samples: [] };
      for (let i = 0; i < 300; i++) {
        const state = json(await call('get_ui_state'));
        report.scan.samples.push(state.scan_progress);
        if (state.scan_progress && !report.scan.captured) {
          await delay(250);
          await screenshot('scan-progress-with-selection');
          report.scan.captured = true;
        }
        if (!state.scan_progress && report.scan.captured) break;
        await delay(100);
      }
      assert(report.scan.captured, 'production scan progress was not observed');
      assert.equal(json(await call('get_ui_state')).scan_progress, null);
      await screenshot('scan-complete-selection-restored');
      report.scan.cleanup = json(await call('remove_folder', { folder_id: added.folder.id, confirm: true }));
      assert(report.scan.cleanup.ok, JSON.stringify(report.scan.cleanup));
    }
    report.final = json(await call('get_ui_state'));
    fs.writeFileSync(path.join(output, 'report.json'), JSON.stringify(report, null, 2));
    console.log(JSON.stringify({ screenshots: report.screenshots.length, media: media.total, animations: report.animations.length,
      fade: report.fade.map(state => ({ elapsedMs: state.elapsedMs, alpha: state.preview_chrome_alpha })), report: path.join(output, 'report.json') }));
  } catch (error) {
    console.error(error); process.exitCode = 1;
  } finally { child.kill(); }
})();
