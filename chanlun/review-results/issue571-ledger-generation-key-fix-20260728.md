# #571 typed ledger generation 键修复实装报告

- **日期**：2026-07-28
- **工位**：`/private/tmp/kimi-nest-mainline`
- **分支 / 开工与收尾 HEAD**：`kimi-nest-mainline-20260717` / `04c360a410`
- **裁定**：同 carrier 覆盖是 bug；主键采用 `(ElementId, generation)`；carrier 关闭时 drain
  全部 live generation，各自结算为真实 `TypedTrade`，不造 supersede 假结算。
- **git 边界**：全程零 git 写操作。未改任何 treasury golden，也未改
  `chanlun/agent-roster*`、`chanlun/review-results/` 下的其他票据或并行线文件。

## 1. 修复面

新增 `rust/src/theta_v0/backtest/open_ledger.rs`，把以下职责从 `fill.rs` 收进单一模块：

1. `OpenTable<LedgerOpen>` 主表以内部 `OpenKey { carrier, generation }` 唯一标识实例；
2. generation 高水位仍按 carrier 单调分配，首次为 0，溢出 fail-loud；
3. `carrier -> BTreeSet<generation>` 确定序二级索引承接 carrier-only 事件；
4. 反向关闭、silent drop、RiskExit、CloseOverlay 均 drain carrier 的全部 live generation，
   每个实例独立构造并落一行 `TypedTrade`；
5. 窗末对所有残留实例落 `Hold`，排序键为
   `(entry_bar, carrier.level, carrier.ordinal, generation)`；
6. ShortDiff/TW 按实例对称开关；PanDiv 父快照聚合同 carrier 的非 ShortDiff units；
7. 风控逐 generation 读取冻结 stop；触发方向保持 carrier/ActiveLeg 的结构方向契约，
   任一实例命中后由 carrier-only RiskExit drain 全部实例。

`fill.rs` 从开工 4211 行降到 4046 行；本票 diff 为 `+37/-202`，净减 165 行，满足“禁止净增”。
新模块 599 行，最大函数 44 行、最大参数数 5；模块自身
`rustfmt --edition 2021 --config skip_children=true --check` 通过，票内 Rust diff
`git diff --check` 通过。

I/O 错误不再静默吞掉：`OPSEM_DUMP_DIR`、`THETA_STRICT_NEST_SIDECAR` 的 NotUnicode 分支
fail-loud；目录/文件创建、trade/tower 写入和 flush 都带路径或 carrier/generation/exit_bar
上下文 panic。索引悬空、重复键、generation 溢出、PanDiv 非有限或溢出 units 同样 fail-loud。

## 2. TDD 与五组守卫

### 2.1 先红

先用 carrier 单槽表写 C1 复现：同 carrier、同 bar 依次开 Long/Short，再关闭。
测试实际只剩 generation 1 Short，期望 generation 0 Long + generation 1 Short，失败：

```text
left:  [TestOpen { generation: 1, side: Short }]
right: [TestOpen { generation: 0, side: Long },
        TestOpen { generation: 1, side: Short }]
```

证据：`/tmp/issue571-tdd-red.log`。

### 2.2 修后五绿

| 守卫 | 断言 |
|---|---|
| 同 bar 异向 | C1 Long/Short 两 generation 各落一条 `TypedTrade` |
| 跨 bar 同向重开 | entry bar 10/20、generation 0/1 均保留 |
| 跨 bar 异向重开 | 两实例均落 `CloseShortDiff` |
| RiskExit 全清 | 三个 live ShortDiff 全落 `RiskExit`，TW `open_legacy_legs` 3→0 |
| 窗末多 generation Hold | 三实例全落 `Hold`，按 entry/carrier/generation 确定序输出 |

`cargo test --lib open_ledger::tests`：5 passed / 0 failed。另复核：

- `typed_ledger_`：3 passed / 0 failed / 1 ignored；
- `opsem_dump_env_gated_bit_exact`：通过；
- `k_theta_risk_gate_reads_frozen_entry_stop_for_drifted_leg`：通过；
- `lee_m3_risk_gate_stays_per_bar_under_event_gating`：1191 快照保持并通过。

## 3. BTC wf8 同 HEAD A/B

### 3.1 口径

前件不是旧 `/tmp/issue71-*` 产物，而是从同一 HEAD `04c360a410` 用 `git archive HEAD`
展开到隔离目录后重跑；后件用本票未提交代码重跑。两边均执行：

```bash
M8_SYMBOL=BTC M8_WIN_FILTER=wf8 \
  cargo test --release --lib \
  theta_v0::backtest::wverify_run::issue71_chi_gamma_validation \
  -- --ignored --nocapture
```

前件：`/tmp/issue571-before-current`；最终后件：`/tmp/issue571-after-final`。
四臂 harness 自带的基线输出一致、OPSEM 隔离一致、Gamma 双跑一致三条硬断言全部通过。

### 3.2 opened ↔ typed trade 双向 join

join 键为 `(entry_bar, level, source_index, dir)`，用 multiset 双向核对。

| 臂 | 修前 opened/trades | 修前 opened 无 trade / trade 无 opened | 修后 opened/trades | 修后双向失配 |
|---|---:|---:|---:|---:|
| Arm0 无 χ | 508 / 499 | 9 / 0 | 508 / 508 | 0 / 0 |
| Arm1 χ teap=false | 5 / 5 | 0 / 0 | 5 / 5 | 0 / 0 |
| Arm2 χ teap=true | 154 / 151 | 3 / 0 | 154 / 154 | 0 / 0 |
| Arm3 χ 隔离 | 2 / 2 | 0 / 0 | 2 / 2 | 0 / 0 |
| **合计** | **669 / 657** | **12 / 0** | **669 / 669** | **0 / 0** |

救回的 12 条为：

- Arm0：bar 2978、3047 Long、3047 Short、78090、99685、149455、150372、152690、209394；
- Arm2：bar 149455、150372、245306。

因此票面“约 +12”在同 HEAD 四臂合计上精确兑现；Arm0 是 +9，Arm2 是 +3。

### 3.3 账户读数与 PnL

| 臂 | n_orders 前→后 | account trades 前→后 | `trade_pnls_with_forced` Σ 前→后 | Δ |
|---|---:|---:|---:|---:|
| Arm0 | 833→840 | 479→479 | 1,173,840.77604132→803,492.78792633 | -370,347.98811499 |
| Arm1 | 303→303 | 299→299 | -11,679,257.06010596→同值 | 0 |
| Arm2 | 301→301 | 165→165 | 2,623,082.67309706→同值 | 0 |
| Arm3 | 12→12 | 9→9 | -52,177.40911600→同值 | 0 |

Arm0 的变化来自训练 typed ledger 补齐后反馈到估计器/后续订单，不是把 9 行 mechanically
加到账户 trade 数；Arm2 的 3 行只修 typed 生命周期，账户轨迹不变。

账户恒等按生产 `RDecomposition` 的独立累计器复核：

| 状态 | ΣΔpnl（`net_r`） | 账户 Δ（`ledger_delta` / equity Δ） | 守恒残差 |
|---|---:|---:|---:|
| 修前 | 1,179,251.258659047075 | 1,179,251.258659337880 | -2.896413207054e-7 |
| 修后 | 808,840.721612043213 | 808,840.721612473950 | -4.288740456104e-7 |

两边均在规模容差内成立。上表不拿 `trade_pnls_with_forced` 直接冒充账户 Δ：
该向量含窗口终点“虚拟强平费”，而末 bar equity 是虚拟强平前 MtM；#570 已明确两者是不同域。

## 4. treasury #385–#389 影响清单

### 4.1 当前 #387/#446 ArmR golden 的实测漂移

另按 #387 ArmR 真实命令在同 HEAD 跑 BTC wf8：

```bash
M8_WIN_FILTER=wf8 VOICE_EXEC=1 THETA_NEST_CERT_GATE=1 \
  OPSEM_DUMP_DIR=<dir> cargo test --release --lib \
  theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos \
  -- --ignored --nocapture
```

| 项 | 修前（命中当前 golden） | 修后 |
|---|---:|---:|
| typed trades | 159 | 160 |
| bytes | 216129 | 217520 |
| FNV-1a64 | `0x18291f8ba8f1d40f` | `0x0c0872d94e5d6630` |
| Long / Short | 87 / 72 | 87 / 73 |
| Σ typed `pnl_raw_unlevered` | 17,271.630000000063 | 17,281.570000000065 |

