const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

module.exports = async ({ call, payload, profile, output, delay }) => {
  const invoke = async (tool, args = {}) => payload(await call(tool, args));
  await delay(1800);
  await invoke('control_ui', { action: 'bulk_actions' });
  await delay(350);
  const state = await invoke('get_ui_state');
  assert(state.file_manager, 'FileManager window did not publish its live state');
  assert.notEqual(state.file_manager.bulk_action, null, 'bulk operation panel did not open');
  const screenshot = await call('screenshot_window');
  const image = screenshot.content.find(item => item.type === 'image');
  assert(image, 'FileAtlas MCP did not return a screenshot');
  fs.writeFileSync(path.join(output, 'file-operations-panel.png'), Buffer.from(image.data, 'base64'));
  const responsive = await invoke('get_ui_state');
  assert.equal(responsive.file_manager.bulk_action, state.file_manager.bulk_action);
  fs.writeFileSync(path.join(profile, 'file-operations-report.json'), JSON.stringify({
    panel_action: state.file_manager.bulk_action,
    mcp_responsive_after_open: true,
    native_save_dialog_interaction: 'not covered; MCP does not expose OS dialog controls',
  }, null, 2));
};
