// Run against a previously populated performance profile, in a new MCP process.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,step,delay,profile})=>{
 const invoke=async(t,a={})=>payload(await call(t,a));
 const until=async(test)=>{for(let i=0;i<300;i++){const s=await invoke('workcrate-ui.state');if(test(s))return s;await delay(100);}throw Error('Reopen did not settle');};
 const saved=JSON.parse(fs.readFileSync(path.join(profile,'performance-result.json')));
 const source=path.join(profile,'fixtures','large.jpg');
 fs.renameSync(source,source+'.offline');
 try{
   await invoke('workcrate-ui.navigate',{project_id:saved.project_id});
   const s=await until(s=>s.ui?.screen==='pages'&&!s.ui.busy&&s.ui.preview_cache_hits===3);
   assert.equal(s.ui.previews_ready,3);assert.equal(s.ui.display_images,0);
   await step({tool:'workcrate-ui.state',capture:'reopened-with-original-offline'});
 }finally{fs.renameSync(source+'.offline',source);}
 await invoke('workcrate-ui.navigate',{project_id:saved.project_id,board_id:saved.ids[0]});
 await until(s=>s.ui?.board_id===saved.ids[0]&&s.ui.display_images===1&&!s.ui.busy);
 await delay(500);
 const s=await invoke('workcrate-ui.state');
 assert(s.ui.display_rgba_bytes<3840*2160*4,'Editor should upload viewport tier, not all original pixels');
 await step({tool:'workcrate-ui.state',capture:'reopened-viewport-resolution'});
 fs.writeFileSync(path.join(profile,'reopen-result.json'),JSON.stringify({cache_hits:3,display_rgba_bytes:s.ui.display_rgba_bytes,original_rgba_bytes:3840*2160*4},null,2));
};
