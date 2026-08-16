# #786 Q3 挂起勘察：classifier/ 格局——整树切换 vs 增量收口（scoped 架构评审）

- 日期：2026-08-16
- 票据：#786 Q3（#648 处置）挂起期间的 scoped 架构勘察，编排者 2026-08-16 批准（「可以」）
- 方法：`git ls-tree` + 内容级对比 **main @ fe93f34dc7** vs **kimi 封存 tip b242452174** 的 `rust/src/theta_v0/classifier/` 子树；词汇按 codebase-design（模块/接口/深/接缝/局部性）
- 边界：只判不改；未动任何代码

## 发现（事实层）

- **F1 差距核心不在目录，在 mod.rs**：kimi tip `mod.rs` = **157 行薄壳**（纯 re-export + 契约文档）；main `mod.rs` = **6421 行**。
- **F2 main mod.rs 的构成**（实测行号）：管线代码 **~2148 行**（23 个顶层 fn：`classify`/`classify_impl`/`classify_level`/`classify_with_tower*`/`segment_to_unit`/`unit_to_segment`/`detect_centers_*`/`extract_first_third/second_for_level` 等 + `LevelState`/`Classification` 两 struct 及 impl）＋ **内联 `mod tests` ~4273 行**（:2149 起至文件尾）。
- **F3 管线的家**：kimi 侧管线住 `pipeline.rs`（416 行，**kimi 侧活体**）；main 的 `pipeline.rs` 被 C6（#745）删除——删除本身正确（main 树里它是零引用孤儿，main 的活管线在 mod.rs 内联）。
- **F4 外围拆分 main 已齐甚至超出**：两边共有 `level_view/`、`ledger_kernel/`、`retrace_ledger/`；main 独有 `diag/`（C4）、`chain_cert/`、`bsp_bridge/`、`cand_event/`（#634）——**kimi 格局的外围价值，main 已通过有机拆分拿到**。
- **F5 kimi 独有组织 = 同实体的不同摆法**：kimi 的 `incremental/`（5 件）与 `tests/`（5 件）是 kimi 侧活体目录；main 的等价物（`classify_with_tower_incremental` 等增量变体、全部测试）**在 mod.rs 内联**。C7-E1 删的是 main 树里的死壳副本，非 kimi 的活体。
- **F6 kimi tip 内容已陈**：其 `tower_cache.rs` 604 行 vs main C4 新抽取的 571 行（内容不同源）；其 `pipeline.rs` 416 行 vs main 管线已增长（incremental/events 变体、#987 力度链接线）。**整切 = 把 main 三周增量重放进 kimi 的旧文件**，逐件对账，成本与在飞线碰撞风险俱高。

## 架构判读（codebase-design 词汇）

- **接缝**：`classify*` 族签名 = classifier 模块的公共接口（消费方：backtest/strategy 经 re-export）。两边接口面等价。
- **kimi 格局的本质价值** = **薄壳 + 域模块**：实现藏在命名域文件后，模块根只做 re-export——读者读 mod.rs 不需要穿过 6000 行编排代码（**局部性**），域变更集中在域文件（**深度**）。
- **main 现状**：外围域已拆（F4），**核心（管线 + 测试）仍内联在模块根**——恰好是 kimi 格局解决的那一半还没做。

## 结论

**终态同形、路径改增量。** kimi 格局的终态形状（薄壳 mod.rs + 域模块）可以纯移动两步到达，不移植任何陈旧内容：

- **T1**：内联测试 ~4273 行 → `classifier/tests/` 目录（或 `mod_tests.rs` 单件）——即 #715 调查 C1 条，窗口本就挂 #648；
- **T2**：管线 ~2148 行 → `pipeline.rs` **复名**（与 C6 删除记录对账，名分注释写明「此为 mod.rs 内联管线的纯移动抽取，非旧孤儿复活」）。

两步均为 #497/#573/#634/C4 已验证四次的**纯移动零行为**模式（`pub use` re-export 保持接口面不动，消费方零影响）。kimi tip 从「移植源」降级为「对照参照」（核对域归属用）。**#643(b)「拆分格局为正房」的裁定实质不变**——变的是到达路径，不是方向。

## 代价照实

- 纯移动大 diff 与在飞线（classifier 是热区）有 rebase 摩擦——但比整切小一个量级，且每步独立可验证（cargo test 计数不动）；
- `pipeline.rs` 复名须对账 C6 删除记录（上面 T2 已开名分注释要求）；
- T1/T2 后 main 与 kimi tip 的**文件清单仍不完全一致**（kimi 的 `incremental/` 五件组织不照搬——其内容实体 main 以别的组织持有）。若将来要逐文件对齐 kimi 清单，另案。

## 对 #648 的改写建议（待 Q3 续裁拍板）

- 方法段改为 T1/T2 两增量步（起点 = 当前 main，不再从 kimi tip 拉子树）；
- 验收沿用原票：接口 parity（re-export 不动）+ `cargo test --lib` 计数不变 + 红线零语义改动；
- 执行窗口：T1/T2 各自挑并行线着陆间隙，纯移动 diff 降低碰撞面。
