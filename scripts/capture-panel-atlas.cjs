// Capture every FileAtlas panel/screen through the same-process FileAtlas MCP.
//
// Phases (PANEL_ATLAS_PHASE, comma separated; each phase is a separate MCP child so a
// crash in one screen cannot lose the others):
//   browser  seeding + main window/sidebar/inspector/tools/popovers/grouping/columns
//   settings Settings tabs + plugin pages
//   modals   workspace manager, metadata export, update check, navigation tag dialogs
//   panels   floating PanelCatalog panels + WorkCrate + debug drawer + combined state
//   all      every phase in one process (only useful while nothing crashes the app)
//
// Set PANEL_ATLAS_WORKCRATE_EDIT=1 to additionally build WorkCrate content through the
// official session (create pages, refresh previews, place an image, edit material fields).
// Those calls go through official_session, so while the user's VectorCraft GUI listens on
// the configured control port they drive that running window and switch its document.
//
// Run one phase per invocation, e.g.
//   PANEL_ATLAS_PHASE=browser node scripts/fileatlas-mcp-debug.cjs \
//     --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/<run> \
//     --plugins dev-test --access-level full --scenario scripts/capture-panel-atlas.cjs
// Screenshots land in <profile>/screenshots; manifest.json records every step outcome.
const fs = require('node:fs');
const path = require('node:path');

const PHASE = (process.env.PANEL_ATLAS_PHASE || 'all').toLowerCase();
const has = section => PHASE === 'all' || PHASE.split(',').map(s => s.trim()).includes(section);

