// Local gallery server: static files + write-back API that bumps versions.
//   node tools/serve.mjs [--port 8181]
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { paths, root, readJson } from './lib.mjs';
import { rebuild, saveSvg } from './build.mjs';

const argv = process.argv.slice(2);
const portIndex = argv.indexOf('--port');
const port = portIndex < 0 ? 8181 : Number(argv[portIndex + 1]);

const mime = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml; charset=utf-8',
  '.png': 'image/png',
};

function send(res, status, body, type = 'application/json; charset=utf-8') {
  res.writeHead(status, { 'content-type': type, 'cache-control': 'no-store' });
  res.end(body);
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    let text = '';
    req.on('data', chunk => {
      text += chunk;
      if (text.length > 4_000_000) reject(Error('body too large'));
    });
    req.on('end', () => {
      try {
        resolve(text ? JSON.parse(text) : {});
      } catch (error) {
        reject(error);
      }
    });
    req.on('error', reject);
  });
}

const server = http.createServer(async (req, res) => {
  const url = new URL(req.url ?? '/', 'http://127.0.0.1');
  const route = url.pathname;
  try {
    if (req.method === 'POST' && route === '/api/save') {
      const body = await readBody(req);
      const result = saveSvg(String(body.id ?? ''), String(body.content ?? ''), body.note ? String(body.note) : null);
      return send(res, 200, JSON.stringify(result));
    }
    if (req.method === 'POST' && route === '/api/rebuild') {
      const body = await readBody(req);
      return send(res, 200, JSON.stringify(rebuild({ note: body.note ? String(body.note) : null })));
    }
    if (req.method === 'GET' && route === '/api/state') {
      const manifest = fs.existsSync(paths.manifest) ? readJson(paths.manifest) : null;
      return send(res, 200, JSON.stringify({ ok: true, stats: manifest?.stats ?? null, revision: manifest?.catalogRevision ?? 0 }));
    }
    if (req.method !== 'GET' && req.method !== 'HEAD') return send(res, 405, '{"error":"method not allowed"}', 'application/json');

    const relative = route === '/' ? 'index.html' : decodeURIComponent(route).replace(/^\/+/, '');
    const target = path.resolve(root, relative);
    if (target !== root && !target.startsWith(root + path.sep)) return send(res, 403, '{"error":"forbidden"}');
    if (!fs.existsSync(target) || fs.statSync(target).isDirectory()) return send(res, 404, '{"error":"not found"}');
    return send(res, 200, fs.readFileSync(target), mime[path.extname(target).toLowerCase()] ?? 'application/octet-stream');
  } catch (error) {
    return send(res, 400, JSON.stringify({ error: String(error?.message ?? error) }));
  }
});

server.listen(port, '127.0.0.1', () => {
  console.log(`ui-atlas: http://127.0.0.1:${port}/  (保存接口已启用)`);
});
