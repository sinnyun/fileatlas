const fs=require('node:fs');
const path=require('node:path');
const assert=require('node:assert/strict');
module.exports=async({call,payload,step,delay,profile})=>{
  const data=JSON.parse(fs.readFileSync(path.join(profile,'workcrate-scenario.json'),'utf8'));
  const invoke=async(tool,args={})=>payload(await call(tool,args));
  const project_id=data.projects[0].id;
  await invoke('workcrate-ui.navigate',{project_id,board_id:data.ids[0]});
  let state;
  for(let i=0;i<100;i++){state=await invoke('workcrate-ui.state');if(state.ui?.screen==='editor'&&!state.ui.busy&&state.ui.page_count===6)break;await delay(200);}
  await delay(500);
  const ui=await invoke('get_ui_state');
  const [x,y]=ui.panel_instances.find(p=>p.id.startsWith('workcrate/overview@')).client_bounds.split(',').map(Number);
  for(const action of ['pointer_down','pointer_up']){await invoke('control_ui',{action,value:`${x+1038},${y+350}`});await delay(200);}
  state=await invoke('workcrate-ui.state');
  assert.deepEqual(state.ui.editing,[project_id,'project']);
  await step({tool:'workcrate-ui.state',capture:'editor-global-name'});
  console.log('Global name inline editor opened through native MCP pointer');
};
