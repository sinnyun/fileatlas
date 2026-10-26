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
  await call('control_ui', { action: 'pointer_down', value: '1310,872' });
  await delay(250);
  await call('control_ui', { action: 'pointer_up', value: '1310,872' });
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