账户/treasury wf8 表逐位不变：`n_orders=246`、`ΣN_tΔP_t=7,368,379`、
`Comm+Slip=580,653`、`Funding=506,442`、`execR=6,281,284`、`MaxDD=0.0812`、
`R=6,782,644`、`LCB=-2,427,669`、三态 `INCONCLUSIVE`。

结论：本票在 ArmR/wf8 上只修回一条 typed 生命周期行，但这已足以让 byte digest 合法漂移；
现有 golden 不能继续声称“当前代码全中”。

### 4.2 逐票登记

| 票 | 受影响面 | 本票处置 |
|---|---|---|
| #385 母 SPEC | “臂R digest/关键读数作回归锁”中的 digest 前提被有意打破；历史验收报告仍是历史证据，不再证明当前 typed 产物 | 只登记，不改母 SPEC |
| #386 | 仓内未找到可唯一归属 #386 的权威报告或数值 baseline（仅数据 JSON 中偶然数字命中） | 标为待编排者/issue body 补锚；不虚构“零影响” |
| #387 T1 ArmR | 当前 golden 已经 #446 重锚到 141/184/159；wf8 实测 159→160、digest 漂移 | **未改** golden/校验器；p3fold、wf7 尚未重跑，数值影响未知 |
| #388 T2 ArmD | 与 ArmR 共用 typed open/close 生产路径；三窗 trades/digest、费率分解的逐笔集合均属潜在受影响面 | 未重跑，未改报告；须在获准重基线时三窗重验 |
| #389 T3 ArmC/总报告 | 汇总 #387/#388 且以 ArmR golden 为回归门；旧“digest 全中”声明不再适用于当前代码 | 未改报告/帽臂读数；须随专门 rebaseline 一并复核 |

**红线**：`chanlun/review-results/treasury-reverify-t1-armR-trades-golden-20260727.json`
和 `scripts/check_armR_trades_digest.py` 均未改。是否接受此次有因漂移、何时跑三窗 R/D/C 并重锚，
等待编排者明示，禁止本票静默翻基线。

## 5. 指纹

| 阶段 | `cargo test` |
|---|---|
| 开工 | 2025 passed / 1 failed / 136 ignored |
| 收尾 | 2030 passed / 1 failed / 136 ignored |
| 允许 delta | +5，恰为本票五组新守卫 |

前后唯一失败均为
`theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`（#115 在案）；
既有失败集未扩大。最终 `cargo build --all-targets` exit 0。

日志：

- `/tmp/issue571-cargo-test-before.log`
- `/tmp/issue571-cargo-test-after-final.log`
- `/tmp/issue571-cargo-build-all-targets.log`
- `/tmp/issue571-wf8-before-current.log`
- `/tmp/issue571-wf8-after-final.log`
- `/tmp/issue571-treasury-before-wf8.log`
- `/tmp/issue571-treasury-after-final-wf8.log`

## 6. 本票提交文件清单（未提交）

1. `rust/src/theta_v0/backtest/open_ledger.rs`（新增）
2. `rust/src/theta_v0/backtest/fill.rs`
3. `rust/src/theta_v0/backtest/admission.rs`
4. `rust/src/theta_v0/backtest/opsem_dump.rs`
5. `rust/src/theta_v0/backtest/mod.rs`
6. `rust/src/theta_v0/backtest/runner.rs`（**修复轮新增到清单**：风控门守卫迁到生产
   `OpenTable` + 新增混合方向守卫 + `RISK_TRIGGER_SURFACE_GATE_OFF_SNAPSHOT` golden 再生）
7. `chanlun/review-results/issue571-ledger-generation-key-fix-20260728.md`（本报告）

收尾 `git status --short` 另见 `chanlun/agent-roster-2026-07-21.md`、
`chanlun/escalate/chi-line-falsification-ruling-20260728.md`、
`rust/src/theta_v0/classifier/nest_lifecycle.rs` 及多份非本票报告的并行改动；这些均不在本票文件清单，
本工位未触碰、未还原。

---

# 修复轮：两轴评审 FAIL 项修复（2026-07-28，第二执行体）

- **执行体**：Claude Opus 5（前一执行体 codex 因额度死在启动前，零改动）。
- **工位 / 分支**：`/private/tmp/kimi-nest-mainline` / `kimi-nest-mainline-20260717`。
- **开工 HEAD**：`6e0db65a6a`；收尾时主线已被并行工位推进到 `5505eaaed2`（本票未参与）。
- **git 边界**：全程零 git 写操作。未碰 `nest_lifecycle.rs`、`chanlun/agent-roster*`、
  `chanlun/escalate/chi-line-falsification-ruling-20260728.md`、`chanlun/review-results/`
  下的其他票据。全程前台单线程，无子代理、无后台进程。

## 7. Spec HIGH 修复：风控 stop 从「腿方向」迁到「实例方向」

### 7.1 修前的实际语义（前一轮遗留）

前一轮虽然把 `k_theta_risk_gate` 改成经 `LiveOpenView` 枚举 carrier 的全部 live generation，
但**触发方向仍统一取 `ActiveLeg.dir`**：`exit_side` 由 `leg.dir` 定，命中后也按 `leg.dir` 记到
`long_stop`/`short_stop`。这与 #570 探针 §3「风控止损」行的要求（**按实例 side/stop 聚合
long/short stop**）不符——`ActiveLeg.dir` 是 carrier 走势的 eps（结构方向），与 campaign 的持仓
方向无关；同 carrier 并存 Long/Short 两 generation 时，空头实例的止损会被记到多头门上。

### 7.2 修后

`admission.rs::stop_flags`（新拆出的纯判定 helper）：

- 遍历 `prev_active` **只为取 carrier**，不再从 `leg.dir` 派生任何判定；
- 每个 live 实例按**自身** `position_node_id.side` 决定 `exit_side`
  （Long→`FillSide::Sell`、Short→`FillSide::Buy`），读自身冻结的 `entry_stop` 判 `stop_hit`；
- Long/Short 两向分别 OR 聚合（同向多实例任一触及即该方向 stop 成立 = 最紧者定门）；
- `untradable` bar 整体不判（提到循环外，语义与旧 `!bar.untradable` 逐次判等价）；
- 实例 side 为 `Flat` 时防御性 no-op（campaign 身份不构造 Flat），不再是 `unreachable!()`。

`ActiveLeg.dir` 的 `Flat` 分支不再 `continue`——方向判定已完全由实例承担，腿方向不参与。

### 7.3 混合方向守卫（新增测试）

`runner.rs::k_theta_risk_gate_aggregates_mixed_direction_generations_per_instance`：
同一 carrier 上 `open` 两个 generation（Long stop=90 在下方、Short stop=200 在上方），
**活动腿 `dir` 全程固定为 Long**，四象限断言：

| bar (high/low) | 期望 | 实测 |
|---|---|---|
| 110 / 80 | 仅 `stop_long` | ✓ |
| 250 / 100 | 仅 `stop_short`（旧口径会记成 `stop_long`） | ✓ |
| 250 / 80 | 两向同真 | ✓ |
| 110 / 100 | 两向皆假 | ✓ |

第二行是本守卫的判别力所在：若判定仍读 `leg.dir`，该行必红。

## 8. Standards 修复

| 评审项 | 处置 | 证据 |
|---|---|---|
| HIGH `LiveOpenView` 归位 | trait + `impl for OpenTable<LedgerOpen>` 移入 `open_ledger.rs`（在飞表的天然内聚位置）；`admission.rs` 只 `use super::open_ledger::LiveOpenView` | `open_ledger.rs:117-131` |
| HIGH 删测试垫片 | `#[cfg(test)] impl LiveOpenView for HashMap<...>` **删除**；两个消费它的 runner 守卫改用生产 `OpenTable::open` 建表（新建表 helper `open_campaign_for_gate` / `gate_bar` / `gate_leg` 在 runner 测试模块内共用） | `admission.rs` 已无 `HashMap` 垫片；`runner.rs` 守卫全走生产键形 |
| MED `k_theta_risk_gate` 95 行/7 参 | 拆为 `k_theta_risk_gate`（31 行 / **1 参**）+ `resolve_risk_mode`（15 行）+ `stop_flags`（21 行）；7 位置参捆成 `RiskGateCtx`（5 字段，账户三数收进 `RiskAccount`） | 见下表 |
| MED `opsem_dump.rs::diff_tower` 93 行 / 嵌套 >4 | 拆为 `diff_tower`（24 行）+ `emit_initial_centers`（24 行）+ `diff_level`（47 行）；不活跃分支改 early-return 后最大嵌套 4 | 见下表 |
| 行数「零净增」 | `admission.rs` **1858 → 1827**，低于本 diff 前基线 1829（净减 2 行） | `wc -l` |

