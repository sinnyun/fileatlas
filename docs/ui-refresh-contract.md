# UI 刷新与后台读取统一约定

2026-10-15，按用户要求统一现有实现，保持已接受的界面结构。

## 一个入口，三项职责

`photag-ui-bridge/src/refresh.rs` 的 `poll` 是 GPUI 定时轮询唯一入口。
定时器到点只采样，回调返回实际变化，公共入口每次最多调用一次 `notify`。
实体销毁后弱引用更新失败即结束；有限任务用 `PollResult::until(changed, active)`
在完成或断开时提交最终变化并停止。没有变化的空轮询不触发绘制。

`ChangeGate<T>` 记住上次签名，统一判重。签名可以是修订号、弹层状态、鼠标坐标、
播放位置或它们的组合。播放位置、解码结果、渐隐透明度实际变化时继续绘制，
暂停或动画结束后停止。业务状态变化仍由所属功能定义，不能把“任务正在运行”当作变化。

`read_model::ReadModel<K,V>` 统一被动数据读取。`request(key, loader)` 声明需求，
`poll()` 交付结果；`update(key, loader)` 适用于模型轮询中合并这两步。
`value()` 供界面读取，`key()` 和 `is_pending()` 用于加载状态。
每个模型最多一个工作线程和一个最新排队请求；快速 A→B→C 切换只执行 A 和最终 C。
过期结果不能覆盖当前选择，A→B→A 可以复用仍在执行的 A。
成功、空结果和错误均缓存；worker 异常断开变为终态错误。相同键不会因为重绘重试。

## 使用示例

```rust
// 视图构造时安装一次；不用在 render 内注册定时器。
let mut revision = ChangeGate::default();
refresh::poll(cx, Duration::from_millis(200), move |view, _| {
    let state_changed = revision.changed(view.service.revision());
    let result_changed = view.details.poll();
    (state_changed | result_changed).into()
});

// 在选择事件或非阻塞的需求声明中，键必须包含影响结果的身份/数据版本。
view.details.request((selected_id, data_revision), move || read_details(selected_id));
// render 只投影缓存；不能执行查询/文件 IO，也不能循环 notify。
match view.details.value() { /* loading / ready / failed */ }
```

模型缺失选择时清除该需求/结果，不请求读取。已经选中的文件离开当前加载页后，
选择本身仍然有效，右侧信息按文件 ID 读取；当前可见页不是选中信息的有效性条件。
取消选择后不显示信息；删除或源文件缺失则显示缓存的失败/缺失状态。
用户明确重试、文件/数据修订变化时改变键或重置模型，不能每帧自动重试。

## 已接入的位置

所有原有 GPUI `.timer(...)` 均移至公共入口：Shell、浮动面板、导航与扫描进度、
Gallery、Settings 与更新进度、Preview 与 AI 建议、音视频播放器、元数据导出、
调试面板及开发测试状态。

共用读取模型覆盖 Settings 标签选项、Gallery 统计/批量匹配/EXIF/右侧信息回退/
选择详情/拖拽路径、Preview 文件属性/尺寸/EXIF/索引信息/视频信息/变换显示路径/
标签，以及播放器标签。各功能保留业务键、数据修订来源和结果展示，删除各自的
工作线程回传、pending 判重和过期结果比较模板。

按钮命令、文件操作、写入和事务仍保留原有服务/事件路径；批量执行前重新验证目标。
UI Bridge 是公共呈现工具，不增加插件管理器，不改变 SQLite schema。

## 防止回退

### 结果就绪通知（2026-10-16）

目录导航使用公共 `refresh::Wake/listen`：动作、目录块和 ReadModel 完成时
唤醒原有采样函数，实际变化才 notify。Wake 合并为一个许可，不积累消息，
不新增空闲定时器；每个信号只有一个消费者。视图保存 listener Task，销毁时取消。
`ReadModel::set_wake` 不改变键/修订、在途合并、过期拒绝和失败缓存规则。
既有100/200ms轮询继续兜底；首批和完成结果立即交付，中间昂贵快照至少间隔50ms。
验收必须同时检查响应时间和静止状态，不能仅提高轮询频率换取响应。

`node scripts/check-ui-refresh.cjs` 检查工作区中新增的独立 GPUI 定时器，CI 同步执行。
该检查只约束入口，不证明回调返回值正确。公共模型单测覆盖重复采样、错误缓存、
最新请求合并、过期结果、worker 失败、播放进度与最终通知。
实际行为继续通过 PhoTag 同进程 MCP 成对采样：动作后允许有限动画，静止时
日志序号/数据库读取/预览渲染计数停止增长；真实写入后缓存须更新，正常播放位置须前进。
测试中的真实输入或最小化中断单独记录；有限重试不能把持续循环当作通过。

本次统一后的实际结果：同进程 MCP 两轮136场景，134静止、2实际播放，无持续或阻塞项；
工作区89项测试通过、3项原有忽略。完整原始记录、输入影响、复查和性能计时边界见
[验证记录](validation-ui/2026-10-15-idle-audit/README.md)。