module.exports = async ({ call, payload, delay, profile, output, root }) => {
  const manifest = fs.existsSync(path.join(profile, 'manifest.json'))
    ? JSON.parse(fs.readFileSync(path.join(profile, 'manifest.json'), 'utf8'))
    : [];
  let current = null;
  let dead = false;
  const flush = () =>
    fs.writeFileSync(path.join(profile, 'manifest.json'), JSON.stringify(manifest, null, 2));

  async function record(name, note, status = 'pending') {
    current = { name, note, status };
    manifest.push(current);
    flush();
  }
  function noteFailure(error) {
    const text = String(error?.message || error);
    current.error = text.slice(0, 240);
    // A dead child answers nothing; stop burning 30s timeouts for the remaining screens.
    if (text.includes('MCP timeout') || text.includes('Broken pipe')) dead = true;
  }
  async function shot(name, note, require3000x2000 = false) {
    if (dead) return record(name, note, 'skipped-process-dead');
    await record(name, note);
    for (let attempt = 0; attempt < 3; attempt++) {
      try {
        const captured = await call('screenshot_window');
        const image = (captured.content || []).find(item => item.type === 'image');
        if (image) {
          const bytes = Buffer.from(image.data, 'base64');
          const dimensions = { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) };
          if (require3000x2000 && (dimensions.width !== 3000 || dimensions.height !== 2000)) {
            throw Error(`expected 3000x2000 screenshot; received ${dimensions.width}x${dimensions.height}`);
          }
          fs.writeFileSync(path.join(output, `${name}.png`), bytes);
          current.status = 'captured';
          current.screenshot_dimensions = dimensions;
          try {
            const geometry = payload(await call('debug.ui_geometry'));
            fs.writeFileSync(
              path.join(output, `${name}.geometry.json`),
              JSON.stringify(geometry, null, 2),
            );
            current.geometry_count = geometry?.count ?? 0;
          } catch (error) {
            current.geometry_error = String(error?.message || error).slice(0, 240);
          }
          flush();
          return true;
        }
      } catch (error) {
        noteFailure(error);
        if (dead) break;
      }
      await delay(500);
    }
    if (!dead) current.status = 'failed';
    flush();
    return false;
  }
  async function act(name, tool, args, waitMs = 400) {
    if (dead) return record(name, `call ${tool}`, 'skipped-process-dead'), null;
    await record(name, `call ${tool}`);
    try {
      const result = payload(await call(tool, args || {}));
      current.status = 'done';
      current.result = summarize(result);
      flush();
      await delay(waitMs);
      return result;
    } catch (error) {
      current.status = 'error';
      noteFailure(error);
      flush();
      return null;
    }
  }
  const close = (name, tool, args) => act(name, tool, args, 300);
  const dismiss = async name => {
    await act(`${name}-escape`, 'control_ui', { action: 'key_press', value: 'escape' }, 500);
    return act(`${name}-close`, 'control_navigation', { action: 'close' }, 400);
  };

  await shot('000-boot-window', 'wait for the first GPUI frame');
  // The window is not addressable the instant the child starts; retry the resize until
  // the native handle exists, then re-shoot so the atlas is captured at full size.
  let resized = false;
  for (let attempt = 0; attempt < 20 && !resized; attempt++) {
    await record(`resize-3000x2000-attempt-${attempt}`, 'wait for a native window handle');
    try {
      await call('resize_window', { width: 3000, height: 2000 });
      current.status = 'done';
      resized = true;
    } catch (error) {
      noteFailure(error);
      await delay(1500);
    }
    flush();
  }
  if (!dead) await delay(1500);
  await shot('001-boot-window-3000x2000', 'main window after resize', true);

  const fixtureRoot = path.join(root, 'test-data', 'fileatlas-fixtures');
  const fixtureFiles = collectFiles(fixtureRoot, 20);
  let mediaIds = [];
  let imageIds = [];
  async function rows(attempts) {
    for (let attempt = 0; attempt < attempts && mediaIds.length === 0; attempt++) {
      // A bare query only returns the live pane's own directory, so seed detection uses a
      // path term that matches every fixture row.
      const value = payload(
        await call('file_manager.search_media', { limit: 60, search: 'fileatlas-fixtures' }),
      );
      mediaIds = extractIds(value);
      imageIds = (value?.items || [])
        .filter(item => item.media_type === 'image')
        .map(item => item.id);
      if (mediaIds.length === 0) await delay(1500);
    }
    return mediaIds;
  }

  // ---- seeding -------------------------------------------------------------
  if (has('browser')) {
    await act('seed/scan-fixture', 'dev-test.scan_fixture', {}, 1500);
    for (let attempt = 0; attempt < 30; attempt++) {
      const status = payload(await call('dev-test.status'));
      if (status && !status.running && status.task?.running !== true) break;
      await delay(1000);
    }
    await rows(30);
    await act('seed/rows-ready', 'dev-test.status', {}, 400);
    if (mediaIds.length === 0) await record('seed/media-ids', 'no indexed rows after fixture scan', 'warn');

    const tagIds = [];
    for (const name of ['设计资产', '投放渠道']) {
      const created = await act(`seed/tag-${name}`, 'navigation.create_tag', { name });
      const id = created?.tag?.id ?? created?.id;
      if (typeof id === 'number') tagIds.push(id);
    }
    if (tagIds.length > 0) {
      const child = await act('seed/tag-child', 'navigation.create_tag', { name: '主题背景', parent_id: tagIds[0] });
      const childId = child?.tag?.id ?? child?.id;
      if (typeof childId === 'number') tagIds.push(childId);
      if (mediaIds.length > 0) {
        await act('seed/assign-tags', 'tagging.assign', { media_ids: mediaIds.slice(0, 6), tag_ids: tagIds }, 900);
      }
    }
    for (const [index, text] of clipboardTexts.entries()) {
      await act(`seed/clipboard-${index}`, 'clipboard.add_text', { content: text });
    }
    await act('seed/staging', 'file-staging.add', { paths: fixtureFiles.slice(0, 6) }, 800);
  } else {
    await rows(12);
  }
  // The fixtures root holds only README.txt, so every browser screen would show one row.
  // Show descendants to put the real media files (with cached thumbnails) in the pane.
  await act('seed/show-descendants', 'control_file_manager', { action: 'show_descendants', enabled: true }, 2500);
  const seedTags = payload(await call('tagging.list_tags'));
  const firstTagId = extractIds(seedTags)[0];
  const firstMediaId = imageIds[0] ?? mediaIds[0];


  // ---- browser: main window, sidebar, inspector, tools, popovers ----------
  if (has('browser')) {
    // Remember the pane's own directory string (the app canonicalizes paths with a \\?\
    // prefix) so navigation screens can put it back afterwards.
    const bootUi = payload(await call('get_ui_state'));
    const rootDir = bootUi?.file_manager?.current_directory;
    const folderCards = bootUi?.file_manager?.directories ?? [];
    const contentFolder =
      folderCards.find(card => /02-mixed/.test(card.name || '')) ?? folderCards[0];

    await shot('100-main-single-folders', 'single column, folders sidebar, indexed rows');
    await act('sidebar/tags', 'control_ui', { action: 'navigation_tab', value: 'tags' }, 900);
    await shot('101-sidebar-tags', 'sidebar tag tree with nested tag');
    await act('sidebar/extensions', 'control_ui', { action: 'navigation_tab', value: 'extensions' }, 900);
    await shot('102-sidebar-extensions', 'sidebar extensions/project-tree page');
    await act('sidebar/folders', 'control_ui', { action: 'navigation_tab', value: 'folders' }, 900);

    if (firstMediaId !== undefined) {
      await act('inspector/close-for-selection', 'control_ui', { action: 'inspector_close' }, 800);
      await act('select/file', 'control_ui', { action: 'select_media', value: String(firstMediaId) }, 1500);
      await shot('110-file-selected-no-inspector', 'selection highlight only');
    }
    await act('inspector/open', 'control_ui', { action: 'inspector_open' }, 900);
    await act('inspector/tab-information', 'control_ui', { action: 'inspector_tab', value: 'information' }, 1200);
    await shot('111-inspector-file-information', 'file context, information tab');
    await act('inspector/tab-history', 'control_ui', { action: 'inspector_tab', value: 'history' }, 1200);
    await shot('112-inspector-file-history', 'history tab (Lore versions)');
    await act('inspector/tab-extensions', 'control_ui', { action: 'inspector_tab', value: 'extensions' }, 1200);
    await shot('113-inspector-file-extensions', 'add-ons tab');
    await act('inspector/tab-information-again', 'control_ui', { action: 'inspector_tab', value: 'information' }, 900);
    if (contentFolder?.path) {
      await act('inspector/context-folder', 'control_navigation', { action: 'select_path', value: contentFolder.path }, 1800);
      await shot('114-inspector-folder', 'folder inspector context');
    }
    await act('inspector/close', 'control_ui', { action: 'inspector_close' }, 700);
    if (rootDir) {
      await act('browser/restore-root', 'control_file_manager', { action: 'navigate', value: rootDir }, 2200);
      await act('browser/re-show-descendants', 'control_file_manager', { action: 'show_descendants', enabled: true }, 2200);
    }

    await act('tools/open', 'control_file_manager', { action: 'tools_open' }, 1200);
    await shot('120-tools-panel', 'per-column tools region: filters + bulk + statistics');
    await act('tools/statistics', 'control_ui', { action: 'statistics' }, 1400);
    await shot('121-statistics', 'library statistics dialog');
    await close('tools/statistics-close', 'control_ui', { action: 'file_manager_dialogs_close' });
    await act('tools/bulk', 'control_file_manager', { action: 'bulk_open' }, 1300);
    await shot('122-bulk-actions', 'bulk action configuration panel');
    await close('tools/bulk-close', 'control_file_manager', { action: 'bulk_close' });
    await close('tools/close', 'control_file_manager', { action: 'tools_close' });

    for (const [index, kind] of popovers.entries()) {
      await act(`popover/${kind}`, 'control_file_manager', { action: 'popover', value: kind }, 1000);
      await shot(`13${index}-popover-${kind}`, 'top-bar filter popover');
    }
    await close('popover/close', 'control_file_manager', { action: 'popover', value: 'close' });
    await act('search/open', 'control_file_manager', { action: 'search', value: 'fixtures' }, 2200);
    await shot('140-search-results', 'filename/path search with results');
    await act('search/scopes', 'control_file_manager', { action: 'search_scopes_open', enabled: true }, 1200);
    const scopeState = payload(await call('get_ui_state'));
    await record('search/scopes-state', 'search_scopes_open flag after the request');
    current.status = 'done';
    current.result = { search_scopes_open: scopeState?.file_manager?.search_scopes_open };
    flush();
    await shot('141-search-scope-panel', 'search scope chooser');
    await close('search/scopes-close', 'control_file_manager', { action: 'search_scopes_open', enabled: false });
    await close('search/clear', 'control_file_manager', { action: 'search', value: '' });

    if (firstMediaId !== undefined) {
      await act('menu/file', 'control_file_manager', { action: 'open_media_menu', media_id: firstMediaId }, 1200);
      await shot('150-file-context-menu', 'right-click actions for one file');
      await close('menu/file-close', 'control_file_manager', { action: 'close_media_menu' });
    }
    await act('group/file-type', 'control_file_manager', { action: 'grouping', value: 'file_type' }, 1800);
    await shot('151-grouped-by-file-type', 'file type group headers in the browser');
    await act('group/month', 'control_file_manager', { action: 'grouping', value: 'month' }, 1800);
    await shot('152-grouped-by-month', 'month group headers');
    await close('group/off', 'control_file_manager', { action: 'grouping', value: 'none' });

    await act('columns/dual', 'control_ui', { action: 'columns_layout', value: 'dual' }, 2200);
    await shot('160-columns-dual', 'two independent browser columns');
    await act('columns/quad', 'control_ui', { action: 'columns_layout', value: 'quad' }, 2400);
    await shot('161-columns-quad', 'four browser columns');
    await act('columns/activate-2', 'control_ui', { action: 'activate_column', value: '2' }, 1200);
    await shot('162-columns-quad-active-pane', 'active column indicator');
    await act('columns/single', 'control_ui', { action: 'columns_layout', value: 'single' }, 1500);
  }

  // ---- Settings ------------------------------------------------------------
  if (has('settings')) {
    await act('settings/open', 'control_ui', { action: 'settings_open' }, 1400);
    for (const [file, tab] of settingsTabs) {
      await act(`settings/tab-${tab}`, 'control_ui', { action: 'settings_tab', value: tab }, 1300);
      await shot(file, `Settings tab: ${tab}`);
    }
    // Plugin settings pages are keyed by Cordis plugin id; try the known spellings and
    // keep the first one that resolves instead of failing the whole phase.
    for (const [file, candidates] of pluginSettingsPages) {
      let opened = false;
      for (const id of candidates) {
        const result = await act(`settings/plugin-page-${id}`, 'control_ui', { action: 'settings_plugin_page', value: id }, 1600);
        if (result) {
          opened = true;
          break;
        }
      }
      if (opened) await shot(file, `Settings plugin page (${file})`);
      else await record(`${file}-skipped`, 'no plugin settings page id resolved', 'warn');
    }
    await close('settings/close', 'control_ui', { action: 'settings_close' }, 900);
  }

  // ---- modals (navigation tag dialogs last: they crashed the app once) -----
  if (has('modals')) {
    await act('modal/workspace-manager', 'control_ui', { action: 'workspace_manager', value: 'open' }, 1500);
    await shot('620-workspace-manager', 'program workspace manager');
    await close('modal/workspace-manager-close', 'control_ui', { action: 'workspace_manager', value: 'close' });

    await act('modal/metadata-export', 'control_ui', { action: 'export_open' }, 1500);
    await shot('621-metadata-export', 'metadata export dialog');
    await close('modal/metadata-export-close', 'control_ui', { action: 'export_close' });

    await act('modal/folder-search', 'control_navigation', { action: 'folder_search', value: 'mixed' }, 1600);
    await shot('640-folder-search', 'sidebar folder search results');
    await dismiss('modal/folder-search');

    await act('modal/update-check', 'control_ui', { action: 'settings_check_updates' }, 2600);
    await shot('622-update-check', 'manual update check progress/offer');
    await close('modal/update-close', 'control_ui', { action: 'settings_close' });

    for (const [index, action] of navDialogs.entries()) {
      const args = { action };
      if (action !== 'create' && firstTagId) args.tag_id = firstTagId;
      await act(`modal/nav-${action}`, 'control_navigation', args, 1500);
      await shot(`63${index}-navigation-${action}-dialog`, `Navigation ${action} tag dialog`);
      await dismiss(`modal/nav-${action}`);
      if (dead) break;
    }
  }

  // ---- floating panels + WorkCrate + debug --------------------------------
  if (has('panels')) {
    // Panels left open by an earlier phase (or a panel whose close needs an instance id
    // we could not read) would overlap every following frame.
    const closeAllPanels = async label => {
      const state = payload(await call('get_ui_state'));
      const ids = (state?.panel_instances ?? []).map(entry => entry.id);
      for (const id of ids) {
        await act(`${label}/close-${id}`, 'control_ui', { action: 'panel_close', value: id }, 250);
      }
      return ids;
    };
    // panel_close takes the instance id panel_open returned, not the definition id.
    const openPanel = async (file, id, note, openMs = 1800) => {
      const opened = await act(`panel/open-${id}`, 'control_ui', { action: 'panel_open', value: id }, openMs);
      await shot(file, note);
      if (opened?.instance_id) {
        await close(`panel/close-${id}`, 'control_ui', { action: 'panel_close', value: opened.instance_id });
      } else {
        await closeAllPanels(`panel/sweep-${id}`);
      }
      return opened;
    };
    await closeAllPanels('panels/before');
    for (const [file, id] of panels) await openPanel(file, id, `floating panel ${id}`);
    await openPanel('703-panel-screenshot-tool', 'screenshot-tool/panel', 'modal screenshot tool panel');

    // WorkCrate. create_page/refresh_project/open_board/place_image/update_page/
    // export_pages go through official_session: with the user's VectorCraft GUI
    // listening on the control port they drive that running window and switch it to
    // this sweep's test document (explicitly authorized for this run).
    const projectDir = path.join(profile, 'workcrate-project');
    fs.mkdirSync(projectDir, { recursive: true });
    const created = await act('workcrate/create-project', 'workcrate-ui.manage_project', {
      action: 'create',
      name: '截图巡检工程',
      directory: projectDir,
    }, 4000);
    const state = payload(await call('workcrate-ui.state')) ?? {};
    const projectId = created?.project?.id ?? state?.projects?.[0]?.id;
    const pageIds = [];
    await closeAllPanels('workcrate/before');
    const workcrateOpened = await act('panel/open-workcrate', 'control_ui', { action: 'panel_open', value: 'workcrate/overview' }, 2600);
    const workcrateInstance = workcrateOpened?.instance_id;
    await shot('800-workcrate-projects', 'WorkCrate tier 1: project management');
    if (projectId) {
      if (process.env.PANEL_ATLAS_WORKCRATE_EDIT === '1') {
        for (const index of [0, 1]) {
          const page = await act(`workcrate/create-page-${index}`, 'workcrate-ui.create_page', { project_id: projectId }, 4000);
          if (page?.board_id) pageIds.push(page.board_id);
        }
        await act('workcrate/refresh', 'workcrate-ui.refresh_project', { project_id: projectId }, 5000);
        const firstBoard = pageIds[0] ?? findBoardId(payload(await call('workcrate-ui.state', { project_id: projectId })));
        if (firstBoard) {
          await act('workcrate/open-board', 'workcrate-ui.open_board', { project_id: projectId, board_id: firstBoard }, 3500);
          const image = fixtureFiles.find(file => /\.png$/i.test(file)) ?? fixtureFiles[0];
          if (image) {
            await act('workcrate/place-image', 'workcrate-ui.place_image', { project_id: projectId, board_id: firstBoard, path: image }, 5000);
            await shot('805-workcrate-placed-image', 'page after placing a source image');
          }
          await act('workcrate/update-page-fields', 'workcrate-ui.update_page', {
            project_id: projectId,
            board_id: firstBoard,
            info: {
              material_category: '主题背景',
              material: 'PVC 雪弗板',
              size: 'A3 横版',
              quantity: '2 套',
            },
          }, 2500);
        }
      }
      await act('workcrate/navigate-pages', 'workcrate-ui.navigate', { project_id: projectId }, 2500);
      await shot('801-workcrate-pages', 'WorkCrate tier 2: page matrix + material band');
      const board =
        pageIds[0] ?? findBoardId(payload(await call('workcrate-ui.state', { project_id: projectId })));
      if (board) {
        await act('workcrate/navigate-editor', 'workcrate-ui.navigate', { project_id: projectId, board_id: board }, 3000);
        await shot('804-workcrate-editor', 'WorkCrate tier 3: page editor screen');
        await act('workcrate/navigate-pages-again', 'workcrate-ui.navigate', { project_id: projectId }, 2500);
        await shot('802-workcrate-page-preview', 'page matrix cached native preview');
      }
      await openPanel(
        '803-workcrate-project-defaults',
        'workcrate/project-defaults',
        'modal: project default information form',
        2200,
      );
    }
    await close('panel/close-workcrate', 'control_ui', { action: 'panel_close', value: workcrateInstance ?? 'workcrate/overview' });

    await act('debug/open', 'debug.control', { action: 'open' }, 1400);
    await shot('900-debug-drawer', 'event/log debug drawer');
    await close('debug/close', 'debug.control', { action: 'close' });

    const comboClipboard = await act('combo/open-clipboard', 'control_ui', { action: 'panel_open', value: 'clipboard-workbench/panel' }, 1400);
    const comboLauncher = await act('combo/open-launcher', 'control_ui', { action: 'panel_open', value: 'program-launcher/panel' }, 1600);
    await shot('910-two-panels-over-main', 'two floating panels over the browser');
    await close('combo/close-clipboard', 'control_ui', { action: 'panel_close', value: comboClipboard?.instance_id ?? 'clipboard-workbench/panel' });
    await close('combo/close-launcher', 'control_ui', { action: 'panel_close', value: comboLauncher?.instance_id ?? 'program-launcher/panel' });
  }

  const ui = payload(await call('get_ui_state'));
  fs.writeFileSync(path.join(profile, `final-ui-state-${PHASE.replace(/[^a-z,]/g, '')}.json`), JSON.stringify(ui, null, 2));
  writeIndex(path.join(output, 'INDEX.md'), manifest, PHASE);
  console.log(
    JSON.stringify({
      phase: PHASE,
      captured: manifest.filter(item => item.status === 'captured').length,
      problems: manifest.filter(item => !['captured', 'done'].includes(item.status)).length,
      processDead: dead,
    }),
  );
};

