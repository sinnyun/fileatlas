// Same-process MCP check of the persistent subfolder toggle background.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, profile, output }) => {
  const invoke = async (name, args = {}) => payload(await call(name, args));
  const state = () => invoke('get_ui_state');
  await delay(3500);
  const initial = await state();
  assert.equal(initial.window_status.bounds.width, 3286);
  assert.equal(initial.browser.show_descendants, false);
  const geometry = initial.window_status.geometry.screenshot_to_pointer;
  const position = (x, y) => `${Math.round((x - geometry.offset_x) / geometry.divide_by)},${Math.round((y - geometry.offset_y) / geometry.divide_by)}`;
  async function pointer(action, value) {
    for (let i = 0; i < 20; i++) {
      try { return await invoke('control_ui', { action, value }); }
      catch (error) {
        if (!String(error).includes('pointer request is still pending')) throw error;
        await delay(150);
      }
    }
    throw Error('MCP pointer queue did not drain');
  }
  async function toggle(expected) {
    const value = position(2335, 83);
    await pointer('pointer_down', value);
    await delay(150);
    await pointer('pointer_up', value);
    await delay(500);
    assert.equal((await state()).browser.show_descendants, expected);
  }
  async function capture(name, hover = false) {
    await pointer('pointer_move', hover ? position(2335, 83) : position(2050, 500));
    await delay(500);
    const shot = await call('screenshot_window');
    fs.writeFileSync(path.join(output, `${name}.png`), Buffer.from(shot.content.find(b => b.type === 'image').data, 'base64'));
  }
  await capture('subfolders-off');
  await toggle(true);
  await capture('subfolders-on');
  await capture('subfolders-on-hover', true);
  await pointer('pointer_move', position(2050, 500));
  await delay(500);
  const enabled = await state();
  await delay(1500);
  const quiet = await state();
  assert.equal(quiet.browser.show_descendants, true);
  assert.equal(quiet.gallery.snapshot_revision, enabled.gallery.snapshot_revision);
  await toggle(false);
  await capture('subfolders-off-restored');
  fs.writeFileSync(path.join(profile, 'subfolder-fill-report.json'), JSON.stringify({ initial, enabled, quiet, restored: await state() }, null, 2));
};
