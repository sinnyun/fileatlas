// Reopen the same MCP profile and exercise the recovered pending conflict.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, profile }) => {
  const manifest = JSON.parse(fs.readFileSync(path.join(profile, 'pending-restart-manifest.json'), 'utf8'));
  const invoke = async (tool, args = {}) => payload(await call(tool, args));
  async function until(probe, label) {
    let result;
    for (let i = 0; i < 80; i++) {
      result = await probe();
      if (result) return result;
      await delay(250);
    }
    throw Error(`Timed out waiting for ${label}: ${JSON.stringify(result)}`);
  }
  await invoke('control_file_manager', { action: 'navigate', value: manifest.fixture });
  await until(async () => {
    const state = await invoke('get_ui_state');
    return state?.file_manager?.matched_count > 0;
  }, 'reopened FileManager library');
  const graph = await invoke('list_file_branches', { media_id: manifest.media_id });
  const pending = graph?.graph?.pendingMerge;
  assert(pending, `pending merge did not survive process restart: ${JSON.stringify(graph)}`);
  assert.equal(pending.sourceBranch, manifest.pending_merge.sourceBranch);
  assert.equal(pending.targetBranch, manifest.pending_merge.targetBranch);
  assert.deepEqual(pending.conflictFiles, manifest.pending_merge.conflictFiles);
  assert.deepEqual(fs.readFileSync(manifest.file), fs.readFileSync(manifest.expected_source));

  const aborted = await invoke('manage_file_branch', { media_id: manifest.media_id, action: 'abort', confirm: true });
  assert.equal(aborted?.ok, true, JSON.stringify(aborted));
  const cleared = await invoke('list_file_branches', { media_id: manifest.media_id });
  assert.equal(cleared?.graph?.pendingMerge, null, JSON.stringify(cleared?.graph));
  assert.deepEqual(fs.readFileSync(manifest.file), fs.readFileSync(manifest.expected_source));
  const report = {
    restarted: true,
    media_id: manifest.media_id,
    recovered_pending_merge: pending,
    resolution_after_restart: aborted,
    cases: ['pending conflict source/target/path metadata survived a complete FileAtlas MCP child restart', 'abort after restart cleared merge state and preserved original source bytes'],
  };
  fs.writeFileSync(path.join(profile, 'pending-restart-report.json'), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report));
};
