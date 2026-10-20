const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile,output})=>{
  const prior=JSON.parse(fs.readFileSync(path.join(profile,'address-report.json'),'utf8'));
  const invoke=async(t,a={})=>payload(await call(t,a)),state=()=>invoke('get_ui_state'),address=s=>s.sidebar_folders?.address_bar;
  const clean=p=>p?.replace(/^\\\\\?\\/,'').toLowerCase();
  async function until(test){for(let i=0;i<150;i++){const s=await state();if(test(s))return s;await delay(100);}throw Error('address pointer timed out');}
  async function at(p){await until(s=>clean(s.file_manager?.current_directory)===clean(p)&&clean(address(s)?.path)===clean(p)&&!s.browser.status[2]);await delay(400);}
  async function click(x,y){await invoke('control_ui',{action:'pointer_down',value:`${x},${y}`});await delay(350);await invoke('control_ui',{action:'pointer_up',value:`${x},${y}`});await delay(400);}
  await delay(1500);await invoke('control_file_manager',{action:'navigate',value:prior.d});await at(prior.d);await invoke('control_file_manager',{action:'navigate',value:prior.b});await at(prior.b);
  // Logical client coordinates observed in address-breadcrumb.png at 150% DPI.
  await click(294,28);await at(prior.d);await click(325,28);await at(prior.b);await click(356,28);await at(prior.a);
  await click(294,28);await at(prior.b);await click(842,28);await at(prior.a);
  await click(1100,28);await until(s=>address(s)?.editing);await invoke('control_ui',{action:'key_press',value:'escape'});await until(s=>!address(s)?.editing);
  const before=(await state()).file_manager.snapshot_revision;await click(842,28);assert.equal((await state()).file_manager.snapshot_revision,before);
  const r=await call('screenshot_window');for(const b of r.content??[])if(b.type==='image')fs.writeFileSync(path.join(output,'address-pointer.png'),Buffer.from(b.data,'base64'));
  fs.writeFileSync(path.join(profile,'address-pointer-report.json'),JSON.stringify({cases:['real pointer Back, Forward, Up','real pointer ancestor breadcrumb','blank address area enters editing; Escape cancels','current breadcrumb click does not reload'],final:await state()},null,2));
};
