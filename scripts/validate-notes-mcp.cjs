const assert = require('node:assert/strict');

module.exports = async ({ call, payload, step, delay, profile }) => {
  const folderResult = payload(await call('notes.create_folder', { name: 'Research' }));
  const folder = folderResult.path;
  const target = payload(await call('notes.create', {
    title: 'Target', folder, body: '# Target\n\nA linked note.'
  }));
  const source = payload(await call('notes.create', {
    title: 'Source', body: '# Source\n\nSee [[Target]].'
  }));
  const fixture = require('node:path').resolve('test-data/fileatlas-fixtures/01-small/small.png');
  const image = payload(await call('notes.insert_image', { id: source.id, source_path: fixture }));
  const updated = payload(await call('notes.update', {
    id: source.id,
    body: `# Source\n\nSee [[Target]].\n\n${image.markdown}`,
    category: 'Research', status: 'in_progress', tags: ['rust', 'ideas']
  }));
  assert.equal(updated.category, 'Research');
  assert.deepEqual(updated.tags, ['ideas', 'rust']);
  const search = payload(await call('notes.search', { query: 'ideas' }));
  assert(search.items.some(note => note.id === source.id));
  const beforeRename = payload(await call('notes.read', { id: target.id }));
  assert.equal(beforeRename.backlinks.length, 1);
  const renamed = payload(await call('notes.rename', { id: target.id, title: 'Renamed target' }));
  const moved = payload(await call('notes.move', { id: source.id, folder }));
  assert(moved.path.startsWith('Research/'));
  const afterRename = payload(await call('notes.read', { id: source.id }));
  assert(afterRename.body.includes('[[Renamed target]]'));
  assert.equal(payload(await call('notes.list', {})).items.length, 2);

  await delay(1500);
  await call('control_ui', { action: 'panel_open', value: 'notes/workbench' });
  await step({ tool: 'get_ui_state', capture: 'notes-panel', wait_ms: 750 });
  const uiState = payload(await call('get_ui_state', {}));
  const windowState = uiState.window_status;
  const geometry = windowState.geometry;
  const screenshot = await call('screenshot_window');
  const screenshotBlock = screenshot.content.find(item => item.type === 'image');
  assert(screenshotBlock, 'MCP screenshot returned no image for coordinate calibration');
  const screenshotPng = Buffer.from(screenshotBlock.data, 'base64');
  const screenshotWidth = screenshotPng.readUInt32BE(16);
  const screenshotHeight = screenshotPng.readUInt32BE(20);
  const toPointer = (xFraction, yFraction) => [
    Math.round((xFraction * screenshotWidth - geometry.screenshot_to_pointer.offset_x) / geometry.screenshot_to_pointer.divide_by),
    Math.round((yFraction * screenshotHeight - geometry.screenshot_to_pointer.offset_y) / geometry.screenshot_to_pointer.divide_by)
  ];
  const click = async (x, y) => {
    const value = `${x},${y}`;
    await call('control_ui', { action: 'pointer_down', value });
    await delay(300);
    await call('control_ui', { action: 'pointer_up', value });
    await delay(350);
  };
  const editorPoint = toPointer(900 / 2048, 432 / 1141);
  const splitButtonPoint = toPointer(1175 / 2048, 319 / 1141);
  await click(...editorPoint);
  await delay(300);
  const focusedUi = payload(await call('notes.ui_state', {}));
  assert.equal(focusedUi.editor_focused, true, 'pointer input did not focus the note editor');
  await call('control_ui', { action: 'key_press', value: 'ctrl-end' });
  await delay(250);
  const liveMarker = ' LIVE_PREVIEW_MCP_MARKER';
  await call('control_ui', { action: 'input_text', value: liveMarker });
  await delay(600);
  await click(...splitButtonPoint);
  let liveUi = payload(await call('notes.ui_state', {}));
  for (let attempt = 0; attempt < 20; attempt++) {
    if (liveUi.mode === 'split' && liveUi.dirty &&
        liveUi.editor_sample.includes(liveMarker.trim()) &&
        liveUi.preview_sample.includes(liveMarker.trim())) break;
    await delay(250);
    liveUi = payload(await call('notes.ui_state', {}));
  }
  assert.equal(liveUi.mode, 'split');
  assert.equal(liveUi.dirty, true);
  assert(liveUi.editor_sample.includes(liveMarker.trim()), 'same-process typing did not reach the note editor');
  assert(liveUi.preview_sample.includes(liveMarker.trim()), 'unsaved editor text did not reach the live preview');
  await step({ tool: 'get_ui_state', capture: 'notes-preview', wait_ms: 500 });
  await call('control_ui', { action: 'pointer_down', value: '28,276' });
  await delay(250);
  await call('control_ui', { action: 'pointer_up', value: '28,276' });
  await step({ tool: 'get_ui_state', capture: 'notes-navigation', wait_ms: 750 });

  const trashed = payload(await call('notes.trash', { id: source.id, confirm: true }));
  const restored = payload(await call('notes.restore', { path: trashed.trashed }));
  assert.equal(restored.id, source.id);
  const folderTrash = payload(await call('notes.trash_folder', { path: folder, confirm: true }));
  const trashItems = payload(await call('notes.trash_list', {}));
  assert(trashItems.items.some(item => item.path === folderTrash.path && item.is_folder));
  await call('control_ui', { action: 'pointer_down', value: '980,872' });
  await delay(250);
  await call('control_ui', { action: 'pointer_up', value: '980,872' });
  await step({ tool: 'get_ui_state', capture: 'notes-trash-panel', wait_ms: 500 });
  await call('resize_window', { width: 960, height: 720 });
  await step({ tool: 'get_ui_state', capture: 'notes-narrow-window', wait_ms: 500 });
  const restoredFolder = payload(await call('notes.restore_folder', { path: folderTrash.path }));
  assert.equal(restoredFolder.name, 'Research');
  const finalFolderTrash = payload(await call('notes.trash_folder', { path: restoredFolder.path, confirm: true }));
  await call('notes.delete_trash', { path: finalFolderTrash.path, confirm: true });
  assert.equal(payload(await call('notes.trash_list', {})).items.length, 0);
  const outline = payload(await call('notes.outline', { id: source.id }));
  assert.equal(outline.items[0].title, 'Source');
  console.log(JSON.stringify({ profile, folder, source: source.id, target: renamed.id, image: image.markdown, restored: restored.path }));
};
