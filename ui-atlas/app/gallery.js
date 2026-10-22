// FileAtlas 界面图谱：分类浏览、检索、明暗切换、网页内编辑回写与版本历史。
const TOKEN_BLOCK = /<!--\s*FA:TOKENS:BEGIN\s*-->[\s\S]*?<!--\s*FA:TOKENS:END\s*-->\s*/;
const http = location.protocol !== 'file:';
const state = { cat: '全部', group: '全部', status: '全部', q: '', id: null, view: 'grid' };
let manifest = window.UI_ATLAS;
let sources = window.UI_ATLAS_SVGS ?? {};

const $ = id => document.getElementById(id);
const esc = text => String(text).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

function entries() {
  return Object.values(manifest.entries);
}

function filtered() {
  const q = state.q.trim().toLowerCase();
  return entries()
    .filter(item => state.cat === '全部' || item.category === state.cat)
    .filter(item => state.group === '全部' || item.group === state.group)
    .filter(item => state.status === '全部' || statusLabel(item) === state.status)
    .filter(item => !q || [item.title, item.id, item.owner, item.category, item.group, item.spec].join(' ').toLowerCase().includes(q))
    .sort((a, b) => a.id.localeCompare(b.id));
}

function statusLabel(item) {
  return { traced: '已描摹', todo: '待描摹', missing: '文件缺失' }[item.status] ?? item.status;
}

function bare(id) {
  const source = sources[id];
  if (!source) return null;
  return source.replace(TOKEN_BLOCK, '');
}

