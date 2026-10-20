// Persist a real merge conflict, then let fileatlas-mcp-debug stop this process.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');

module.exports = async ({ call, payload, delay, profile }) => {
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
  const fixture = path.join(profile, 'lore-restart-fixture');
  fs.mkdirSync(fixture, { recursive: true });
  const file = path.join(fixture, 'pending-merge.png');
  const images = {};
  for (const color of ['green', 'blue', 'red']) {
    images[color] = path.join(profile, `pending-${color}.png`);
    execFileSync('ffmpeg', ['-y', '-f', 'lavfi', '-i', `color=c=${color}:s=160x90`, '-frames:v', '1', images[color]], { stdio: 'ignore', windowsHide: true });
  }
  fs.copyFileSync(images.green, file);
  const folder = await invoke('add_folder', { path: fixture, confirm: true });
  assert.equal(folder?.ok, true, JSON.stringify(folder));
  await invoke('control_file_manager', { action: 'navigate', value: fixture });
  await until(async () => {
    const state = await invoke('get_ui_state');
    return state?.file_manager?.matched_count > 0;
  }, 'fixture scan');
  const found = await invoke('search_media', { limit: 100, sort: 'name_asc' });
  const media = found?.items?.find(item => item.filename === 'pending-merge.png');
  assert(media, `fixture media is missing: ${JSON.stringify(found)}`);
  await invoke('create_file_version', { media_id: media.id, title: 'Restart base', confirm: true });
  const graph = await invoke('list_file_branches', { media_id: media.id });
  const main = graph?.graph?.branches?.find(branch => branch.isCurrent)?.name;
  assert(main, `initial branch missing: ${JSON.stringify(graph)}`);
  await invoke('manage_file_branch', { media_id: media.id, action: 'create', name: 'restart/feature', confirm: true });
  await invoke('manage_file_branch', { media_id: media.id, action: 'switch', name: 'restart/feature', confirm: true });
  fs.copyFileSync(images.blue, file);
  await invoke('create_file_version', { media_id: media.id, title: 'Restart feature edit', confirm: true });
  await invoke('manage_file_branch', { media_id: media.id, action: 'switch', name: main, confirm: true });
  fs.copyFileSync(images.red, file);
  await invoke('create_file_version', { media_id: media.id, title: 'Restart main edit', confirm: true });
  const merge = await invoke('manage_file_branch', { media_id: media.id, action: 'merge', source_branch: 'restart/feature', confirm: true });
  assert.equal(merge?.result?.hasConflicts, true, JSON.stringify(merge));
  const pending = await invoke('list_file_branches', { media_id: media.id });
  assert(pending?.graph?.pendingMerge, `merge metadata was not persisted: ${JSON.stringify(pending)}`);
  assert.deepEqual(fs.readFileSync(file), fs.readFileSync(images.red), 'conflict changed source bytes before restart');
  const manifest = {
    media_id: media.id,
    file,
    fixture,
    base_branch: main,
    incoming_branch: 'restart/feature',
    expected_source: images.red,
    pending_merge: pending.graph.pendingMerge,
  };
  fs.writeFileSync(path.join(profile, 'pending-restart-manifest.json'), JSON.stringify(manifest, null, 2));
  console.log(JSON.stringify({ prepared: true, manifest }));
};
