const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,step,delay,profile})=>{
 const invoke=async(t,a={})=>payload(await call(t,a));
 const ui=async(action,value)=>{for(let i=0;;i++){try{const r=await invoke('control_ui',{action,...(value===undefined?{}:{value})});await delay(100);return r;}catch(e){if(i>40||!String(e).includes('pending'))throw e;await delay(90);}}};
 const state=(ids=[])=>invoke('workcrate-ui.state',{control_ids:ids});
 const until=async(test)=>{for(let i=0;i<180;i++){const s=await state();if(test(s))return s;await delay(100);}await step({tool:'workcrate-ui.state',capture:'preset-failed'});throw Error('Preset flow did not settle');};
 const saved=JSON.parse(fs.readFileSync(path.join(profile,'editing-result.json')));
 await invoke('workcrate-ui.navigate',{project_id:saved.project_id,board_id:saved.board_id});
 await until(s=>s.ui?.board_id===saved.board_id&&!s.ui.busy);await delay(400);
 await ui('settings_open');await ui('settings_plugin_page','fileatlas.workcrate-ui');await delay(500);
 await step({tool:'get_ui_state',capture:'workcrate-plugin-settings'});
 let panel=(await invoke('get_ui_state')).panels.find(p=>p.id==='settings'||p.id.startsWith('settings@'));
 assert(panel,'Settings panel must be open');let pb=panel.bounds.split(',').map(Number);
 const scroll=async(delta)=>{await ui('pointer_scroll',`${pb[0]+pb[2]-35},${pb[1]+pb[3]*0.65},${delta}`);await delay(250);};
 const bound=async(id,settings=true)=>{
   for(let i=0;i<25;i++){
     const raw=(await state([id])).controls[id];
     if(raw){const b=raw.split(',').map(Number);const top=settings?pb[1]+180:200,bottom=settings?pb[1]+pb[3]-22:1060;
       if(b[1]>=top&&b[1]+b[3]<=bottom)return b;
       if(settings)await scroll(b[1]<top?400:-450);
       else{await ui('pointer_scroll',`1730,750,${b[1]<top?400:-450}`);await delay(200);}
     }else{if(settings)await scroll(-450);else await delay(100);}
   }
   await step({tool:'get_ui_state',capture:'preset-control-not-visible'});throw Error(`Control not visible: ${id}`);
 };
 const click=async(id,settings=true)=>{const b=await bound(id,settings);await ui('pointer_down',`${b[0]+b[2]/2},${b[1]+b[3]/2}`);await ui('pointer_up',`${b[0]+b[2]/2},${b[1]+b[3]/2}`);await delay(150);};
 const type=async(id,value)=>{await click(id);await ui('key_press','ctrl-a');await ui('input_text',value);};
 if(!(await state()).field_presets.fields.some(f=>f.key==='extra:安装方式')) {
 for(const [name,choices] of [['制作工艺',['写真覆膜','UV 打印']],['安装方式',['落地','悬挂']]]){
   let stored=(await state()).field_presets.fields.find(f=>f.key===`extra:${name}`);
   if(!stored){await type('settings-preset-name',name);await click('settings-preset-add-field');await until(s=>s.field_presets.fields.some(f=>f.key===`extra:${name}`));}
   for(let i=0;i<choices.length;i++){
     stored=(await state()).field_presets.fields.find(f=>f.key===`extra:${name}`);
     if(stored?.values[i]===choices[i])continue;
     if(!stored?.values[i])await click(`settings-add:extra:${name}`);
     await type(`settings-value-extra:${name}-${i}`,choices[i]);
     await until(s=>s.field_presets.fields.find(f=>f.key===`extra:${name}`)?.values[i]===choices[i]);
   }
 }
 }
 // No Save click: leaving valid input must publish it automatically.
 await delay(350);
 let s=await until(s=>s.field_presets.fields.some(f=>f.key==='extra:安装方式'&&f.values.length===2));
 assert.deepEqual(s.field_presets.fields.find(f=>f.key==='extra:制作工艺').values,['写真覆膜','UV 打印']);
 await step({tool:'get_ui_state',capture:'custom-fields-saved'});
 await ui('settings_close');await delay(350);
 await click('preset-extra:制作工艺',false);await step({tool:'workcrate-ui.state',capture:'field-preset-dropdown'});
 const beforeFieldWrite=(await state()).ui;
 {const b=await bound('preset-extra:制作工艺',false);await ui('pointer_down',`${b[0]+55},${b[1]+b[3]+16}`);await ui('pointer_up',`${b[0]+55},${b[1]+b[3]+16}`);}
 s=await until(s=>!s.ui.busy&&['写真覆膜','UV 打印'].includes(s.projects.find(p=>p.id===saved.project_id).boards.find(b=>b.id===saved.board_id).info.extra['制作工艺']));
 if(beforeFieldWrite.canvas_image_id) s=await until(s=>s.ui.canvas_image_id!==beforeFieldWrite.canvas_image_id);
 const choice=s.projects.find(p=>p.id===saved.project_id).boards.find(b=>b.id===saved.board_id).info.extra['制作工艺'];
 assert.equal(s.ui.load_generation,beforeFieldWrite.load_generation,'editing a page field must not reload the project catalog');
 assert.notEqual(s.ui.canvas_image_id,beforeFieldWrite.canvas_image_id,'page information must update its A3 header layer');
 assert.deepEqual(s.ui.display_image_ids,beforeFieldWrite.display_image_ids,'field edits must preserve linked image textures');
 await click('preset-material',false);{const b=await bound('preset-material',false);await ui('pointer_down',`${b[0]+55},${b[1]+b[3]+16}`);await ui('pointer_up',`${b[0]+55},${b[1]+b[3]+16}`);}
 await until(s=>!s.ui.busy&&s.projects.find(p=>p.id===saved.project_id).boards.find(b=>b.id===saved.board_id).info.material.length>0);
 const beforeSameNavigation=(await state()).ui;
 await invoke('workcrate-ui.navigate',{project_id:saved.project_id,board_id:saved.board_id});
 await delay(250);
 s=await state();
 assert.equal(s.ui.load_generation,beforeSameNavigation.load_generation,'repeated navigation to the selected page must be a no-op');
 const beforeExternalWrite=s.ui;
 await invoke('workcrate-ui.update_page',{project_id:saved.project_id,board_id:saved.board_id,info:{quantity:'2 套'}});
 s=await until(s=>s.ui?.board_id===saved.board_id&&s.ui.canvas_image_id!==beforeExternalWrite.canvas_image_id);
 assert.equal(s.ui.load_generation,beforeExternalWrite.load_generation,'same-board MCP updates must refresh locally without catalog reload');
 assert.deepEqual(s.ui.display_image_ids,beforeExternalWrite.display_image_ids,'same-board MCP updates must retain linked-image textures');
 await step({tool:'workcrate-ui.state',capture:'preset-values-applied'});
 const before=(await state()).ui;await delay(1000);assert.deepEqual((await state()).ui,before);
 fs.writeFileSync(path.join(profile,'presets-auto-result.json'),JSON.stringify({project_id:saved.project_id,board_id:saved.board_id,choice,checks:['settings page registered','two custom fields added using visible settings controls','two choices per field auto-saved without Save button','custom preset dropdown selected with live defaults','field write preserves load generation and linked image textures while updating the A3 header layer','builtin preset dropdown selected','repeated same-page navigation does not reload','same-board MCP update refreshes locally and preserves image textures','quiet state']},null,2));
};
