// Invoke three times with the same isolated profile. All UI input stays in GPUI.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
module.exports = async ({ step, profile, delay, processId }) => {
  const invoke = async (tool, args = {}, wait_ms = 0) => (await step({ tool, arguments: args, wait_ms })).result;
  const markerPath = path.join(profile, 'clipboard-restart-phase.json');
  const marker = fs.existsSync(markerPath) ? JSON.parse(fs.readFileSync(markerPath, 'utf8')) : null;
  let state = await invoke('get_clipboard_state');
  const record = phase => fs.writeFileSync(path.join(profile, `restart-${phase}.json`), JSON.stringify({ processId, state }, null, 2));
  if (marker?.phase === 'deleted') {
    assert.equal(state.count, 0, 'deleted history stays deleted across a new child process');
    assert.equal(state.categories.length, 0, 'deleted groups stay deleted across a new child process');
    record('deleted-verified');
    return;
  }
  let item = marker?.item;
  if (!marker) item = (await invoke('add_clipboard_text', { content: 'Clipboard restart persistence fixture' })).id;
  else {
    assert.equal(state.count, 1, 'history survives a new child process');
    assert.equal(state.categories[0]?.id, marker.group);
    assert.equal(state.items[0]?.category_id, marker.group, 'group assignment survives a new child process');
    record('restored');
  }
  await invoke('control_ui', { action: 'panel_open', value: 'clipboard-workbench/panel' }, 800);
  const ui = await invoke('get_ui_state');
  const panel = ui.panel_instances.find(p => p.id === ui.active_panel);
  const [px, py] = panel.bounds.split(',').map(Number);
  const click = async (x, y) => {
    for (const action of ['pointer_move', 'pointer_down', 'pointer_up']) {
      for (let attempt = 0; ; attempt++) {
        try { await invoke('control_ui', { action, value: `${px + x},${py + y}` }, 300); break; }
        catch (error) {
          if (attempt >= 20 || !String(error).includes('still pending')) throw error;
          await delay(100);
        }
      }
    }
  };
  if (!marker) {
    await click(200, 530);
    await invoke('control_ui', { action: 'input_text', value: '重启验证' }, 400);
    await click(80, 615);
    state = await invoke('get_clipboard_state');
    const group = state.categories.find(g => g.name === '重启验证');
    assert(group);
    await click(990, 468);
    await click(990, 528);
    state = await invoke('get_clipboard_state');
    assert.equal(state.items[0].category_id, group.id);
    fs.writeFileSync(markerPath, JSON.stringify({ phase: 'created', group: group.id, item }));
    record('created');
    await step({ tool: 'get_clipboard_state', capture: 'clipboard-restart-created' });
  } else {
    await step({ tool: 'get_clipboard_state', capture: 'clipboard-restart-restored' });
    await click(80, 570);
    await click(160, 625);
    await click(80, 615);
    state = await invoke('get_clipboard_state');
    assert.equal(state.categories.length, 0);
    assert.equal(state.items[0].category_id, null);
    await click(1118, 80);
    state = await invoke('get_clipboard_state');
    assert.equal(state.count, 0, 'delete selected history through its actual GPUI control');
    record('deleted');
    fs.writeFileSync(markerPath, JSON.stringify({ ...marker, phase: 'deleted' }));
  }
};
