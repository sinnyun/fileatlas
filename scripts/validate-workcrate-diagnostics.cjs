// WorkCrate resource-diagnostic copy: same-process FileAtlas MCP scenario.
//
// Attaches a real native document whose linked image is absent, and proves the panel
// receives a Chinese, actionable sentence instead of the official report JSON.
//
// node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe \
//   --profile .local/validation/profiles/wc-diagnostics --access-level full \
//   --scenario scripts/validate-workcrate-diagnostics.cjs
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const BROKEN = "F:\\AI\\geren\\pho_cordis\\开发中间会话传递\\新建项目 1.vectorcraft";
const MISSING = "2026-10-21 20-46-04.png";

module.exports = async ({ call, payload, profile }) => {
  assert.ok(fs.existsSync(BROKEN), `diagnostic fixture document must exist: ${BROKEN}`);
  const directory = path.join(profile, "diagnostics-project");
  fs.mkdirSync(directory, { recursive: true });
  await call("workcrate-ui.configure_vectorcraft", {
    executable: "C:\\Program Files\\VectorCraft\\vectorcraft.exe",
    cli_executable: "C:\\Program Files\\VectorCraft\\vectorcraft-cli.exe",
    port: 17979,
  });
  const created = payload(
    await call("workcrate-ui.manage_project", {
      action: "create",
      name: "诊断提示验证",
      directory,
    }),
  );
  const projectId = created.project.id;
  let failure = "";
  try {
    await call("workcrate-ui.attach_document", { project_id: projectId, path: BROKEN });
  } catch (error) {
    failure = String(error.message ?? error);
  }
  assert.ok(failure, "attaching a document with a missing link must fail");
  assert.ok(
    failure.includes(`资源文件缺失：${MISSING}`),
    `the message must name the missing file: ${failure}`,
  );
  assert.ok(failure.includes("关联工程"), `the message must state the repair step: ${failure}`);
  assert.ok(failure.includes("重新校验"), `the repair step must be revalidation: ${failure}`);
  // The headless read root sits in the document's folder, so its report also flags a link
  // that exists elsewhere on disk; that sandbox artifact must not be shown as missing.
  assert.ok(
    !failure.includes("00-main-library.png"),
    `a linked file that exists must not be reported missing: ${failure}`,
  );
  for (const leaked of ["missingLinks", "native resource diagnostics", "\\\\?\\", "ids"]) {
    assert.ok(!failure.includes(leaked), `internal detail leaked into the panel: ${failure}`);
  }
  const state = payload(await call("workcrate-ui.state", {}));
  assert.ok(
    state.projects.some((item) => item.id === projectId),
    "a refused attachment must leave the project in place",
  );
  console.log(JSON.stringify({ projectId, failure }, null, 2));
};