const clipboardTexts = [
  '主题背景主视觉已定稿，输出 A3 横版。',
  'https://example.com/brand/guide',
  '物料清单：立牌 x2、指引牌 x10、吊旗 x20。',
  '备注：下周与渠道确认投放档期。',
];
const popovers = ['type', 'tags', 'extensions', 'date', 'rating', 'sort'];
const settingsTabs = [
  ['600-settings-appearance', 'appearance'],
  ['601-settings-shortcuts', 'shortcuts'],
  ['602-settings-tags', 'tags'],
  ['603-settings-ai', 'ai'],
  ['604-settings-updates', 'updates'],
  ['605-settings-software', 'software'],
  ['606-settings-plugins', 'plugins'],
  ['607-settings-plugin', 'plugin'],
];
// 'edit' is deliberately absent: opening the tag edit dialog panics GPUI's a11y tree
// ("Duplicate a11y node id", gpui-pre-0.3.8/src/window/a11y.rs:415) and kills the process.
const navDialogs = ['create', 'delete', 'merge', 'move'];
const pluginSettingsPages = [
  ['610-settings-page-workcrate', ['fileatlas.workcrate-ui', 'workcrate-ui']],
  ['611-settings-page-material-theme', ['fileatlas.material-theme', 'material-theme']],
  ['612-settings-page-mcp', ['fileatlas.mcp', 'mcp']],
  ['613-settings-page-i18n', ['fileatlas.i18n', 'i18n']],
];
const panels = [
  ['700-panel-clipboard-workbench', 'clipboard-workbench/panel'],
  ['701-panel-file-staging', 'file-staging/panel'],
  ['702-panel-program-launcher', 'program-launcher/panel'],
  ['704-panel-dev-test-status', 'dev-test/status'],
  ['705-panel-dev-test-status-multi', 'dev-test/status-multi'],
];

