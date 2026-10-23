// Migrate literal gaps, paddings, and margins to the only three shared spacing tiers.
// Usage: node scripts/normalize-ui-gap-tokens.cjs [crates]
const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const root = process.cwd();
const scope = process.argv[2] || 'crates';
const files = execFileSync('rg', ['--files', scope, '-g', '*.rs'], { cwd: root, encoding: 'utf8' })
  .trim().split(/\r?\n/).filter(Boolean);
const spacing = /\.(gap|p|px|py|pt|pb|pl|pr|m|mx|my|mt|mb|ml|mr)\(px\(\s*(-?\d+(?:\.\d*)?)\s*\)\)/g;
const utilitySpacing = /\.(gap|p|px|py|pt|pb|pl|pr|m|mx|my|mt|mb|ml|mr)_([1-4])\s*\(\s*\)/g;
let replacements = 0;
const counts = new Map();
for (const file of files) {
  const fullPath = path.join(root, file);
  const source = fs.readFileSync(fullPath, 'utf8');
  const normalizedFile = file.replaceAll('\\', '/');
  const theme = normalizedFile.startsWith('crates/fileatlas-ui-bridge/') ? 'crate::theme' : 'fileatlas_ui_bridge::theme';
  const replaceTier = (method, value) => {
    const tier = value <= 1 ? 'SPACE_COMPACT' : value <= 2 ? 'SPACE_STANDARD' : 'SPACE_EXPANDED';
    replacements += 1;
    counts.set(tier, (counts.get(tier) || 0) + 1);
    const helper = tier === 'SPACE_COMPACT' ? 'space_compact' : tier === 'SPACE_STANDARD' ? 'space_standard' : 'space_expanded';
    return `.${method}(px(${theme}::${helper}()))`;
  };
  const replaceLines = input => input.split(/\r?\n/).map(line => {
    if (/^\s*\/\//.test(line)) return line;
    return line.replace(spacing, (_, method, raw) => {
      const value = Number(raw);
      const tier = value <= 5 ? 1 : value <= 10 ? 2 : 3;
      return replaceTier(method, tier);
    }).replace(utilitySpacing, (_, method, rawTier) => replaceTier(method, Number(rawTier)));
  }).join(source.includes('\r\n') ? '\r\n' : '\n');
  const next = replaceLines(source);
  if (next !== source) fs.writeFileSync(fullPath, next, 'utf8');
}
console.log(`files=${files.length} replacements=${replacements}`);
for (const [tier, count] of counts) console.log(`${tier}: ${count}`);
