const assert = require('node:assert/strict');
module.exports = async ({ call, payload, delay, step, profile }) => {
  const invoke = async (name, args={}) => payload(await call(name,args));
  await delay(2500);
  const board = await invoke('whiteboard.create',{title:'面板验证画板'});
  await delay(600);
  const ui = await invoke('get_ui_state');
  const panel = ui.panel_instances.find(item=>item.id.startsWith('whiteboard/panel@'));
  assert(panel,'canvas panel mounted');
  const capture = async (name, side, drawer) => {
    await invoke('whiteboard.panel',{side,panel:drawer});
    await delay(700);
    await step({tool:'screenshot_window',capture:name});
  };
  await step({tool:'screenshot_window',capture:'canvas-default'});
  await capture('canvas-boards','left','boards');
  await capture('canvas-resources','left','resources');
  await capture('canvas-layers','right','layers');
  await capture('canvas-object','right','object');
  await invoke('whiteboard.panel',{side:'left',panel:'boards'});
  await delay(700);
  await invoke('whiteboard.panel',{side:'right',panel:'object'});
  await delay(700);
  await step({tool:'screenshot_window',capture:'canvas-both-drawers'});
  await invoke('whiteboard.panel',{side:'left',panel:'close'});
  await invoke('whiteboard.panel',{side:'right',panel:'close'});
  await delay(250);
  console.log(JSON.stringify({passed:true,board_id:board.id,panel,profile,captures:['canvas-default','canvas-boards','canvas-resources','canvas-layers','canvas-object','canvas-both-drawers']}));
};
