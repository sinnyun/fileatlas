// Real version resources and nonexistent path, no external installer or launch.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, profile, root, output }) => {
  const target = path.resolve(process.env.PHOTAG_EVERYTHING_EXE || '');
  assert(target.startsWith(profile + path.sep), 'version fixture must stay inside isolated profile');
  const report = { cases: [], states: [] };
  const invoke = async (name, args = {}) => {
    const result = payload(await call(name, args));
    assert(!result?.error, JSON.stringify(result));
    return result;
  };
  const wait = async state => {
    let snapshot;
    for (let attempt = 0; attempt < 80; attempt++) {
      snapshot = await invoke('get_ui_state');
      if (snapshot.gallery?.everything?.state === state) return snapshot;
      await delay(100);
    }
    assert.fail(`expected ${state}: ${JSON.stringify(snapshot?.gallery?.everything)}`);
  };
  const capture = async state => {
    const snapshot = await wait(state);
    await delay(200);
    const result = await call('screenshot_window');
    const block = result.content.find(item => item.type === 'image');
    assert(block);
    fs.writeFileSync(path.join(output, `everything-${state}.png`), Buffer.from(block.data, 'base64'));
    report.states.push(snapshot.gallery.everything);
    return snapshot;
  };
  await capture('missing');
  report.cases.push('missing executable shows Install Everything beside search');
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.copyFileSync(path.join(root, 'target/everything-14-version-fixture/Everything.exe'), target);
  await invoke('control_gallery', { action: 'everything_recheck' });
  const old = await capture('incompatible');
  assert.match(old.gallery.everything.version, /^1\.4\./);
  report.cases.push('real Everything 1.4 version resource shows Install compatible Everything');
  fs.copyFileSync(process.env.PHOTAG_EVERYTHING_GOOD_FIXTURE, target);
  await invoke('control_gallery', { action: 'everything_recheck' });
  const ready = await capture('ready');
  assert.match(ready.gallery.everything.version, /^1\.5\./);
  await delay(1200);
  const quiet = await invoke('get_ui_state');
  assert.equal(ready.gallery.snapshot_revision, quiet.gallery.snapshot_revision);
  report.cases.push('rechecking real Everything 1.5 removes install warning; quiet state stays unchanged');
  fs.writeFileSync(path.join(profile, 'everything-status-report.json'), JSON.stringify(report, null, 2));
};
