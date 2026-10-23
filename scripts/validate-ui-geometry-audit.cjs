module.exports = async ({ step, delay }) => {
  await delay(4000);
  await step({ tool: 'get_ui_state', capture: 'geometry-light-main' });
  const launcher = await step({
    tool: 'control_ui',
    arguments: { action: 'panel_open', value: 'program-launcher/panel' },
    wait_ms: 700,
  });
  await step({ tool: 'get_ui_state', capture: 'geometry-light-launcher' });
  await step({ tool: 'control_ui', arguments: { action: 'panel_close', value: launcher.result.instance_id }, wait_ms: 400 });
  const workcrate = await step({
    tool: 'control_ui',
    arguments: { action: 'panel_open', value: 'workcrate/overview' },
    wait_ms: 900,
  });
  await step({ tool: 'get_ui_state', capture: 'geometry-light-workcrate' });
  await step({ tool: 'control_ui', arguments: { action: 'panel_close', value: workcrate.result.instance_id }, wait_ms: 400 });
  await step({ tool: 'control_ui', arguments: { action: 'settings_open' }, wait_ms: 700 });
  await step({ tool: 'get_ui_state', capture: 'geometry-light-settings' });
  await step({ tool: 'control_ui', arguments: { action: 'settings_close' }, wait_ms: 400 });
};
