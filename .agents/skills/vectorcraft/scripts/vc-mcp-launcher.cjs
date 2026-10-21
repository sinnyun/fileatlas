#!/usr/bin/env node
/*
 * Generic VectorCraft MCP launcher: finds the official vectorcraft-cli binary on this machine and
 * exposes its stdio MCP server to any MCP client (Codex, Claude Code, Cursor, VS Code, ...) without
 * hardcoding a per-host path. Bytes between the client and the server are forwarded untouched, so the
 * tool surface stays exactly the official one.
 *
 *   node vc-mcp-launcher.cjs                       # auto: attach a running `vectorcraft --control` app,
 *                                                  #   else fall back to the in-process headless engine
 *   node vc-mcp-launcher.cjs --headless            # never touch the desktop app
 *   node vc-mcp-launcher.cjs --connect 127.0.0.1:7979   # pin the live window; exit if it is absent
 *
 * Launcher-only options (never forwarded): --exe PATH --cwd DIR --which --print-config TARGET
 * --direct --install TARGET --dry-run. Forwarded: --headless --connect ADDR
 * --automation-read-root DIR --automation-write-root DIR (short forms --read-root/--write-root).
 */
const { spawn, spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const IS_WIN = process.platform === "win32";
const CLI_NAME = IS_WIN ? "vectorcraft-cli.exe" : "vectorcraft-cli";

function candidates() {
  const env = process.env;
  const home = env.HOME || env.USERPROFILE || "";
  const list = [env.VECTORCRAFT_CLI, env.VECTORCRAFT_MCP_EXE, env.FILEATLAS_VECTORCRAFT_EXE].filter(Boolean);
  if (IS_WIN) {
    list.push(
      path.join(env["ProgramFiles"] || "C:\\Program Files", "VectorCraft", CLI_NAME),
      path.join(env["ProgramFiles(x86)"] || "C:\\Program Files (x86)", "VectorCraft", CLI_NAME),
      path.join(env.LOCALAPPDATA || path.join(home, "AppData", "Local"), "Programs", "VectorCraft", CLI_NAME),
    );
  } else if (process.platform === "darwin") {
    list.push(
      "/Applications/VectorCraft.app/Contents/MacOS/vectorcraft-cli",
      path.join(home, "Applications", "VectorCraft.app", "Contents", "MacOS", "vectorcraft-cli"),
      path.join(home, "Applications", "VectorCraft", CLI_NAME),
    );
  } else {
    list.push(
      "/opt/VectorCraft/vectorcraft-cli",
      "/usr/local/bin/vectorcraft-cli",
      "/usr/bin/vectorcraft-cli",
      path.join(home, ".local", "bin", "vectorcraft-cli"),
      path.join(home, "VectorCraft", CLI_NAME),
    );
  }
  list.push(CLI_NAME, "vectorcraft-cli");
  return [...new Set(list)];
}

function isQualified(c) {
  return c.includes("/") || c.includes("\\") || (IS_WIN && /^[a-zA-Z]:/.test(c));
}

function findExe() {
  for (const c of candidates()) {
    if (isQualified(c)) {
      try {
        if (fs.existsSync(c)) return c;
      } catch {
        /* keep looking */
      }
      continue;
    }
    const probe = spawnSync(c, ["--version"], { encoding: "utf8", windowsHide: true });
    if (!probe.error && probe.status === 0) return c;
  }
  return null;
}

function parseArgs(argv) {
  const opts = { mode: "auto", readRoots: [], writeRoots: [], forward: [], exe: null, cwd: null };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const next = (name) => {
      const v = argv[++i];
      if (v === undefined) throw new Error(`${name} needs a value`);
      return v;
    };
    if (a === "--headless") opts.mode = "headless";
    else if (a === "--connect") {
      opts.mode = "connect";
      opts.addr = next("--connect");
    } else if (a === "--read-root" || a === "--automation-read-root") opts.readRoots.push(next(a));
    else if (a === "--write-root" || a === "--automation-write-root") opts.writeRoots.push(next(a));
    else if (a === "--exe") opts.exe = next("--exe");
    else if (a === "--cwd") opts.cwd = next("--cwd");
    else if (a === "--which") opts.print = "which";
    else if (a === "--direct") opts.direct = true;
    else if (a === "--dry-run") opts.dryRun = true;
    else if (a === "--print-config") opts.print = next("--print-config");
    else if (a === "--install") opts.install = next("--install");
    else if (a === "--help" || a === "-h") opts.print = "help";
    else throw new Error(`unknown option ${a} (see --help)`);
  }
  return opts;
}

