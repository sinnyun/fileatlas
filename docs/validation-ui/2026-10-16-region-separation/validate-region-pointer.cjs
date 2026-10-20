// Real pointer evidence for the Settings region-separation control (same-process GPUI events).
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, request, profile, output }) => {
  const report = { profile, clicks: { button: process.env.FILEATLAS_POINTER_CLICK, item: process.env.FILEATLAS_POINTER_ITEM } };
  const save = () => fs.writeFileSync(path.join(profile, 'region-pointer-report.json'), JSON.stringify(report, null, 2));
  async function invoke(tool, args = {}) {
    const value = payload(await call(tool, args));
    assert(!value?.error, `${tool} failed: ${JSON.stringify(value)}`);
    return value;
  }
  const state = () => invoke('get_ui_state');
  async function capture(name) {
    const result = await call('screenshot_window');
    for (const block of result.content ?? []) if (block.type === 'image') fs.writeFileSync(path.join(output, name + '.png'), Buffer.from(block.data, 'base64'));
  }
  async function send(action, value) {
    for (let attempt = 0; attempt < 40; attempt++) {
      const result = await request('tools/call', { name: 'control_ui', arguments: { action, value } });
      const body = payload(result);
      const error = typeof body?.error === 'string' ? body.error : JSON.stringify(body ?? result);
      if (error.includes('still pending')) { await delay(250); continue; }
      assert(!body?.error, `${action} failed: ${error}`);
      return body;
    }
    throw Error(`pointer queue never drained for ${action}`);
  }
  async function tap(coords) {
    await send('pointer_move', coords);
    await delay(250);
    await send('pointer_down', coords);
    await delay(250);
    await send('pointer_up', coords);
    await delay(400);
  }

  await delay(1500);
  const start = await state();
  report.startup_separation = start.region_separation;
  report.geometry = start.window_status?.geometry;
  report.bounds = start.window_status?.bounds;
  save();
  await invoke('resize_window', { width: 1942, height: 1256 });
  await delay(400);
  await invoke('control_ui', { action: 'settings_open' });
  await invoke('control_ui', { action: 'settings_tab', value: 'appearance' });
  await delay(500);
  await capture('pointer-settings-open');
  if (process.env.FILEATLAS_POINTER_TAB) {
    await tap(process.env.FILEATLAS_POINTER_TAB);
    report.tab_probe = { clicked: process.env.FILEATLAS_POINTER_TAB, settings_tab: (await state()).settings_tab };
    await invoke('control_ui', { action: 'settings_tab', value: 'appearance' });
    save();
  }
  if (!process.env.FILEATLAS_POINTER_CLICK) { report.cases = ['probe run: screenshot + geometry captured']; save(); return; }

  await tap(process.env.FILEATLAS_POINTER_CLICK);
  await capture('pointer-dropdown');
  if (process.env.FILEATLAS_POINTER_ITEM) {
    const before = (await state()).region_separation;
    await tap(process.env.FILEATLAS_POINTER_ITEM);
    for (let i = 0; i < 60; i++) { if ((await state()).region_separation !== before) break; await delay(100); }
    const after = (await state()).region_separation;
    report.change = { before, after, disk: JSON.parse(fs.readFileSync(path.join(profile, 'settings.json'), 'utf8')).region_separation };
    await invoke('control_ui', { action: 'settings_close' });
    await delay(500);
    await capture('pointer-main-after');
    assert.notEqual(after, before, 'a real pointer click must change the applied separation');
    report.cases = ['pointer click on the Settings dropdown changed the live region separation'];
  }
  save();
};
