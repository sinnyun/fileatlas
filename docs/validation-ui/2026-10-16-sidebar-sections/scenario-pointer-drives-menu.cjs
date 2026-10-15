// Stage G: pointer-opened popups with corrected geometry, plus one pointer-driven menu action.
// Measured from 70-drives-header-menu.png: drives header icons at screenshot y 160, quick access header y 212,
// top-level quick access rows from y 252 with 36px pitch, row ellipsis centred at x 380.
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

  // Deterministic layout: drives collapsed, quick access expanded with no tree rows open, favorites collapsed.
  let state = await waitFor('sidebar snapshot', (v) => v.page === 'folders');
  if (!state.drives_collapsed) await request({ kind: 'toggle_section', key: 'drives' });
  if (state.quick_access_collapsed) await request({ kind: 'toggle_section', key: 'quick_access' });
  if (!state.favorites_collapsed) await request({ kind: 'toggle_section', key: 'favorites' });
  state = await waitFor('sections set', (v) => v.drives_collapsed && v.favorites_collapsed && !v.quick_access_collapsed);
  for (const row of [...state.system_quick_access, ...state.quick_access]) {
    if (row.expanded) await request({ kind: 'toggle_expanded', path: row.path });
  }
  state = await waitFor('rows collapsed', (v) => ![...v.system_quick_access, ...v.quick_access].some((r) => r.expanded));
  const rows = [...state.system_quick_access, ...state.quick_access];
  console.log('rows', JSON.stringify(rows.map((r) => r.path)));
  const last = 252 + 36 * (rows.length - 1);

  await click(345, 160, '80-drives-menu');
  await click(430, 246, '81-network-hidden');
  const hidden = await waitFor('network drives hidden by menu click', (v) => v.show_network_drives === false);
  console.log('show_network_drives after menu click', hidden.show_network_drives);
  await click(345, 160, '82-drives-menu-again');
  await click(430, 203, '83-network-shown');
  await waitFor('network drives shown by menu click', (v) => v.show_network_drives === true);
  await click(600, 500, null, 300);
  await click(380, 252, '84-system-row-menu');
  await click(600, 500, null, 300);
  await click(380, last, '85-user-row-menu');
  await step({ tool: 'get_ui_state', capture: '86-final' });
};
