// Same-process verification of the debug drawer's repaint-noise display filter.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
module.exports = async ({call,payload,delay,profile,output,exe}) => {
  const report = {executable:exe, started:new Date().toISOString(),
    sha256:require('node:crypto').createHash('sha256').update(fs.readFileSync(exe)).digest('hex')};
  const invoke = async (tool,args={}) => payload(await call(tool,args));
  const routine = r => ['DEBUG','TRACE'].includes(r.level) && r.target==='gpui::window' && r.message.includes('Sending a11y tree update:');
  const save = () => fs.writeFileSync(path.join(profile,'hover-report.json'),JSON.stringify(report,null,2));
  try {
    await delay(1500);
    report.accessibility = await invoke('control_ui',{action:'accessibility_activate'});
    assert.equal(report.accessibility.provider_response,true);
    await invoke('control_debug',{action:'open'}); await delay(500);
    const before = await invoke('get_debug_info',{page_size:200});
    for (const value of ['740,58','790,58','740,58','790,58','740,58','790,58','1,1']) {
      await invoke('control_ui',{action:'pointer_move',value}); await delay(150);
    }
    await delay(1000);
    report.raw = await invoke('get_debug_info',{page_size:200});
    report.displayed = await invoke('get_debug_info',{page_size:200,hide_ui_refresh:true});
    report.hover_records = report.raw.logs.records.filter(r=>r.id>before.logs.latest_id && routine(r));
    assert(report.hover_records.length>0,'actual GPUI hover must produce repaint telemetry for the test');
    assert(report.displayed.ui.filter.hide_ui_refresh);
    assert(report.displayed.logs.records.every(r=>!routine(r)));
    await invoke('control_ui',{action:'statistics'}); await delay(1500);
    report.operations = await invoke('get_debug_info',{page_size:200,hide_ui_refresh:true});
    assert(report.operations.logs.records.some(r=>r.category==='database'));
    assert(report.operations.logs.records.every(r=>!routine(r)));
    report.search = await invoke('get_debug_info',{search:'Sending a11y tree update:',hide_ui_refresh:true});
    assert.equal(report.search.logs.total,0);
    report.page = await invoke('get_debug_info',{page:1,page_size:2,hide_ui_refresh:true});
    assert.equal(report.page.logs.records.length,2);
    assert(report.page.logs.records.every(r=>!routine(r)));
    const shot = await call('screenshot_window');
    fs.writeFileSync(path.join(output,'hover-filter.png'),Buffer.from(shot.content.find(c=>c.type==='image').data,'base64'));
    report.result='passed';
  } finally {report.finished=new Date().toISOString();save();}
};
