const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

module.exports = async ({call, payload, step, delay, profile}) => {
  const invoke = async (tool, arguments_ = {}, options = {}) =>
    (await step({tool, arguments: arguments_, ...options})).result;
  await step({tool:'get_ui_state',until:{field:'window_status.valid',equals:true,attempts:60,interval_ms:400}});
  await invoke('control_ui',{action:'settings_open'});
  await invoke('control_ui',{action:'settings_plugin_page',value:'fileatlas.mcp'});
  await delay(600);
  const initial = await invoke('get_ui_state');
  assert.equal(initial.settings_plugins.mcp_allow_mutations, true);
  const shot = await call('screenshot_window');
  const image = shot.content.find(item=>item.type==='image');
  assert(image, 'MCP screenshot missing');
  fs.writeFileSync(path.join(profile,'screenshots','mcp-setting-enabled.png'),Buffer.from(image.data,'base64'));
  fs.writeFileSync(path.join(profile,'state-enabled.json'),JSON.stringify(initial,null,2));
  await invoke('control_ui',{action:'pointer_down',value:'1600,503'},{wait_ms:350});
  await invoke('control_ui',{action:'pointer_up',value:'1600,503'},{wait_ms:350});
  await delay(500);
  const disabled = await invoke('get_ui_state');
  assert.equal(disabled.settings_plugins.mcp_allow_mutations, false, 'Settings toggle should immediately disable MCP modifications');
  await step({tool:'file_manager.set_rating',arguments:{media_ids:[1],rating:1},expect_error:'Settings → MCP'});
  fs.writeFileSync(path.join(profile,'state-disabled.json'),JSON.stringify(disabled,null,2));
  const blocked = await call('screenshot_window');
  const disabledImage = blocked.content.find(item=>item.type==='image');
  fs.writeFileSync(path.join(profile,'screenshots','mcp-setting-disabled.png'),Buffer.from(disabledImage.data,'base64'));
};
