const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({step,delay,profile})=>{
  const previous=JSON.parse(fs.readFileSync(path.join(profile,'favorite-groups-summary.json'),'utf8'));
  let s;
  for(let n=0;n<120;n++){
    s=(await step({tool:'get_ui_state'})).result;
    if(s.sidebar_folders?.favorite_groups.some(g=>g.id===previous.work)&&!s.sidebar_folders.drives_pending)break;
    await delay(150);
  }
  await delay(700);s=(await step({tool:'get_ui_state',capture:'restarted'})).result;
  const groups=s.sidebar_folders.favorite_groups.map(g=>({id:g.id,name:g.name,expanded:g.expanded,folders:g.folders.map(f=>({id:f.id,path:f.path,display_name:f.display_name}))}));
  assert.deepEqual(groups,previous.groups,'groups/aliases/order/collapse/removal survive fresh process');
  assert.equal(groups.find(g=>g.id===previous.defaultId).folders.length,0,'removed legacy shortcut is not reimported');
  await step({tool:'control_ui',arguments:{action:'sidebar_request',request:{kind:'add_current_favorite',group_id:previous.ai}},expect_error:'PHOTAG_MCP_ALLOW_MUTATIONS'});
  fs.writeFileSync(path.join(profile,'favorite-groups-restart.json'),JSON.stringify({passed:true,groups,process:'fresh same-process MCP child, mutations disabled'},null,2));
};
