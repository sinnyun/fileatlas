const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

module.exports = async ({call, step, delay, profile}) => {
  const invoke = async (tool, arguments_ = {}, options = {}) =>
    (await step({tool, arguments: arguments_, ...options})).result;
  await step({tool:'get_ui_state',until:{field:'window_status.valid',equals:true,attempts:60,interval_ms:400}});
  await invoke('control_ui',{action:'settings_open'});
  await invoke('control_ui',{action:'settings_plugin_page',value:'fileatlas.mcp'});
  await delay(700);
  const initial = await invoke('get_ui_state');
  assert.equal(initial.settings_plugins.mcp_access_level, 'full');
  const shot = await call('screenshot_window');
  const image = shot.content.find(item=>item.type==='image');
  assert(image, 'MCP screenshot missing');
  fs.writeFileSync(path.join(profile,'screenshots','mcp-access-full.png'),Buffer.from(image.data,'base64'));
  fs.writeFileSync(path.join(profile,'state-full.json'),JSON.stringify(initial,null,2));
  await step({tool:'file-operations.rename_media',arguments:{media_id:9223372036854770000,new_filename:'noop.jpg'},expect_error:'confirm=true'});
  await step({tool:'file-operations.rename_media',arguments:{media_id:9223372036854770000,new_filename:'noop.jpg',confirm:true},expect_error:'not indexed'});
  const click = async (x,y) => {
    const point = `${x},${y}`;
    await invoke('control_ui',{action:'pointer_down',value:point},{wait_ms:350});
    await invoke('control_ui',{action:'pointer_up',value:point},{wait_ms:350});
    await delay(500);
  };
  // Convert screenshot pixels to logical client coordinates using window_status.geometry.
  await click(1448,506);
  const standard = await invoke('get_ui_state');
  assert.equal(standard.settings_plugins.mcp_access_level, 'standard');
  await invoke('clipboard.add_text',{content:'standard access stage'});
  await step({tool:'file-operations.rename_media',arguments:{media_id:9223372036854770000,new_filename:'noop.jpg',confirm:true},expect_error:'Full control'});
  const standardShot = await call('screenshot_window');
  const standardImage = standardShot.content.find(item=>item.type==='image');
  fs.writeFileSync(path.join(profile,'screenshots','mcp-access-standard.png'),Buffer.from(standardImage.data,'base64'));
  fs.writeFileSync(path.join(profile,'state-standard.json'),JSON.stringify(standard,null,2));
  await click(1367,506);
  const off = await invoke('get_ui_state');
  assert.equal(off.settings_plugins.mcp_access_level, 'off');
  await step({tool:'clipboard.add_text',arguments:{content:'must be blocked'},expect_error:'Standard control'});
  const offShot = await call('screenshot_window');
  const offImage = offShot.content.find(item=>item.type==='image');
  fs.writeFileSync(path.join(profile,'screenshots','mcp-access-off.png'),Buffer.from(offImage.data,'base64'));
  fs.writeFileSync(path.join(profile,'state-off.json'),JSON.stringify(off,null,2));
};
