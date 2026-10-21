#!/usr/bin/env node
/* VectorCraft batch CLI wrapper: locates vectorcraft-cli.exe and prints compact, token-cheap results. */
const { spawnSync } = require("node:child_process");
const path = require("node:path");

const CANDIDATES = [
  process.env.VECTORCRAFT_CLI,
  process.env.FILEATLAS_VECTORCRAFT_EXE,
  "C:\\Program Files\\VectorCraft\\vectorcraft-cli.exe",
  "C:\\Program Files (x86)\\VectorCraft\\vectorcraft-cli.exe",
  path.join(process.env.LOCALAPPDATA || "", "Programs", "VectorCraft", "vectorcraft-cli.exe"),
  "vectorcraft-cli",
].filter(Boolean);

function fail(msg) {
  process.stdout.write(`ERROR: ${msg}\n`);
  process.exit(2);
}

function locate() {
  for (const c of CANDIDATES) {
    const r = spawnSync(c, ["--version"], { encoding: "utf8", windowsHide: true });
    if (!r.error && r.status === 0) return { exe: c, version: (r.stdout || "").trim() };
  }
  return null;
}

function runCli(args, opts = {}) {
  const found = locate();
  if (!found) {
    fail(
      "vectorcraft-cli not found. Set VECTORCRAFT_CLI to the absolute path of vectorcraft-cli.exe " +
        `(tried: ${CANDIDATES.join(", ")}).`,
    );
  }
  const r = spawnSync(found.exe, args, {
    encoding: "utf8",
    windowsHide: true,
    maxBuffer: opts.maxBuffer || 256 * 1024 * 1024,
    cwd: opts.cwd,
  });
  if (r.error) fail(`${found.exe} failed to start: ${r.error.message}`);
  return { ...r, exe: found.exe, version: found.version };
}

function readCatalog() {
  const r = runCli(["commands"]);
  if (r.status !== 0) fail(`commands failed: ${(r.stderr || "").trim()}`);
  const parsed = JSON.parse(r.stdout);
  return Array.isArray(parsed) ? parsed : parsed.commands || [];
}

function summarizeRun(r) {
  const lines = (r.stdout || "").split(/\r?\n/).filter((l) => l.trim());
  for (const line of lines) {
    let j;
    try {
      j = JSON.parse(line);
    } catch {
      process.stdout.write(line + "\n");
      continue;
    }
    const label = j.command || j.export || j.open || j.step;
    const res = j.result === undefined ? j : j.result;
    process.stdout.write(`${j.step || "step"} ${label}: ${JSON.stringify(res).slice(0, 700)}\n`);
  }
  if ((r.stderr || "").trim()) process.stdout.write(`stderr: ${r.stderr.trim().slice(0, 1200)}\n`);
  process.stdout.write(`exit=${r.status}\n`);
  return r.status;
}

function printParams(catalog, id, full) {
  const entry = catalog.find((c) => c.id === id);
  if (!entry) {
    const near = catalog
      .filter((c) => c.id.startsWith(id.split(".").slice(0, -1).join(".") + "."))
      .map((c) => c.id);
    fail(`no command named ${id}. Siblings: ${near.slice(0, 25).join(", ") || "(none)"}`);
  }
  const params = entry.params || "(no parameters documented)";
  const clipped = full || params.length <= 2600 ? params : `${params.slice(0, 2600)}\n… ${params.length - 2600} more chars (pass --full)`;
  process.stdout.write(
    `${entry.id}  label=${entry.label}  menu=${(entry.menu || []).join(" > ")}  enabled=${entry.enabled}\n` +
      `shortcut=${entry.shortcut || "-"}\n` +
      `${clipped}\n`,
  );
}

function main() {
  const [cmd, ...rest] = process.argv.slice(2);
  if (!cmd || cmd === "which") {
    const found = locate();
    if (!found) fail("vectorcraft-cli not found; see ERROR text above for the paths tried.");
    process.stdout.write(`${found.exe}\n${found.version}\n`);
    return found.exe === "vectorcraft-cli" ? 0 : 0;
  }
  if (cmd === "version") {
    const r = runCli(["--version"]);
    process.stdout.write(r.stdout.trim() + "\n");
    return r.status;
  }
  if (cmd === "search") {
    const limitIdx = rest.indexOf("--limit");
    const limit = limitIdx >= 0 ? Number(rest.splice(limitIdx, 2)[1]) : 40;
    const full = rest.includes("--full");
    const items = rest.filter((a) => a !== "--full");
    const needle = items.join(" ").toLowerCase();
    const catalog = readCatalog();
    const hits = catalog.filter(
      (c) =>
        !needle ||
        c.id.toLowerCase().includes(needle) ||
        (c.label || "").toLowerCase().includes(needle) ||
        (c.params || "").toLowerCase().includes(needle),
    );
    process.stdout.write(`${hits.length} of ${catalog.length} commands match "${needle}"\n`);
    for (const h of hits.slice(0, limit)) {
      process.stdout.write(
        full
          ? `${h.id} | ${h.label} | enabled=${h.enabled} | ${(h.params || "").slice(0, 900)}\n`
          : `${h.id} | ${h.label} | enabled=${h.enabled} | menu=${(h.menu || []).join(">")}\n`,
      );
    }
    if (hits.length > limit) process.stdout.write(`… ${hits.length - limit} more (raise --limit or narrow the query)\n`);
    return 0;
  }
  if (cmd === "params") {
    if (!rest[0]) fail("usage: params <command.id> [--full]");
    printParams(readCatalog(), rest[0], rest.includes("--full"));
    return 0;
  }
  if (cmd === "run" || cmd === "convert" || cmd === "info" || cmd === "bench" || cmd === "perf") {
    const args = cmd === "run" || cmd === "convert" ? rest : rest;
    const r = runCli([cmd, ...args]);
    if (cmd === "info") {
      process.stdout.write(r.stdout);
      if (r.status !== 0) process.stdout.write(`stderr: ${(r.stderr || "").trim()}\nexit=${r.status}\n`);
      return r.status;
    }
    return summarizeRun(r);
  }
  if (cmd === "raw") {
    const r = runCli(rest);
    process.stdout.write(r.stdout);
    if (r.stderr) process.stdout.write(`stderr: ${r.stderr}\n`);
    process.stdout.write(`exit=${r.status}\n`);
    return r.status;
  }
  fail(
    `unknown subcommand "${cmd}". Use: which | version | search <keywords> [--limit N] [--full] | ` +
      `params <command.id> | run <cli args> | convert <args> | info <file> | bench <args> | perf <args> | raw <args>`,
  );
}

try {
  process.exitCode = main();
} catch (e) {
  fail(e.message);
}
