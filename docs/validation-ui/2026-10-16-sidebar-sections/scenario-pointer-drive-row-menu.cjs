// Stage K: pointer-opened menu on a drive row (measured from 95-drives-expanded.png: first drive row y 201).
module.exports = async ({ step, call, payload, delay }) => {
  let ox = 11, oy = 45, s = 1.5;
  const snapshot = async () => payload(await call('get_ui_state')).sidebar_folders;
  const waitFor = async (label, check) => {
    for (let attempt = 0; attempt < 24; attempt++) {
      const state = await snapshot();
      if (state && check(state)) return state;
      await delay(250);
    }
    throw new Error(`Timed out waiting for: ${label}`);
  };
  const request = (body) =>
    step({ tool: 'control_ui', arguments: { action: 'sidebar_request', request: body }, wait_ms: 250 });
  const click = async (sx, sy, capture, wait = 700) => {
    const value = `${Math.round((sx - ox) / s)},${Math.round((sy - oy) / s)}`;
    await step({ tool: 'control_ui', arguments: { action: 'pointer_move', value }, wait_ms: 150 });
    await step({ tool: 'control_ui', arguments: { action: 'pointer_down', value }, wait_ms: 150 });
    await step({ tool: 'control_ui', arguments: { action: 'pointer_up', value }, wait_ms: wait, capture });
  };

  await step({ tool: 'get_ui_state', until: { field: 'window_status.valid', equals: true, attempts: 20, interval_ms: 500 }, wait_ms: 800 });
  await step({ tool: 'resize_window', arguments: { width: 1280, height: 850 }, wait_ms: 400 });
  const ready = await step({ tool: 'get_ui_state' });
  const geometry = ready.result.window_status.geometry;
  ({ offset_x: ox, offset_y: oy } = geometry.screenshot_to_pointer);
  s = geometry.scale_factor;
  await step({ tool: 'control_ui', arguments: { action: 'sidebar_width', value: '264' }, wait_ms: 300 });
  let state = await waitFor('sidebar snapshot', (v) => v.page === 'folders');
  if (state.drives_collapsed) await request({ kind: 'toggle_section', key: 'drives' });
  if (!state.quick_access_collapsed) await request({ kind: 'toggle_section', key: 'quick_access' });
  if (!state.favorites_collapsed) await request({ kind: 'toggle_section', key: 'favorites' });
  if (!state.folders_collapsed) await request({ kind: 'toggle_section', key: 'folders' });
  await waitFor('drives expanded, rest collapsed', (v) =>
    !v.drives_collapsed && v.quick_access_collapsed && v.favorites_collapsed && v.folders_collapsed);

  await click(380, 201, '96-drive-row-menu');
  await click(600, 500, null, 300);
  await click(340, 201, '97-drive-row-menu-alt');
  await step({ tool: 'get_ui_state', capture: '98-final' });
};
