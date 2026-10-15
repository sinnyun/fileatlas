const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile,output})=>{
  const previous=JSON.parse(fs.readFileSync(path.join(profile,'sidebar-review-report.json'),'utf8'));
  const invoke=async(t,a={})=>payload(await call(t,a)),state=()=>invoke('get_ui_state');
  async function until(check){for(let i=0;i<120;i++){const s=await state();if(check(s))return s;await delay(100);}throw Error('sidebar pointer state timed out');}
  await delay(1500);let s=await until(s=>s.sidebar_folders&&!s.sidebar_folders.drives_pending);
  if(s.sidebar_folders.quick_access_collapsed)await invoke('control_ui',{action:'sidebar_request',request:{kind:'toggle_section',key:'quick_access'}});
  await until(s=>!s.sidebar_folders.quick_access_collapsed);await delay(500);
  s=await state();if(!s.sidebar_folders.quick_access.find(e=>e.path===previous.fixture).expanded){await invoke('control_ui',{action:'sidebar_request',request:{kind:'toggle_expanded',path:previous.fixture}});await until(s=>s.sidebar_folders.quick_access.find(e=>e.path===previous.fixture).expanded);}
  const geometry=(await state()).window_status.geometry;
  // Chevron observed in sidebar-ready-for-pointer.png: native screenshot (109,470).
  const transform=geometry.screenshot_to_pointer;
  const x=(109-transform.offset_x)/transform.divide_by,y=(470-transform.offset_y)/transform.divide_by;
  const states=[];
  for(let i=0;i<4;i++){
    const before=(await state()).sidebar_folders.quick_access.find(e=>e.path===previous.fixture).expanded;
    await invoke('control_ui',{action:'pointer_down',value:`${x},${y}`});await delay(350);await invoke('control_ui',{action:'pointer_up',value:`${x},${y}`});
    const next=await until(s=>s.sidebar_folders.quick_access.find(e=>e.path===previous.fixture).expanded!==before);states.push(next.sidebar_folders.quick_access.find(e=>e.path===previous.fixture));
  }
  assert(states[3].expanded);await until(s=>s.sidebar_folders.quick_access.find(e=>e.path===previous.fixture).child_count===3);
  const result=await call('screenshot_window');for(const b of result.content??[])if(b.type==='image')fs.writeFileSync(path.join(output,'sidebar-pointer-expanded.png'),Buffer.from(b.data,'base64'));
  await invoke('control_ui',{action:'sidebar_request',request:{kind:'toggle_section',key:'quick_access'}});
  fs.writeFileSync(path.join(profile,'sidebar-pointer-report.json'),JSON.stringify({cases:['real GPUI quick-access chevron collapses and expands four times'],geometry,x,y,states},null,2));
};

