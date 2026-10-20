// Enforce the public theme boundary, including nested/new plugin views.
// This lexical check supplements (and cannot replace) same-process visual QA.
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');

function violations(relative, source) {
  const uiFile = /\/src\/(?:view(?:\/|\.rs$)|shell(?:\/|\.rs$)|ui\.rs$|main\.rs$|components\.rs$|panels\.rs$|tag_capsule\.rs$|theme(?:\/|\.rs$)|settings_page(?:\/|\.rs$))/.test(relative) ||
    /\bimpl\s+(?:gpui_kit::)?Render\b|\.(?:bg|text_color|border_color)\s*\(/.test(source);
  const themeMutation = /\bTheme::(?:change|update|global_mut)|\.colors\.[a-z_]+\s*=/;
  if (!uiFile && relative !== "crates/fileatlas-plugin-drag-drop/src/dispatch.rs" && !themeMutation.test(source)) return [];
  const errors = [];
  source.split(/\r?\n/).forEach((line, index) => {
    if (/^\s*\/\//.test(line)) return;
    // The software settings file also contains native registry declarations.
    if (relative === 'crates/fileatlas-plugin-settings/src/view/software.rs' &&
        /^const (HKEY_CURRENT_USER|HKEY_LOCAL_MACHINE|RRF_RT_ANY|RRF_SUBKEY_WOW6464KEY|RRF_SUBKEY_WOW6432KEY):/.test(line)) return;
    if (/\b0x(?:[\da-f]_*){6,8}(?:u32)?\b/i.test(line))
      errors.push(`${relative}:${index + 1}: use a semantic theme token, not a local paint literal`);
    if (/"#[0-9a-f]{6}(?:[0-9a-f]{2})?"/i.test(line))
      errors.push(`${relative}:${index + 1}: no UI-local hexadecimal palette; use native theme roles`);
    if (/\b(?:hsla|hsl|rgb)\(\s*(?!0x)\d/.test(line) || /\brgba\(\s*[1-9]/.test(line))
      errors.push(`${relative}:${index + 1}: no numeric paint constructors; use native theme roles`);
    if (/\btheme::(?:SURFACE_|TEXT_|FILL_|DANGER\b|WARNING\b|SUCCESS\b|STAR\b)/.test(line))
      errors.push(`${relative}:${index + 1}: legacy palette constants are forbidden`);
    if (/\bmode_color\s*\(|\b(?:ACCENT|LIGHT_MODE)\s*:\s*Atomic/.test(line))
      errors.push(`${relative}:${index + 1}: do not recreate a mode palette outside GPUI Kit`);
    if (relative !== 'crates/fileatlas-ui-bridge/src/theme/runtime.rs' && themeMutation.test(line))
      errors.push(`${relative}:${index + 1}: component theme is owned by UI Bridge theme`);
    if (!relative.endsWith('/fileatlas-ui-bridge/src/components.rs') && /\b(?:Input|Textarea)::new\s*\(/.test(line))
      errors.push(`${relative}:${index + 1}: use components::text_input/search_input/textarea`);
    // Icons must come from the bundled catalog (theme::icon / IconName), never
    // from a text glyph: glyphs ignore theme sizing and break non-Segoe fonts.
    for (const literal of line.replace(/\/\/.*$/, '').match(/"(?:[^"\\]|\\.)*"/g) || []) {
      const body = literal.slice(1, -1);
      const glyphIcon =
        /[\u25A0-\u27BF\u2B00-\u2BFF\u{1F000}-\u{1FAFF}\u{E000}-\u{F8FF}\uFE0F]/u.test(body) ||
        /^[\s\u2190-\u21FF\u25A0-\u27BF\u2B00-\u2BFF\u{1F000}-\u{1FAFF}\u{E000}-\u{F8FF}\uFE0F]*[\u2190-\u21FF\u25A0-\u27BF\u2B00-\u2BFF\u{1F000}-\u{1FAFF}\u{E000}-\u{F8FF}\uFE0F][\s\u2190-\u21FF\u25A0-\u27BF\u2B00-\u2BFF\u{1F000}-\u{1FAFF}\u{E000}-\u{F8FF}\uFE0F]*$/u.test(body);
      if (glyphIcon) {
        errors.push(`${relative}:${index + 1}: render icons with theme::icon(IconName::…) instead of the text glyph ${literal}`);
        break;
      }
    }
  });
  return errors;
}

if (process.argv.includes('--self-test')) {
  const assert = require('node:assert/strict');
  const file = 'crates/fileatlas-plugin-new/src/view/cards.rs';
  assert.equal(violations(file, '.bg(rgb(0x123456))').length, 1);
  assert.equal(violations(file, 'const LOCAL_COLOR: u32 = 0x123456;').length, 1);
  assert.equal(violations(file, '.bg(rgba(0x1234_5678))').length, 1);
  assert.equal(violations(file, 'const COLOR: u32 = 0x123456u32;').length, 1);
  assert.equal(violations(file, 'Theme::update(cx, |_| {});').length, 1);
  assert.equal(violations(file, '.bg(rgb(theme::SURFACE_CARD))').length, 1);
  assert.equal(violations(file, 'Input::new(&state)').length, 1);
  assert.equal(violations(file, 'components::text_input(&state)').length, 0);
  assert.equal(violations('crates/fileatlas-ui-bridge/src/theme.rs', 'const COLOR: u32 = 0x123456;').length, 1);
  assert.equal(violations(file, '.bg(hsla(0.5, 0.8, 0.6, 1.0))').length, 1);
  assert.equal(violations(file, 'let palette = ["#123456"];').length, 1);
  assert.equal(violations(file, 'theme.colors.primary = color;').length, 1);
  assert.equal(violations('crates/fileatlas-plugin-new/src/lib.rs', 'Theme::update(cx, |_| {});').length, 1);
  assert.equal(violations('crates/fileatlas-plugin-new/src/custom.rs', '.bg(rgb(0x123456))').length, 1);
  assert.equal(violations('crates/fileatlas-plugin-settings/src/view/software.rs',
    'const HKEY_CURRENT_USER: Hkey = 0x8000_0001u32 as i32 as isize;').length, 0);
  assert.equal(violations(file, '.label("★")').length, 1);
  assert.equal(violations(file, '.label(if descending { "↓" } else { "↑" })').length, 1);
  assert.equal(violations(file, '.child("● Scan complete")').length, 1);
  assert.equal(violations(file, 'let marker = if p { "✓ " } else { "" };').length, 1);
  assert.equal(violations(file, '.label(format!("{} ▾", self.sort_by.label()))').length, 1);
  assert.equal(violations(file, '.child("🎨 Theme")').length, 1);
  assert.equal(violations(file, `.child("${String.fromCharCode(0xE021)}")`).length, 1);
  assert.equal(violations(file, '.label(tr("Export metadata…"))').length, 0);
  assert.equal(violations(file, 'format!("Rating {} → {}", old, new)').length, 0);
  assert.equal(violations(file, '.child(tr("9 programs · 3 categories"))').length, 0);
  assert.equal(violations(file, '.label(" ")').length, 0);
  assert.equal(violations(file, '.icon(theme::small_icon(IconName::Star)).dropdown_caret(true)').length, 0);
  assert.equal(violations(file, '.child("Loading icons…") // ◆ note').length, 0);
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
