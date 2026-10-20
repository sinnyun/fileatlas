// Passive version-state invalidation: unrelated DB writes must not reopen Lore.
const fs=require('node:fs'), path=require('node:path'), assert=require('node:assert/strict');
module.exports=async({step,delay,profile})=>{
  const go=async(tool,args={},extra={})=>(await step({tool,arguments:args,...extra})).result;
  const state=()=>go('get_ui_state');
  const logs=()=>go('get_debug_info',{search:'MeterProvider.ExistingMeterReturned',page_size:5});
  const until=async check=>{for(let i=0;i<120;i++){const s=await state();if(check(s))return s;await delay(200);}throw Error('version idle state timeout');};
  const dir=path.join(profile,'fixture'),file=path.join(dir,'idle.txt');
  fs.mkdirSync(dir,{recursive:true});fs.writeFileSync(file,'first saved bytes');
  await delay(1300);await go('add_folder',{path:dir,confirm:true});
  await go('control_file_manager',{action:'navigate',value:dir});
  await until(s=>s.file_manager?.matched_count===1&&!s.browser.worker_active);
  const id=(await go('search_media',{})).items.find(m=>m.filename==='idle.txt').id;
  const versionState=s=>s.file_manager?.version_states.find(v=>v.media_id===id);
  const status=expected=>until(s=>!s.file_manager.version_states_loading&&versionState(s)?.state?.status===expected);
  await go('create_file_version',{media_id:id,title:'01',confirm:true});await status('saved');
  await go('control_debug',{action:'open'});await delay(1400);
  const before=await logs();const quiet=[];
  for(let i=0;i<3;i++){
    await go('add_clipboard_text',{content:`Unrelated clipboard write ${i}`});
    // A normal repaint following a background write exposed the old broad key.
    await go('control_ui',{action:i%2?'hover_clear':'hover_media',value:String(id)});
    await delay(1300);const after=await logs();
    assert.equal(after.logs.total,before.logs.total,'unrelated database writes must not repeat Lore reads');
    quiet.push(after.logs.total);
  }
  await go('control_ui',{action:'hover_clear'});await delay(900);
  const rev=(await state()).file_manager.snapshot_revision;
  await delay(2200);assert.equal((await logs()).logs.total,before.logs.total);
  assert.equal((await state()).file_manager.snapshot_revision,rev);
  await go('get_ui_state',{}, {capture:'quiet-no-history'});
  fs.writeFileSync(file,'real source content changed');
  const modified=await status('modified');assert.equal(versionState(modified).badge_visible,true);
  const changed=await logs();assert(changed.logs.total>before.logs.total,'real change must still read Lore');
  await go('get_ui_state',{}, {capture:'real-file-modified'});
  await go('create_file_version',{media_id:id,title:'02',confirm:true});await status('saved');
  await delay(1200);const final=await logs();await delay(2000);
  assert.equal((await logs()).logs.total,final.logs.total,'settled version action must return to idle');
  const versions=(await go('list_file_versions',{media_id:id})).versions;
  assert.equal(versions.length,2);
  fs.writeFileSync(path.join(profile,'version-idle-summary.json'),JSON.stringify({passed:true,
    no_history_page:true,unrelated_writes:3,quiet_meter_counts:quiet,quiet_revision:rev,
    initial_meter_count:before.logs.total,real_file_change_meter_count:changed.logs.total,
    settled_after_create_meter_count:final.logs.total,version_count:versions.length,
    checks:['unrelated clipboard writes + repaint preserve Lore cache','quiet debug drawer',
      'real file watcher invalidation and unsaved badge','explicit create clears unsaved badge','settled action stops reads']},null,2));
};
