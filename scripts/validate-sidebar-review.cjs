// Regressions for the Explorer sidebar. App actions use its same-process MCP.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile,output})=>{
  const report={cases:[],calls:[]},save=()=>fs.writeFileSync(path.join(profile,'sidebar-review-report.json'),JSON.stringify(report,null,2));
  async function invoke(tool,args={}){const value=payload(await call(tool,args));assert(!value?.error,JSON.stringify(value));report.calls.push({tool,args,value});save();return value;}
  const state=()=>invoke('get_ui_state');
  async function until(test){for(let i=0;i<150;i++){const s=await state();if(test(s))return s;await delay(100);}throw Error('sidebar review state timed out');}
  async function sidebar(kind,args={}){await invoke('control_ui',{action:'sidebar_request',request:{kind,...args}});await delay(200);}
  async function capture(name){const result=await call('screenshot_window');for(const b of result.content??[])if(b.type==='image')fs.writeFileSync(path.join(output,name+'.png'),Buffer.from(b.data,'base64'));}
  const fixture=path.join(profile,'review-folder');fs.mkdirSync(path.join(fixture,'first'),{recursive:true});fs.mkdirSync(path.join(fixture,'second'),{recursive:true});fs.writeFileSync(path.join(fixture,'direct.txt'),'local truth');
  await delay(1200);await until(s=>s.sidebar_folders);
  const libraryBefore=await invoke('list_library');
  await sidebar('open_editor',{editor:{target:'add_quick_access'},text:path.join(profile,'does-not-exist')});await sidebar('confirm_editor');
  await until(s=>s.sidebar_folders.editor?.error);assert.deepEqual((await invoke('list_library')).folders,libraryBefore.folders);
  await sidebar('cancel_editor');await until(s=>!s.sidebar_folders.editor);report.cases.push('invalid inline editor path reports locally and preserves the library');
  await sidebar('set_network_drives',{show:false});await until(s=>!s.sidebar_folders.drives_pending);
  await sidebar('open_editor',{editor:{target:'create_group'},text:'Review group'});await sidebar('confirm_editor');
  let s=await until(s=>s.sidebar_folders.favorite_groups.some(g=>g.name==='Review group'));const group=s.sidebar_folders.favorite_groups.find(g=>g.name==='Review group').id;
  await sidebar('add_quick_access',{path:fixture});await sidebar('add_favorite_folder',{group_id:group,path:fixture});
  await sidebar('open_editor',{editor:{target:'rename_quick_access',path:fixture},text:'Review alias'});await sidebar('confirm_editor');
  s=await until(s=>s.sidebar_folders.quick_access.some(e=>e.display_name==='Review alias'));const revision=s.sidebar_folders.config_revision;
  await sidebar('add_quick_access',{path:fixture});await sidebar('set_network_drives',{show:false});
  s=await state();assert.equal(s.sidebar_folders.config_revision,revision);assert.equal(s.sidebar_folders.quick_access.find(e=>e.path===fixture).display_name,'Review alias');report.cases.push('duplicate adds preserve aliases; no-op edits do not publish revisions');
  await sidebar('open_editor',{editor:{target:'rename_quick_access',path:fixture},text:'Review alias'});await sidebar('confirm_editor');await until(s=>!s.sidebar_folders.editor);report.cases.push('unchanged edit confirms and closes without saving a new revision');
  await sidebar('activate_path',{path:fixture});s=await until(s=>s.gallery?.current_directory?.endsWith('review-folder')&&!s.browser.status[2]);assert(!s.sidebar_folders.quick_access.find(e=>e.path===fixture).expanded);report.cases.push('navigation does not implicitly expand or enumerate the sidebar path');
  await sidebar('toggle_expanded',{path:fixture});await until(s=>s.sidebar_folders.quick_access.find(e=>e.path===fixture).child_count===2);
  fs.mkdirSync(path.join(fixture,'third'));await until(s=>s.sidebar_folders.quick_access.find(e=>e.path===fixture).child_count===3);report.cases.push('native directory changes refresh an expanded active sidebar folder');
  for(let i=0;i<4;i++){await sidebar('toggle_expanded',{path:fixture});const expanded=i%2===1;await until(s=>s.sidebar_folders.quick_access.find(e=>e.path===fixture).expanded===expanded);}
  report.cases.push('repeated collapse and expansion use one path state');
  await sidebar('toggle_section',{key:'quick_access'});await until(s=>s.sidebar_folders.quick_access_collapsed);await capture('quick-collapsed');
  await sidebar('toggle_section',{key:'quick_access'});await until(s=>!s.sidebar_folders.quick_access_collapsed);report.cases.push('quick_access semantic and rendered section key agree');
  await sidebar('toggle_group',{group_id:group});await until(s=>!s.sidebar_folders.favorite_groups.find(g=>g.id===group).expanded);await sidebar('toggle_group',{group_id:group});await until(s=>s.sidebar_folders.favorite_groups.find(g=>g.id===group).expanded);report.cases.push('group state changes are published even when row count stays equal');
  s=await state();const rootsRevision=s.sidebar_folders.roots_revision;await sidebar('refresh_roots');await until(s=>s.sidebar_folders.roots_revision>rootsRevision&&!s.sidebar_folders.drives_pending);report.cases.push('explicit drive refresh has an independent revision');
  const settings=path.join(profile,'settings.json'),backup=path.join(profile,'settings-review-backup.json');
  fs.renameSync(settings,backup);fs.mkdirSync(settings);
  try {
    await sidebar('set_network_drives',{show:true});await until(s=>s.sidebar_folders.save_error);await capture('sidebar-save-failed');report.cases.push('settings write failure is visible');
    fs.rmdirSync(settings);fs.renameSync(backup,settings);await sidebar('retry_save');await until(s=>!s.sidebar_folders.save_error);
    assert.equal(JSON.parse(fs.readFileSync(settings,'utf8')).sidebar_folders.show_network_drives,true);report.cases.push('same revision can retry after a failed save');
  } finally {
    if(fs.existsSync(settings)&&fs.statSync(settings).isDirectory())fs.rmdirSync(settings);
    if(fs.existsSync(backup)){if(fs.existsSync(settings))fs.unlinkSync(backup);else fs.renameSync(backup,settings);}
  }
  await sidebar('set_network_drives',{show:false});await until(s=>!s.sidebar_folders.drives_pending);
  for(const key of ['drives','folders']){s=await state();if(!s.sidebar_folders[key+'_collapsed'])await sidebar('toggle_section',{key});}
  await capture('sidebar-ready-for-pointer');
  await invoke('control_debug',{action:'open'});await delay(1800);const before=await invoke('get_debug_info',{category:'',search:'',page_size:200});await delay(2500);const after=await invoke('get_debug_info',{category:'',search:'',page_size:200});assert.equal(after.logs.latest_id,before.logs.latest_id);report.idle={before:before.logs.latest_id,after:after.logs.latest_id};await invoke('control_debug',{action:'close'});report.cases.push('debug drawer idle state does not repeat I/O logs or repaint records');
  await sidebar('toggle_section',{key:'quick_access'});await sidebar('toggle_group',{group_id:group});
  const drive=fixture.slice(0,3);await sidebar('toggle_drive',{root:drive});await until(s=>s.sidebar_folders.drives.find(d=>d.root.toLowerCase()===drive.toLowerCase())?.expanded);
  report.fixture=fixture;report.group=group;report.drive=drive;report.final=await state();save();
};
