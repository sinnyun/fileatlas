const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {execFileSync}=require('node:child_process');
module.exports=async({call,payload,step,delay,root,profile})=>{
 const invoke=async(name,args={})=>payload(await call(`workcrate-ui.${name}`,args));
 const official=()=>JSON.parse(execFileSync('powershell.exe',['-NoProfile','-Command',"@(Get-Process -Name vectorcraft,vectorcraft-cli -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id) | ConvertTo-Json -Compress"],{encoding:'utf8',windowsHide:true}).trim()||'[]');
 const baseline=official();assert.deepEqual(baseline,[],'Use an isolated run with no official processes; do not terminate user processes');
 const executable=process.env.FILEATLAS_VECTORCRAFT_EXE;assert(executable&&fs.existsSync(executable));
 const state=(ids=[])=>invoke('state',{control_ids:ids});
 const ready=async(screen,count)=>{for(let i=0;i<160;i++){const s=await state();if(s.ui?.screen===screen&&!s.ui.busy&&s.ui.preview_pending===0&&(count===undefined||s.ui.page_count===count))return s;await delay(200);}await step({tool:'workcrate-ui.state',capture:'offline-failed'});throw Error(`Not ready ${screen}`)};
 const control=async(action,value)=>{for(let i=0;;i++){try{await call('control_ui',{action,...(value===undefined?{}:{value})});await delay(100);return;}catch(e){if(i>=40||!String(e).includes('pending'))throw e;await delay(100)}}};
 const click=async id=>{const raw=(await state([id])).controls[id];assert(raw,`Missing ${id}`);const[x,y,w,h]=raw.split(',').map(Number);await control('pointer_down',`${x+w*.65},${y+h/2}`);await control('pointer_up',`${x+w*.65},${y+h/2}`)};
 const record=path.join(profile,'offline-project.json');let data;
 if(!process.env.WORKCRATE_OFFLINE_REOPEN){
   await invoke('navigate');await ready('projects');
   await control('settings_open');await control('settings_plugin_page','fileatlas.workcrate-ui');await delay(400);
   await click('settings-vectorcraft-executable');await control('key_press','ctrl-a');await control('input_text',executable);
   await click('settings-vectorcraft-save');
   const configFile=path.join(profile,'workcrate','vectorcraft.json');
   for(let i=0;i<80;i++){if(fs.existsSync(configFile)&&JSON.parse(fs.readFileSync(configFile)).executable===executable)break;if(i===79)throw Error('Plugin config did not persist');await delay(200)}
   await step({tool:'workcrate-ui.state',capture:'offline-plugin-settings'});
   await control('settings_close');assert.deepEqual(official(),baseline,'Settings must not launch official processes');
   const directory=path.join(profile,'user-project-folder');fs.mkdirSync(directory,{recursive:true});
   const created=await invoke('manage_project',{action:'create',name:'离线原生工程',directory});assert.equal(created.boards.length,1);
   data={project_id:created.project.id,ids:[created.boards[0].id],directory};
   data.ids.push((await invoke('create_page',{project_id:data.project_id})).board_id);
   const fixture=path.join(root,'test-data/fileatlas-fixtures/03-nested/level-one/level-two/nested-photo.jpeg');
   await invoke('manage_project',{action:'cover',project_id:data.project_id,path:fixture});
   await invoke('place_image',{project_id:data.project_id,board_id:data.ids[0],path:fixture});
   await invoke('update_page',{project_id:data.project_id,board_id:data.ids[0],title:'离线缓存页面',info:{material:'写真裱板',size:'80 × 180 cm',quantity:'2 套'}});
   const artifacts=path.join(profile,'artifacts');fs.mkdirSync(artifacts,{recursive:true});
   await invoke('export_pages',{project_id:data.project_id,board_ids:data.ids,path:path.join(artifacts,'headless.pdf'),format:'pdf',confirm:true});
   await invoke('export_pages',{project_id:data.project_id,board_ids:[data.ids[0]],path:path.join(artifacts,'headless.png'),format:'image',confirm:true});
   assert(fs.readdirSync(directory).some(n=>n.endsWith('.vectorcraft')));
   assert.deepEqual(official(),baseline,'Headless operations must clean up their processes and never launch GUI');
   fs.writeFileSync(record,JSON.stringify(data,null,2));
 }else{data=JSON.parse(fs.readFileSync(record));}
 // An unusable executable proves browsing cannot silently call the official program.
 await invoke('configure_vectorcraft',{executable:path.join(profile,'not-installed','vectorcraft.exe'),port:17979});
 await invoke('navigate');await ready('projects');await step({tool:'workcrate-ui.state',capture:'offline-projects-no-program'});
 await invoke('navigate',{project_id:data.project_id});const pages=await ready('pages',2);assert.equal(pages.ui.previews_ready,2);await step({tool:'workcrate-ui.state',capture:'offline-pages-no-program'});
 await invoke('open_board',{project_id:data.project_id,board_id:data.ids[0]});const editor=await ready('editor',2);assert.equal(editor.ui.boards.find(b=>b.id===data.ids[0]).info.quantity,'2 套');await step({tool:'workcrate-ui.state',capture:'offline-preview-no-program'});
 const before=(await state()).ui;await delay(1500);assert.deepEqual((await state()).ui,before);assert.deepEqual(official(),baseline);
 // Restore the saved installation for the user's next actual editing action.
 await invoke('configure_vectorcraft',{executable,port:17979});
 fs.writeFileSync(path.join(profile,process.env.WORKCRATE_OFFLINE_REOPEN?'offline-reopen-result.json':'offline-result.json'),JSON.stringify({checks:['plugin settings persists without starting programs','chosen folder stores native file','headless creates and exports','internal covers/catalog/previews','browse with missing executable','quiet cached UI','no official processes'],...data},null,2));
 console.log('PASS offline WorkCrate'+(process.env.WORKCRATE_OFFLINE_REOPEN?' reopen':''));
};
