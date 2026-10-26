const assert = require('node:assert/strict');
module.exports = async ({ call, payload, delay, step, profile }) => {
  const invoke = async (name,args={}) => payload(await call(name,args));
  await delay(2500);
  const board = await invoke('whiteboard.create',{title:'画布弹出工具定位验证'});
  await delay(700);
  const state = await invoke('whiteboard.state');
  assert(state.library.boards.some(item=>item.id===board.id),'test board should be active');
  await step({tool:'screenshot_window',capture:'canvas-toolbar-default'});
  const open = async(group,capture) => {
    const result=await invoke('whiteboard.tool_group',{group});
    assert.equal(result.queued,true);
    await delay(500);
    await step({tool:'screenshot_window',capture});
  };
  await open('shapes','canvas-shapes-flyout');
  await open('pages_images','canvas-images-flyout');
  console.log(JSON.stringify({passed:true,board_id:board.id,profile,captures:['canvas-toolbar-default','canvas-shapes-flyout','canvas-images-flyout']}));
};
