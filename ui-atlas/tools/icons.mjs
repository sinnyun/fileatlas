// Vendor the exact Lucide icons this app renders, straight from the pinned
// gpui-kit-assets crate, and index them for the SVG atlas.
//   node tools/icons.mjs
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { paths, repo } from './lib.mjs';

const registry = path.join(os.homedir(), '.cargo', 'registry', 'src');
const candidates = fs.existsSync(registry)
  ? fs
      .readdirSync(registry)
      .flatMap(target =>
        fs
          .readdirSync(path.join(registry, target))
          .filter(name => name.startsWith('gpui-kit-assets-'))
          .map(name => path.join(registry, target, name)),
      )
      .filter(dir => fs.existsSync(path.join(dir, 'assets', 'icons')))
  : [];
if (!candidates.length) {
  console.error('icons: 未找到 gpui-kit-assets 的本地源码（先跑一次 cargo build 拉取依赖）。');
  process.exit(1);
}
candidates.sort();
const crate = candidates[candidates.length - 1];
const iconDir = path.join(crate, 'assets', 'icons');
const version = path.basename(crate).replace('gpui-kit-assets-', '');

function walk(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full, out);
    else if (entry.name.endsWith('.rs')) out.push(full);
  }
  return out;
}

const used = new Set();
for (const file of walk(path.join(repo, 'crates'))) {
  const text = fs.readFileSync(file, 'utf8');
  for (const match of text.matchAll(/IconName::([A-Za-z0-9_]+)/g)) used.add(match[1]);
}

const kebab = name =>
  name
    .replace(/_/g, '-')
    .replace(/([a-z0-9])([A-Z])/g, '$1-$2')
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1-$2')
    .toLowerCase();

fs.mkdirSync(path.join(paths.root, 'icons'), { recursive: true });
const index = {};
const missing = [];
for (const name of [...used].sort()) {
  const file = `${kebab(name)}.svg`;
  const source = path.join(iconDir, file);
  if (!fs.existsSync(source)) {
    missing.push(name);
    continue;
  }
  fs.copyFileSync(source, path.join(paths.root, 'icons', file));
  index[name] = `icons/${file}`;
}
if (fs.existsSync(path.join(crate, 'LICENSE-LUCIDE'))) {
  fs.copyFileSync(path.join(crate, 'LICENSE-LUCIDE'), path.join(paths.root, 'icons', 'LICENSE-LUCIDE'));
}
fs.writeFileSync(
  path.join(paths.root, 'icons', 'index.json'),
  `${JSON.stringify(
    {
      source: `gpui-kit-assets ${version} (pinned crate)`,
      upstream: 'Lucide Icons, ISC licence — see icons/LICENSE-LUCIDE',
      note: '产品实际渲染的同一套图标；SVG 用 <use href="#fa-icon-名称小写-连字符"> 引用，构建时注入 symbol。',
      usedBy: '扫描 crates/ 下全部 IconName:: 引用得到',
      icons: index,
      missing,
    },
    null,
    2,
  )}\n`,
  'utf8',
);
console.log(JSON.stringify({ crate: version, vendored: Object.keys(index).length, missing }));
