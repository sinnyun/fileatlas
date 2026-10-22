# 2026-10-22 WorkCrate 物料栏分组、内容组与仅面板的尺寸标记

## 验证目标

落实三项要求：

1. 所有页面的物料栏进入一个主组（锁定），各画布的物料栏为独立子组；
2. 画布中添加的文本/图片/链接图片进入该页自己的内容主组，与物料栏分开；
3. 页面信息（页面名称、尺寸标记）只是面板内容，不上画布——画布上的物料条
   不再画"规格"行，尺寸标记改为面板数据持久化。

## 官方接口事实（先探测后使用）

- `vectorcraft-cli.exe run` 探测（`.local/validation/probe-lock*.out`）：
  - `object.setProps {id, locked:true}` 生效且**随 `.vectorcraft` 保存/重开持久化**
    （probe-lock11/12：保存后重开 `locked:true`）。
  - 锁定父组**不阻塞**官方 headless 命令改子节点：`text.setText`、`text.setStyle`、
    `edit.clear` 全部成功（probe-lock9/10、probe-lock3/5：node 3 已被删除）。
  - `node.move {id, parent}` 可把节点移入组（index 0 = 底层；组内排序确定）。
  - `object.group` 单子节点可成组、可跨画板；`file.place` 返回 `ids`。
  - `node.move` 与成组会取消全部选中，需要 `select.set {ids}` 恢复（UI 对象面板依赖）。

## 实现要点

- `native.rs`：`OwnedNodes` 增加 `page_group`/`content_group`；`extract_owned` 返回
  `OwnedLayout { boards, header_root }`（主组不带 board id）；`tag_node`/`group_content`
  共用；`catalog` 不再读 size_label。
- `header.rs`：撤销组内先解锁主组 → 排定/建子组（`node.move` 显式索引替代
  sendToBack/bringToFront）→ 子组入主组 → 最后重新锁定主组。
- `editing.rs`/`operations.rs`：`canvas_create_text`、`canvas_place`、`place_at`
  创建内容后 `group_content` 归组并恢复选中。
- 缓存：`refresh_cache_from_client` 与 autosync `rebuild_cache` 用
  `merge_panel_fields` 按页面 id 从旧缓存回填 size_label。

## 同进程 MCP 实测（profile `wc-groups4`，scenario `.local/validation/validate-header-groups.cjs`）

结果 `profiles/wc-groups4/header-groups-result.json` + 截图 `screenshots/header-groups.png`：

- 字段保存往返：`size_label "500 × 100cm"` 经 `update_page` 保存后在目录重建后仍在
  （`sizeLabelSurvivesRebuild`）；
- 画布建文字：对象面板报告 `kind:"type"`（归组+恢复选中后仍选中）；
- 拖入图片：对象面板 `linked:true`（源在文档夹外，会话按设计重启后依旧成立）；
- 新增 A3 页面 + 第二页字段保存成功；
- 结构断言（官方 CLI 读回已保存文档，见 profile `wc-groups3/structure.json` 同构）：
  - 恰 1 个 `header_root` 组，`locked:true`；
  - 恰 2 个 `header_page` 子组（每页一个），其下含 band/chip/全部文字；
  - 恰 1 个 `content_root` 组，内含新建文字与链接图片；
  - 文档中无 `size_label` / `label:size_label` 节点（规格行不再上画布）。

运行 1–3 的失败均为场景断言问题（旧会话 revision 比较、kind 大小写），实现本身
自运行 1 起结构即正确（`profiles/wc-groups/structure-check.json`）。

## 静态检查

- `cargo +1.97.1 fmt`；`cargo test -p fileatlas-plugin-workcrate-boards --lib`：34 通过；
- `cargo check --workspace --locked` 通过；
- 治理检查：ui-theme / ui-refresh / architecture (edges=124) / feature-ledger 全部通过。

## 范围与遗留

- 旧文档的平铺物料栏与已画的规格行在下次字段保存时自动迁移/清除（过期字段清理）；
  未做一次性迁移脚本。
- 用户此前自行添加到画布的内容不会被追溯归组；仅新创建的内容入内容组。
- 主组锁定在 VectorCraft GUI 中阻断选中/拖动物料条；headless 写入不受影响（实测）。
- 本机残留一个非本轮启动的 `vectorcraft.exe --control 17979`（默认端口、连接式启动
  参数），非本场景产物，未处置。
