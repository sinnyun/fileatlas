const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile,output})=>{
  const prior=JSON.parse(fs.readFileSync(path.join(profile,'extended-report.json'),'utf8')),report={cases:[]};
  const invoke=async(tool,args={})=>payload(await call(tool,args));const state=()=>invoke('get_ui_state');
  async function until(check){for(let i=0;i<100;i++){const s=await state();if(check(s))return s;await delay(80);}throw Error('snapshot state timed out');}
  async function gallery(action,args={}){await invoke('control_gallery',{action,...args});await delay(200);}
  const original=prior.photoPath,moved=original+'.offline';await delay(1000);fs.renameSync(original,moved);
  try {
    await gallery('missing_records');await until(s=>s.gallery?.records_count===1);
    const missing=await invoke('manage_tag_records',{action:'list_missing'});assert(missing.records[0].thumb_path.includes('tag-previews'));
    const result=await call('screenshot_window');for(const b of result.content??[])if(b.type==='image')fs.writeFileSync(path.join(output,'browser-saved-preview.png'),Buffer.from(b.data,'base64'));
    report.cases.push('missing image record shows saved identification preview');
  } finally {fs.renameSync(moved,original);await gallery('close_dialogs');}
  const big=process.env.PHOTAG_BROWSER_LARGE_DIR;if(!big)throw Error('PHOTAG_BROWSER_LARGE_DIR must name the isolated 100k fixture');
  await invoke('control_gallery',{action:'navigate',value:big});await until(s=>s.browser.status[2]&&s.browser.worker_active);
  const started=Date.now();await invoke('control_gallery',{action:'cancel_directory'});await until(s=>!s.browser.status[2]&&!s.browser.worker_active);report.cancelObservedMs=Date.now()-started;report.cases.push('cancel acknowledgement includes actual directory producer stop');
  await gallery('navigate',{value:path.dirname(original)});await until(s=>s.gallery?.current_directory?.endsWith('alpha')&&!s.browser.status[2]);report.final=await state();fs.writeFileSync(path.join(profile,'snapshot-report.json'),JSON.stringify(report,null,2));
};