函数指标实测（花括号深度含函数体自身那一层）：

| 函数 | 行数 | 参数 | 最大嵌套 |
|---|---:|---:|---:|
| `k_theta_risk_gate` | 31 | 1 | 2 |
| `resolve_risk_mode` | 15 | 1 | 3 |
| `stop_flags` | 21 | 1 | 4 |
| `diff_tower` | 24 | 3 | 4 |
| `emit_initial_centers` | 24 | 3 | 4 |
| `diff_level` | 47 | 5 | 3 |

`diff_tower` 拆分的**语义保持**：`prev=None`（首个活跃 bar）与 `prev=Some(空级别)` 两分支
不可合并——前者不产 `level_upgrade`，后者产；拆分后仍分列两支，并在注释里显式登记该约束。
逐字节回归：重构前后同一 wf8 跑批的 `tower_events.jsonl` 与 `trades.jsonl` 均 `cmp` 一致。

`opsem_dump.rs` 的 `rustfmt --check` 差异数从基线 13 降到 10（拆分反而更 fmt-friendly）。

## 9. 验收

### 9.1 编译与测试指纹

| 项 | 结果 |
|---|---|
| `cargo build --all-targets` | exit 0（主工位与隔离树均绿） |
| `cargo test` 全量 | **2031 passed / 1 failed / 136 ignored** |
| 唯一失败 | `extract_signals_bit_exact_digest_guard`（#115 在案，前后一致） |
| 相对上一轮 | 2030→2031，delta **+1 = 本轮新增的混合方向守卫**，既有失败集未扩大 |
| `open_ledger` 5 守卫 + 族A 守卫 + 新混合方向守卫 | 7 passed / 0 failed |

日志：`/tmp/issue571-fix2-cargo-test.log`（隔离树）、
`/tmp/issue571-fix2-cargo-test-mainline.log`（主工位）。

### 9.2 `RISK_TRIGGER_SURFACE_GATE_OFF_SNAPSHOT` golden 再生（按机制走完 1–4 步）

修后 `lee_m3_risk_gate_stays_per_bar_under_event_gating` 变红：门控开态实测 **554** ≠ 历史快照
**1191**。按该测试注释「★golden 再生机制」逐步执行，**未**跳过复算直接改常量：

1. 在隔离树临时把 `plan_level_gated_order` 的 `ticks.ticked_levels()` 换成
   `basis ∪ level_order.planned()` 的级别并集（= 关闭事件门控，退化回 M2 每 bar 重估）；
2. 同 fixture（`RW3000M3`，同种子）重跑，读 `ov_m3.level_order.n_risk_gate_active` = **554**；
3. 撤销临时改动（`diff` 回到与主工位逐字节一致，已核）；
4. 复算 554 == 门控开态实测 554 ⟹ **情形 (a)**：历史快照过期，事件门控**未**污染风控域，
   按步骤 4 把常量更新为 554，并在注释中登记漂移成因。

**成因（照实）**：旧快照 1191 是在 stop 触发方向统一取 `ActiveLeg.dir` 的口径下测得——彼时大量
非持仓方向的结构腿被误判为止损触发并计入触发面。改按实例 side 聚合后，触发面缩到 554。
这是 §7 修复的直接下游读数，不是新引入的缺陷。

### 9.3 BTC wf8 A/B

**口径**：后件 = 当前 HEAD `git archive` 展开的隔离树 + 本票 7 个文件注入
（`/tmp/issue571-fix2-tree`，`nest_lifecycle.rs` 经 `diff` 核实为 HEAD 干净版，
不含并行工位当时的半成品）。三腿硬断言（基线一致 / OPSEM 隔离逐字节 / Gamma 双跑逐字节）全过。

**前件复用声明**：前件沿用上一轮 `/tmp/issue571-before-current`（HEAD `04c360a410`）。
依据：`04c360a410..6e0db65a6a` 对 `rust/` 的唯一改动是 `nest_lifecycle.rs`，其生产区 hunk
（`provide_l1_active_pan_live_windows`）**只改注释、`if` 条件逐字未动**，其余 hunk 全在
`#[cfg(test)] mod tests` 内且与 wverify wf8 路径无关 ⟹ 前件二进制语义等价。这是可核验的等价性
论证，不是「差不多所以复用」。

#### opened ↔ typed trade 双向 multiset join（键 `(entry_bar, level, source_index, dir)`）

| 臂 | 修前 opened/trades | 修前失配 | 上一轮修后 | **本轮修复后 opened/trades** | **本轮双向失配** |
|---|---:|---:|---:|---:|---:|
| Arm0 无 χ | 508 / 499 | 9 / 0 | 508 / 508 | 508 / 508 | 0 / 0 |
| Arm1 χ teap=false | 5 / 5 | 0 / 0 | 5 / 5 | 2 / 2 | 0 / 0 |
| Arm2 χ teap=true | 154 / 151 | 3 / 0 | 154 / 154 | 283 / 283 | 0 / 0 |
| Arm3 χ 隔离 | 2 / 2 | 0 / 0 | 2 / 2 | 2 / 2 | 0 / 0 |
| **合计** | **669 / 657** | **12 / 0** | **669 / 669** | **795 / 795** | **0 / 0** |

**结论：修后 opened↔trade 双向 0/0 仍成立**（本票核心不变量未被风控语义变更破坏）。
脚本：`/tmp/issue571_fix2_join.py`。

#### 四臂读数（与上一轮自报值逐一对照，变化照实）

| 臂 | 上一轮 n_orders / n_trades / Σpnl | **本轮** n_orders / n_trades / Σpnl |
|---|---|---|
| Arm0 无 χ | 840 / 479 / +803,492.78792633 | **1063 / 690 / +6,685,276.06227939** |
| Arm1 χ teap=false | 303 / 299 / −11,679,257.06010596 | **12 / 9 / −52,177.40911600** |
| Arm2 χ teap=true | 301 / 165 / +2,623,082.67309706 | **617 / 393 / −43,149.15596093** |
| Arm3 χ 隔离 | 12 / 9 / −52,177.40911600 | 12 / 9 / −52,177.40911600（不变） |

**变化归因（照实，不粉饰）**：Arm0 的 `opened` 计数 508 逐值不变 ⟹ 入场候选面未动；变的是
**关闭/订单流**——止损触发方向口径改了，`stop_long`/`stop_short` 的真值序列随之改变，
`𝒦_Θ` 收窄时点不同 ⟹ 订单数与轨迹变化。χ 臂（Arm1/Arm2）的变化再叠一层：χ 的 μ 表由
**训练窗 typed ledger** 建，训练窗轨迹变 ⟹ μ 表变 ⟹ 准入面变；本轮 Arm1 恰好退化到与
Arm3 同值（`Arm1−Arm3` 三项全 0），这是 teap=false 下无 LCB 证据即不交易的极端表现，
**不是**两臂被接错——Arm3 是隔离臂，其 opened/trades 与 Arm1 各自独立产出后数值相同。
本轮读数**不作** alpha 论据（口径标签 [L1机制/费率未标定] 沿用）。

产物：`/tmp/issue571-fix2-wf8-after/`；日志 `/tmp/issue571-fix2-wf8-after.log`。

### 9.4 treasury ArmR（`m8_e2e_all_systems_oos`，BTC wf8）

| 项 | 上一轮修前（旧 golden） | 上一轮修后 | **本轮修复后** |
|---|---:|---:|---:|
| typed trades | 159 | 160 | **160** |
| bytes | 216,129 | 217,520 | **221,626** |
| FNV-1a64 | `0x18291f8ba8f1d40f` | `0x0c0872d94e5d6630` | **`0x9c6a202dbea2de77`** |
| Long / Short | 87 / 72 | 87 / 73 | **87 / 73** |
| Σ typed `pnl_raw_unlevered` | 17,271.630000000063 | 17,281.570000000065 | **17,281.570000000065** |

typed 生命周期集合与上一轮修后**逐笔一致**（trades 数、L/S、Σpnl 全等）；字节与 digest 漂移
来自逐笔的三个账户轨迹字段，已做字段级 diff 定位：

| 字段 | 差异笔数 |
|---|---:|
| `interpreter_at_entry.lex_argmin_top3` | 158 |
| `units` | 122 |
| `tw_at_entry.eta_bucket` | 7 |

即：`entry_bar` / `exit_bar` / `exit_type` / `certificate` / `pnl_raw_unlevered` 全部不变，
变的是 sizing 与解释器网格随 NAV 轨迹的漂移。

