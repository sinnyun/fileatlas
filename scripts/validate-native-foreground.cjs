const assert = require("node:assert/strict");
module.exports = async ({step,delay,profile,processId}) => {
 const run=async(tool,args={},extra={})=>(await step({tool,arguments:args,...extra})).result;
 const ui=(action,value)=>run("control_ui",{action,...(value===undefined?{}:{value})},{wait_ms:300});
 await step({tool:"get_ui_state",until:{field:"main_window_status.valid",equals:true,attempts:60,interval_ms:300}});
 await ui("window_select","main"); await ui("window_minimize");
 for(const [owner,command] of [["fileatlas.clipboard","clipboard-workbench/open"],["fileatlas.launcher","program-launcher/open"],["fileatlas.screenshot","screenshot-tool/open-manager"]]) {
   await ui("plugin_launch_mode",`${owner}:panel`);
   await run("command_run",{command_id:command},{wait_ms:650});
   let s=await run("get_ui_state");const panel=s.native_panels[0];
   assert.equal(s.native_panels.length,1);assert.equal(s.main_window_status.minimized,true);
   assert.equal(panel.status.active,true,`${owner}: newly opened native window must be foreground`);
   await ui("window_select",panel.id);
   await run("get_ui_state",{}, {capture:owner.split(".")[1]+"-foreground"});
   await ui("window_minimize");
   await run("command_run",{command_id:command},{wait_ms:650});
   s=await run("get_ui_state");assert.equal(s.native_panels[0].id,panel.id);
   assert.equal(s.native_panels[0].status.minimized,false);assert.equal(s.native_panels[0].status.active,true);
   assert.equal(s.main_window_status.minimized,true);
   await delay(1800);const quiet=await run("get_ui_state"); await delay(1200);s=await run("get_ui_state");
   assert.equal(s.native_panels[0].render_count,quiet.native_panels[0].render_count);
   await ui("window_close");
 }
 require("node:fs").writeFileSync(require("node:path").join(profile,"foreground-summary.json"),JSON.stringify({passed:true,same_process:true,processId,tools:3,new_and_reopen_foreground:true,main_stayed_minimized:true,quiet:true},null,2));
};
