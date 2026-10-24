// Same-process MCP audit for named Material theme replacement and UI copy.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

module.exports = async ({ step, delay, call, payload, profile }) => {
  const themeName = 'Overwrite-Theme-Check';
  const generate = async seed => {
    const result = payload(await call('material-theme.generate', {
      seed, variant: 'neutral', contrast: 0, name: themeName, apply: true,
    }));
    assert.equal(result.last_error, null);
    return result;
  };
  const first = await generate('#c2185b');
  const firstJson = fs.readFileSync(first.output, 'utf8');
  const firstConfig = JSON.parse(firstJson).themes.find(theme => theme.mode === 'light');
  const second = await generate('#00897b');
  const secondJson = fs.readFileSync(second.output, 'utf8');
  const backup = fs.readFileSync(second.output.replace(/\.json$/, '.json.bak'), 'utf8');
  assert.equal(second.output, first.output, 'same name must target the same theme file');
  assert.equal(backup, firstJson, 'backup must contain the previous theme exactly');
  assert.notEqual(secondJson, firstJson, 'same-name generation must replace the theme');

  await step({ tool: 'control_ui', arguments: { action: 'settings_open' }, wait_ms: 250 });
  await step({ tool: 'control_ui', arguments: { action: 'settings_plugin_page', value: 'fileatlas.material-theme' }, wait_ms: 500 });
  await delay(300);
  await step({ tool: 'get_ui_state', capture: 'material-theme-overwrite-settings-top' });
  await step({
    tool: 'control_ui',
    arguments: { action: 'pointer_scroll', value: '1500,700,-1200' },
    wait_ms: 400,
  });
  await step({ tool: 'get_ui_state', capture: 'material-theme-overwrite-settings-save' });
  const state = payload(await call('get_ui_state'));
  const generator = payload(await call('material-theme.state'));
  const applied = JSON.parse(secondJson).themes.find(theme => theme.mode === state.theme.mode);
  assert.equal(state.theme.id.includes('Generation-'), true);
  assert.equal(state.theme.accent, parseInt(applied.colors['primary.background'].slice(1), 16));
  assert.equal(generator.output, second.output);
  assert.equal(
    JSON.parse(fs.readFileSync(path.join(profile, 'settings.json'), 'utf8')).theme_file,
    second.output,
  );

  const ui = async (action, value) => payload(await call('control_ui', {
    action, ...(value === undefined ? {} : { value }),
  }));
  const click = async point => {
    for (const action of ['pointer_move', 'pointer_down', 'pointer_up']) {
      await ui(action, point);
      await delay(120);
    }
  };
  const pointer = async (x, y) => {
    const geometry = payload(await call('get_ui_state')).window_status.geometry;
    const map = geometry.screenshot_to_pointer;
    fs.writeFileSync(path.join(profile, 'window-geometry.json'), JSON.stringify(geometry, null, 2));
    const screenshotX = geometry.client_size.width * map.divide_by * x + map.offset_x;
    const screenshotY = geometry.client_size.height * map.divide_by * y + map.offset_y;
    return `${(screenshotX - map.offset_x) / map.divide_by},${(screenshotY - map.offset_y) / map.divide_by}`;
  };
  const inputPoint = await pointer(0.67, 0.69);
  const applyPoint = await pointer(0.71, 0.78);
  fs.writeFileSync(path.join(profile, 'interaction-points.json'), JSON.stringify({ inputPoint, applyPoint }, null, 2));
  await click(inputPoint);
  await ui('key_press', 'ctrl-a');
  await delay(180);
  await ui('input_text', themeName);
  await delay(400);
  await step({ tool: 'get_ui_state', capture: 'material-theme-overwrite-name-entered' });
  await click(applyPoint);
  await delay(900);
  const uiGenerated = payload(await call('material-theme.state'));
  const uiJson = fs.readFileSync(uiGenerated.output, 'utf8');
  const uiBackup = fs.readFileSync(second.output.replace(/\.json$/, '.json.bak'), 'utf8');
  assert.equal(uiGenerated.output, second.output);
  assert.equal(uiBackup, secondJson, 'settings action must back up the most recent same-name theme');
  assert.notEqual(uiJson, secondJson, 'settings action must replace same-name theme');
  const finalState = payload(await call('get_ui_state'));
  const finalConfig = JSON.parse(uiJson).themes.find(theme => theme.mode === finalState.theme.mode);
  assert.equal(finalState.theme.accent, parseInt(finalConfig.colors['primary.background'].slice(1), 16));
  await step({ tool: 'get_ui_state', capture: 'material-theme-overwrite-settings-applied' });
  fs.writeFileSync(path.join(profile, 'material-overwrite-audit.json'), JSON.stringify({
    passed: true,
    ui_same_output: uiGenerated.output === second.output,
    ui_backup_matches_previous: uiBackup === secondJson,
    ui_file_changed: uiJson !== secondJson,
    accent_matches_file: finalState.theme.accent === parseInt(finalConfig.colors['primary.background'].slice(1), 16),
    output: second.output,
    backup: second.output.replace(/\.json$/, '.json.bak'),
    first_config: firstConfig.name,
    applied_theme_id: state.theme.id,
    applied_primary: state.theme.accent,
    settings_action_theme_id: finalState.theme.id,
    settings_action_primary: finalState.theme.accent,
    selected_theme_file: second.output,
  }, null, 2));
};
