const assert = require("node:assert/strict");
module.exports = async ({step,delay,profile,processId,exe}) => {
 const run=async(tool,args={},extra={})=>(await step({tool,arguments:args,...extra})).result;
 const ui=(action,value)=>run("control_ui",{action,...(value===undefined?{}:{value})},{wait_ms:350});
 const state=()=>run("get_ui_state");
 const command=id=>run("command_run",{command_id:id},{wait_ms:600});
 await step({tool:"get_ui_state",until:{field:"main_window_status.valid",equals:true,attempts:60,interval_ms:400}});
 await ui("window_select","main");
 await ui("settings_open"); await ui("settings_plugin_page","photag.clipboard");
 await ui("plugin_launch_mode","photag.clipboard:panel");
 await run("get_ui_state",{}, {capture:"settings-native-mode"});
 await ui("settings_close");
 await ui("window_minimize");
 await command("clipboard-workbench/open");
 let s=await state(); assert.equal(s.main_window_status.minimized,true); assert.equal(s.native_panels.length,1);
 const clip=s.native_panels[0]; assert.equal(clip.status.owned,true); assert.equal(clip.status.visible,true);
 await ui("window_select",clip.id);
 await run("get_ui_state",{}, {capture:"clipboard-native"});
 await ui("window_minimize");
 await command("clipboard-workbench/open");
 assert.equal((await state()).native_panels[0].status.minimized,false);
 // Real same-process pointer input: toggle the clipboard capture button twice.
 const initial=(await run("get_clipboard_state")).capture_paused;
 const click=async(x,y)=>{await ui("pointer_down",`${x},${y}`);await ui("pointer_up",`${x},${y}`);};
 await click(200,692);
 assert.equal((await run("get_clipboard_state")).capture_paused,!initial);
 await click(200,692);
 assert.equal((await run("get_clipboard_state")).capture_paused,initial);
 await run("resize_window",{width:1450,height:1050},{wait_ms:350});
 await run("get_ui_state",{}, {capture:"clipboard-compact"});

 await command("clipboard-workbench/open");
 s=await state(); assert.equal(s.native_panels.length,1); assert.equal(s.native_panels[0].id,clip.id); assert.equal(s.main_window_status.minimized,true);
 await delay(1500); const quiet=await state(); await delay(1500); s=await state();
 assert.equal(s.native_panels[0].render_count,quiet.native_panels[0].render_count,"native window repaints while idle");
 await ui("plugin_launch_mode","photag.launcher:panel");
 await command("program-launcher/open");
 s=await state(); assert.equal(s.native_panels.length,2); assert.equal(s.main_window_status.minimized,true);
 const launcher=s.native_panels.find(p=>p.id!==clip.id);
 await ui("window_select",launcher.id); await run("get_ui_state",{}, {capture:"launcher-native"});
 await ui("key_press","escape");
 s=await state(); assert.equal(s.native_panels.length,1); assert.equal(s.main_window_status.minimized,true);
 await ui("plugin_launch_mode","photag.screenshot:panel");
 await command("screenshot-tool/open-manager");
 s=await state(); assert.equal(s.native_panels.length,2);
 const screenshot=s.native_panels.find(p=>p.id!==clip.id);
 await ui("window_select",screenshot.id);
 await run("get_ui_state",{}, {capture:"screenshot-native"});
 await click(444,266);
 s=await state(); assert.equal(s.native_panels.length,1);
 await command("screenshot-tool/open-manager");
 s=await state(); const reopened=s.native_panels.find(p=>p.id!==clip.id);
 await ui("window_select",reopened.id);
 await ui("window_close");
 s=await state(); assert.equal(s.native_panels.length,1); assert.equal(s.main_window_status.minimized,true);
 await ui("window_select",clip.id);
 await ui("plugin_enabled","photag.clipboard:off"); await delay(500);
 s=await state(); assert.equal(s.native_panels.length,0); assert.equal(s.main_window_status.minimized,true);
 await ui("plugin_enabled","photag.clipboard:on");
 await ui("plugin_launch_mode","photag.clipboard:workspace");
 await command("clipboard-workbench/open");
 s=await state(); assert.equal(s.native_panels.length,0); assert.equal(s.main_window_status.minimized,false);
 await ui("window_select","main"); await run("get_ui_state",{}, {capture:"workspace-restored"});
 await ui("plugin_launch_mode","photag.clipboard:panel"); await command("clipboard-workbench/open");
 s=await state(); assert.equal(s.native_panels.length,1);
 await run("get_ui_state",{}, {capture:"main-content-retained"});
 await ui("window_select",s.native_panels[0].id); await ui("key_press","escape");
 assert.equal((await state()).native_panels.length,0);
 require("node:fs").writeFileSync(require("node:path").join(profile,"native-panels-summary.json"),JSON.stringify({
   passed:true,same_process:true,processId,executable:exe,profile,
   checked:["main-minimized-preserved","native-reuse-and-restore","two-independent-tools","real-pointer-pause-resume",
     "compact-window","idle-root-render-count","escape-close","screenshot-body-close","native-WM_CLOSE",
     "cordis-disable-cleanup","reenable","workspace-mode-restore","main-content-retained"]
 },null,2));
};
