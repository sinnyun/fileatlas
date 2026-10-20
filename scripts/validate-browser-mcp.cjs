// All app interaction uses the same-process FileAtlas MCP. Fixtures stay in target/profile.
const fs=require('node:fs'), path=require('node:path'), assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile,root,output})=>{
  const report={profile,cases:[],calls:[]};
  const save=()=>fs.writeFileSync(path.join(profile,'browser-report.json'),JSON.stringify(report,null,2));
  async function invoke(name,args={}){
    const value=payload(await call(name,args));report.calls.push({name,args,value});save();
    assert(!value?.error,JSON.stringify(value));return value;
  }
  async function state(){return invoke('get_ui_state');}
  async function until(check){for(let i=0;i<120;i++){const s=await state();if(check(s))return s;await delay(100);}report.debug=await invoke('get_debug_info',{category:'',page_size:100,search:'watch'});save();throw Error('Browser state timed out');}
  async function file_manager(action,args={}){await invoke('control_file_manager',{action,...args});await delay(300);}
  const fixture=path.join(profile,'fixture');fs.mkdirSync(path.join(fixture,'nested'),{recursive:true});fs.mkdirSync(path.join(fixture,'empty'),{recursive:true});
  fs.writeFileSync(path.join(fixture,'one.txt'),'one');fs.writeFileSync(path.join(fixture,'two.txt'),'two');fs.writeFileSync(path.join(fixture,'nested','inner.txt'),'inner');
  fs.copyFileSync(path.join(root,'test-data','fileatlas-fixtures','01-small','small.png'),path.join(fixture,'photo.png'));
  await delay(1500);let s=await state();assert(s.browser.filesystem);assert(!s.library_refresh?.running);
  await file_manager('navigate',{value:fixture});
  s=await until(s=>!s.browser.status[2]&&s.file_manager?.matched_count===3);
  assert.equal(s.file_manager.directories.length,2);report.cases.push('unbookmarked direct browse');
  await file_manager('show_descendants',{enabled:true});s=await until(s=>!s.browser.status[2]&&s.file_manager?.matched_count===4);report.cases.push('explicit descendants');
  await file_manager('show_descendants',{enabled:false});await until(s=>!s.browser.status[2]&&s.file_manager?.matched_count===3);
  fs.writeFileSync(path.join(fixture,'new.txt'),'new file');await until(s=>s.file_manager?.matched_count===4);report.cases.push('native watcher create');
  fs.unlinkSync(path.join(fixture,'new.txt'));await until(s=>s.file_manager?.matched_count===3);report.cases.push('native watcher delete');
  const found=await invoke('search_media',{search:'one.txt',limit:10});
  const rows=found.media??found.items??found.results;assert(rows?.length,JSON.stringify(found));const id=rows[0].id;
  const tag=await invoke('create_tag',{name:'BrowserKeep'});await invoke('assign_tags',{media_ids:[id],tag_ids:[tag.id]});report.cases.push('tag persistence');
  const destination=path.join(profile,'relocated.txt');fs.renameSync(path.join(fixture,'one.txt'),destination);await until(s=>s.file_manager?.matched_count===fs.readdirSync(fixture,{withFileTypes:true}).filter(e=>e.isFile()).length);
  const missing=await invoke('manage_tag_records',{action:'list_missing'});assert(missing.records.some(x=>x.id===id));
  await file_manager('missing_records');await delay(500);const shot=await call('screenshot_window');for(const block of shot.content??[])if(block.type==='image')fs.writeFileSync(path.join(output,'missing-records.png'),Buffer.from(block.data,'base64'));report.cases.push('missing records panel');
  await file_manager('record_candidate',{media_id:id,value:destination});
  await file_manager('relink_record',{media_id:id});
  await until(s=>s.file_manager.records_count===0);
  assert.equal((await invoke('manage_tag_records',{action:'list_missing'})).records.length,0);report.cases.push('explicit relink');
  fs.unlinkSync(destination);const nowMissing=await invoke('manage_tag_records',{action:'list_missing'});assert.equal(nowMissing.records.length,1);
  await invoke('manage_tag_records',{action:'delete_missing',record_ids:[id],confirm:true});assert.equal((await invoke('manage_tag_records',{action:'list_missing'})).records.length,0);report.cases.push('delete missing annotation');
  await file_manager('close_dialogs');await file_manager('navigate',{value:fixture});await delay(700);
  report.final=await state();save();return report;
};