账户/treasury wf8 表（**上一轮声称「逐位不变」，本轮不再成立，照实登记**）：

| 项 | 上一轮 | **本轮** |
|---|---:|---:|
| n_orders | 246 | 246（不变） |
| ΣN_tΔP_t | 7,368,379 | **7,604,573** |
| Comm+Slip | 580,653 | **594,942** |
| Funding | 506,442 | **518,703** |
| net_r(execR) | 6,281,284 | **6,490,928** |
| MaxDD | 0.0812 | **0.0814** |
| R(含浮盈) | 6,782,644 | **7,004,502** |
| LCB_OOS(R) | −2,427,669 | **−2,302,490** |
| 三态 | INCONCLUSIVE | INCONCLUSIVE（不变） |
| duplicate_id_violations | 0 | 0（不变） |

**红线仍未动**：`treasury-reverify-t1-armR-trades-golden-20260727.json` 与
`scripts/check_armR_trades_digest.py` 本轮同样未改。§4.2 的逐票登记（#385–#389）全部维持，
且**加重**一档：ArmR digest 已在本轮再次漂移（`0x0c08…`→`0x9c6a…`），账户表也不再逐位不变，
何时三窗 R/D/C 重锚仍等编排者明示。

产物：`/tmp/issue571-fix2-treasury-wf8/`、报告 `/tmp/issue571-fix2-treasury-wf8.md`；
重构后逐字节回归产物 `/tmp/issue571-fix2-treasury-wf8-recheck/`。

## 10. 照实登记的遗留

1. **`admission.rs` 非 rustfmt-clean**：基线该文件已有 21 处 `rustfmt --check` 差异（宽行紧凑
   风格），本轮为守住「≤1829 行」硬上限，新代码沿用同风格 ⟹ 差异数 21→23（+2，均为
   `fold` 闭包与 `resolve_risk_mode` 的 match 链两处宽行）。二者只能取其一，本轮取行数上限；
   新模块 `open_ledger.rs` 保持 rustfmt 完全干净。若编排者判定 fmt 优先，可放宽行数上限后展开。
2. **前件复用**：见 §9.3「前件复用声明」，未在当前 HEAD 上重跑前件四臂。
3. **主工位并行破损（已自愈）**：修复期间并行工位一度把 `nest_lifecycle.rs` 留在半成品态
   （`PanLiveWindow` 新增 `gap_len` 但构造点未同步，主工位 `cargo build` 红）。本工位被禁碰该
   文件，遂改在 `git archive HEAD` 隔离树完成全部验收；收尾时并行工位已提交（HEAD 推进到
   `5505eaaed2`），主工位复绿并复跑得同一指纹 2031/1/136。

## 11. 结果包六要素

1. **结论**：#571 两轴评审的 1 项 Spec HIGH + 4 项 Standards 全部修复；风控止损改为按 campaign
   实例方向聚合；本票核心不变量（opened↔trade 双向 0/0）在新语义下仍成立。
2. **定义依据**：#570 探针 §3「风控止损」行——「必须枚举该 carrier 的实例，按实例 side/stop
   聚合 long/short stop」；`PositionNodeId` 四元组（A9/#166）中 `side` 是**入场 Candidate 方向**，
   与 `ActiveLeg.dir`（carrier 走势 eps）分属两个语义层，后者不是持仓方向的合法代理。
3. **边界条件**：结论翻转的条件有二——(a) 若判定「carrier 结构方向才是风控出口的合法方向契约」
   （即 close_pred 的方向项应绑结构而非持仓），则本修复方向反了，须回退并重做 golden；
   (b) 若同 carrier 多实例的止损应取「最松」而非「最紧」（例如按净头寸而非逐实例判），
   则 OR 聚合须改为其他聚合子，`stop_flags` 是唯一改动点。
4. **下游推论**：风控触发面从 1191 缩到 554（同 fixture）⟹ 所有以旧触发面为前提的
   execution/treasury 读数需重新登记；ArmR digest 与账户表已实测漂移；任何引用
   「wf8 账户表逐位不变」的历史声明对当前代码失效。
5. **谱系引用**：本轮不新增概念分离。相关既有谱系：090 号（严格性/声明膨胀禁令——旧
   golden 注释「触发面不变」在口径变更后必须照实改写，不得静默调常量）；
   coding-style「带修订留痕的单线程状态机不适用不可变律」注记（2026-07-28 编排者裁定）。
6. **影响声明**：改动 `admission.rs`（风控门重构 + ctx 参数捆）、`open_ledger.rs`（trait 归位）、
   `opsem_dump.rs`（`diff_tower` 拆分，dump 逐字节不变）、`runner.rs`（守卫迁生产键形 +
   新守卫 + golden 再生）、`fill.rs`（唯一生产调用点改 ctx 形）。影响面：逐 bar 风控门的
   `stop_long`/`stop_short` 真值序列 ⟹ 订单流 ⟹ 全部 execution/treasury 数值读数。
   未改任何 treasury golden 文件、未改校验脚本、未做 git 写操作。

# §12 golden 重锚执行（2026-07-28，编排者已批准）

## 12.1 依据

编排者已明示批准：按 #571 修复后的新口径重锚 `treasury-reverify-t1-armR-trades-golden-20260727.json`。
执行工位：`/tmp/kimi-nest-mainline`（禁 git 写操作、禁碰并行线在飞面）。

## 12.2 重跑口径

沿用 `scripts/check_armR_trades_digest.py` docstring 的 canonical 再生成命令，三窗逐窗单独跑：

```
cd rust
for tag in p3fold wf7 wf8; do
  M8_WIN_FILTER=$tag VOICE_EXEC=1 THETA_NEST_CERT_GATE=1 \
    M8_REPORT_PATH=/tmp/m8_win_gate_report_$tag.md OPSEM_DUMP_DIR=/tmp/m8_win_gate/$tag \
    cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos \
    -- --ignored --nocapture
done
```

运行在当前工作树（HEAD `5505eaaed2a31cb128ca5b27616e15cf92ad8bc9` + 未提交的 #571 修复，
`git status --short` 17 处改动，即本报告 §1–§9 描述的同一份修复）。wf8 窗口读数与本报告
§9.4「本轮修复后」列**逐字段一致**（execR=+6,490,928、MaxDD=0.0814、R=+7,004,502、
LCB(R)=−2,302,490、trades=160、digest=`0x9c6a202dbea2de77`），确认此次重锚跑的是同一代码态。

## 12.3 新旧 golden 对照

| 窗口 | 项 | 旧 golden（#446 锚，2026-07-27） | **新 golden（#571 锚，本次）** |
|---|---|---:|---:|
| p3fold | n_trades | 141 | **145** |
| p3fold | digest | `0xac5952cfcd8b8746` | **`0xd77d78c75a3a46f0`** |
| p3fold | bytes | 195,220 | **201,115** |
| p3fold | Long/Short | 72/69 | **73/72** |
| wf7 | n_trades | 184 | **189** |
| wf7 | digest | `0x981a0560b8db0b40` | **`0xf43066af2b0b11d8`** |
| wf7 | bytes | 253,658 | **262,476** |
| wf7 | Long/Short | 99/85 | **100/89** |
| wf8 | n_trades | 159 | **160** |
| wf8 | digest | `0x18291f8ba8f1d40f` | **`0x9c6a202dbea2de77`**（同 §9.4「本轮修复后」实测值） |
| wf8 | bytes | 216,129 | **221,626** |
| wf8 | Long/Short | 87/72 | **87/73** |

三窗 n_trades、digest 均较旧锚变化——#571 修复（风控 stop 从腿方向迁到实例方向聚合）触发面
覆盖全部三窗，不只是报告正文详述的 wf8。p3fold/wf7 此前未在 #571 主线中逐窗核对，本次重锚是
两窗首次在新口径下落盘。

## 12.4 regen 与校验执行记录

```
$ python3 scripts/check_armR_trades_digest.py --dump-dir /tmp/m8_win_gate --regen
golden 已重落：.../chanlun/review-results/treasury-reverify-t1-armR-trades-golden-20260727.json

$ python3 scripts/check_armR_trades_digest.py --dump-dir /tmp/m8_win_gate
p3fold: n_trades=145 digest=0xd77d78c75a3a46f0 bytes=201115 ✓
wf7: n_trades=189 digest=0xf43066af2b0b11d8 bytes=262476 ✓
wf8: n_trades=160 digest=0x9c6a202dbea2de77 bytes=221626 ✓
臂R trades 逐位无漂移。
（exit=0）
```