function upstreamArgs(opts) {
  const a = ["mcp"];
  if (opts.mode === "headless") a.push("--headless");
  if (opts.mode === "connect") a.push("--connect", opts.addr);
  for (const r of opts.readRoots) a.push("--automation-read-root", r);
  for (const r of opts.writeRoots) a.push("--automation-write-root", r);
  return a;
}

function selfPath() {
  return path.resolve(__filename);
}

function configTargets(opts, exe) {
  const modeFlags = upstreamArgs(opts).slice(1); // drop the "mcp" subcommand
  if (opts.direct) return { command: exe, args: ["mcp", ...modeFlags] };
  const launcherArgs = [selfPath(), ...modeFlags];
  if (opts.cwd) launcherArgs.push("--cwd", opts.cwd);
  return { command: process.execPath, args: launcherArgs };
}

function tomlString(v) {
  return `'${String(v).replace(/\\/g, "\\\\").replace(/'/g, "\\'")}'`;
}

function codexBlock(rows, opts) {
  const lines = ["[mcp_servers.vectorcraft]", 'type = "stdio"', `command = ${tomlString(rows.command)}`];
  lines.push(`args = [${rows.args.map(tomlString).join(", ")}]`);
  lines.push("startup_timeout_sec = 60");
  const roots = [...opts.readRoots.map((r) => ["VECTORCRAFT_READ_ROOT", r]), ...opts.writeRoots.map((r) => ["VECTORCRAFT_WRITE_ROOT", r])];
  if (opts.exe) roots.push(["VECTORCRAFT_CLI", opts.exe]);
  if (roots.length) {
    lines.push(`env = { ${roots.map(([k, v]) => `${k} = ${tomlString(v)}`).join(", ")} }`);
    lines.push(`env_vars = [${roots.map(([k]) => tomlString(k)).join(", ")}]`);
  }
  return lines.join("\n");
}

function mcpJsonBlock(rows) {
  return JSON.stringify({ mcpServers: { vectorcraft: { command: rows.command, args: rows.args } } }, null, 2);
}

function vscodeJsonBlock(rows) {
  return JSON.stringify(
    {
      servers: {
        vectorcraft: { type: "stdio", command: rows.command, args: rows.args },
      },
    },
    null,
    2,
  );
}

function printConfig(target, opts, exe) {
  const rows = configTargets(opts, exe);
  switch (target) {
    case "codex":
      return codexBlock(rows, opts);
    case "claude":
    case "cursor":
      return mcpJsonBlock(rows);
    case "vscode":
      return vscodeJsonBlock(rows);
    case "json":
    case "generic":
      return JSON.stringify({ name: "vectorcraft", transport: "stdio", ...rows }, null, 2);
    default:
      throw new Error(`unknown config target ${target} (codex|claude|cursor|vscode|json)`);
  }
}

const HELP =
  "usage: node vc-mcp-launcher.cjs [--headless | --connect 127.0.0.1:PORT] [--read-root DIR] [--write-root DIR]\n" +
  "          [--exe PATH] [--cwd DIR]\n" +
  "       node vc-mcp-launcher.cjs --which | --print-config codex|claude|cursor|vscode|json [--direct]\n" +
  "       node vc-mcp-launcher.cjs --install codex [--dry-run]   # $CODEX_HOME/config.toml, one block, backed up\n";

