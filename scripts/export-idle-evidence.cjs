// Export completed same-process MCP runs; never promote incomplete runs to passed.
const fs = require('node:fs');
const path = require('node:path');
const zlib = require('node:zlib');
const crypto = require('node:crypto');
const assert = require('node:assert/strict');
const root = path.resolve(__dirname, '..');
const profile = path.resolve(root, process.argv[2] ?? 'target/mcp-unified-refresh-final');
const output = path.join(root, 'docs/validation-ui/2026-10-15-idle-audit');
fs.mkdirSync(output, { recursive: true });
const summary = { generated: new Date().toISOString(), profile, suites: [], history: [] };
for (const [source, name] of [['idle-report.json', 'unified-matrix'], ['idle-followup-report.json', 'unified-followup']]) {
  const bytes = fs.readFileSync(path.join(profile, source));
  const report = JSON.parse(bytes);
  assert(report.finished, 'incomplete report');
  assert.equal(report.summary.blocked ?? 0, 0, 'blocked report');
  assert.equal(report.summary.sustained ?? 0, 0, 'sustained report');
  assert.equal(report.executable_sha256, crypto.createHash('sha256').update(fs.readFileSync(report.executable)).digest('hex'), 'binary changed since run');
  fs.writeFileSync(path.join(output, name + '.json.gz'), zlib.gzipSync(bytes));
  summary.suites.push({ name, started: report.started, finished: report.finished,
    executable_sha256: report.executable_sha256, summary: report.summary,
    accessibility: report.accessibility, window_interruptions: report.window_interruptions ?? [],
    cases: report.cases.map(c => ({ label: c.label, result: c.result,
      delta: c.delta, final_delta: c.retry_delta ?? c.delta,
      preview_render_delta: c.retry_render_delta ?? c.render_delta,
      retries: c.retries?.length ?? 0,
      image_load: c.label.startsWith('preview-') ? c.after?.view?.preview?.image_load : undefined,
    })) });
}
// Retain original failures/interference as historical observations with their own
// executable hashes; these are deliberately not included in final pass totals.
for (const [source, name] of [
  ['target/idle-audit-before-report.json', 'before'],
  ['target/idle-followup-before-pointer-report.json', 'before-pointer'],
  ['target/idle-followup-input-observed-report.json', 'input-observed'],
  ['target/idle-followup-exif-before-report.json', 'before-exif'],
  ['target/idle-audit-render-paused-report.json', 'render-paused'],
  ['target/mcp-idle-audit-full/idle-report.json', 'before-unification-matrix'],
  ['target/mcp-idle-audit-full/idle-followup-report.json', 'before-unification-followup'],
]) {
  const file = path.join(root, source);
  if (!fs.existsSync(file)) continue;
  const bytes = fs.readFileSync(file), report = JSON.parse(bytes);
  fs.writeFileSync(path.join(output, name + '.json.gz'), zlib.gzipSync(bytes));
  summary.history.push({ name, summary: report.summary, cases: report.cases?.length,
    executable_sha256: report.executable_sha256, finished: report.finished });
}
for (const name of ['settings-ai', 'statistics', 'preview-image', 'preview-broken', 'manual-update-result']) {
  const file = path.join(profile, 'screenshots', name + '.png');
  if (fs.existsSync(file)) fs.copyFileSync(file, path.join(output, name + '.png'));
}
summary.total_cases = summary.suites.reduce((total, suite) => total + suite.cases.length, 0);
fs.writeFileSync(path.join(output, 'summary.json'), JSON.stringify(summary, null, 2) + '\n');
console.log(JSON.stringify({ total_cases: summary.total_cases, suites: summary.suites.map(s => s.summary) }));
