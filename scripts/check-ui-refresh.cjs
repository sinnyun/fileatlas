// Architecture guard: all GPUI timer loops use the shared notification owner.
// This supplements actual same-process MCP tests; it cannot prove idle behavior.
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
const failures = [];
function visit(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const file = path.join(directory, entry.name);
    if (entry.isDirectory()) visit(file);
    else if (entry.name.endsWith('.rs')) {
      const relative = path.relative(root, file).replaceAll('\\', '/');
      if (relative === 'crates/fileatlas-ui-bridge/src/refresh.rs') continue;
      const lines = fs.readFileSync(file, 'utf8').split(/\r?\n/);
      lines.forEach((line, index) => {
        if (/\.timer\s*\(/.test(line) && !line.trimStart().startsWith('//'))
          failures.push(`${relative}:${index + 1}: use fileatlas_ui_bridge::refresh::poll`);
      });
    }
  }
}
visit(path.join(root, 'crates'));
if (failures.length) {
  console.error(failures.join('\n'));
  process.exitCode = 1;
} else console.log('UI timer ownership: passed');
