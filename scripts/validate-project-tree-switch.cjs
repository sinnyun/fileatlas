const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, profile, output }) => {
  const invoke = async (tool, args = {}) => payload(await call(tool, args));
  const report = { profile, projects: [], checks: [] };
  const saveShot = async name => {
    const result = await call('screenshot_window');
    const image = result.content.find(item => item.type === 'image');
    assert(image, 'FileAtlas MCP screenshot did not return an image');
    fs.writeFileSync(path.join(output, `${name}.png`), Buffer.from(image.data, 'base64'));
  };
  const normalize = value => value.replace(/^\\\\\?\\/, '').replaceAll('/', '\\').replace(/\\+$/, '').toLowerCase();
  const waitFor = async predicate => {
    for (let i = 0; i < 80; i++) {
      const state = await invoke('get_ui_state');
      if (predicate(state)) return state;
      await delay(150);
    }
    throw Error('Projects sidebar did not reach the expected state');
  };
  const pointer = async (action, x, y) => {
    for (let retry = 0; ; retry++) {
      try {
        await invoke('control_ui', { action, value: `${x},${y}` });
        return;
      } catch (error) {
        if (retry >= 30 || !String(error).includes('pointer request is still pending')) throw error;
        await delay(150);
      }
    }
  };
  const click = async (x, y) => {
    await pointer('pointer_down', x, y);
    await delay(180);
    await pointer('pointer_up', x, y);
    await delay(350);
  };
  try {
    const alpha = path.join(profile, 'project-alpha');
    const beta = path.join(profile, 'project-beta');
    for (const directory of [
      path.join(alpha, 'docs', 'notes'), path.join(alpha, 'src', 'deep'),
      path.join(beta, 'assets', 'raw'), path.join(beta, 'src', 'engine'),
    ]) fs.mkdirSync(directory, { recursive: true });
    let state = await invoke('get_ui_state');
    if (state.navigation_tab !== 'folders') throw Error(`Unexpected initial tab: ${state.navigation_tab}`);
    await click(22, 206);
    state = await waitFor(value => value.navigation_tab === 'project-tree');

    for (const project of [alpha, beta]) {
      await invoke('control_file_manager', { action: 'navigate', value: project });
      state = await waitFor(value => normalize(value.file_manager?.current_directory ?? '') === normalize(project));
      await click(22, 206);
      state = await waitFor(value => value.navigation_tab === 'project-tree');
      await saveShot(`before-add-${report.projects.length}`);
      await click(85, 110);
      await delay(900);
      await saveShot(`after-add-${report.projects.length}`);
      state = await invoke('get_ui_state');
      report.latest = state;
      const manifestPath = path.join(profile, 'project-tree', 'project-tree.json');
      if (!fs.existsSync(manifestPath)) {
        await saveShot(`add-failed-${report.projects.length}`);
        throw Error(`Project plus did not persist a root. UI state: ${JSON.stringify(state)}`);
      }
      const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
      assert(manifest.roots.some(item => normalize(item.path) === normalize(project)), `Project tree root was not added: ${project}`);
      report.projects.push(project);
    }
    state = await invoke('get_ui_state');
    assert.equal(state.navigation_tab, 'project-tree');
    await saveShot('project-beta-expanded');

    await click(120, 180);
    state = await waitFor(value => normalize(value.file_manager?.current_directory ?? '') === normalize(alpha));
    assert.equal(state.navigation_tab, 'project-tree', 'Switching the project root must preserve the Project tree tab');
    await delay(400);
    await saveShot('project-alpha-expanded');

    await click(72, 180);
    await delay(350);
    await saveShot('project-alpha-collapsed');
    report.checks.push('adding beta selected and expanded beta tree');
    report.checks.push('clicking alpha switched the expanded tree and navigated to alpha');
    report.checks.push('alpha expander collapsed the child tree');
    report.result = 'passed';
    fs.writeFileSync(path.join(profile, 'project-tree-report.json'), JSON.stringify(report, null, 2));
    console.log(JSON.stringify(report));
  } catch (error) {
    report.error = String(error);
    fs.writeFileSync(path.join(profile, 'project-tree-report.json'), JSON.stringify(report, null, 2));
    throw error;
  }
};
