const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const net = require('node:net');
module.exports = async ({step,profile,root,delay}) => {
  const invoke=async(tool,args={},extra={})=>(await step({tool,arguments:args,wait_ms:100,...extra})).result;
  const port=await new Promise(resolve=>{const s=net.createServer();s.listen(0,'127.0.0.1',()=>{const p=s.address().port;s.close(()=>resolve(p));});});
  const directory=path.join(profile,'documents');fs.mkdirSync(directory,{recursive:true});
  await invoke('workcrate-ui.configure_vectorcraft',{executable:'C:\\Program Files\\VectorCraft\\vectorcraft.exe',port});
  const created=await invoke('workcrate-ui.manage_project',{action:'create',name:'黑色物料栏',directory});
  const project_id=created.project.id;
  let board_id=created.boards[0].id;
  const open=async()=>{
    await invoke('workcrate-ui.open_board',{project_id,board_id});
    for(let i=0;i<100;i++){
      const s=await invoke('workcrate-ui.state');
      if(s.ui?.canvas?.state?.board_id===board_id&&!s.ui.canvas.busy&&!s.ui.busy)return;
      await delay(150);
    }
    throw Error('canvas did not settle');
  };
  const black=s=>{for(const n of Object.values(s.nodes))assert.equal(n.summary.fill,'#000000');};
  await open();
  black(await invoke('workcrate-ui.header_state'));
  await invoke('workcrate-ui.update_page',{project_id,board_id,info:{material_category:'展示物料',material:'人物立牌',size:'20 × 30 cm',quantity:'2 套'}});
  black(await invoke('workcrate-ui.header_state'));
  await invoke('workcrate-ui.state',{}, {capture:'black-header-new'});
  const source=path.join(root,'.local/validation/profiles/wc-single-line-2026-10-23-values/documents/legacy.vectorcraft');
  assert.ok(fs.existsSync(source),'requires the previous single-line validation fixture');
  await invoke('workcrate-ui.navigate',{project_id});
  const legacy=path.join(directory,'old-color.vectorcraft');fs.copyFileSync(source,legacy);
  await invoke('workcrate-ui.attach_document',{project_id,path:legacy});
  const refreshed=await invoke('workcrate-ui.refresh_project',{project_id});board_id=refreshed.boards[0].id;
  await open();
  const before=await invoke('workcrate-ui.header_state');assert.equal(before.layout,2);
  assert.ok(Object.values(before.nodes).some(n=>n.summary.fill!=='#000000'));
  await invoke('workcrate-ui.update_page',{project_id,board_id,info:{quantity:'2 套'}});
  const after=await invoke('workcrate-ui.header_state');black(after);
  assert.deepEqual(after.furniture,before.furniture);
  for(const role of Object.keys(before.nodes))assert.equal(after.nodes[role].id,before.nodes[role].id);
  assert.equal(after.last_write.report.initialized,false);
  assert.equal(after.last_write.report.commands.filter(c=>c.command==='text.setStyle').length,1);
  await invoke('workcrate-ui.update_page',{project_id,board_id,info:{quantity:'3 套'}});
  const patched=await invoke('workcrate-ui.header_state');black(patched);
  assert.ok(!patched.last_write.report.commands.some(c=>c.command==='text.setStyle'));
  await invoke('workcrate-ui.state',{}, {capture:'black-header-existing'});
  fs.writeFileSync(path.join(profile,'black-header-summary.json'),JSON.stringify({passed:true,newBlack:true,existingRecoloredInPlace:true,normalPatchDoesNotRestyle:true},null,2));
};
