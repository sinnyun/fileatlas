// Stage D: semantic sidebar verification through MCP requests (no pixel hunting).
const CRATES = 'F:/AI/geren/pho_cordis/crates';
const DOCS = 'F:/AI/geren/pho_cordis/docs';

module.exports = async ({ step, call, payload, delay }) => {
  const snapshot = async () => {
    const state = payload(await call('get_ui_state'));
    return state.sidebar_folders;
  };
  const waitFor = async (label, check) => {
    for (let attempt = 0; attempt < 24; attempt++) {
      const state = await snapshot();
      if (state && check(state)) return state;
      await delay(250);
    }
    throw new Error(`Timed out waiting for: ${label}`);
  };
  const request = async (body, capture) =>
    step({
      tool: 'control_ui',
      arguments: { action: 'sidebar_request', request: body },
      wait_ms: 250,
      ...(capture ? { capture } : {}),
    });

  await step({ tool: 'get_ui_state', until: { field: 'window_status.valid', equals: true, attempts: 20, interval_ms: 500 }, wait_ms: 800 });
  await step({ tool: 'resize_window', arguments: { width: 1280, height: 850 } });
  await step({ tool: 'control_ui', arguments: { action: 'sidebar_width', value: '264' }, wait_ms: 400 });
  const initial = await waitFor('first sidebar snapshot', (state) => state.page === 'folders');
  console.log('initial quick_access', JSON.stringify(initial.quick_access));
  console.log('initial groups', JSON.stringify(initial.favorite_groups.map((group) => group.name)));
  console.log('initial drives', JSON.stringify(initial.drives.map((drive) => [drive.root, drive.label, drive.total_bytes])));

  // 1. network drive toggle round-trip
  await request({ kind: 'set_network_drives', show: false });
  await waitFor('network drives hidden', (state) => state.show_network_drives === false);
  await request({ kind: 'set_network_drives', show: true });
  await waitFor('network drives shown', (state) => state.show_network_drives === true);

  // 2. rebuild quick access from scratch
  const seeded = (await snapshot()).quick_access.map((entry) => entry.path);
  for (const path of seeded) {
    await request({ kind: 'remove_quick_access', path });
  }
  await waitFor('quick access emptied', (state) => state.quick_access.length === 0);
  await request({ kind: 'add_quick_access', path: CRATES });
  await waitFor('quick access added', (state) => state.quick_access.length === 1 && state.quick_access[0].path === CRATES);
  await request({ kind: 'open_editor', text: '重命名测试', editor: { target: 'rename_quick_access', path: CRATES } });
  await waitFor('rename editor open', (state) => state.editor?.target === `RenameQuickAccess { path: "${CRATES}" }` && state.editor?.text === '重命名测试');
  await request({ kind: 'confirm_editor' });
  const renamed = await waitFor('quick access display name', (state) => state.quick_access[0]?.display_name === '重命名测试');
  console.log('renamed entry', JSON.stringify(renamed.quick_access));

  // 3. group lifecycle through the inline editor
  await request({ kind: 'open_editor', text: 'MCP分组', editor: { target: 'create_group' } });
  await waitFor('create group editor', (state) => state.editor?.target === 'CreateGroup');
  await request({ kind: 'confirm_editor' });
  const created = await waitFor('group created', (state) => state.favorite_groups.some((group) => group.name === 'MCP分组'));
  const groupId = created.favorite_groups.find((group) => group.name === 'MCP分组').id;
  await request({ kind: 'add_favorite_folder', group_id: groupId, path: DOCS });
  await waitFor('folder added to group', (state) => state.favorite_groups.find((group) => group.id === groupId)?.folders.length === 1);
  const folderId = (await snapshot()).favorite_groups.find((group) => group.id === groupId).folders[0].id;
  await request({ kind: 'open_editor', text: '文档目录', editor: { target: 'rename_favorite', group_id: groupId, folder_id: folderId } });
  await request({ kind: 'confirm_editor' });
  await waitFor('favorite folder renamed', (state) => state.favorite_groups.find((group) => group.id === groupId)?.folders[0]?.display_name === '文档目录');
  await request({ kind: 'open_editor', text: 'MCP分组改名', editor: { target: 'rename_group', group_id: groupId } });
  await request({ kind: 'confirm_editor' });
  await waitFor('group renamed', (state) => state.favorite_groups.some((group) => group.id === groupId && group.name === 'MCP分组改名'));
  await request({ kind: 'remove_favorite_folder', group_id: groupId, folder_id: folderId });
  await waitFor('favorite folder removed', (state) => state.favorite_groups.find((group) => group.id === groupId)?.folders.length === 0);
  await request({ kind: 'remove_favorite_group', group_id: groupId });
  await waitFor('group removed', (state) => !state.favorite_groups.some((group) => group.id === groupId));

  // 4. editor cancel path
  await request({ kind: 'open_editor', text: '不会保存', editor: { target: 'create_group' } });
  await waitFor('editor reopened', (state) => state.editor?.text === '不会保存');
  await request({ kind: 'cancel_editor' }, '40-editor-cancelled');
  await waitFor('editor cancelled', (state) => state.editor === null && !state.favorite_groups.some((group) => group.name === '不会保存'));

  // 5. section collapse, drive expansion, row activation
  await request({ kind: 'toggle_section', key: 'drives' });
  await waitFor('drives collapsed', (state) => state.drives_collapsed === true);
  await request({ kind: 'toggle_section', key: 'drives' }, '41-drives-expanded');
  await waitFor('drives expanded', (state) => state.drives_collapsed === false);
  const root = (await snapshot()).drives[0]?.root;
  await request({ kind: 'toggle_drive', root });
  await waitFor('drive expanded', (state) => state.drives.find((drive) => drive.root === root)?.expanded === true);
  await request({ kind: 'toggle_drive', root }, '42-drive-children');
  await waitFor('drive collapsed', (state) => state.drives.find((drive) => drive.root === root)?.expanded === false);
  await request({ kind: 'toggle_expanded', path: CRATES });
  await waitFor('quick access row expanded', (state) => state.quick_access[0]?.expanded === true);
  await request({ kind: 'activate_path', path: CRATES });
  const selected = await waitFor('quick access row selected', (state) => state.quick_access[0]?.selected === true);
  console.log('selected rows', JSON.stringify({ drives: selected.drives.filter((d) => d.selected), quick: selected.quick_access.filter((r) => r.selected) }));

  // 6. add to library
  const before = (await snapshot()).library_folders.length;
  await request({ kind: 'add_to_library', path: CRATES }, '43-after-add-library');
  const library = await waitFor('library folder added', (state) => state.library_folders.length > before);
  console.log('library folders', JSON.stringify(library.library_folders));

  // 7. validation rejections
  await step({ tool: 'control_ui', arguments: { action: 'sidebar_request', request: { kind: 'remove_favorite_group', group_id: 'no-such-group' } }, expect_error: 'unknown favorite group' });
  await step({ tool: 'control_ui', arguments: { action: 'sidebar_request', request: { kind: 'toggle_section', key: 'nope' } }, expect_error: 'unknown section key' });
  await step({ tool: 'control_ui', arguments: { action: 'sidebar_request', request: { kind: 'add_quick_access', path: 'Q:/definitely-missing' } }, expect_error: 'not an existing directory' });
  await step({ tool: 'control_ui', arguments: { action: 'sidebar_request', request: { kind: 'toggle_drive', root: 'Q:\\' } }, expect_error: 'drive root is not present' });

  await step({ tool: 'get_ui_state', capture: '44-final' });
};
