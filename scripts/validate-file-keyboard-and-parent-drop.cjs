const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, profile, output }) => {
  const root = process.env.FILEATLAS_TEST_LIBRARY;
  assert(root, 'FILEATLAS_TEST_LIBRARY is required');
  const key = value => value.replace(/^\\\\\?\\/, '').toLowerCase();
  const report = { root, profile, checks: [] };
  const invoke = async (tool, args = {}) => payload(await call(tool, args));
  const save = () => fs.writeFileSync(path.join(profile, 'file-keyboard-drop-report.json'), JSON.stringify(report, null, 2));
  async function until(test) {
    for (let i = 0; i < 100; i++) {
      const state = await invoke('get_ui_state');
      report.latest = state;
      if (test(state)) return state;
      await delay(150);
    }
    throw Error('FileManager did not reach the expected state');
  }
  async function action(action, args = {}) {
    await invoke('control_file_manager', { action, ...args });
    await delay(250);
  }
  async function ui(action, value) {
    await invoke('control_ui', { action, value });
    await delay(250);
  }
  async function pointer(action, [x, y]) {
    await ui(action, `${x},${y}`);
  }
  try {
    await invoke('dev-test.scan_fixture');
    for (let i = 0; i < 100; i++) {
      if (!(await invoke('dev-test.status')).running) break;
      await delay(200);
    }
    await action('show_descendants', { enabled: true });
    await until(s => s.file_manager?.grouping_mode === 'subfolder');
    await action('navigate', { value: root });
    let state = await until(s => key(s.file_manager?.current_directory ?? '') === key(root));
    await action('sort', { value: 'name_asc' });
    state = await until(s => {
      const frames = Object.values(s.file_manager?.group_frames ?? {});
      return frames.some(f => key(f.path).endsWith('\\alpha'))
        && frames.some(f => key(f.path).endsWith('\\beta'));
    });
    const frameFor = (directory, current = state) => Object.values(current.file_manager.group_frames)
      .find(f => key(f.path) === key(directory) && f.first);
    const alpha = frameFor(path.join(root, 'alpha'));
    const beta = frameFor(path.join(root, 'beta'));
    assert(alpha && beta && !frameFor(root), 'Expected child frames and an empty parent folder area');
    const source = [alpha.bounds[0] + 32, alpha.bounds[1] + 75];
    await pointer('pointer_down', source);
    await pointer('pointer_up', source);
    state = await until(s => s.file_manager?.selected_media_ids?.length > 0);
    await ui('key_press', 'ctrl-c');
    state = await until(s => s.file_manager?.file_copy_buffer_count === state.file_manager.selected_media_ids.length);
    report.checks.push({ action: 'ctrl+c', selected: state.file_manager.selected_media_ids, copy_buffer_count: state.file_manager.file_copy_buffer_count });
    await pointer('pointer_move', [beta.bounds[0] + beta.bounds[2] / 2, beta.bounds[1] + 18]);
    await ui('key_press', 'ctrl-v');
    const copiedName = 'alpha.png';
    for (let i = 0; i < 80 && !fs.existsSync(path.join(root, 'beta', copiedName)); i++) await delay(150);
    assert(fs.existsSync(path.join(root, 'beta', copiedName)), 'Ctrl+V did not copy the selected file into the folder under the pointer');
    report.checks.push({ action: 'ctrl+v', destination: path.join(root, 'beta'), copied: copiedName });

    state = await until(s => Object.values(s.file_manager?.group_frames ?? {}).some(f => key(f.path).endsWith('\\alpha') && f.first));
    const alphaFrame = frameFor(path.join(root, 'alpha'), state);
    const frames = Object.values(state.file_manager.group_frames);
    assert(alphaFrame && !frameFor(root, state), 'Expected the child source and no direct parent group');
    const dragFrom = [alphaFrame.bounds[0] + 32, alphaFrame.bounds[1] + 75];
    const bottom = Math.max(...frames.map(frame => frame.bounds[1] + frame.bounds[3]));
    const clientHeight = state.window_status.geometry.client_size.height;
    const dropOnMain = [alphaFrame.bounds[0] + alphaFrame.bounds[2] / 2, Math.min(bottom + 40, clientHeight - 60)];
    assert(!frames.some(frame => dropOnMain[1] >= frame.bounds[1]
      && dropOnMain[1] <= frame.bounds[1] + frame.bounds[3]), 'Expected blank parent grid space below the child groups');
    await pointer('pointer_down', dragFrom);
    await pointer('pointer_drag', [dragFrom[0] + 18, dragFrom[1] + 10]);
    await pointer('pointer_drag', dropOnMain);
    const hoverShot = await call('screenshot_window');
    const hoverImage = hoverShot.content.find(item => item.type === 'image');
    assert(hoverImage);
    fs.writeFileSync(path.join(output, 'parent-drop-hover.png'), Buffer.from(hoverImage.data, 'base64'));
    await pointer('pointer_up', dropOnMain);
    for (let i = 0; i < 80 && !fs.existsSync(path.join(root, 'alpha.png')); i++) await delay(150);
    assert(fs.existsSync(path.join(root, 'alpha.png')), 'Child-to-parent drop did not move the file into the main folder');
    assert(!fs.existsSync(path.join(root, 'alpha', 'alpha.png')), 'Moved source still exists in the child folder');
    report.checks.push({ action: 'drag-child-to-parent', source: path.join(root, 'alpha', 'alpha.png'), destination: root });

    state = await until(s => {
      const frames = Object.values(s.file_manager?.group_frames ?? {});
      return frames.some(f => key(f.path) === key(root)) && frames.some(f => key(f.path).endsWith('\\beta'));
    });
    const betaAfterMove = frameFor(path.join(root, 'beta'), state);
    assert(betaAfterMove, 'Expected beta frame after moving a child file into the parent');
    const betaFile = [betaAfterMove.bounds[0] + 210, betaAfterMove.bounds[1] + 75];
    await pointer('pointer_down', betaFile);
    await pointer('pointer_up', betaFile);
    state = await until(s => s.file_manager?.selected_media_ids?.length === 1);
    await ui('key_press', 'ctrl-c');
    await until(s => s.file_manager?.file_copy_buffer_count === 1);
    const visibleFrames = Object.values(state.file_manager.group_frames);
    const blankParent = [betaAfterMove.bounds[0] + 500,
      Math.min(Math.max(...visibleFrames.map(frame => frame.bounds[1] + frame.bounds[3])) + 40,
        state.window_status.geometry.client_size.height - 60)];
    await pointer('pointer_move', blankParent);
    await ui('key_press', 'ctrl-v');
    for (let i = 0; i < 80 && !fs.existsSync(path.join(root, 'beta.jpg')); i++) await delay(150);
    assert(fs.existsSync(path.join(root, 'beta.jpg')), 'Ctrl+V into blank main-folder space did not copy the selected file');
    report.checks.push({ action: 'ctrl+v-parent-blank', destination: root, copied: 'beta.jpg' });
    const screenshot = await call('screenshot_window');
    const image = screenshot.content.find(item => item.type === 'image');
    assert(image);
    fs.writeFileSync(path.join(output, 'file-keyboard-parent-drop.png'), Buffer.from(image.data, 'base64'));
    report.result = 'passed';
    save();
    console.log(JSON.stringify({ result: report.result, checks: report.checks }));
  } catch (error) {
    report.error = String(error);
    save();
    throw error;
  }
};
