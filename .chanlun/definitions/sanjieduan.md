# 三阶段与本金（Sanjieduan / Principal）

> 正本：本文件是 三阶段与本金 的教义正本。
> 最后裁定：[#1348](https://github.com/xy7365527-lang/NewChanlun/issues/1348)（2026-10-13）；[#1406](https://github.com/xy7365527-lang/NewChanlun/issues/1406) OQ-2a 补裁（2026-10-14）；[#1619](https://github.com/xy7365527-lang/NewChanlun/issues/1619) WE-1/WE-2 映射（2026-10-21）
> 受影响代码清单：本文件 §5（基线 main `465174240b`；FIN 线 `0d8a6017` 单独标注）、§6.4（对拍锁，PR #1616 分支 `673726b813`）；工作草稿副本 [`.chanlun/review-results/issue1323-grilling-20261013/1348-AFFECTED-CODE.md`](../review-results/issue1323-grilling-20261013/1348-AFFECTED-CODE.md)，票关后随归档出仓

**状态**: 已裁定（#1348 第一至第三问的教义部分；数据形态、零分母与公司行动见 [ADR 0031](../../docs/adr/0031-principal-batches-phase-predicate-corporate-actions.md)）
**最后更新**: 2026-10-21
**原文依据**: 第 31、35、40、49 课（博文，`docs/chanlun/text/blog/`）

**与既有生成态文件的关系**：[`fengkong.md`](fengkong.md)「概念3：成本归零策略」与 [`chanlun-trading-system.md`](chanlun-trading-system.md)「概念2：资金管理」描述的是同一组三阶段，二者仍为生成态操作说明。自 #1348 起，阶段如何判定、本金按什么口径计，以本文件为准；两处与本文件不一致时以本文件为准。

## §1 原文锚

全部按四步核过（行号、逐字、正文界、行首与行内注），路径一律 `docs/chanlun/text/blog/`。

- `031-第31课.md:24`【正文，界 :32】「一个最简单又最有效的管理，就是当成本为0以前，要把成本变为0；当成本变成0以后，就要挣股票，直到股票见到历史性大顶」
- `031-第31课.md:30`【正文，界 :32】「直到牛市结束前，才把所有股票全部清仓。而资金，就可以不断增加参与的股票种类，把这程序不断下去，这样，操作资金不会增加」
- `031-第31课.md:169`【界后，作者答疑回复：`:165` 回复块署名「缠中说禅：2007-02-15 16:16:43」，`:167` 为读者问，`:169` 在「==」后】「成本为0前，只补进相同的数量，仓位不增加。成本为0后，抛出后，跌回来，就把抛出的钱，全补进去，这样买回来的数量一定多了，股票才会越来越多」
- `035-第35课.md:32`【正文，界 :38】「只有当成本为0时，才算真正脱离风险」
- `040-第40课.md:20`【正文，界 :24】「每一重都对应着一定的资金与筹码」
- `049-第49课.md:64`【正文，界 :78，行首无注】「在中枢上方全部抛出筹码，在下方如数接回」

## §2 阶段是成本状态的函数 `[旧缠论]`

三阶段（降成本 → 退本金 → 增股数）由该重当前的真实成本 K 与已退本金决定（§1 `031:24`、`035:32`）。费用、亏损等真实经济事件可以使 K 回到正数，此时该重重新处于「成本为0以前」，扩量资格随之失效。

**与已交付单向 latch 的分工**（#1348 第三问，`[新缠论:选择]`）：`TStage`（Rust `rust/src/theta_v0/strategy/ledger.rs:94`；Lean `formal/Origin/TotalWealth.lean:140`、`:194`、`:208`）是 campaign 内的阶段**事件史**投影，只记「达标事件曾经发生」，AsKnown 不回填；**扩量资格**由唯一谓词 `growth_eligible` 按 ADR 0031 裁定三的分方向当前账本事实计算（多头为 K、P、P_recovered；空头为费后已实现利润、保证金批次与负债），latch 不得作为它的输入。两者是「历史事实」与「当前准入」两个对象，不是同一判断的两档（#799 限定词①）。更正类事件（bust）引起的阶段重算按 SPEC C08.6 以 RecomputedWithRevision 独立命名。

**分方向的单向蕴含**（#1406 OQ-2a 补裁）：多头 `growth_eligible ⇒ latch = EarningShares`；空头 `growth_eligible ⇒ latch = CapitalRecovered`，空头 latch 不进入赚股第三相。依据为 ADR 0031 裁定一的空头回本判据、裁定三的分方向资格谓词与 SPEC C05.5。两条均非双向等价：回本后的亏损往返即使没有费用，也可令当前资格失效而阶段事件史保留；latch 仍不得作为资格输入。

**证明与对拍义务归 #1406**：`TotalWealth.lean` 既有阶段单调性定理不等于上述资格蕴含已获证明。交付时须声明候选 Lean 模型的实际覆盖面；若只覆盖多头，空头条由 Rust 同输入同输出锁（#804）承担，不得声称空头资格关系已形式证明。

## §3 本金按来源分批，利润不是本金 `[旧缠论]`

本金 P ＝ 该重已授权来源的真实自有投入之和，分批冻结：开局批 P0（多头重为开局锁定的自有现金/保证金；空头重为投入保证金 M，ADR 0013 裁定三）。回本后利润再投入是**同一重的股数增长**（§1 `031:169`），进 income-book 的资本化 A 栏，不进 P；提取 W 也不动 P。新增外来投入只能以显式授权批次进入（§1 `031:30`「操作资金不会增加」；ADR 0016 裁定五「扩张走换档，不挪钱」）。回本目标 Σ P_b 只增不减。

回本后滚出的筹码与资本化现金记增长权益 G（ADR 0034），不是本金；换档转移形成的目标重本金批次来源为 `transfer(id)`，金额 = 转移现金 + 转移筹码的账面成本。（#1355／#1357）

## §4 数量归零不是账的终点 `[旧缠论]`

§1 `049:64` 全部抛出再如数接回，中间数量为零而操作没有中断。金额账（K、P、R、费用）属重，跨 Voice 连续；归属指针属 Voice/family/command（ADR 0010「声部死而重不死」）。数量为零期间单位量输出 Typed NotComputable。整重经营终结（§1 `031:30`「才把所有股票全部清仓」；Lean `clearCampaign`）是另一事件，归 FU-07，本文件不裁。

## §5 受影响代码清单

基线：main `465174240b989a1d99f99a047fbb46b3ee215f49`；FIN = #1394 财务线 `0d8a6017231348e2c4a005f68d4e2e7a457a59fe`（未合入 main）。改码归 #1406、#1421 与 TB-06 叶票；本节只列影响面。

| 侧 | 文件:行 | 基线 | 处置 |
|---|---|---|---|
| Rust | `rust/src/economic_session/book.rs:151` | main | 分量白名单增 principal_contribution / fee_only / dividend_cash / withholding_tax / split |
| Rust | `rust/src/economic_session/book.rs:225` | main | `du==0` 二分：无费用分量拒；有已声明费用分量则接纳、只改 K |
| Rust | `rust/src/economic_session/book.rs:347` | main | `principal_baseline` → Known 批次清单；`capital_phase` → phase_events + growth_eligible；`unit_cost_policy` → 整数对/NotComputable |
| Rust | `rust/src/economic_session/book.rs:356-357` | main | 非负存量域不变，q=0 合法 |
| Rust | `rust/src/economic_session/book.rs:370` | main | Π/A/W/R 算子不变；A 不进 P |
| Rust | `rust/src/economic_session/book.rs:398-413` | main | K 有符号累加（处置现金、回收、分红进入后可为负），不 clamp |
| Rust | `rust/src/economic_session/store.rs:636` | main | 审计重放覆盖 fee_only / principal_contribution / 公司行动事件 |
| Rust | `rust/src/economic_session/voice.rs:149` | main | 新 Voice 开局依 Applied 原件；不接旧尾账 |
| Rust | `rust/src/economic_session/voice.rs:165` | main | 开局量 > 0 不变 |
| Rust | `rust/src/theta_v0/strategy/ledger.rs:94` | main | `TStage` 只作阶段事件史投影；分工注释须带 #1348 |
| Rust | `rust/src/theta_v0/strategy/ledger.rs:121` | main | `advance_to` 不改 |
| Rust | `rust/src/theta_v0/strategy/ledger.rs:175` | main | `TwState`：notional_in 不是本金批次；stage 不作准入输入 |
| Rust | `rust/src/theta_v0/strategy/ledger.rs:716/742` | main | `LedgerEvent` / `ledger_step_components` 不变 |
| Rust | `rust/src/theta_v0/backtest/fill.rs:200-255` | main | 旧净额臂（全平重置 entry_cost，`:224`）与本裁定不同口径，不作本金定义；列入 C11 收编/退役清单，本轮不代裁终态 |
| Rust | `rust/src/session_protocol.rs:39` | main | 引用：`parse_canonical_i64` 规范十进制整数文本，不改 |
| Rust | `rust/src/economic_session/finance.rs:143-145` | FIN | 引用：`checked_add` 整数约定，不改 |
| Rust | `rust/src/economic_session/finance.rs:208-210/227` | FIN | 引用：money_unit / quantity_unit / quantum 政策字段与 `quantum == "1"` 校验，不改 |
| Rust | `rust/src/economic_session/finance.rs:234-239` | FIN | 引用：`baseline.amount` 是 E 共同责任界，与本金 P 禁止互代，不改 |
| Lean | `formal/Origin/TotalWealth.lean:97` | main | `TWState.notional_in` 登记为非本金批次名分；不改定义 |
| Lean | `formal/Origin/TotalWealth.lean:140` | main | `advanceTo` 不改 |
| Lean | `formal/Origin/TotalWealth.lean:194` | main | `stage_rank_monotone` 不改；#1406 OQ-2a 义务：多头 `growth_eligible ⇒ latch = EarningShares`，空头 `growth_eligible ⇒ latch = CapitalRecovered` 且无第三相；非双向等价，latch 不作资格输入；Lean 覆盖面与 Rust 同输入同输出锁见 §2 |
| Lean | `formal/Origin/TotalWealth.lean:208` | main | EarningShares 不可回退（`earning_no_regress`）不改 |
| Lean | `formal/Origin/LedgerReconciliation.lean:467` | main | `hAlloc` 前件不改；不从中推本金 |
| Python | `s_session/economic_query.py:72` | main | 只透传 principal / capital_phase / unit_cost 的 typed 结构 |
| Python | `s_session/economic_http.py:36/62` | main | GET/POST 入口只透传，不新判 |
| JS | `s_session/browser/economic.html:67` | main | 「本金与阶段」卡片显示 NotComputable 原因与 K/q 整数对，不计算比率 |
| JS | `s_session/browser/tb03a-client.js:124` | main | 图像校验键集若扩 principal/phase_events 须同步 |

## §6 WE-1/WE-2 工作例与义务映射（#1619）

### §6.1 来源与名分

WE-1/WE-2/WE-2b 原出 #1406 备料 PREP-R2.md（2026-10-14，仓外 `/private/tmp/goal1323-native-20261012/tb1406-prep-r2/`）。该原件**不可恢复**：原目录已不存在，本机迁移清单与 G: 归档均无此文件，`git log --all` 无此路径（另见 #1406 [评论 6244920680](https://github.com/xy7365527-lang/NewChanlun/issues/1406#issuecomment-6244920680)）。其「34 条义务」的 R01–R34 编号**退役，不重造**；下表的映射目标直接写来源条款（ADR 0024/0031/0036、本文件 §1/§2、#1406 根裁定 [评论 6161055625](https://github.com/xy7365527-lang/NewChanlun/issues/1406#issuecomment-6161055625) OQ-2/OQ-6/OQ-7）。

数值原件经 git 恢复（C0 提交 `c1ac352aac`，provenance 自述「数字来源 #1406 备料 r2 PREP-R2.md §4.3」）。原件把 WE-1 与 WE-2 合写为一个条目，分界不可恢复：二者合为一个冻结例，「WE-1/WE-2」只作例名，不追溯拆分。

**数值名分**：`TEST-L1`、`TEST-CASH`、`fixed_q=10`、P0=10000/1000 及下表全部金额均为 **TestOnly 冻结值**，只证明 `tb03c-book/1` 规则域，不是资金政策。

### §6.2 测试锁载体（不是正本）

| 载体 | sha256 | 以何为准 |
|---|---|---|
| `s_session/tests/fixtures/tb03c/HAND-ORACLE.json` 的 `book_cases[0]`（chong-L1）与 `book_cases[1]`（chong-E1） | `39d4af35236c229512a1cec3bfbb07e366ed02896d1aee93e75bee6a8179ba3c` | 提交 `dfd2d4c227` |
| `s_session/tests/fixtures/tb03c/raw/chong-L1.json` | `9c6bf06f91a2a7c64d65e97af473a30531010335c1cd470a81de0a3a2e87ee61` | 提交 `c1ac352aac` |
| `s_session/tests/fixtures/tb03c/raw/chong-E1.json` | `ba4f6782393997ea7ea80034abd9998da46e56a9e79d1e2114eed6b8d91103f8` | 提交 `c1ac352aac` |

入 main 前以上述提交对象为准，入 main 后以同目录 `MANIFEST.json`（`book_revision` = `tb03c-book/1`）钉住的字节为准，两者 sha256 相同。**夹具与本节不一致时，本节为正本，判夹具有缺陷**；改夹具字节须开新 `book_revision`，不得就地改 `tb03c-book/1`（#1406 根裁定 OQ-7）。

### §6.3 映射表

**WE-1/WE-2（chong-L1，多头）**

| 步（pointer） | 原件事件 | 期待（摘） | 义务来源 |
|---|---|---|---|
| 0 `/events/0` | 开局 10：现金 9950，佣金 50，P0=10000 | K=10000，q=10，P=10000，P_recovered=0，cost_remaining=10000，fees=50，growth_eligible=NotEligible，growth_enabled=NotEligible，latch=CostReduction，gate=null，unit_cost={10000,10} | ADR 0031 裁定一（开局批、`principal_contribution`）；ADR 0024 裁定四（成本基含手续费）；ADR 0036 五（多头 = K）；ADR 0031 裁定二（整数对） |
| 1 `/events/1` | 短差卖 4：处置 4800，佣金 24 | K=5224，q=6，Pi=776，R=776，realized_gross=800，fees=74 | ADR 0024 裁定四（扣已实现回收）；ADR 0031 裁定二 |
| 2 `/events/2` | 同量回补 4：现金 4200，佣金 21 | K=9445，q=10，fees=95 | §1 `031:169`「成本为0前，只补进相同的数量」；ADR 0031 裁定三（K 有符号） |
| 3 `/events/3` | 短差卖 4：处置 16000，佣金 80（正向跨闸） | K=−6475，cost_remaining=−6475，Pi=12696，realized_gross=12800，fees=175，NotEligible，latch=CostReduction；phase_event `cost_remaining_nonpositive`@103 | ADR 0036 四（跨闸、K 不 clamp）；ADR 0031 裁定三（谓词要求 P_recovered ≥ P，单有 K ≤ 0 不够）；§1 `031:24` |
| 4 `/events/4` | 同量回补 4：现金 6000，佣金 30 | K=−445，q=10，fees=205 | 同步 2 |
| 5 `/events/5` | `recover_capital` 10000 | P_recovered=10000，Eligible，latch=EarningShares，gate=SimulatedGateReached，growth_enabled=AwaitingPolicy；`growth_eligible` 由 NotEligible 转 Eligible@105 | ADR 0031 裁定一、三；ADR 0036 七、八；本文件 §2 OQ-2a 段；§1 `035:32` |
| 6 `/events/6` | du=0 的 `fee_only` 600 | K=155，fees=805，NotEligible，growth_enabled=NotEligible，latch=EarningShares（不退）；Eligible 转 NotEligible@106，`cost_remaining_positive`@106 | ADR 0031 裁定二、三；本文件 §2（费用使 K 回正、资格失效；OQ-2a 段） |
| 7 `/events/7` | du=0 且无任何分量 | SchemaUnsupported | ADR 0031 裁定二「无费用分量仍拒」 |
| 8 `/events/8` | du=0 的成交类分量 | SchemaUnsupported | 同上 |
| 9 `/events/9` | du≠0 却带 `fee_only` | SchemaUnsupported | ADR 0031 裁定二（`fee_only` 以 du=0 为前件） |

**WE-2b（chong-E1，多头，OQ-2 反证）**

| 步 | 原件事件 | 期待（摘） | 义务来源 |
|---|---|---|---|
| 0 | 开局 10，现金 1000，P0=1000 | K=1000，P=1000，fees=0，NotEligible，latch=CostReduction | ADR 0031 裁定一 |
| 1 | 卖 5，处置 1500 | K=−500，q=5，Pi=1000，realized_gross=1000；`cost_remaining_nonpositive`@201 | ADR 0036 四 |
| 2 | `recover_capital` 1000 | Eligible，latch=EarningShares，gate=SimulatedGateReached，growth_enabled=AwaitingPolicy；NotEligible 转 Eligible@202 | ADR 0031 裁定三；ADR 0036 七、八 |
| 3 | 卖 1 得 100（无费用） | K=−600，q=4，仍为 Eligible，latch=EarningShares | ADR 0031 裁定三 |
| 4 | 追高买回 1 付 700（无费用） | K=100，q=5，fees=0，NotEligible，growth_enabled=NotEligible，latch=EarningShares；Eligible 转 NotEligible@204，`cost_remaining_positive`@204 | #1406 根裁定 OQ-2（「无费用事件时二者一致」被反证）；本文件 §2 OQ-2a 段 |

### §6.4 受影响代码清单

行号取 PR #1616 分支 `673726b813`（未合入 main）。以下为同输入同输出的对拍锁，不是 grep 锁（#804）。

| 侧 | 文件:行 | 处置 |
|---|---|---|
| Python | `s_session/tests/test_tb03c_oracle.py:27` | `test_manifest_pins_every_fixture_byte`：钉 §6.2 字节 |
| Python | `s_session/tests/test_tb03c_oracle.py:45` | `test_every_hand_step_matches_independent_oracle`：逐步对 §6.3 |
| Python | `s_session/tests/test_tb03c_oracle.py:78` | `test_two_fresh_processes_print_identical_bytes` |
| Python | `s_session/tests/tb03c_oracle.py:24` | `DU0_ONLY` 含 `fee_only`（WE-1/WE-2 步 6–9） |
| Rust | `rust/src/economic_session/tb03c_book_tests.rs:338` | `tb03c_rules_match_hand_oracle_on_every_step` |
| Rust | `rust/src/economic_session/tb03c_book_tests.rs:447` | `eligibility_implies_latch_and_latch_never_regresses`（OQ-2 单向蕴含） |
| 生成器 | `s_session/tests/fixtures/tb03c/generate_fixtures.py:117`、`:130` | WE-1/WE-2 与 WE-2b 段；改字节须开新 `book_revision` |

**落地拆分**：#1619 只负责本节正本落地，并核对 git 对象字节与 §6.2 所钉 sha256 一致，不依赖 PR #1616 合入。夹具入 main 后锁能否跑通，归 #1406 的 oracle 验收项承担；本节不改 #1406 其余欠项。

## 更新日志

- 2026-10-21：新增 §6（#1619）。PREP-R2 原件不可恢复，R01–R34 编号退役；WE-1/WE-2（合为一例）与 WE-2b 的步值映射直接指向 ADR 0024/0031/0036、本文件 §1/§2 与 #1406 根裁定 OQ-2/OQ-6/OQ-7；夹具降为测试锁载体，按路径和 sha256 引用；无代码行为变更。

- 2026-10-15：同步 #1406 OQ-2a（2026-10-14 补裁），资格与 latch 关系分方向写为单向蕴含，订正 §5 并明确 Lean/Rust 验证覆盖义务；无代码行为变更。
- 2026-10-13：v1.0 新建（#1348 裁定落正本层）。
