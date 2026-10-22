// Dump the live theme role projection (and a reference screenshot) for the
// ui-atlas SVG library. Run through scripts/fileatlas-mcp-debug.cjs --scenario.
const fs = require('node:fs');
const path = require('node:path');

module.exports = async ({ call, payload, profile }) => {
  const expected = path.basename(profile).endsWith('light') ? 'light' : 'dark';
  let theme = null;
  for (let attempt = 0; attempt < 20; attempt++) {
    theme = payload(await call('get_ui_state'))?.theme;
    if (theme?.roles && theme.mode === expected) break;
    await new Promise(resolve => setTimeout(resolve, 1000));
  }
  if (!theme || !theme.roles) throw new Error('get_ui_state.theme.roles missing; rebuild fileatlas-shell first.');
  if (theme.mode !== expected) throw new Error(`Theme mode stayed ${theme.mode}, expected ${expected}.`);
  const mode = theme.mode === 'light' ? 'light' : 'dark';
  const out = path.join(profile, `theme-roles-${mode}.json`);
  fs.writeFileSync(
    out,
    JSON.stringify(
      {
        source: 'FileAtlas MCP get_ui_state.theme (same-process, live GPUI Kit theme)',
        capturedAt: new Date().toISOString(),
        themeId: theme.id,
        mode,
        accent: hexify(theme.accent),
        roles: theme.roles,
        contentColors: hexify(theme.content_colors),
      },
      null,
      2,
    ),
  );
  const shot = await waitForWindow(call, payload);
  const image = shot.content.find(item => item.type === 'image');
  if (image) fs.writeFileSync(path.join(profile, `window-${mode}.png`), Buffer.from(image.data, 'base64'));
  console.log(JSON.stringify({ ok: true, mode, roles: Object.keys(theme.roles).length, file: out }));
};

// The MCP child owns the window, but GPUI needs a first frame before a capture exists.
async function waitForWindow(call) {
  let lastError = null;
  for (let attempt = 0; attempt < 20; attempt++) {
    try {
      const shot = await call('screenshot_window');
      if (shot.content.some(item => item.type === 'image')) return shot;
      lastError = Error('screenshot_window returned no image');
    } catch (error) {
      lastError = error;
    }
    await new Promise(resolve => setTimeout(resolve, 1000));
  }
  throw lastError;
}

// get_ui_state reports unmanaged content colors as packed 0xRRGGBB numbers.
function hexify(value) {
  if (typeof value === 'number') return `#${value.toString(16).padStart(6, '0')}`;
  if (Array.isArray(value)) return value.map(hexify);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, hexify(item)]));
  }
  return value;
}
