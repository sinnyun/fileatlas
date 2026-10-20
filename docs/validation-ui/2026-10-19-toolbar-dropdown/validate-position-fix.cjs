// Run through scripts/fileatlas-mcp-debug.cjs with an isolated fresh profile.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, profile, output }) => {
  const invoke = async (tool, args = {}) => payload(await call(tool, args));
  const state = () => invoke('get_ui_state');
  const pane = (s, index) => s.program_workspace.current_snapshot?.columns[index];
  async function until(predicate) {
    for (let attempt = 0; attempt < 40; attempt++) {
      const value = await state();
      if (predicate(value)) return value;
      await delay(150);
    }
    throw Error('Toolbar state did not settle');
  }
  await delay(3500);
  const initial = await state();
  const geometry = initial.window_status.geometry.screenshot_to_pointer;
  async function pointer(action, value) {
    for (let attempt = 0; attempt < 20; attempt++) {
      try {
        return await invoke('control_ui', { action, value });
      } catch (error) {
        if (!String(error).includes('pointer request is still pending')) throw error;
        await delay(150);
      }
    }
    throw Error('MCP pointer queue did not drain');
  }
  // These coordinates refer to the inspected 3286×1831 screenshot at 150% DPI.
  assert.equal(initial.window_status.bounds.width, 3286);
  const point = (x, y) => `${Math.round((x - geometry.offset_x) / geometry.divide_by)},${Math.round((y - geometry.offset_y) / geometry.divide_by)}`;
  async function click(x, y) {
    const value = point(x, y);
    await pointer('pointer_down', value);
    await delay(100);
    await pointer('pointer_up', value);
    await delay(350);
  }
  async function capture(name) {
    await pointer('pointer_move', point(2050, 500));
    await delay(350);
    const shot = await call('screenshot_window');
    fs.writeFileSync(path.join(output, `${name}.png`), Buffer.from(shot.content.find(b => b.type === 'image').data, 'base64'));
  }
  const openMenu = async () => { await click(2270, 83); await delay(250); };
  assert.equal(initial.program_workspace.layout, 'single');
  await capture('corrected-single');
  await openMenu();
  await capture('corrected-single-menu');
  await click(2325, 179);
  await until(s => s.program_workspace.layout === 'dual');
  await click(2209, 83);
  await until(s => pane(s, 0)?.list_layout === true);
  await click(2335, 83);
  await until(s => pane(s, 0)?.show_subfolders === true);
  await capture('corrected-dual-toggle-on');
  await openMenu();
  await capture('corrected-dual-menu');
  await click(2325, 308);
  const second = await until(s => s.program_workspace.active_column === 1 && pane(s, 1));
  assert.equal(pane(second, 1).list_layout, false);
  assert.equal(pane(second, 1).show_subfolders, false);
  await click(2335, 83);
  await until(s => pane(s, 1)?.show_subfolders === true);
  await click(2335, 83);
  await until(s => pane(s, 1)?.show_subfolders === false);
  await openMenu();
  await click(2325, 221);
  await until(s => s.program_workspace.layout === 'quad');
  await openMenu();
  await capture('corrected-quad-menu');
  await click(2325, 392);
  await until(s => s.program_workspace.active_column === 3);
  await openMenu();
  await click(2325, 137);
  const restored = await until(s => s.program_workspace.layout === 'single' && s.program_workspace.active_column === 0);
  assert.equal(pane(restored, 0).list_layout, true);
  assert.equal(pane(restored, 0).show_subfolders, true);
  await click(2158, 83);
  await until(s => s.file_manager.layout === 'grid');
  await click(2335, 83);
  await until(s => s.browser.show_descendants === false);
  await capture('corrected-final');
  const settled = await state();
  await delay(1500);
  const quiet = await state();
  assert.equal(quiet.program_workspace.revision, settled.program_workspace.revision);
  assert.equal(quiet.file_manager.snapshot_revision, settled.file_manager.snapshot_revision);
  fs.writeFileSync(path.join(profile, 'position-fix-report.json'), JSON.stringify({
    geometry, cases: ['single/dual/quad selected through actual dropdown', 'active columns 2 and 4 selected through dropdown', 'grid/list and subfolder controls follow the active pane', 'first pane state restored after returning to single', 'subfolder toggle on and off', 'quiet revisions unchanged over 1500 ms'],
    initial, second, restored, settled, quiet,
  }, null, 2));
};
