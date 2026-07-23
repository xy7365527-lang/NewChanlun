# #152 终验A：ADR 0001 对齐审计（教义锚）

日期：2026-07-23。审计对象：生产 π loop（`rust/src/theta_v0/strategy/` + `backtest/runner.rs` + `coverage.rs`）对照 `docs/adr/0001-graded-exit-and-shortdiff-doctrine.md`。方法：五条目拆分，5× Opus 只读审计工位独立并行，本体交叉裁决。零代码改动。

## 逐条目判定

| # | 条目 | 判定 | 要点 |
|---|---|---|---|
| 1 | P1–P8 全互斥通道（first-match、C0 兜底穷尽） | **偏离** | channel.rs 解释器本体与 ADR 数学构造严格对齐（first-match 构造 + 2^8 穷举见证 + C0 显式兜底），但**生产 π loop 未消费该解释器**（`channel::` 全部外部引用在 `#[cfg(test)]` 内）；生产走 runner→`pi_theta_step_traced`→`interp::interpret` 的散装 P1..P10 链，行为近似、无结构对应。另：P8 加仓恒 false 占位；生产 Hold 隐式无显式裁决；ADR 内部（S4 vs 优先级条目）与代码通道编号错位一格 |
| 2 | 出场只消费同级别已确认证书 | **对齐** | 单源收敛于 `interp::interpret_with_close_triggers`（interp.rs:1277）：级别精确 u32 键匹配、查无即 fail-closed；`nest_confirmed` 硬门在 close/open 之前拦截归 record（interp.rs:1313–1315）。#146 验收测试在位（coverage.rs:5127/5183/5207–5215）。非证书出场路径（RiskExit、短差 overlay、AncOK 剪枝）各有 ADR 明文授权 |
| 3 | 短差显式机制（四子项） | **部分对齐** | σ_u=−σ_{p(u)}：对齐，多层强制无绕过（voice.rs:53、oscillation.rs:296/683、ledger.rs:896）。父仓不动：语义/记账层对齐，但成交层为净额单出口，冻结不可在 fill 层证实。分腿记账：**偏离**——`SplitLegLedger`（P_sep）完整实现但未接线生产，只活在测试与只读 overlay。毛暴露递归：单层在生产强制（quota ≤ parent.qty，#151/744da5ff81）；多级递归收缩（孙⊆子⊆父 child_view）仅在未接线账本 |
| 4 | AncOK 子树清仓（活动集层） | **对齐（有保留）** | 不变量 `A_{t+1}=AncOK[(A_t∖D_t)∪O_t]` 在生产两条路径均兑现（coverage.rs:2290 `ancestor_close_by_id` 全链判据；runner.rs:3320 `cascade_exit_decisions` 最深优先），短差腿无豁免。保留：#148 T4 专用 `subtree_close`/`step_active_set_with_subtree_close` 零生产调用；restore 注入使 raw 集比公式字面扩集（保守方向，有 no-patch 论证）；预窗持仓腿静默剪除不入 typed 记录 |
| 5 | typed exit 单源五枚举无镜像 | **条件对齐** | 单源 `ExitType`（interp.rs:336）恰五枚举逐字合 ADR S2；`reverse_exit_type` 无方向分支；生产调用点全走单源（coverage→runner typed ledger）。债务：`closed_loop/sell.rs::SellDecision` G4 落地后已废未删；`econ_positive.rs::ExitDecision` 序列化标签与 ExitType 碰撞（标签层双源） |

## 交叉裁决（本体）

1. **系统性缺口（CONFIRMED，高严重度）：教义模块层与生产执行层脱节。** 三个工位独立收敛到同一结构事实——channel.rs P1–P8 解释器（条目1 发现4.1）、`SplitLegLedger` 分账本（条目3 子项3）、#148 T4 `subtree_close`（条目4 D1）均为"带完整证明/否证测试的教义忠实实现，但零生产调用"。生产 π loop 以散装等价物（interp fold + oscillation 单层 + ancestor_close_by_id）近似兑现教义语义。这不是行为 bug——条目2/4 证实关键不变量在生产路径成立——而是**结构对齐债**：教义锚的"结构对应"声明在生产层不成立。
2. **行为层结论：** 同级别证书门、方向约束、AncOK 不变量、typed exit 单源均在生产路径行为成立且有测试守护。未发现任何前视、抢跑、静默丢弃、跨级消费的行为性违反。
3. **ADR 文本债（两名审计员独立命中）：** 二类反向归 CloseRoot 是 ADR 未裁处的实现读法（条目1 发现4.2 = 条目5 发现2a）；S4 与优先级条目的 P4 编号自相矛盾（条目1 发现4.5）。需 ADR 修订裁定，而非改码。
4. **降级项：** 父仓不动在净额执行下不可 fill 层证实（条目3 子项2）——这正是 ADR 行34 引多空对冲.pdf 定理1 批判的净额视角；当前以加法抵消补丁维持，脆弱但成立。

## 总判定

**行为对齐、结构偏离。** 生产 π loop 在可观测行为上未违反 ADR 0001 五条目的任何硬约束；但"教义→代码结构一一对应"在三处断裂（P1–P8 解释器、P_sep 分账本、T4 子树清仓均未接线）。教义锚成立的有效域：**行为层（L1 测试见证）**，不含结构层。

## 下游处置建议（不在本票执行）

- 新票或并入 #139：裁决"接线 vs 让步"——把 channel.rs/SplitLegLedger 接入 `run_theta_v0_pi`，或在 ADR 显式记录"v1 生产以散装等价物实现"的让步条款。
- ADR 修订：补裁二类反向归属；修 S4/优先级条目 P4 编号错位；补记短差平仓插槽。
- 小清理票：删 `SellDecision` 死路径；统一 `ExitDecision` 序列化标签前缀避免同名桶碰撞。
- #148 验收口径需人工裁决：T4 函数要求接线还是仅要求不变量成立。

## 结果包要素

结论=如上；定义依据=ADR 0001 全文 + 五工位报告（file:line 证据内嵌于各条目）；边界条件=若发现 `channel::`/`SplitLegLedger` 存在非限定路径的生产引用（grep 置信 ~0.9），结构偏离结论翻转；下游推论=见处置建议；影响声明=只读审计，零代码改动，本报告与 roster 为唯一新增文件。
