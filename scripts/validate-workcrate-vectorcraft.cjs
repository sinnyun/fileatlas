// Same-process FileAtlas MCP validation; requires an isolated official VectorCraft instance.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({call,payload,step,delay,profile,root}) => {
  const invoke = async (tool,args={}) => payload(await call(`workcrate-ui.${tool}`,args));
  const executable = process.env.FILEATLAS_VECTORCRAFT_EXE;
  assert(executable && fs.existsSync(executable), 'Set FILEATLAS_VECTORCRAFT_EXE to the official executable');
  const artifacts = path.join(profile,'artifacts'); fs.mkdirSync(artifacts,{recursive:true});
  await invoke('configure_vectorcraft',{executable,port:Number(process.env.WORKCRATE_NATIVE_PORT || 17979)});
  assert.equal((await invoke('connect_vectorcraft')).connected,true);
  const {project} = await invoke('manage_project',{action:'create',name:'原生 VectorCraft 流程验证'});
  const project_id = project.id;
  const first = await invoke('create_page',{project_id});
  const second = await invoke('create_page',{project_id});
  assert.notEqual(first.board_id,second.board_id);
  assert(Math.abs(first.width_pt - 420*72/25.4)<0.1);
  const source=path.join(artifacts,'source-photo.jpeg');
  fs.copyFileSync(path.join(root,'test-data/fileatlas-fixtures/03-nested/level-one/level-two/nested-photo.jpeg'),source);
  await invoke('place_image',{project_id,board_id:first.board_id,path:source});
  await invoke('update_page',{project_id,board_id:first.board_id,title:'人物立牌',info:{material:'写真裱板',size:'80 × 180 cm',quantity:'2 套',extra:{备注:'中文物料栏验证'}}});
  await invoke('update_page',{project_id,board_id:second.board_id,title:'第二页订单',info:{material:'第二页标识',quantity:'1 套'}});
  const opened = await invoke('open_board',{project_id,board_id:first.board_id});
  assert.equal(opened.board.title,'人物立牌');
  assert.equal(opened.board.info.material,'写真裱板');
  assert.equal(opened.board.info.extra['备注'],'中文物料栏验证');
  const ready = async screen => {
    for(let i=0;i<120;i++) {
      const state = await invoke('state');
      if(state.ui?.screen===screen && !state.ui.busy && state.ui.preview_pending===0) return state;
      await delay(250);
    }
    throw Error(`WorkCrate ${screen} did not settle`);
  };
  await invoke('navigate'); await ready('projects');
  await step({tool:'workcrate-ui.state',capture:'native-projects'});
  await invoke('navigate',{project_id}); const pages = await ready('pages');
  assert.equal(pages.ui.page_count,2);
  await step({tool:'workcrate-ui.state',capture:'native-pages'});
  await invoke('open_board',{project_id,board_id:first.board_id}); await ready('editor');
  const ui = async(action,value) => {
    for(let i=0;;i++) {
      try {await call('control_ui',{action,value}); await delay(100); return;}
      catch(error) {if(i>40 || !String(error).includes('pending')) throw error; await delay(100);}
    }
  };
  const click = async id => {
    const state = await invoke('state',{control_ids:[id]});
    const raw = state.controls[id]; assert(raw,`Missing measured control ${id}`);
    const [x,y,w,h] = raw.split(',').map(Number);
    await ui('pointer_down',`${x+w*.65},${y+h/2}`);
    await ui('pointer_up',`${x+w*.65},${y+h/2}`);
  };
  await click('field-quantity');
  await step({tool:'workcrate-ui.state',capture:'native-quantity-edit-open'});
  await click('field-edit-input');
  await ui('key_press','ctrl-a'); await ui('input_text','3 套');
  await step({tool:'workcrate-ui.state',capture:'native-quantity-edit-typed'});
  await click('field-edit-save');
  for(let i=0;i<120;i++) {
    const state=await invoke('state');
    if(!state.ui.busy && state.ui.boards?.find(b=>b.id===first.board_id)?.info.quantity==='3 套') break;
    if(i===119) {await step({tool:'workcrate-ui.state',capture:'native-quantity-edit-failed'}); throw Error('Visible quantity save did not settle');}
    await delay(250);
  }
  await ready('editor');
  const edited=await invoke('open_board',{project_id,board_id:first.board_id});
  assert.equal(edited.board.info.quantity,'3 套','Visible form must update native text');
  await ready('editor');
  await step({tool:'workcrate-ui.state',capture:'native-preview-and-material-fields'});
  await invoke('save_project',{project_id,confirm:true});
  for(const [format,name,ids] of [['pdf','native-artboards.pdf',[second.board_id,first.board_id]],['image','native-page.png',[first.board_id]],['image','native-page.jpg',[first.board_id]]]) {
    const output = path.join(artifacts,name);
    await invoke('export_pages',{project_id,board_ids:ids,path:output,format,confirm:true});
    assert(fs.statSync(output).size>100);
  }
  const copy = await invoke('manage_project',{action:'duplicate',project_id,name:'原生工程副本'});
  await invoke('navigate',{project_id:copy.project.id});
  const copied = await ready('pages'); assert.equal(copied.ui.page_count,2);
  await invoke('manage_project',{action:'archive',project_id:copy.project.id});
  await invoke('manage_project',{action:'restore',project_id:copy.project.id});
  await invoke('delete_project',{project_id:copy.project.id,confirm:true});
  await invoke('navigate',{project_id,board_id:first.board_id}); await ready('editor');
  await invoke('open_editor',{project_id,board_id:first.board_id});
  await invoke('save_project',{project_id,confirm:true}); await ready('editor');
  const quietBefore=(await invoke('state')).ui;
  await delay(1500);
  assert.deepEqual((await invoke('state')).ui,quietBefore,'Settled WorkCrate snapshot must stay quiet');
  fs.writeFileSync(path.join(profile,'native-validation.json'),JSON.stringify({project_id,boards:[first.board_id,second.board_id],checks:['official connection','native artboards','Chinese material fields readback','three-level UI','official PDF PNG JPEG','native document duplication','archive restore delete','visible material form save','official editor handoff','quiet settled snapshot']},null,2));
};
