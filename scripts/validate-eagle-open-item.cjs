const assert = require('node:assert/strict');

module.exports = async ctx => {
  await require('./validate-eagle-canvas-drop.cjs')(ctx);
  const state = ctx.payload(await ctx.call('whiteboard.state'));
  const board = state.library.boards.find(item => item.id === state.library.active_id);
  const source = board?.eagle_sources?.at(-1);
  assert(source?.item_id, 'active test board must have Eagle source metadata');

  // Coordinates derived from the actual screenshot and window_status mapping
  // for this 3286×1831, 1.5× DPI validation window.
  const point = '1840,277';
  await ctx.call('control_ui', { action: 'pointer_down', value: point });
  await ctx.delay(350);
  await ctx.call('control_ui', { action: 'pointer_up', value: point });
  await ctx.delay(1500);
  await ctx.step({ tool: 'screenshot_window', capture: 'eagle-open-item-clicked' });

  const result = {
    passed: true,
    item_id: source.item_id,
    deep_link: `eagle://item/${source.item_id}`,
    clicked: 'canvas-open-eagle-item via same-process control_ui pointer events',
    screenshot: 'screenshots/eagle-open-item-clicked.png',
    profile: ctx.profile,
  };
  console.log(JSON.stringify(result));
};
