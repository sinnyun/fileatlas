// Connect to the already-running GUI; never starts or stops FileAtlas.
const fs = require('node:fs');
const path = require('node:path');
const endpoint = process.env.FILEATLAS_MCP_URL || 'http://127.0.0.1:38473/mcp';
let session, id = 0;
async function request(method, params) {
  const response = await fetch(endpoint, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json, text/event-stream',
      ...(session ? { 'Mcp-Session-Id': session, 'MCP-Protocol-Version': '2025-03-26' } : {}) },
    body: JSON.stringify({ jsonrpc: '2.0', id: ++id, method, params }),
    signal: AbortSignal.timeout(30000),
  });
  if (!response.ok) throw Error(`${response.status}: ${await response.text()}`);
  session = response.headers.get('mcp-session-id') || session;
  const raw = await response.text();
  const message = response.headers.get('content-type')?.includes('text/event-stream')
    ? raw.split('\n').filter(line => line.startsWith('data: {')).map(line => JSON.parse(line.slice(6))).find(item => item.id === id)
    : JSON.parse(raw);
  if (!message || message.error) throw Error(JSON.stringify(message || raw));
  return message.result;
}
(async () => {
  await request('initialize', { protocolVersion: '2025-03-26', capabilities: {}, clientInfo: { name: 'fileatlas-http-debug', version: '1.0' } });
  const [tool = 'get_ui_state', args = '{}', output] = process.argv.slice(2);
  const result = tool === 'tools/list' ? await request(tool, {}) : await request('tools/call', { name: tool, arguments: JSON.parse(args) });
  if (output) {
    const absolute = path.resolve(output);
    const allowed = path.resolve('.local/validation') + path.sep;
    if (!absolute.startsWith(allowed)) throw Error('Output must be under .local/validation');
    fs.mkdirSync(path.dirname(absolute), { recursive: true });
    const image = result.content?.find(item => item.type === 'image');
    fs.writeFileSync(absolute, image ? Buffer.from(image.data, 'base64') : JSON.stringify(result, null, 2));
  }
  for (const item of result.content || []) {
    if (item.type === 'text') console.log(item.text);
    else if (item.type === 'image') console.log(JSON.stringify({ image: item.mimeType, output }));
  }
  if (tool === 'tools/list') console.log(JSON.stringify(result));
  if (result.isError) process.exitCode = 1;
})().catch(error => { console.error(error); process.exitCode = 1; });
