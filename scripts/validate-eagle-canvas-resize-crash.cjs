const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

module.exports = async ({ call, payload, delay, step, profile }) => {
  const invoke = async (name, args = {}) => payload(await call(name, args));
  const invokeUi = async (action, value) => {
    for (let i = 0; i < 40; i++) {
      try {
        const result = await invoke('control_ui', { action, value });
        await delay(120);
        return result;
      } catch (error) {
        if (!String(error).includes('pending')) throw error;
        await delay(100);
      }
    }
    throw Error('FileAtlas MCP input queue stayed busy');
  };

  const original = process.env.FILEATLAS_EAGLE_REPRO_IMAGE;
  assert(original && fs.existsSync(original), 'Set FILEATLAS_EAGLE_REPRO_IMAGE to an Eagle original image path');
  const board = await invoke('whiteboard.create', { title: 'Eagle 拖入缩放崩溃复现' });
  await delay(700);
  await invoke('whiteboard.add_images', { id: board.id, paths: [original], at: [520, 360] });
  let state;
  for (let i = 0; i < 40; i++) {
    state = await invoke('whiteboard.state');
    const record = state.library.boards.find(item => item.id === board.id);
    const scene = record ? JSON.parse(record.scene) : null;
    if (scene?.elements.some(element => element.kind?.image)) break;
    await delay(150);
  }
  const record = state.library.boards.find(item => item.id === board.id);
  assert(record, 'repro board was not persisted');
  const scene = JSON.parse(record.scene);
  const image = scene.elements.find(element => element.kind?.image);
  assert(image, 'Eagle image was not inserted');
  const initial = { camera: scene.camera, image: image.kind.image };
  fs.writeFileSync(path.join(profile, 'eagle-canvas-resize-before.json'), JSON.stringify(initial, null, 2));
  await step({ tool: 'screenshot_window', capture: 'before-resize' });
  const uiState = await invoke('get_ui_state');
  fs.writeFileSync(path.join(profile, 'eagle-canvas-resize-ui.json'), JSON.stringify(uiState, null, 2));
  const pointer = async (action, x, y) => invokeUi(action, `${x},${y}`);
  // MCP screenshot-to-pointer conversion is documented by window_status:
  // first restore the screenshot's native size, subtract capture offset, then
  // divide by the Windows scale factor.
  const screenshot = fs.readFileSync(path.join(profile, 'screenshots', 'before-resize.png'));
  const screenshotScale = screenshot.readUInt32BE(16) / 2048;
  const down = [890, 384].map((value, index) => (value * screenshotScale - (index ? 45 : 11)) / 1.5);
  const resized = [down[0] + 55, down[1] - 35];
  await pointer('pointer_move', ...down);
  await pointer('pointer_down', ...down);
  for (let i = 1; i <= 5; i++) {
    await pointer('pointer_drag', down[0] + (resized[0] - down[0]) * i / 5, down[1] + (resized[1] - down[1]) * i / 5);
  }
  await pointer('pointer_up', ...resized);
  await delay(500);
  await step({ tool: 'screenshot_window', capture: 'after-resize' });
  state = await invoke('whiteboard.state');
  let updatedRecord = state.library.boards.find(item => item.id === board.id);
  let updatedScene = JSON.parse(updatedRecord.scene);
  let updatedImage = updatedScene.elements.find(element => element.kind?.image)?.kind.image;
  assert(updatedImage.w !== initial.image.w || updatedImage.h !== initial.image.h, 'resize gesture did not change image geometry');
  const resizedGeometry = { camera: updatedScene.camera, image: updatedImage };

  const offsets = [11, 45];
  const imageCenterOnScreenshot = [759, 459].map((value, index) => (value * screenshotScale - offsets[index]) / 1.5);
  const viewportOrigin = imageCenterOnScreenshot.map((value, index) =>
    value - ((index ? initial.image.y + initial.image.h / 2 - initial.camera.y : initial.image.x + initial.image.w / 2 - initial.camera.x) * initial.camera.zoom));
  const moveFrom = [
    viewportOrigin[0] + (updatedImage.x + updatedImage.w / 2 - updatedScene.camera.x) * updatedScene.camera.zoom,
    viewportOrigin[1] + (updatedImage.y + updatedImage.h / 2 - updatedScene.camera.y) * updatedScene.camera.zoom,
  ];
  const moveTo = [moveFrom[0] + 35, moveFrom[1] + 25];
  await pointer('pointer_move', ...moveFrom);
  await pointer('pointer_down', ...moveFrom);
  await pointer('pointer_drag', ...[moveFrom[0] + 12, moveFrom[1] + 8]);
  await pointer('pointer_drag', ...moveTo);
  await pointer('pointer_up', ...moveTo);
  await delay(500);
  await step({ tool: 'screenshot_window', capture: 'after-move' });
  state = await invoke('whiteboard.state');
  updatedRecord = state.library.boards.find(item => item.id === board.id);
  updatedScene = JSON.parse(updatedRecord.scene);
  updatedImage = updatedScene.elements.find(element => element.kind?.image)?.kind.image;
  assert(updatedImage.x !== resizedGeometry.image.x || updatedImage.y !== resizedGeometry.image.y, 'move gesture did not change image geometry');
  const final = { resized: resizedGeometry.image, moved: updatedImage };
  fs.writeFileSync(path.join(profile, 'eagle-canvas-resize-result.json'), JSON.stringify({ board_id: board.id, initial, final, process_alive: true }, null, 2));
  console.log(JSON.stringify({ stage: 'resize-and-move-complete', board_id: board.id, initial: initial.image, final }));
};
