const fs = require('node:fs');
const path = require('node:path');
module.exports = async ({step,call,payload,delay,root,profile}) => {
  const run = async (tool, args={}, wait=250, capture) => (await step({tool,arguments:args,wait_ms:wait,...(capture?{capture}:{})})).result;
  await step({tool:'get_ui_state',until:{field:'window_status.valid',equals:true,attempts:60,interval_ms:500}});
  await step({tool:'get_ui_state',until:{field:'gallery.matched_count',at_least:0,attempts:60,interval_ms:300}});
  await run('resize_window',{width:1920,height:1560},300);
  await run('control_ui',{action:'inspector_open'},150);
  await run('control_ui',{action:'inspector_width',value:'360'},300);
  await run('control_gallery',{action:'grouping',value:'none'},300);
  const fixture=path.join(profile,'fixtures','2026-秋日旅行');
  const library=await run('list_library');
  if (!library.folders.some(f=>f.path.includes('2026-秋日旅行'))) await run('add_folder',{path:fixture,confirm:true},300);
  const tags={};
  let tree=(await run('list_library')).tags;
  const flat=t=>t.flatMap(v=>[v,...flat(v.children)]);
  for (const [name,parent,color] of [['旅行',null,'#E36B6B'],['生活',null,'#54AEB5'],['工作',null,'#8B929E'],['日本','旅行','#3185ED'],['欧洲','旅行','#9B68D7'],['城市','旅行','#E3BD68'],['自然','旅行','#68B887'],['美食','旅行','#9B68D7']]) {
    let tag=flat(tree).find(t=>t.name===name);
    if (!tag) tag=await run('create_tag',{name,parent_id:parent?tags[parent]:null},100);
    tags[name]=tag.id;
    await run('control_navigation',{action:'tag_appearance',tag_id:tag.id,color},100);
    tree=(await run('list_library')).tags;
  }
  await run('control_navigation',{action:'select_path',value:fixture},1000);
  await step({tool:'get_ui_state',until:{field:'gallery.matched_count',equals:18,attempts:80,interval_ms:300},wait_ms:700,capture:'folder-complete'});
  let media=await run('search_media',{limit:100,sort:'name_asc'},300);
  let photos=media.items.filter(m=>m.full_path.includes('2026-秋日旅行')&&m.filename.startsWith('DSC'));
  if(photos.length!==18) throw Error('Expected 18 real images, observed '+photos.length);
  await run('assign_tags',{media_ids:photos.map(m=>m.id),tag_ids:[tags['旅行']]},500);
  let offset=0;
  for(const [name,count] of [['日本',6],['欧洲',4],['城市',3],['自然',3],['美食',2]]) {
    await run('assign_tags',{media_ids:photos.slice(offset,offset+count).map(m=>m.id),tag_ids:[tags[name]]},300);
    offset+=count;
  }
  const photo=photos.find(m=>m.filename==='DSC08812.jpg');
  await run('assign_tags',{media_ids:[photo.id],tag_ids:[tags['欧洲'],tags['自然'],tags['美食']]},400);
  await run('set_rating',{media_ids:[photo.id],rating:4},500);
  await run('control_ui',{action:'select_media',value:String(photo.id)},800);
  await run('control_ui',{action:'inspector_tab',value:'information'},300);
  const ui=payload(await call('get_ui_state'));
  if(!ui.gallery.inspector_sections.includes('exif')) await run('control_gallery',{action:'inspector_section',value:'exif'},700);
  await run('get_ui_state',{},1000,'file-complete');
  const selected=await run('get_media',{media_ids:[photo.id]});
  if(selected.media.missing===true) throw Error('Fixture is unexpectedly missing');
  await run('control_ui',{action:'inspector_tab',value:'history'},500,'history-complete');
  fs.writeFileSync(path.join(profile,'fixture-state.json'),JSON.stringify({fixture,tags,photo:await run('get_media',{media_ids:[photo.id]}),library:await run('list_library')},null,2));
  console.log(JSON.stringify({seeded:true,fixture,tags,photo_id:photo.id,ready_for_pointer:true}));
};
