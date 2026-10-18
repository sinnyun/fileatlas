// Same-process settings concepts/schema regression. Isolated dev-test only.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
module.exports = async ({step, delay, profile}) => {
  const invoke = async (tool, args = {}, extra = {}) => (await step({tool, arguments:args, ...extra})).result;
  const state = () => invoke('get_ui_state');
  const ui = (action,value) => invoke('control_ui',{action,...(value===undefined?{}:{value})},{wait_ms:250});
  const shot = name => invoke('get_ui_state',{}, {capture:name});
  await step({tool:'get_ui_state',until:{field:'window_status.valid',equals:true,attempts:60,interval_ms:400}});
  await invoke('resize_window',{width:1890,height:1410},{wait_ms:400});
  await ui('settings_open');
  await ui('settings_tab','shortcuts');
  await step({tool:'get_ui_state',until:{field:'settings_tag_choices.state',equals:'ready',attempts:40,interval_ms:200}});
  const existing=(await state()).settings_tag_choices.choices.map(tag=>tag.name);
  for(const name of ['人物','风景','室内','室外']) if(!existing.includes(name)) await invoke('create_tag',{name});
  await step({tool:'get_ui_state',until:{field:'settings_tag_choices.choices.length',at_least:4,attempts:40,interval_ms:200}});
  for (const tab of ['appearance','shortcuts','tags','ai','updates','software','plugins']) {
    await ui('settings_tab',tab); await delay(400); await shot(tab);
  }
  for (const id of ['photag.launcher','photag.clipboard','photag.screenshot']) {
    await ui('settings_plugin_page',id); await delay(400); await shot(id.split('.')[1]);
  }
  const before = await state();
  await delay(1200);
  const after = await state();
  assert.equal(after.gallery.snapshot_revision,before.gallery.snapshot_revision);
  assert.equal(after.settings_render_count,before.settings_render_count,"settled Settings must not repaint on timer wakes");
  assert.deepEqual(after.settings_tag_choices,before.settings_tag_choices);
  await invoke('resize_window',{width:1510,height:1000},{wait_ms:500});
  await shot('compact-screenshot');
  await ui('settings_tab','shortcuts'); await shot('compact-shortcuts');
  const panel=(await state()).panels.find(p=>p.id==='settings');
  const [x,y,w,h]=panel.bounds.split(',').map(Number);
  await ui('pointer_scroll',`${x+w-100},${y+h-100},-900`); await shot('compact-shortcuts-scrolled');
  fs.writeFileSync(path.join(profile,'settings-summary.json'),JSON.stringify({same_process:true,quiet_state:true,pages:10,scale:after.window_status.geometry.scale_factor},null,2));
};
