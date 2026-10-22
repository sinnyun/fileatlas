// Follow-up to workcrate-canvas-scenario.cjs in the same isolated profile.
// Real GPUI text input, right inspector, mode switching, local project covers.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
module.exports = async ({call, payload, step, delay, profile}) => {
  const fixture = JSON.parse(fs.readFileSync(path.join(profile, 'canvas-scenario.json')));
  const invoke = async (tool, args = {}) => payload(await call(tool, args));
  const read = (ids = []) => invoke('workcrate-ui.state', {control_ids: ids});
  const until = async (predicate, ids = []) => {
    for (let i=0;i<80;i++) { const s = await read(ids); if(predicate(s)) return s; await delay(150); }
    throw Error('Editor did not reach expected state');
  };
  const input = async (action, value) => {
    for(let i=0;;i++) {
      try { await invoke('control_ui', {action, value}); break; }
      catch(e) { if(i>30 || !String(e).includes('pending')) throw e; await delay(100); }
    }
    await delay(150);
  };
  const click = async id => {
    const s = await until(s => !s.ui?.busy && !s.ui?.canvas?.busy && s.controls?.[id], [id]);
    const [x,y,w,h] = s.controls[id].split(',').map(Number);
    await input('pointer_down', `${x+w/2},${y+h/2}`);
    await input('pointer_up', `${x+w/2},${y+h/2}`);
  };
  await invoke('workcrate-ui.open_board', {project_id: fixture.project, board_id: fixture.board});
  await click('workcrate-canvas-edit');
  const open = await until(s => s.ui?.canvas?.state && !s.ui.canvas.busy);
  const a = open.ui.canvas.state.artboard;
  const created = await invoke('workcrate-ui.canvas_create_text', {x:a.x+350,y:a.y+180,text:'编辑验证',size:24,confirm:true});
  const id = created.canvas.selection[0];
  await until(s => s.ui?.canvas?.object?.id===id && !s.ui.canvas.busy);
  await click('canvas-text-input');
  await input('key_press', 'ctrl-a');
  await input('input_text', '多行文字第一行');
  await input('key_press', 'enter');
  await input('input_text', '第二行修改成功');
  await click('workcrate-canvas-set-text');
  const edited = await until(s => s.ui?.canvas?.object?.text==='多行文字第一行\n第二行修改成功' && !s.ui.canvas.busy);
  assert.equal(edited.ui.boards.find(b=>b.id===fixture.board).canvas_revision, edited.ui.canvas.state.revision);
  await click('canvas-size-input');
  await input('input_text','32');
  const before = (await read()).ui.canvas.state.revision;
  await click('canvas-apply-style');
  await until(s=>s.ui?.canvas?.state?.revision>before && !s.ui.canvas.busy);
  await click('canvas-align-center');
  await until(s=>!s.ui?.canvas?.busy);
  await step({tool:'workcrate-ui.state',capture:'inspector-multiline-style'});
  await click('workcrate-canvas-preview');
  await step({tool:'workcrate-ui.state',capture:'preview-multiline-result'});
  await click('workcrate-canvas-edit');
  await step({tool:'resize_window', arguments:{width:1280,height:960},wait_ms:800,capture:'editor-small-window'});
  // Scroll the actual inspector, keeping the canvas and rail stationary.
  const boxState = await read(['canvas-text-input']);
  const [sx,sy,sw] = boxState.controls['canvas-text-input'].split(',').map(Number);
  await input('pointer_scroll',`${sx+sw/2},${Math.min(sy+20,800)},-600`);
  await step({tool:'workcrate-ui.state',capture:'editor-small-inspector-scroll'});
  await click('workcrate-pages-home');
  await until(s=>s.ui?.screen==='pages' && !s.ui.busy && !s.ui.preview_pending);
  await step({tool:'workcrate-ui.state',capture:'pages-updated-cached-previews'});
  await click('workcrate-projects-home');
  await until(s=>s.ui?.screen==='projects' && !s.ui.busy);
  await step({tool:'workcrate-ui.state',capture:'projects-local-page-cover'});
  const first = (await read()).ui;
  await delay(2000);
  assert.deepEqual((await read()).ui,first,'project view stays quiet');
  fs.writeFileSync(path.join(profile,'editor-refresh-result.json'),JSON.stringify({passed:true,textId:id,multiline:edited.ui.canvas.object.text},null,2));
};
