// Guard the temporary geometry audit at shared component boundaries so new
// base components cannot silently bypass the review frames.
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '..');
const read = relative => fs.readFileSync(path.join(root, relative), 'utf8');
const checks = [
  ['UI Bridge reusable review frames and instrumented shared style tiers', 'crates/fileatlas-ui-bridge/src/components.rs', [
    'pub fn review_section<', 'pub fn review_row<', 'pub fn review_card<',
    'fn workbench_region(self, sidebar: bool)', 'fn workbench_section(self)',
    'fn workbench_card(self, selected: bool)', 'fn workbench_nav(self, selected: bool)',
    'fn workbench_selectable_surface(self, selected: bool)', 'fn workbench_tree_row(self, selected: bool)',
    'pub fn card_surface<T: Styled + ParentElement>(content: T, selected: bool)', 'pub fn divider() -> Div',
    'pub fn vertical_divider() -> Div', 'pub fn modal_frame(content: Div) -> Div',
    'pub fn modal_overlay(content: Div) -> Div', 'pub fn color_swatch(',
    'pub fn icon_choice_button(', 'pub fn popover(id: impl Into<ElementId>) -> Popover',
    'pub fn checkbox(id: impl Into<ElementId>, checked: bool) -> Checkbox',
    'pub fn radio(id: impl Into<ElementId>, checked: bool) -> Radio',
    'pub fn date_picker(state: &Entity<DatePickerState>) -> DatePicker',
    'pub fn tab_button(',
    'pub fn artboard<T: Styled + ParentElement>(content: T) -> T',
    'pub fn selection_frame(left: f32, top: f32, width: f32, height: f32) -> Div',
    'pub fn selection_region(left: f32, top: f32, width: f32, height: f32) -> Div',
    'pub fn media_selection_overlay(size: f32, selection: f32, keyboard_focus: bool) -> Div',
    'pub fn badge(content: Div, selected: bool) -> Div',
    'pub fn colored_badge(content: Div, color: u32) -> Div',
    'pub fn timeline_marker(left: f32, top: f32, size: f32, color: u32, selected: bool) -> Div',
    'pub fn accent_card(content: Div, color: u32, selected: bool) -> Div',
    'fn workbench_control_frame(self, error: bool)',
    'fn workbench_control_surface(self, active: bool)',
    'fn workbench_group_frame(self, first: bool, last: bool, highlighted: bool)',
    'pub fn drop_target_style<T: Styled>(style: T) -> T',
    'pub fn timeline_connector(left: f32, top: f32, width: f32, height: f32, color: u32) -> Div',
    'pub fn keycap(label: impl Into<SharedString>, size: KeycapSize) -> Div',
  ]],
  ['section headings and heading blocks', 'crates/fileatlas-ui-bridge/src/components.rs', [
    'pub fn section_heading(label:', 'pub fn heading_block(title:',
  ]],
  ['sidebar sections and footer', 'crates/fileatlas-ui-bridge/src/components.rs', [
    'pub fn rail_footer(theme_control:', 'pub fn sidebar_section(body:',
    'pub fn sidebar_header(',
  ]],
  ['settings rows carry row bounds', 'crates/fileatlas-ui-bridge/src/settings_page/render.rs', [
    'components::review_row(',
  ]],
  ['tag capsules carry card bounds', 'crates/fileatlas-ui-bridge/src/tag_capsule.rs', [
    'crate::components::review_card(',
  ]],
  ['grouped headers and frames carry section bounds', 'crates/fileatlas-plugin-file-manager/src/view/month_header.rs', [
    'components::review_section(',
  ]],
  ['group frames carry section bounds', 'crates/fileatlas-plugin-file-manager/src/view/group_frame.rs', [
    'components::review_section(',
  ]],
  ['media cards carry card bounds', 'crates/fileatlas-plugin-file-manager/src/view/card.rs', [
    'components::review_card(',
  ]],
  ['theme and Settings controls use the shared rail footer', 'crates/fileatlas-plugin-settings/src/view/rail_entry.rs', [
    'components::rail_footer(mode, settings)',
  ]],
  ['single-line and multiline inputs expose audit frames', 'crates/fileatlas-ui-bridge/src/components.rs', [
    'pub fn text_input(state: &Entity<InputState>) -> Div',
    'pub fn textarea(state: &Entity<TextareaState>) -> Div',
    'fn audited_input(control: impl IntoElement) -> Div',
    'frame.child(audit_overlay(theme::AUDIT_FRAME_INPUT, 2.))',
  ]],
  ['floating panels mark their outer frame', 'crates/fileatlas-ui-bridge/src/panels.rs', [
    'theme::geometry_audit_enabled()', 'theme::AUDIT_FRAME_OUTER',
  ]],
  ['navigation drive, quick-access and favorite frames use the shared sidebar section', 'crates/fileatlas-plugin-navigation/src/view.rs', [
    'components::sidebar_section(body)',
  ]],
  ['launcher sidebar frame uses the shared section component', 'crates/fileatlas-plugin-launcher/src/view/sidebar.rs', [
    'fileatlas_ui_bridge::components::section()',
  ]],
  ['clipboard category editor uses the shared card component', 'crates/fileatlas-plugin-clipboard/src/view/sidebar.rs', [
    'fileatlas_ui_bridge::components::card_surface(',
  ]],
  ['plugin dialogs delegate frame and backdrop to UI Bridge', 'crates/fileatlas-plugin-file-manager/src/view/regions.rs', [
    'components::modal_overlay(',
  ]],
  ['plugin divider uses the shared geometry', 'crates/fileatlas-plugin-settings/src/view/layout.rs', [
    'components::vertical_divider()', 'components::DividerEdge::Top',
  ]],
  ['plugin content card uses a shared card frame', 'crates/fileatlas-plugin-file-manager/src/view/bulk_actions.rs', [
    'fileatlas_ui_bridge::components::card_surface(',
  ]],
  ['plugin data color choices use the shared swatch component', 'crates/fileatlas-plugin-navigation/src/view/tag_modal.rs', [
    'fileatlas_ui_bridge::components::color_swatch(', 'components::icon_choice_button(',
  ]],
  ['keyboard labels use a shared keycap component', 'crates/fileatlas-plugin-settings/src/view/shortcuts.rs', [
    'fileatlas_ui_bridge::components::keycap(',
  ]],
  ['canvas selection and handles use the shared overlay frame', 'crates/fileatlas-plugin-workcrate-ui/src/view/canvas_surface.rs', [
    'components::artboard(', 'components::selection_frame(',
  ]],
  ['media selection overlays use a shared selection component', 'crates/fileatlas-plugin-file-manager/src/view/card.rs', [
    'components::media_selection_overlay(',
  ]],
  ['rubber-band selection uses a shared selection component', 'crates/fileatlas-plugin-file-manager/src/view/render.rs', [
    'components::selection_region(',
  ]],
  ['file-manager sliders use shared track and thumb styling', 'crates/fileatlas-plugin-file-manager/src/view/sliders.rs', [
    'components::range_slider_visual(',
  ]],
  ['media, status and format markers use shared semantic components', 'crates/fileatlas-plugin-file-manager/src/view/card.rs', [
    'components::status_badge(', 'components::media_badge(', 'components::thumbnail_frame(', 'components::media_hover_overlay(',
  ]],
  ['project-tree guide marks use the shared guide component', 'crates/fileatlas-plugin-project-tree/src/view.rs', [
    'components::tree_guide_dots(',
  ]],
  ['media and status markers use shared components', 'crates/fileatlas-plugin-file-manager/src/view/file_format.rs', [
    'components::file_format_badge(',
  ]],
  ['navigation hover reveal uses the shared interaction state', 'crates/fileatlas-plugin-navigation/src/view/render/tag_render.rs', [
    'workbench_reveal_on_group_hover(', 'workbench_visible(',
  ]],
  ['floating and media messages use shared surfaces', 'crates/fileatlas-plugin-workcrate-ui/src/view/projects.rs', [
    'components::menu_panel(',
  ]],
  ['FileManager status notice uses the shared toast component', 'crates/fileatlas-plugin-file-manager/src/view/regions.rs', [
    'components::status_toast(',
  ]],
  ['unavailable thumbnails use a shared overlay component', 'crates/fileatlas-plugin-file-staging/src/view.rs', [
    'components::thumbnail_message_overlay(',
  ]],
  ['history timeline uses a shared rail component', 'crates/fileatlas-plugin-file-manager/src/view/inspector/history_tree.rs', [
    'components::timeline_rail(',
  ]],
];

