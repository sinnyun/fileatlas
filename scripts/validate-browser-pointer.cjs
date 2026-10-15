const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile,output})=>{
  const invoke=async(tool,args={})=>payload(await call(tool,args));
  const state=()=>invoke('get_ui_state');
  async function until(check){for(let i=0;i<80;i++){const s=await state();if(check(s))return s;await delay(100);}throw Error('pointer state timed out');}
  async function click(x,y){await invoke('control_ui',{action:'pointer_down',value:`${x},${y}`});await delay(120);await invoke('control_ui',{action:'pointer_up',value:`${x},${y}`});await delay(500);}
  await delay(1200);await until(s=>s.gallery&&!s.browser.status[2]);
  const before=await state();assert(!before.browser.show_descendants);
  // 1942x1256 capture, 150% DPI: toolbar coordinates in the GPUI client.
  await click(680,28);const enabled=await until(s=>s.browser.show_descendants&&!s.browser.status[2]);
  const result=await call('screenshot_window');for(const b of result.content??[])if(b.type==='image')fs.writeFileSync(path.join(output,'browser-pointer-descendants.png'),Buffer.from(b.data,'base64'));
  await click(680,28);const disabled=await until(s=>!s.browser.show_descendants&&!s.browser.status[2]);assert.equal(disabled.gallery.grouping_mode,before.gallery.grouping_mode);
  fs.writeFileSync(path.join(profile,'pointer-report.json'),JSON.stringify({cases:['GPUI pointer toggles descendants on and off','previous grouping restored'],before,enabled,disabled},null,2));
};
