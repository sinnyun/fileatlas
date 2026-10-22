// Isolated compatibility fixture, prepared before the same-process MCP child starts.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..'),base=path.join(root,'.local/validation/profiles');
const profile=path.resolve(process.argv[2]||path.join(base,'wc-portable-legacy-2026-10-23'));
assert.ok(profile.startsWith(base+path.sep));assert.ok(!fs.existsSync(profile),'use a fresh isolated profile');
const source=JSON.parse(fs.readFileSync(path.join(base,'wc-portable-2026-10-23/portable-summary.json')));
const folder=path.join(profile,'documents','legacy');fs.mkdirSync(path.dirname(folder),{recursive:true});
fs.cpSync(source.folder,folder,{recursive:true});
const id=source.project_id,sourceData=path.join(folder,'.workcrate',id),target=path.join(profile,'workcrate','projects',id);
fs.mkdirSync(path.dirname(target),{recursive:true});fs.renameSync(sourceData,target);
const manifest=path.join(target,'project.json'),doc=JSON.parse(fs.readFileSync(manifest)),project=doc.project;
doc.native_document=path.join(folder,doc.native_document);doc.references.forEach(r=>{if(!path.isAbsolute(r.path))r.path=path.join(folder,r.path);});
delete doc.portable;delete doc.project;
fs.writeFileSync(manifest,JSON.stringify(doc,null,2));
fs.writeFileSync(path.join(profile,'workcrate/projects.json'),JSON.stringify({version:1,projects:[project]},null,2));
fs.writeFileSync(path.join(profile,'legacy-fixture.json'),JSON.stringify({folder,id,native:doc.native_document,A:source.A,B:source.B},null,2));
