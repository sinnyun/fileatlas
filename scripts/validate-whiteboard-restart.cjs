const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
module.exports=async({call,payload,delay,step,request,profile})=>{
  const invoke=async(name,args={})=>payload(await call(name,args));
  const previous=JSON.parse(fs.readFileSync(path.join(profile,'whiteboard-result.json'),'utf8'));
  const initial=await invoke('whiteboard.state');
  const board=initial.library.boards.find(b=>b.id===previous.id);
  assert.deepEqual(JSON.parse(board.scene),previous.scene,'native scene must survive child-process restart');
  await invoke('whiteboard.open',{id:previous.id});
  await delay(800);
  await step({tool:'screenshot_window',capture:'whiteboard-restarted'});
  // Existing runtime registry owns stop/start and all native views/MCP registrations.
  await invoke('control_ui',{action:'plugin_enabled',value:'fileatlas.whiteboard:off'});
  let absent=false;
  for(let i=0;i<40;i++){
    const tools=(await request('tools/list',{})).tools;
    if(!tools.some(t=>t.name==='whiteboard.state')){absent=true;break;}
    await delay(100);
  }
  assert(absent,'plugin stop must unregister feature MCP tools');
  let stopped;
  for(let i=0;i<40;i++){
    stopped=await invoke('get_ui_state');
    if(!stopped.open_panel_ids.some(id=>id.startsWith('whiteboard/')) && !stopped.sidebar_folders.plugin_buttons.includes('whiteboard/sidebar-view'))break;
    await delay(100);
  }
  await step({tool:'screenshot_window',capture:'whiteboard-disabled'});
  assert(!stopped.open_panel_ids.some(id=>id.startsWith('whiteboard/')),'plugin stop must remove panel instances');
  assert(!stopped.sidebar_folders.plugin_buttons.includes('whiteboard/sidebar-view'),'plugin stop must remove rail entry');
  await invoke('control_ui',{action:'plugin_enabled',value:'fileatlas.whiteboard:on'});
  let restored;
  for(let i=0;i<40;i++){
    if((await request('tools/list',{})).tools.some(t=>t.name==='whiteboard.state')){restored=await invoke('whiteboard.state');break;}
    await delay(100);
  }
  assert(restored,'plugin must restart through Cordis');
  assert.deepEqual(JSON.parse(restored.library.boards.find(b=>b.id===previous.id).scene),previous.scene);
  // Click the registered rail entry in the actual FileAtlas 0.2 sidebar
  // (this host: 150% DPI, logical client coords). Navigation currently appends
  // a re-enabled contribution after its existing cached rail views.
  for(const action of ['pointer_move','pointer_down','pointer_up']){
    await invoke('control_ui',{action,value:'25,612'}); await delay(150);
  }
  await delay(500);
  await step({tool:'screenshot_window',capture:'whiteboard-rail-click'});
  assert((await invoke('get_ui_state')).open_panel_ids.some(id=>id.startsWith('whiteboard/panel@')),'rail click must open the native board panel');
  await step({tool:'screenshot_window',capture:'whiteboard-reenabled'});
  fs.writeFileSync(path.join(profile,'whiteboard-restart-result.json'),JSON.stringify({passed:true,cases:['native scene and active board survive process restart','Cordis stop removes tools, panel and rail','Cordis restart restores native scene','actual rail click opens the native board']},null,2));
  console.log(JSON.stringify({passed:true,restart:true,lifecycle:true,profile}));
  fs.copyFileSync(path.join(profile,'report.json'),path.join(profile,'whiteboard-restart-report.json'));
};
