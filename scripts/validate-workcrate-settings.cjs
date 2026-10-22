// Reopens the defaults profile, checks persisted scopes, complete settings and small-window scrolling.
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
module.exports=async({call,payload,step,delay,profile})=>{
  const invoke=async(tool,args={},extra={})=>(await step({tool,arguments:args,wait_ms:180,...extra})).result;
  const ui=(action,value)=>invoke('control_ui',{action,...(value===undefined?{}:{value})});
  const read=async(ids=[],project)=>payload(await call('workcrate-ui.state',{control_ids:ids,...(project?{project_id:project}:{})}));
  const until=async(test,label,ids=[])=>{let s;for(let i=0;i<100;i++){s=await read(ids);if(test(s))return s;await delay(150);}throw Error(`${label}: ${JSON.stringify(s)}`);};
  const box=value=>value.split(',').map(Number);
  const click=async(x,y)=>{await ui('pointer_down',`${x},${y}`);await ui('pointer_up',`${x},${y}`);};
  const clickId=async id=>{if(id.startsWith("settings-"))await show(id);const s=await read([id]);const[x,y,w,h]=box(s.controls[id]);await click(x+w/2,y+h/2);};
  const type=async(id,value)=>{await clickId(id);await ui('key_press','ctrl-a');if(value)await ui('input_text',value);else await ui('key_press','backspace');};
  const show=async id=>{
    for(let i=0;i<16;i++){
      const s=await read([id]);const host=payload(await call('get_ui_state',{}));
      const p=host.settings_open?host.panels.find(p=>p.id==='settings'):host.panel_instances.find(p=>p.id===host.active_panel);
      const[x,y,w,h]=box(p.bounds);
      let delta=-250;
      if(s.controls[id]){const[,iy,,ih]=box(s.controls[id]);if(iy>=y+45&&iy+ih<=y+h-(host.settings_open?58:12))return;if(iy<y+45)delta=250;}
      await ui('pointer_scroll',`${x+w-100},${y+h/2},${delta}`);
    }
    throw Error(`could not scroll ${id} into view`);
  };
  const saved=JSON.parse(fs.readFileSync(path.join(profile,'defaults-summary.json'),'utf8'));
  await delay(1200);
  let s=await read([],saved.project);
  assert.equal(s.project_presets.fields[0].values[0],'修改后的专用类别');
  assert.ok(s.field_presets.fields[0].values.includes('修改后的专用类别'));
  await ui('settings_open');await ui('settings_plugin_page','fileatlas.workcrate-ui');
  await until(s=>s.defaults_form?.project_id===null,'program form');
  await invoke('workcrate-ui.state',{}, {capture:'settings-program-top'});
  // Same-process measured form coordinates; offset follows the inspected SettingsPage rows.
  const scope=box((await read(['settings-preset-scope'])).controls['settings-preset-scope']);
  const sx=scope[0]+scope[2]-130, sy=scope[1]+scope[3]/2;
  await click(sx,sy+110);await invoke('get_ui_state',{}, {capture:'settings-sort-menu'});
  await click(sx,sy+174);
  await until(s=>s.defaults_form.preferences?.sort_by_name===true,'default sort selection');
  await click(sx,sy+166);await click(sx,sy+230);
  await until(s=>s.defaults_form.preferences?.image_export_jpeg===true,'default export selection');
  await show('settings-vectorcraft-port');
  await type('settings-vectorcraft-cli','C:\\Program Files\\VectorCraft\\vectorcraft-cli.exe');
  await type('settings-vectorcraft-port','0');await clickId('settings-vectorcraft-save');
  await until(s=>/65535/.test(s.defaults_form?.config_error||''),'invalid port feedback');
  await invoke('workcrate-ui.state',{}, {capture:'settings-invalid-port'});
  const port=JSON.parse(fs.readFileSync(path.join(profile,'workcrate','vectorcraft.json'),'utf8')).port;
  await type('settings-vectorcraft-port',String(port));await clickId('settings-vectorcraft-save');
  await until(s=>s.defaults_form.config_saved&&!s.defaults_form.config_busy&&!s.defaults_form.config_error,'program config saved');
  await delay(400);
  const config=JSON.parse(fs.readFileSync(path.join(profile,'workcrate','vectorcraft.json'),'utf8'));
  assert.equal(config.cli_executable,'C:\\Program Files\\VectorCraft\\vectorcraft-cli.exe');
  await invoke('workcrate-ui.state',{}, {capture:'settings-complete-program-config'});
  await ui('settings_close');
  await invoke('workcrate-ui.navigate',{project_id:saved.project});
  await until(s=>s.ui?.project_id===saved.project&&!s.ui.busy,'project view');
  assert.equal((await read()).ui.sort_by_name,true);
  await clickId('project-defaults');
  await until(s=>s.defaults_form?.project_id===saved.project,'project defaults');
  await invoke('resize_window',{width:1600,height:1100});
  await show('settings-preset-name');
  const custom=`制作工艺${Date.now()}`;
  await type('settings-preset-name',custom);await show('settings-preset-add-field');await clickId('settings-preset-add-field');
  await until(s=>!s.defaults_form.pending&&!s.defaults_form.busy&&s.defaults_form.presets.fields.some(f=>f.key===`extra:${custom}`),'custom project field');
  await invoke('workcrate-ui.state',{}, {capture:'settings-small-custom-fields'});
  const after=await read([],saved.project);
  assert.ok(after.project_presets.fields.some(f=>f.key===`extra:${custom}`));
  assert.ok(!after.field_presets.fields.some(f=>f.key===`extra:${custom}`));
  await show('settings-preset-save');
  await invoke('workcrate-ui.state',{}, {capture:'settings-small-bottom'});
  await invoke('resize_window',{width:3286,height:1831});
  fs.writeFileSync(path.join(profile,'settings-summary.json'),JSON.stringify({passed:true,checks:['restart scoped defaults','default sort/format UI','CLI persisted','invalid port refused','small window scroll','custom project field isolation']},null,2));
};
