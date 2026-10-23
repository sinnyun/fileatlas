// Protect the product-wide typography and three-tier spacing contract.
// This is a lexical guard; same-process MCP remains the visual/layout check.
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');

// These are existing geometry-specific exceptions. Keep their expressions and
// counts frozen so they cannot become a precedent for new plugin spacing.
const approvedDynamicSpacing = new Map([
  ['crates/fileatlas-plugin-file-manager/src/view/render.rs', new Map([
    ['GRID_PADDING', 2], ['month_top_gap', 1], ['header_gap', 1],
  ])],
  ['crates/fileatlas-plugin-file-manager/src/view/bulk_actions/filters.rs', new Map([
    ['(depth as f32) * 16.', 1],
  ])],
  ['crates/fileatlas-plugin-file-manager/src/view/filter_row/multi_select.rs', new Map([
    ['fileatlas_ui_bridge::theme::space_compact() + fileatlas_ui_bridge::theme::space_expanded() * *depth as f32', 1],
  ])],
  ['crates/fileatlas-plugin-file-manager/src/view/inspector.rs', new Map([
    ['(*depth as f32) * 14. + 7.', 1],
  ])],
  ['crates/fileatlas-plugin-file-manager/src/view/tagging_picker.rs', new Map([
    ['(depth as f32) * 18.', 1],
  ])],
  ['crates/fileatlas-plugin-file-manager/src/view/inspector/history_tree.rs', new Map([
    ['if nested { 48. } else { 28. }', 1],
  ])],
  ['crates/fileatlas-plugin-navigation/src/view/tag_picker.rs', new Map([
    ['depth as f32 * 12.', 2],
  ])],
  ['crates/fileatlas-plugin-navigation/src/view/render/tag_render.rs', new Map([
    ['fileatlas_ui_bridge::theme::space_compact() + depth as f32 * fileatlas_ui_bridge::theme::space_expanded()', 1],
    ['fileatlas_ui_bridge::theme::space_standard() + depth as f32 * fileatlas_ui_bridge::theme::space_expanded()', 1],
  ])],
  ['crates/fileatlas-plugin-navigation/src/view/tag_modal.rs', new Map([
    ['fileatlas_ui_bridge::theme::space_compact() + depth as f32 * fileatlas_ui_bridge::theme::space_expanded()', 1],
  ])],
  ['crates/fileatlas-plugin-project-tree/src/view.rs', new Map([
    ['fileatlas_ui_bridge::theme::space_standard() + node.depth as f32 * fileatlas_ui_bridge::theme::space_expanded()', 1],
  ])],
  ['crates/fileatlas-plugin-settings/src/view/layout.rs', new Map([
    ['theme::SETTINGS_CONTENT_PADDING', 1],
  ])],
  ['crates/fileatlas-ui-bridge/src/tag_capsule.rs', new Map([
    ['gap', 1], ['padding', 1],
  ])],
]);

function normalizeExpression(value) {
  return value.replace(/\s+/g, ' ').trim();
}

function matchingParen(source, openIndex) {
  let depth = 0;
  for (let index = openIndex; index < source.length; index += 1) {
    if (source[index] === '(') depth += 1;
    else if (source[index] === ')') {
      depth -= 1;
      if (depth === 0) return index;
    }
  }
  return -1;
}

