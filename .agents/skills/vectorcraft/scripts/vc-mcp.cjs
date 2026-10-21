#!/usr/bin/env node
/* Drive the VectorCraft MCP server (stdio JSON-RPC) in headless or connected mode, one call at a time. */
const { spawn } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const CLI_CANDIDATES = [
  process.env.VECTORCRAFT_CLI,
  process.env.FILEATLAS_VECTORCRAFT_EXE,
  "C:\\Program Files\\VectorCraft\\vectorcraft-cli.exe",
  "C:\\Program Files (x86)\\VectorCraft\\vectorcraft-cli.exe",
  path.join(process.env.LOCALAPPDATA || "", "Programs", "VectorCraft", "vectorcraft-cli.exe"),
  "vectorcraft-cli",
].filter(Boolean);

function findCli() {
  for (const c of CLI_CANDIDATES) {
    try {
      if (require("node:child_process").spawnSync(c, ["--version"], { encoding: "utf8" }).status === 0) return c;
    } catch {
      /* keep looking */
    }
  }
  throw new Error(`vectorcraft-cli not found (tried: ${CLI_CANDIDATES.join(", ")}). Set VECTORCRAFT_CLI.`);
}

function parseArgs(argv) {
  const opts = { calls: [], maxText: 1200, timeoutMs: 120000, mode: "auto", shot: null, depth: 0, inspect: false };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const next = () => argv[++i];
    if (a === "--headless") opts.mode = "headless";
    else if (a === "--connect") { opts.mode = "connect"; opts.addr = next(); }
    else if (a === "--read-root") (opts.readRoots ||= []).push(next());
    else if (a === "--write-root") (opts.writeRoots ||= []).push(next());
    else if (a === "--call") opts.calls.push({ tool: next(), args: JSON.parse(next() || "{}") });
    else if (a === "--script") {
      for (const step of JSON.parse(fs.readFileSync(next(), "utf8")))
        opts.calls.push(step.tool ? { tool: step.tool, args: step.args || {} } : { method: step.method, params: step.params || {} });
    }
    else if (a === "--tools") opts.request = { method: "tools/list", params: {} };
    else if (a === "--resources") opts.request = { method: "resources/list", params: {} };
    else if (a === "--prompts") opts.request = { method: "prompts/list", params: {} };
    else if (a === "--inspect") { opts.inspect = true; }
    else if (a === "--depth") opts.depth = Number(next());
    else if (a === "--shot") { opts.shot = next(); opts.calls.push({ tool: "screenshot", args: { path: opts.shot } }); }
    else if (a === "--max-text") opts.maxText = Number(next());
    else if (a === "--timeout") opts.timeoutMs = Number(next());
    else if (a === "--help") {
      process.stdout.write(
        "usage: node vc-mcp.cjs [--headless | --connect 127.0.0.1:7979] [--read-root DIR] [--write-root DIR]\n" +
          "          [--tools | --resources | --prompts]\n" +
          "          [--call TOOL '{json}']... [--script steps.json] [--inspect] [--depth N] [--shot out.png]\n" +
          "          [--max-text N] [--timeout MS]\n" +
          "steps.json: [{\"tool\":\"draw_shape\",\"args\":{...}}, {\"method\":\"tools/list\",\"params\":{}}]\n",
      );
      process.exit(0);
    } else throw new Error(`unknown option ${a}`);
  }
  if (opts.inspect) opts.calls.push({ tool: "inspect_document", args: { depth: opts.depth } });
  return opts;
}

function serverArgs(opts) {
  const a = ["mcp"];
  if (opts.mode === "headless") a.push("--headless");
  if (opts.mode === "connect") a.push("--connect", opts.addr);
  for (const r of opts.readRoots || []) a.push("--automation-read-root", r);
  for (const r of opts.writeRoots || []) a.push("--automation-write-root", r);
  return a;
}

