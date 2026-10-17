// Enforce PhoTag's compile-time crate layers and low-level dependency boundary.
// Keep the layer map and same-layer exceptions reviewable in this file.
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '..');
const cratesDir = path.join(root, 'crates');

const layers = [
  ['photag-contracts', 'photag-plugin-database'],
  [
    'photag-plugin-thumbnails',
    'photag-plugin-quicklook',
    'photag-plugin-drag-drop',
    'photag-plugin-updater',
    'photag-plugin-file-operations',
    'photag-plugin-session',
    'photag-plugin-version-info',
  ],
  ['photag-plugin-indexing', 'photag-plugin-watcher'],
  [
    'photag-ui-bridge',
    'photag-plugin-tagging',
    'photag-plugin-history',
    'photag-plugin-grouping',
    'photag-plugin-navigation',
    'photag-plugin-debug',
    'photag-plugin-dev-test',
    'photag-plugin-metadata-export',
    'photag-plugin-i18n',
  ],
  ['photag-plugin-settings'],
  ['photag-plugin-versions'],
  ['photag-plugin-ai'],
  ['photag-plugin-gallery'],
  ['photag-plugin-mcp', 'photag-shell'],
];

// Same-layer dependencies are intentional peers and must be reviewed here.
const sameLayerEdges = new Set([
  'photag-plugin-watcher -> photag-plugin-indexing',
  'photag-plugin-tagging -> photag-ui-bridge',
  'photag-plugin-history -> photag-plugin-tagging',
  'photag-plugin-history -> photag-ui-bridge',
  'photag-plugin-grouping -> photag-ui-bridge',
  'photag-plugin-navigation -> photag-plugin-tagging',
  'photag-plugin-navigation -> photag-ui-bridge',
  'photag-plugin-debug -> photag-ui-bridge',
  'photag-plugin-dev-test -> photag-ui-bridge',
  'photag-plugin-metadata-export -> photag-ui-bridge',
  'photag-plugin-i18n -> photag-ui-bridge',
  'photag-plugin-ai -> photag-plugin-settings',
  'photag-shell -> photag-plugin-mcp',
]);

const uiPackages = new Set(['gpui', 'gpui-pre', 'gpui-kit', 'egui', 'eframe', 'winit']);
const failures = [];
const membersText = fs.readFileSync(path.join(root, 'Cargo.toml'), 'utf8');
const memberDirs = [...membersText.matchAll(/"(crates\/[a-z0-9-]+)"/g)].map((m) => m[1]);
const packageToDir = new Map();
const crateToLayer = new Map();

layers.forEach((names, layer) => {
  for (const name of names) {
    if (crateToLayer.has(name)) failures.push(`layer map repeats crate ${name}`);
    crateToLayer.set(name, layer);
  }
});

function dependencySections(text) {
  const result = [];
  let active = false;
  let current = [];
  for (const line of text.split(/\r?\n/)) {
    const header = line.match(/^\s*\[([^\]]+)\]\s*$/);
    if (header) {
      if (active) result.push(current.join('\n'));
      active = /^(?:target\..+\.)?(?:dependencies|dev-dependencies|build-dependencies)$/.test(header[1]);
      current = [];
    } else if (active) {
      current.push(line);
    }
  }
  if (active) result.push(current.join('\n'));
  return result;
}

function runtimeDependencySections(text) {
  const result = [];
  let active = false;
  let current = [];
  for (const line of text.split(/\r?\n/)) {
    const header = line.match(/^\s*\[([^\]]+)\]\s*$/);
    if (header) {
      if (active) result.push(current.join('\n'));
      active = /^(?:target\..+\.)?dependencies$/.test(header[1]);
      current = [];
    } else if (active) {
      current.push(line);
    }
  }
  if (active) result.push(current.join('\n'));
  return result;
}

function localDependencies(manifestText, manifestDir) {
  const dependencies = [];
  const ui = new Set();
  const sections = dependencySections(manifestText);
  for (const section of sections) {
    for (const line of section.split(/\r?\n/)) {
      const entry = line.match(/^\s*([A-Za-z0-9_-]+)\s*=\s*(.*)$/);
      if (!entry) continue;
      const alias = entry[1];
      const value = entry[2];
      const packageName = value.match(/\bpackage\s*=\s*"([^"]+)"/)?.[1] ?? alias;
      if (uiPackages.has(alias) || uiPackages.has(packageName)) ui.add(packageName);
      const localPath = value.match(/\bpath\s*=\s*"([^"]+)"/)?.[1];
      if (localPath) dependencies.push(path.resolve(manifestDir, localPath));
    }
  }
  return { dependencies, ui };
}

