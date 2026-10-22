const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const net=require('node:net');
const {execFileSync}=require('node:child_process');
module.exports=async({step,profile})=>{
  const invoke=async(tool,args={})=>(await step({tool,arguments:args,wait_ms:150})).result;
  const port=await new Promise(resolve=>{const s=net.createServer();s.listen(0,'127.0.0.1',()=>{const p=s.address().port;s.close(()=>resolve(p));});});
  const directory=path.join(profile,'documents');fs.mkdirSync(directory,{recursive:true});
  await invoke('workcrate-ui.configure_vectorcraft',{executable:'C:\\Program Files\\VectorCraft\\vectorcraft.exe',port});
  const created=await invoke('workcrate-ui.manage_project',{action:'create',name:'PDF默认配置验证',directory});
  const project_id=created.project.id,A=created.boards[0].id;
  const B=(await invoke('workcrate-ui.create_page',{project_id})).board_id;
  await invoke('workcrate-ui.update_page',{project_id,board_id:A,info:{material:'PDF测试物料',quantity:'2'}});
  for(const [name,board_ids] of [['single',[A]],['multiple',[A,B]]]){
    const output=path.join(directory,`${name}.pdf`);
    const result=await invoke('workcrate-ui.export_pages',{project_id,board_ids,path:output,format:'pdf',confirm:true});
    assert.equal(result.page_count,board_ids.length);
    // Parse the generated PDF catalogue, including compressed object streams.
    const report=JSON.parse(execFileSync('python',['-c',
      'import json,sys,fitz; r=fitz.open(sys.argv[1]); objects="\\n".join(r.xref_object(i) for i in range(1,r.xref_length())); print(json.dumps({"pages":r.page_count,"attachments":r.embfile_names(),"associated_files":r.xref_get_key(r.pdf_catalog(),"AF")[0]!="null","embedded_files":"/EmbeddedFiles" in objects or "/EmbeddedFile" in objects,"header":r.metadata["format"],"text":r[0].get_text()}))',output],{encoding:'utf8'}));
    assert.equal(report.pages,board_ids.length);assert.deepEqual(report.attachments,[]);
    assert.equal(report.associated_files,false);assert.equal(report.embedded_files,false);
    assert.equal(report.header,'PDF 1.7');assert.ok(report.text.replace(/\s/g,'').includes('PDF测试物料'));
    fs.writeFileSync(path.join(directory,`${name}-inspection.json`),JSON.stringify(report,null,2));
  }
};