function main() {
  const opts = parseArgs(process.argv.slice(2));
  const child = spawn(findCli(), serverArgs(opts), { stdio: ["pipe", "pipe", "pipe"], windowsHide: true });
  const pending = new Map();
  const buffer = { data: "" };
  let stderrTail = "";
  child.stdout.on("data", (chunk) => {
    buffer.data += chunk.toString("utf8");
    let idx;
    while ((idx = buffer.data.indexOf("\n")) >= 0) {
      const line = buffer.data.slice(0, idx).trim();
      buffer.data = buffer.data.slice(idx + 1);
      if (!line) continue;
      let msg;
      try {
        msg = JSON.parse(line);
      } catch {
        process.stdout.write(`unparsable: ${line.slice(0, 200)}\n`);
        continue;
      }
      const p = msg.id !== undefined && pending.get(msg.id);
      if (p) {
        pending.delete(msg.id);
        p.resolve(msg);
      } else if (msg.method === "notifications/message") {
        process.stdout.write(`log[${msg.params?.level}]: ${String(msg.params?.data).slice(0, 200)}\n`);
      } else if (msg.method === "notifications/progress") {
        process.stdout.write(`progress ${msg.params?.progress}/${msg.params?.total ?? "?"}: ${msg.params?.message || ""}\n`);
      }
    }
  });
  child.stderr.on("data", (c) => {
    stderrTail = (stderrTail + c.toString("utf8")).slice(-8000);
  });
  child.on("exit", (code) => {
    for (const p of pending.values()) p.reject(new Error(`server exited (code=${code}) before answering: ${stderrTail.slice(-500)}`));
    pending.clear();
  });

  let nextId = 1;
  const send = (method, params) =>
    new Promise((resolve, reject) => {
      const id = nextId++;
      const timer = setTimeout(() => {
        pending.delete(id);
        reject(new Error(`timeout after ${opts.timeoutMs}ms waiting for ${method}`));
      }, opts.timeoutMs);
      pending.set(id, {
        resolve: (m) => {
          clearTimeout(timer);
          resolve(m);
        },
        reject: (e) => {
          clearTimeout(timer);
          reject(e);
        },
      });
      child.stdin.on("error", () => {});
      child.stdin.write(JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n");
    });
  const notify = (method, params) => {
    child.stdin.on("error", () => {});
    child.stdin.write(JSON.stringify({ jsonrpc: "2.0", method, params }) + "\n");
  };

  const report = (label, msg) => {
    if (msg.error) {
      process.stdout.write(`${label}: JSON-RPC error ${JSON.stringify(msg.error).slice(0, opts.maxText)}\n`);
      return 1;
    }
    const r = msg.result;
    if (r && Array.isArray(r.tools)) {
      process.stdout.write(`tools (${r.tools.length}): ${r.tools.map((t) => t.name).join(", ")}\n`);
      return 0;
    }
    if (r && Array.isArray(r.resources)) {
      process.stdout.write(`resources: ${r.resources.map((x) => x.uri).join(", ")}\n`);
      return 0;
    }
    if (r && Array.isArray(r.prompts)) {
      process.stdout.write(`prompts: ${r.prompts.map((p) => p.name).join(", ")}\n`);
      return 0;
    }
    if (r && Array.isArray(r.content)) {
      const bad = !!r.isError;
      const parts = r.content.map((c) =>
        c.type === "image" ? `[${c.mimeType} ${(c.data || "").length} base64 bytes]` : String(c.text ?? "").slice(0, opts.maxText),
      );
      process.stdout.write(`${label}: ${bad ? "isError " : ""}${parts.join(" | ")}\n`);
      return bad ? 1 : 0;
    }
    process.stdout.write(`${label}: ${JSON.stringify(r).slice(0, opts.maxText)}\n`);
    return 0;
  };

  (async () => {
    let rc = 0;
    await send("initialize", {
      protocolVersion: "2025-06-18",
      capabilities: {},
      clientInfo: { name: "vc-mcp", version: "1" },
    });
    notify("notifications/initialized", {});
    if (opts.request) {
      rc = report(opts.request.method, await send(opts.request.method, opts.request.params));
    } else {
      for (const call of opts.calls) {
        const msg = call.method
          ? await send(call.method, call.params)
          : await send("tools/call", { name: call.tool, arguments: call.args });
        rc = report(call.tool || call.method, msg) || rc;
      }
    }
    child.kill();
    if (stderrTail.trim()) process.stdout.write(`server stderr: ${stderrTail.trim().slice(-600)}\n`);
    process.exitCode = rc;
  })().catch((e) => {
    process.stdout.write(`ERROR: ${e.message}\n`);
    child.kill();
    process.exitCode = 2;
  });
}

main();
