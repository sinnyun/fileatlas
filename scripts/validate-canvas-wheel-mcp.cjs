const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, profile }) => {
  const invoke = async (name, args = {}) => payload(await call(name, args));
  const board = await invoke('whiteboard.create', { title: 'MCP 滚轮语义验证' });
  await delay(500);
  const readCamera = async () => {
    const state = await invoke('whiteboard.state');
    const record = state.library.boards.find(item => item.id === board.id);
    assert(record, 'created board should remain in whiteboard state');
    return JSON.parse(record.scene).camera;
  };
  const initial = await readCamera();
  const anchor = [120, 80];
  const zoomInput = await invoke('whiteboard.scroll', {
    id: board.id,
    delta_x: 0,
    delta_y: 160,
    control: false,
    at: anchor,
  });
  assert.equal(zoomInput.queued, true);
  let zoomed = initial;
  for (let attempt = 0; attempt < 30; attempt++) {
    await delay(100);
    zoomed = await readCamera();
    if (Math.abs(zoomed.zoom - initial.zoom) > 0.01) break;
  }
  assert(Math.abs(zoomed.zoom - 1.4) < 0.01, `ordinary wheel should zoom: ${JSON.stringify({ initial, zoomed })}`);
  const beforeWorld = [initial.x + anchor[0] / initial.zoom, initial.y + anchor[1] / initial.zoom];
  const afterWorld = [zoomed.x + anchor[0] / zoomed.zoom, zoomed.y + anchor[1] / zoomed.zoom];
  assert(Math.abs(beforeWorld[0] - afterWorld[0]) < 0.01, 'zoom should keep the pointer world x fixed');
  assert(Math.abs(beforeWorld[1] - afterWorld[1]) < 0.01, 'zoom should keep the pointer world y fixed');

  const panInput = await invoke('whiteboard.scroll', {
    id: board.id,
    delta_x: 0,
    delta_y: 80,
    control: true,
    at: anchor,
  });
  assert.equal(panInput.queued, true);
  let panned = zoomed;
  for (let attempt = 0; attempt < 30; attempt++) {
    await delay(100);
    panned = await readCamera();
    if (Math.abs(panned.y - zoomed.y) > 0.01) break;
  }
  assert.equal(panned.zoom, zoomed.zoom, 'Ctrl+wheel should preserve zoom');
  assert(Math.abs(panned.x - zoomed.x) < 0.01, 'vertical Ctrl+wheel should not move the canvas horizontally');
  assert(Math.abs((panned.y - zoomed.y) + 80 / zoomed.zoom) < 0.01, 'Ctrl+wheel should pan by screen delta at current zoom');
  const result = { passed: true, initial, zoomed, panned, anchor, profile };
  console.log(JSON.stringify(result));
};
