const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile})=>{
  const expected=JSON.parse(fs.readFileSync(path.join(profile,'session.json'),'utf8'));
  const tagExpected=path.join(profile,'address-tag-expected.json');
  await delay(3000);const s=payload(await call('get_ui_state')),a=s.sidebar_folders.address_bar;
  assert(!a.can_back&&!a.can_forward,'history is local to a window, not persisted row IDs');
  if(fs.existsSync(tagExpected)){const e=JSON.parse(fs.readFileSync(tagExpected,'utf8'));assert.deepEqual(s.file_manager.included_tags,[e.tagId]);assert.equal(a.path,null);}
  else if(expected.selected_path){assert.equal(a.path,expected.selected_path.replace(/^\\\\\?\\/,''));assert(s.file_manager.current_directory.endsWith(path.basename(expected.selected_path)));}
  else if(expected.selected_tag_ids.length){assert.deepEqual(s.file_manager.included_tags,expected.selected_tag_ids);assert.equal(a.path,null);}
  let blocked=false;try{await call('control_ui',{action:'address_bar',request:{kind:'edit'}});}catch{blocked=true;}assert(blocked);
  fs.writeFileSync(path.join(profile,'address-restart-report.json'),JSON.stringify({cases:['saved directory/tag scope remains stable after restart','history starts at restored location','read-only address mutation guard'],state:s},null,2));
};
