const fs=require('node:fs'), path=require('node:path'), assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile,root,output,exe})=>{
 const invoke=async(tool,args={})=>payload(await call(tool,args));
 const report={exe,sha256:require('node:crypto').createHash('sha256').update(fs.readFileSync(exe)).digest('hex'),started:new Date().toISOString()};
 try {
  await delay(1500);await invoke('control_ui',{action:'accessibility_activate'});
  await invoke('control_debug',{action:'open'});
  const fixtures=path.join(profile,'fixtures');fs.cpSync(path.join(root,'test-data/photag-library'),fixtures,{recursive:true});
  fs.copyFileSync(path.join(root,'target/preview-latency-fixtures/cold-24mp.jpg'),path.join(fixtures,'large.jpg'));
  await invoke('add_folder',{path:fixtures,confirm:true});
  let media;for(let i=0;i<100;i++){media=await invoke('search_media',{limit:100});if(media.items.some(m=>m.filename==='large.jpg'))break;await delay(200);}
  assert(media.items.some(m=>m.filename==='large.jpg'));
  for(let i=0;i<240;i++){
   const state=await invoke('get_ui_state');
   media=await invoke('search_media',{limit:100});
   if(!state.scan_progress&&media.items.find(m=>m.filename==='large.jpg')?.thumb_path)break;
   await delay(250);
  }
  assert(media.items.find(m=>m.filename==='large.jpg')?.thumb_path,'indexing must supply thumbnail before reuse test');
  const open=async name=>{const m=media.items.find(m=>m.filename===name);assert(m);await invoke('control_gallery',{action:'preview',media_id:m.id});await delay(4000);};
  await open('large.jpg');report.first=await invoke('get_debug_info',{category:'preview',page_size:200});
  const complete=report.first.logs.records.filter(r=>r.message.includes('preview decode finished'));
  const fresh=complete.find(r=>r.message.includes('large.jpg')&&r.message.includes('max_dim=96'));
  assert(fresh,'indexed thumbnail reuse must be observable');
  assert(fresh.message.includes('reused=true'));
  assert.match(fresh.message,/(?:^| )decode_ms=0(?:\.0+)?(?: |$)/);
  for(const key of ['job_id=','source=','cache_path=','file_bytes=','source_dimensions=','output_dimensions=','role=','queue_ms=','cache_check_ms=','read_ms=','header_ms=','decode_ms=','orientation_ms=','resize_ms=','write_ms=','duration_ms=','total_ms=','generation=','status=','cancellation_reason='])assert(fresh.message.includes(key),key);
  report.progressive=complete.filter(r=>r.message.includes('large.jpg')&&r.message.includes('max_dim=1600'));
  assert(report.progressive.length<=1,'at most one progressive attempt per open');
  assert(report.first.logs.records.some(r=>r.message.includes('preview image full paint')&&r.message.includes('source=')&&r.message.includes('duration_scope=\"open_to_paint\"')));
  await invoke('control_preview',{action:'close'});await open('large.jpg');
  report.warm=await invoke('get_debug_info',{category:'preview',page_size:200});
  assert(report.warm.logs.records.some(r=>r.message.includes('large.jpg')&&r.message.includes('cached=true')&&r.message.includes('preview decode finished')));
  await invoke('control_preview',{action:'close'});await open('broken-image.jpg');
  report.failure=await invoke('get_debug_info',{category:'preview',page_size:200});
  assert(report.failure.logs.records.some(r=>r.level==='ERROR'&&r.message.includes('preview decode finished')&&r.message.includes('broken-image.jpg')&&r.message.includes('error_stage=\"header\"')));
  const failedBefore=await invoke('get_debug_info',{category:'preview',page_size:200});
  const failedViewBefore=await invoke('get_view_state');
  await delay(3500);const failedAfter=await invoke('get_debug_info',{category:'preview',page_size:200});
  assert.equal(failedAfter.logs.total,failedBefore.logs.total,'broken image must not retry on every poll');
  const failedViewAfter=await invoke('get_view_state');
  assert.equal(failedViewAfter.preview.image_load.render_count,failedViewBefore.preview.image_load.render_count,'broken image must settle');
  report.failed_idle={before:failedBefore.logs.total,after:failedAfter.logs.total,view_before:failedViewBefore,view_after:failedViewAfter};
  await invoke('control_preview',{action:'close'});await open('large.jpg');
  report.actions=[];
  for(const [action,extra] of [['filmstrip_show',{}],['next',{}],['previous',{}],['zoom',{zoom_percent:150}],['rotate_clockwise',{}],['rotate_counter_clockwise',{}],['fit',{}]]){
   await invoke('control_preview',{action,...extra});await delay(1800);
   const state=await invoke('get_view_state');report.actions.push({action,state});
   assert(state.preview.source,'preview must remain open');
  }
  const actionState=action=>report.actions.find(x=>x.action===action).state.preview;
  assert.notEqual(actionState('next').source,actionState('filmstrip_show').source);
  assert.equal(actionState('previous').source,actionState('filmstrip_show').source);
  assert.equal(actionState('zoom').zoom_percent,150);
  assert.equal(actionState('rotate_clockwise').quarter_turns,1);
  assert.equal(actionState('rotate_counter_clockwise').quarter_turns,0);
  assert.equal(actionState('fit').fit_to_window,true);
  for(let i=0;i<4;i++){
   await invoke('control_gallery',{action:'preview',media_id:media.items.find(m=>m.filename==='large.jpg').id});await delay(180);
   await invoke('control_preview',{action:'next'});await delay(180);
   await invoke('control_preview',{action:'close'});await delay(180);
  }
  await delay(4000);report.rapid=await invoke('get_view_state');assert.equal(report.rapid.preview.source,null);
  await invoke('control_preview',{action:'close'});
  await invoke('control_debug',{action:'category',value:'preview'});
  await invoke('control_debug',{action:'select',value:String(fresh.id)});await delay(500);
  const shot=await call('screenshot_window');fs.writeFileSync(path.join(output,'decode-details.png'),Buffer.from(shot.content.find(c=>c.type==='image').data,'base64'));
  report.before=await invoke('get_debug_info',{category:'preview',page_size:200});await delay(3000);report.after=await invoke('get_debug_info',{category:'preview',page_size:200});
  assert.equal(report.after.logs.total,report.before.logs.total,'completed jobs must stop logging');
  report.raw_before=await invoke('get_debug_info',{page_size:200,hide_ui_refresh:false});
  await delay(3000);report.raw_after=await invoke('get_debug_info',{page_size:200,hide_ui_refresh:false});
  assert.equal(report.raw_after.logs.latest_id,report.raw_before.logs.latest_id,'idle must not generate raw refresh logs');
  report.result='passed';
 }finally{report.finished=new Date().toISOString();fs.writeFileSync(path.join(profile,'decode-details-report.json'),JSON.stringify(report,null,2));}
};
