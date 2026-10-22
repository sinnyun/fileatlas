const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const net=require('node:net');
const {execFileSync}=require('node:child_process');
module.exports=async({step,call,payload,delay,profile})=>{
  const invoke=async(tool,args={})=>(await step({tool,arguments:args,wait_ms:100})).result;
  const directory=path.join(profile,'documents');fs.mkdirSync(directory,{recursive:true});
  const image=path.join(directory,'photo.png');
  execFileSync('python',['-c','import sys,numpy as n; from PIL import Image; rng=n.random.default_rng(42); y,x=n.mgrid[:800,:1200]; a=n.stack([x*255/1199,y*255/799,(x+y)*255/1998],axis=2)+rng.normal(0,12,(800,1200,3)); Image.fromarray(n.clip(a,0,255).astype("uint8")).save(sys.argv[1])',image]);
  const port=await new Promise(resolve=>{const s=net.createServer();s.listen(0,'127.0.0.1',()=>{const p=s.address().port;s.close(()=>resolve(p));});});
  await invoke('workcrate-ui.configure_vectorcraft',{executable:'C:\\Program Files\\VectorCraft\\vectorcraft.exe',port});
  const created=await invoke('workcrate-ui.manage_project',{action:'create',name:'PDF压缩验证',directory});
  const project_id=created.project.id,A=created.boards[0].id;
  const B=(await invoke('workcrate-ui.create_page',{project_id})).board_id;
  await invoke('workcrate-ui.update_page',{project_id,board_id:A,info:{material:'JPEG压缩物料',quantity:'2'}});
  await invoke('workcrate-ui.open_board',{project_id,board_id:A});
  let ready=false;for(let i=0;i<120;i++){const s=payload(await call('workcrate-ui.state',{}));if(s.ui?.canvas.state?.board_id===A&&!s.ui.canvas.busy&&!s.ui.busy){ready=true;break;}await delay(150);}assert.ok(ready);
  await invoke('workcrate-ui.canvas_place',{path:image,at:[80,120],link:false,confirm:true});
  const reports={};
  for(const [name,board_ids] of [['single',[A]],['multiple',[A,B]]]){
    const output=path.join(directory,`${name}.pdf`);
    const result=await invoke('workcrate-ui.export_pages',{project_id,board_ids,path:output,format:'pdf',confirm:true});
    assert.equal(result.page_count,board_ids.length);
    const report=JSON.parse(execFileSync('python',['-c',
      'import json,sys,fitz; r=fitz.open(sys.argv[1]); objects="\\n".join(r.xref_object(i) for i in range(1,r.xref_length())); images=[{"width":i[2],"height":i[3],"filter":r.xref_get_key(i[0],"Filter")[1]} for i in r[0].get_images()]; r[0].get_pixmap(matrix=fitz.Matrix(.8,.8)).save(sys.argv[2]); print(json.dumps({"pages":r.page_count,"attachments":r.embfile_names(),"embedded_files":"/EmbeddedFile" in objects,"text":r[0].get_text(),"images":images}))',output,path.join(directory,`${name}-preview.png`)],{encoding:'utf8'}));
    assert.equal(report.pages,board_ids.length);assert.deepEqual(report.attachments,[]);assert.equal(report.embedded_files,false);
    assert.ok(report.text.replace(/\s/g,'').includes('JPEG压缩物料'));assert.ok(report.images.length);
    if(!process.env.FILEATLAS_PDF_BASELINE)assert.ok(report.images.some(i=>i.filter.includes('DCTDecode')),'opaque PNG must use JPEG compression in PDF');
    reports[name]={...report,bytes:fs.statSync(output).size};
  }
  const jpg=path.join(directory,'single.jpg');
  await invoke('workcrate-ui.export_pages',{project_id,board_ids:[A],path:jpg,format:'image',confirm:true});
  reports.jpg={bytes:fs.statSync(jpg).size};
  fs.writeFileSync(path.join(profile,'compression-summary.json'),JSON.stringify(reports,null,2));
};
