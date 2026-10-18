// Real GPUI pointer interaction through same-process PhoTag MCP only.
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
module.exports = async ({step, delay, profile}) => {
  const invoke = async (tool, args = {}, extra = {}) => (await step({tool, arguments:args, ...extra})).result;
  const state = () => invoke('get_ui_state');
  const ui = (action, value) => invoke('control_ui', {action, value}, {wait_ms:180});
  const shot = name => invoke('get_ui_state', {}, {capture:name});
  const until = async check => {
    for (let n=0; n<100; n++) { const s=await state(); if(check(s)) return s; await delay(150); }
    throw Error('toolbar state timeout');
  };
  const click = async (x,y) => {
    for (const action of ['pointer_move','pointer_down','pointer_up']) {
      for(let retry=0;;retry++) try { await ui(action,`${x},${y}`); break; }
      catch(e) { if(retry>=5 || !String(e).includes('pointer request is still pending')) throw e; await delay(200); }
    }
  };
  const fixture=path.join(profile,'toolbar-fixture');
  fs.mkdirSync(path.join(fixture,'child'),{recursive:true});
  fs.writeFileSync(path.join(fixture,'one.txt'),'toolbar fixture');
  fs.writeFileSync(path.join(fixture,'child','two.txt'),'nested toolbar fixture');
  await delay(1300);
  await invoke('control_gallery',{action:'navigate',value:fixture});
  let s=await until(s=>s.gallery?.matched_count===1 && !s.browser.worker_active);
  const width=s.window_status.geometry.client_size.width;
  // Positions derived from the inspected rendered screenshot, logical client units.
  const right=width-s.inspector_width-18, first=right-240+15, y=81;
  await shot('main');
  await click(first+34,y); await until(s=>s.gallery.layout==='list'); await shot('list-selected');
  await click(first,y); await until(s=>s.gallery.layout==='grid');
  await click(first+68,y); await until(s=>s.browser.show_descendants && s.gallery.matched_count===2 && !s.browser.worker_active);
  await shot('subfolders-selected');
  await click(first+68,y); await until(s=>!s.browser.show_descendants && s.gallery.matched_count===1 && !s.browser.worker_active);
  const sessionFile=path.join(profile,'session.json');
  const before=JSON.parse(fs.readFileSync(sessionFile,'utf8')).thumb_size;
  await click(right-20,y); await delay(350);
  const after=JSON.parse(fs.readFileSync(sessionFile,'utf8')).thumb_size;
  assert.notEqual(after,before,'real relocated slider must change persisted size'); await shot('zoom-larger');
  await ui('pointer_move',`${first},${y}`); await delay(900); await shot('grid-tooltip');
  await click(width-205,28); await until(s=>s.gallery.search_entire_computer);
  await shot('search-entire-computer');
  await click(width-205,28); await until(s=>!s.gallery.search_entire_computer);
  await click(width-169,28); await until(s=>s.gallery.search_scopes_open); await shot('scope-popover');
  await click(1000,170); await until(s=>!s.gallery.search_scopes_open);
  await click(width-62,28); await until(s=>s.tools_open); await shot('tools');
  await click(width-62,28); await until(s=>!s.tools_open);
  await ui('pointer_move','1000,500'); await delay(700);
  const quiet=(await state()).gallery.snapshot_revision; await delay(900);
  assert.equal((await state()).gallery.snapshot_revision,quiet);
  await invoke('resize_window',{width:1510,height:1000},{wait_ms:600});
  await shot('compact');
  fs.writeFileSync(path.join(profile,'toolbar-summary.json'),JSON.stringify({passed:true,
    checks:['real relocated grid/list','real relocated descendants with 1→2→1 files','real relocated slider persistence',
      'icon tooltip','search location icon toggle','scope popover icon','tools icon','quiet snapshot','compact screenshot'],
    thumb_size:{before,after},quiet_revision:quiet,scale:s.window_status.geometry.scale_factor},null,2));
};
