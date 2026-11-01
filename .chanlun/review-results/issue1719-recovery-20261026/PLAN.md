# #1719 精确 B 绑定与第二层清账组件计划

本文件为绑定 #1719 的活期工作草稿。本轮只完成 42R/42W 指定的组件切片，不能代替整票三端点验收。

工位为当前仓库，分支 `sandcastle/issue-1719`，继承 HEAD 为 `7b5cde506be906fd2cef5f3c0de28e0752e45e18`。按用户命令首先无 index 重放 checkpoint26/wip.patch，并复制旧 REPORT 作为起点。旧 WIP 的四文件语义缺陷由本轮接续修复。未提交、未写 Git metadata、未触碰其他工位。

## 写区和实现

1. `nest_lifecycle/pan.rs` 增加带具名政策的伴随输出。两入口共用原 active provider 判据，在 `centers[center_index]` 仍在手时同步产生完整 CenterId、原 LifecycleKey、原注册 bar、原政策。旧 provider 的签名和 PanLiveWindow 不变。
2. `center_break.rs` 从真实 `CenterLifecycleEvent::Broken` 提取完整原件，独立保留物理 bar 和 known-at。Reset、Superseded、Born 无法经此转换成破坏事实。外部调用方须提供原输入的物理 bar 映射，不能默认用确认源坐标或消费钟代替。
3. 原 book 迁移点同步移动绑定。普通桥保留原 B，CenterUpgrade 在真实新 B 绑定到达前不清账；随后只更新当前 B，原对象、原 B、原注册钟和原政策不变。终态前身的新身份认领独立建绑定。
4. 清账核遍历全部合法目标，限定原政策 center_break、完整 B/level 精确对应、从未可证且仍 Provisional。应用键包含完整事实身份、级别、原对象和原版本。独立审计保存原件、原绑定、结算键和消费钟。缺账返回空，不调用缺账 expect。
5. 事实时钟遵守 `registered_at <= physical_bar <= known_at <= consumed_at`，对象进度不能越过 known-at。结算钟取 known-at，消费钟单列。已收到的有效目标事实冻结原钟和原件，迟到拒绝后不得改 known-at 再放行。
6. 新测试均为明确声明的合成结构夹具，经真实 active provider、CenterEventMachine 和原账本运行；未把它们写成正式 S 已发布输入。

## 验证与交棒

本机不运行 cargo/rustc/rustfmt。已做补丁空白检查、tree-sitter Rust 语法检查和旧 feed/provider 函数逐字节比较，均不能替代类型检查或测试。

宿主受管封包后，固定新提交 SHA，经 nc-remote 运行 REPORT 列出的 check、14 项 `i4_bridge_` 测试及旧 channel_switch 回归。核验日志 SHA、退出码和非零测试数。格式化也留给宿主远端，格式化有差异须带回当前工位并重新封包后验证。非作者独审由宿主对最终精确 head 另派。

## 明确未交付

正式 S producer/kind/解码及对象引用闭包、B 原子持久与新进程恢复、正式 launcher 回放/paper 同核消费均未交付。不修改公开 event/checkpoint schema、生产 launcher、finance 或 S producer，不把 p123、Clone 或合成 fixture 升格为上述验收。整票 #1719 不关票，未取得整票 PASS。