golden 的 `provenance.anchors` 未删旧锚（#446 历史锚保留在数组首位，脚本按协议只追加不覆盖），
新增第二枚锚记录 `source_base_head=5505eaaed2a31cb128ca5b27616e15cf92ad8bc9`、
`source_worktree="dirty（git status --short: 17 paths）"`、`dump_dir=/tmp/m8_win_gate`，
可读出「HEAD 5505eaae + 未提交 #571 修复」的来源链，满足 provenance 校验（`_git_commit_problem`
/ `_git_ancestry_problem` 对两枚锚均通过，脚本复跑 exit=0 已验证）。

## 12.5 全量指纹复核

```
$ cd rust && cargo test --release
test result: FAILED. 2031 passed; 1 failed; 136 ignored; 0 measured; 0 filtered out
```

唯一失败 `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`（即已知项
#115，与本票改动无关），指纹 2031/1/136 与工位任务书要求一致。

## 12.6 结论

ArmR trades golden 已按 #571 修复后代码态重锚，三窗全绿，provenance 可追溯，指纹核对通过。
编排者批准在案（见本节 §12.1，任务下发文本 2026-07-28）。

---

# §13 #596 影子评审处置（2026-07-28，第三执行体）

工位 `/tmp/kimi-nest-mainline`，范围钉死 #596 影子评审的四项（HIGH-1 + MED-1/2/3），不外扩。
禁 git 写操作（提交归编排侧）；并行线在飞面（`p123_fast_replay.rs`、`pi_bsp_timing.rs`、
`gamma_dump.rs`、`l3_delta_r_alpha.rs`、`mu_estimator.rs`、`wverify_run.rs`、
`issue71_chi_gamma.rs`、`agent-roster*`）未碰。

## 13.1 HIGH-1：`parent_units` 双计（已修）

### 13.1.1 根因

`open_ledger.rs` 的 `parent_units` 对同 carrier 全部非 ShortDiff 实例的 `units` **求和**。但
`LedgerOpen::units` 的来源是 `fill.rs` 开腿处的
`step_trace.sep_legs.iter().find(|s| s.id == leg.id).map(|s| s.q_units)`——`SepLeg` **按 carrier 唯一**
（`strategy/coverage/step.rs` #512 守卫：`next_active` 出现重复 `ElementId` 即 panic）。所以同 carrier
的 N 个 live generation 各自持有的是**同一份 carrier 级目标**的 N 份拷贝，不是 N 份可叠加的腿量。

求和 ⟹ `OscillationParentLeg::target_units` 翻 N 倍 ⟹ 震荡子仓下单量翻 N 倍。

这是本票（#571）**新引入**的缺陷：修前 `open_trades.get(&leg.id)` 恰取一份，是正确值；改键时把
「枚举全部实例」误当成「累加全部实例」。

### 13.1.2 修法

取**单份**：`generation` 最大者——即最近一次开腿的 sizing 快照，与 #571 前 supersede 语义下
`get(carrier)` 的返回值同值。「最大 generation」由 `live_for_carrier` 的 `BTreeSet<u32>` 升序迭代
保证（末次赋值即最大），不是「任取一份」的模糊口径。逐实例断言（`units` finite 且 ≥0）保留在
全部在飞实例上，fail-loud 覆盖面不缩。

### 13.1.3 TDD 红→绿

先写守卫（`open_ledger.rs` `mod tests`，新辅助 `open_units_at` 显式给 `units`；未改既有 `open_at`
签名以避开并行线测试设施面）：

| 测试 | 形态 | 修前（红） | 修后（绿） |
|---|---|---|---|
| `parent_units_is_single_carrier_snapshot_not_sum` | 同 carrier 同 bar Amb Long + Amb Short，各 40（#570 §2 bar3047 Arm0 实证形态） | `Some(80)` | `Some(40)` |
| `parent_units_takes_latest_generation_snapshot` | 跨 bar 同向重开，40 → 55 | `Some(95)` | `Some(55)` |
| `parent_units_skips_shortdiff_instances` | Amb 40 + ShortDiff 9；及全 ShortDiff | 通过（求和已 filter） | `Some(40)` / `None` |

### 13.1.4 可达性与触发面

同 carrier 多实例可达在案（#570 §2：Arm0 bar3047 L1#3 至 bar3176 才关）。但 `parent_units` 的
**唯一调用点**在 `fill.rs` 的 `if let Some(hist) = pan_div_hist.as_deref()` 分支内 ⟹ 需 `pan_div_hist`
注入才触发；该开关默认关，treasury 三窗读数未被污染（§13.4 实测 exit=0 佐证）。

## 13.2 MED-1：drain 的有效域声明（已补）

`open_ledger.rs` 模块头新增「drain 的有效域声明」段，要点：

- **已验证（L2）**：typed ledger ↔ **决策层** `StepTrace` 生命周期事件一一对应（wf8 双向 multiset
  join 795/795、失配 0/0，键 `(entry_bar, level, source_index, dir)`）。有效域仅此一条对应关系。
- **未验证且该 join 结构上无法证伪**：typed ledger ↔ **执行层**腿数。自 #571 起同 carrier 可并存
  N 个 live generation 各落一条 typed 行，而净额执行路径与 `SepLeg`（按 carrier 唯一）对该 carrier
  只有一条记录 ⟹ **typed trade 数 ≥ 执行层腿数**，同 carrier 多实例时严格大于。795/795 join 的
  两侧**都在决策层**，任何执行层读数都不进该 join ⟹ 它通过与否都不能证伪该不等式。
- **下游约束**：凡把 typed ledger 行数 / 行级 `units` 当执行层腿数或腿级持仓消费的读数，落在
  已验证有效域**之外**，需自带执行层证据。

认识论等级标注遵 `formalization-validity-domain`：795/795 是 L2，不得升格为「typed ledger 与执行
层一致」的声明。

### 13.2.1 口径影响逐消费口落位：μ 桶 / Σpnl（#600 Spec MED-2 补）

#596 MED-1 原文要求「标注 typed ledger 的 trade 计数自本票起 **≥** 执行层实际仓位数，**及其对
μ 桶 / Σpnl 的口径影响**」。上节只落了不等式，点名的口径影响未逐消费口写清。本条补齐（读码为证）：

| 下游读数 | 数据源 | 受 drain 多实例影响？ | 依据 |
|---|---|---|---|
| treasury/m8 `Σpnl`（含浮盈）、`max_dd`、`strat_return` | `trade_pnls_with_forced` ← 净额执行 `apply_order`/`apply_fill` | **否** | 该链不读 `typed_ledger`；每 carrier 净额只有一条持仓（`fill.rs:78`、`:297`） |
| `metrics.n_trades` | `trade_pnls.len()`（`metrics.rs:145`） | **否** | 计的是净额实现 PnL 笔数，不是 typed 账本行数 |
| `RDecomposition` 的 `cum_price_pnl = Σ units·Δpx` | fill loop 局部**净额持仓**变量 `units`（`fill.rs:1035`） | **否** | 此 `units` 是账户净持仓，**不是** `TypedTrade::units` |
| **μ 桶样本**（`MuEstimator::observe`） | `build_mu_from_bars` 对 typed ledger **逐行** observe（`l3_delta_r_alpha.rs` §G4 循环） | **是** | 同 carrier N 个 live generation ⟹ N 个样本 |
| **残差链** `ResidualTrade`（alpha 分离 / L3 面） | 同一循环 `records.push` | **是** | 同上 |

**μ 侧影响的精确形式**：样本**值不变**——`x_gamma` 是 qty=1 的持仓期相对收益（#65 量纲③，
`÷entry_px`），不读 `TypedTrade::units` ⟹ N 份拷贝不会把单笔收益放大 N 倍，`units` 的 N 份拷贝
问题止于 §13.1 的 `parent_units`。受影响的是**计数与独立性**：同 carrier 的 N 条 typed 行对应执行层
**同一条**持仓（`SepLeg` 按 carrier 唯一，#512 守卫），却各落一个 μ 观测 ⟹ 桶内 n 被抬高，且这 N 个
样本互为重复观测（非独立同分布，相关结构未测）。后果落在依赖样本量的量上：`mu_lcb` 按 n 而非
n_eff 计 ⟹ 置信下界**偏乐观**、χ 门**偏松**。方向可判，**幅度未测**（需按 `(carrier, bar)` 聚类统计
多实例占比，本票未做）。

**追加的下游约束**（在 §13.2 三条之外）：

