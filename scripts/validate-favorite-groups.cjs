// Native same-process GPUI clicks through FileAtlas MCP, isolated dev-test profile.
const fs=require('node:fs'), path=require('node:path'), assert=require('node:assert/strict');
module.exports=async({step,delay,profile})=>{
  const invoke=async(tool,args={},extra={})=>(await step({tool,arguments:args,...extra})).result;
  const state=()=>invoke('get_ui_state');
  const ui=(action,value)=>invoke('control_ui',{action,...(value===undefined?{}:{value})},{wait_ms:180});
  const sidebar=async(kind,args={})=>{await invoke('control_ui',{action:'sidebar_request',request:{kind,...args}});await delay(240);};
  const until=async(check)=>{for(let n=0;n<120;n++){const s=await state();if(check(s))return s;await delay(150);}throw Error('favorite group state timeout');};
  const shot=name=>invoke('get_ui_state',{}, {capture:name});
  const group=(s,id)=>s.sidebar_folders.favorite_groups.find(g=>g.id===id);
  const normalize=p=>p.replaceAll('\\','/').toLowerCase().replace(/\/$/,'');
  const click=async(x,y)=>{
    for(const action of ['pointer_move','pointer_down','pointer_up']){
      for(let retry=0;;retry++)try{await ui(action,`${x},${y}`);break;}catch(e){if(retry>5||!String(e).includes('pointer request is still pending'))throw e;await delay(200);}
    }
  };
  const header=async(id)=>{
    const s=await until(s=>group(s,id)?.header_bounds?.[2]>0);
    return group(s,id).header_bounds;
  };
  const plus=async(id)=>{const[x,y,w]=await header(id);await click(x+w-11,y+17);};
  const navigate=async(folder)=>{
    await invoke('control_file_manager',{action:'navigate',value:folder});
    await until(s=>normalize(s.sidebar_folders.favorite_current_path??'')===normalize(folder)&&!s.browser.worker_active);
  };
  const create=async(name)=>{
    await sidebar('open_editor',{editor:{target:'create_group'},text:name});await sidebar('confirm_editor');
    const s=await until(s=>s.sidebar_folders.favorite_groups.some(g=>g.name===name));
    return s.sidebar_folders.favorite_groups.find(g=>g.name===name).id;
  };
  const fixture=path.join(profile,'favorites-fixture'), one=path.join(fixture,'one'), two=path.join(fixture,'two'), legacy=path.join(fixture,'legacy');
  for(const p of [one,two,legacy,path.join(one,'child')])fs.mkdirSync(p,{recursive:true});
  fs.writeFileSync(path.join(one,'notes.txt'),'isolated favorite navigation fixture');
  await delay(1300);await ui('sidebar_width','380');await ui('navigation_tab','folders');
  await sidebar('set_network_drives',{show:false});
  await invoke('add_folder',{path:legacy,confirm:true});
  let s=await until(s=>s.sidebar_folders.favorite_groups.some(g=>g.folders.some(f=>normalize(f.path)===normalize(legacy))));
  const defaultId=s.sidebar_folders.favorite_groups.find(g=>g.folders.some(f=>normalize(f.path)===normalize(legacy))).id;
  const ai=await create('AI'), work=await create('工作');
  await navigate(one);const libraryBefore=(await invoke('list_library')).folders;
  await plus(ai);s=await until(s=>group(s,ai).folders.length===1);
  assert.equal(normalize(group(s,ai).folders[0].path),normalize(one));
  assert.equal(s.folder_picker.pending,false);assert.equal(s.sidebar_folders.editor,null);
  assert.deepEqual((await invoke('list_library')).folders,libraryBefore,'plus adds a shortcut, not another index root');
  const revision=s.sidebar_folders.config_revision;await plus(ai);s=await state();
  assert.equal(group(s,ai).folders.length,1);assert.equal(s.sidebar_folders.config_revision,revision,'duplicate plus is a no-op');
  await sidebar('toggle_group',{group_id:ai});await navigate(two);await plus(ai);
  s=await until(s=>group(s,ai).folders.length===2);assert(group(s,ai).expanded,'adding to a collapsed group reveals it');
  await navigate(path.join(one,'child'));await plus(work);
  s=await until(s=>group(s,work).folders.length===1);
  assert.equal(normalize(group(s,work).folders[0].path),normalize(path.join(one,'child')),'plus uses current folder instead of parent/index root');
  const first=group(s,ai).folders[0];
  await sidebar('open_editor',{editor:{target:'rename_favorite',group_id:ai,folder_id:first.id},text:'模型目录'});await sidebar('confirm_editor');
  await navigate(one);await plus(ai);s=await state();assert.equal(group(s,ai).folders[0].display_name,'模型目录');
  await sidebar('toggle_expanded',{path:one});await until(s=>group(s,ai).folders[0].child_count===1);
  await shot('independent-groups');
  const[x,y,w]=await header(work);await click(x+w-33,y+17);await shot('group-menu');await ui('key_press','escape');
  const[hx,hy]=await header(ai);await click(hx+9,hy+17);await until(s=>!group(s,ai).expanded);await shot('collapsed-group');
  await click(hx+9,hy+17);await until(s=>group(s,ai).expanded);
  const long=path.join(two,'这是一段很长的收藏目录名称用于验证省略显示和分组边界','nested-folder-with-a-long-name');
  fs.mkdirSync(long,{recursive:true});await navigate(long);await plus(work);await until(s=>group(s,work).folders.length===2);await shot('long-path');
  // Home has no committed directory. The visible plus must remain disabled.
  await invoke('control_navigation',{action:'all'});await until(s=>s.sidebar_folders.favorite_current_path===null);
  const beforeHome=JSON.stringify((await state()).sidebar_folders.favorite_groups.map(g=>g.folders));
  await plus(ai);assert.equal(JSON.stringify((await state()).sidebar_folders.favorite_groups.map(g=>g.folders)),beforeHome);
  await shot('home-plus-disabled');
  // Check stale directory validation through the same visible plus handler.
  await navigate(long);fs.rmdirSync(long);await plus(ai);s=await until(s=>!!s.sidebar_folders.favorite_action_error);
  assert.equal(group(s,ai).folders.length,2);await shot('missing-current-folder');
  await navigate(two);await plus(ai);await until(s=>!s.sidebar_folders.favorite_action_error);
  // Removing migrated shortcuts never removes the original library registration.
  s=await state();const old=group(s,defaultId).folders[0];
  await sidebar('remove_favorite_folder',{group_id:defaultId,folder_id:old.id});
  await until(s=>group(s,defaultId).folders.length===0);
  assert.deepEqual((await invoke('list_library')).folders,libraryBefore);
  await shot('empty-group');
  await delay(700);const quiet=(await state()).sidebar_folders;await delay(900);
  assert.deepEqual((await state()).sidebar_folders,quiet,'quiet groups must not save or republish changed state');
  await invoke('resize_window',{width:1510,height:1000},{wait_ms:500});
  await ui('pointer_scroll','220,450,-650');await shot('compact-scrolled');
  await sidebar('toggle_group',{group_id:ai});await until(s=>!group(s,ai).expanded);
  await delay(600);s=await state();
  const result={passed:true,defaultId,ai,work,libraryBefore,quiet_revision:quiet.config_revision,
    groups:s.sidebar_folders.favorite_groups.map(g=>({id:g.id,name:g.name,expanded:g.expanded,folders:g.folders.map(f=>({id:f.id,path:f.path,display_name:f.display_name}))})),
    checks:['legacy import','independent group frames','real plus targets current directory','no picker/editor/index mutation','duplicate preserves revision and alias','collapsed plus reveals group','nested current path','actual header menu/collapse','long path','stale directory rejection','remove preserves library','quiet state','small-window scroll']};
  fs.writeFileSync(path.join(profile,'favorite-groups-summary.json'),JSON.stringify(result,null,2));
};
