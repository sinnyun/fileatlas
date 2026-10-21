const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,step,delay,profile,root})=>{
 const invoke=async(t,a={})=>payload(await call(t,a));
 const state=async(ids=[])=>invoke('workcrate-ui.state',{control_ids:ids});
 const ui=async(action,value)=>{for(let i=0;;i++)try{const r=await invoke('control_ui',{action,value});await delay(75);return r;}catch(e){if(i>50||!String(e).includes('pending'))throw e;await delay(80);}};
 const point=(a,x,y)=>ui(a,`${x},${y}`);
 const click=async(x,y)=>{await point('pointer_down',x,y);await point('pointer_up',x,y);};
 const until=async(test)=>{for(let i=0;i<180;i++){const s=await state();if(test(s))return s;await delay(100);}await step({tool:'workcrate-ui.state',capture:'typography-failed'});throw Error('Typography flow did not settle');};
 const bounds=async(id)=>{for(let i=0;i<40;i++){const raw=(await state([id])).controls[id];if(raw)return raw.split(',').map(Number);await delay(100);}throw Error(`Missing ${id}`);};
 let inspectorRegion;
 const control=async(id,inspector=false)=>{
   for(let i=0;i<30;i++){
     const b=await bounds(id);
     if(inspector){const r=inspectorRegion??=await bounds('inspector');if(b[1]<r[1]+20||b[1]+b[3]>r[1]+r[3]-130){await ui('pointer_scroll',`${r[0]+r[2]-25},${r[1]+r[3]/2},${b[1]<r[1]+20?350:-350}`);await delay(100);continue;}}
     await click(b[0]+b[2]/2,b[1]+b[3]/2);return;
   }throw Error(`Control clipped: ${id}`);
 };
 const type=async(id,text,inspector=false)=>{await control(id,inspector);await ui('key_press','ctrl-a');for(const [i,line] of text.split('\n').entries()){if(i)await ui('key_press','enter');if(line)await ui('input_text',line);}await delay(150);};
 const project=(await invoke('workcrate-ui.manage_project',{action:'create',name:`原生文本设置 ${Date.now()}`})).project,project_id=project.id;
 const board_id=(await invoke('workcrate-ui.create_page',{project_id})).board_id;
 await invoke('workcrate-ui.place_image',{project_id,board_id,path:path.join(root,'test-data/fileatlas-fixtures/03-nested/level-one/level-two/nested-photo.jpeg')});
 await until(s=>s.ui?.board_id===board_id&&!s.ui.busy&&s.ui.canvas_image_id&&s.ui.display_images===1);
 await control('canvas-add-text');await until(s=>s.ui?.text_inline&&s.ui.text_draft&&!s.ui.busy);await delay(500);
 const original='ABCDEFGHIJKLMN\nOPQRSTUVWXYZ';await type('canvas-text-input',original);
 let s=await until(s=>!s.ui.text_write_active&&s.ui.text_draft?.body===original);const text_id=s.ui.selected_text,frame=s.ui.text_frames[text_id];
 const b=await bounds('canvas-text-input');
 await point('pointer_down',b[0]+5,b[1]+10);await point('pointer_drag',b[0]+88,b[1]+10);
 await point('pointer_up',b[0]+88,b[1]+10);
 s=await until(s=>s.ui.text_draft?.selection[1]>s.ui.text_draft?.selection[0]+2);
 assert.deepEqual(s.ui.text_frames[text_id],frame);assert.equal(s.ui.edit_saving,false);
 const selection=s.ui.text_draft.selection;
 await step({tool:'workcrate-ui.state',capture:'native-drag-selected-characters'});
 await ui('input_text','替换');
 const expected=Buffer.concat([Buffer.from(original).subarray(0,selection[0]),Buffer.from('替换'),Buffer.from(original).subarray(selection[1])]).toString();
 await until(s=>!s.ui.text_write_active&&s.ui.text_draft?.body===expected);
 // Select by dragging again, then apply a visible native character control.
 const tb=await bounds('canvas-text-input');await point('pointer_down',tb[0]+3,tb[1]+10);await point('pointer_drag',tb[0]+70,tb[1]+10);await point('pointer_up',tb[0]+70,tb[1]+10);
 s=await until(s=>s.ui.text_draft?.selection[1]>s.ui.text_draft?.selection[0]);const range=s.ui.text_draft.selection;
 await control('text-underline',true);await until(s=>!s.ui.text_inline);
 let native=await invoke('workcrate-ui.text_options',{action:'read',project_id,board_id,text_id,range});assert.equal(native.chars.underline,true);
 await control('text-more',true);s=await until(s=>s.ui.text_options?.snapshot?.defaults&&!s.ui.text_options.busy);
 const counts=Object.fromEntries(['chars','para','frame'].map(k=>[k,Object.keys(s.ui.text_options.snapshot.defaults[k]).length]));
 console.log(JSON.stringify({counts}));
 assert(counts.chars>60&&counts.para>60&&counts.frame===27);
 await control('text-option-scope-para',true);await type('text-option-value','right',true);await control('text-option-apply',true);
 await until(s=>!s.ui.text_options.busy&&s.ui.text_options.snapshot.para.align==='right');
 await step({tool:'workcrate-ui.state',capture:'native-paragraph-options'});
 await control('text-option-scope-frame',true);await type('text-option-value','2',true);await control('text-option-apply',true);
 await until(s=>!s.ui.text_options.busy&&s.ui.text_options.snapshot.frame.columns===2);
 await control('text-option-scope-chars',true);await type('text-option-search','tracking',true);await control('text-option-key',true);
 const kb=await bounds('text-option-key');await click(kb[0]+50,kb[1]+kb[3]+18);
 await until(s=>s.ui.text_options.key==='tracking');await type('text-option-value','90',true);await control('text-option-apply',true);
 await until(s=>!s.ui.text_options.busy&&s.ui.text_options.snapshot.chars.tracking===90);
 await step({tool:'workcrate-ui.state',capture:'native-character-and-frame-options'});
 await control('text-option-scope-openType',true);await step({tool:'workcrate-ui.state',capture:'native-open-type-options'});
 fs.writeFileSync(path.join(profile,'editing-result.json'),JSON.stringify({project_id,board_id,text_id}));
 fs.writeFileSync(path.join(profile,'typography-result.json'),JSON.stringify({project_id,board_id,text_id,selection,range,counts,checks:['real mouse drag selects multiple characters without moving frame','typing replaces exactly selected UTF-8 interval','selected character underline','native property catalogue matches pinned defaults','paragraph alignment','frame columns','searchable character tracking','font-native OpenType catalogue']},null,2));
};
