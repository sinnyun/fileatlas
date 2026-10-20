const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,step,delay,profile})=>{
 const invoke=async(t,a={})=>payload(await call(t,a));
 const data=JSON.parse(fs.readFileSync(path.join(profile,'workcrate-scenario.json')));
 const ready=async(screen)=>{for(let i=0;i<120;i++){const s=await invoke('workcrate-ui.state');if(s.ui?.screen===screen&&!s.ui.busy)return s;await delay(150);}throw Error('View failed to settle');};
 for(const [screen,args] of [['projects',{}],['pages',{project_id:data.projects[0].id}],['editor',{project_id:data.projects[0].id,board_id:data.ids[0]}]]){
  await invoke('workcrate-ui.navigate',args);await ready(screen);await step({tool:'workcrate-ui.state',wait_ms:900,capture:`${screen}-final`});
 }
 const before=await invoke('workcrate-ui.state');await delay(1200);const after=await invoke('workcrate-ui.state');assert.deepEqual(before.ui,after.ui,'Settled editor should remain quiet');
};
