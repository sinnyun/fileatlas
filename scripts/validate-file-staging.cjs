const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

module.exports = async ({ call, payload, profile, output, delay, request }) => {
  const invoke = async (name, args = {}) => payload(await call(name, args));
  const sourceA = path.join(profile, 'staging-fixtures', 'source-a');
  const sourceB = path.join(profile, 'staging-fixtures', 'source-b');
  const copyTarget = path.join(profile, 'staging-fixtures', 'copy-target');
  const moveTarget = path.join(profile, 'staging-fixtures', 'move-target');
  fs.mkdirSync(sourceA, { recursive: true }); fs.mkdirSync(sourceB, { recursive: true });
  fs.mkdirSync(copyTarget, { recursive: true }); fs.mkdirSync(moveTarget, { recursive: true });
  const first = path.join(sourceA, 'first.txt'); const second = path.join(sourceB, 'second.txt');
  fs.writeFileSync(first, 'first staging fixture'); fs.writeFileSync(second, 'second staging fixture');

  const tools = (await request('tools/list', {})).tools.map(item => item.name);
  for (const name of ['file-staging.list','file-staging.add','file-staging.remove','file-staging.clear','file-staging.refresh_status','file-staging.remove_missing','file-staging.copy','file-staging.move']) assert(tools.includes(name), `Missing MCP tool ${name}`);
  const added = await invoke('file-staging.add', { paths: [first, second, first] });
  assert.equal(added.items.length, 2, 'duplicate canonical paths collapse to one basket entry');
  assert.deepEqual((await invoke('file-staging.list')).items.map(item => path.basename(item.path)).sort(), ['first.txt','second.txt']);
  await invoke('get_ui_state');
  await invoke('control_ui', { action: 'panel_open', value: 'file-staging/panel' }); await delay(1500);
  const shot = await call('screenshot_window');
  const image = shot.content.find(item => item.type === 'image'); assert(image, 'Expected panel screenshot');
  fs.writeFileSync(path.join(output, 'staging-basket.png'), Buffer.from(image.data, 'base64'));

  const copied = await invoke('file-staging.copy', { destination_folder: copyTarget, confirm: true });
  assert(copied.items.every(item => item.completed), 'two distinct files copy successfully');
  assert.equal(fs.readFileSync(path.join(copyTarget, 'first.txt'), 'utf8'), 'first staging fixture');
  assert.equal(fs.readFileSync(path.join(copyTarget, 'second.txt'), 'utf8'), 'second staging fixture');
  assert.equal((await invoke('file-staging.list')).items.length, 0, 'successful paths leave the basket');

  await invoke('file-staging.add', { paths: [first, second] });
  fs.writeFileSync(path.join(copyTarget, 'first.txt'), 'destination must survive');
  const conflict = await invoke('file-staging.copy', { destination_folder: copyTarget, confirm: true });
  assert(conflict.items.some(item => !item.completed && item.error), 'collision reports a per-path failure');
  assert.equal(fs.readFileSync(path.join(copyTarget, 'first.txt'), 'utf8'), 'destination must survive', 'collision never overwrites destination');
  assert((await invoke('file-staging.list')).items.some(item => path.basename(item.path) === 'first.txt'), 'failed path remains retryable');
  await invoke('file-staging.clear');

  const movable = path.join(sourceA, 'movable.txt'); fs.writeFileSync(movable, 'move staging fixture');
  await invoke('file-staging.add', { paths: [movable] });
  const moved = await invoke('file-staging.move', { destination_folder: moveTarget, confirm: true });
  assert.equal(moved.items[0].completed, true); assert.equal(fs.existsSync(movable), false);
  assert.equal(fs.readFileSync(path.join(moveTarget, 'movable.txt'), 'utf8'), 'move staging fixture');

  const missing = path.join(sourceB, 'missing-after-stage.txt'); fs.writeFileSync(missing, 'temporary');
  await invoke('file-staging.add', { paths: [missing] }); fs.unlinkSync(missing);
  const refreshed = await invoke('file-staging.refresh_status'); assert.equal(refreshed.items[0].available, false);
  assert.equal((await invoke('file-staging.remove_missing')).items.length, 0);
  await invoke('file-staging.add', { paths: [second] });
  fs.writeFileSync(path.join(profile, 'staging-report.json'), JSON.stringify({cases:['multi-directory add and canonical de-duplication','same-process native panel screenshot','batch copy and successful-item removal','no-overwrite collision with retry retention','batch move and source removal','missing-path refresh and pruning'], tools:tools.filter(name=>name.startsWith('file-staging.')), final_items:(await invoke('file-staging.list')).items}, null, 2));
};