function dynamicSpacingCalls(source) {
  const calls = [];
  const methodPattern = /\.(gap|p|px|py|pt|pb|pl|pr|m|mx|my|mt|mb|ml|mr)\s*\(/g;
  for (const match of source.matchAll(methodPattern)) {
    const methodOpen = source.indexOf('(', match.index);
    const methodClose = matchingParen(source, methodOpen);
    if (methodClose < 0) continue;
    const args = source.slice(methodOpen + 1, methodClose).trim();
    if (!args.startsWith('px')) continue;
    const pxOpen = args.indexOf('(');
    if (pxOpen < 0) continue;
    const pxClose = matchingParen(args, pxOpen);
    if (pxClose < 0 || args.slice(pxClose + 1).trim()) continue;
    const expression = normalizeExpression(args.slice(pxOpen + 1, pxClose));
    if (!/^(?:(?:fileatlas_ui_bridge::)?theme::|crate::theme::)?space_(?:compact|standard|expanded)\(\)$/.test(expression)) {
      calls.push({ expression, index: match.index });
    }
  }
  return calls;
}

function checkDynamicSpacing(relative, source) {
  const errors = [];
  const expected = approvedDynamicSpacing.get(relative) ?? new Map();
  const found = new Map();
  for (const call of dynamicSpacingCalls(source)) {
    const remaining = (expected.get(call.expression) ?? 0) - (found.get(call.expression) ?? 0);
    const lineNumber = source.slice(0, call.index).split(/\r?\n/).length;
    if (remaining <= 0) {
      errors.push(`${relative}:${lineNumber}: spacing must use space_compact/standard/expanded(); unapproved dynamic expression: ${call.expression}`);
    } else {
      found.set(call.expression, (found.get(call.expression) ?? 0) + 1);
    }
  }
  for (const [expression, count] of expected) {
    const actual = found.get(expression) ?? 0;
    if (actual !== count) errors.push(`${relative}: expected ${count} frozen spacing exception(s) for ${expression}, found ${actual}`);
  }
  return errors;
}

function violations(relative, source) {
  const errors = [];
  const isUiSource = relative.startsWith('crates/fileatlas-plugin-') ||
    relative.startsWith('crates/fileatlas-shell/') ||
    relative.startsWith('crates/fileatlas-ui-bridge/src/');
  if (!isUiSource) return errors;
  errors.push(...checkDynamicSpacing(relative, source));

  source.split(/\r?\n/).forEach((line, index) => {
    if (/^\s*\/\//.test(line)) return;
    const lineNumber = index + 1;
    if (/\.(?:gap|p|px|py|pt|pb|pl|pr|m|mx|my|mt|mb|ml|mr)_[1-9]\d*\s*\(\s*\)/.test(line)) {
      errors.push(`${relative}:${lineNumber}: replace GPUI Kit numeric spacing shortcuts with space_compact/standard/expanded()`);
    }
    const sharedTypography = relative === 'crates/fileatlas-ui-bridge/src/components.rs' ||
      relative === 'crates/fileatlas-ui-bridge/src/tag_capsule.rs';
    if (!sharedTypography && /\.(?:text_size|text_(?:xs|sm|base|lg|xl|[0-9]xl)|line_height|font_(?:bold|semibold|medium|weight))\s*\(/.test(line)) {
      errors.push(`${relative}:${lineNumber}: typography is owned by WorkbenchTypography and the shared UI Bridge`);
    }
    if (relative.startsWith('crates/fileatlas-plugin-') && /\.font_family\s*\(/.test(line)) {
      errors.push(`${relative}:${lineNumber}: font family is set once by the Shell root`);
    }
  });
  return errors;
}

function verifyTokens() {
  const themePath = 'crates/fileatlas-ui-bridge/src/theme.rs';
  const theme = fs.readFileSync(path.join(root, themePath), 'utf8');
  const expected = [
    [/pub const FONT_SIZE_TITLE:\s*f32\s*=\s*15\.0\s*;/, 'title font size 15'],
    [/pub const FONT_SIZE_BODY:\s*f32\s*=\s*14\.0\s*;/, 'body font size 14'],
    [/pub const FONT_SIZE_ASSISTANT:\s*f32\s*=\s*12\.0\s*;/, 'assistant font size 12'],
    [/pub const LINE_HEIGHT_TITLE:\s*f32\s*=\s*20\.0\s*;/, 'title line height 20'],
    [/pub const LINE_HEIGHT_BODY:\s*f32\s*=\s*18\.0\s*;/, 'body line height 18'],
    [/pub const LINE_HEIGHT_ASSISTANT:\s*f32\s*=\s*16\.0\s*;/, 'assistant line height 16'],
    [/pub fn space_compact\(\)\s*->\s*f32\s*\{\s*4\.\s*\*\s*density_scale\(\)\s*\}/, 'compact spacing base 4'],
    [/pub fn space_standard\(\)\s*->\s*f32\s*\{\s*8\.\s*\*\s*density_scale\(\)\s*\}/, 'standard spacing base 8'],
    [/pub fn space_expanded\(\)\s*->\s*f32\s*\{\s*16\.\s*\*\s*density_scale\(\)\s*\}/, 'expanded spacing base 16'],
  ];
  const errors = expected.filter(([pattern]) => !pattern.test(theme)).map(([, label]) => `${themePath}: expected ${label}`);
  const componentsPath = 'crates/fileatlas-ui-bridge/src/components.rs';
  const components = fs.readFileSync(path.join(root, componentsPath), 'utf8');
  for (const [tier, fragments] of Object.entries({
    title: ['FONT_SIZE_TITLE', 'LINE_HEIGHT_TITLE', '.font_semibold()'],
    body: ['FONT_SIZE_BODY', 'LINE_HEIGHT_BODY', 'FontWeight::NORMAL'],
    assistant: ['FONT_SIZE_ASSISTANT', 'LINE_HEIGHT_ASSISTANT', 'FontWeight::NORMAL'],
  })) {
    const start = components.indexOf(`fn workbench_${tier}(self) -> Self {`);
    const end = components.indexOf('\n    #[track_caller]', start + 1);
    const body = start < 0 ? '' : components.slice(start, end < 0 ? undefined : end);
    for (const fragment of fragments) {
      if (!body.includes(fragment)) errors.push(`${componentsPath}: workbench_${tier} must use ${fragment}`);
    }
  }
  return errors;
}

if (process.argv.includes('--self-test')) {
  const assert = require('node:assert/strict');
  const file = 'crates/fileatlas-plugin-new/src/view.rs';
  assert.equal(violations(file, '.gap_2()').length, 1);
  assert.equal(violations(file, '.gap_10()').length, 1);
  assert.equal(violations(file, '.p_3()').length, 1);
  assert.equal(violations(file, '.gap_0()').length, 0);
  assert.equal(violations(file, '.gap(px(theme::space_compact()))').length, 0);
  assert.equal(violations(file, '.gap(px(local_gap))').length, 1);
  assert.equal(violations(file, '.pl(px(depth as f32 * 12.))').length, 1);
  assert.equal(violations(file, '.p(px(if dense { 4. } else { 8. }))').length, 1);
  assert.equal(violations('crates/fileatlas-plugin-navigation/src/view/tag_picker.rs', '.pl(px(depth as f32 * 12.))\n.pl(px(depth as f32 * 12.))').length, 0);
  assert.equal(violations('crates/fileatlas-plugin-navigation/src/view/tag_picker.rs', '.pl(px(depth as f32 * 12.))').length, 1);
  assert.equal(violations('crates/fileatlas-plugin-navigation/src/view/tag_picker.rs', '.pl(px(depth as f32 * 12.))\n.pl(px(depth as f32 * 12.))\n.pl(px(depth as f32 * 12.))').length, 1);
  assert.equal(violations(file, '.text_size(px(13.))').length, 1);
  assert.equal(violations(file, '.workbench_title()').length, 0);
  assert.equal(violations('crates/fileatlas-ui-bridge/src/components.rs', '.text_size(px(theme::FONT_SIZE_BODY))').length, 0);
  assert.equal(violations('crates/fileatlas-shell/src/shell.rs', '.font_family(theme::UI_FONT_FAMILY)').length, 0);
  console.log('UI design-token guard probes: passed');
} else {
  const errors = verifyTokens();
  function visit(directory) {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const file = path.join(directory, entry.name);
      if (entry.isDirectory()) visit(file);
      else if (file.endsWith('.rs')) {
        const relative = path.relative(root, file).replaceAll('\\', '/');
        errors.push(...violations(relative, fs.readFileSync(file, 'utf8')));
      }
    }
  }
  visit(path.join(root, 'crates'));
  if (errors.length) {
    console.error(errors.join('\n'));
    process.exitCode = 1;
  } else {
    console.log('UI design-token contract: passed (3 typography tiers, 3 spacing tiers)');
  }
}
