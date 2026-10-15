// Real filesystem fixtures; all app interaction uses the same-process MCP.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),zlib=require('node:zlib');
function png(width,height,color){
  function chunk(type,data){const name=Buffer.from(type),bytes=Buffer.concat([name,data]);let crc=0xffffffff;for(const b of bytes){crc^=b;for(let i=0;i<8;i++)crc=(crc>>>1)^((crc&1)?0xedb88320:0);}const n=Buffer.alloc(4),c=Buffer.alloc(4);n.writeUInt32BE(data.length);c.writeUInt32BE((crc^0xffffffff)>>>0);return Buffer.concat([n,bytes,c]);}
  const header=Buffer.alloc(13);header.writeUInt32BE(width);header.writeUInt32BE(height,4);header[8]=8;header[9]=2;
  const rows=Buffer.alloc(height*(1+width*3));for(let y=0;y<height;y++)for(let x=0;x<width;x++)for(let c=0;c<3;c++)rows[y*(1+width*3)+1+x*3+c]=color[c];
  return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',header),chunk('IDAT',zlib.deflateSync(rows)),chunk('IEND',Buffer.alloc(0))]);
}
module.exports=async({call,payload,delay,profile,output,processId})=>{
  const report={cases:[],processId,observations:[]};const save=()=>fs.writeFileSync(path.join(profile,'extended-report.json'),JSON.stringify(report,null,2));
  async function invoke(tool,args={}){const result=payload(await call(tool,args));assert(!result?.error,JSON.stringify(result));return result;}
  async function state(){return invoke('get_ui_state');}
  async function until(predicate,timeout=60000){const started=Date.now();while(Date.now()-started<timeout){const s=await state();if(predicate(s))return s;await delay(100);}save();throw Error('extended state timed out');}
  async function gallery(action,args={}){await invoke('control_gallery',{action,...args});await delay(300);}
  async function capture(name){const result=await call('screenshot_window');for(const block of result.content??[])if(block.type==='image')fs.writeFileSync(path.join(output,name+'.png'),Buffer.from(block.data,'base64'));}
  const a=path.join(profile,'alpha'),b=path.join(profile,'beta'),big=process.env.PHOTAG_BROWSER_LARGE_DIR||path.join(profile,'100k');
  for(const folder of [a,b,big])fs.mkdirSync(folder,{recursive:true});
  const picture=path.join(a,'version.png');fs.writeFileSync(picture,png(60,30,[70,100,200]));fs.writeFileSync(path.join(b,'beta.txt'),'beta');
  await delay(1200);await gallery('navigate',{value:a});await until(s=>!s.browser.status[2]&&s.gallery?.matched_count===1);
  await until(s=>s.gallery?.loaded_count===1);await delay(500);
  const photo=(await invoke('search_media',{search:'version.png'})).items[0];assert(photo);
  const tag=await invoke('create_tag',{name:'Persistent'});await invoke('assign_tags',{media_ids:[photo.id],tag_ids:[tag.id]});
  const favoriteA=await invoke('add_folder',{path:a,confirm:true}),favoriteB=await invoke('add_folder',{path:b,confirm:true});
  report.favorites={favoriteA,favoriteB};save();
  const folders=(await invoke('list_library')).folders;const first=folders.find(f=>f.path.replace(/^\\\\\?\\/,'').toLowerCase()===a.toLowerCase());const second=folders.find(f=>f.path.replace(/^\\\\\?\\/,'').toLowerCase()===b.toLowerCase());assert(first&&second);
  await invoke('manage_favorite',{action:'rename',folder_id:first.id,name:'Photo alias'});await invoke('manage_favorite',{action:'move',folder_id:second.id,delta:-1});
  const changed=(await invoke('list_library')).folders;assert.equal(changed[0].id,second.id);assert.equal(changed.find(f=>f.id===first.id).name,'Photo alias');assert(fs.existsSync(a));report.cases.push('bookmark alias and order');
  await gallery('navigate',{value:a});await until(s=>!s.browser.status[2]&&s.gallery?.matched_count>=1);
  await gallery('preview',{media_id:photo.id});await delay(1000);const before=await invoke('get_view_state');report.previewBefore=before.preview;
  fs.writeFileSync(picture,png(100,80,[180,50,80]));
  await delay(1800);const refreshed=await invoke('get_view_state');report.previewAfter=refreshed.preview;report.overwriteState=await state();report.overwriteDebug=await invoke('get_debug_info',{category:'',search:'',page_size:200});save();assert(refreshed.preview.revision>before.preview.revision);await invoke('control_preview',{action:'close'});await delay(600);let detail=await invoke('get_media',{media_ids:[photo.id]});
  for(let i=0;i<50&&detail.media.width!==100;i++){await delay(150);detail=await invoke('get_media',{media_ids:[photo.id]});}
  assert.equal(detail.media.width,100);assert.equal(detail.media.height,80);const after=await invoke('get_view_state');assert(after.preview.revision>before.preview.revision);report.previewAfter=after.preview;report.cases.push('same path overwrite refreshes thumbnail dimensions and preview revision');
  await invoke('control_preview',{action:'close'});
  await invoke('control_navigation',{action:'all'});await delay(400);await gallery('tag',{tag_id:tag.id});await until(s=>s.gallery?.matched_count===1&&!s.gallery.browsing);
  await gallery('locate_file',{media_id:photo.id});await until(s=>s.gallery?.browsing&&s.gallery.selected_media_ids.includes(photo.id));report.cases.push('tag result locate and select');
  await capture('browser-main');
  fs.mkdirSync(path.join(a,'created-dir'));await until(s=>s.gallery?.directories.some(d=>d.name==='created-dir'));report.cases.push('native watcher directory create');
  await gallery('grouping',{value:'file_type'});await gallery('show_descendants',{enabled:true});await until(s=>!s.browser.status[2]&&s.gallery.grouping_mode==='subfolder');await gallery('show_descendants',{enabled:false});await until(s=>!s.browser.status[2]&&s.gallery.grouping_mode==='file_type');report.cases.push('descendants restores previous grouping');
  await invoke('remove_folder',{folder_id:first.id,confirm:true});assert.equal((await invoke('get_media',{media_ids:[photo.id]})).tags.length,1);assert(fs.existsSync(picture));await invoke('add_folder',{path:a,confirm:true});report.cases.push('remove favorite preserves tags and local files');
  await gallery('thumbnails',{enabled:false});const disabled=path.join(a,'disabled.png');fs.writeFileSync(disabled,png(25,20,[80,150,40]));await until(s=>s.gallery.matched_count>=3);await delay(500);const disabledRow=(await invoke('search_media',{search:'disabled.png'})).items[0];assert(disabledRow);assert.equal((await invoke('get_media',{media_ids:[disabledRow.id]})).media.thumb_path,null);await gallery('thumbnails',{enabled:true});await delay(1000);assert((await invoke('get_media',{media_ids:[disabledRow.id]})).media.thumb_path);report.cases.push('disabled thumbnails do not decode new visible images');
  await invoke('control_debug',{action:'open'});await delay(2000);const idleA=await invoke('get_debug_info',{category:'',search:'',page_size:200});await delay(2500);const idleB=await invoke('get_debug_info',{category:'',search:'',page_size:200});report.idle={before:idleA,after:idleB};await invoke('control_debug',{action:'close'});save();
  const fixtureStarted=Date.now();if(fs.readdirSync(big).length!==100000)for(let i=0;i<100000;i++)fs.writeFileSync(path.join(big,String(i).padStart(6,'0')+'.dat'),'');report.fixtureMs=Date.now()-fixtureStarted;
  const started=Date.now();await invoke('control_gallery',{action:'navigate',value:big});
  let firstMs=null;while(Date.now()-started<90000){const s=await state();report.observations.push({elapsed:Date.now()-started,count:s.gallery?.matched_count,directory:s.gallery?.current_directory,status:s.browser.status,worker:s.browser.worker_active});if(s.gallery?.current_directory?.endsWith('100k')&&s.gallery.matched_count>0&&firstMs===null)firstMs=Date.now()-started;if(s.gallery?.matched_count===100000&&!s.browser.status[2])break;await delay(150);}
  const loaded=await state();assert.equal(loaded.gallery.matched_count,100000);report.performance={firstPublishedMs:firstMs,completePublishedMs:Date.now()-started};assert(report.observations.some(o=>o.directory?.endsWith('100k')&&o.count>0&&o.status[2]),'directory must publish partial results before enumeration completes');report.cases.push('partial results published before 100000 real direct files finish');report.memory=JSON.parse(require('node:child_process').execFileSync('powershell',['-NoProfile','-Command',`Get-Process -Id ${processId} | Select-Object WorkingSet64,PeakWorkingSet64,CPU | ConvertTo-Json`],{encoding:'utf8'}));save();
  await gallery('layout',{value:'list'});await gallery('scroll',{value:'99950'});await until(s=>s.gallery.window_offset>99000);await capture('browser-100k-list');report.cases.push('list scroll beyond 200 row window');
  await gallery('refresh');await gallery('cancel_directory');await until(s=>!s.browser.status[2]&&!s.browser.worker_active);report.cases.push('cancel stops directory producer');
  await invoke('control_gallery',{action:'navigate',value:big});await delay(30);await invoke('control_gallery',{action:'navigate',value:a});await delay(30);await invoke('control_gallery',{action:'navigate',value:b});await until(s=>s.gallery?.current_directory?.endsWith('beta')&&s.gallery.matched_count===1&&!s.browser.status[2]);await delay(700);assert((await state()).gallery.current_directory.endsWith('beta'));report.cases.push('rapid navigation rejects stale results');
  await gallery('navigate',{value:a});await until(s=>s.gallery?.current_directory?.endsWith('alpha')&&!s.browser.status[2]&&s.gallery.matched_count===fs.readdirSync(a,{withFileTypes:true}).filter(e=>e.isFile()).length);const untagged=(await invoke('search_media',{search:'version.xmp'})).items[0];assert(untagged);await gallery('select_many',{media_ids:[untagged.id]});report.selectedPath=untagged.full_path;report.selectedOldId=untagged.id;await delay(500);report.final=await state();report.photoPath=picture;report.photoId=photo.id;save();
};