function renderTree() {
  const byCat = new Map();
  for (const item of entries()) {
    const cat = byCat.get(item.category) ?? { total: 0, traced: 0, groups: new Map() };
    cat.total += 1;
    if (item.status === 'traced') cat.traced += 1;
    const group = cat.groups.get(item.group) ?? { total: 0, traced: 0 };
    group.total += 1;
    if (item.status === 'traced') group.traced += 1;
    cat.groups.set(item.group, group);
    byCat.set(item.category, cat);
  }
  const cats = [...byCat.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  const stats = manifest.stats;
  const html = [
    `<div class="progress"><i style="width:${Math.round((stats.traced / stats.total) * 100)}%"></i></div>`,
    `<div class="note">全库 ${stats.total} 项 · 已描摹 ${stats.traced} · 待描摹 ${stats.todo}${stats.missing ? ` · 缺失 ${stats.missing}` : ''}</div>`,
    `<div class="cat"><h2>浏览</h2>`,
    `<button data-cat="全部" data-group="全部" class="${state.cat === '全部' ? 'on' : ''}"><span>全部条目</span><span class="count">${stats.total}</span></button>`,
    `<button data-view="changes" class="${state.view === 'changes' ? 'on' : ''}"><span>变更记录</span><span class="count">${manifest.catalogRevision}</span></button>`,
    `<button data-view="tokens" class="${state.view === 'tokens' ? 'on' : ''}"><span>设计令牌</span><span class="count">${Object.keys(manifest.tokens.light).length}</span></button>`,
    `</div>`,
  ];
  for (const [cat, info] of cats) {
    html.push(`<div class="cat"><h2>${esc(cat)} ${info.traced}/${info.total}</h2>`);
    html.push(
      `<button data-cat="${esc(cat)}" data-group="全部" class="${state.cat === cat && state.group === '全部' ? 'on' : ''}"><span>全部</span><span class="count">${info.total}</span></button>`,
    );
    for (const [group, g] of [...info.groups.entries()].sort((a, b) => a[0].localeCompare(b[0]))) {
      html.push(
        `<button data-cat="${esc(cat)}" data-group="${esc(group)}" class="${state.cat === cat && state.group === group ? 'on' : ''}"><span>${esc(group)}</span><span class="count">${g.traced}/${g.total}</span></button>`,
      );
    }
    html.push(`</div>`);
  }
  $('tree').innerHTML = html.join('');
  $('tree').querySelectorAll('button').forEach(button => {
    button.addEventListener('click', () => {
      if (button.dataset.view) state.view = button.dataset.view;
      else {
        state.view = 'grid';
        state.cat = button.dataset.cat;
        state.group = button.dataset.group;
      }
      render();
    });
  });
}

function card(item) {
  const svg = bare(item.id);
  const thumb = svg ?? `<div class="note">${esc(item.spec ?? '尚未描摹')}</div>`;
  return `<article class="card ${item.status}" data-id="${esc(item.id)}" title="${esc(item.id)}">
    <div class="thumb">${thumb}</div>
    <div class="name">${esc(item.title)}</div>
    <div class="meta">
      <span class="badge ${item.status}">${statusLabel(item)}</span>
      <span>${item.version ? `v${item.version}` : '未建版'}</span>
      <span>${esc(item.group)}</span>
      ${svg ? '' : `<span class="note">${esc(item.owner)}</span>`}
    </div>
  </article>`;
}

function renderGrid() {
  const list = filtered();
  $('main').innerHTML = `<div class="grid">${list.map(card).join('') || '<div class="note">没有匹配的条目</div>'}</div>`;
  $('main').querySelectorAll('.card').forEach(node => node.addEventListener('click', () => select(node.dataset.id)));
}

function renderChanges() {
  const rows = [];
  for (const item of entries()) {
    for (const entry of item.history ?? []) rows.push({ ...entry, id: item.id, title: item.title });
  }
  rows.sort((a, b) => String(b.at).localeCompare(String(a.at)));
  $('main').innerHTML = `<h2>变更记录</h2><p class="note">目录修订号 ${manifest.catalogRevision}。内容哈希变化自动升修订号，尺寸变化升次版本号。</p>
    <div class="rows">${rows.slice(0, 200).map(row => `<div class="row"><b>v${esc(row.version)}</b><span>${esc(row.title)}</span><code>${esc(row.id)}</code><span class="note">${esc(row.note ?? '')} · ${esc(String(row.at).slice(0, 16).replace('T', ' '))}</span></div>`).join('')}</div>`;
}

function renderTokens() {
  const mode = document.documentElement.dataset.faTheme === 'light' ? 'light' : 'dark';
  const roles = manifest.tokens[mode];
  const geometry = manifest.tokens.geometry;
  $('main').innerHTML = `<h2>设计令牌</h2>
    <p class="note">颜色角色取自实机 GPUI Kit 主题（${esc(manifest.app['theme' + (mode === 'light' ? 'Light' : 'Dark')] ?? '')}，转储时间 ${esc(manifest.app.tokenCapturedAt[mode] ?? '')}）；几何常量解析自 crates/fileatlas-ui-bridge/src/theme.rs。</p>
    <h3>颜色角色 ${Object.keys(roles).length}</h3>
    <div class="swatches">${Object.entries(roles).map(([name, value]) => `<div class="swatch"><i style="background:${esc(value)}"></i><code>${esc(name)}</code><span class="note">${esc(value)}</span></div>`).join('')}</div>
    <h3>几何与字号 ${Object.keys(geometry).length}</h3>
    <div class="swatches">${Object.entries(geometry).map(([name, value]) => `<div class="swatch"><code>${esc(name)}</code><span class="note">${esc(value)}</span></div>`).join('')}</div>`;
}

function select(id) {
  state.id = id;
  renderDetail();
}

function renderDetail() {
  const panel = $('detail');
  const item = manifest.entries[state.id];
  if (!item) {
    panel.hidden = true;
    panel.innerHTML = '';
    return;
  }
  panel.hidden = false;
  const svg = bare(item.id);
  const source = sources[item.id] ?? '';
  panel.innerHTML = `
    <div class="toolbar"><h2>${esc(item.title)}</h2><span class="badge ${item.status}">${statusLabel(item)}</span>${item.version ? `<span class="chip">v${esc(item.version)}</span>` : ''}</div>
    <div class="preview">${svg ?? '<div class="note">该条目尚未描摹为 SVG。</div>'}</div>
    <div class="kv">
      <b>组件标识</b><code>${esc(item.id)}</code>
      <b>分类</b><span>${esc(item.category)} / ${esc(item.group)}</span>
      <b>归属</b><span>${esc(item.owner || '—')}</span>
      <b>源文件</b><span>${(item.source ?? []).map(path => `<code>${esc(path)}</code>`).join('、') || '—'}</span>
      <b>规格</b><span>${esc(item.spec || '—')}</span>
      <b>尺寸</b><span>${item.size ? `${item.size.width} × ${item.size.height}` : '—'}</span>
      <b>文件</b><code>${esc(item.file)}</code>
      <b>哈希</b><code>${esc(item.sha256 ?? '—')}</code>
      <b>更新时间</b><span>${esc(item.updatedAt ?? '—')}</span>
    </div>
    ${svg ? `<h3>编辑</h3>
      <textarea id="src" class="area" spellcheck="false">${esc(source)}</textarea>
      <div class="toolbar">
        <input id="note" class="field" placeholder="本次修改说明（写入版本历史）">
        <button id="save" class="on">保存并升版本</button>
        <button id="revert">还原</button>
        <button id="copy">复制 SVG</button>
        <span id="msg" class="note"></span>
      </div>
      <p class="note">${http ? '保存会写回 svg/ 下同名文件，内容哈希变化即自动升版本号并刷新本页目录。' : '直接以 file:// 打开时无法回写，运行 npm run serve 后在浏览器里保存。'}</p>` : ''}
    <h3>版本历史</h3>
    <div class="rows">${(item.history ?? []).slice().reverse().map(entry => `<div class="row"><b>v${esc(entry.version)}</b><span class="note">${esc(entry.note ?? '')} · ${esc(String(entry.at).slice(0, 16).replace('T', ' '))}</span><code>${esc(entry.sha)}</code></div>`).join('') || '<div class="note">暂无</div>'}</div>
    <div class="toolbar"><button id="close" class="ghost">关闭详情</button></div>`;
  $('close').addEventListener('click', () => {
    state.id = null;
    renderDetail();
  });
  $('save')?.addEventListener('click', save);
  $('revert')?.addEventListener('click', () => {
    $('src').value = source;
    $('msg').textContent = '已还原为磁盘内容';
  });
  $('copy')?.addEventListener('click', async () => {
    await navigator.clipboard.writeText(source);
    $('msg').textContent = '已复制';
  });
}

async function save() {
  const item = manifest.entries[state.id];
  const message = $('msg');
  message.textContent = '保存中…';
  try {
    const response = await fetch('/api/save', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ id: item.id, content: $('src').value, note: $('note').value || null }),
    });
    const result = await response.json();
    if (!response.ok || result.error) throw new Error(result.error ?? `HTTP ${response.status}`);
    await reload();
    message.textContent = result.from && result.from !== result.version ? `已保存 v${result.from} → v${result.version}` : `已保存 v${result.version}`;
  } catch (error) {
    message.textContent = `保存失败：${error.message}`;
  }
}

