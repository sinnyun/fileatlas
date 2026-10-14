// Same-process PhoTag MCP client for reproducible redesign debugging.
// No dependencies, OS input injection, or attachment to an existing window.
const fs = require('node:fs');
const path = require('node:path');
const { spawn } = require('node:child_process');
const readline = require('node:readline');
const assert = require('node:assert/strict');

const root = path.resolve(__dirname, '..');
const argv = process.argv.slice(2);
function option(name, fallback) {
  const index = argv.indexOf(name);
  return index < 0 ? fallback : argv[index + 1];
}
if (!option('--exe')) {
  console.error('Usage: node scripts/photag-mcp-debug.cjs --exe <exe> [--profile target/mcp-handoff] [--plan <json>] [--interactive] [--allow-mutations]');
  process.exit(2);
}
const exe = path.resolve(root, option('--exe'));
const profile = path.resolve(root, option('--profile', 'target/mcp-handoff'));
const relativeProfile = path.relative(path.join(root, 'target'), profile);
if (relativeProfile.startsWith('..') || path.isAbsolute(relativeProfile)) {
  throw Error('The development profile must be inside this workspace target directory.');
}
if (!fs.existsSync(exe)) throw Error(`Build the selected executable first: ${exe}`);
const output = path.join(profile, 'screenshots');
fs.mkdirSync(output, { recursive: true });
fs.mkdirSync(path.join(profile, 'local', 'PhoTag'), { recursive: true });
for (const [name, value] of [
  ['language.json', { language: 'zh-CN' }],
  ['settings.json', { auto_check_updates: true }],
]) {
  const file = path.join(profile, name);
  if (!fs.existsSync(file)) fs.writeFileSync(file, JSON.stringify(value, null, 2));
}
const child = spawn(exe, ['--mcp'], {
  cwd: root,
  env: {
    ...process.env,
    PHOTAG_DEV_PLUGINS: 'dev-test',
    PHOTAG_DEV_TEST_DIR: profile,
    LOCALAPPDATA: path.join(profile, 'local'),
    PHOTAG_MCP_ALLOW_MUTATIONS: argv.includes('--allow-mutations') ? '1' : '0',
  },
  stdio: ['pipe', 'pipe', 'pipe'],
});
child.stderr.pipe(fs.createWriteStream(path.join(profile, 'stderr.log'), { flags: 'a' }));
let buffer = '', nextId = 1;
const pending = new Map();
child.stdout.setEncoding('utf8');
child.stdout.on('data', chunk => {
  buffer += chunk;
  while (buffer.includes('\n')) {
    const end = buffer.indexOf('\n');
    const line = buffer.slice(0, end);
    buffer = buffer.slice(end + 1);
    if (!line.trim()) continue;
    let message;
    try { message = JSON.parse(line); }
    catch { console.error(`Unexpected MCP stdout: ${line}`); continue; }
    const callback = pending.get(message.id);
    if (callback) { pending.delete(message.id); callback(message); }
  }
});
function request(method, params) {
  return new Promise((resolve, reject) => {
    const id = nextId++;
    const timeout = setTimeout(() => {
      pending.delete(id);
      reject(Error(`MCP timeout: ${method}; inspect ${profile}/stderr.log`));
    }, 30000);
    pending.set(id, message => {
      clearTimeout(timeout);
      message.error ? reject(Error(JSON.stringify(message.error))) : resolve(message.result);
    });
    child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
  });
}
const delay = ms => new Promise(resolve => setTimeout(resolve, Math.min(Math.max(ms, 0), 10000)));
function payload(result) {
  const block = result.content?.find(item => item.type === 'text');
  if (!block) return null;
  try { return JSON.parse(block.text); } catch { return block.text; }
}
async function call(tool, args = {}) {
  const result = await request('tools/call', { name: tool, arguments: args });
  const value = payload(result);
  if (result.isError || (value && typeof value === 'object' && value.error != null)) {
    throw Error(JSON.stringify(value ?? result.content));
  }
  return result;
}
function field(value, dottedPath) {
  return dottedPath.split('.').reduce((current, key) => current?.[key], value);
}
const report = { executable: exe, profile, started: new Date().toISOString(), steps: [] };
async function step(command) {
  if (command.method) return request(command.method, command.params ?? {});
  let result, expectedError;
  try { result = await call(command.tool, command.arguments ?? {}); }
  catch (error) {
    if (!command.expect_error || !String(error).includes(command.expect_error)) throw error;
    expectedError = String(error);
  }
  assert(!command.expect_error || expectedError, `Expected MCP error containing ${command.expect_error}`);
  if (command.until) {
    let completed = false;
    for (let attempt = 0; attempt < (command.until.attempts ?? 30); attempt++) {
      const observed = field(payload(result), command.until.field);
      const matches = command.until.at_least === undefined
        ? JSON.stringify(observed) === JSON.stringify(command.until.equals)
        : typeof observed === 'number' && observed >= command.until.at_least;
      if (matches) {
        completed = true; break;
      }
      await delay(command.until.interval_ms ?? 500);
      result = await call(command.tool, command.arguments ?? {});
    }
    assert(completed, `Condition not reached: ${JSON.stringify(command.until)}`);
  }
  await delay(command.wait_ms ?? 0);
  const record = { tool: command.tool, arguments: command.arguments ?? {}, wait_ms: command.wait_ms ?? 0, ...(command.until ? { until: command.until } : {}), result: expectedError ? { expected_error: expectedError } : payload(result) };
  if (command.capture) {
    if (!/^[a-zA-Z0-9_-]+$/.test(command.capture)) throw Error('Capture name must be a simple file stem.');
    const screenshot = await call('screenshot_window');
    const image = screenshot.content.find(item => item.type === 'image');
    assert(image, 'MCP screenshot returned no image');
    record.screenshot = path.join(output, command.capture + '.png');
    fs.writeFileSync(record.screenshot, Buffer.from(image.data, 'base64'));
  }
  report.steps.push(record);
  fs.writeFileSync(path.join(profile, 'report.json'), JSON.stringify(report, null, 2));
  return record;
}

(async () => {
  try {
    await request('initialize', {
      protocolVersion: '2024-11-05', capabilities: {},
      clientInfo: { name: 'photag-redesign-debug', version: '1.0' },
    });
    child.stdin.write(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n');
    if (option('--plan')) {
      const commands = JSON.parse(fs.readFileSync(path.resolve(root, option('--plan')), 'utf8'));
      for (const command of commands) console.log(JSON.stringify(await step(command)));
    }
    if (argv.includes('--interactive')) {
      console.log(JSON.stringify({ ready: true, profile, example: { tool: 'get_ui_state', capture: 'main' } }));
      const input = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
      for await (const line of input) {
        if (!line.trim()) continue;
        if (line.trim() === 'exit') break;
        try { console.log(JSON.stringify(await step(JSON.parse(line)))); }
        catch (error) { console.error(String(error)); }
      }
    }
    if (!option('--plan') && !argv.includes('--interactive')) {
      console.log(JSON.stringify(await request('tools/list', {}), null, 2));
    }
  } catch (error) {
    console.error(error);
    process.exitCode = 1;
  } finally {
    child.kill();
  }
})();