function summarize(result) {
  if (result === null || typeof result !== 'object') return undefined;
  const keys = Object.keys(result);
  return keys.length > 12 ? keys.slice(0, 12) : result;
}

function findBoardId(state) {
  const found = [];
  const visit = node => {
    if (Array.isArray(node)) node.forEach(visit);
    else if (node && typeof node === 'object') {
      if (typeof node.board_id === 'string') found.push(node.board_id);
      else if (typeof node.id === 'string' && (node.artboard_index !== undefined || node.title || node.name))
        found.push(node.id);
      else Object.values(node).forEach(visit);
    }
  };
  visit(state);
  return found[0];
}

function extractIds(value) {
  const found = [];
  const visit = node => {
    if (Array.isArray(node)) node.forEach(visit);
    else if (node && typeof node === 'object') {
      if (typeof node.media_id === 'number') found.push(node.media_id);
      else if (
        typeof node.id === 'number' &&
        (node.path || node.name || node.title || node.filename || node.full_path)
      )
        found.push(node.id);
      else Object.values(node).forEach(visit);
    }
  };
  visit(value);
  return [...new Set(found)];
}

function collectFiles(dir, limit) {
  const out = [];
  const stack = [dir];
  while (stack.length && out.length < limit) {
    for (const entry of readDir(stack.pop())) {
      if (entry.isDirectory) stack.push(entry.full);
      else if (/\.(jpe?g|png|webp|gif|bmp|mp4|mov|wav|mp3)$/i.test(entry.name)) out.push(entry.full);
      if (out.length >= limit) break;
    }
  }
  return out;
}

function readDir(dir) {
  try {
    return fs
      .readdirSync(dir, { withFileTypes: true })
      .map(entry => ({ name: entry.name, isDirectory: entry.isDirectory(), full: path.join(dir, entry.name) }));
  } catch {
    return [];
  }
}

function writeIndex(file, manifest, phase) {
  // Phases append, so a re-run can record the same screen twice; keep the latest entry.
  const latest = new Map();
  for (const item of manifest) latest.set(item.name, item);
  const entries = [...latest.values()];
  const problems = entries.filter(item => !['captured', 'done'].includes(item.status));
  const lines = ['# FileAtlas panel screenshot atlas', '', `Last phase run: ${phase}`, ''];
  let index = 0;
  for (const item of entries) {
    if (item.status !== 'captured') continue;
    index += 1;
    lines.push(`${String(index).padStart(2, '0')}. ![${item.name}](./${item.name}.png) **${item.name}** — ${item.note}`);
  }
  lines.push('', '## Non-captured steps', '');
  if (problems.length === 0) lines.push('_none_');
  for (const item of problems) lines.push(`- **${item.name}** — ${item.status}${item.error ? ` — ${item.error}` : ''}`);
  fs.writeFileSync(file, lines.join('\n') + '\n');
}
