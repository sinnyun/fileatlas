// Capture the native GPUI Kit tooltip through same-process FileAtlas MCP.
// The tooltip's main text uses the body tier; any shortcut hint uses assistant.
module.exports = async ({ step, call, payload, delay }) => {
  const invoke = async (tool, arguments_ = {}, extra = {}) =>
    (await step({ tool, arguments: arguments_, ...extra })).result;
  let state;
  for (let attempt = 0; attempt < 40; attempt++) {
    state = await invoke('get_ui_state');
    if (state.window_status?.geometry?.client_size) break;
    await delay(250);
  }
  if (!state.window_status?.geometry?.client_size) throw new Error('window geometry unavailable');
  const geometry = payload(await call('debug.ui_geometry'));
  const target = (geometry.entries || []).find(entry =>
    entry.tier === 'button' && entry.component_id?.includes('file_manager-layout-grid'));
  if (!target) throw new Error('file layout button geometry unavailable');
  const x = target.x + target.width / 2;
  const y = target.y + target.height / 2;
  await invoke('control_ui', { action: 'pointer_move', value: `${x},${y}` }, { wait_ms: 250 });
  await delay(1100);
  await invoke('get_ui_state', {}, { capture: 'typography-tooltip' });
};
