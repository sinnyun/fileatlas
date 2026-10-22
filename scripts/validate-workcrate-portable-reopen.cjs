const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
module.exports=async({step,call,payload,delay,profile,root})=>{
  const source=process.env.FILEATLAS_PORTABLE_SOURCE||path.join(root,'.local/validation/profiles/wc-portable-2026-10-23/portable-summary.json');
  const data=JSON.parse(fs.readFileSync(source));
  const invoke=async(tool,args={},extra={})=>(await step({tool,arguments:args,wait_ms:150,...extra})).result;
  assert.equal(payload(await call('workcrate-ui.state',{})).projects.length,0);
  const result=await invoke('workcrate-ui.project_folder',{action:'open',folder:data.folder});
  assert.equal(result.projects[0].id,data.project_id);assert.equal(result.projects[0].name,'移动后重命名');
  const s=await invoke('workcrate-ui.state',{project_id:data.project_id});
  assert.equal(s.project_presets.fields[0].values[0],'便携类别');
  await invoke('workcrate-ui.navigate',{project_id:data.project_id});
  let ready=false;for(let i=0;i<120;i++){const s=payload(await call('workcrate-ui.state',{}));if(s.ui?.page_count===2&&s.ui.previews_ready===2&&!s.ui.busy){ready=true;assert.deepEqual(s.ui.boards.map(b=>b.id),[data.A,data.B]);break;}await delay(150);}assert.ok(ready);
  await invoke('workcrate-ui.state',{}, {capture:'portable-fresh-profile-pages'});
  fs.writeFileSync(path.join(profile,'portable-reopen-summary.json'),JSON.stringify({passed:true,source,checks:['fresh profile starts with no projects','recovered original project identity and renamed title','project presets and cached page previews retained','no VectorCraft configuration or process required']},null,2));
};
