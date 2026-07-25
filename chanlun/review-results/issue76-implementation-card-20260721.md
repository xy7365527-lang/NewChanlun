# #76 实装卡：出场门真链切换（切口 B）

- 日期：2026-07-21
- 来源：issue #76（SPEC #73 A 线第三票）；N3 前置探查报告（agent-70，切口 B 节）；goal 判据③「进出场门消费真链替代 v0 基例」。
- 前置：#75 落地 + #78 完成（同 crate 串行序：#75 → #78 → #76）。**#75 的身份桥结论与索引生命周期设计是本卡的输入，开工前先读 #75 交付报告。**
- 纪律：禁 git mutation；禁改 Cargo.toml；TDD 先行；门关分支逐字节不变是回归命门。

## 现状（探查实锚，行号以 worktree 为准）

- 出场反向项消费点：`rust/src/theta_v0/strategy/exit.rs:213-220`——`exit_decision_impl(.., nest_cert_gate)` 分支：`reverse_signal(hv.side, &d.bsp) && (!nest_cert_gate || reverse_nest_cert_base(d))`。
- v0 基例本体：`exit.rs:181-184` `reverse_nest_cert_base(d)` = `interp::nest_confirm(d.level, d.signal_index, &d.bsp, dir)`——只消费 `d.bsp` 的 conf_plus/conf_minus 单元素链过 chi_bool（`interp.rs:369-395`，自述「Classification 不导出塔」只产基例）。
- 调用点：`runner.rs:3532-3538`（v1 退出循环，`plan_and_fill_mtm` :3353）与 `runner.rs:3921-3927`（dual，`plan_and_fill_mtm_dual` :3703）。
- 单一来源纪律：`exit.rs:170-173`（进出场同机同谓词，禁 fork）。

## 增补（#75 交付后的实锚，2026-07-21）

#75 已落地，出场侧直接复用其机器（禁另起炉灶）：

- **身份桥（已确证）**：候选 `(level, source_index, side)` → `NestEventIdentity`：级别桥 ℓ = lvl+1（T1 移位）；值桥 `source_index == seg_c_full.1`（**不是** turn_source——t* 收束后 turn_source ≤ seg_c_full.1，等值查必漏，#75 单测显式断言证否）。
- **复用对象**：`NestChainGate`（runner.rs:1137）的增量事件账本 + 索引；反查 = `has_bridge_key`（:1381）/`typed_lookup`（:1397）；判定 = `cert.certificate().n_delta()`（nest.rs 递归核）。
- **出场侧接法**：π 层已持有 per-bar `NestChainGate`（消费点接线 :2162-2180）——方案 (b) 预注入从「新建索引」简化为「把 gate 的 typed_lookup 查询能力注入退出循环」，反向决策 d 经同一身份桥反查。nest-None 时出场侧口径 = 不准出（Xzd 回退是进场侧语义，出场反向项没有 Xzd 对应物——#75 admit 的 Xzd 回退只服务进场）。
- **对照读出**：v0 基例 `reverse_nest_cert_base` 保留为出场侧对照读出（双读落账），判定只用真链——与 #94 的 cross 记账修正同口径（复用通道单独记账，剔除自证成分）。

## 关键障碍（探查 §4 切口 B）

`plan_and_fill_mtm`/`_dual` 作用域内**没有 classification/tower**（签名 runner.rs:3353-3358 只有 decisions+bars）。真链判定需要 bar i 因果前缀的证书索引（#74 构建器产物）。两候选：

- **(a) 穿参**：把 `#75 在 π 层维护的 per-bar 证书索引`（或其只读句柄）穿入 plan_and_fill_mtm/_dual → exit_decision_impl。改动面 = 两循环签名 + 两调用点 + exit.rs。
- **(b) 预注入**：π 层每 bar 预算「反向证书准入集」（side+level+source_index → bool/证书句柄），作为参数注入退出循环；exit_decision_impl 只查集。

**倾向 (b)**：退出循环签名语义污染最小（注入一个查询闭包/表，而非整个塔），且与进场门共用同一索引实例——**同一装配源是硬要求（禁第二查法）**。(a) 仅在 (b) 证明不可行时退用，偏离须说明理由。

## 实装要点

1. 反向查询键：反向决策 d 的 `(level, signal_index, side)` → 经 #75 确证的身份桥 → `NestEventIdentity` 反查索引。**复用 #75 的桥，不得另写。**
2. 准出谓词：证书存在 ∧ `n_delta` 真链判定（nest.rs 递归核），与进场同一谓词方向相反。
3. v0 基例保留为对照读出（双读落账进 NEST_GATE_STATS/退出统计），判定只用真链。
4. `nest_cert_gate=false` 分支短路（exit.rs:217）一行不动；门关全路径逐字节不变。
5. judge_at/divergence_confirmed 一个 bit 不动；nest.rs 装配核不动。

## 测试（先行）

- 门关短路逐字节不变（既有回归锁过）。
- 门开反向真链准出案例：有真链证书 → 准出；无证书/链断 → 不准出（v0 基例会放行的对照案例优先）。
- v1/dual 两退出循环各自覆盖。
- 身份桥反查 miss → 诚实不准出（不得 fallback 到 v0 基例判定——只进对照读出）。

## 验收线

- 全量 `cargo test --release --lib` 零变红（基线 = #78 落地后读数）。
- NEST_GATE_STATS 含出场侧真链拒绝率 + 链深构成 + v0 对照差。
- 门开冒烟（截断）出场侧读数如实入交付报告。
