// Request-to-published-model latency; not a pixel-first-frame clock.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile})=>{
  const a=path.join(profile,'A'),b=path.join(profile,'B');fs.mkdirSync(path.join(a,'Child'),{recursive:true});fs.mkdirSync(b,{recursive:true});
  for(let i=0;i<128;i++)fs.writeFileSync(path.join(a,i+'.txt'),'file');
  for(let i=0;i<12;i++)fs.mkdirSync(path.join(b,String(i)),{recursive:true});
  fs.writeFileSync(path.join(a,'Child','child.txt'),'child');
  const invoke=async(t,args={})=>payload(await call(t,args)),state=()=>invoke('get_ui_state');
  const key=p=>p?.replace(/^\\\\\?\\/,'').replaceAll('/','\\').replace(/\\$/,'').toLowerCase();
  const samples=[];await delay(1200);
  for(let i=0;i<10;i++){
    const dest=i%2?a:b,started=performance.now();await invoke('control_file_manager',{action:'navigate',value:dest});let addressMs=null,result;
    while(performance.now()-started<10000){const s=await state();if(key(s.sidebar_folders?.address_bar?.path)===key(dest)&&addressMs===null)addressMs=performance.now()-started;
      if(key(s.file_manager?.current_directory)===key(dest)&&!s.browser.status[2]&&(i%2?s.file_manager.matched_count===128:s.file_manager.directories.length===12)){result={requestToAddressMs:addressMs,requestToContentMs:performance.now()-started,generation:s.browser.status[0]};break;}await delay(5);}
    assert(result,'directory timing timed out');samples.push({iteration:i,directory:path.basename(dest),...result});await delay(250);
  }
  const sorted=samples.map(s=>s.requestToContentMs).sort((a,b)=>a-b),report={samples,medianMs:sorted[5],maxMs:sorted.at(-1),minMs:sorted[0]};
  fs.writeFileSync(path.join(profile,'navigation-timing.json'),JSON.stringify(report,null,2));
};
