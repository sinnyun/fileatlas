// End-to-end Lore version/branch checks through this PhoTag child's MCP server.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');

module.exports = async ({ call, payload, delay, request, profile }) => {
  const report = { cases: [], calls: [] };
  const record = (name, result) => {
    report.calls.push({ name, result });
    fs.writeFileSync(path.join(profile, 'lore-version-report.json'), JSON.stringify(report, null, 2));
    return result;
  };
  const invoke = async (tool, args = {}) => record(tool, payload(await call(tool, args)));
  const until = async (probe, label) => {
    let result;
    for (let i = 0; i < 80; i++) {
      result = await probe();
      if (result) return result;
      await delay(250);
    }
    throw Error(`Timed out waiting for ${label}: ${JSON.stringify(result)}`);
  };
  const normalizePath = value => path.resolve(String(value).replace(/^\\\\\?\\/, '')).toLowerCase();

  const listed = (await request('tools/list', {})).tools.map(tool => tool.name);
  for (const name of ['list_file_versions', 'create_file_version', 'activate_file_version', 'list_file_branches', 'manage_file_branch']) {
    assert(listed.includes(name), `MCP tools/list is missing ${name}`);
  }
  report.cases.push('all Lore version and branch tools are registered');
  await assert.rejects(
    call('manage_file_branch', { media_id: -1, action: 'create', name: 'must-not-create', confirm: false }),
    /confirm=true/,
  );
  report.cases.push('branch mutation rejects requests without explicit confirmation');

  const source = path.join(profile, 'lore-version-fixture');
  fs.mkdirSync(source, { recursive: true });
  const file = path.join(source, 'history-fixture.png');
  const baseImage = path.join(profile, 'base.png');
  const featureImage = path.join(profile, 'feature.png');
  const mainImage = path.join(profile, 'main.png');
  for (const [color, output] of [['green', baseImage], ['blue', featureImage], ['red', mainImage]]) {
    execFileSync('ffmpeg', ['-y', '-f', 'lavfi', '-i', `color=c=${color}:s=160x90`, '-frames:v', '1', output], { stdio: 'ignore', windowsHide: true });
  }
  fs.copyFileSync(baseImage, file);
  const add = await invoke('add_folder', { path: source, confirm: true });
  assert(add && !add.error, `fixture folder registration failed: ${JSON.stringify(add)}`);
  await invoke('control_gallery', { action: 'navigate', value: source });
  await until(async () => {
    const state = await invoke('get_ui_state');
    return state?.gallery?.matched_count > 0 ? state : null;
  }, 'Gallery indexing and folder selection');

  const media = await until(async () => {
    const found = await invoke('search_media', { limit: 100, sort: 'name_asc' });
    return found?.items?.find(item => normalizePath(item.full_path) === normalizePath(file));
  }, 'indexed Lore fixture');
  const mediaId = media.id;

  const base = await invoke('create_file_version', { media_id: mediaId, title: 'Base', confirm: true });
  assert.equal(base?.ok, true, JSON.stringify(base));
  const history = await invoke('list_file_versions', { media_id: mediaId });
  const baseVersion = history?.versions?.find(version => version.title === 'Base');
  assert(baseVersion?.versionId, `Base revision missing: ${JSON.stringify(history)}`);

  const initial = await invoke('list_file_branches', { media_id: mediaId });
  const main = initial?.graph?.branches?.find(branch => branch.isCurrent)?.name;
  assert(main, `Current Lore branch missing: ${JSON.stringify(initial)}`);
  await invoke('manage_file_branch', { media_id: mediaId, action: 'create', name: 'feature/mcp', confirm: true });
  const created = await invoke('list_file_branches', { media_id: mediaId });
  assert(created.graph.branches.some(branch => branch.name === 'feature/mcp'));
  await invoke('manage_file_branch', { media_id: mediaId, action: 'switch', name: 'feature/mcp', confirm: true });
  fs.copyFileSync(featureImage, file);
  await invoke('create_file_version', { media_id: mediaId, title: 'Feature edit', confirm: true });
  await invoke('manage_file_branch', { media_id: mediaId, action: 'switch', name: main, confirm: true });
  assert.deepEqual(fs.readFileSync(file), fs.readFileSync(baseImage));
  fs.copyFileSync(mainImage, file);
  await invoke('create_file_version', { media_id: mediaId, title: 'Main edit', confirm: true });

  const conflict = await invoke('manage_file_branch', { media_id: mediaId, action: 'merge', source_branch: 'feature/mcp', confirm: true });
  assert.equal(conflict?.result?.hasConflicts, true, JSON.stringify(conflict));
  const pending = await invoke('list_file_branches', { media_id: mediaId });
  assert(pending.graph.pendingMerge, `pending merge was not exposed: ${JSON.stringify(pending)}`);
  assert.deepEqual(fs.readFileSync(file), fs.readFileSync(mainImage), 'conflicted merge modified source bytes');
  const aborted = await invoke('manage_file_branch', { media_id: mediaId, action: 'abort', confirm: true });
  assert.equal(aborted?.ok, true, JSON.stringify(aborted));
  const cleared = await invoke('list_file_branches', { media_id: mediaId });
  assert.equal(cleared.graph.pendingMerge, null);
  assert.deepEqual(fs.readFileSync(file), fs.readFileSync(mainImage));
  report.cases.push('MCP merge conflict is persisted, visible, and abortable without changing source bytes');

  const conflictAgain = await invoke('manage_file_branch', { media_id: mediaId, action: 'merge', source_branch: 'feature/mcp', confirm: true });
  assert.equal(conflictAgain?.result?.hasConflicts, true, JSON.stringify(conflictAgain));
  const resolved = await invoke('manage_file_branch', { media_id: mediaId, action: 'resolve', strategy: 'theirs', confirm: true });
  assert.equal(resolved?.result?.completed, true, JSON.stringify(resolved));
  assert.deepEqual(fs.readFileSync(file), fs.readFileSync(featureImage));

  await invoke('manage_file_branch', { media_id: mediaId, action: 'archive', name: 'feature/mcp', confirm: true });
  const archived = await invoke('list_file_branches', { media_id: mediaId, include_archived: true });
  assert(archived.graph.branches.some(branch => branch.name === 'feature/mcp' && branch.isArchived));

  await invoke('activate_file_version', { media_id: mediaId, version_id: baseVersion.versionId, confirm: true });
  assert.deepEqual(fs.readFileSync(file), fs.readFileSync(baseImage));
  report.cases.push('MCP create/list/restore revisions, create/switch/archive branches, and resolve incoming merge conflict');
  fs.writeFileSync(path.join(profile, 'lore-version-report.json'), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report));
};
