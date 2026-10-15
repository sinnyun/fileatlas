const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,delay,profile,output})=>{
  const report={cases:[]},invoke=async(t,a={})=>payload(await call(t,a)),state=()=>invoke('get_ui_state');
  const clean=p=>p?.replace(/^\\\\\?\\UNC\\/,'\\\\').replace(/^\\\\\?\\/,'').replaceAll('/','\\').replace(/\\$/,'').toLowerCase();
  async function until(test){let last;for(let i=0;i<200;i++){last=await state();if(test(last))return last;await delay(100);}fs.writeFileSync(path.join(profile,'address-timeout.json'),JSON.stringify(last,null,2));throw Error('address state timed out');}
  const address=s=>s.sidebar_folders?.address_bar;
  async function at(p){return until(s=>clean(s.gallery?.current_directory)===clean(p)&&clean(address(s)?.path)===clean(p)&&!s.browser.status[2]);}
  async function action(kind,args={}){await invoke('control_ui',{action:'address_bar',request:{kind,...args}});await delay(200);}
  async function key(value){await invoke('control_ui',{action:'key_press',value});await delay(400);}
  async function navigate(p){await invoke('control_gallery',{action:'navigate',value:p});await at(p);await delay(300);}
  async function capture(name){const r=await call('screenshot_window');for(const b of r.content??[])if(b.type==='image')fs.writeFileSync(path.join(output,name+'.png'),Buffer.from(b.data,'base64'));}
  const root=path.join(profile,'资料 空间'),a=path.join(root,'A'),b=path.join(a,'B'),d=path.join(root,'D'),gone=path.join(root,'gone');for(const p of [b,d,gone])fs.mkdirSync(p,{recursive:true});fs.writeFileSync(path.join(a,'one.txt'),'one');fs.writeFileSync(path.join(b,'two.txt'),'two');
  await delay(1500);await navigate(a);let s=await state();assert.equal(address(s).breadcrumbs[0].label,'F:');assert.equal(address(s).breadcrumbs.at(-1).label,'A');assert(address(s).breadcrumbs.some(c=>c.label==='资料 空间'));report.cases.push('full drive-to-directory breadcrumb including Unicode and spaces');
  await invoke('control_gallery',{action:'open_directory',value:(await state()).gallery.directories.find(p=>p.name==='B').path});await at(b);await action('up');await at(a);await action('back');await at(b);await action('forward');await at(a);report.cases.push('folder-card navigation, up, back and forward share history');
  await action('back');await at(b);await navigate(d);assert(!address(await state()).can_forward);await action('back');await at(b);report.cases.push('ordinary navigation clears the forward branch');
  await action('crumb',{path:a});await at(a);report.cases.push('ancestor breadcrumb switches the live directory');
  await action('edit');await action('set_text',{text:'"'+d+'"'});await capture('address-editing');await key('enter');await at(d);assert(!address(await state()).editing);report.cases.push('actual GPUI Enter submits a quoted address');
  await key('ctrl-l');await until(s=>address(s)?.editing);await action('set_text',{text:'..\\A\\B'});await key('enter');await at(b);report.cases.push('relative address resolves against current directory');
  await action('edit');await action('set_text',{text:path.join(root,'missing')});await key('enter');await until(s=>address(s)?.error);assert.equal(clean(address(await state()).path),clean(b));await capture('address-invalid');await key('escape');await until(s=>!address(s)?.editing&&!address(s)?.error);await at(b);report.cases.push('invalid path preserves location; actual Escape cancels editing');
  const drive=path.parse(b).root;await action('crumb',{path:drive});await at(drive);assert(!address(await state()).can_up);await delay(800);const before=(await state()).gallery.snapshot_revision;await action('up');await delay(500);assert.equal((await state()).gallery.snapshot_revision,before);await action('back');await at(b);report.cases.push('drive root has no fake parent; disabled up does not reload');
  const tag=await invoke('create_tag',{name:'AddressTag'});await invoke('control_gallery',{action:'tag',tag_id:tag.id});await until(s=>!s.gallery?.browsing&&s.gallery?.included_tags.includes(tag.id)&&address(s)?.path===null);await action('back');await at(b);await action('forward');await until(s=>s.gallery?.included_tags.includes(tag.id)&&address(s)?.path===null);report.cases.push('tag results and directory navigation restore through shared history');
  await navigate(gone);await navigate(d);fs.rmdirSync(gone);await action('back');await until(s=>address(s)?.error);await at(d);assert(!address(await state()).can_forward);await action('cancel');report.cases.push('missing history target reports failure without advancing cursor');
  await navigate(b);await delay(1800);const stable=await state();await delay(1500);const after=await state();assert.equal(after.gallery.snapshot_revision,stable.gallery.snapshot_revision);assert.equal(after.browser.status[0],stable.browser.status[0]);report.cases.push('settled address bar does not retrigger directory loads');
  await capture('address-breadcrumb');report.tagId=tag.id;report.root=root;report.a=a;report.b=b;report.d=d;report.final=after;
  fs.writeFileSync(path.join(profile,'address-report.json'),JSON.stringify(report,null,2));
};
