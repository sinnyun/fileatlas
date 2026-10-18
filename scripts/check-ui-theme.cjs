// Enforce the public theme boundary, including nested/new plugin views.
// This lexical check supplements (and cannot replace) same-process visual QA.
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');

function violations(relative, source) {
  if (/^crates\/photag-ui-bridge\/src\/theme(?:\/|\.rs$)/.test(relative)) return [];
  const uiFile = /\/src\/(?:view(?:\/|\.rs$)|shell(?:\/|\.rs$)|main\.rs$|components\.rs$|panels\.rs$|tag_capsule\.rs$)/.test(relative);
  if (!uiFile) return [];
  const errors = [];
  source.split(/\r?\n/).forEach((line, index) => {
    if (/^\s*\/\//.test(line)) return;
    // The software settings file also contains native registry declarations.
    if (relative === 'crates/photag-plugin-settings/src/view/software.rs' &&
        /^const (HKEY_CURRENT_USER|HKEY_LOCAL_MACHINE|RRF_RT_ANY|RRF_SUBKEY_WOW6464KEY|RRF_SUBKEY_WOW6432KEY):/.test(line)) return;
    if (/\b0x(?:[\da-f]_*){6,8}(?:u32)?\b/i.test(line))
      errors.push(`${relative}:${index + 1}: use a semantic theme token, not a local paint literal`);
    if (/\bTheme::(?:change|update)|\bThemeMode::/.test(line))
      errors.push(`${relative}:${index + 1}: component theme is owned by UI Bridge theme`);
    if (!relative.endsWith('/photag-ui-bridge/src/components.rs') && /\b(?:Input|Textarea)::new\s*\(/.test(line))
      errors.push(`${relative}:${index + 1}: use components::text_input/search_input/textarea`);
  });
  return errors;
}

if (process.argv.includes('--self-test')) {
  const assert = require('node:assert/strict');
  const file = 'crates/photag-plugin-new/src/view/cards.rs';
  assert.equal(violations(file, '.bg(rgb(0x123456))').length, 1);
  assert.equal(violations(file, 'const LOCAL_COLOR: u32 = 0x123456;').length, 1);
  assert.equal(violations(file, '.bg(rgba(0x1234_5678))').length, 1);
  assert.equal(violations(file, 'const COLOR: u32 = 0x123456u32;').length, 1);
  assert.equal(violations(file, 'Theme::update(cx, |_| {});').length, 1);
  assert.equal(violations(file, '.bg(rgb(theme::SURFACE_CARD))').length, 0);
  assert.equal(violations(file, 'Input::new(&state)').length, 1);
  assert.equal(violations(file, 'components::text_input(&state)').length, 0);
  assert.equal(violations('crates/photag-ui-bridge/src/theme.rs', 'const COLOR: u32 = 0x123456;').length, 0);
  assert.equal(violations('crates/photag-plugin-settings/src/view/software.rs',
    'const HKEY_CURRENT_USER: Hkey = 0x8000_0001u32 as i32 as isize;').length, 0);
  console.log('UI theme guard probes: passed');
} else {
  const errors = [];
  function visit(directory) {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const file = path.join(directory, entry.name);
      if (entry.isDirectory()) visit(file);
      else if (file.endsWith('.rs')) errors.push(...violations(
        path.relative(root, file).replaceAll('\\', '/'), fs.readFileSync(file, 'utf8')));
    }
  }
  visit(path.join(root, 'crates'));
  if (errors.length) { console.error(errors.join('\n')); process.exitCode = 1; }
  else console.log('UI theme ownership: passed');
}
