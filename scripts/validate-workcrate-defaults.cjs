// Same-process MCP: native material category, scoped defaults and one-column promotion.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const net = require('node:net');
module.exports = async ({ call, payload, step, delay, profile }) => {
  const ui = async (action, value) => {
    for (let i = 0; i < 40; i++) {
      try { return (await step({ tool: 'control_ui', arguments: { action, ...(value === undefined ? {} : { value }) }, wait_ms: 100 })).result; }
      catch (e) { if (!String(e).includes('still pending')) throw e; await delay(150); }
    }
    throw Error('UI input queue did not drain');
  };
  const read = async (ids = [], project) => payload(await call('workcrate-ui.state', { control_ids: ids, ...(project ? { project_id: project } : {}) }));
  const until = async (test, label, ids = [], project) => {
    let state;
    for (let i = 0; i < 100; i++) { state = await read(ids, project); if (test(state)) return state; await delay(150); }
    throw Error(`${label}: ${JSON.stringify(state)}`);
  };
  const click = async (x, y) => { await ui('pointer_down', `${x},${y}`); await ui('pointer_up', `${x},${y}`); };
  const clickId = async id => {
    const state = await until(s => s.controls[id], `control ${id}`, [id]);
    const [x,y,w,h] = state.controls[id].split(',').map(Number);
    await click(x+w/2, y+h/2);
  };
  const type = async (id, value) => { await clickId(id); await ui('key_press','ctrl-a'); if (value) await ui('input_text',value); else await ui('key_press','backspace'); };
  const capture = name => step({ tool:'workcrate-ui.state', arguments:{}, capture:name });
  await delay(1200);
  await ui('settings_open'); await ui('settings_plugin_page','fileatlas.workcrate-ui');
  await until(s => s.defaults_form, 'plugin settings mounted');
  await capture('program-settings');
  await ui('settings_close');
  const port = await new Promise(resolve => { const server=net.createServer(); server.listen(0,'127.0.0.1',()=>{const p=server.address().port;server.close(()=>resolve(p));}); });
  await step({tool:'workcrate-ui.configure_vectorcraft',arguments:{executable:'C:\\Program Files\\VectorCraft\\vectorcraft.exe',port}});
  const documents=path.join(profile,'documents'); fs.mkdirSync(documents,{recursive:true});
  const create = async name => (await step({tool:'workcrate-ui.manage_project',arguments:{action:'create',name,directory:documents}})).result;
  const a=await create('默认信息测试A'), b=await create('默认信息测试B');
  const project=a.project.id, board=a.boards[0].id;
  await step({tool:'workcrate-ui.open_board',arguments:{project_id:project,board_id:board}});
  await until(s => s.ui?.board_id===board && !s.ui.busy && !s.ui.canvas.busy && s.ui.previews_ready===1,'editor ready');
  await clickId('field-local'); await type('field-edit-input','活动展示'); await ui('key_press','enter');
  await until(s => !s.ui.field_saves.length && s.ui.boards[0].info.material_category==='活动展示','category saved');
  await step({tool:'workcrate-ui.manage_project',arguments:{action:'rename',project_id:project,name:'项目标题已改变'}});
  await delay(600);
  assert.equal((await read()).ui.boards[0].info.material_category,'活动展示');
  await capture('independent-category');
  await clickId('project-defaults');
  await until(s => s.defaults_form?.project_id===project,'project defaults mounted');
  await clickId('settings-add:local');
  await type('settings-value-local-0','专用类别');
  await until(s=>!s.defaults_form.pending && !s.defaults_form.busy && s.project_presets.fields[0].values[0]==='专用类别','project default saved',[],project);
  await type('settings-value-local-0','修改后的专用类别');
  await until(s=>!s.defaults_form.pending && !s.defaults_form.busy && s.project_presets.fields[0].values[0]==='修改后的专用类别','project default modified',[],project);
  const before=await read([],project);
  assert.ok(!before.field_presets.fields[0].values.includes('修改后的专用类别'));
  assert.ok(!(await read([],b.project.id)).effective_presets.fields[0].values.includes('修改后的专用类别'));
  await capture('project-defaults');
  await clickId('settings-promote-local');
  await until(s=>s.field_presets.fields[0].values.includes('修改后的专用类别'),'column promoted');
  const promoted=await read([],project);
  assert.deepEqual(promoted.field_presets.fields.slice(1),before.field_presets.fields.slice(1));
  assert.equal(promoted.effective_presets.fields[0].values.filter(v=>v==='修改后的专用类别').length,1);
  const host = payload(await call('get_ui_state', {}));
  await ui('panel_close',host.open_panel_ids.find(id=>id.startsWith('workcrate/project-defaults@')));
  await clickId('preset-local'); await capture('category-menu');
  // Native dropdown keyboard selection uses the same visible menu as a pointer choice.
  const menuState=await read(['preset-local']);
  const [mx,my,mw,mh]=menuState.controls['preset-local'].split(',').map(Number);
  await click(mx+mw/2,my+mh+16);
  await until(s=>!s.ui.field_saves.length && s.ui.boards[0].info.material_category==='修改后的专用类别','preset applied');
  await clickId('field-local'); await type('field-edit-input',''); await ui('key_press','enter');
  await until(s=>!s.ui.field_saves.length && s.ui.boards[0].info.material_category==='','category cleared without inheritance');
  await capture('empty-category');
  await step({tool:'workcrate-ui.navigate',arguments:{project_id:project}});
  await step({tool:'workcrate-ui.refresh_project',arguments:{project_id:project}});
  await step({tool:'workcrate-ui.open_board',arguments:{project_id:project,board_id:board}});
  await until(s=>s.ui?.board_id===board && !s.ui.busy && !s.ui.canvas.busy && s.ui.previews_ready===1,'native category reopened');
  assert.equal((await read()).ui.boards[0].info.material_category,'');
  const quiet=await read(); await delay(3500); const settled=await read();
  assert.equal(settled.ui.reloads,quiet.ui.reloads);
  fs.writeFileSync(path.join(profile,'defaults-summary.json'),JSON.stringify({passed:true,project,other:b.project.id,board,checks:['plugin settings','independent category','project add/modify','project isolation','single-column promotion','deduplicated dropdown','native cleared category roundtrip','quiet reloads']},null,2));
};
