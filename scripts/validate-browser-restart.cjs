const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile})=>{
  const previous=JSON.parse(fs.readFileSync(path.join(profile,'extended-report.json'),'utf8'));
  const report={cases:[]};const read=async(tool,args={})=>payload(await call(tool,args));
  let state;
  for(let i=0;i<100;i++){state=await read('get_ui_state');if(!state.browser.status[2]&&state.file_manager?.selected_media_ids.length)break;await delay(100);}
  assert(!state.browser.show_descendants);assert(!state.library_refresh?.running);
  const input=path.join(profile,'restart-input.json');if(fs.existsSync(input)){const stale=JSON.parse(fs.readFileSync(input,'utf8')).forced_stale_ids;assert(state.file_manager.selected_media_ids.every(id=>!stale.includes(id)));report.cases.push('stale saved cache IDs ignored');}
  const library=await read('list_library');assert.equal(library.folders.length,2);report.cases.push('bookmarks restored without startup library scan');
  const selected=(await read('get_media',{media_ids:[state.file_manager.selected_media_ids[0]]})).media;
  assert.equal(selected.full_path,previous.selectedPath);report.cacheIdChanged=selected.id!==previous.selectedOldId;report.cases.push('untagged selection restored by path');
  const tagged=(await read('search_media',{tag_ids:library.tags.filter(t=>t.name==='Persistent').map(t=>t.id)})).items;assert.equal(tagged.length,1);report.cases.push('durable tag association restored');
  let blocked=false;try{await call('create_tag',{name:'ReadOnlyRejected'});}catch{blocked=true;}assert(blocked);report.cases.push('read only mutation guard');
  const original=previous.photoPath,moved=original+'.offline';
  fs.renameSync(original,moved);
  try {
    const missing=await read('manage_tag_records',{action:'list_missing'});assert.equal(missing.records.length,1);assert(missing.records[0].thumb_path.includes('tag-previews'));assert(fs.existsSync(missing.records[0].thumb_path));report.cases.push('missing record retains independent identification preview after restart');
  } finally {fs.renameSync(moved,original);}
  report.state=state;report.selected=selected;fs.writeFileSync(path.join(profile,'restart-report.json'),JSON.stringify(report,null,2));
};
