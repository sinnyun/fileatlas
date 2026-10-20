// Application input and screenshots use the same-process FileAtlas MCP only.
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
const {execFileSync} = require('node:child_process');
module.exports = async ({call, payload, delay, profile, root, output, request}) => {
  const report = {cases:[], calls:[]};
  const save = () => fs.writeFileSync(path.join(profile,'quicklook-report.json'),JSON.stringify(report,null,2));
  async function invoke(tool,args={}) { const result=payload(await call(tool,args)); report.calls.push({tool,args,result}); save(); return result; }
  const state = () => invoke('get_ui_state');
  const inspectViewer = () => call('inspect_quicklook',{}).then(payload);
  async function captureViewer(name) {
    const result=await call('screenshot_quicklook',{});
    for(const block of result.content??[]) if(block.type==='image') fs.writeFileSync(path.join(output,name+'.png'),Buffer.from(block.data,'base64'));
    return result;
  }
  async function until(test) { let s; for(let n=0;n<100;n++){s=await state();if(test(s))return s;await delay(100);} throw Error('Timed out: '+JSON.stringify(s)); }
  async function capture(name) { const r=await call('screenshot_window');for(const b of r.content??[])if(b.type==='image')fs.writeFileSync(path.join(output,name+'.png'),Buffer.from(b.data,'base64')); }
  const fixture=path.join(profile,'原始文件 空间-'+Date.now());fs.mkdirSync(fixture,{recursive:true});
  fs.copyFileSync(path.join(root,'test-data/fileatlas-fixtures/01-small/small.png'),path.join(fixture,'01 图片 & $原图.png'));
  fs.copyFileSync(path.join(root,'test-data/fileatlas-fixtures/04-no-thumbnail/sample.wav'),path.join(fixture,'02 声音.wav'));
  // Generate an actual video fixture, without launching a second application UI.
  execFileSync('ffmpeg',['-y','-f','lavfi','-i','color=c=blue:s=160x90:d=1','-c:v','libx264','-pix_fmt','yuv420p',path.join(fixture,'03 视频.mp4')],{stdio:'ignore',windowsHide:true});
  fs.writeFileSync(path.join(fixture,'04 文本.txt'),'QuickLook integration fixture');
  await invoke('add_folder',{path:fixture,confirm:true});
  await delay(1500);
  await invoke('control_file_manager',{action:'navigate',value:fixture});
  await until(s=>s.file_manager?.matched_count===4&&!s.browser.status[2]);
  await invoke('control_file_manager',{action:'sort',value:'name_asc'});
  await delay(500);
  const tools=(await request('tools/list',{})).tools.map(t=>t.name);
  assert(!tools.includes('control_preview'));assert(!tools.includes('control_playback'));
  report.cases.push('old image/audio/video viewer controls removed from MCP');
  const found=await invoke('search_media',{limit:100,sort:'name_asc'});
  const normal=p=>p.replace(/^\\\\\?\\/,'').toLowerCase();
  const items=found.items.filter(item=>normal(item.full_path).startsWith(normal(fixture)));assert.equal(items.length,4);
  report.items=items.map(({id,filename,media_type,thumb_path})=>({id,filename,media_type,thumb_path}));save();
  assert(items.find(item=>item.media_type==='image')?.thumb_path);
  await capture('file_manager-preserved');
  const privilege=await inspectViewer();
  const dispatched=['pipe_toggle_sent'];
  const latencies=[];
  const view=async()=>payload(await call('get_view_state',{})).quicklook;
  async function latency(target,media_type,t0){
    const stem=path.basename(target);
    let outcome=null;
    for(let n=0;n<300&&!outcome;n++){const q=await view();if(q?.path===target&&dispatched.includes(q.outcome))outcome=q.outcome;else await delay(10);}
    const dispatchMs=Date.now()-t0;let windowMs=-1;
    // Match this file's own viewer title: any visible QuickLook window may still be
    // showing the previous file, which would make the measurement look too fast.
    for(let n=0;n<600&&windowMs<0;n++){const e=await inspectViewer();if((e.windows??[]).some(w=>w.title.includes(stem)))windowMs=Date.now()-t0;else await delay(10);}
    const entry={media_type,outcome,dispatch_ms:dispatchMs,window_ms:windowMs,viewer_stem:stem};
    latencies.push(entry);save();return entry;
  }
  // FileAtlas sends requests only: with QuickLook closed it must prompt, never start it.
  if(!privilege.pipe_probe?.instance_listening) {
    await invoke('control_ui',{action:'select_media',value:String(items[0].id)});
    await delay(200);
    const t0=Date.now();
    await invoke('control_file_manager',{action:'preview',media_id:items[0].id});
    const s=await until(x=>x.quicklook.error?.includes('not responding'));
    const elapsedMs=Date.now()-t0;
    assert(s.quicklook_notice,'the failed request raised no GPUI alert');
    assert(!s.open_panel_ids.some(id=>id.includes('preview')||id.includes('playback')));
    const observed=await inspectViewer();
    assert(!observed.windows?.length,'FileAtlas opened a viewer although QuickLook is closed');
    assert.equal(observed.pipe_probe.instance_listening,false);
    report.not_running={elapsed_ms:elapsedMs,notice:s.quicklook_notice,dispatch:s.quicklook,windows:observed.windows};
    await capture('quicklook-not-running-alert');
    // The same prompt must come from a real Space press, not only the command entry.
    await invoke('control_ui',{action:'key_press',value:'escape'});await delay(250);
    for(const action of ['pointer_down','pointer_up']) { await invoke('control_ui',{action,value:'340,180'});await delay(200); }
    await invoke('control_ui',{action:'select_media',value:String(items[0].id)});await delay(250);
    const spaceT0=Date.now();
    await invoke('control_ui',{action:'key_press',value:'space'});
    const after=await until(x=>x.quicklook.error?.includes('not responding')&&x.quicklook.path===items[0].full_path);
    report.not_running.space_ms=Date.now()-spaceT0;
    assert(after.quicklook_notice,'Space raised no GPUI alert');
    assert.equal((await inspectViewer()).windows?.length??0,0);
    report.cases.push('a closed QuickLook yields a translated GPUI prompt, no window and no started process');
    report.finished=new Date().toISOString();save();
    console.log(JSON.stringify({cases:report.cases,not_running:report.not_running,note:'QuickLook is closed; the pipe dispatch cases need it running'}));
    return;
  }
  for(const item of items) {
    await invoke('control_ui',{action:'select_media',value:String(item.id)});
    await delay(200);
    const t0=Date.now();
    await invoke('control_file_manager',{action:'preview',media_id:item.id});
    if(privilege.higher_integrity_process){
      const s=await until(s=>s.quicklook.path===item.full_path&&s.quicklook.error?.includes('running as administrator'));
      assert(s.quicklook_notice?.includes('管理员身份'));
      assert(!s.open_panel_ids.some(id=>id.includes('preview')||id.includes('playback')));
      report.cases.push(item.media_type+' is blocked with a clear privilege mismatch message');
      if(item.media_type==='image')await capture('quicklook-privilege-mismatch');
      await invoke('control_ui',{action:'key_press',value:'escape'});await delay(150);
    } else {
      const entry=await latency(item.full_path,item.media_type,t0);
      assert(entry.outcome,`QuickLook never dispatched ${item.media_type}: ${JSON.stringify(await view())}`);
      const s=await state();
      assert(!s.open_panel_ids.some(id=>id.includes('preview')||id.includes('playback')));
      assert.equal(s.quicklook.error,null);
      const external=await inspectViewer();
      assert(external.windows?.length,`no visible QuickLook window for ${item.media_type}`);
      report.quicklook_window_checks??=[];
      report.quicklook_window_checks.push({media_type:item.media_type,latency:entry,dispatch:s.quicklook,windows:external.windows});save();
      await delay(2000);
      const afterRender=await inspectViewer();
      report.quicklook_window_checks.at(-1).windows_after_render=afterRender.windows??[];
      const capture=await captureViewer('quicklook-'+item.media_type);
      assert(!capture.isError);
      report.cases.push('FileManager dispatches '+item.media_type+' and a visible QuickLook window is captured');
    }
    save();
    await delay(500);
  }
  // Toggle the last shown file again so the Space press starts from a closed viewer.
  if((await inspectViewer()).windows?.length){
    await invoke('control_file_manager',{action:'preview',media_id:items.at(-1).id});
    for(let n=0;n<60&&(await inspectViewer()).windows?.length;n++)await delay(100);
  }
  // Put focus on the first file_manager card through real GPUI pointer dispatch.
  for(const action of ['pointer_down','pointer_up']) {
    await invoke('control_ui',{action,value:'340,180'});await delay(200);
  }
  await invoke('control_ui',{action:'select_media',value:String(items[0].id)});await delay(250);
  const spaceT0=Date.now();
  await invoke('control_ui',{action:'key_press',value:'space'});
  if(privilege.higher_integrity_process){
    const s=await until(s=>s.quicklook.path===items[0].full_path&&s.quicklook.error?.includes('running as administrator'));
    assert(s.quicklook_notice?.includes('管理员身份'));
    report.cases.push('real GPUI Space reports the privilege mismatch instead of silently claiming success');
    await capture('inspector-quicklook-privilege-alert');
    await invoke('control_ui',{action:'key_press',value:'escape'});await delay(200);
  } else {
    const entry=await latency(items[0].full_path,'space_key',spaceT0);
    assert(entry.outcome,`Space never dispatched QuickLook: ${JSON.stringify(await view())}`);
    report.cases.push('real GPUI Space opens the selected original, including Unicode, spaces, & and $');
    await capture('inspector-preserved');
  }
  // Quiet reads must not launch or repeatedly access the external viewer.
  await delay(1000);
  const before=await invoke('get_debug_info',{search:'fileatlas_plugin_quicklook',page_size:100});
  const settled=await state();await delay(1500);const after=await state();
  const logsAfter=await invoke('get_debug_info',{search:'fileatlas_plugin_quicklook',page_size:100});
  assert.deepEqual(after.quicklook,settled.quicklook);
  assert.deepEqual(logsAfter.logs,before.logs);
  report.cases.push('settled viewer state and QuickLook action logs remain unchanged during passive reads');
  const original=path.join(fixture,items[0].filename), parked=path.join(profile,'parked-original.png');
  fs.renameSync(original,parked);
  try {
    await invoke('control_file_manager',{action:'preview',media_id:items[0].id});
    await until(s=>s.quicklook.error==='The selected file no longer exists.'&&s.quicklook_notice);
    await delay(300);await capture('missing-original-alert');
    report.cases.push('removed original reports a translated GPUI alert instead of opening a viewer');
    await invoke('control_ui',{action:'key_press',value:'escape'});await delay(250);
  } finally { fs.renameSync(parked,original); }
  await invoke('control_ui',{action:'settings_open'});await delay(400);
  await capture('settings-without-viewer-toolbar');
  const settingsState=await state();assert.equal(settingsState.settings_tab,'appearance');
  await assert.rejects(()=>invoke('control_ui',{action:'settings_tab',value:'toolbar'}),/unsupported Settings tab/);
  await invoke('control_ui',{action:'settings_close'});await delay(300);
  report.cases.push('Settings starts on Appearance and rejects the removed viewer Toolbar tab');
  // Close whatever the run left on screen so a finished check owns no viewer window.
  if((await inspectViewer()).windows?.length){
    await invoke('control_ui',{action:'select_media',value:String(items[0].id)});
    await invoke('control_file_manager',{action:'preview',media_id:items[0].id});
    for(let n=0;n<60&&(await inspectViewer()).windows?.length;n++)await delay(100);
    report.cases.push('the run closes its own QuickLook window again through FileManager');
  }
  report.final=await state();report.finished=new Date().toISOString();save();
  console.log(JSON.stringify({cases:report.cases,fixture,latencies,privilege:privilege.higher_integrity_process??null,quietLogs:[before.logs.retained,logsAfter.logs.retained]}));
};
