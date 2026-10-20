// Keep reports outside the watched directory so the test does not create its own events.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile})=>{
  const invoke=async(t,a={})=>payload(await call(t,a)),state=()=>invoke('get_ui_state');
  async function until(check){for(let i=0;i<150;i++){const s=await state();if(check(s))return s;await delay(100);}throw Error('self-watch state timed out');}
  await delay(1000);await invoke('control_file_manager',{action:'navigate',value:profile});await until(s=>s.file_manager?.browsing&&!s.browser.status[2]);
  await delay(3500);const before=await state(),mtime=fs.statSync(path.join(profile,'session.json')).mtimeMs;
  await delay(3500);const after=await state();assert.equal(fs.statSync(path.join(profile,'session.json')).mtimeMs,mtime);assert.deepEqual(after.browser.status,before.browser.status);
  const fixture=path.join(profile,'external-change.txt');fs.writeFileSync(fixture,'external change');await until(s=>s.file_manager?.matched_count>before.file_manager.matched_count);
  await delay(3000);const settled=fs.statSync(path.join(profile,'session.json')).mtimeMs;await delay(2500);assert.equal(fs.statSync(path.join(profile,'session.json')).mtimeMs,settled);
  fs.writeFileSync(profile+'-selfwatch-report.json',JSON.stringify({cases:['browsing profile does not rewrite session while idle','external file change refreshes and settles'],before,after},null,2));
};
