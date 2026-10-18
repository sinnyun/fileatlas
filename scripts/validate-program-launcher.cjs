// Same-process launcher interaction evidence. Run through photag-mcp-debug.cjs --scenario.
const fs=require('node:fs');
const path=require('node:path');
const assert=require('node:assert/strict');
const {execFileSync}=require('node:child_process');
module.exports=async({step,delay,profile})=>{
  const invoke=async(tool,args={},extra={})=>(await step({tool,arguments:args,...extra})).result;
  const fixture=path.join(profile,'launcher-fixture.exe');
  const marker=path.join(profile,'double-click-launched.txt');
  const source=path.join(profile,'launcher-fixture.rs');
  fs.writeFileSync(source,'#![windows_subsystem = "windows"]\nfn main(){let p=std::env::current_exe().unwrap();std::fs::write(p.parent().unwrap().join("double-click-launched.txt"),"launched from PhoTag").unwrap();}\n');
  execFileSync('rustc',['+1.97.1','--crate-name','launcher_validation_fixture',source,'-o',fixture],{windowsHide:true});
  if(fs.existsSync(marker))fs.unlinkSync(marker);
  await invoke('add_launcher_program',{path:fixture});
  await invoke('control_ui',{action:'panel_open',value:'program-launcher/panel'},{wait_ms:200});
  let state;
  for(let i=0;i<80;i++){
    state=await invoke('get_launcher_state');
    if(state.ui && !state.scanning && !state.ui.icons_pending)break;
    await delay(400);
  }
  assert(state.ui && !state.ui.icons_pending,'icon read must finish');
  assert(state.ui.icons_loaded>0,'native shell icons must render');
  const geometry=await invoke('get_ui_state');
  let [x,y,w,h]=geometry.panel_instances.find(p=>p.id==='program-launcher/panel@1').bounds.split(',').map(Number);
  const pointer=async(action,px,py,extra={})=>invoke('control_ui',{action,value:`${px},${py}`},{wait_ms:140,...extra});
  const click=async(px,py)=>{await pointer('pointer_move',px,py);await pointer('pointer_down',px,py);await pointer('pointer_up',px,py);};
  const key=async(value)=>invoke('control_ui',{action:'key_press',value},{wait_ms:160});
  const input=async(value)=>invoke('control_ui',{action:'input_text',value},{wait_ms:200});
  const search=async(value)=>{await click(x+420,y+72);await key('ctrl-a');if(value)await input(value);else await key('backspace');};
  await invoke('screenshot_window',{}, {capture:'icon-grid'});
  await search('launcher-fixture');
  state=await invoke('get_launcher_state');
  await invoke('screenshot_window',{}, {capture:'search-input'});
  assert.equal(state.ui.visible_count,1,'search must filter actual launcher view');
  const fixtureId=state.ui.visible_ids[0];
  for(let i=0;i<120;i++){
    state=await invoke('get_launcher_state');
    if(!state.ui.icons_pending&&state.ui.icon_cache.manual_programs[String(fixtureId)]!=='pending')break;
    await delay(250);
  }
  const manualIconSource=state.ui.icon_cache.manual_programs[String(fixtureId)];
  assert(['shell-extracted','disk-cache'].includes(manualIconSource),`manually added executable must resolve its native icon (got ${manualIconSource})`);
  await click(x+292,y+235);
  state=await invoke('get_launcher_state');
  assert.equal(state.ui.selected_entry,fixtureId,'left click must select');
  assert(!fs.existsSync(marker),'single click must not launch');
  await click(x+1080,y+230);
  await key('ctrl-a'); await input('00 样例启动器');
  state=await invoke('get_launcher_state');
  assert.equal(state.ui.draft_name,'00 样例启动器','inspector text must update');
  await click(x+1080,y+h-95);
  state=await invoke('get_launcher_state');
  assert.equal(state.entries.find(e=>e.id===fixtureId).display_name,'00 样例启动器','inspector save must persist name');
  await search('样例启动器');
  await click(x+1080,y+290); await invoke('screenshot_window',{}, {capture:'inspector-category-options'}); await click(x+1000,y+346);
  await click(x+1080,y+h-95);
  state=await invoke('get_launcher_state');
  assert.equal(state.entries.find(e=>e.id===fixtureId).category_id,state.categories[1].id,'inspector must save category');
  await click(x+1080,y+331);
  await click(x+1080,y+h-95);
  state=await invoke('get_launcher_state');
  assert.equal(state.entries.find(e=>e.id===fixtureId).hidden,true,'inspector must save hidden state');
  for(let i=0;i<20&&state.ui.visible_count!==0;i++){await delay(80);state=await invoke('get_launcher_state');}
  assert.equal(state.ui.visible_count,0,'hidden program must leave ordinary grid');
  await click(x+1080,y+331);await click(x+1080,y+h-95);
  state=await invoke('get_launcher_state');
  assert.equal(state.entries.find(e=>e.id===fixtureId).hidden,false);
  await pointer('pointer_double_down',x+292,y+235);
  await pointer('pointer_double_up',x+292,y+235);
  for(let i=0;i<20&&!fs.existsSync(marker);i++)await delay(100);
  assert(fs.existsSync(marker),'double click must launch the harmless executable');
  fs.unlinkSync(marker);
  await invoke('launch_launcher_program',{id:fixtureId,confirm:true});
  for(let i=0;i<20&&!fs.existsSync(marker);i++)await delay(100);
  assert(fs.existsSync(marker),'confirmed MCP launch must run the harmless fixture');
  await pointer('pointer_right_down',x+292,y+235);
  await pointer('pointer_right_up',x+292,y+235,{capture:'right-click-menu'});
  await key('escape');
  await search('');
  const original=state.entries.find(e=>e.id===fixtureId);
  const made=await invoke('create_launcher_category',{name:'MCP验证分类'});
  const testCategory=made.category;
  assert(testCategory&&testCategory.id,'MCP must create and return category identity');
  await invoke('rename_launcher_category',{id:testCategory.id,name:'MCP验证分类-已改名'});
  await invoke('update_launcher_program',{id:fixtureId,name:'MCP验证程序',category_id:testCategory.id,hidden:true});
  state=await invoke('get_launcher_state');
  let changed=state.entries.find(e=>e.id===fixtureId);
  assert.equal(changed.display_name,'MCP验证程序');assert.equal(changed.category_id,testCategory.id);assert.equal(changed.hidden,true);
  const refusedDelete=await invoke('delete_launcher_category',{id:testCategory.id,confirm:false},{expect_error:'confirm=true'});
  assert(JSON.stringify(refusedDelete).includes('confirm=true'),'category deletion must require confirmation');
  const refusedRemove=await invoke('remove_launcher_program',{id:fixtureId,confirm:false},{expect_error:'confirm=true'});
  assert(JSON.stringify(refusedRemove).includes('confirm=true'),'list removal must require confirmation');
  const refusedLaunch=await invoke('launch_launcher_program',{id:fixtureId,confirm:false},{expect_error:'confirm=true'});
  assert(JSON.stringify(refusedLaunch).includes('confirm=true'),'launch must require confirmation');
  const refusedUninstall=await invoke('uninstall_launcher_program',{id:fixtureId,confirm:false},{expect_error:'confirm=true'});
  assert(JSON.stringify(refusedUninstall).includes('confirm=true'),'uninstall must require confirmation');
  await invoke('update_launcher_program',{id:fixtureId,name:original.display_name,category_id:testCategory.id,hidden:false});
  await invoke('delete_launcher_category',{id:testCategory.id,confirm:true});
  state=await invoke('get_launcher_state');
  const retainedEntry=state.entries.find(e=>e.id===fixtureId);
  const general=state.categories.find(category=>category.name==='General');
  assert(retainedEntry&&general&&retainedEntry.category_id===general.id,'deleting a category must retain its programs and move them to General');
  await invoke('remove_launcher_program',{id:fixtureId,confirm:true});
  state=await invoke('get_launcher_state');
  assert(!state.entries.some(e=>e.id===fixtureId),'confirmed removal must remove only the launcher registration');
  await invoke('add_launcher_program',{path:fixture});
  state=await invoke('get_launcher_state');
  const normalizedPath=value=>value.replace(/^\\\\\?\\/,'').replaceAll('/','\\').toLowerCase();
  const restored=state.entries.find(e=>normalizedPath(e.launch_path)===normalizedPath(fixture));
  assert(restored,'program can be re-added after launcher-only removal');
  const refreshed=await invoke('refresh_launcher');
  assert(refreshed.started,'MCP should start discovery refresh through the service');
  for(let i=0;i<50;i++){state=await invoke('get_launcher_state');if(!state.scanning)break;await delay(100);}
  assert(!state.scanning,'MCP-triggered discovery should settle');
  const currentFixture=state.entries.find(e=>normalizedPath(e.launch_path)===normalizedPath(fixture));
  assert(currentFixture,'rescan should preserve manually registered program');
  // The fixture keeps a stable id across manual re-add; update following UI checks.
  const activeFixtureId=currentFixture.id;
  await invoke('uninstall_launcher_program',{id:activeFixtureId,confirm:true},{expect_error:'no registered uninstaller'});
  const verifySort=(s,field,descending)=>{
    const entries=new Map(s.entries.map(e=>[e.id,e]));
    let previous=null,unknown=false,previousPin=2;
    for(const id of s.ui.visible_ids){
      const item=entries.get(id);const pinRank={all:2,category:1,none:0}[item.pin_scope];
      assert(pinRank<=previousPin,'pin scope must precede ordinary sorting');
      if(pinRank!==previousPin){previous=null;unknown=false;previousPin=pinRank;}
      const value=field==='display_name'?item[field].toLowerCase():item[field];
      if(value==null){unknown=true;continue;}
      assert(!unknown,'unknown metadata must sort last');
      if(previous!==null)assert(descending?previous>=value:previous<=value,`${field} order`);
      previous=value;
    }
  };
  state=await invoke('get_launcher_state');
  const photoshop=state.entries.find(e=>e.display_name==='Adobe Photoshop 2025');
  const creativeCloud=state.entries.find(e=>e.display_name==='Adobe Creative Cloud');
  assert(photoshop&&creativeCloud,'launcher scan should contain the two validation programs');
  await invoke('set_launcher_pin',{id:activeFixtureId,scope:'all'});
  await invoke('set_launcher_pin',{id:creativeCloud.id,scope:'all'});
  await invoke('set_launcher_pin',{id:photoshop.id,scope:'category'});
  state=await invoke('get_launcher_state');
  for(let i=0;i<30;i++){
    const pinned=new Map(state.entries.map(entry=>[entry.id,entry.pin_scope]));
    if(pinned.get(activeFixtureId)==='all'&&pinned.get(creativeCloud.id)==='all'&&pinned.get(photoshop.id)==='category'&&state.ui.visible_ids[2]===photoshop.id)break;
    await delay(80);state=await invoke('get_launcher_state');
  }
  assert.deepEqual(state.ui.visible_ids.slice(0,2).sort(),[activeFixtureId,creativeCloud.id].sort(),'multiple global pins must stay above all other entries');
  assert.equal(state.ui.visible_ids[2],photoshop.id,'category pin must follow global pins and precede unpinned entries');
  verifySort(state,'display_name',false);
  // The popup opens under the sorting button; each native menu row is 28 logical px.
  await click(x+878,y+133);
  await invoke('screenshot_window',{}, {capture:'sort-options'});
  await click(x+888,y+196);
  state=await invoke('get_launcher_state');
  assert.equal(state.ui.sort,'installed');verifySort(state,'installed_on',true);
  await click(x+920,y+133);
  state=await invoke('get_launcher_state');verifySort(state,'installed_on',false);
  await click(x+878,y+133);await click(x+888,y+224);
  state=await invoke('get_launcher_state');
  assert.equal(state.ui.sort,'size');verifySort(state,'size_bytes',true);
  await click(x+920,y+133);
  state=await invoke('get_launcher_state');verifySort(state,'size_bytes',false);
  await click(x+878,y+133);await click(x+888,y+168);
  state=await invoke('get_launcher_state');assert.equal(state.ui.sort,'name');verifySort(state,'display_name',false);
  await click(x+920,y+133);state=await invoke('get_launcher_state');verifySort(state,'display_name',true);
  await click(x+920,y+133);
  await search('Adobe Photoshop 2025'); await click(x+292,y+235);
  await pointer('pointer_right_down',x+292,y+235);
  await pointer('pointer_right_up',x+292,y+235,{capture:'application-context-menu'});
  await key('escape'); await search('');
  await click(x+920,y+133); await click(x+920,y+133);
  await invoke('screenshot_window',{}, {capture:'icon-grid-inspector'});
  const revision=state.revision;
  await click(x+910,y+72);
  for(let i=0;i<50;i++){
    state=await invoke('get_launcher_state');
    if(!state.scanning&&state.revision>revision&&!state.ui.icons_pending)break;
    await delay(200);
  }
  assert(state.revision>revision&&!state.scanning,'rescan must finish and notify the view');
  await delay(800); const quietBefore=await invoke('get_launcher_state');
  await delay(1000); const quietAfter=await invoke('get_launcher_state');
  assert.equal(quietBefore.revision,quietAfter.revision);
  assert.equal(quietBefore.ui.render_count,quietAfter.ui.render_count,'settled launcher must stay quiet');
  await pointer('pointer_move',x+w-2,y+h-2);
  await pointer('pointer_down',x+w-2,y+h-2);
  await pointer('pointer_drag',x+w-252,y+h-102);
  await pointer('pointer_up',x+w-252,y+h-102);
  await delay(800); await invoke('screenshot_window',{}, {capture:'narrow-layout'});
  await invoke('control_ui',{action:'panel_close',value:'program-launcher/panel@1'},{wait_ms:200});
  await invoke('control_ui',{action:'panel_open',value:'program-launcher/panel'},{wait_ms:700});
  const reopened=await invoke('get_launcher_state');
  assert(!reopened.ui.icons_pending&&reopened.ui.icons_loaded===state.ui.icons_loaded,'reopening must reuse icon cache');
  state=await invoke('get_launcher_state');
  for(let i=0;i<30;i++){
    const pinned=new Map(state.entries.map(entry=>[entry.id,entry.pin_scope]));
    if(pinned.get(activeFixtureId)==='all'&&pinned.get(creativeCloud.id)==='all'&&pinned.get(photoshop.id)==='category'&&state.ui.visible_ids[2]===photoshop.id)break;
    await delay(80);state=await invoke('get_launcher_state');
  }
  const pinnedById=new Map(state.entries.map(entry=>[entry.id,entry.pin_scope]));
  assert.equal(pinnedById.get(activeFixtureId),'all');
  assert.equal(pinnedById.get(creativeCloud.id),'all');
  assert.equal(pinnedById.get(photoshop.id),'category');
  assert.deepEqual(state.ui.visible_ids.slice(0,2).sort(),[activeFixtureId,creativeCloud.id].sort(),'multiple global pins must stay above all other entries');
  assert.equal(state.ui.visible_ids[2],photoshop.id,'category pin must follow global pins and precede unpinned entries');
  await invoke('screenshot_window',{}, {capture:'pinned-programs'});
  const activeLauncherPanel=(await invoke('get_ui_state')).panel_instances.find(panel=>panel.id.startsWith('program-launcher/panel@'));
  await invoke('control_ui',{action:'panel_close',value:activeLauncherPanel.id},{wait_ms:200});
  await invoke('control_ui',{action:'panel_open',value:'program-launcher/panel'},{wait_ms:300});
  state=await invoke('get_launcher_state');
  assert.equal(state.entries.find(e=>e.id===activeFixtureId).pin_scope,'all','pin choices must persist after panel reopen');
  // Open the actual native file picker through the GPUI button. MCP state must
  // remain callable while it is open, proving the host thread is not blocked.
  await click(x+820,y+72);
  for(let i=0;i<30;i++){
    state=await invoke('get_launcher_state');
    if(state.ui.add_program_pending)break;
    await delay(100);
  }
  assert(state.ui.add_program_pending,'native picker task should stay pending until the dialog is dismissed');
  assert(state.ui.render_count>0,'PhoTag must remain responsive while the native picker is open');
  const summary={reopened_icon_cache:true,manual_icon_source:manualIconSource,programs:state.entries.length,icons_loaded:state.ui.icons_loaded,icons_failed:state.ui.icons_failed,
    install_dates:state.entries.filter(e=>e.installed_on).length,sizes:state.entries.filter(e=>e.size_bytes!=null).length,
    single_click:'selected without launch',double_click:'fixture executable wrote marker',
    inspector:'Chinese name, category and hidden state saved through real controls',sorting:'all 3 fields, both directions; unknown last',
    pinning:'two global and one category pin; priority survives all six sorts and panel reopen',
    quiet:{before:quietBefore.ui.render_count,after:quietAfter.ui.render_count},
    mcp:'category CRUD with program retention, partial program edits, scan, confirmed fixture launch, list-remove/uninstall confirmation gates',
    native_picker:'opened from GPUI; same-process MCP remained responsive',fixtureId:activeFixtureId};
  fs.writeFileSync(path.join(profile,'launcher-validation.json'),JSON.stringify(summary,null,2));
  console.log(JSON.stringify(summary));
};