function main() {
  let opts;
  try {
    opts = parseArgs(process.argv.slice(2));
  } catch (e) {
    process.stderr.write(`vectorcraft-mcp-launcher: ${e.message}\n${HELP}`);
    process.exit(2);
  }
  if (opts.print === "help") {
    process.stdout.write(HELP);
    return;
  }
  const exe = opts.exe || findExe();
  if (opts.mode === "connect" && (opts.readRoots.length || opts.writeRoots.length)) {
    process.stderr.write(
      "vectorcraft-mcp-launcher: automation roots imply the headless engine, so they cannot combine with\n" +
        "--connect. Drop --connect (confine the headless session) or confine the app itself with\n" +
        "`vectorcraft --control PORT --automation-read-root DIR --automation-write-root DIR`.\n",
    );
    process.exit(2);
  }
  if (!exe) {
    process.stderr.write(
      "vectorcraft-mcp-launcher: vectorcraft-cli not found. Install VectorCraft, set VECTORCRAFT_CLI to the\n" +
        "absolute binary path, or pass --exe PATH.\n",
    );
    process.exit(127);
  }
  if (opts.print === "which") {
    const v = spawnSync(exe, ["--version"], { encoding: "utf8", windowsHide: true });
    process.stdout.write(`${exe}${v.status === 0 ? `  (${String(v.stdout).trim()})` : "  (could not run --version)"}\n`);
    return;
  }
  if (opts.print) {
    process.stdout.write(`${printConfig(opts.print, opts, exe)}\n`);
    return;
  }
  if (opts.install) {
    installConfig(opts, exe);
    return;
  }

  const args = upstreamArgs(opts);
  const child = spawn(exe, args, {
    stdio: ["pipe", "pipe", "pipe"],
    cwd: opts.cwd || process.cwd(),
    windowsHide: true,
  });
  const log = (m) => process.stderr.write(`vectorcraft-mcp-launcher: ${m}\n`);
  log(`started ${exe} ${args.join(" ")} (pid ${child.pid}, cwd ${opts.cwd || process.cwd()})`);

  child.stdout.pipe(process.stdout);
  child.stderr.on("data", (d) => process.stderr.write(d));
  child.on("error", (e) => {
    log(`failed to start ${exe}: ${e.message}`);
    process.exit(127);
  });
  child.on("exit", (code, signal) => {
    if (signal) log(`upstream exited on signal ${signal}`);
    else if (code !== 0) log(`upstream exited with code ${code}`);
    process.exit(code == null ? 1 : code);
  });
  process.stdin.pipe(child.stdin);
  child.stdin.on("error", (e) => log(`upstream stdin error: ${e.message}`));
  process.stdin.on("error", (e) => log(`client stdin error: ${e.message}`));
  for (const sig of ["SIGINT", "SIGTERM"]) {
    process.on(sig, () => {
      log(`forwarding ${sig} to upstream`);
      try {
        child.kill(sig);
      } catch {
        /* already gone */
      }
    });
  }
}

function codexHome() {
  return process.env.CODEX_HOME || path.join(process.env.HOME || process.env.USERPROFILE || "", ".codex");
}

const BLOCK_KEY = "[mcp_servers.vectorcraft]";

function replaceTomlBlock(text, block) {
  const lines = text.split(/\r?\n/);
  const out = [];
  let removed = false;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (line.trim() === BLOCK_KEY) {
      removed = true;
      i++;
      while (i < lines.length && !/^\s*\[/.test(lines[i])) i++;
      i--;
      out.push(block);
      out.push("");
      continue;
    }
    out.push(line);
  }
  if (!removed) {
    while (out.length && out[out.length - 1] === "") out.pop();
    out.push("", block, "");
  }
  return { text: `${out.join("\n").replace(/\n{3,}/g, "\n\n").trimEnd()}\n`, removed };
}

function installConfig(opts, exe) {
  if (opts.install !== "codex") {
    process.stderr.write(`vectorcraft-mcp-launcher: --install supports codex (got ${opts.install}).\n`);
    process.exit(2);
  }
  const rows = configTargets(opts, exe);
  const block = codexBlock(rows, opts);
  const file = path.join(codexHome(), "config.toml");
  const current = fs.existsSync(file) ? fs.readFileSync(file, "utf8") : "";
  const next = replaceTomlBlock(current, block);
  if (opts.dryRun) {
    process.stdout.write(`--- would write ${file} ---\n${block}\n(previous block ${next.removed ? "replaced" : "none found"})\n`);
    return;
  }
  if (current) {
    const stamp = new Date().toISOString().replace(/[:.]/g, "-");
    const backup = `${file}.${stamp}.bak`;
    fs.writeFileSync(backup, current);
    process.stderr.write(`vectorcraft-mcp-launcher: backed up ${file} to ${backup}\n`);
  }
  fs.writeFileSync(file, next.text, "utf8");
  process.stderr.write(`vectorcraft-mcp-launcher: wrote ${BLOCK_KEY} in ${file}\n`);
}

main();
