// Validate the SVG atlas, embed live theme tokens and auto-bump versions.
//   node tools/build.mjs            write token blocks + regenerate catalog
//   node tools/build.mjs --check    report drift without writing
//   node tools/build.mjs --note "…"  attach a note to this batch of version bumps
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';
import {
  paths,
  root as rootDir,
  roleVar,
  constVar,
  readJson,
  writeJson,
  stripTokenBlock,
  tokensForSvg,
  svgId,
  svgSize,
  listSvgFiles,
  TOKEN_BLOCK,
} from './lib.mjs';

const argv = process.argv.slice(2);
const flag = name => argv.includes(name);
const option = name => {
  const index = argv.indexOf(name);
  return index < 0 ? null : argv[index + 1];
};

function hash(text) {
  return crypto.createHash('sha256').update(text).digest('hex').slice(0, 16);
}

function bump(version, kind) {
  const [major, minor, patch] = version.split('.').map(Number);
  if (kind === 'minor') return `${major}.${minor + 1}.0`;
  return `${major}.${minor}.${patch + 1}`;
}

function ensureTokenBlock(svg, block) {
  if (TOKEN_BLOCK.test(svg)) {
    TOKEN_BLOCK.lastIndex = 0;
    return svg.replace(TOKEN_BLOCK, `${block}\n`);
  }
  TOKEN_BLOCK.lastIndex = 0;
  const open = /<svg\b[^>]*>/.exec(svg);
  if (!open) return null;
  const at = open.index + open[0].length;
  return `${svg.slice(0, at)}\n  ${block}\n${svg.slice(at)}`;
}

