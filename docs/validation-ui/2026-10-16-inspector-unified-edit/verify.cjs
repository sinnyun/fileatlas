// Same-process MCP; pointer coordinates from this host's screenshot geometry.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
module.exports = async ({step, profile, delay}) => {
  const run = async (tool, args = {}, capture, wait = 400) => (await step({tool,
    arguments: args, wait_ms: wait, ...(capture ? {capture} : {})})).result;
  const pointer = async (action, x, y) => {
    for(let attempt=0; attempt<8; attempt++) {
      try { return await run('control_ui', {action, value:`${x},${y}`}, null, 450); }
      catch(error) {
        if(!String(error).includes('pointer request is still pending') || attempt===7) throw error;
        await delay(500);
      }
    }
  };
  const click = async (x, y) => {
    await pointer('pointer_down',x,y);
    await pointer('pointer_up',x,y);
  };
  const tag = async () => (await run('list_library')).tags.find(t => t.id === 1);
  await step({tool:'get_ui_state',until:{field:'window_status.valid',equals:true,attempts:60,interval_ms:500}});
  await step({tool:'get_ui_state',until:{field:'file_manager.matched_count',at_least:0,attempts:60,interval_ms:300}});
  await run('resize_window', {width:1920,height:1560});
  await run('control_ui', {action:'inspector_open'});
  await run('control_ui', {action:'inspector_width',value:'360'});
  await run('control_navigation', {action:'all'});
  await run('control_ui', {action:'navigation_tab',value:'tags'});
  await run('control_file_manager', {action:'tag',tag_id:1}, null, 700);
  // Reset only this known isolated fixture after an interrupted verification.
  const initial = await tag();
  await run('control_file_manager', {action:'inspector_tag_name',value:'旅行'});
  await click(981,359);
  await click(949,426);
  if(!initial.display) await click(1218,503);
  await click(1083,547);
  const before = await tag();
  assert.equal(before.name, '旅行');
  assert.equal(before.color, '#E36B6B');
  assert.equal(before.display, true);
  await run('get_ui_state', {}, 'tag-expanded');
  await run('control_file_manager', {action:'inspector_tag_name',value:'旅行 · 统一确认'});
  await click(948,359); // Blue swatch.
  await click(1140,426); // Star icon.
  await click(1218,503); // Visibility draft off.
  const pending = await tag();
  assert.deepEqual(pending, before, 'Draft controls must not write before confirmation');
  await run('get_ui_state', {}, 'tag-draft');
  await click(1083,547); // One confirmation for all four fields.
  const changed = await tag();
  assert.equal(changed.name, '旅行 · 统一确认');
  assert.equal(changed.color, '#3185ED');
  assert.equal(changed.icon, 'Star');
  assert.equal(changed.display, false);
  assert(changed.children.every(t => !t.display));
  await run('get_ui_state', {}, 'tag-confirmed');
  await run('control_file_manager', {action:'inspector_tag_name',value:'旅行'});
  await click(981,359); // Coral.
  await click(949,426); // Default Tag icon.
  await click(1218,503);
  await click(1083,547);
  const restored = await tag();
  assert.equal(restored.name, '旅行');
  assert.equal(restored.color, '#E36B6B');
  assert.equal(restored.icon, 'Tag');
  assert.equal(restored.display, true);
  assert(restored.children.every(t => t.display));
  await run('get_ui_state', {}, 'tag-restored');

  await run('control_navigation', {action:'select_path', value:path.join(profile,'fixtures','2026-秋日旅行')}, null, 700);
  await step({tool:'get_ui_state',until:{field:'file_manager.matched_count',equals:18,attempts:60,interval_ms:300}});
  await run('control_ui', {action:'select_media',value:'1'});
  for(const tab of ['information','history','extensions']) {
    await run('control_ui', {action:'inspector_tab',value:tab});
    const state = await run('get_ui_state', {}, `file-${tab}`);
    assert.equal(state.file_manager.inspector_context, 'file');
    assert.deepEqual(state.file_manager.selected_media_ids, [1]);
  }
  await run('resize_window', {width:1920,height:1080});
  await run('control_ui', {action:'inspector_width',value:'300'});
  await run('control_ui', {action:'inspector_tab',value:'history'});
  await run('get_ui_state', {}, 'history-small');
  await run('control_navigation', {action:'all'});
  await run('control_file_manager', {action:'tag',tag_id:1}, null, 700);
  await run('get_ui_state', {}, 'tag-small');
  const quietBefore = await run('get_debug_info', {page_size:10}, null, 1800);
  const quietAfter = await run('get_debug_info', {page_size:10});
  assert.equal(quietBefore.logs.latest_id, quietAfter.logs.latest_id);
  fs.writeFileSync(path.join(profile,'unified-result.json'), JSON.stringify({passed:true,
    draft_unchanged:true, one_confirm_saved_all:true, restored:true,
    quiet:[quietBefore.logs.latest_id,quietAfter.logs.latest_id]},null,2));
  console.log(JSON.stringify({passed:true,quiet:quietAfter.logs.latest_id}));
};
