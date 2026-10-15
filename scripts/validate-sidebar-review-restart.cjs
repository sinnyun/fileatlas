const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile})=>{
  const previous=JSON.parse(fs.readFileSync(path.join(profile,'sidebar-review-report.json'),'utf8'));
  let state;for(let i=0;i<150;i++){state=payload(await call('get_ui_state'));if(state.sidebar_folders&&!state.sidebar_folders.drives_pending&&!state.browser.status[2])break;await delay(100);}
  await delay(2500);state=payload(await call('get_ui_state'));
  const s=state.sidebar_folders;
  assert(s.quick_access_collapsed);assert(!s.favorite_groups.find(g=>g.id===previous.group).expanded);
  assert(s.quick_access.some(e=>e.display_name==='Review alias'));assert(s.drives.find(d=>d.root.toLowerCase()===previous.drive.toLowerCase()).expanded);
  assert(state.gallery.current_directory.endsWith('review-folder'));assert(!state.library_refresh?.running);
  let blocked=false;try{await call('control_ui',{action:'sidebar_request',request:{kind:'refresh_roots'}});}catch{blocked=true;}assert(blocked);
  fs.writeFileSync(path.join(profile,'sidebar-restart-report.json'),JSON.stringify({cases:['section and group collapse restored','drive expansion restored','alias and selected path restored','no startup library scan','read-only mutation guard'],state},null,2));
};
