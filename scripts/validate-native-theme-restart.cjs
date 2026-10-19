const assert=require('node:assert/strict');
module.exports=async({step,delay})=>{
  await delay(1500);const state=(await step({tool:'get_ui_state',capture:'material-restart'})).result;
  assert(state.theme.id.includes('Material #5a8a88'));assert.equal(state.theme.mode,'light');
  const generated=(await step({tool:'material-theme.state'})).result;
  assert.equal(generated.preferences.seed,'#5a8a88');assert.equal(generated.generated,true);
};
