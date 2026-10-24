// Same-process MCP visual audit for Material color roles. All generated themes,
// fixtures, reports and screenshots stay in the caller's isolated profile.
const fs = require('node:fs');
const path = require('node:path');
module.exports = async ({ step, delay, call, payload, root, profile }) => {
  let windowReady = false;
  for (let attempt = 0; attempt < 30; attempt++) {
    const state = payload(await call('get_ui_state'));
    if (state?.window_status?.valid && state.window_status.owned) {
      windowReady = true;
      break;
    }
    await delay(1000);
  }
  if (!windowReady) throw Error('The same-process MCP window did not become ready within 30 seconds');

  const themeName = `ColorRoleAudit${Date.now()}`;
  const seed = '#C2185B';
  const manualColors = {
    background: '#7CFC00',
    foreground: '#172554',
    'secondary.background': '#00897B',
    'success.background': '#2E7D32',
    'warning.background': '#EF6C00',
    'danger.background': '#C62828',
    'info.background': '#0088AA',
  };
  const generated = payload(await call('material-theme.generate', {
    seed, variant: 'neutral', contrast: 0,
    name: themeName, apply: true, manual_colors: manualColors,
  }));
  if (generated.last_error) throw Error(`Theme generation failed: ${generated.last_error}`);
  await delay(700);
  const captures = [];
  const shot = async name => {
    await step({ tool: 'get_ui_state', capture: name });
    captures.push(`${name}.png`);
  };
  await shot('material-color-main-empty');
  await step({ tool: 'control_ui', arguments: { action: 'settings_open' }, wait_ms: 350 });
  await step({ tool: 'control_ui', arguments: { action: 'settings_plugin_page', value: 'fileatlas.material-theme' }, wait_ms: 450 });
  await shot('material-color-settings-top');
  await step({ tool: 'control_ui', arguments: { action: 'pointer_scroll', value: '1500,700,-1100' }, wait_ms: 400 });
  await shot('material-color-settings-middle');
  await step({ tool: 'control_ui', arguments: { action: 'pointer_scroll', value: '1500,700,-1100' }, wait_ms: 400 });
  await shot('material-color-settings-save');
  await step({ tool: 'control_ui', arguments: { action: 'settings_close' }, wait_ms: 350 });

  const fixture = path.join(root, 'test-data', 'fileatlas-fixtures');
  const scan = payload(await call('dev-test.scan_fixture'));
  if (!scan?.accepted) throw Error(`Fixture scan was not accepted: ${JSON.stringify(scan)}`);
  let scanStatus;
  for (let attempt = 0; attempt < 60; attempt++) {
    scanStatus = payload(await call('dev-test.status'));
    if (scanStatus && !scanStatus.running && scanStatus.status?.task?.running !== true) break;
    await delay(500);
  }
  if (!scanStatus || scanStatus.running || scanStatus.status?.task?.running === true) {
    throw Error(`Fixture scan did not finish: ${JSON.stringify(scanStatus)}`);
  }
  await step({ tool: 'control_file_manager', arguments: { action: 'navigate', value: fixture }, wait_ms: 1200 });
  await step({ tool: 'control_file_manager', arguments: { action: 'show_descendants', enabled: true }, wait_ms: 800 });
  await step({ tool: 'control_file_manager', arguments: { action: 'grouping', value: 'none' }, wait_ms: 700 });
  const populated = payload(await call('get_ui_state'));
  if ((populated?.file_manager?.matched_count ?? 0) === 0) {
    throw Error(`Fixture rows are not visible: ${JSON.stringify(populated?.file_manager)}`);
  }
  await shot('material-color-populated-grid');
  const result = payload(await call('file_manager.search_media', { limit: 60, search: 'fileatlas-fixtures' }));
  const first = result?.items?.[0];
  if (!first?.id) throw Error(`No indexed fixture item found: ${JSON.stringify(result)}`);
  await step({ tool: 'control_ui', arguments: { action: 'select_media', value: String(first.id) }, wait_ms: 900 });
  await shot('material-color-populated-grid-selected');
  await step({ tool: 'control_file_manager', arguments: { action: 'layout', value: 'list' }, wait_ms: 700 });
  await shot('material-color-populated-list');
  await step({ tool: 'control_ui', arguments: { action: 'select_media', value: String(first.id) }, wait_ms: 700 });
  await shot('material-color-populated-list-selected');

  const nestedFolder = path.join(fixture, '03-nested');
  await step({ tool: 'control_file_manager', arguments: { action: 'navigate', value: nestedFolder }, wait_ms: 900 });
  await step({ tool: 'control_file_manager', arguments: { action: 'layout', value: 'grid' }, wait_ms: 350 });
  const hoverState = payload(await call('get_ui_state'));
  const hoverGeometry = hoverState.window_status.geometry;
  const hoverMap = hoverGeometry.screenshot_to_pointer;
  const moveHover = async (x, y, name) => {
    const rawX = hoverGeometry.client_size.width * hoverMap.divide_by * x + hoverMap.offset_x;
    const rawY = hoverGeometry.client_size.height * hoverMap.divide_by * y + hoverMap.offset_y;
    const targetX = (rawX - hoverMap.offset_x) / hoverMap.divide_by;
    const targetY = (rawY - hoverMap.offset_y) / hoverMap.divide_by;
    const point = `${targetX},${targetY}`;
    await step({ tool: 'control_ui', arguments: { action: 'pointer_move', value: point }, wait_ms: 100 });
    let arrived = false;
    for (let attempt = 0; attempt < 30; attempt++) {
      await delay(100);
      const current = payload(await call('get_ui_state'));
      const pointer = current?.input_activity?.pointer;
      if (pointer && Math.abs(pointer[0] - targetX) < 1 && Math.abs(pointer[1] - targetY) < 1) {
        arrived = true;
        break;
      }
    }
    if (!arrived) throw Error(`Pointer did not reach ${name} target (${point})`);
    await shot(name);
  };
  await moveHover(0.17, 0.145, 'material-color-folder-card-hover');
  await moveHover(0.07, 0.48, 'material-color-folder-tree-hover');

  const state = payload(await call('get_ui_state'));
  const material = payload(await call('material-theme.state'));
  const themeSet = JSON.parse(fs.readFileSync(material.output, 'utf8'));
  const roles = state.theme.roles;
  const settings = JSON.parse(fs.readFileSync(path.join(profile, 'settings.json'), 'utf8'));
  for (const mode of ['light', 'dark']) {
    const colors = themeSet.themes.find(theme => theme.mode === mode)?.colors;
    if (!colors) throw Error(`Generated ThemeSet is missing ${mode} mode`);
    for (const [role, expected] of Object.entries(manualColors)) {
      if (colors[role]?.toLowerCase() !== expected.toLowerCase()) {
        throw Error(`${mode} ${role} is ${colors[role]}, expected ${expected}`);
      }
    }
  }
  for (const role of ['native_button_active', 'native_button_secondary_active', 'native_secondary_active']) {
    const expected = themeSet.themes.find(theme => theme.mode === state.theme.mode).colors['accent.background'];
    if (roles[role]?.toLowerCase() !== expected.toLowerCase()) {
      throw Error(`${role} is ${roles[role]}, expected shared interaction family ${expected}`);
    }
  }
  fs.writeFileSync(path.join(profile, 'material-color-audit.json'), JSON.stringify({
    output: material.output,
    selected_theme_file: settings.theme_file,
    theme_id: state.theme.id,
    mode: state.theme.mode,
    scan_status: scanStatus,
    fixture_rows: state.file_manager.matched_count,
    selected_media_ids: state.file_manager.selected_media_ids,
    native_roles: roles,
    white_foreground_roles: Object.fromEntries(Object.entries(roles).filter(([key, value]) => key.includes('foreground') && value?.toLowerCase() === '#ffffff')),
    manual_roles_light: Object.fromEntries(Object.keys(manualColors).map(key => [key, themeSet.themes.find(t => t.mode === 'light').colors[key]])),
    manual_roles_dark: Object.fromEntries(Object.keys(manualColors).map(key => [key, themeSet.themes.find(t => t.mode === 'dark').colors[key]])),
    shared_hover_roles: Object.fromEntries(['button.hover.background','button.secondary.hover.background','secondary.hover.background','accent.background','list.hover.background','table.hover.background'].map(key => [key, themeSet.themes.find(t => t.mode === state.theme.mode).colors[key]])),
    palette_family_count: 8,
    pointer_hover_screenshots: ['material-color-folder-card-hover.png', 'material-color-folder-tree-hover.png'],
    captures,
  }, null, 2));
};
