const assert = require('node:assert/strict');

module.exports = async ({step}) => {
  await step({tool:'get_ui_state',until:{field:'window_status.valid',equals:true,attempts:60,interval_ms:400}});
  const state = await step({tool:'get_ui_state'});
  assert.equal(state.result.settings_plugins.mcp_access_level, 'off', 'A fresh profile must default to MCP Off');
  await step({tool:'clipboard.add_text',arguments:{content:'must be blocked'},expect_error:'Standard control'});
};
