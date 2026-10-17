# 回退后再次保存：官方 API 与本项目适配

## 官方资料

- [Lore 官方 shared-store 工作流](https://epicgames.github.io/lore/how-to/run-parallel-instances-over-a-shared-store/)：分支 tip 前移会使基于旧状态的 commit 被拒绝；恢复步骤为 unstage、sync、stage、commit，unstage 保留工作区内容。
- [Lore GUI Operation reference](https://www.loregui.com/docs/op-reference)：`file.unstage(paths)` 移除暂存；`revision.sync(revision, forwardChanges, reset)` 同步指定 revision；空 revision 表示 latest；`file.stage` 与 `revision.commit` 分别暂存和提交。
- 固定依赖 `BiloxiStudios/loregui@35eaf6e` 的 `crates/lore-vm/tests/storage_shared_store_sync.rs::concurrent_branch_advance_is_rejected_until_sync` 覆盖相同错误和官方恢复顺序。

## 复现与原因

本轮在修改生产实现前，增加 `01 → 02 → 切回01 → 修改 → 创建03` 服务回归，
实际得到完全相同的 `Branch has been advanced by another instance` 错误。
因此本例不需要真的存在另一个进程：历史 checkout 的 revision 为01，而 shared
store 中分支 tip 仍为02，commit 的前置状态不匹配。

参考项目 `newworkspace` 的 `create_version` 也直接 stage/commit，没有这个旧
checkout 的预处理。参考项目的已有显示/切换功能不能作为该新增流程已经覆盖的证据。

## 固定 Rust API 的调用顺序

| 顺序 | API | 作用 |
| --- | --- | --- |
| 1 | `repository::status`、`branch::list` | 比较 checkout revision 和当前分支本地 `latest`；忽略新仓库的全零 tip |
| 2 | `file::unstage(FileUnstageArgs { paths: [private_content] })` | 只在旧 checkout 遗留暂存时移除服务自身的失败快照；拒绝冲突和非服务内容 |
| 3 | `revision::sync(RevisionSyncArgs { revision: local_latest, reset: true, .. })`、`repository::flush` | 同步私有快照基线；原路径源文件不受此 reset 影响 |
| 4 | 把当前源文件复制到私有 content；`file::stage` | 将用户编辑后的完整内容暂存到最新分支基线 |
| 5 | `revision::commit`、`repository::flush`、`VersionInfoService::save` | 保存03及显示元数据，保留01/02 |

离线实测中，空 `RevisionSyncArgs.revision` 会返回 `No remote configured`，
所以这里明确传入 `branch.list` 返回的非零本地 tip；不照搬在线 CLI 的空 sync。

单文件版本列表采用按创建顺序追加快照的语义：03 的内容基于用户对01的编辑，
Lore 父 revision 是已有分支 tip02，01/02 的内容保持可切换。没有自动创建新分支。
干净且未编辑的历史源文件不会因基线同步而被重复提交成新版本。

自动化回归还人为留下旧流程 commit 失败后的持久化 staged content，并验证下一次
公开 `create_version` 能恢复、保存04；同时检查原始编辑字节、既有版本和父 revision。

实际源文件 MCP 验证通过：原路径 `00.png` 上先完成01/02往返，再停01编辑创建03；
三份版本都恢复出各自字节，历史页保持可见，quiet 查询代次22→22。源文件已恢复
开始时字节和时间戳。见 [运行报告](edit-after-rewind.json) 与
[恢复校验](edit-after-rewind-restoration.json)。