for (const member of memberDirs) {
  const dir = path.resolve(root, member);
  const manifest = path.join(dir, 'Cargo.toml');
  if (!fs.existsSync(manifest)) {
    failures.push(`workspace member has no manifest: ${member}`);
    continue;
  }
  const text = fs.readFileSync(manifest, 'utf8');
  const packageName = text.match(/^\s*name\s*=\s*"([^"]+)"/m)?.[1];
  if (!packageName) {
    failures.push(`package name missing in ${member}/Cargo.toml`);
    continue;
  }
  packageToDir.set(packageName, dir);
  if (!crateToLayer.has(packageName)) failures.push(`crate missing from layer map: ${packageName}`);
  for (const section of runtimeDependencySections(text)) {
    for (const line of section.split(/\r?\n/)) {
      const entry = line.match(/^\s*([A-Za-z0-9_-]+)\s*=\s*(.*)$/);
      if (!entry) continue;
      const dependencyName = entry[2].match(/\bpackage\s*=\s*"([^"]+)"/)?.[1] ?? entry[1];
      if (dependencyName === 'rusqlite' && packageName !== 'photag-plugin-database') {
        failures.push(`${packageName} must not depend on rusqlite at runtime`);
      }
    }
  }
}

for (const name of crateToLayer.keys()) {
  if (!packageToDir.has(name)) failures.push(`layer map references a non-member crate: ${name}`);
}

for (const [name, dir] of packageToDir) {
  const text = fs.readFileSync(path.join(dir, 'Cargo.toml'), 'utf8');
  const { dependencies, ui } = localDependencies(text, dir);
  const fromLayer = crateToLayer.get(name);
  if (fromLayer < 3 && ui.size) {
    failures.push(`${name} is in layer ${fromLayer} but directly depends on UI packages: ${[...ui].join(', ')}`);
  }
  for (const dependencyDir of dependencies) {
    const dependency = [...packageToDir.entries()].find(([, candidate]) => candidate === dependencyDir)?.[0];
    if (!dependency) continue;
    const dependencyLayer = crateToLayer.get(dependency);
    if (dependencyLayer > fromLayer) {
      failures.push(`${name} (layer ${fromLayer}) depends on higher layer ${dependency} (layer ${dependencyLayer})`);
    } else if (dependencyLayer === fromLayer && !sameLayerEdges.has(`${name} -> ${dependency}`)) {
      failures.push(`${name} has an unapproved same-layer dependency on ${dependency}`);
    }
  }

  for (const file of rustFiles(path.join(dir, 'src'))) {
    if (/(^|_)(tests?|test_utils)\.rs$/.test(path.basename(file))) continue;
    const source = stripInlineTestModule(stripTestOnly(fs.readFileSync(file, 'utf8')));
    for (const match of source.matchAll(/photag_plugin_([a-z0-9_]+)::([a-z][a-z0-9_]*)::/g)) {
      const dependency = `photag-plugin-${match[1].replaceAll('_', '-')}`;
      if (dependency === name) continue;
      if (match[2] === 'queries') failures.push(`${name} reaches private Database query module in ${path.relative(root, file)}`);
    }
  }
}

// Raw SQLite callbacks belong to DatabaseService. Feature crates use its
// typed operations; inline unit-test setup is excluded from this production guard.
for (const [name, dir] of packageToDir) {
  if (name === 'photag-plugin-database') continue;
  const src = path.join(dir, 'src');
  if (!fs.existsSync(src)) continue;
  for (const file of rustFiles(src)) {
    if (/(^|_)(tests?|test_utils)\.rs$/.test(path.basename(file))) continue;
    const text = stripInlineTestModule(fs.readFileSync(file, 'utf8'));
    if (/\bConnection\s*::\s*(open|open_in_memory|open_with_flags)\s*\(/.test(text)) {
      failures.push(`${name} opens a SQLite connection outside DatabaseService: ${path.relative(root, file)}`);
    }
    if (/(?:\.|::)\s*with_(?:reader|writer|cache_writer)\s*\(/.test(text)) {
      failures.push(`${name} uses a raw DatabaseService SQLite callback: ${path.relative(root, file)}`);
    }
    if (/\brusqlite\s*::/.test(text)) {
      failures.push(`${name} references rusqlite outside DatabaseService: ${path.relative(root, file)}`);
    }
  }
}

function stripInlineTestModule(source) {
  const marker = /#\[cfg\(test\)\]\s*mod\s+tests\s*\{/g;
  const match = marker.exec(source);
  if (!match) return source;
  const open = source.indexOf('{', match.index);
  let depth = 0;
  for (let index = open; index < source.length; index++) {
    if (source[index] === '{') depth++;
    else if (source[index] === '}' && --depth === 0) {
      return source.slice(0, match.index) + source.slice(index + 1);
    }
  }
  return source;
}

function stripTestOnly(source) {
  return source.replace(/^\s*#\[cfg\(test\)\]\s*(?:use\s+[^;]+;|mod\s+[^;]+;?)\s*/gm, '');
}

function rustFiles(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...rustFiles(file));
    else if (entry.name.endsWith('.rs')) out.push(file);
  }
  return out;
}

if (failures.length) {
  console.error(failures.join('\n'));
  process.exitCode = 1;
} else {
  console.log(`layering guard: passed (${crateToLayer.size} crates, ${layers.length} layers, ${sameLayerEdges.size} approved peer edges)`);
}
