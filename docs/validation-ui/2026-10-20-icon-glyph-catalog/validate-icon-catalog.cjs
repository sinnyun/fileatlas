// Run through scripts/fileatlas-mcp-debug.cjs with an isolated fresh profile:
// FILEATLAS_DEV_PLUGINS=all node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe \
//   --profile .local/validation/profiles/icon-glyph-catalog --access-level standard \
//   --scenario docs/validation-ui/2026-10-20-icon-glyph-catalog/validate-icon-catalog.cjs
const fs = require('node:fs');
const path = require('node:path');

module.exports = async ({ call, payload, delay, output }) => {
  const shot = async (name) => {
    const image = await call('screenshot_window');
    fs.writeFileSync(path.join(output, `${name}.png`),
      Buffer.from(image.content.find(b => b.type === 'image').data, 'base64'));
  };
  // 150% DPI: screenshot pixels -> logical client coordinates at the 1500x940 window size.
  const at = (sx, sy) => `${Math.round((sx - 11) / 1.5)},${Math.round((sy - 45) / 1.5)}`;
  const send = async (action, sx, sy) => {
    await call('control_ui', { action, value: at(sx, sy) });
    await delay(260);
  };
  const scroll = async (sx, sy, delta) => {
    await call('control_ui', { action: 'pointer_scroll', value: `${at(sx, sy)},${delta}` });
    await delay(500);
  };
  const openLauncher = async () => {
    await call('control_ui', { action: 'panel_open', value: 'program-launcher/panel' });
    await delay(2000);
    await call('resize_window', { width: 1500, height: 940 });
    for (let i = 0; i < 40; i++) {
      const snap = payload(await call('launcher.snapshot'));
      if (!snap.scanning && snap.entries.length > 5) return snap;
      await delay(500);
    }
    throw Error('launcher scan did not settle');
  };
  const pickTile = async () => {
    await send('pointer_move', 452, 367);
    await send('pointer_down', 452, 367);
    await send('pointer_up', 452, 367);
    await delay(1000);
  };

  let snap = await openLauncher();
  await delay(1200);
  await shot('10-idle-footer');
  // The grid sorts by display name ascending, so the first visible entry is the first tile.
  const visible = snap.entries.filter(e => !e.hidden)
    .sort((a, b) => (a.display_name || '').localeCompare(b.display_name || ''));
  const entry = visible[0];
  if (!entry) throw Error('no visible entry to pin');

  await call('launcher.set_pin', { id: entry.id, scope: 'category' });
  await delay(900);
  // Right-click menu -> 置顶 submenu: the active scope carries the native check mark.
  await send('pointer_right_down', 452, 367);
  await send('pointer_right_up', 452, 367);
  await delay(900);
  await send('pointer_move', 560, 484);
  await delay(1500);
  await shot('20-context-pin-checked');
  await send('pointer_down', 900, 860);
  await send('pointer_up', 900, 860);
  await delay(600);

  // Inspector: native carets plus the unchecked Square hidden toggle.
  await pickTile();
  await shot('30-inspector-top');
  await scroll(1349, 600, -100);
  await scroll(1349, 600, -100);
  await shot('40-inspector-pin-row');
  await send('pointer_move', 1300, 463);
  await send('pointer_down', 1300, 463);
  await send('pointer_up', 1300, 463);
  await delay(1200);
  await shot('50-inspector-pin-checked');
  await send('pointer_down', 900, 860);
  await send('pointer_up', 900, 860);
  await delay(600);

  await call('launcher.set_pin', { id: entry.id, scope: 'all' });
  await delay(900);
  await shot('70-badge-starfill');
  await call('launcher.set_pin', { id: entry.id, scope: 'none' });
  await delay(600);

  // Debug drawer: the three filter buttons use the library caret, not a glyph.
  await call('debug.control', { action: 'open' });
  await delay(1500);
  await shot('60-debug-drawer');
  const drawerState = await call('debug.query', { page_size: 1 });
  const opened = (drawerState.content || []).some(b => /"open"\s*:\s*true/.test(b.text || ''));
  if (!opened) throw Error('debug drawer did not open');
  await call('debug.control', { action: 'close' });
  await delay(600);
  await send('pointer_down', 1452, 66);
  await send('pointer_up', 1452, 66);
  await delay(900);

  // Address bar breadcrumb trail with the Ellipsis edit button.
  await call('control_navigation', { action: 'select_path', value: process.cwd() });
  await delay(1800);
  await shot('65-address-trail');

  // Hidden state of the inspector toggle.
  await call('launcher.update_program', { id: entry.id, hidden: true });
  await delay(900);
  await openLauncher();
  await send('pointer_move', 135, 337);
  await send('pointer_down', 135, 337);
  await send('pointer_up', 135, 337);
  await delay(1200);
  await pickTile();
  await shot('71-inspector-squarecheck');
  await call('launcher.update_program', { id: entry.id, hidden: false });
};