- 引用 typed trade 计数作「交易笔数」的读数须标注为**决策层腿事件数**，非执行层仓位数；treasury
  系列**不在此列**（已核实其 `Σpnl`/`n_trades` 全走净额链，见上表前三行）。
- μ / χ 训练面的 n 与 LCB 在同 carrier 多实例存在时是**上界乐观**读数。#571 **之前**的历史 μ 读数
  不受此影响（单槽 supersede ⟹ 一 carrier 至多一行）；#571 之后重建的 μ 表须带此标注。
- 认识论等级：上表六行全是 **L0**（代码路径事实，读码可证）；「是」两行的**影响幅度**无 L2 测量
  ⟹ 既不得声称「可忽略」，也不得声称「已污染基线」。本条**不改任何代码**，是有效域声明的补全。

## 13.3 MED-2：`open_share_leg` 同 bar 序——取「回退原序」

### 13.3.1 未声明的改动（#571 引入）

| | 顺序 |
|---|---|
| #571 前 | 四类关闭（`close_share_leg`）→ **然后** `for (c,_) in opened { open_share_leg() }` = **先关后开** |
| #571 后（修前） | `opened` 循环内 `open_share_leg()` → **然后** drain 结算（`close_share_leg`）= **先开后关** |

### 13.3.2 两序不等价（结构层）

两侧守卫**非对称**：开侧 `TwEvent::OpenShareLeg.is_legal_from` 按 `stage < EarningShares` 判；
关侧 `close_share_leg` 按 `open_legacy_legs >= 1` 判。同 bar 既开又关、且 `legs` 起始为 0 时：

- 先关后开：`legs=0` → close 被守卫跳过 → open → **终值 1**
- 先开后关：`legs=0` → open → `legs=1` → close → **终值 0**

`open_legacy_legs` 是 P2/P3/P4 谓词的**唯一承重分量**（`EnterEarning`/`ClearCampaign` 都要求
`legs==0`）⟹ 终值差异会传导到 stage 相变。

### 13.3.3 处置：回退原序（非「声明 + 前提证成」）

两选项中取回退。理由：靠「EarningShares 生产不可达」证等价是**有条件**的严格性——该前提是经验
命题（GAP3 系列的 L2/L3 读数），一旦某窗口触达 EarningShares 即失效，且失效时无 fail-loud。回退
使等价性回到**结构层无条件成立**（与 #571 前逐字节同序），不欠任何前提。

落地：`fill.rs` 把 `open_share_leg()` 从 `opened` 循环内移出，恢复到 drain 结算块之后、
`on_stage_event` 之前（#571 前原位），并恢复原 OQ-9 注释块 + 新增 `★#596 MED-2` 段说明
**本段位置承重、不得再并入开腿循环**及其精确不等价条件。

### 13.3.4 经验层佐证

三窗 treasury 校验 exit=0（§13.4）⟹ 两序在 p3fold/wf7/wf8 上**无数值差异**（该窗口集内不等价
场景未触发，L2）。回退的价值不在改数值，而在把等价性从「窗口相关的经验事实」提到「结构层
无条件」。

## 13.4 MED-3：treasury golden 复核——数值已验，commit 锚待编排侧

按 §12.2 canonical 命令三窗逐窗重跑（本票代码态），再跑校验（**未** `--regen`）：

```
$ python3 scripts/check_armR_trades_digest.py --dump-dir /tmp/m8_win_gate_596
p3fold: n_trades=145 digest=0xd77d78c75a3a46f0 bytes=201115 ✓
wf7: n_trades=189 digest=0xf43066af2b0b11d8 bytes=262476 ✓
wf8: n_trades=160 digest=0x9c6a202dbea2de77 bytes=221626 ✓
臂R trades 逐位无漂移。
（exit=0）
```

wf8 四层读数与 §9.4 / §12.2 逐字段一致（execR=+6,490,928、MaxDD=0.0814、R=+7,004,502、
LCB(R)=−2,302,490）⟹ 跑的确是同一口径，且 HIGH-1 修复 + MED-2 回退**不改三窗读数**（与预期一致：
HIGH-1 只在 `pan_div_hist` 注入路径可达，默认关）。

**照实登记的未闭合部分**：MED-3 的形式诉求是「golden 能从一个 **commit** 复现」。本工位禁 git
写操作 ⟹ 无法产生该 commit，也无法在其上跑 `--regen`。当前工作区仍脏（本票四项改动 + 并行线
`#581` 在飞面）。故：

- **数值层已闭合**：现锚三窗在本票代码态下逐位重现（exit=0），编排侧提交后的 commit 与本工作区
  内容一致 ⟹ 在该 commit 上 `--regen` 的数值必与现锚相同。
- **provenance 层未闭合**：golden 的 `provenance.anchors` 末枚仍是 `source_worktree="dirty（17 paths）"`。
  待编排侧提交本票后，在**干净** commit 上执行一次 `--regen` 追加第三枚锚即闭合：

```
cd rust && for tag in p3fold wf7 wf8; do M8_WIN_FILTER=$tag VOICE_EXEC=1 THETA_NEST_CERT_GATE=1 \
  M8_REPORT_PATH=/tmp/m8_$tag.md OPSEM_DUMP_DIR=/tmp/m8_win_gate/$tag \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture; done
cd .. && python3 scripts/check_armR_trades_digest.py --dump-dir /tmp/m8_win_gate --regen \
     && python3 scripts/check_armR_trades_digest.py --dump-dir /tmp/m8_win_gate
```

本工位**未**在脏工作区跑 `--regen`——那只会再追加一枚 dirty 锚，不解决 MED-3 指出的问题。

**三窗跑的工作区快照（照实登记）**：跑校验时工作区 = 本票四项改动 + 当时并行线在飞的 7 个文件
（`p123_fast_replay.rs`、`pi_bsp_timing.rs`、`gamma_dump.rs`、`l3_delta_r_alpha.rs`、
`mu_estimator.rs`、`wverify_run.rs`、`issue71_chi_gamma.rs`）；后五者对 lib 的改动经逐行核对**全部
是注释措辞 + 一处 `debug_assert!`→`assert!`（`mu_estimator.rs` 量纲③正值检查，未触发）+ 一条常量**，
不进数值路径。本工位收尾时并行线（#532/#533/#581）已进一步推进并新增 `admission.rs`、`runner.rs`、
`wverify_run/m8.rs` 等改动面——**那些改动不在本次 treasury 三窗校验的覆盖内**，其数值影响须由各自
票据自证，不得引用本节 exit=0 作背书。

## 13.5 验收

| 项 | 结果 |
|---|---|
| `cargo build --all-targets` | exit 0 |
| `cargo test --release` 全量 | **2034 passed / 1 failed / 136 ignored** |
| 相对 §12.5 | 2031→2034，delta **+3 = 本轮三条 HIGH-1 守卫**，既有失败集未扩大 |
| 唯一失败 | `extract_signals_bit_exact_digest_guard`（#115 在案，与本票无关） |
| `open_ledger::tests` 全部 | 8 passed / 0 failed（原 5 + 新 3） |
| treasury 三窗校验 | exit=0，逐位无漂移 |

指纹与三窗跑的时点同为「本票四项改动 + 上述 7 个并行线在飞文件」的工作区态（§13.4 末段）；此后
并行线继续推进的改动面不在该指纹覆盖内。

## 13.6 结果包六要素

1. **结论**：#596 影子评审 HIGH-1（`parent_units` 双计）已修——取 carrier 单份（最大 generation）
   而非求和；MED-1 有效域声明已补（模块头 + §13.2）；MED-2 取回退原序（无条件等价）；MED-3 数值
   层已验（三窗 exit=0），provenance 的 commit 锚待编排侧提交后补跑 `--regen`。
2. **定义依据**：`SepLeg` 按 carrier 唯一（`coverage/step.rs` #512 守卫对 `next_active` 重复
   `ElementId` fail-loud）+ `LedgerOpen::units` 定义（`ledger.rs:126`「透传 `SepLeg::q_units`，开腿登记
   时从 `step_trace.sep_legs` 冻结」）⟹ 同 carrier 多实例的 `units` 是同一份目标的拷贝，非可叠加量。
   MED-2 依据 `TwEvent::is_legal_from`（`strategy/ledger.rs:520-531`）两侧守卫判据不同。
