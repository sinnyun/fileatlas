// Same-process native lifecycle and directory refresh regression.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile,output})=>{
  const report={cases:[],samples:[],activation:[]};
  const invoke=async(t,a={})=>payload(await call(t,a)),state=()=>invoke('get_ui_state');
  async function until(check){for(let i=0;i<180;i++){const s=await state();if(check(s))return s;await delay(100);}throw Error('refresh state timed out');}
  async function capture(name){const r=await call('screenshot_window');for(const b of r.content??[])if(b.type==='image')fs.writeFileSync(path.join(output,name+'.png'),Buffer.from(b.data,'base64'));}
  const fixture=path.join(profile,'files');fs.mkdirSync(fixture,{recursive:true});for(let i=0;i<450;i++)fs.writeFileSync(path.join(fixture,String(i).padStart(4,'0')+'.txt'),'fixture');
  await delay(1200);await invoke('control_gallery',{action:'navigate',value:fixture});
  let first=null,seen=false;
  for(let i=0;i<48;i++){
    const s=await state();if(s.gallery?.current_directory?.endsWith('files')){
      report.samples.push({status:s.browser.status,count:s.gallery.matched_count,revision:s.gallery.snapshot_revision});
      if(s.gallery.matched_count){seen=true;if(!first)first=s.browser.status[0];}
      if(seen){assert(s.gallery.matched_count>0,'content disappeared after first display');assert.equal(s.browser.status[0],first,'unchanged directory was enumerated again');}
    }await delay(100);
  }
  assert(seen);let settled=await until(s=>!s.browser.status[2]&&s.gallery?.matched_count===450);report.cases.push('one enumeration on entry; content does not disappear after first display');
  await invoke('control_gallery',{action:'layout',value:'list'});await invoke('control_gallery',{action:'scroll',value:'350'});settled=await until(s=>s.gallery.window_offset>200);
  const media=(await invoke('search_media',{search:'0440.txt'})).items[0];await invoke('control_gallery',{action:'select_many',media_ids:[media.id]});await delay(700);
  for(let i=0;i<3;i++){
    const before=await state();await invoke('control_ui',{action:'window_minimize'});await until(s=>s.window_status.minimized);await delay(500);
    await invoke('control_ui',{action:'window_activate'});await until(s=>!s.window_status.minimized&&s.window_status.active);await delay(1500);
    const after=await state();assert.equal(after.browser.status[0],before.browser.status[0]);assert.equal(after.gallery.snapshot_revision,before.gallery.snapshot_revision);assert.equal(after.gallery.window_offset,before.gallery.window_offset);assert.deepEqual(after.gallery.selected_media_ids,before.gallery.selected_media_ids);report.activation.push({before,after});
  }
  report.cases.push('three actual minimize/activate cycles preserve enumeration, query revision, window and selection');await capture('after-activation');
  await invoke('control_ui',{action:'window_minimize'});await until(s=>s.window_status.minimized);
  const offset=(await state()).gallery.window_offset;
  fs.writeFileSync(path.join(fixture,'external.txt'),'real external change');await invoke('control_ui',{action:'window_activate'});await until(s=>s.window_status.active&&s.gallery.matched_count===451);
  assert.equal((await state()).gallery.window_offset,offset,'real refresh should retain the rich media window');
  assert((await state()).gallery.selected_media_ids.includes(media.id));report.cases.push('real file change while minimized remains visible and preserves selection');
  fs.unlinkSync(path.join(fixture,'external.txt'));await until(s=>s.gallery.matched_count===450);await delay(1600);
  report.final=await state();report.cases.push('native deletion refreshes normally');
  fs.writeFileSync(path.join(profile,'refresh-report.json'),JSON.stringify(report,null,2));
};
