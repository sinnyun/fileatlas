const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile})=>{
  const previous=JSON.parse(fs.readFileSync(path.join(profile,'address-report.json'),'utf8'));
  await delay(1500);await call('control_file_manager',{action:'tag',tag_id:previous.tagId});
  for(let i=0;i<100;i++){const s=payload(await call('get_ui_state'));if(s.file_manager.included_tags.includes(previous.tagId)&&s.sidebar_folders.address_bar.path===null){await delay(800);fs.writeFileSync(path.join(profile,'address-tag-expected.json'),JSON.stringify({tagId:previous.tagId}));return;}await delay(100);}
  assert.fail('tag navigation did not settle');
};
