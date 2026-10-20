// Read-only browsing of existing directories; all reports/cache go to the isolated profile.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile})=>{
  const roots=(process.env.FILEATLAS_REAL_DIRS||'D:\\D盘收集资料库;D:\\桌面文件夹').split(';');
  const expected=roots.map(root=>({root,entries:fs.readdirSync(root,{withFileTypes:true}).filter(e=>e.isDirectory()||e.isFile()||e.isSymbolicLink()).length}));
  const invoke=async(t,args={})=>payload(await call(t,args)),state=()=>invoke('get_ui_state');
  const key=p=>p?.replace(/^\\\\\?\\/,'').replaceAll('/','\\').replace(/\\$/,'').toLowerCase();
  const fingerprint=()=>expected.map(({root})=>fs.readdirSync(root).map(name=>{const s=fs.statSync(path.join(root,name),{bigint:true});return [name,s.size.toString(),s.mtimeNs.toString()];}));
  const sourceBefore=fingerprint();
  const samples=[];await delay(1200);
  for(let i=0;i<10;i++){
    const dest=expected[i%expected.length],started=performance.now();await invoke('control_file_manager',{action:'navigate',value:dest.root});let addressMs=null,firstMs=null,result;
    while(performance.now()-started<30000){const s=await state(),elapsed=performance.now()-started;
      if(key(s.sidebar_folders?.address_bar?.path)===key(dest.root)&&addressMs===null)addressMs=elapsed;
      if(key(s.file_manager?.current_directory)===key(dest.root)){
        const total=s.file_manager.matched_count+s.file_manager.directories.length;
        if((total>0||dest.entries===0)&&firstMs===null)firstMs=elapsed;
        if(!s.browser.status[2]&&total===dest.entries){result={requestToAddressMs:addressMs,requestToFirstContentMs:firstMs,requestToCompleteContentMs:elapsed,files:s.file_manager.matched_count,directories:s.file_manager.directories.length};break;}
      }await delay(5);
    }
    assert(result,'real directory timing timed out');samples.push({iteration:i,path:dest.root,...result});await delay(300);
  }
  const sorted=samples.map(s=>s.requestToFirstContentMs).sort((a,b)=>a-b),report={samples,medianFirstMs:sorted[5],minFirstMs:sorted[0],maxFirstMs:sorted.at(-1),sourceWriteActions:0};
  assert.deepEqual(fingerprint(),sourceBefore,'source names/size/mtime changed during the run');report.sourceMetadataUnchanged=true;
  fs.writeFileSync(path.join(profile,'real-navigation-timing.json'),JSON.stringify(report,null,2));
};
