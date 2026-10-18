// Restart verification for the launcher's persistent native icon cache.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({ step, delay, profile }) => {
  const invoke = async (tool, arguments_ = {}, extra = {}) =>
    (await step({ tool, arguments: arguments_, ...extra })).result;
  await invoke('control_ui', { action: 'panel_open', value: 'program-launcher/panel' }, { wait_ms: 300 });
  let state = await invoke('get_launcher_state');
  for (let attempt = 0; attempt < 20 && !state.ui?.icon_cache; attempt++) {
    await delay(100);
    state = await invoke('get_launcher_state');
  }
  const first_open = {
    scanning: state.scanning,
    entries: state.entries.length,
    icons_pending: state.ui?.icons_pending ?? null,
    disk_hits: state.ui?.icon_cache?.disk_hits ?? null,
  };
  assert(first_open.entries > 0, 'persisted programs should be visible without waiting for discovery refresh');
  const started = Date.now();
  for (let attempt = 0; attempt < 160; attempt++) {
    state = await invoke('get_launcher_state');
    const icons = state.ui?.icon_cache;
    if (!state.scanning && !state.ui?.icons_pending && icons?.disk_hits > 0) break;
    await delay(250);
  }
  assert(state.ui && !state.scanning && !state.ui.icons_pending, 'launcher and icon read must settle after restart');
  const manual = state.entries.find(entry => entry.source === 'manual');
  assert(manual, 'the prior run must have left a manually added fixture in the isolated profile');
  assert.equal(state.ui.icon_cache.manual_programs[String(manual.id)], 'disk-cache', 'manual program icon should load from persistent cache after restart');
  assert(state.ui.icon_cache.disk_hits > 0, 'existing native program icons should be served from persistent cache');
  assert.equal(state.ui.icon_cache.shell_extractions, 0, 'warm restart should not re-extract icons already cached');
  await invoke('screenshot_window', {}, { capture: 'icon-grid-after-restart' });
  const cacheDir = path.join(profile, 'program-icons');
  const cachedPngs = fs.readdirSync(cacheDir).filter(name => name.endsWith('.png')).length;
  assert(cachedPngs > 0, 'persistent icon cache files should exist beside the isolated PhoTag profile');
  const summary = {
    first_open,
    icon_cache_ready_ms: Date.now() - started,
    programs: state.entries.length,
    persistent_disk_hits: state.ui.icon_cache.disk_hits,
    native_extractions: state.ui.icon_cache.shell_extractions,
    failed_icon_programs: state.ui.icon_cache.failed_programs,
    cached_pngs: cachedPngs,
    manual_program_icon: state.ui.icon_cache.manual_programs[String(manual.id)],
  };
  fs.writeFileSync(path.join(profile, 'launcher-icon-cache-restart.json'), JSON.stringify(summary, null, 2));
  console.log(JSON.stringify(summary));
};