const failures = [];
for (const [label, file, needles] of checks) {
  const source = read(file);
  for (const needle of needles) {
    if (!source.includes(needle)) failures.push(`${file}: ${label} missing ${needle}`);
  }
}

const sharedComponents = read('crates/fileatlas-ui-bridge/src/components.rs');
for (const api of ['pub enum SurfaceRole', 'pub enum CornerRole', 'pub trait WorkbenchPaint', 'pub enum ForegroundRole', 'pub trait WorkbenchForeground', 'pub enum StatusTone', 'pub fn status_dot', 'pub fn status_badge', 'pub fn status_toast', 'pub fn menu_panel', 'pub fn thumbnail_message_overlay', 'pub fn timeline_rail', 'pub fn media_badge', 'pub fn thumbnail_frame', 'pub fn media_hover_overlay', 'pub fn tree_guide_dots', 'pub fn range_slider_visual', 'workbench_reveal_on_group_hover']) {
  if (!sharedComponents.includes(api)) failures.push(`UI Bridge shared paint API missing ${api}`);
}
if (!sharedComponents.includes('fn audited_input(control: impl IntoElement) -> Div') ||
    !sharedComponents.includes('frame.child(audit_overlay(theme::AUDIT_FRAME_INPUT, 2.))')) {
  failures.push('shared single-line and multiline inputs must use the paint-only audit overlay');
}

// Modal backdrops and surfaces are host components, never plugin paint recipes.
const pluginSources = [];
for (const entry of fs.readdirSync(path.join(root, 'crates'), { withFileTypes: true })) {
  if (!entry.isDirectory() || !entry.name.startsWith('fileatlas-plugin-') || entry.name === 'fileatlas-plugin-mcp') continue;
  const src = path.join(root, 'crates', entry.name, 'src');
  const walk = dir => {
    for (const child of fs.readdirSync(dir, { withFileTypes: true })) {
      const file = path.join(dir, child.name);
      if (child.isDirectory()) walk(file);
      else if (child.isFile() && file.endsWith('.rs')) pluginSources.push(file);
    }
  };
  if (fs.existsSync(src)) walk(src);
}
for (const file of pluginSources) {
  const source = fs.readFileSync(file, 'utf8');
  if (/theme::scrim\s*\(/.test(source)) failures.push(`${path.relative(root, file)}: plugin must use components::modal_overlay instead of painting a modal scrim`);
  if (/theme::modal_rgba\s*\(/.test(source)) failures.push(`${path.relative(root, file)}: plugin must use a shared surface instead of painting modal rgba`);
}

if (failures.length) {
  console.error(failures.join('\n'));
  process.exitCode = 1;
} else {
  console.log(`UI geometry audit coverage: passed (${checks.length} shared component groups)`);
}
