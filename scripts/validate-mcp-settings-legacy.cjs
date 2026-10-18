const assert = require('node:assert/strict');

module.exports = async ({step}) => {
  await step({tool:'get_ui_state',until:{field:'window_status.valid',equals:true,attempts:60,interval_ms:400}});
  const state = await step({tool:'get_ui_state'});
  assert.equal(state.result.settings_plugins.mcp_access_level, 'full', 'Legacy true must retain its former all-mutations permission');
  await step({tool:'file-operations.rename_media',arguments:{media_id:9223372036854770000,new_filename:'noop.jpg'},expect_error:'confirm=true'});
};