async function reload() {
  if (!http) return;
  const [nextManifest, nextSources] = await Promise.all([
    fetch('/catalog/manifest.json').then(response => response.json()),
    fetch('/catalog/sources.js').then(response => response.text()),
  ]);
  manifest = nextManifest;
  const marker = 'window.UI_ATLAS_SVGS =';
  const json = nextSources.slice(nextSources.indexOf(marker) + marker.length).trim().replace(/;$/, '');
  sources = JSON.parse(json);
  render();
}

function render() {
  $('revision').textContent = `目录修订 ${manifest.catalogRevision}`;
  renderTree();
  if (state.view === 'changes') renderChanges();
  else if (state.view === 'tokens') renderTokens();
  else renderGrid();
  renderDetail();
}

function boot() {
  const stored = localStorage.getItem('fa-atlas-theme');
  document.documentElement.dataset.faTheme = stored ?? 'dark';
  $('theme').addEventListener('click', () => {
    document.documentElement.dataset.faTheme = document.documentElement.dataset.faTheme === 'light' ? 'dark' : 'light';
    localStorage.setItem('fa-atlas-theme', document.documentElement.dataset.faTheme);
    if (state.view === 'tokens') renderTokens();
  });
  $('search').addEventListener('input', event => {
    state.q = event.target.value;
    if (state.view !== 'grid') state.view = 'grid';
    renderGrid();
  });
  $('status').addEventListener('click', event => {
    const button = event.target.closest('button');
    if (!button) return;
    state.status = button.dataset.status;
    [...$('status').children].forEach(node => node.classList.toggle('on', node === button));
    renderGrid();
  });
  if (!http) $('mode').textContent = '只读（file:// 打开）';
  render();
}

boot();
