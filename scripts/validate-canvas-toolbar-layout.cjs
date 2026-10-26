const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, step, profile }) => {
  const invoke = async (name, args = {}) => payload(await call(name, args));
  await delay(2500);
  const board = await invoke('whiteboard.create', { title: '工具栏布局验证' });
  await delay(700);
  const state = await invoke('whiteboard.state');
  assert(
    state.library.boards.some(item => item.id === board.id),
    'test board should be active',
  );
  await step({ tool: 'screenshot_window', capture: 'canvas-toolbar-layout' });
  const group = await invoke('whiteboard.tool_group', { group: 'shapes' });
  assert.equal(group.queued, true);
  await delay(450);
  await step({ tool: 'screenshot_window', capture: 'canvas-toolbar-shapes' });
  for (const menu of ['color', 'font', 'templates']) {
    const result = await invoke('whiteboard.toolbar_menu', { menu });
    assert.equal(result.queued, true);
    await delay(450);
    await step({ tool: 'screenshot_window', capture: `canvas-toolbar-${menu}` });
  }
  console.log(JSON.stringify({
    passed: true,
    board_id: board.id,
    profile,
    captures: [
      'canvas-toolbar-layout',
      'canvas-toolbar-shapes',
      'canvas-toolbar-color',
      'canvas-toolbar-font',
      'canvas-toolbar-templates',
    ],
  }));
};
