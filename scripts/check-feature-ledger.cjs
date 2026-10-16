// Feature ledger guard: stage statuses stay inside the agreed vocabulary and
// any claimed progress row must cite evidence files that exist.
// Rules and tokens: docs/engineering-governance.md (L1).
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '..');
const ledger = path.join(root, 'docs', 'functional-development-plan.md');
const allowedPrefixes = ['未开始', '进行中', '已完成', '随开发执行'];

const failures = [];
if (!fs.existsSync(ledger)) {
  console.error(`missing ledger: docs/functional-development-plan.md`);
  process.exitCode = 1;
} else {
  const text = fs.readFileSync(ledger, 'utf8');
  const docDir = path.dirname(ledger);

  // Every relative link target must resolve.
  for (const m of text.matchAll(/\]\(([^)#\s]+)(#[^)]*)?\)/g)) {
    const url = m[1];
    if (/^[a-z]+:/.test(url)) continue;
    const target = path.resolve(docDir, url.replace(/^\.\//, ''));
    if (!fs.existsSync(target)) failures.push(`L1 broken link: (${url}) from docs/functional-development-plan.md`);
  }

  // Stage table rows: F0..F3 with bounded status vocabulary and evidence.
  const rows = text.split(/\r?\n/).filter((line) => /^\|\s*F\d/.test(line));
  if (rows.length < 4) failures.push(`L1 stage table lost rows (found ${rows.length}, expect >= 4)`);
  for (const row of rows) {
    const cells = row.split('|').map((c) => c.trim());
    const stage = cells[1];
    const status = cells[3] || '';
    if (!allowedPrefixes.some((p) => status.startsWith(p)))
      failures.push(`L1 ${stage}: status outside vocabulary: "${status.slice(0, 40)}"`);
    if (!/未开始/.test(status) && !/\]\([^)]+\)/.test(row))
      failures.push(`L1 ${stage}: claimed progress without an evidence link`);
  }

  if (failures.length) {
    console.error(failures.join('\n'));
    process.exitCode = 1;
  } else {
    console.log(`feature ledger: passed (${rows.length} stage rows, links resolve)`);
  }
}
