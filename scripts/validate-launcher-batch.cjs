const assert=require('node:assert/strict');
module.exports=async({call,payload,delay,output,step})=>{
  const invoke=async(name,args={})=>payload(await call(name,args));
  const click=async(x,y)=>{
    for(const action of ['pointer_move','pointer_down','pointer_up'])
      await step({tool:'control_ui',arguments:{action,value:`${Math.round(x)},${Math.round(y)}`},wait_ms:300});
  };
  const type=async(value)=>step({tool:'control_ui',arguments:{action:'input_text',value},wait_ms:400});
  await invoke('control_ui',{action:'panel_open',value:'program-launcher/panel'});
  await delay(1000);
  await invoke('control_ui',{action:'window_select',value:'main'});
  let state=await invoke('launcher.snapshot');
  const entries=state.entries.slice(0,2);
  assert.equal(entries.length,2,'two launcher entries are required for list verification');
  const panel=(await invoke('get_ui_state')).panel_instances.find(item=>item.id==='program-launcher/panel@1');
  assert(panel,'launcher panel must be mounted in the same MCP process');
  const [px,py,pw]=panel.client_bounds.split(',').map(Number);
  // MCP pointer input uses logical client coordinates; transform from the captured 2048px preview.
  await click(px+pw*.59,py+68);
  for(let i=0;i<12;i++){
    await delay(150);
    state=await invoke('launcher.snapshot');
    if(state.ui?.batch_mode)break;
  }
  assert.equal(state.ui.batch_mode,true,'toolbar click must switch to batch list');
  await step({tool:'screenshot_window',capture:'launcher-batch-list'});

  const before=entries.map(entry=>({id:entry.id,name:entry.display_name,category_id:entry.category_id,hidden:entry.hidden}));
  // First two visible rows, on their checkbox side.
  const rowX=px+pw*.215;
  await click(rowX,py+218);
  await click(rowX,py+258);
  state=await invoke('launcher.snapshot');
  assert.deepEqual(state.ui.batch_selected_ids,before.map(entry=>entry.id),'row clicks must select two distinct programs');
  await step({tool:'screenshot_window',capture:'launcher-batch-selection'});

  const categories=state.categories;
  const destination=categories.find(category=>before.every(entry=>entry.category_id!==category.id));
  // Choose a real destination from the right batch inspector's category menu.
  if(destination){
    await click(px+pw*.90,py+122);
    await delay(200);
    await step({tool:'screenshot_window',capture:'launcher-batch-category-menu'});
    // Category menu rows are ordered: Do not change, General, then saved categories.
    const destinationIndex=categories.findIndex(category=>category.id===destination.id);
    await click(px+pw*.83,py+160+(destinationIndex+1)*27);
    state=await invoke('launcher.snapshot');
    assert.equal(state.ui.batch_category,destination.id,'category dropdown choice must update the batch draft');
  }
  await click(px+pw*.895,py+194);
  await type('批量验-');
  await click(px+pw*.895,py+258);
  await type('-批量验');
  state=await invoke('launcher.snapshot');
  assert.equal(state.ui.batch_prefix,'批量验-','prefix input must receive typed content');
  assert.equal(state.ui.batch_suffix,'-批量验','suffix input must receive typed content');
  await click(px+pw*.895,py+435);
  state=await invoke('launcher.snapshot');
  assert.equal(state.ui.batch_hidden,true,'visibility choice must update the batch draft');
  await step({tool:'screenshot_window',capture:'launcher-batch-edit-draft'});
  await click(px+pw*.895,py+482);
  state=await invoke('launcher.snapshot');
  for(const entry of before){
    const updated=state.entries.find(item=>item.id===entry.id);
    assert.equal(updated.display_name,`批量验-${entry.name}-批量验`);
    assert.equal(updated.hidden,true);
    if(destination) assert.equal(updated.category_id,destination.id);
  }
  await step({tool:'screenshot_window',capture:'launcher-batch-applied'});
  for(const entry of before) await invoke('launcher.update_program',{id:entry.id,name:entry.name,category_id:entry.category_id,hidden:entry.hidden});
  state=await invoke('launcher.snapshot');
  for(const entry of before){
    const restored=state.entries.find(item=>item.id===entry.id);
    assert.equal(restored.display_name,entry.name);
    assert.equal(restored.hidden,entry.hidden);
    assert.equal(restored.category_id,entry.category_id);
  }
  console.log(JSON.stringify({result:'batch mode, row selection, category/name/visibility edits and restoration passed',ids:before.map(entry=>entry.id),panel:panel.bounds,profile:output}));
};
