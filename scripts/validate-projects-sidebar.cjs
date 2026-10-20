const fs = require('node:fs');
const path = require('node:path');

module.exports = async ({ call, payload, delay, profile, output }) => {
  const invoke = async (tool, args = {}) => payload(await call(tool, args));
  const pointer = async (action, x, y) => invoke('control_ui', {
    action,
    value: `${x},${y}`,
  });
  const click = async (x, y) => {
    await pointer('pointer_down', x, y);
    await delay(1000);
    await pointer('pointer_up', x, y);
    await delay(180);
  };
  const initial = await invoke('get_ui_state');
  if (initial.navigation_tab !== 'folders') throw new Error(`Unexpected initial sidebar tab: ${initial.navigation_tab}`);
  await click(22, 206);
  let state = await invoke('get_ui_state');
  if (state.navigation_tab !== 'projects') throw new Error(`Projects tab did not activate: ${state.navigation_tab}`);
  let shot = await call('screenshot_window');
  let image = shot.content.find(item => item.type === 'image');
  fs.writeFileSync(path.join(output, 'projects-sidebar-restored.png'), Buffer.from(image.data, 'base64'));

  const tree = path.join(profile, 'fixture-worktree');
  for (const child of ['docs/notes', 'src/deep']) fs.mkdirSync(path.join(tree, child), { recursive: true });
  await invoke('control_file_manager', { action: 'navigate', value: tree });
  await delay(1000);
  state = await invoke('get_ui_state');
  const normalize = value => value.replace(/^\\\\\?\\/, '').replaceAll('/', '\\').replace(/\\+$/, '').toLowerCase();
  if (normalize(state.file_manager.current_directory ?? '') !== normalize(tree)) throw new Error(`Fixture folder did not open: ${state.file_manager.current_directory}`);
  await click(22, 206);
  state = await invoke('get_ui_state');
  if (state.navigation_tab !== 'projects') throw new Error('Projects tab did not activate after opening a folder.');
  await click(245, 110);
  await delay(1000);
  shot = await call('screenshot_window');
  image = shot.content.find(item => item.type === 'image');
  fs.writeFileSync(path.join(output, 'projects-sidebar-added.png'), Buffer.from(image.data, 'base64'));
  state = await invoke('get_ui_state');
  if (state.navigation_tab !== 'projects') throw new Error('Sidebar left the Projects tab unexpectedly.');
  const manifest = JSON.parse(fs.readFileSync(path.join(profile, 'projects', 'projects.json'), 'utf8'));
  if (manifest.version !== 1 || !manifest.projects.some(project => normalize(project.path) === normalize(tree))) {
    throw new Error(`Project path was not persisted: ${JSON.stringify(manifest)}`);
  }
  await click(118, 315);
  await delay(1000);
  state = await invoke('get_ui_state');
  const expectedDirectory = path.join(tree, 'src', 'deep');
  if (state.navigation_tab !== 'projects') throw new Error(`Selecting a project child changed the sidebar tab: ${state.navigation_tab}`);
  if (normalize(state.file_manager.current_directory ?? '') !== normalize(expectedDirectory)) {
    throw new Error(`Selecting a nested project directory did not navigate the browser: ${state.file_manager.current_directory}`);
  }
  await pointer('pointer_move', 900, 700);
  await delay(300);
  shot = await call('screenshot_window');
  image = shot.content.find(item => item.type === 'image');
  fs.writeFileSync(path.join(output, 'projects-sidebar-navigated.png'), Buffer.from(image.data, 'base64'));
  console.log(JSON.stringify({ initialTab: initial.navigation_tab, activeTab: state.navigation_tab, manifest, tree, navigatedTo: state.file_manager.current_directory, screenshots: ['projects-sidebar-restored.png', 'projects-sidebar-added.png', 'projects-sidebar-navigated.png'] }));
};
