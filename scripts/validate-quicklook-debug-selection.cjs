const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile,output,root})=>{
  const report={steps:[]};
  const invoke=async(tool,args={})=>payload(await call(tool,args));
  const save=()=>fs.writeFileSync(path.join(profile,'debug-selection-report.json'),JSON.stringify(report,null,2));
  const fixture=path.join(profile,`debug-selection-fixture-${Date.now()}`);
  fs.mkdirSync(fixture,{recursive:true});
  fs.copyFileSync(path.join(root,'test-data/photag-library/01-small/small.png'),path.join(fixture,'日志选择测试.png'));
  await invoke('add_folder',{path:fixture,confirm:true});
  await invoke('control_gallery',{action:'navigate',value:fixture});
  let gallery;for(let n=0;n<100;n++){gallery=await invoke('get_ui_state');if(gallery.gallery?.matched_count>=1&&!gallery.browser.status[2])break;await delay(100);}
  assert(gallery.gallery?.matched_count>=1,'fixture folder did not finish indexing');
  await invoke('control_gallery',{action:'sort',value:'name_asc'});await delay(250);
  const search=await invoke('search_media',{limit:20,sort:'name_asc'});
  const normal=p=>p.replace(/^\\\\\?\\/,'').toLowerCase();
  const media=search.items.find(item=>normal(item.full_path).startsWith(normal(fixture)));assert(media);
  await invoke('control_ui',{action:'select_media',value:String(media.id)});await delay(200);
  const targetName=path.basename(media.full_path).toLowerCase();
  const initiallyVisible=await invoke('inspect_quicklook');
  const targetWasVisible=(initiallyVisible.windows??[]).some(window=>window.title.toLowerCase().includes(targetName));
  const targetVisible=async()=>((await invoke('inspect_quicklook')).windows??[]).some(window=>window.title.toLowerCase().includes(targetName));
  const waitForTarget=async visible=>{for(let n=0;n<30;n++){if(await targetVisible()===visible)return true;await delay(100);}return false;};
  await invoke('control_gallery',{action:'preview',media_id:media.id});
  let state;for(let n=0;n<60;n++){state=await invoke('get_ui_state');if(state.quicklook.error||state.quicklook.outcome)break;await delay(100);}
  assert(state.quicklook.error||state.quicklook.outcome,'QuickLook request produced neither an outcome nor an error');
  if(state.quicklook.error?.includes('running as administrator')){
    assert(state.quicklook_notice);assert(state.quicklook_notice.includes('管理员身份'));
  }
  const external=await invoke('inspect_quicklook');
  report.quicklook={state:state.quicklook,notice:state.quicklook_notice,external};save();
  if(state.quicklook.outcome){
    if(targetWasVisible){await waitForTarget(false);}
    else if(await waitForTarget(true)){
      await invoke('control_gallery',{action:'preview',media_id:media.id});
      await waitForTarget(false);
    }
  }
  if(state.quicklook.error?.includes('running as administrator')){
    assert.deepEqual(external.windows,[]);
    const appShot=await call('screenshot_window');for(const b of appShot.content??[])if(b.type==='image')fs.writeFileSync(path.join(output,'quicklook-privilege-alert.png'),Buffer.from(b.data,'base64'));
  }
  if(state.quicklook.error){
    await invoke('control_ui',{action:'key_press',value:'escape'});await delay(250);
  }
  await invoke('control_debug',{action:'open'});await delay(400);
  const drawerShot=await call('screenshot_window');for(const b of drawerShot.content??[])if(b.type==='image')fs.writeFileSync(path.join(output,'debug-drawer-before-selection.png'),Buffer.from(b.data,'base64'));
  await invoke('get_debug_info',{page_size:20});
  await invoke('control_ui',{action:'pointer_down',value:'420,855'});await delay(100);
  await invoke('control_ui',{action:'pointer_drag',value:'1000,930'});await delay(100);
  await invoke('control_ui',{action:'pointer_up',value:'1000,930'});await delay(250);
  const selected=await invoke('get_selected_debug_logs');
  assert(selected.selected_ids.length>=2,`drag selection selected ${selected.selected_ids.length} rows`);
  assert(selected.text.includes('duration='));assert(selected.text.includes('target='));
  report.selected={ids:selected.selected_ids,preview:selected.text.slice(0,160)};save();
  await invoke('control_debug',{action:'copy'});await delay(700);
  const after=await invoke('get_debug_info',{page_size:20});
  assert(after.ui.feedback?.includes('已复制'));
  report.copy_feedback=after.ui.feedback;save();
  const shot=await call('screenshot_window');for(const b of shot.content??[])if(b.type==='image')fs.writeFileSync(path.join(output,'debug-selection.png'),Buffer.from(b.data,'base64'));
  report.finished=new Date().toISOString();save();
  console.log(JSON.stringify({quicklook:report.quicklook.notice,selected:report.selected.ids,copy:report.copy_feedback}));
};
