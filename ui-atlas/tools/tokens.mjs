// Regenerate ui-atlas design tokens.
//   geometry  <- parsed straight out of crates/fileatlas-ui-bridge/src/theme.rs
//   roles     <- live FileAtlas MCP get_ui_state.theme.roles dumps (--import)
// Usage: node tools/tokens.mjs [--import .local/validation/profiles] [--write-svgs]
import fs from 'node:fs';
import path from 'node:path';
import { paths, readJson, writeJson, roleVar, constVar, root, repo } from './lib.mjs';

const argv = process.argv.slice(2);
const option = name => {
  const index = argv.indexOf(name);
  return index < 0 ? null : argv[index + 1];
};

function parseGeometry() {
  const source = fs.readFileSync(paths.themeRs, 'utf8');
  const geometry = {};
  for (const match of source.matchAll(/^pub const ([A-Z0-9_]+): (f32|&str) = (.+);$/gm)) {
    const [, name, kind, raw] = match;
    if (kind === 'f32') {
      const value = Number(raw);
      if (Number.isFinite(value)) geometry[name] = value;
    } else {
      geometry[name] = raw.replace(/^"|"$/g, '');
    }
  }
  return geometry;
}

function importRoles(mode) {
  const dir = option('--import');
  if (!dir) return null;
  const absolute = path.resolve(repo, dir);
  const candidates = [
    path.join(absolute, `theme-roles-${mode}.json`),
    path.join(absolute, `ui-tokens-${mode}`, `theme-roles-${mode}.json`),
    path.join(absolute, `profiles`, `ui-tokens-${mode}`, `theme-roles-${mode}.json`),
  ];
  const file = candidates.find(candidate => fs.existsSync(candidate));
  if (!file) {
    console.error(`tokens: no ${mode} dump found, tried ${candidates.join(' | ')}`);
    process.exitCode = 1;
    return null;
  }
  const dump = readJson(file);
  if (!dump.roles) {
    console.error(`tokens: ${file} has no roles field`);
    process.exitCode = 1;
    return null;
  }
  return {
    source: dump.source ?? 'FileAtlas MCP get_ui_state.theme.roles',
    capturedAt: dump.capturedAt ?? null,
    themeId: dump.themeId ?? null,
    mode,
    roles: dump.roles,
    contentColors: dump.contentColors ?? null,
  };
}

function loadTheme(mode) {
  const imported = importRoles(mode);
  if (imported) return imported;
  const file = path.join(paths.tokens, `theme.${mode}.json`);
  if (!fs.existsSync(file)) {
    console.error(`tokens: missing ${file}; run node tools/capture.mjs first.`);
    process.exit(1);
  }
  return readJson(file);
}

const geometry = parseGeometry();
writeJson(path.join(paths.tokens, 'geometry.json'), {
  source: 'crates/fileatlas-ui-bridge/src/theme.rs',
  note: '由 tools/tokens.mjs 解析生成，请勿手改；改 Rust 常量后重跑。',
  constants: geometry,
});

const light = loadTheme('light');
const dark = loadTheme('dark');
for (const [mode, theme] of [['light', light], ['dark', dark]]) {
  writeJson(path.join(paths.tokens, `theme.${mode}.json`), { ...theme, geometry });
}

const vars = [];
for (const [role, value] of Object.entries(light.roles)) vars.push(`  ${roleVar(role)}: ${value};`);
for (const [name, value] of Object.entries(geometry)) {
  vars.push(`  ${constVar(name)}: ${typeof value === 'number' ? `${value}px` : `"${value}"`};`);
}
const darkVars = Object.entries(dark.roles).map(([role, value]) => `  ${roleVar(role)}: ${value};`);
fs.mkdirSync(paths.app, { recursive: true });
fs.writeFileSync(
  path.join(paths.app, 'tokens.css'),
  `/* 生成文件：node tools/tokens.mjs。颜色角色取自实机 GPUI Kit 主题，几何取自 theme.rs。 */\n` +
    `:root {\n${vars.join('\n')}\n}\n` +
    `[data-fa-theme="dark"] {\n${darkVars.join('\n')}\n}\n`,
  'utf8',
);

console.log(
  JSON.stringify({
    roles: Object.keys(light.roles).length,
    constants: Object.keys(geometry).length,
    lightTheme: light.themeId,
    darkTheme: dark.themeId,
    capturedAt: { light: light.capturedAt, dark: dark.capturedAt },
  }),
);
