# #795 22 张图裁定落点核对：多少落在实际会跑的代码上

- 日期：2026-07-30；票据：[#795](https://github.com/xy7365527-lang/NewChanlun/issues/795)（parent map [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787)，blocked-by [#794](https://github.com/xy7365527-lang/NewChanlun/issues/794)）
- 性质：只读勘察，未改任何代码/issue/图正文。
- **勘察基准树**：本 worktree HEAD 为 `19b4015927`（2026-04-20，无 `rust/`/`formal/`/`analysis/`），
  故 `git archive main` 解包到 `/tmp/nc-main-795` 只读勘察；本 worktree 与主仓均零改动。
- **落点判据来源**：直接采用 [#794](https://github.com/xy7365527-lang/NewChanlun/issues/794) 报告
  `.chanlun/review-results/issue794-live-path-doctrine-20260730.md` 已定位的实际执行链——
  `bin/theta_backtest.rs → backtest::runner::run_theta_v0_pi`（⑤段）/
  `nautilus::backtest_engine::run_theta_backtest`（⑥段），七概念全部只吃 `theta_v0/`，
  区间套证书门 `THETA_NEST_CERT_GATE` 默认关闭（实测 `nest_gate_off=1`）。
- **终点声明来源**：直接采用 [#791](https://github.com/xy7365527-lang/NewChanlun/issues/791) 报告
  `.chanlun/review-results/issue791-line-overlap-matrix-20260730.md` 的终点声明提取表 + 重叠矩阵，未重做提取。
- **四类判据**：在链上（落在 #794 定位的执行路径上，实测/直接引用调用点）／在旁支（编译进树但这条链跑不到——CLI 二进制、env 门关闭的分支、仅 `#[cfg(test)]` 内消费）／不在树（Lean、文档、纯裁决流程票、已删）／查不到（未能在本次预算内追出证据）。**不做价值判断**，只测分布。

---

## 一、22 行落点表（浓缩版）

| # | 图名 | 主要产出（摘） | 落点 | 证据 |
|---|---|---|---|---|
| 59 | 全定义策略端到端 | v1真链/LEE/χ/C1双开/hedge-mode/买卖点生命周期 | **在链上** | hedge-mode 对冲腿注释直接见于 `theta_v0/backtest/runner.rs:385,476`、`fill.rs:783,4656,5325`（⑤⑥两段均消费的文件）；LEE/treasury 命中 `backtest/{fill,runner,config,metrics}.rs` |
| 106 | 内在级别多视窗 | 身份桥→三元锚+严格链 `typed_lookup_multi`/`by_triple_anchor` | **在旁支** | 落在 `backtest/admission.rs`，其读数受 `nest_cert_gate_enabled()` 门控（`admission.rs:62,83`），#794 实测该门默认关（`nest_gate_off=1`）——编译进树、本次配置下不执行 |
| 126 | 塔侧跨级链增产 | 证书索引 `typed_found` 命中率修正 | **在旁支** | `typed_found` 是 `admission.rs` 内 T4 shadow 列（注释自陈），同一 `nest_cert_gate_enabled()` 门控，同 #106 |
| 135 | 按级别管理出场 | 分级出场账本 `retrace_ledger::CenterFrame` | **在链上** | `classifier/signal.rs:98,2196,5475+` 直接 `use super::retrace_ledger`，signal.rs 是 #794 认定的买卖点判据文件 |
| 174 | 出场通道结构收敛 | 声部记账/ADR0001 会计结构 | **在旁支** | 关键词命中集中在 `rust/src/fugue_v3/{accounting,layer,prove}.rs`（顶层独立模块，#794 实测 `theta_v0` 对其零 crate 内依赖）+ CLI `bin/theta_overlay.rs`/`bin/pi_bsp_timing.rs`（#790 认定的零调用方 CLI 二进制） |
| 221 | orphan处置图 | 45 个孤岛去向裁定 | **不在树** | 纯资产清点裁决，#791 矩阵自判「全空」，无运行时产物 |
| 250 | wf7跨级链死活判定 | 「问题作废」+「真链判据=Lift 真值判据」定义层 | **不在树** | #791 自己判定为「分层非重复」，该定义实际实装在 #530 的 Lean（见下） |
| 278 | 多空双开与反向操作对齐 | hedge-mode 对冲腿/散装域/中枢生命周期事件机 | **在链上** | 同 #59 证据点：`backtest/runner.rs:385,476`、`fill.rs:783,4656,5325` 直接实现 hedge-mode |
| 379 | 短差账本 | `TStage`/`TwEvent`/`EarningShares` 通道 | **在链上（含运行时死态）** | `strategy/ledger.rs` 被 `backtest/fill.rs`（`global_risk_close`/`RDecomposition`）与 `runner.rs` 消费；**但** `runner.rs:728` 源码自陈「本闭环在生产策略下不触达 `EarningShares`（stage 恒 `CostReduction`）」——机制在链上、终态从未被生产路径实际达到 |
| 390 | 验收判据口径唯一性 | 划线+处置裁定 | **不在树** | 纯审计流程票，矩阵全空 |
| 445 | 交付纪律 | 交棒可交/to-spec 判据 | **不在树** | 纯流程规范，矩阵全空 |
| 500 | 卫生宪法图 | 世代命名单源化 | **不在树** | 命名/清算裁决非运行时代码，矩阵标注「退役 v1/dual 引擎属清算非新造」 |
| 529 | 塔内原生化长线 | `CandDeltaEvent`/`TowerChainCertificate`/N0-N7 | **在旁支** | `chain_cert::` 在 `classifier/mod.rs` 的用例位于 `#[cfg(test)]` 区块内（`mod.rs:2011` 起）；矩阵自己标注 Consume_at「尚未接执行，声明中」 |
| 530 | Lean形式化残余 | `LiftContext.lean`、Θ_parse 清零 | **不在树** | `formal/Origin/LiftContext.lean`，Lean 按判据定义即不在树（不做价值判断，这是合理归宿） |
| 566 | π钱安全语义钉死 | 级联止损/TW 账本口径 e2e 锁 | **在链上** | 「级联止损」命中 `backtest/{incremental,admission,econ_positive}.rs`+`classifier/decompose.rs`；`TwEvent`/`TStage` 机制同 #379，被 `fill.rs`/`runner.rs` 消费 |
| 582 | 一类点分档与Reset名义 | 37:18 分档 `T3InCGrade`/`ResetBroadcast` | **在链上** | `classifier/signal.rs:462-489` 直接实现「37:18 分档大闸」；`ResetBroadcast`/`CloseRoot` 命中 `backtest/fill.rs`/`selector.rs`（均在链上文件） |
| 597 | PanLive全层级完成前可见 | L2/L3 活窗 provider | **在旁支** | 核心对象 `classifier/nest_lifecycle.rs` 全仓零外部调用方；主验证载体是 `bin/p409_pan_live_probe.rs`（#790 认定类零调用方 CLI 探针） |
| 654 | 陈旧信号与最弱二类处置 | `#647`门收场/最弱二类策略纪律 | **在链上** | 命中 `backtest/{signal,fill}.rs`（在链文件）+ `wverify_run.rs`/`gamma_dump.rs` |
| 695 | 欠账总账归口 | 68 笔逐笔终态清点 | **不在树** | 自身是记账/清点票，不产运行时代码；其开出的新图 L04→#737 单列一行判定 |
| 737 | missing_cert成因定案 | （Decisions so far 为空） | **查不到** | 尚无任何产出可测——不是「不在树」，是还没做完 |
| 743 | 端到端模块化 | `bsp_at` 函数族单源化 | **在链上** | `bsp_at()` 被 `backtest/signal.rs:119`、`backtest/fill.rs:4195` 直接调用，两处均在 #794 认定的判据链路上（注：判据本身出自 `signal.rs`，`bsp.rs` 只提供查询——#743 收敛的正是这条被实际执行的查询路径） |
| 767 | wayfinder工作流正本 | 工作流文档落盘 | **不在树** | 纯流程文档，票面「两行声明」自称不带实装 |

---

## 二、四类计数

| 类别 | 计数 | 图号 |
|---|---|---|
| **在链上** | 8 | 59, 135, 278, 379, 566, 582, 654, 743 |
| **在旁支** | 5 | 106, 126, 174, 529, 597 |
| **不在树** | 8 | 221, 250, 390, 445, 500, 530, 695, 767 |
| **查不到** | 1 | 737 |
| 合计 | 22 | |

---

## 三、逐条清单：在旁支 + 查不到（做了但跑不到的候选）

### 在旁支（5 张，编译进树，本链默认配置跑不到）

1. **#106 内在级别多视窗**——三元锚/严格链 `typed_lookup_multi` 落在 `backtest/admission.rs`，
   受 `nest_cert_gate_enabled()` 门控（`admission.rs:62,83`）。#794 实测该门默认关闭
   （`THETA_NEST_CERT_GATE` 未设，`nest_gate_off=1`）——**同一区间套证书门**，与 #794 的核心发现
   「区间套本链一次都没跑」是同一件事的不同侧面。
2. **#126 塔侧跨级链增产**——`typed_found` 命中率修正是 `admission.rs` 内部 T4 影子对照列
   （源码自陈「shadow comparison」），门控同 #106。
3. **#174 出场通道结构收敛**——声部记账/ADR0001 会计结构主体落在 `rust/src/fugue_v3/`
   （顶层独立模块，#794 实测 `theta_v0` 对其**零 crate 内依赖**），仅经 `bin/theta_overlay.rs`/
   `bin/pi_bsp_timing.rs` 两个 CLI 二进制触达——这两个正是 #790 点名「9 个 CLI 全仓零调用方，
   只能人手动跑」的那类。
4. **#529 塔内原生化长线**——`CandDeltaEvent`/`TowerChainCertificate`（N0-N5 已交付部分）的
   `chain_cert::` 消费点在 `classifier/mod.rs` 中位于 `#[cfg(test)]` 区块内（`chain_book_over_prefixes`
   等函数，`mod.rs:2011` 起的测试模块），**生产路径未消费**；矩阵原文自己标注 Consume_at
   「尚未接执行，声明中」——即 N0-N5 编译进树、单测覆盖，但要等 N7 消费点接上才会被
   `run_theta_v0_pi` 真正跑到。这与 #787 立图起源提到的「三片雾悬空」是同一现象在代码层的实证。
5. **#597 PanLive全层级完成前可见**——L2/L3 活窗核心对象 `classifier/nest_lifecycle.rs`
   全仓零外部调用方（grep 无命中）；主验证载体 `bin/p409_pan_live_probe.rs` 属零调用方 CLI 探针类。

### 查不到（1 张）

1. **#737 missing_cert成因定案**——OPEN，frontier，`Decisions so far` 为空，尚未产出任何决策或
   代码，本票无法判定其落点（这不是「不在树」，是「还没做完」——090 照实，不能把「查不到」
   写成「不在树」）。

---

## 四、#795 指名要答的三个问题

**注**：以下三条的对象（#485/#620/#158-161）**不在 #791 的 22 张图清单内**（它们是独立的
spec/task 票，非 wayfinder:map），本报告对它们的追证深度低于上面 22 行主表，如实标注置信度。

1. **#485（高级别三类点，anchor 门+归属链+终态切换，仍 OPEN，指向 #473）/ #620（账本内核抽取，
   仍 OPEN，指向 #465 裁定 A）/ #106（内在级别多视窗）**——
   #106 已在主表判定**在旁支**（同一 `nest_cert_gate_enabled()` 门）。
   #485、#620 两票经 `gh issue view` 确认**均为 OPEN 状态**（未关票），本次预算内未能对其
   代码落点做逐符号追证——**查不到**（诚实标注，非「不在树」）。#485 的 anchor 门机制与 #106
   共享 `admission.rs` 基础设施的可能性较高（同属跨级证书链簇，参见 #791 簇 A），但未经实测确认，
   不作为结论列出。
2. **反向根 R1–R5（#158–#161）**——`gh issue view 158` 确认标题为「反向根 R1：前置验证——
   顶层子树清仓后声部栈空性」，状态 **OPEN**。票面即标注为「前置验证」（尚未进入实装阶段），
   本次未在 `rust/src` 中定位到与「反向根」直接对应的模块或函数命名——**查不到**（本次预算未做
   逐票深挖，票面状态本身也提示可能尚未落码；不排除落在未用中文关键词搜到的对象里）。
3. **#529 塔内原生化 N0–N7**——已在「在旁支」清单第 4 条给出实测证据：**N0-N5 编译进树、
   有单测覆盖，但生产消费点（`chain_cert::` 在 `classifier/mod.rs` 的使用）位于 `#[cfg(test)]`
   区块内，`run_theta_v0_pi` 不吃它**。答案是「是，在等 N7（或某个尚未落地的 Consume_at 消费点）
   接上才会被跑到」——与矩阵原文「声明中」一致。

---

## 五、未测项（090 照实）

- #485/#620/#158-161 四票的精确代码落点未逐符号追证，标注「查不到」而非猜测性归类。
- #379/#566 的 `EarningShares`/TW 机制虽判「在链上」，但其中至少一个终态（`EarningShares`）
  经源码自陈在生产策略下从未被实际进入——本报告未对其余状态转移逐一验证是否同样存在
  「编译可达但运行时不可达」的情况，只报告了已发现的这一处。
- 本次未跑 `cargo test`/`cargo build`，全部落点判据基于静态 grep + 调用点追溯（跟随
  #794 已验证过的插桩证据链路，未重新插桩）。

---

## 报告落点

`/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-ad8dfce67e654b6f4/.chanlun/review-results/issue795-map-landing-audit-20260730.md`
