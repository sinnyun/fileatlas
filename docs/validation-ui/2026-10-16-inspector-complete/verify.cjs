// Replay against the isolated fixture prepared in target; every UI operation
// is a public same-process FileAtlas MCP call, never an OS input injection.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
module.exports = async ({step,call,payload,profile,delay}) => {
  const run = async (tool, args={}, wait=300, capture) => (await step({tool,arguments:args,wait_ms:wait,...(capture?{capture}:{})})).result;
  const ui = async capture => run('get_ui_state',{},200,capture);
  const click = async (x,y) => {
    await run('control_ui',{action:'pointer_down',value:`${x},${y}`},180);
    await run('control_ui',{action:'pointer_up',value:`${x},${y}`},450);
  };
  const rootTag = async () => (await run('list_library')).tags.find(t=>t.id===1);
  const fixture = path.join(profile,'fixtures','2026-秋日旅行');
  assert(fs.existsSync(path.join(fixture,'DSC08812.jpg')),'Prepare isolated fixture first');
  await step({tool:'get_ui_state',until:{field:'window_status.valid',equals:true,attempts:60,interval_ms:500}});
  await step({tool:'get_ui_state',until:{field:'file_manager.matched_count',at_least:0,attempts:60,interval_ms:300}});
  await run('resize_window',{width:1920,height:1800});
  await run('control_ui',{action:'inspector_open'});
  await run('control_ui',{action:'inspector_width',value:'360'});
  await run('control_navigation',{action:'all'});
  await run('control_ui',{action:'navigation_tab',value:'tags'});
  await run('control_file_manager',{action:'tag',tag_id:1},650);
  assert.equal((await ui('tag-complete')).file_manager.inspector_context,'tag');
  assert.equal((await rootTag()).media_count,18);
  await run('control_file_manager',{action:'inspector_tag_name',value:'旅行 · 测试'});
  await click(1100,629);
  assert.equal((await rootTag()).name,'旅行 · 测试');
  await ui('tag-renamed-final');
  await run('control_file_manager',{action:'inspector_tag_name',value:'旅行'});
  await click(1100,629);
  assert.equal((await rootTag()).name,'旅行');
  await ui('tag-restored-final');
  await click(948,698);
  assert.equal((await rootTag()).color,'#3185ED');
  await click(981,698);
  assert.equal((await rootTag()).color,'#E36B6B');
  await click(1218,737);
  let tag=await rootTag(); assert.equal(tag.display,false); assert(tag.children.every(c=>!c.display));
  await ui('tag-display-off');
  await click(1218,737);
  tag=await rootTag(); assert.equal(tag.display,true); assert(tag.children.every(c=>c.display));
  await ui('tag-complete');
  await click(1227,884);
  await ui('tag-child-menu');
  await run('control_ui',{action:'key_press',value:'escape'});

  await run('control_ui',{action:'navigation_tab',value:'folders'});
  await run('control_navigation',{action:'select_path',value:fixture},800);
  await step({tool:'get_ui_state',until:{field:'file_manager.matched_count',equals:18,attempts:60,interval_ms:300}});
  assert.equal((await ui('folder-complete')).file_manager.inspector_context,'folder');
  await run('control_ui',{action:'deselect'});
  assert.equal((await ui('program-complete')).file_manager.inspector_context,'program');
  const generation=(await ui()).browser.status[0];
  await run('control_navigation',{action:'select_path',value:fixture},500);
  let state=await ui('folder-reselected');
  assert.equal(state.file_manager.inspector_context,'folder');
  assert.equal(state.browser.status[0],generation,'same folder selection must not re-enumerate');
  await run('control_ui',{action:'select_media',value:'1'});
  await click(870,900); // blank area beside the grid, not a file card
  assert.equal((await ui('program-blank-click')).file_manager.inspector_context,'program');
  const changedFile=path.join(fixture,'review-change.jpg');
  assert(!fs.existsSync(changedFile));
  fs.copyFileSync(path.join(fixture,'DSC08812.jpg'),changedFile);
  try {
    await step({tool:'get_ui_state',until:{field:'file_manager.matched_count',equals:19,attempts:60,interval_ms:300},capture:'program-real-change'});
    assert.equal((await ui()).file_manager.inspector_context,'program');
  } finally { fs.unlinkSync(changedFile); }
  await step({tool:'get_ui_state',until:{field:'file_manager.matched_count',equals:18,attempts:60,interval_ms:300},capture:'program-complete'});

  await run('control_ui',{action:'select_media',value:'1'},600);
  await run('control_ui',{action:'inspector_tab',value:'information'});
  state=await ui();
  for(const section of ['basic','tags','exif','path']) {
    if(!state.file_manager.inspector_sections.includes(section)) await run('control_file_manager',{action:'inspector_section',value:section});
  }
  await ui('file-complete');
  const media=await run('get_media',{media_ids:[1]});
  assert.equal(media.media.missing,false); assert(media.media.thumb_path); assert.equal(media.media.rating,4); assert.equal(media.tags.length,5);
  await run('control_file_manager',{action:'inspector_section',value:'basic'});
  await ui('file-exif-complete');
  await run('control_ui',{action:'inspector_tab',value:'extensions'});
  await ui('file-extensions-empty');
  await run('control_ui',{action:'inspector_tab',value:'history'});
  state=await ui('history-complete');
  assert.equal(state.inspector_demo.count,3); assert.equal(state.inspector_demo.selected,'v1');
  await click(1110,470);
  assert.equal((await ui('history-selected-v0')).inspector_demo.selected,'v0');
  await click(1170,235);
  assert.equal((await ui('history-switched-v0')).inspector_demo.active,'v0');
  await click(1005,235);
  state=await ui('history-created-v2');
  assert.equal(state.inspector_demo.count,4); assert.equal(state.inspector_demo.active,'v2');
  await click(1090,280);
  await ui('history-branch-menu');
  await click(955,371);
  state=await ui('history-branch-filtered');
  assert.equal(state.inspector_demo.filter,'2'); assert.equal(state.inspector_demo.selected,'v0.1');
  await click(1090,280); await click(955,316);
  assert.equal((await ui()).inspector_demo.filter,'0');
  await run('resize_window',{width:1920,height:1560});
  await ui('history-standard-height');
  await run('control_ui',{action:'inspector_width',value:'300'});
  await ui('history-narrow');
  await run('control_ui',{action:'inspector_width',value:'360'});
  await delay(700);
  const before=await run('get_debug_info',{page_size:10},2000);
  const after=await run('get_debug_info',{page_size:10});
  assert.equal(after.logs.latest_id,before.logs.latest_id,'settled inspector must stay quiet');
  fs.writeFileSync(path.join(profile,'final-result.json'),JSON.stringify({passed:true,tag:await rootTag(),media,quiet:[before.logs.latest_id,after.logs.latest_id],final_ui:state},null,2));
  console.log(JSON.stringify({passed:true,contexts:4,real_files:18,subfolders:5,tags:8,history_demo_versions:4,quiet:[before.logs.latest_id,after.logs.latest_id]}));
};
