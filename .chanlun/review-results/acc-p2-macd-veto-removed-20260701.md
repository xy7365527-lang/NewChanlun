---
task: "goal g-20260701T200047Z-8f4f50e7 / acceptance 工位 acc-P2-macd-veto-removed"
verdict: PASSED（验收已满足——a11fcd7d2c 已做 gate→feature，WIP P2-R2 删死 sidecar 改 struct_break_dir）
epistemic_level: L1（管线正确性验证：cargo test 确认候选域严格超集，合成输入；非 L2 真实数据 alpha 验证）
date: 2026-07-01
verifier: acc-P2-macd-veto-removed 工位
---

# 验收：P2 MACD 一票否决消除（外部裁决 Q6 Fix1）

## 结论（PASSED）

P2 第一类买卖点候选域的 MACD 一票否决**已消除**，无残余 veto 分支需删除。

工作分两步已完成：
1. commit `a11fcd7d2c`（feat：MACD gate→feature）——删 `judge_first_cached` 内 `!diverges ⟹ return None` 的 MACD veto，C<A 从候选 gate 降为 buy1/sell1 bit 判据。
2. 工作树 WIP（P2-R2，codex-review-20260701-2251 护栏7）——删除从未接通生产的 `StructBreakFeature` sidecar，改用 `BspPoint.struct_break_dir` 无条件承载破中枢方向；macd_c_lt_a 信息完全可从产出 BspPoint 派生（`struct_break_dir=Some ∧ buy1/sell1` ⟺ 背驰确认；`struct_break_dir=Some ∧ 六 bit 全零` ⟺ 未背驰）。

## 证伪检查（可机器验证，全部 passed）

`cargo test --lib theta_v0::classifier::signal` → **29 passed / 0 failed / 3 ignored**（timing 类）。

关键证伪测试：
- `struct_candidate_domain_strictly_supersets_macd_veto_domain`（signal.rs:973）——同一 C≥A 破中枢输入，结构版 `extract_signals` 产出候选**严格多于** MACD-veto oracle `extract_signals_orig`（`assert!(structural.len() > veto.len())`），且差集恰是带 `struct_break_dir=Some ∧ class_index()==0`（零 bit，未冒充第一类）的被恢复候选。→ 判据「|结构候选域| > |MACD-veto 候选域|」成立。**passed**。
- `diverging_break_sets_buy1_and_struct_break_dir`（signal.rs:940）——背驰确认路径仍严格置 buy1 ∧ struct_break_dir=Some(Long)，class_index 语义冻结。**passed**。

## veto 残留核查（无残留）

signal.rs 现存 `return None`（277/281/289 行）全是**结构性判据**，非 MACD：
- 277：未破最后中枢（几何分量不足）
- 281：无 prev_center 同向离开段（A/C 无法配对）
- 289：段无法映射到 closes 区间（无 MACD 面积坐标系）

生产路径 `mod.rs:248` 调用无 veto 的结构版 `signal::extract_signals`。veto oracle（`judge_first_orig` signal.rs:1504、`extract_signals_orig` signal.rs:1551）均在 `#[cfg(test)] mod tests`（669 行起）内，仅作对照 oracle，不进生产。

## 结果包六要素

1. **结论**：验收 PASSED。MACD 一票否决已从候选 gate 降为 feature/bit 判据；破中枢结构候选（趋势 ∧ 破最后中枢 ∧ A/C 可配对）全部进样本，被 MACD C≥A 判负的候选经 struct_break_dir 恢复方向进样本。29 个 signal 测试全绿含证伪判据。
2. **定义依据**：缠师第17课「MACD 看背驰是辅助非判据」；codex 裁决 Q6 Fix1（MACD gate→feature）；codex-cand-predicate-C1（Cand 是背驰段候选区间存在性谓词，非破中枢/非一买——MACD C<A 只是背驰的工程充分证据非充要）。输入满足「趋势方向 ∧ 破最后中枢几何 ∧ A/C 段可映射 closes」即入候选，MACD C<A 只影响 buy1/sell1 bit。
3. **边界条件**：结论翻转条件——(a) 若生产路径重新引入基于 macd_c_lt_a 的 `return None`（候选预删），veto 复活；(b) 若 `struct_candidate_domain_strictly_supersets_macd_veto_domain` 转红（结构候选域不再严格超集），说明 veto 语义回流；(c) 若 struct_break_dir 停止无条件置（改为仅背驰时置），零 bit 候选丢失方向 = 隐性 veto。
4. **下游推论**：候选域扩大 ⟹ 下游 χ selector 拿到含「破中枢但未背驰」的样本，可学习/否证 MACD 有效性（消选择偏差的目的）。BspPoint/BspBits/Gamma bit-exact 核心零改动（`struct_break_dir` 独立于 class_index），下游 assemble_gamma 不变。
5. **谱系引用**：671 号 pending（P2 R2 MACD c≥a preveto 选择偏差）；codex-decide-20260701-2120/2121（gate→feature 裁决）；codex-review-20260701-2251 护栏7（删死 sidecar）；codex-cand-predicate-C1-20260701（Cand 语义）。本验收未发现新概念分离，无需新谱系。
6. **影响声明**：本工位仅验证，未改代码。验证范围限 signal.rs 候选域（P2 第一类 MACD veto）。**范围外提示**：工作树同一 WIP 含 frontier 修复（mod.rs had_emitted_window，非 signal 路径），导致 `incremental_tower_*` 2 个测试失败（见 code-verify-wip-20260702.md）；按 275/652 局部依赖——MACD veto 候选域不消费 frontier had_emitted_window 输出，两者无数据依赖，frontier 失败不阻塞本 acceptance 判定 PASSED。但整树 commit 需先解决 frontier 失败（不在本工位职责内）。

## 认识论等级说明（formalization-validity-domain 规则）

本验收 **L1**：证伪判据由合成输入（`prices` 手构 C≥A 破中枢场景）驱动，验证的是「候选域生成管线正确性」（结构候选严格超集 veto 候选），**不是** MACD-as-feature 在真实数据上产生可交易 alpha 的 L2/L3 验证。消选择偏差是必要条件（让 χ 有机会否证 MACD），alpha 有效性属下游 W-VERIFY 的 L2/L3 职责，不在本 acceptance 判据内。
