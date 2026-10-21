// Compatibility entry point. Current WorkCrate validation is offline-first;
// the previous always-connected desktop protocol scenario is superseded.
module.exports = require('./validate-workcrate-offline.cjs');
