// Same-process native PhoTag MCP theme/layout regression; isolated profile only.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
module.exports = async ({step, delay, profile, root}) => {
  const invoke = async (tool, args = {}, extra = {}) =>
    (await step({tool, arguments: args, ...extra})).result;
  const state = () => invoke('get_ui_state');
  const ui = (action, value) => invoke('control_ui', {action, ...(value === undefined ? {} : {value})}, {wait_ms: 250});
  const shot = name => invoke('get_ui_state', {}, {capture: name});
  const click = async (x, y) => {
    for (const action of ['pointer_move', 'pointer_down', 'pointer_up']) {
      for (let retry = 0; ; retry++) {
        try { await ui(action, `${x},${y}`); break; }
        catch (error) {
          if (retry >= 5 || !String(error).includes('pointer request is still pending')) throw error;
          await delay(250);
        }
      }
    }
  };
  const until = async check => {
    for (let n = 0; n < 80; n++) { const s = await state(); if (check(s)) return s; await delay(200); }
    throw Error('theme fixture state timed out');
  };
  await delay(1500);
  let s = await state();
  assert.equal(s.theme.id, 'workbench-navy');
  assert.equal(s.theme.accent, 0x2787ff);
  assert.equal(s.theme.selection, 0x163b63);
  assert.equal(s.theme.base, 0x0e1823);
  assert.equal(s.theme.body_size, 14);
  await invoke('control_gallery', {action:'navigate', value:path.join(root,'test-data/photag-library/02-mixed-formats')});
  s = await until(s => s.gallery?.matched_count > 0 && !s.browser.worker_active);
  const media = (await invoke('search_media', {})).items;
  if (media.length) await invoke('control_gallery', {action:'select_many', media_ids:[media[0].id]}, {wait_ms:500});
  await shot('main');
  await ui('inspector_tab', 'history');
  await shot('history');
  await ui('inspector_tab', 'information');
  await ui('statistics');
  await until(s => s.gallery?.dialog_reads?.statistics?.state === 'ready');
  await shot('statistics');
  await ui('gallery_dialogs_close');
  assert.equal((await state()).gallery.statistics_open,false);
  await ui('bulk_actions');
  await until(s => s.gallery?.dialog_reads?.bulk?.state === 'ready');
  await shot('bulk-actions');
  await ui('gallery_dialogs_close');
  await ui('export_open');
  await shot('export');
  await ui('export_close');

  for (const content of ['统一主题验证：卡片、导航、按钮和输入框共用工作台样式。', 'https://example.org/theme', '{"theme":"workbench-navy"}'])
    await invoke('add_clipboard_text', {content});
  await ui('panel_open','clipboard-workbench/panel');
  s = await shot('clipboard');
  let panel = s.panel_instances.find(p => p.id === s.active_panel);
  const clipboardPanel = panel.id;
  let [x,y,w,h] = panel.bounds.split(',').map(Number);
  await click(x+110,y+142);
  await ui('input_text','没有匹配的主题测试内容');
  assert.equal((await invoke('get_clipboard_state')).ui.visible_ids.length,0);
  await shot('clipboard-empty-focus');
  await ui('key_press','ctrl-a'); await ui('key_press','backspace');
  await ui('pointer_move', `${x+400},${y+190}`);
  await shot('clipboard-selected-hover');
  await ui('pointer_move', `${x+400},${y+300}`);
  await shot('clipboard-hover');
  await click(x+w-376,y+80);
  await shot('clipboard-menu');
  await ui('key_press','escape');
  await ui('panel_close', clipboardPanel);

  // Add this build as an icon fixture; do not execute it.
  await invoke('add_launcher_program', {path:path.join(root,'target/debug/photag-shell.exe')});
  await ui('panel_open','program-launcher/panel');
  for (let n=0;n<160;n++) {
    const launcher = await invoke('get_launcher_state');
    if(launcher.ui && !launcher.scanning && !launcher.ui.icons_pending) break;
    await delay(250);
  }
  s = await shot('launcher');
  panel = s.panel_instances.find(p => p.id === s.active_panel);
  [x,y,w,h] = panel.bounds.split(',').map(Number);
  await click(x+420,y+72); await ui('input_text','photag-shell');
  await delay(500);
  await click(x+292,y+235);
  const launcherSelected = await invoke('get_launcher_state');
  assert.equal(launcherSelected.ui.selected_entry,launcherSelected.ui.visible_ids[0]);
  await shot('launcher-selected-focus');
  await delay(700);
  const launcherQuiet = await invoke('get_launcher_state');
  await delay(900);
  assert.equal((await invoke('get_launcher_state')).ui.render_count,launcherQuiet.ui.render_count,
    'settled launcher must not repaint from theme polling');
  const launcherPanel = s.active_panel;
  await ui('panel_close',launcherPanel);

  await ui('settings_open'); await ui('settings_tab','appearance');
  s = await shot('settings-appearance');
  const [sx,sy,sw,sh] = s.panels.find(p => p.id === 'settings').bounds.split(',').map(Number);
  // Coordinates are measured from the existing Settings horizontal layout.
  // Appearance title + language contribution + heading precede the swatches.
  fs.writeFileSync(path.join(profile,'theme-geometry.json'),JSON.stringify({sx,sy,sw,sh,scale:s.window_status.geometry.scale_factor},null,2));
  // Measured from the first same-process screenshot: first swatch center (198,202), row step 35.33.
  await click(sx+198,sy+273);
  assert.equal((await state()).theme.accent,0x5a8a88,'real appearance swatch must change the global accent');
  await shot('settings-teal');
  await ui('settings_close');
  await ui('panel_open','clipboard-workbench/panel');
  await shot('clipboard-teal');
  await ui('panel_close',(await state()).active_panel);
  await ui('panel_open','program-launcher/panel');
  await shot('launcher-teal');
  await ui('panel_close',(await state()).active_panel);
  await ui('settings_open'); await ui('settings_tab','appearance');
  await click(sx+198,sy+202);
  assert.equal((await state()).theme.accent,0x2787ff,'restore shared default accent');
  for (const tab of ['shortcuts','tags','ai','updates','software']) {
    await ui('settings_tab',tab); await delay(400); await shot(`settings-${tab}`);
  }
  await ui('settings_close');
  s = await state();
  const before = {gallery:s.gallery.snapshot_revision, clipboard:(await invoke('get_clipboard_state')).revision};
  await delay(1100);
  s = await state();
  assert.equal(s.gallery.snapshot_revision,before.gallery,'quiet theme must not requery Gallery');
  assert.equal((await invoke('get_clipboard_state')).revision,before.clipboard,'quiet theme must not mutate clipboard');
  await invoke('resize_window',{width:1510,height:1000},{wait_ms:600});
  await ui('settings_open'); await ui('settings_tab','shortcuts');
  await shot('compact-settings');
  await ui('settings_close');
  await ui('panel_open','clipboard-workbench/panel');
  s = await shot('compact-clipboard');
  panel = s.panel_instances.find(p => p.id === s.active_panel);
  [x,y,w,h] = panel.bounds.split(',').map(Number);
  await ui('pointer_scroll',`${x+160},${y+440},-600`);
  await shot('compact-clipboard-scrolled');
  fs.writeFileSync(path.join(profile,'theme-summary.json'),JSON.stringify({passed:true,
    checks:['shared tokens','main','history','statistics','bulk','export','clipboard search/empty/focus','launcher','all settings tabs','quiet revisions','compact settings/clipboard','real swatch accent switch across settings/clipboard/launcher','selected and normal card hover','clipboard menu'],
    limitations:['native OS dialogs not invoked'],
    scale:s.window_status.geometry.scale_factor},null,2));
};
