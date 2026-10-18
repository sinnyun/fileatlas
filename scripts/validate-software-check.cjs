const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ call, payload, delay, profile, output }) => {
  const records = [];
  const callValue = async (tool, args = {}) => {
    const result = await call(tool, args);
    const value = payload(result);
    assert(!value?.error, `${tool} failed: ${JSON.stringify(value)}`);
    return value;
  };
  const getState = () => callValue('get_ui_state');
  const waitForInventory = async () => {
    let state;
    for (let attempt = 0; attempt < 80; attempt++) {
      state = await getState();
      if (state.settings_software_check?.state === 'ready') return state;
      await delay(100);
    }
    throw Error(`Software inventory did not become ready: ${JSON.stringify(state?.settings_software_check)}`);
  };

  await delay(1200);
  await callValue('control_ui', { action: 'settings_open' });
  await callValue('control_ui', { action: 'settings_tab', value: 'software' });
  const state = await waitForInventory();
  const software = state.settings_software_check.software;
  assert.equal(state.settings_tab, 'software');
  assert.equal(software.length, 3);
  const quicklook = software.find(item => item.name === 'QuickLook');
  const everything = software.find(item => item.name === 'Everything');
  assert.equal(quicklook?.state, 'installed');
  assert.equal(everything?.state, 'installed');
  assert(quicklook.version && quicklook.path, 'QuickLook version and path should be shown');
  assert(everything.version && everything.path, 'Everything version and path should be shown');
  assert.equal(quicklook.github_url, 'https://github.com/QL-Win/QuickLook');
  assert.equal(everything.download_url, 'https://www.voidtools.com/downloads/');
  assert.equal(everything.github_url, 'https://github.com/voidtools/ES');

  const screenshot = await call('screenshot_window');
  const image = screenshot.content.find(block => block.type === 'image');
  assert(image, 'PhoTag MCP did not return the settings screenshot');
  fs.writeFileSync(path.join(output, 'settings-software-check.png'), Buffer.from(image.data, 'base64'));
  records.push('Settings Software tab displays QuickLook and Everything as installed with versions and executable locations.');
  records.push('QuickLook GitHub link, Everything official download page and official ES GitHub link are present.');
  records.push('No install, launch, or search call to QuickLook or Everything was made.');
  fs.writeFileSync(path.join(profile, 'software-check-report.json'), JSON.stringify({
    settings_tab: state.settings_tab,
    software,
    records,
    screenshot: path.join(output, 'settings-software-check.png'),
  }, null, 2));
  console.log(JSON.stringify({ records, software }));
};
