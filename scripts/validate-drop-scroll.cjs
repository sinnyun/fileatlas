// Drop in the middle of a multi-page FileManager and assert the viewport never resets.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, profile, output, exe, processId }) => {
  const root = process.env.FILEATLAS_TEST_LIBRARY;
  const key = value => value.replace(/^\\\\\?\\/, '').toLowerCase();
  const invoke = async (tool, args = {}) => payload(await call(tool, args));
  const report = { root, profile, exe, processId, samples: [] };
  const save = () => fs.writeFileSync(path.join(profile, 'drop-scroll-report.json'), JSON.stringify(report, null, 2));
  const offset = state => state.program_workspace?.current_snapshot?.columns?.[0]?.scroll_offset;
  const frames = state => Object.values(state.file_manager?.group_frames ?? {});
  const frameFor = (state, folder) => frames(state).find(frame => key(frame.path).endsWith(`\\${folder}`) && frame.first);
  async function until(test) {
    for (let i = 0; i < 150; i++) {
      const state = await invoke('get_ui_state');
      report.latest = state;
      if (test(state)) return state;
      await delay(150);
    }
    throw Error('FileManager did not reach the requested test state');
  }
  async function action(action, args = {}) {
    await invoke('control_file_manager', { action, ...args });
    await delay(300);
  }
  async function pointer(action, [x, y]) {
    for (let attempt = 0; ; attempt++) {
      try {
        await invoke('control_ui', { action, value: `${x},${y}` });
        break;
      } catch (error) {
        if (attempt >= 30 || !String(error).includes('pointer request is still pending')) throw error;
        await delay(100);
      }
    }
    await delay(300);
  }
  async function shot(name) {
    const result = await call('screenshot_window');
    const image = result.content.find(item => item.type === 'image');
    assert(image);
    fs.writeFileSync(path.join(output, `${name}.png`), Buffer.from(image.data, 'base64'));
  }
  try {
    await invoke('dev-test.scan_fixture');
    for (let i = 0; i < 150; i++) {
      if (!(await invoke('dev-test.status')).running) break;
      await delay(200);
    }
    await action('show_descendants', { enabled: true });
    await until(state => state.file_manager?.grouping_mode === 'subfolder');
    await action('navigate', { value: root });
    await until(state => key(state.file_manager?.current_directory ?? '') === key(root) && state.file_manager.matched_count === 760);
    await action('sort', { value: 'name_asc' });
    await until(state => state.file_manager?.sort === 'NameAsc');
    await action('debug_internal_file_drag', { enabled: true });
    await until(state => state.file_manager?.debug_internal_file_drag);
    await action('scroll', { value: '360' });
    let state = await until(state => offset(state) > 5000 && frameFor(state, '10-source') && frameFor(state, '20-target') && state.file_manager.window_offset > 200);
    const sourcePath = path.join(root, '10-source', 'source-00.jpg');
    const destinationPath = path.join(root, '20-target', 'source-00.jpg');
    const sourceFrame = frameFor(state, '10-source');
    const targetFrame = frameFor(state, '20-target');
    const source = [sourceFrame.bounds[0] + 30, sourceFrame.bounds[1] + 75];
    const target = [targetFrame.bounds[0] + 30, targetFrame.bounds[1] + 75];
    assert(source[1] > 100 && target[1] < 1140, 'fixture drop points must be visible');
    // Keep an unaffected neighboring file selected while dragging source-00.
    const selectionPoint = [source[0] + 176, source[1]];
    await pointer('pointer_down', selectionPoint);
    await pointer('pointer_up', selectionPoint);
    state = await until(state => state.file_manager?.selected_media_ids?.length === 1 && frameFor(state, '10-source') && frameFor(state, '20-target'));
    const media = { id: state.file_manager.selected_media_ids[0] };
    report.before = state; report.source = source; report.target = target; save();
    await shot('before-drop');
    await pointer('pointer_down', source);
    await pointer('pointer_drag', [source[0] + 20, source[1] + 8]);
    await pointer('pointer_drag', target);
    await pointer('pointer_up', target);
    for (let i = 0; i < 35; i++) {
      state = await invoke('get_ui_state');
      report.samples.push({ scroll: offset(state), window_offset: state.file_manager.window_offset,
        matched_count: state.file_manager.matched_count,
        selected: state.file_manager.selected_media_ids, revision: state.file_manager.snapshot_revision });
      await delay(100);
    }
    report.after = state; save();
    await shot('after-drop');
    assert(fs.existsSync(destinationPath) && !fs.existsSync(sourcePath), 'drop did not move the real file');
    assert(state.file_manager.groups.some(([group, count]) => key(group).endsWith('\\20-target') && count === 21));
    assert.equal(state.file_manager.matched_count, 760);
    assert(Math.abs(offset(state) - offset(report.before)) < 1, `viewport jumped: ${offset(report.before)} -> ${offset(state)}`);
    assert(report.samples.every(sample => sample.scroll > 5000), 'viewport reset during asynchronous refresh');
    assert(report.samples.every(sample => sample.matched_count >= 759), 'background rescan replaced the list with a transient partial inventory');
    assert(state.file_manager.selected_media_ids.includes(media.id), 'selection was lost after moving within the current scope');
    assert(state.file_manager.window_offset > 200, 'rich window reset to the first page');
    await action('debug_internal_file_drag', { enabled: false });
    const settled = await invoke('get_ui_state'); await delay(1000);
    const quiet = await invoke('get_ui_state');
    assert.equal(quiet.file_manager.snapshot_revision, settled.file_manager.snapshot_revision);
    report.quiet = [settled.file_manager.snapshot_revision, quiet.file_manager.snapshot_revision];
    report.result = 'passed'; save();
    console.log(JSON.stringify({ result: 'passed', before: offset(report.before), after: offset(report.after), samples: report.samples.length, quiet: report.quiet }));
  } catch (error) {
    report.error = String(error);
    save();
    await shot('failure').catch(() => {});
    throw error;
  }
};
