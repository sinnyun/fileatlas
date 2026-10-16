const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
module.exports = async ({step,profile}) => {
  const run=async(tool,args={},wait=350,capture)=>(await step({tool,arguments:args,wait_ms:wait,...(capture?{capture}:{})})).result;
  await step({tool:'get_ui_state',until:{field:'window_status.valid',equals:true,attempts:60,interval_ms:500}});
  await run('resize_window',{width:1920,height:1800});
  await run('control_ui',{action:'inspector_width',value:'360'});
  await run('control_navigation',{action:'select_path',value:path.join(profile,'fixtures','2026-秋日旅行')},800);
  await step({tool:'get_ui_state',until:{field:'gallery.matched_count',equals:18,attempts:60,interval_ms:300}});
  await run('control_ui',{action:'select_media',value:'1'},500);
  await run('control_ui',{action:'inspector_tab',value:'information'});
  let state=await run('get_ui_state');
  assert.equal(state.inspector_demo.enabled,false);
  for(const section of ['basic','tags','exif','path']) {
    if(!state.gallery.inspector_sections.includes(section)) await run('control_gallery',{action:'inspector_section',value:section});
  }
  await run('get_ui_state',{},700,'file-complete');
  await run('control_gallery',{action:'inspector_section',value:'basic'});
  await run('get_ui_state',{},400,'file-exif-complete');
  await run('control_ui',{action:'inspector_tab',value:'history'});
  state=await run('get_ui_state',{},400,'history-normal');
  assert.equal(state.inspector_demo.enabled,false);
  assert.equal(state.inspector_demo.count,null);
  await run('resize_window',{width:1920,height:1080});
  await run('control_ui',{action:'inspector_width',value:'300'});
  await run('get_ui_state',{},400,'history-normal-small');
  const before=await run('get_debug_info',{page_size:10},1500);
  const after=await run('get_debug_info',{page_size:10});
  assert.equal(before.logs.latest_id,after.logs.latest_id);
  fs.writeFileSync(path.join(profile,'normal-result.json'),JSON.stringify({passed:true,demo_enabled:false,quiet:[before.logs.latest_id,after.logs.latest_id]},null,2));
  console.log(JSON.stringify({passed:true,demo_enabled:false,quiet:[before.logs.latest_id,after.logs.latest_id]}));
};
