// Refresh design tokens from the running app: MCP dump (dark + light) -> tokens -> catalog.
//   node tools/capture.mjs [--exe target/debug/fileatlas-shell.exe]
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { repo, paths } from './lib.mjs';

const argv = process.argv.slice(2);
const exeIndex = argv.indexOf('--exe');
const exe = exeIndex < 0 ? 'target/debug/fileatlas-shell.exe' : argv[exeIndex + 1];

function run(command, args, label) {
  const result = spawnSync(command, args, { cwd: repo, stdio: 'inherit', shell: false });
  if (result.error) throw new Error(`${label}: ${result.error.message}`);
  if (result.status !== 0) throw new Error(`${label} 失败，退出码 ${result.status}`);
}

const profilesRoot = path.join(repo, '.local/validation/profiles');
for (const mode of ['dark', 'light']) {
  const profile = path.join(profilesRoot, `ui-tokens-${mode}`);
  fs.mkdirSync(profile, { recursive: true });
  fs.writeFileSync(path.join(profile, 'language.json'), JSON.stringify({ language: 'zh-CN' }), 'utf8');
  fs.writeFileSync(
    path.join(profile, 'settings.json'),
    JSON.stringify({ auto_check_updates: false, color_mode: mode }),
    'utf8',
  );
  run(
    process.execPath,
    [
      path.join(repo, 'scripts/fileatlas-mcp-debug.cjs'),
      '--exe', exe,
      '--profile', path.relative(repo, profile).replace(/\\/g, '/'),
      '--scenario', 'scripts/dump-ui-tokens.cjs',
    ],
    `MCP ${mode} 转储`,
  );
}

run(process.execPath, [path.join(paths.tokens, '..', 'tools', 'tokens.mjs'), '--import', '.local/validation/profiles'], '令牌生成');
run(process.execPath, [path.join(repo, 'ui-atlas/tools/build.mjs')], '目录重建');
console.log('ui-atlas: 令牌与截图基准已按实机刷新。');
