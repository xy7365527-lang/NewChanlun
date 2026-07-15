# #90 静默双核缝合修复 — 实现记录（2026-07-15）

依据裁定：`chanlun/escalate/silent-dual-core-c1-seam-ruling-20260715.md`（C1 方案：seed 显式携带 core 来源，禁止静默缝合）。

## 变更

- `rust/src/theta_v0/classifier/level_view.rs`
  - seed 携带 `SeedCoreProvenance`（`SelfConsistent` / `InheritedRecut`），继承核与自核不一致时不再静默采用继承核，改为显式标注来源并走 fail-closed 判定路径。
- `rust/src/theta_v0/classifier/first_retrace_replay.rs`
  - 回放侧按 provenance 消费 seed，V1/V2 版本元组区分（`EXTENDED_TO_EXACT_THREE_V1` 语义不变，新语义走 V2），不改变既有 V1 判定。
- `rust/src/bin/p83_yield_remeasure.rs`、`p84_failclosed_audit.rs`、`p86_anchor_hypothesis.rs`
  - 适配新 seed API（仅构造/读取端签名变化，无判定语义变化）。

## 验证

- lib 测试：1635 passed / 0 failed（含 level_view seam 用例）。
- `p89_dual_core_audit`（BTC 1m 全量，V2 语义重跑）：
  - `P89_SUMMARY dual_core_total=0 recut_children_total=4589 invalid_total=215 mismatch_normal_total=0 invalid_normal_total=0 verdict_mismatch_all_recut=true verdict_invalid_all_recut=true`
  - 生产域上继承核与自核零分歧 → 全部 `SelfConsistent`；`InheritedRecut` 改写在真实数据上零触发，V2 为语义保证而非行为漂移。
  - 215 例 InvalidSeed 与 #89 审计一致（recut D1 fail-closed 已知域），无新增。

## 结论

静默缝合路径已消除：core 来源显式化，双核分歧不可能再被无声吞掉；真实数据上无行为变化，回归全绿。
