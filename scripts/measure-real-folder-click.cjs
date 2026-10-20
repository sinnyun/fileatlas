// A real GPUI folder-card double click, through the app's same-process MCP.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile})=>{
  const parent='D:\\桌面文件夹',target=path.join(parent,'套图0306');
  const invoke=async(t,a={})=>payload(await call(t,a)),state=()=>invoke('get_ui_state');
  const clean=p=>p?.replace(/^\\\\\?\\/,'').replaceAll('/','\\').toLowerCase();
  async function until(f){for(let i=0;i<150;i++){const s=await state();if(f(s))return s;await delay(20);}throw Error('real click timed out');}
  const samples=[];await delay(1200);
  for(let n=0;n<5;n++){
    await invoke('control_file_manager',{action:'navigate',value:parent});const s=await until(s=>clean(s.file_manager?.current_directory)===clean(parent)&&!s.browser.status[2]&&s.file_manager.directories.length===9);await delay(500);
    const index=s.file_manager.directories.findIndex(d=>d.name==='套图0306');assert(index>=0);
    const width=s.window_status.geometry.client_size.width,side=s.sidebar_width||264,inspector=s.inspector_width||300,columns=Math.floor((width-side-inspector-16)/166);
    assert(index<columns,'remeasure second-row geometry for this window');const x=side+16+index*166+75,y=190;
    await invoke('control_ui',{action:'pointer_double_down',value:`${x},${y}`});await delay(350);const started=performance.now();await invoke('control_ui',{action:'pointer_double_up',value:`${x},${y}`});
    const result=await until(s=>clean(s.file_manager?.current_directory)===clean(target)&&!s.browser.status[2]&&s.file_manager.matched_count===12&&s.file_manager.directories.length===4);
    samples.push({requestToContentMs:performance.now()-started,x,y,generation:result.browser.status[0]});
  }
  fs.writeFileSync(path.join(profile,'real-click-timing.json'),JSON.stringify({parent,target,samples,sourceWriteActions:0},null,2));
};
