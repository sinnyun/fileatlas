// Shared helpers for the ui-atlas tooling: paths, token naming, SVG parsing.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
export const repo = path.resolve(root, '..');

export const paths = {
  root,
  svg: path.join(root, 'svg'),
  tokens: path.join(root, 'tokens'),
  catalog: path.join(root, 'catalog'),
  app: path.join(root, 'app'),
  themeRs: path.join(repo, 'crates/fileatlas-ui-bridge/src/theme.rs'),
  manifest: path.join(root, 'catalog/manifest.json'),
  dataJs: path.join(root, 'catalog/data.js'),
  sourcesJs: path.join(root, 'catalog/sources.js'),
  inventory: path.join(root, 'catalog/inventory.json'),
};

// The token block is machine-managed, so it must not count as a design change.
export const TOKEN_BLOCK = /<!--\s*FA:TOKENS:BEGIN\s*-->[\s\S]*?<!--\s*FA:TOKENS:END\s*-->\s*/g;

export function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

export function writeJson(file, value) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

export function roleVar(role) {
  return `--fa-${role.replace(/_/g, '-')}`;
}

export function constVar(name) {
  return `--fa-${name.toLowerCase().replace(/_/g, '-')}`;
}

export function stripTokenBlock(svg) {
  return svg.replace(TOKEN_BLOCK, '');
}

export function svgSize(svg) {
  const box = /viewBox\s*=\s*"([^"]+)"/.exec(svg);
  if (box) {
    const parts = box[1].trim().split(/[\s,]+/).map(Number);
    if (parts.length === 4 && parts.every(Number.isFinite)) {
      return { width: parts[2], height: parts[3] };
    }
  }
  const width = /\swidth\s*=\s*"([\d.]+)"/.exec(svg);
  const height = /\sheight\s*=\s*"([\d.]+)"/.exec(svg);
  return {
    width: width ? Number(width[1]) : null,
    height: height ? Number(height[1]) : null,
  };
}

export function listSvgFiles(dir = paths.svg) {
  if (!fs.existsSync(dir)) return [];
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...listSvgFiles(full));
    else if (entry.name.endsWith('.svg')) out.push(full);
  }
  return out.sort();
}

export function svgId(file) {
  return path.relative(paths.svg, file).replace(/\\/g, '/').replace(/\.svg$/, '');
}

export function tokensForSvg(themeLight, themeDark) {
  const lines = [':root{'];
  for (const [role, value] of Object.entries(themeLight.roles)) {
    lines.push(`${roleVar(role)}:${value};`);
  }
  for (const [name, value] of Object.entries(themeLight.geometry ?? {})) {
    lines.push(`${constVar(name)}:${typeof value === 'number' ? `${value}px` : value};`);
  }
  lines.push('}');
  const dark = [];
  for (const [role, value] of Object.entries(themeDark.roles)) {
    dark.push(`${roleVar(role)}:${value};`);
  }
  return `<!-- FA:TOKENS:BEGIN --><style>${lines.join('')}@media (prefers-color-scheme:dark){:root{${dark.join('')}}}</style><!-- FA:TOKENS:END -->`;
}