3. **边界条件**：结论翻转的条件有三——(a) 若判定 `units` 应改为**实例级增量**语义（每 generation
   持有自己那份增量），则求和才是对的，但那要求 `SepLeg` 先按 `(carrier, generation)` 分解，即
   `coverage` 层改键，本修复须整体回退；(b) 若父腿快照的正确口径是「最早 generation」或「按 side
   分向取」而非「最新」，`parent_units` 是唯一改动点；(c) MED-2 若判定 EarningShares 生产可达性
   已由某 L2/L3 证据钉死为不可达，则「声明 + 前提」与回退等价，回退可撤。
4. **下游推论**：震荡（PanDiv）子仓 sizing 的父腿基数在同 carrier 多实例场景下由 N 份降为 1 份 ⟹
   任何以 `pan_div_hist` 注入态跑出的子仓下单量读数（若存在）须重跑；默认关态下全部既有 treasury/
   execution 读数不受影响（三窗 exit=0 实证）。MED-1 的有效域声明使「typed trade 数 = 执行层腿数」
   这一从未验证的推论**显式失效**——下游若有依赖须自带执行层证据。
5. **谱系引用**：本轮不新增概念分离。相关既有谱系：090 号（严格性/声明膨胀禁令——MED-2 不接受
   「有前提的等价」冒充无条件等价）；`formalization-validity-domain`（MED-1 的 L2 有效域标注，
   有效域 ≠ 定义域）；`no-patch-mentality` 第 8 条（务实禁令——MED-2 不取「先声明、以后再说」）。
6. **影响声明**：改 `open_ledger.rs`（`parent_units` 求和→单份 + 模块头有效域声明 + 3 条守卫 +
   测试辅助 `open_units_at`）、`fill.rs`（`open_share_leg` 回退到 drain 之后原位 + 注释）。
   未改任何 golden 文件、未改校验脚本、未做 git 写操作、未碰并行线在飞面。

## 13.7 修复 diff 两轴评审（范围钉死本票四项）

**异质源不可用，照实登记**：本轮尝试 `codex exec -s read-only` 做异质两轴评审，返回
`ERROR: You've hit your usage limit ... try again at Aug 3rd, 2026`（OpenAI 配额耗尽，在案已知项）。
工位纪律禁子代理/禁后台 ⟹ 两轴由本执行体自审，**同源**，不冒充异质裁定。

### 轴1 Standards

| 项 | 结论 | 依据 |
|---|---|---|
| `coding-style` 不可变律 | PASS | `parent_units` 是只读方法（`&self`），无就地修改；`open_share_leg` 移动不改变任何数据结构的可变性 |
| 文件规模（<800 行） | PASS | `open_ledger.rs` 751 行 |
| rustfmt | PASS | `rustfmt --edition 2021 --check open_ledger.rs` exit=0（§10 的「新模块保持 rustfmt 完全干净」维持） |
| 声明与实际一致（090 号，无声明膨胀） | PASS | 注释声明「`BTreeSet` 升序 ⟹ 末次赋值 = 最大 generation」，实际 `live_for_carrier` 迭代 `generations_by_carrier: HashMap<_, BTreeSet<u32>>` 的 `&BTreeSet` 迭代器，升序成立；溢出断言从 `total.is_finite()` 收窄为 `units.round() <= u64::MAX`，因 finite 与 ≥0 已在循环内逐实例断言，无重复声明也无缺口 |
| `no-patch-mentality` | PASS | MED-2 未取「加声明留前提」的半修法，取无条件等价的回退（§13.3.3）；无 TODO 遗留 |
| `formalization-validity-domain` | PASS | MED-1 声明显式标注 L2 等级并写明「该 join 结构上无法证伪执行层不等式」，未把 L2 升格 |

### 轴2 Spec（四项，不多不少）

| # | 项 | 结论 | 核对 |
|---|---|---|---|
| 1 | `parent_units` 求和 → 取单份（最大 generation） | PASS | 红→绿实证（80→40、95→55）；调用点唯一（`fill.rs:1382`），无遗漏 |
| 2 | 模块头补 drain 有效域声明 | PASS | 见 §13.2；同时在 `parent_units` doc 交叉引用 |
| 3 | `open_share_leg` 回退到 drain 结算块之后 | PASS | 与 `git show 1ba0bcc49e^:fill.rs` 的原位（四类关闭之后、`on_stage_event` 之前）逐位同序 |
| 4 | 新增三条守卫测试 | PASS | 2 条先红后绿、1 条覆盖 ShortDiff 过滤与 `None` 分支；未改既有 `open_at` 签名（避开并行线测试设施面） |
| — | 范围外改动 | 无 | diff 仅 `open_ledger.rs` + `fill.rs` 两文件 248 行 |

### 重点核对项（逐条）

1. **`live_for_carrier` 是否真按 generation 升序** — 是。索引 `generations_by_carrier` 的值类型是
   `BTreeSet<u32>`，`(&BTreeSet).into_iter()` 按 `Ord` 升序 ⟹ 「末次赋值 = 最大 generation」成立。
   该依赖已写入 `parent_units` doc（若日后索引换成 `HashSet`/`Vec`，此处即失效，doc 是显式契约）。
2. **移动 `open_share_leg` 后 TW 计数语义** — 正确。恢复 #571 前原位后，同 bar 序为
   「四类 drain 关闭（含 `close_share_leg`）→ `opened` 的 `open_share_leg` → `on_stage_event`」，
   与 #571 前逐位同序；三窗 treasury 校验 exit=0 佐证无数值差异。
3. **遗漏调用点** — 生产 π 主循环内无遗漏（`open_share_leg` 该路径唯一）。**范围外观察（未改，
   照实登记）**：`fill.rs` 另有 `#68② dual ledger` 路径的 `open_share_leg`/`close_share_leg`
   （约 `:2747/:2829/:2837`），其注释自述「本路径无 stage 推进机构，恒 CostReduction，
   `is_legal_from` 恒真」⟹ §13.3.2 的不等价条件在该路径**不成立**（开侧守卫恒真）。该路径非本票
   引入、不在 #596 四项范围内，本轮未动。

---

# §14 #600 二轮修复批（2026-07-28，第三执行体）

工位 `/tmp/kimi-nest-mainline`。范围钉死 #600 两轴 MED 全单（Spec MED-1/2/3 + Std MED-1/2/3/4 +
LOW×2），不外扩。禁 git 写操作；并行线在飞面（`p123_fast_replay.rs`、`nest_lifecycle.rs`、
`pi_bsp_timing.rs`、`mu_estimator.rs`、`wverify_run*`、`agent-roster*` 等）未碰。

## 14.1 Spec MED-1（主症）：`parent_units` 恢复 #571 前语义

**症**：§13.1 的修法「先 `filter(entry_v != ShortDiff)`、后取末位」与 #571 前单槽语义**不逐点等价**。
单槽世界（`git show 1ba0bcc49e^:fill.rs:1392-1396`）是 `get(carrier)` 取唯一槽 → 判 `entry_v ==
ShortDiff` ⟹ `None`。当最新一次开腿是 ShortDiff 时：单槽世界返回 `None`（旧 Ambient 快照已被
supersede 覆盖、不存在），反序实现却把那条**已被覆盖的旧 Ambient 快照**取出来当父腿给震荡子仓定量。

**修**：先取 `generation` 最大者、**后**判其 `entry_v`：

```rust
let latest = self.live_for_carrier(carrier)
    .max_by_key(|open| open.position_node_id.generation)?;
if latest.entry_v == Vertical::ShortDiff { return None; }
```

**doc 订正**：§13.1 写入的「与 `get(carrier)` 返回值同值」在旧实现下是**不实声明**（上述反序情形不
同值）。新 doc 按多实例世界的精确口径重写：取法与单槽世界**逐点等价**，并写明顺序不可倒及其
反例（旧快照复活冒充父腿）。

## 14.2 Std MED-1/2/3：自证取法 + 判据可分 + 测试辅助去重

| 项 | 处置 |
|---|---|
| Std MED-1 | 「最大 generation」由 `max_by_key(position_node_id.generation)` **自证**，不再依赖 `BTreeSet` 升序迭代的隐式契约（§13.7 重点核对项 1 的那条 doc 契约随之作废——契约不再被需要）。变量名 `latest` 现名副实（此前指「末个非 ShortDiff」） |
| Std MED-2 | `parent_units_takes_latest_generation_snapshot` 数值方向反向：gen0=**55** → gen1=**40**，期望 `Some(40)`。这使三种读法可分——max(generation)=40（真判据）、max(units)=55、迭代序末位=40 但依赖 `BTreeSet` 契约 |
| Std MED-3 | `open_at` 委托 `open_units_at(.., 1.0)`，逐字段重复的构造体删除（口径单源） |

