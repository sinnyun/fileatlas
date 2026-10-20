const assert = require('node:assert/strict');
module.exports = async ({ call, payload, profile }) => {
  const result = payload(await call('file-staging.list'));
  assert.equal(result.items.length, 1);
  assert(result.items[0].path.endsWith('source-b\\second.txt'));
  assert.equal(result.items[0].available, true);
  const empty = payload(await call('file-staging.clear'));
  assert.equal(empty.items.length, 0);
  require('node:fs').writeFileSync(require('node:path').join(profile, 'staging-restart-report.json'), JSON.stringify({ restored: result.items[0].path, cleared: true }, null, 2));
};
