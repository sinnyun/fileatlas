// Confirm that shared button, folder/tree and table hovers share the primary family.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

module.exports = async ({ step, delay, call, payload, profile }) => {
  const accent = '#FF00AA';
  await delay(3500);
  const generated = payload(await call('material-theme.generate', {
    seed: accent, variant: 'neutral', contrast: 0,
    name: 'Hover-Accent-Check', apply: true,
  }));
  assert.equal(generated.last_error, null);
  const generatedSet = JSON.parse(fs.readFileSync(path.join(profile, 'themes', 'Hover-Accent-Check.json'), 'utf8'));
  for (const mode of ['light', 'dark']) {
    const colors = generatedSet.themes.find(theme => theme.mode === mode).colors;
    for (const role of ['button.hover.background', 'button.secondary.hover.background', 'secondary.hover.background', 'accent.background', 'list.hover.background', 'table.hover.background']) {
      assert.equal(colors[role].toLowerCase(), colors['accent.background'].toLowerCase(), `${mode} ${role}`);
    }
  }
  await delay(500);
  const before = payload(await call('get_ui_state'));
  const activeModeConfig = generatedSet.themes.find(theme => theme.mode === before.theme.mode);
  const configuredHover = activeModeConfig.colors['accent.background'].toLowerCase();
  assert.equal(before.theme.roles.native_accent.toLowerCase(), activeModeConfig.colors['primary.background'].toLowerCase());
  assert.equal(before.theme.roles.native_button_hover.toLowerCase(), configuredHover);
  await step({ tool: 'get_ui_state', capture: 'material-hover-accent-before' });

  const geometry = before.window_status.geometry;
  const map = geometry.screenshot_to_pointer;
  const screenshotX = geometry.client_size.width * 0.16 * map.divide_by + map.offset_x;
  const screenshotY = geometry.client_size.height * 0.03 * map.divide_by + map.offset_y;
  const pointer = `${(screenshotX - map.offset_x) / map.divide_by},${(screenshotY - map.offset_y) / map.divide_by}`;
  await step({ tool: 'control_ui', arguments: { action: 'pointer_move', value: pointer }, wait_ms: 400 });
  await step({ tool: 'get_ui_state', capture: 'material-hover-accent-hovered' });
  const after = payload(await call('get_ui_state'));
  fs.writeFileSync(path.join(profile, 'material-hover-accent-audit.json'), JSON.stringify({
    passed: true,
    primary_seed: accent,
    native_accent: after.theme.roles.native_accent,
    native_button_hover: after.theme.roles.native_button_hover,
    generated_hover_roles: Object.fromEntries(['light', 'dark'].map(mode => {
      const colors = generatedSet.themes.find(theme => theme.mode === mode).colors;
      return [mode, { accent: colors['accent.background'], list: colors['list.hover.background'], table: colors['table.hover.background'] }];
    })),
    button_hover_role: after.theme.roles.native_button_hover,
    pointer,
    screenshot: ['material-hover-accent-before.png', 'material-hover-accent-hovered.png'],
    note: 'One generated accent tonal role supplies native ghost button, list/card/tree and table hover roles; the material seed drives the primary family.',
  }, null, 2));
};
