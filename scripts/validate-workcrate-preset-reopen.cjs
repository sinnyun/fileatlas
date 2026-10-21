const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
module.exports=async({call,payload,step,delay,profile})=>{
 const invoke=async(t,a={})=>payload(await call(t,a));
 const saved=JSON.parse(fs.readFileSync(path.join(profile,'presets-auto-result.json')));
 const text=JSON.parse(fs.readFileSync(path.join(profile,'typography-result.json')));
 await invoke('workcrate-ui.navigate',{project_id:saved.project_id,board_id:saved.board_id});
 let s;for(let i=0;i<150;i++){s=await invoke('workcrate-ui.state',{control_ids:['preset-extra:安装方式']});if(s.ui?.board_id===saved.board_id&&!s.ui.busy&&s.controls['preset-extra:安装方式'])break;await delay(100);}
 assert.deepEqual(s.field_presets.fields.find(f=>f.key==='extra:安装方式').values,['落地','悬挂']);
 assert.deepEqual(s.field_presets.fields.find(f=>f.key==='extra:制作工艺').values,['写真覆膜','UV 打印']);
 const native=await invoke('workcrate-ui.text_options',{action:'read',project_id:text.project_id,board_id:text.board_id,text_id:text.text_id,range:text.range});
 assert.equal(native.chars.underline,true);assert.equal(native.chars.tracking,90);assert.equal(native.frame.columns,2);
 const b=s.controls['preset-extra:安装方式'].split(',').map(Number);
 for(const action of ['pointer_down','pointer_up']){await invoke('control_ui',{action,value:`${b[0]+b[2]/2},${b[1]+b[3]/2}`});await delay(120);}
 await step({tool:'workcrate-ui.state',capture:'presets-and-native-styles-after-restart'});
 fs.writeFileSync(path.join(profile,'preset-reopen-result.json'),JSON.stringify({checks:['two fields and four choices survive process restart','editor preset button available after reopen','selected underline and tracking survive restart','native frame columns survive restart']},null,2));
};
