// Run on the baseline executable, then the optimized executable with separate profiles.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,step,delay,profile,root})=>{
 const invoke=async(t,a={})=>payload(await call(t,a));
 const state=()=>invoke('workcrate-ui.state');
 const until=async(test)=>{for(let i=0;i<900;i++){const s=await state();if(test(s))return s;await delay(100);}throw Error('Performance flow timed out');};
 const source=path.join(profile,'fixtures','large.jpg');
 fs.mkdirSync(path.dirname(source),{recursive:true});
 if(!fs.existsSync(source))fs.copyFileSync(path.join(root,'.local/validation/profiles/workcrate-performance-before/fixtures/large.jpg'),source);
 const project=(await invoke('workcrate-ui.manage_project',{action:'create',name:'大图排版性能'})).project;
 const ids=[],imports=[];
 for(let i=0;i<3;i++){
   const page=await invoke('workcrate-ui.create_page',{project_id:project.id});ids.push(page.board_id);
   const start=performance.now();
   await invoke('workcrate-ui.place_image',{project_id:project.id,board_id:page.board_id,path:source});
   const s=await until(s=>s.ui?.board_id===page.board_id&&!s.ui.busy&&(s.ui.display_images===undefined||s.ui.display_images>0));
   imports.push({ms:Math.round(performance.now()-start),display_rgba_bytes:s.ui.display_rgba_bytes??null});await delay(300);
 }
 const timings=[];
 for(let n=0;n<3;n++){
   await invoke('workcrate-ui.navigate',{});await until(s=>s.ui?.screen==='projects'&&!s.ui.busy);
   const start=performance.now();await invoke('workcrate-ui.navigate',{project_id:project.id});
   const s=await until(s=>s.ui?.project_id===project.id&&s.ui.screen==='pages'&&!s.ui.busy&&s.ui.page_count===3);
   timings.push({ms:Math.round(performance.now()-start),cache_hits:s.ui.preview_cache_hits??null});
   if(s.ui.previews_ready!==undefined)await until(s=>s.ui.previews_ready===3);
 }
 await step({tool:'workcrate-ui.state',capture:'pages-warm-cache'});
 const out=path.join(profile,'artifacts');fs.mkdirSync(out,{recursive:true});
 const exports=[];
 for(const [format,selection,name] of [['image',[ids[0]],'page.png'],['pdf',ids,'project.pdf']]){
   const start=performance.now();const result=await invoke('workcrate-ui.export_pages',{project_id:project.id,board_ids:selection,path:path.join(out,name),format,confirm:true});
   exports.push({format,ms:Math.round(performance.now()-start),result});
 }
 let s=await state();
 const optimized=s.ui.preview_cache_hits!==undefined;
 const progress=[];
 if(optimized){
   await invoke('workcrate-ui.export_pages',{project_id:project.id,board_ids:ids,path:out,format:'image',background:true,confirm:true});
   await until(s=>s.ui.busy&&s.ui.export_status);
   for(let i=0;i<900;i++){
     s=await state();if(s.ui.export_status&&!progress.includes(s.ui.export_status)){
       progress.push(s.ui.export_status);
       if(progress.length===2)await step({tool:'workcrate-ui.state',capture:'export-progress'});
     }
     if(!s.ui.busy&&!s.ui.export_status){assert(s.ui.message.includes('完成'),s.ui.message);break;}
     await delay(70);
   }
   assert(progress.some(x=>x.includes('/3 页')),'Live total/page progress missing');
   await step({tool:'workcrate-ui.state',capture:'export-complete'});
   await invoke('workcrate-ui.export_pages',{project_id:project.id,board_ids:[ids[0]],path:path.join(out,'missing-directory','failed.png'),format:'image',background:true,confirm:true});
   await until(s=>!s.ui.busy&&!s.ui.export_status&&s.ui.message?.includes('导出失败'));
   await step({tool:'workcrate-ui.state',capture:'export-failure'});
   // Persisted page previews still work while originals are temporarily unavailable.
   fs.renameSync(source,source+'.offline');
   try{
     await invoke('workcrate-ui.navigate',{});await until(s=>s.ui.screen==='projects');
     await invoke('workcrate-ui.navigate',{project_id:project.id});
     s=await until(s=>s.ui.screen==='pages'&&!s.ui.busy&&s.ui.preview_cache_hits===3);
     assert.equal(s.ui.previews_ready,3);
   }finally{fs.renameSync(source+'.offline',source);}
 }
 await invoke('workcrate-ui.navigate',{project_id:project.id,board_id:ids[0]});
 s=await until(s=>s.ui?.board_id===ids[0]&&!s.ui.busy);
 if(optimized)await until(s=>s.ui.display_images===1);
 await delay(600);await step({tool:'workcrate-ui.state',capture:'large-image-editor'});
 const result={project_id:project.id,ids,timings,imports,exports,progress};
 fs.writeFileSync(path.join(profile,'performance-result.json'),JSON.stringify(result,null,2));
 console.log(JSON.stringify(result));
};