## 14.3 Spec MED-3 / Std MED-4：两处越界声明订正

- **Spec MED-3**（`fill.rs` TW 开腿段注释）：原文「**不依赖**『EarningShares 生产不可达』这一经验
  前提即无条件等价」越界——回退的是**顺序**，不是**计数语义**。drain 使关侧对每个 carrier 派
  N 次 `close_share_leg`（每 live generation 一次），pre-#571 单槽至多一次 ⟹ `open_legacy_legs`
  计数随 drain 已变。改写为：顺序回退至 pre-#571 原位；计数语义随 drain 变化已在 `open_ledger`
  模块头声明；此处只成立「与 #571 前**同序**」，不成立「无条件等价」。
- **Std MED-4**（`fill.rs:1288-1291` #563 订正）：该订正此前只落代码注释，gap2 设计稿原句未动
  ⟹ 声明面两处不一致。本轮**回写设计稿**：
  `chanlun/review-results/gap2-gamma-candidate-dump-design-20260719.md` §4① 原句下追加勘误行
  （原句保留、订正追记：「零额外指令」应读作「零额外分配/写入」），代码注释同步指向该勘误行。

## 14.4 Spec MED-2：§13.2.1 口径影响条目

见 §13.2.1（#596 点名的「对 μ 桶 / Σpnl 的口径影响」逐消费口落位）。要点（限定版，#600 三轮
微修再收窄）：treasury 侧 `Σpnl`/`n_trades`/`cum_price_pnl` **读数链**不读 `typed_ledger`（`fill.rs:78`、
`:297`、`metrics.rs:145` 锚）⟹ 该读数链不受 drain 影响。但 `open_legacy_legs`/`parent_units` →
`pan_div_child_units`（sizing）→ 订单量 → Σpnl 的**决策层回写**路径存在（`fill.rs:1389` 锚，
`parent_units` 读 typed ledger 供 `OscillationParentLeg` 接入 sizing）；当前靠
`center_oscillation.enabled=false` 默认关兜底（`pan_div_hist=None` ⟹ `pan_div_child_units` 恒 0，
bit-exact 不动 Σpnl，`fill.rs:1611`）。该路径的有效域声明照实补齐：**关**（当前默认）下「不受影响」
是 L0 代码路径事实；**开**（`center_oscillation.enabled=true`）场景下该路径是否受影响、影响幅度
——**未验域**，本票未测，不得外推「不受影响」到开启场景。μ 桶与残差链逐 typed 行 observe，
**受**影响，形式是样本计数与独立性（值不变）⟹ `mu_lcb` 偏乐观、χ 门偏松，方向可判、幅度未测。

## 14.5 LOW×2

- **LOW-1**：溢出断言边界 `units.round() <= u64::MAX as f64` → `<`。`u64::MAX as f64` 舍入为
  2^64（> `u64::MAX`），`<=` 会放行恰好 2^64 的值而 `as u64` 饱和成 `u64::MAX` ⟹ 错值静默通过。
- **LOW-2**：补两条 fail-loud 断言的测试覆盖（此前两条断言均无测试）：
  `parent_units_fails_loud_on_non_finite_units`（`f64::NAN`）、
  `parent_units_fails_loud_on_u64_overflow`（`1.9e19` > `u64::MAX`），均 `#[should_panic]` 锁 panic 文案。

## 14.6 TDD 红→绿

先写守卫再改实现（`cargo test --release --lib open_ledger`）：

| 测试 | 形态 | 修前 | 修后 |
|---|---|---|---|
| `parent_units_follows_latest_instance_vertical` | ① 最新=ShortDiff（gen0 Amb 40 → gen1 ShortDiff 9）② 最新=Amb（gen0 ShortDiff 9 → gen1 Amb 40）③ 全 ShortDiff | **FAILED**（① 得 `Some(40)`，应 `None`） | ok（`None` / `Some(40)` / `None`） |
| `parent_units_takes_latest_generation_snapshot`（改数值方向） | gen0 units=55 → gen1 units=40 | — | ok（`Some(40)`） |
| `parent_units_fails_loud_on_non_finite_units` | `units = NAN` | — | ok（panic 命中「非法 units」） |
| `parent_units_fails_loud_on_u64_overflow` | `units = 1.9e19` | — | ok（panic 命中「units 溢出」） |

红态实证（改实现前）：`test result: FAILED. 9 passed; 1 failed`，失败项即
`parent_units_follows_latest_instance_vertical`，panic 为 `left: Some(40) / right: None`。

## 14.7 验收

| 项 | 结果 |
|---|---|
| `cargo build --release --all-targets` | **exit=0**（首跑曾撞并行线 #601 半成品的 3 条 `E0432/E0599`——全在 `p123_fast_replay.rs` 引用 `nest_lifecycle` 新符号，与本批零符号交集；并行线 17:46 落盘后复跑即绿） |
| `cargo test --release --no-fail-fast` | lib **2040 passed / 1 failed / 137 ignored**，唯一失败 = `extract_signals_bit_exact_digest_guard`（#491 在案 digest guard）；集成 target 全绿。基线指纹 2034/1/136 → 本批新增 3 条（全绿）；余下差额来自并行线同期落盘（工作区共享，未提交 diff 中 `nest_lifecycle.rs` 新增 4 条 `#[test]`、`wverify_run.rs` 新增 2 条含 2 条 `#[ignore]`），净差与其内部改名/删除相抵，**未逐条核**——照实登记，不声称已归因到个位 |
| `cargo test --release --lib open_ledger` | **10 passed / 0 failed** |
| `rustfmt --edition 2021 --check open_ledger.rs` | exit=0（`fill.rs` 未纳入——该文件 rustfmt 非干净态早于本票，本轮只动注释文本） |
| 文件规模 | `open_ledger.rs` 803 行（<800 上限已越 3 行，照实登记：`common/coding-style` 的 800 行硬顶，超出部分全部是本批新增的守卫测试 + doc；生产代码段未增行。是否拆 `mod tests` 到独立文件属结构裁量，超出本批钉死范围，未自行外扩） |

## 14.8 结果包六要素

1. **结论**：#600 两轴 MED 全单 + LOW×2 全部落地，改 `open_ledger.rs`（`parent_units` 语义回退
   + `max_by_key` 自证 + doc 重写 + 4 条测试新增/改写 + `open_at` 委托）、`fill.rs`（两处越界/悬空
   声明订正，纯注释）、gap2 设计稿（§4① 勘误行）、本报告（§13.2.1 + §14）。
2. **定义依据**：#571 前语义原文 = `git show 1ba0bcc49e^:rust/src/theta_v0/backtest/fill.rs:1392-1396`
   （`get` → 判 ShortDiff → `None`）；`SepLeg` per-carrier 唯一 = `coverage/step.rs` #512 守卫；
   μ 量纲③ = #65（`build_mu_from_bars` 逐行 observe、qty=1 相对收益）；净额 PnL 链 =
   `metrics.rs:145` + `fill.rs:78/297/1035`。
3. **边界条件**：Spec MED-1 的修法在「同 carrier 恒 ≤1 个 live 实例」时与旧实现无差（退化为单槽）
   ⟹ 差异只在 #570 §2 实证的多实例形态上；`parent_units` 唯一调用点需 `pan_div_hist` 注入
   （默认关）⟹ 当前 treasury 读数不受本批影响。§13.2.1 的「treasury 不受影响」在「未来有读数
   改从 typed ledger 取笔数/units」时翻转。
4. **下游推论**：`parent_units` 的 doc 契约从「`BTreeSet` 升序」改为「`max_by_key` 自证」⟹ 索引
   容器换型（`HashSet`/`Vec`）不再破坏该函数（§13.7 重点核对项 1 的风险消解）。μ 表重建面须带
   §13.2.1 的「n 偏乐观」标注。
5. **谱系引用**：090 号（声明膨胀禁止——本批三处越界声明的订正依据：「与 `get` 同值」「无条件
   等价」「零额外指令」）；`formalization-validity-domain`（§13.2.1 全表标 L0，幅度未测不外推）;
   `no-patch-mentality`（LOW-1 的边界 `<` 不取「实践中不会到 2^64」的务实回避）。
6. **影响声明**：改 `rust/src/theta_v0/backtest/open_ledger.rs`、
   `rust/src/theta_v0/backtest/fill.rs`（仅注释）、
   `chanlun/review-results/gap2-gamma-candidate-dump-design-20260719.md`（追加勘误行，原句未改）、
   本报告。未改任何 golden/fixture/脚本，零 git 写操作，未碰并行线在飞面。
