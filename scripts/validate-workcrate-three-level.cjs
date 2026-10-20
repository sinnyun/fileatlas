// Runs only through the same-process FileAtlas MCP using an isolated profile.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

module.exports = async ({call,payload,step,delay,root,profile}) => {
  const invoke = async (tool,args={}) => payload(await call(tool,args));
  const fixture = path.join(root,'test-data/fileatlas-fixtures/03-nested/level-one/level-two/nested-photo.jpeg');
  const second = path.join(root,'test-data/fileatlas-fixtures/01-small/small.png');
  const artifacts = path.join(profile,'artifacts');fs.mkdirSync(artifacts,{recursive:true});
  const projects=[];
  for (const name of ['1024 工程师文化节','2026 新品节项目','2022 新品开放项目']) {
    const result=await invoke('workcrate-ui.manage_project',{action:'create',name});
    projects.push(result.project);
    await invoke('workcrate-ui.manage_project',{action:'cover',project_id:result.project.id,path:fixture});
  }
  const project_id=projects[0].id, ids=[];
  for(let i=0;i<6;i++) {
    const page=await invoke('workcrate-ui.create_page',{project_id});ids.push(page.board_id);
    assert(Math.abs(page.width_pt-420*72/25.4)<0.01);
    assert(Math.abs(page.height_pt-297*72/25.4)<0.01);
    await invoke('workcrate-ui.place_image',{project_id,board_id:page.board_id,path:fixture});
    await invoke('workcrate-ui.update_page',{project_id,board_id:page.board_id,title:['主题背景.jpg','人物立牌.jpg','游戏区.jpg','打卡墙.jpg','活动日程.jpg','指引牌.jpg'][i],info:{material:'人物立牌',size:'80 × 180 cm',quantity:'20 套'}});
  }
  await invoke('workcrate-ui.place_image',{project_id,board_id:ids[0],path:second,x_pt:600,y_pt:280,width_pt:200});
  await invoke('workcrate-ui.update_page',{project_id,board_id:ids[0],info:{project_name_override:'局部名称示例'}});
  await invoke('workcrate-ui.manage_project',{action:'rename',project_id,name:'1024 工程师文化节'});
  const state=await invoke('workcrate-ui.state');
  const project=state.projects.find(p=>p.id===project_id);
  assert.equal(project.boards.length,6);
  const first=project.boards.find(p=>p.id===ids[0]);
  assert.equal(first.info.project_name_override,'局部名称示例');
  assert.deepEqual(first.info.physical_size_cm,[42,29.7]);
  assert(first.info.pixel_size.every(value=>value>0));
  const copy=await invoke('workcrate-ui.manage_project',{action:'duplicate',project_id,name:'复制验证项目'});
  const copied=(await invoke('workcrate-ui.state')).projects.find(p=>p.id===copy.project.id);
  assert.equal(copied.boards.length,6);
  assert(copied.boards.every(p=>!ids.includes(p.id)));
  await invoke('workcrate-ui.manage_project',{action:'archive',project_id:copy.project.id});
  await invoke('workcrate-ui.manage_project',{action:'restore',project_id:copy.project.id});
  await invoke('workcrate-ui.delete_project',{project_id:copy.project.id,confirm:true});
  assert(fs.existsSync(fixture));
  const ready=async screen=>{for(let i=0;i<100;i++){const state=await invoke('workcrate-ui.state');if(state.ui?.screen===screen&&!state.ui.busy&&(screen==='projects'||state.ui.page_count===6))return;await delay(200);}throw Error(`WorkCrate ${screen} did not settle`);};
  await invoke('workcrate-ui.navigate');await ready('projects');
  await step({tool:'workcrate-ui.navigate',wait_ms:1000,capture:'projects'});
  await invoke('workcrate-ui.navigate',{project_id});await ready('pages');
  await step({tool:'workcrate-ui.state',wait_ms:300,capture:'pages'});
  await invoke('workcrate-ui.navigate',{project_id,board_id:ids[0]});await ready('editor');
  await step({tool:'workcrate-ui.state',wait_ms:1000,capture:'editor'});
  await step({tool:'workcrate-ui.export_pages',arguments:{project_id,board_ids:ids,path:path.join(artifacts,'all-pages.pdf'),format:'pdf',confirm:true}});
  await step({tool:'workcrate-ui.export_pages',arguments:{project_id,board_ids:[ids[0]],path:path.join(artifacts,'first-page.png'),format:'image',confirm:true}});
  await step({tool:'workcrate-ui.export_pages',arguments:{project_id,board_ids:[ids[0]],path:path.join(artifacts,'first-page.jpg'),format:'image',confirm:true}});
  const before=await invoke('get_ui_state');await delay(1200);const after=await invoke('get_ui_state');
  fs.writeFileSync(path.join(profile,'workcrate-scenario.json'),JSON.stringify({projects,ids,source:first.info.pixel_size,checks:['A3','first image metadata','local name override','duplicate page identities','archive/restore','delete retains sources','PDF/PNG/JPEG export'],quietBefore:before,quietAfter:after},null,2));
  console.log(JSON.stringify({project_id,board_ids:ids,profile,checks:'passed'}));
};
