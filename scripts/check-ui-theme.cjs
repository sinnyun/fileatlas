// Enforce the public theme boundary, including nested/new plugin views.
// This lexical check supplements (and cannot replace) same-process visual QA.
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
// Existing feature canvases are legacy, explicitly reviewed uses for editor
// interaction/measurement. New plugins must route visual paint through UI Bridge.
const approvedPluginCanvasCalls = new Map([
  ['crates/fileatlas-plugin-debug/src/view.rs', 1],
  ['crates/fileatlas-plugin-debug/src/view/selection.rs', 1],
  ['crates/fileatlas-plugin-file-manager/src/view/filter_row/helpers.rs', 1],
  ['crates/fileatlas-plugin-file-manager/src/view/group_frame.rs', 1],
  ['crates/fileatlas-plugin-file-manager/src/view/sliders.rs', 1],
  ['crates/fileatlas-plugin-navigation/src/view/sections/render/favorites.rs', 1],
  ['crates/fileatlas-plugin-workcrate-ui/src/view/tracking.rs', 1],
]);

function violations(relative, source) {
  const pluginSource = relative.startsWith('crates/fileatlas-plugin-');
  const sourceSyntax = source
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/\/\/.*$/gm, '')
    .replace(/"(?:\\.|[^"\\])*"/g, '""');
  const pluginUiPrimitive = pluginSource &&
    /\.(?:bg|text_color|border(?:_[a-z]+)?|rounded(?:_[a-z]+)?|shadow(?:_[a-z]+)?|opacity|group_hover)\s*\(|\b(?:Button|Input|Textarea|Checkbox|Radio|DatePicker|Popover)::new\s*\(|theme::(?:scrim|modal_rgba)\s*\(/.test(source);
  const pluginStyleSource = pluginSource &&
    /\bimpl\s+(?:gpui_kit::)?Render\b|\b(?:gpui::)?Style\s*\{|\bdiv\s*\(|\.style\s*\(/.test(sourceSyntax);
  // The canvas engine is a graphics/document core copied from the MIT upstream
  // package. Its custom canvas painter consumes the host WhiteboardStyle callback;
  // it is not product chrome or a feature plugin. The owning canvas plugin remains
  // fully checked below and must route its controls through UI Bridge.
  const canvasEngineCore = relative.startsWith('crates/fileatlas-canvas-engine/src/');
  const uiFile = !canvasEngineCore && (pluginUiPrimitive ||
    /\/src\/(?:view(?:\/|\.rs$)|shell(?:\/|\.rs$)|ui\.rs$|main\.rs$|components\.rs$|panels\.rs$|tag_capsule\.rs$|theme(?:\/|\.rs$)|settings_page(?:\/|\.rs$))/.test(relative) ||
    pluginStyleSource || /\bimpl\s+(?:gpui_kit::)?Render\b|\.(?:bg|text_color|border_color)\s*\(/.test(source));
  const themeMutation = /\bTheme::(?:change|update|global_mut)|\.colors\.[a-z_]+\s*=/;
  const errors = [];
  if (pluginSource) {
    const canvasCalls = (source.match(/\b(?:gpui::)?canvas\s*\(/g) || []).length;
    const approvedCanvasCalls = approvedPluginCanvasCalls.get(relative) ?? 0;
    if (canvasCalls > approvedCanvasCalls) {
      errors.push(`${relative}: new direct GPUI canvas painting is not allowed in plugins; add the shared behavior to UI Bridge`);
    }
    if (/\bCanvas::new\s*\(|\bpaint_quad\s*\(/.test(source)) {
      errors.push(`${relative}: direct GPUI paint primitives belong to UI Bridge shared components`);
    }
    for (const component of ['Button', 'Input', 'Textarea', 'Checkbox', 'Radio', 'DatePicker', 'Popover']) {
      const aliases = [
        ...[...source.matchAll(new RegExp(`\\b${component}\\s+as\\s+([A-Za-z_][A-Za-z0-9_]*)`, 'g'))].map(match => match[1]),
        ...[...source.matchAll(new RegExp(`\\btype\\s+([A-Za-z_][A-Za-z0-9_]*)\\s*=\\s*(?:gpui_kit::component::[a-z_]+::)?${component}\\s*;`, 'g'))].map(match => match[1]),
      ];
      for (const alias of aliases) {
        if (new RegExp(`\\b${alias}::new\\s*\\(`).test(source)) {
          errors.push(`${relative}: ${alias} aliases ${component}; create it through UI Bridge components`);
        }
      }
    }
  }
  if (!uiFile && relative !== "crates/fileatlas-plugin-drag-drop/src/dispatch.rs" && !themeMutation.test(source)) return errors;
  source.split(/\r?\n/).forEach((line, index) => {
    if (/^\s*\/\//.test(line)) return;
    // The software settings file also contains native registry declarations.
    if (relative === 'crates/fileatlas-plugin-settings/src/view/software.rs' &&
        /^const (HKEY_CURRENT_USER|HKEY_LOCAL_MACHINE|RRF_RT_ANY|RRF_SUBKEY_WOW6464KEY|RRF_SUBKEY_WOW6432KEY):/.test(line)) return;
    const temporaryAuditPaint = /TEMP typography audit|AUDIT_(?:FRAME|TEXT)_/.test(line) ||
      (relative.endsWith('/fileatlas-ui-bridge/src/theme/runtime.rs') && /0x0066CC/.test(line));
    if (!temporaryAuditPaint && /\b0x(?:[\da-f]_*){6,8}(?:u32)?\b/i.test(line))
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
    if (relative.startsWith('crates/fileatlas-plugin-') && /\bButton::new\s*\(/.test(line))
      errors.push(`${relative}:${index + 1}: create buttons through UI Bridge components::button/action/icon_button`);
    if (relative.startsWith('crates/fileatlas-plugin-') && /\bPopover::new\s*\(/.test(line))
      errors.push(`${relative}:${index + 1}: create popovers through UI Bridge components::popover`);
    if (relative.startsWith('crates/fileatlas-plugin-') && /\.border_(?:color|[trbl]_[0-9]+|[0-9]+)\s*\(/.test(line))
      errors.push(`${relative}:${index + 1}: border, divider and drag-target states belong to shared UI Bridge components`);
    if (relative.startsWith('crates/fileatlas-plugin-') && /\.border\s*\(/.test(line))
      errors.push(`${relative}:${index + 1}: border construction belongs to shared UI Bridge components`);
    if (relative.startsWith('crates/fileatlas-plugin-') && (/\.style\s*\(/.test(line) || /\b(?:gpui::)?(?:Style|StyleRefinement)\b(?:\s*\{|::)/.test(line)))
      errors.push(`${relative}:${index + 1}: plugin-local GPUI Style objects bypass shared component ownership`);
    if (relative.startsWith('crates/fileatlas-plugin-') && /\b(?:Checkbox|Radio|DatePicker)::new\s*\(/.test(line))
      errors.push(`${relative}:${index + 1}: create standard selection/date controls through UI Bridge components`);
    if (relative.startsWith('crates/fileatlas-plugin-') && /\.hover\s*\(/.test(line))
      errors.push(`${relative}:${index + 1}: define hover states through shared UI Bridge components, not in a plugin`);
    if (relative.startsWith('crates/fileatlas-plugin-') && /\.text_color\s*\(/.test(line))
      errors.push(`${relative}:${index + 1}: choose a semantic ForegroundRole; theme colors are painted by UI Bridge`);
    if (relative.startsWith('crates/fileatlas-plugin-') && /\.(?:bg|rounded(?:_[a-z]+)?)\s*\(/.test(line))
      errors.push(`${relative}:${index + 1}: use shared SurfaceRole/CornerRole components instead of local fills or corners`);
    if (relative.startsWith('crates/fileatlas-plugin-') && /\.(?:opacity|group_hover)\s*\(/.test(line))
      errors.push(`${relative}:${index + 1}: interactive visibility and hover treatment belong to shared components`);
    const sharedUiSource = relative.startsWith('crates/fileatlas-plugin-') ||
      relative.startsWith('crates/fileatlas-shell/') ||
      relative.startsWith('crates/fileatlas-ui-bridge/src/');
    if (sharedUiSource &&
        /\.(?:gap|p|px|py|pt|pb|pl|pr|m|mx|my|mt|mb|ml|mr)\(\s*px\(\s*-?\d/.test(line))
      errors.push(`${relative}:${index + 1}: use a shared theme spacing tier instead of a local numeric gap, padding, or margin`);
    if (sharedUiSource &&
        /\.(?:gap|p|px|py|pt|pb|pl|pr|m|mx|my|mt|mb|ml|mr)\(\s*px\(\s*(?:(?:fileatlas_ui_bridge::)?theme::|crate::theme::)?(?:SPACE_(?!COMPACT\b|STANDARD\b|EXPANDED\b)[A-Z0-9_]+|(?:PANEL|REGION|COLUMN|FORM_CONTROL|TYPOGRAPHY_[A-Z]+)_GAP)\s*\)\)/.test(line))
      errors.push(`${relative}:${index + 1}: use only SPACE_COMPACT, SPACE_STANDARD, or SPACE_EXPANDED for layout spacing`);
    if (relative.startsWith('crates/fileatlas-plugin-') && /\.(?:text_size|text_(?:xs|sm|base|lg|xl|[0-9]xl)|line_height)\s*\(/.test(line))
      errors.push(`${relative}:${index + 1}: use only WorkbenchTypography title/body/assistant tiers; do not set font size or line height in a plugin`);
    if (relative.startsWith('crates/fileatlas-plugin-') && /\.font_(?:bold|semibold|medium|weight|family)\s*\(/.test(line))
      errors.push(`${relative}:${index + 1}: plugin typography weight and font family are owned by the shared theme`);
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
  const nonView = 'crates/fileatlas-plugin-new/src/internal.rs';
  assert.equal(violations(file, '.bg(rgb(0x123456))').length, 2);
  assert.equal(violations(file, 'const LOCAL_COLOR: u32 = 0x123456;').length, 1);
  assert.equal(violations(file, '.bg(rgba(0x1234_5678))').length, 2);
  assert.equal(violations(file, 'const COLOR: u32 = 0x123456u32;').length, 1);
  assert.equal(violations(file, 'Theme::update(cx, |_| {});').length, 1);
  assert.equal(violations(file, '.bg(rgb(theme::SURFACE_CARD))').length, 2);
  assert.equal(violations(file, 'Input::new(&state)').length, 1);
  assert.equal(violations(file, 'Button::new("x")').length, 1);
  assert.equal(violations(file, 'Popover::new("x")').length, 1);
  assert.equal(violations(file, '.border_1()').length, 1);
  assert.equal(violations(file, '.border_t_1()').length, 1);
  assert.equal(violations(file, '.border_color(rgb(theme::accent()))').length, 1);
  assert.equal(violations(file, '.border_b_1()').length, 1);
  assert.equal(violations(file, '.border(px(1.))').length, 1);
  assert.equal(violations(file, '.bg(rgb(theme::surface_panel()))').length, 1);
  assert.equal(violations(file, '.rounded(px(theme::ROUND_MD))').length, 1);
  assert.equal(violations(file, '.rounded_full()').length, 1);
  assert.equal(violations(nonView, '.rounded_full()').length, 1);
  assert.equal(violations(file, '.opacity(0.)').length, 1);
  assert.equal(violations(file, '.group_hover("row", |style| style.opacity(1.))').length, 1);
  assert.equal(violations(nonView, '.group_hover("row", |style| style.opacity(1.))').length, 1);
  assert.equal(violations(file, '.text_color(rgb(theme::text_muted()))').length, 1);
  assert.equal(violations(file, 'Checkbox::new("x")').length, 1);
  assert.equal(violations(file, 'Radio::new("x")').length, 1);
  assert.equal(violations(file, 'DatePicker::new(&state)').length, 1);
  assert.equal(violations(file, '.hover(|style| style.bg(rgb(theme::surface_hover())))').length, 2);
  assert.equal(violations('crates/fileatlas-ui-bridge/src/components.rs', '.hover(|style| style.bg(rgb(theme::surface_hover())))').length, 0);
  assert.equal(violations(file, '.gap(px(5.))').length, 1);
  assert.equal(violations(file, '.gap(px(theme::SPACE_1))').length, 1);
  assert.equal(violations(file, '.gap(px(theme::SPACE_COMPACT))').length, 0);
  assert.equal(violations(file, '.px(px(8.))').length, 1);
  assert.equal(violations(file, '.font_semibold()').length, 1);
  assert.equal(violations(file, '.text_xs()').length, 1);
  assert.equal(violations(file, '.text_size(px(13.0))').length, 1);
  assert.equal(violations(file, '.line_height(px(20.0))').length, 1);
  assert.equal(violations(file, '.font_medium()').length, 1);
  assert.equal(violations(file, '.font_weight(FontWeight::NORMAL)').length, 1);
  assert.equal(violations(file, '.font_family("Arial")').length, 1);
  assert.equal(violations('crates/fileatlas-plugin-new/src/custom_button.rs', 'use gpui_kit::component::button::Button as Primary; Primary::new("x")').length, 1);
  assert.equal(violations('crates/fileatlas-plugin-new/src/custom_button.rs', 'type Primary = Button; Primary::new("x")').length, 1);
  assert.equal(violations('crates/fileatlas-plugin-new/src/custom_style.rs', 'let style = Style { background: color };').length, 1);
  assert.equal(violations('crates/fileatlas-plugin-new/src/custom_style.rs', 'div().style(|style| style.background = color)').length, 1);
  assert.equal(violations('crates/fileatlas-plugin-new/src/custom_paint.rs', 'canvas(bounds, |bounds, _, window| window.paint_quad(quad(bounds)))').length, 2);
  assert.equal(violations('crates/fileatlas-plugin-debug/src/view.rs', 'canvas(bounds, |bounds, _, _| {})').length, 0);
  assert.equal(violations('crates/fileatlas-plugin-debug/src/view.rs', 'canvas(bounds, |bounds, _, _| {}); canvas(bounds, |bounds, _, _| {})').length, 1);
  assert.equal(violations(file, '.workbench_assistant()\n.text_color(rgb(theme::text_muted()))').length, 1);
  assert.equal(violations(file, '.text_color(rgb(theme::text_primary()))\n.workbench_assistant()').length, 1);
  assert.equal(violations(file, 'components::text_input(&state)').length, 0);
  assert.equal(violations('crates/fileatlas-ui-bridge/src/theme.rs', 'const COLOR: u32 = 0x123456;').length, 1);
  assert.equal(violations(file, '.bg(hsla(0.5, 0.8, 0.6, 1.0))').length, 2);
  assert.equal(violations(file, 'let palette = ["#123456"];').length, 1);
  assert.equal(violations(file, 'theme.colors.primary = color;').length, 1);
  assert.equal(violations('crates/fileatlas-plugin-new/src/lib.rs', 'Theme::update(cx, |_| {});').length, 1);
  assert.equal(violations('crates/fileatlas-plugin-new/src/custom.rs', '.bg(rgb(0x123456))').length, 2);
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
