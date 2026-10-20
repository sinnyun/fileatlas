// Same-process GPUI pointer drops through the real card and group-frame handlers.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, profile, output }) => {
  const root = process.env.FILEATLAS_TEST_LIBRARY;
  const key = value => value.replace(/^\\\\\?\\/, '').toLowerCase();
  const report = { root, profile, cases: [] };
  const save = () => fs.writeFileSync(path.join(profile, 'group-frame-report.json'), JSON.stringify(report, null, 2));
  const invoke = async (tool, args = {}) => payload(await call(tool, args));
  async function stateUntil(test) {
    for (let i = 0; i < 100; i++) {
      const state = await invoke('get_ui_state');
      report.latest = state;
      if (test(state)) return state;
      await delay(150);
    }
    throw Error('Live FileManager did not reach the expected state');
  }
  async function action(action, args = {}) {
    await invoke('control_file_manager', { action, ...args });
    await delay(250);
  }
  async function pointer(action, [x, y]) {
    for (let i = 0; ; i++) {
      try {
        await invoke('control_ui', { action, value: `${x},${y}` });
        break;
      } catch (error) {
        if (i >= 30 || !String(error).includes('pointer request is still pending')) throw error;
        await delay(150);
      }
    }
    await delay(450);
  }
  async function shot(name) {
    const result = await call('screenshot_window');
    const image = result.content.find(item => item.type === 'image');
    assert(image);
    fs.writeFileSync(path.join(output, `${name}.png`), Buffer.from(image.data, 'base64'));
  }
  try {
    await invoke('dev-test.scan_fixture');
    for (let i = 0; i < 100; i++) {
      const status = await invoke('dev-test.status');
      if (!status.running) break;
      await delay(200);
    }
    await action('show_descendants', { enabled: true });
    await stateUntil(state => state.file_manager?.grouping_mode === 'subfolder');
    await action('navigate', { value: root });
    await stateUntil(state => key(state.file_manager?.current_directory ?? '') === key(root));
    await action('sort', { value: 'name_asc' });
    await action('debug_internal_file_drag', { enabled: true });
    await stateUntil(state => state.file_manager?.debug_internal_file_drag === true);
    let state = await stateUntil(state => state.file_manager?.matched_count === 30
      && Object.values(state.file_manager?.group_frames ?? {}).filter(frame => key(frame.path).endsWith('\\beta')).length >= 2);
    report.before = state.file_manager;
    await shot('group-frames-before');
    for (const [index, target] of ['header', 'content-blank', 'continuation', 'card', 'collapsed'].entries()) {
      const sourceName = `0${index + 1}-${target}.jpg`;
      if (target === 'collapsed') {
        const beta = Object.values(state.file_manager.group_frames).find(frame => key(frame.path).endsWith('\\beta'));
        await action('toggle_group', { value: beta.path });
        state = await stateUntil(s => s.file_manager?.collapsed_groups?.includes(beta.path)
          && Object.values(s.file_manager?.group_frames ?? {}).some(frame => frame.path === beta.path && frame.first && frame.bounds[3] < 100));
      } else {
        state = await stateUntil(s => {
          const frames = Object.values(s.file_manager?.group_frames ?? {});
          return frames.some(frame => key(frame.path).endsWith('\\alpha') && frame.first)
            && frames.some(frame => key(frame.path).endsWith('\\beta') && frame.first === (target !== 'continuation'));
        });
      }
      const frames = Object.values(state.file_manager.group_frames);
      const alpha = frames.find(frame => key(frame.path).endsWith('\\alpha') && frame.first);
      const beta = frames.find(frame => key(frame.path).endsWith('\\beta') && frame.first === (target !== 'continuation'));
      assert(alpha && beta, `Missing visible source/target frame for ${target}`);
      const [ax, ay] = alpha.bounds;
      const [bx, by, bw, bh] = beta.bounds;
      const source = [ax + 30, ay + 75];
      const dest = target === 'header' || target === 'collapsed' ? [bx + bw / 2, by + 20]
        : target === 'card' ? [bx + 30, by + 75]
        : [bx + bw - 30, by + bh - 6];
      report.cases.push({ sourceName, target, source, dest, alpha, beta }); save();
      await pointer('pointer_down', source);
      await pointer('pointer_drag', [source[0] + 18, source[1] + 10]);
      await pointer('pointer_drag', dest);
      if (target === 'content-blank') await shot('group-frame-content-hover');
      await pointer('pointer_up', dest);
      for (let i = 0; i < 60 && !fs.existsSync(path.join(root, 'beta', sourceName)); i++) await delay(150);
      assert(fs.existsSync(path.join(root, 'beta', sourceName)), `Drop onto ${target} did not move ${sourceName}`);
      assert(!fs.existsSync(path.join(root, 'alpha', sourceName)));
      report.cases.at(-1).passed = true; save();
      state = await stateUntil(s => s.file_manager?.matched_count === 30
        && s.file_manager?.groups?.some(([group, count]) => key(group).endsWith('\\beta') && count === 26 + index));
    }
    await shot('group-frames-after');
    await action('debug_internal_file_drag', { enabled: false });
    await stateUntil(s => s.file_manager?.debug_internal_file_drag === false);
    const quietBefore = await invoke('get_ui_state');
    await delay(1000);
    const quietAfter = await invoke('get_ui_state');
    report.quiet = [quietBefore.file_manager.snapshot_revision, quietAfter.file_manager.snapshot_revision];
    assert.equal(...report.quiet, 'settled group frames must not reload the FileManager');
    report.after = state.file_manager;
    report.result = 'passed'; save();
    console.log(JSON.stringify({ result: 'passed', cases: report.cases.length }));
  } catch (error) {
    report.error = String(error); save();
    await shot('failure').catch(() => {});
    throw error;
  }
};
