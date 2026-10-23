// Summarize geometry snapshots emitted by capture-panel-atlas.cjs.
// Usage: node scripts/summarize-ui-geometry.cjs .local/validation/profiles/<run>
const fs = require('node:fs');
const path = require('node:path');

const profile = process.argv[2];
if (!profile) {
  console.error('Usage: node scripts/summarize-ui-geometry.cjs <validation-profile>');
  process.exit(2);
}
const directory = path.join(profile, 'screenshots');
if (!fs.existsSync(directory)) {
  console.error(`Geometry snapshot directory not found: ${directory}`);
  process.exit(2);
}

const files = fs.readdirSync(directory)
  .filter(name => name.endsWith('.geometry.json'))
  .sort();
const entries = files.flatMap(name => {
  const value = JSON.parse(fs.readFileSync(path.join(directory, name), 'utf8'));
  return (value.entries || []).map(entry => ({ ...entry, screenshot: name }));
});
const rounded = value => Number(value.toFixed(2));
const grouped = new Map();
for (const entry of entries) {
  const style = [entry.tier, entry.font_size, entry.line_height, entry.font_weight, entry.font_family].join('|');
  grouped.set(style, [...(grouped.get(style) || []), entry]);
}

console.log(`snapshots=${files.length} entries=${entries.length}`);
console.log('TYPOGRAPHY / EFFECTIVE OBSERVER TEXT STYLE');
for (const [style, values] of [...grouped.entries()].sort(([a], [b]) => a.localeCompare(b))) {
  const [tier, size, line, weight, family] = style.split('|');
  if (!['title', 'body', 'assistant'].includes(tier)) continue;
  console.log(`  ${tier}: ${size}px / ${line}px, ${weight}, ${family} — ${values.length}`);
}

console.log('COMPONENT BOUNDS (logical pixels)');
for (const tier of [...new Set(entries.map(entry => entry.tier))].sort()) {
  const values = entries.filter(entry => entry.tier === tier);
  const range = key => `${rounded(Math.min(...values.map(item => item[key])))}..${rounded(Math.max(...values.map(item => item[key])))}`;
  console.log(`  ${tier}: ${values.length}; width ${range('width')}; height ${range('height')}`);
}
console.log('SHARED SIZE TOKENS VS MEASURED BOUNDS');
for (const tier of ['button', 'navigation', 'card']) {
  const values = entries.filter(entry => entry.tier === tier &&
    (entry.height_token != null || entry.gap_token != null || entry.padding_x_token != null));
  const groups = new Map();
  for (const entry of values) {
    const key = `height=${rounded(entry.height)} token=${entry.height_token ?? '-'} min_width=${entry.min_width_token ?? '-'} padding_x=${entry.padding_x_token ?? '-'} padding_y=${entry.padding_y_token ?? '-'} gap=${entry.gap_token ?? '-'}`;
    groups.set(key, (groups.get(key) || 0) + 1);
  }
  for (const [key, count] of groups) console.log(`  ${tier}: ${key} — ${count}`);
}

console.log('BUTTON HEIGHT DEVIATIONS FROM SHARED TOKEN');
const buttonDeviations = entries.filter(entry => entry.tier === 'button' &&
  entry.height_token != null && Math.abs(entry.height - entry.height_token) > 0.5);
if (buttonDeviations.length) {
  for (const entry of buttonDeviations) {
    console.log(`  ${rounded(entry.height)}px (token ${entry.height_token}px) ${entry.component_id} at ${entry.source_file}:${entry.source_line} (${entry.screenshot})`);
  }
} else {
  console.log('  none');
}
console.log('SPACING LIMIT');
console.log('  Button-to-button gaps and frame insets are omitted: snapshots do not yet carry parent/row IDs, so coordinate-only pairing is ambiguous across columns and nested layouts.');

const expected = { title: '15|20|FontWeight(600.0)', body: '14|18|FontWeight(400.0)', assistant: '12|16|FontWeight(400.0)' };
console.log('TYPOGRAPHY VARIANTS TO REVIEW');
let variants = 0;
for (const [tier, signature] of Object.entries(expected)) {
  const [size, line, weight] = signature.split('|');
  const mismatches = entries.filter(entry => entry.tier === tier &&
    (String(entry.font_size) !== size || String(entry.line_height) !== line || entry.font_weight !== weight));
  variants += mismatches.length;
  for (const entry of mismatches) {
    console.log(`  ${tier} ${entry.font_size}/${entry.line_height} ${entry.font_weight} at ${entry.source_file}:${entry.source_line} (${entry.screenshot})`);
  }
}
if (!variants) console.log('  none');
