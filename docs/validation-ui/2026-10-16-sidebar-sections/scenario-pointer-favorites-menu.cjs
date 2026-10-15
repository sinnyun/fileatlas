// Stage I: pointer-opened menus of the grouped-favorites section.
// Measured from 90-favorites-expanded.png: favorites header y 683, group row y 722, favourite folder row y 759,
// all with the row ellipsis centred at screenshot x 380.
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
  if (!state.drives_collapsed) await request({ kind: 'toggle_section', key: 'drives' });
  if (state.quick_access_collapsed) await request({ kind: 'toggle_section', key: 'quick_access' });
  if (!state.folders_collapsed) await request({ kind: 'toggle_section', key: 'folders' });
  if (state.favorites_collapsed) await request({ kind: 'toggle_section', key: 'favorites' });
  state = await waitFor('sections set', (v) => v.drives_collapsed && v.folders_collapsed && !v.favorites_collapsed);
  for (const row of [...state.system_quick_access, ...state.quick_access]) {
    if (row.expanded) await request({ kind: 'toggle_expanded', path: row.path });
  }
  await waitFor('rows collapsed', (v) => ![...v.system_quick_access, ...v.quick_access].some((r) => r.expanded));

  await click(380, 683, '91-favorites-header-menu');
  await click(600, 500, null, 300);
  await click(380, 722, '92-group-row-menu');
  await click(600, 500, null, 300);
  await click(380, 759, '93-favorite-folder-menu');
  await step({ tool: 'get_ui_state', capture: '94-final' });
};