export function rebuild({ check = false, note = null } = {}) {
  const inventory = readJson(paths.inventory);
  const light = readJson(path.join(paths.tokens, 'theme.light.json'));
  const dark = readJson(path.join(paths.tokens, 'theme.dark.json'));
  const tokenBlock = tokensForSvg(light, dark);
  const knownVars = new Set([
    ...Object.keys(light.roles).map(roleVar),
    ...Object.keys(light.geometry ?? {}).map(constVar),
  ]);
  const previous = fs.existsSync(paths.manifest) ? readJson(paths.manifest) : { catalogRevision: 0, entries: {} };
  const previousEntries = previous.entries ?? {};

  const files = listSvgFiles();
  const found = new Map();
  const sources = {};
  const problems = [];
  const now = new Date().toISOString();
  const entries = {};
  const bumps = [];

  for (const file of files) {
    const id = svgId(file);
    const raw = fs.readFileSync(file, 'utf8');
    const withTokens = ensureTokenBlock(raw, tokenBlock);
    if (withTokens === null) {
      problems.push(`${id}: 缺少 <svg> 根元素`);
      continue;
    }
    if (withTokens !== raw && !check) fs.writeFileSync(file, withTokens, 'utf8');
    const design = stripTokenBlock(withTokens);
    for (const match of design.matchAll(/var\((--fa-[a-z0-9-]+)/g)) {
      if (!knownVars.has(match[1])) problems.push(`${id}: 引用了未知令牌 ${match[1]}`);
    }
    const hardcoded = [...design.matchAll(/\b(?:fill|stroke|stop-color)\s*=\s*"#/g)];
    if (hardcoded.length) problems.push(`${id}: 有 ${hardcoded.length} 处写死色值，应改用 var(--fa-…) 令牌`);
    sources[id] = withTokens;
    const sha = hash(design);
    const meta = inventory.find(item => item.id === id);
    if (!meta) problems.push(`${id}: 有 SVG 文件但未登记在 catalog/inventory.json`);
    const size = svgSize(design);
    if (!size.width || !size.height) problems.push(`${id}: 缺少 viewBox/width/height，无法测量尺寸`);
    const old = previousEntries[id];
    let version;
    let history;
    if (!old || !old.version) {
      version = '1.0.0';
      history = [{ version, sha, at: now, note: note ?? '首次描摹' }];
      bumps.push({ id, from: null, to: version });
    } else if (old.sha256 !== sha) {
      const kind = old.size && size && (old.size.width !== size.width || old.size.height !== size.height) ? 'minor' : 'patch';
      version = bump(old.version, kind);
      history = [...(old.history ?? []), { version, sha, at: now, note: note ?? (kind === 'minor' ? '尺寸变化' : '内容更新') }];
      bumps.push({ id, from: old.version, to: version });
    } else {
      version = old.version;
      history = old.history ?? [{ version, sha, at: now, note: '首次描摹' }];
    }
    found.set(id, true);
    entries[id] = {
      id,
      title: meta?.title ?? id,
      category: meta?.category ?? '未分类',
      group: meta?.group ?? '未分组',
      owner: meta?.owner ?? '',
      source: meta?.source ?? [],
      ref: meta?.ref ?? null,
      spec: meta?.spec ?? null,
      status: 'traced',
      file: `svg/${id}.svg`,
      version,
      sha256: sha,
      size,
      updatedAt: old && old.sha256 === sha ? (old.updatedAt ?? now) : now,
      history: history.slice(-40),
    };
  }

  for (const meta of inventory) {
    if (found.has(meta.id)) continue;
    const old = previousEntries[meta.id];
    entries[meta.id] = {
      id: meta.id,
      title: meta.title,
      category: meta.category,
      group: meta.group,
      owner: meta.owner ?? '',
      source: meta.source ?? [],
      ref: meta.ref ?? null,
      spec: meta.spec ?? null,
      status: old?.version || meta.status === 'traced' ? 'missing' : 'todo',
      file: `svg/${meta.id}.svg`,
      version: old?.version ?? null,
      sha256: old?.sha256 ?? null,
      size: old?.size ?? null,
      updatedAt: old?.updatedAt ?? null,
      history: old?.history ?? [],
    };
  }

  const list = Object.values(entries);
  const traced = list.filter(item => item.status === 'traced').length;
  const changed = bumps.length > 0 || previous.catalogRevision === 0 || Object.keys(previousEntries).length !== Object.keys(entries).length;
  const manifest = {
    generatedAt: now,
    catalogRevision: check ? previous.catalogRevision : (changed ? (previous.catalogRevision ?? 0) + 1 : previous.catalogRevision ?? 1),
    app: {
      name: 'FileAtlas',
      themeLight: light.themeId,
      themeDark: dark.themeId,
      tokenCapturedAt: { light: light.capturedAt, dark: dark.capturedAt },
    },
    tokens: {
      light: light.roles,
      dark: dark.roles,
      contentColors: { light: light.contentColors, dark: dark.contentColors },
      geometry: light.geometry ?? {},
    },
    stats: {
      total: list.length,
      traced,
      todo: list.filter(item => item.status === 'todo').length,
      missing: list.filter(item => item.status === 'missing').length,
      categories: list.reduce((acc, item) => {
        acc[item.category] = acc[item.category] ?? { total: 0, traced: 0 };
        acc[item.category].total += 1;
        if (item.status === 'traced') acc[item.category].traced += 1;
        return acc;
      }, {}),
    },
    entries,
  };

  if (check) {
    const drift = files.filter(file => {
      const raw = fs.readFileSync(file, 'utf8');
      return ensureTokenBlock(raw, tokenBlock) !== raw;
    });
    if (drift.length) problems.push(`令牌块需要刷新: ${drift.map(svgId).join(', ')}`);
  } else {
    writeJson(paths.manifest, manifest);
    fs.writeFileSync(paths.dataJs, `window.UI_ATLAS = ${JSON.stringify(manifest)};\n`, 'utf8');
    fs.writeFileSync(paths.sourcesJs, `window.UI_ATLAS_SVGS = ${JSON.stringify(sources)};\n`, 'utf8');
  }

  const summary = {
    check,
    revision: manifest.catalogRevision,
    total: manifest.stats.total,
    traced: manifest.stats.traced,
    todo: manifest.stats.todo,
    missing: manifest.stats.missing,
    bumped: bumps.map(item => `${item.id} ${item.from ?? '新增'}→${item.to}`),
    problems,
  };
  return summary;
}

// Write an edited SVG back to disk and let the version rules react to it.
export function saveSvg(id, content, note = null) {
  if (!/^[a-z0-9][a-z0-9._/-]*$/.test(id) || id.includes('..')) throw new Error(`非法组件 id：${id}`);
  if (!readJson(paths.inventory).some(item => item.id === id)) throw new Error(`未登记的组件 id：${id}`);
  if (!/<svg\b/i.test(content)) throw new Error('内容必须是完整的 SVG 文档');
  const file = path.join(paths.svg, `${id.replace(/\//g, path.sep)}.svg`);
  const before = fs.existsSync(paths.manifest) ? readJson(paths.manifest).entries[id]?.version ?? null : null;
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, content, 'utf8');
  const summary = rebuild({ note: note ?? '网页内编辑保存' });
  const after = readJson(paths.manifest).entries[id];
  return {
    ok: true,
    id,
    file: path.relative(rootDir, file).replace(/\\/g, '/'),
    from: before,
    version: after?.version ?? null,
    revision: summary.catalogRevision,
    problems: summary.problems,
  };
}

const self = fileURLToPath(import.meta.url);if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve(self)) {
  const summary = rebuild({ check: flag('--check'), note: option('--note') });
  console.log(JSON.stringify(summary, null, 2));
  if (summary.problems.length) process.exitCode = 1;
}
