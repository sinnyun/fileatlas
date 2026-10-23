// Locate plugin-local numeric spacing overrides for a batch UI normalization pass.
// Usage: node scripts/audit-ui-geometry-source.cjs [crates]
const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');

const root = process.cwd();
const scope = process.argv.slice(2).find(argument => !argument.startsWith('--')) || 'crates';
const files = execFileSync('rg', ['--files', scope, '-g', '*.rs'], { cwd: root, encoding: 'utf8' })
  .trim().split(/\r?\n/).filter(Boolean);
const patterns = [
  { category: 'spacing', regex: /\.(gap|p|px|py|pt|pb|pl|pr|m|mx|my|mt|mb|ml|mr)\(px\(\s*(-?\d+(?:\.\d*)?)\s*\)\)/g },
  { category: 'dimension', regex: /\.(w|h|min_w|min_h|max_w|max_h|size)\(px\(\s*(-?\d+(?:\.\d*)?)\s*\)\)/g },
  { category: 'typography', regex: /\.(text_size|line_height)\(px\(\s*(-?\d+(?:\.\d*)?)\s*\)\)/g },
];
const byMethod = new Map();
const byValue = new Map();
const byCategory = new Map();
const byFile = new Map();
const rows = [];

for (const file of files) {
  const source = fs.readFileSync(path.join(root, file), 'utf8');
  const lines = source.split(/\r?\n/);
  lines.forEach((text, index) => {
    for (const { category, regex } of patterns) for (const match of text.matchAll(regex)) {
      const [, method, value] = match;
      const normalized = `${method}=${value}`;
      byCategory.set(category, (byCategory.get(category) || 0) + 1);
      byMethod.set(method, (byMethod.get(method) || 0) + 1);
      byValue.set(normalized, (byValue.get(normalized) || 0) + 1);
      byFile.set(file, (byFile.get(file) || 0) + 1);
      rows.push({ file, line: index + 1, method, value, text: text.trim(), lines });
    }
    const constructors = text.match(/Button::new\(/g)?.length || 0;
    if (constructors) {
      byCategory.set('direct Button::new', (byCategory.get('direct Button::new') || 0) + constructors);
      byFile.set(file, (byFile.get(file) || 0) + constructors);
      rows.push({ file, line: index + 1, method: 'Button::new', value: String(constructors), text: text.trim(), lines });
    }
  });
}

console.log(`files=${files.length} direct_geometry_and_button_sites=${rows.length}`);
console.log('\nBY CATEGORY');
for (const [category, count] of [...byCategory].sort(([a], [b]) => a.localeCompare(b))) {
  console.log(`  ${category}: ${count}`);
}
console.log('\nBY METHOD');
for (const [method, count] of [...byMethod].sort(([a], [b]) => a.localeCompare(b))) {
  console.log(`  ${method}: ${count}`);
}
console.log('\nBY METHOD AND VALUE');
for (const [key, count] of [...byValue].sort(([a], [b]) => a.localeCompare(b, undefined, { numeric: true }))) {
  console.log(`  ${key}: ${count}`);
}
console.log('\nBY FILE');
for (const [file, count] of [...byFile].sort(([, a], [, b]) => b - a || 0)) {
  console.log(`  ${file}: ${count}`);
}
console.log('\nLOCATIONS');
for (const row of rows) {
  console.log(`  ${row.file}:${row.line}: .${row.method}(px(${row.value})) :: ${row.text}`);
  if (process.argv.includes('--context') && row.method === 'gap' && ![2, 4, 6, 8, 10, 12, 16, 24].includes(Number(row.value))) {
    const start = Math.max(0, row.line - 3);
    const end = Math.min(row.lines.length, row.line + 2);
    for (let index = start; index < end; index++) {
      console.log(`    ${index + 1}${index + 1 === row.line ? '>' : ' '} ${row.lines[index].trim()}`);
    }
  }
}
