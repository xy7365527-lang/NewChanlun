# 状态机与生命周期既有实现盘点（ReviseEvent / 五钟是否已有雏形）

- 日期：2026-07-20
- 工位：调研文档工位（只读盘点；未改 rust/src 一行，未做 git mutation）
- 分支：`kimi-nest-mainline-20260717`（worktree `/tmp/kimi-nest-mainline`）
- 检索面：`rust/src/**`（含 `theta_v0/`、`trading/`、`bin/`）与 `formal/**`（*.lean）
- 对照基准：E2E 原型 `doc-divergence-endtoend-prototype-20260718.md` §1 表（:25 E2E-D5）、§1 修订协议（:66、:81、:83）、§4.1 因果状态机（:142-160）；教义 024:24（C 段完成时结算）、061:26（活假设→观察→力度反超则背驰段不成立）

## 0. 结论先行

1. **E2E-D5 四态（`Provisional｜Confirmed｜Invalidated｜Unresolved`）作为事件生命周期状态机：rust/formal 全缺。** `Provisional` 全仓零命中（仅 Lean `ActiveTail.provDir` 注释出现英文 "provisional"，formal/Strict/Parse.lean:91）；`Unresolved` 只有 provider 完备性审计计数 `unresolved_targets`（p92_nest_replay_postruling.rs:361-369 等），无事件状态语义；`Confirmed`/`Invalidated` 命中全部是别语义（证书终态、静态判据谓词、持仓腿状态）。
2. **`ReviseEvent`：rust/formal 零命中。** E2E §1:66/:81 定义的 `ReviseEvent_at` 修订协议（revision/supersedes_revision、转移表、五钟写入纪律 §1:83）无任何代码对应物。
3. **五钟：仅 `judge_at` 单钟有载体**（`NestCandidateEvent.judge_at`，level_view.rs:489），且语义是「prefix 首次观察/首证钟」单钟，不是五钟；`observed_at/confirmed_at/invalidated_at/structure_end_at/first_provable_at` 字段级零命中，`first_provable` 仅存在于 exit.rs:154-156 的 v1 预留注释。
4. **「活假设→完成→证伪」（061:26 + 024:24）在背驰事件域：全缺；在结构/对象层有二态碎片雏形**（`MoveStatus::{Active,Completed}`、`CpLifecycleStatus::{Pending,Closed}`、Lean `ConfirmedMove/ActiveTail` 类型分离），但均不含证伪路径，也不在 `NestCandidateEvent` 域。
5. 总判定：**部分缺 X → 事件域接近全缺**（详见 §3）。

## 1. 命中分类表

分类口径：①证书终态（一次性判定结果，非时变状态）②时间戳/钟 ③真生命周期状态机（多态 + 推进函数）④无关同名。

### 1.1 `Provisional` —— 零命中

| 命中 | 位置 | 语义 | 分类 |
|---|---|---|---|
| 英文词 "provisional" | formal/Strict/Parse.lean:91（`ActiveTail.provDir` 注释「当下**临时**方向（provisional——可能随未来延伸翻转，不是终局）」） | 未完成尾部的临时方向，类型层「活假设」语义 | ③的教义雏形（类型层，非事件状态机） |

rust/src 全零命中。E2E-D5 的 `Provisional` 态无任何代码载体。

### 1.2 `Confirmed` —— 命中全部为证书终态/静态判据/分类标签

| 命中 | 位置 | 语义 | 分类 |
|---|---|---|---|
| `SegmentTermination::FirstKindConfirmed` / `SecondKindConfirmed` | rust/src/theta_v0/parser/segment.rs:200,205 | 段终止方式（第一/二种情况确认），一次性段证书终态 | ①证书终态 |
| `SecondKindResult::Confirmed { end_offset }` | rust/src/theta_v0/parser/second_kind.rs:136,154 | 第二特征序列分型出现 ⟹ 段端确认 | ①证书终态 |
| `NestTurnClass::NestedConfirmed` | rust/src/theta_v0/classifier/turn_class.rs:78,113,388 | 链顶背书形态分类（043:28/30 情况一），partition 分类非时变状态 | ①分类标签（终态快照口径） |
| `CenterConfirmedComplete` | formal/Origin/CenterComplete.lean:146；定理 :166/:251/:293/:348 | 完整中枢判据谓词（静态，三段合取） | ①静态判据谓词 |
| `FeatureConfirmed` | formal/Origin/SegmentFeatureComplete.lean:123（与 `SegEndComplete` 同义反复 :143-145） | 段终静态判据 | ①静态判据谓词 |
| `ConfirmedMove` | formal/Strict/Parse.lean:79-83（「交易只用 confirmed」:77） | 已闭合走势结构体；与 `ActiveTail` 构成 confirmed/active 类型分离（:106-116，:417 `ConfirmedActiveSplit`） | ③雏形（类型层完成/暂定二分，无转移函数） |
| `nestingConfirmed : Bool` | formal/Tlayers/Operational.lean:629-643 | 操作层布尔标志 | ④无关同名 |

### 1.3 `Invalidated` —— 命中在持仓域生命周期，背驰事件域零命中

| 命中 | 位置 | 语义 | 分类 |
|---|---|---|---|
| `HeldLegState::Invalidated` | rust/src/theta_v0/strategy/persistent.rs:59（枚举 :50-60：`LivePresent｜LiveDetached｜Closed｜Invalidated`，anc.pdf §10 四态） | 持仓腿（held leg）四态之一：结构或风险显式作废 | ③真生命周期状态机（**交易持仓域**，非背驰事件域） |
| `held_state` 判 `Invalidated` | persistent.rs:394-395；coverage.rs:2165,2213 | 不在 registry 或 `e.invalidated` ⟹ 作废 | ③同上 |
| `invalidate_cp_lifecycle_dirty_dependencies` | rust/src/theta_v0/classifier/recursive_tower.rs:1437（:1461 把 dirty 对象 `lifecycle` 置回 `Pending`） | c_p 对象 dirty 依赖**回退**机制（Closed→Pending） | ③生命周期维护函数（回退 ≠ E2E-D5 的 `Invalidated` 终态） |

背驰事件（`NestCandidateEvent`）域无任何 `Invalidated` 路径——p95 实证翻转只记审计计数（见 §2.3）。

### 1.4 `Unresolved` —— 只有审计计数，无事件状态

| 命中 | 位置 | 语义 | 分类 |
|---|---|---|---|
| `unresolved_targets` | rust/src/bin/p92_nest_replay_postruling.rs:189,361-369；p116_turnpoint_anchor_existence.rs:290,462-470；p123_fast_replay.rs:452,647-655；p124_shard_replay.rs:503,759-769；p124_merge.rs:546-555,887-897 | provider prefix pass 未解析目标数，完备性门 `unresolved_targets == 0` | ④无关同名（审计计数） |
| `mixed_unresolved` | rust/src/bin/p112_trend_predicate_caseaudit.rs:976,1003 | 链型分类计数 | ④无关同名 |

### 1.5 `first_provable` —— 零实装，仅 v1 预留注释

| 命中 | 位置 | 语义 | 分类 |
|---|---|---|---|
| 注释×3 | rust/src/theta_v0/strategy/exit.rs:154-156（「v0 基例的 first_provable ≡ 候选确认 bar……v1 E2E-N3 真链接入后改读**最深处证书 first_provable**（E2E-L 五钟，roadmap:75），经 `reverse_nest_cert_base` 接口预留」）、:180 | 五钟之一，接口预留未实装 | ②时间戳（**缺**，仅注释） |

formal 零命中。

### 1.6 `observed_at` / `confirmed_at` / `invalidated_at` —— 字段级零命中

rust/src 与 formal 均无此三字段。`confirmed_at7` 等仅出现于 backtest 测试函数名（backtest/runner.rs:4707 起 `buy1_at3_confirmed_at7`），④无关同名。

### 1.7 `judge_at` —— 唯一有载体的钟（单钟，非五钟）

| 命中 | 位置 | 语义 | 分类 |
|---|---|---|---|
| `NestCandidateEvent.judge_at: usize` | rust/src/theta_v0/classifier/level_view.rs:489；定义注释 :477-478「由 prefix 首次观察写入，不读取 `CompletedFreezeEvent.created_at` 或挂钟」；写入点 :712,:786（`= view.query.as_of`） | 事件的 prefix 首次观察/首证钟 | ②时间戳（首证钟载体） |
| `TypedNestCertificate.judge_at: Vec<usize>` | rust/src/theta_v0/classifier/nest.rs:417（sidecar 注释 :411；访问器 :436-437；D3 单调性检查 :458-459） | 逐 rung 判定钟向量；CERT 行主键 = judge_at 向量（p124_merge.rs:30「首证钟主键 = CERT judge_at 向量」） | ②时间戳（首证钟主键） |
| 「judge_at 只登记 D3 违反率，绝不作硬门」 | nest.rs:617 | 钟的纪律性限制 | ②纪律注记 |
| bin 回填 | p92_nest_replay_postruling.rs:202、p116:303/894/906、p123:488、p124_merge.rs:681、p108:356（`event.judge_at = book…` / `= first`） | 终态 CERT 装配时用合并 book 回填 judge_at | ②时间戳（注意：终态回填与「prefix 首次观察写入不后移」的 E2E §1:83 纪律不同——回填是终态装配口径） |
| `FrozenCompletedMove.judge_at` | rust/src/theta_v0/classifier/level_view_store.rs:27,42,294,382-395（`entry_bar = judge_at+1` :395） | 冻结完成走势的判定钟 | ②时间戳 |
| `e.judge_at` | p95_carriedonly_ab.rs:591；p100/p101/p106（CERT 行逗号钟表解析，取 max 作证书时钟） | dump 解析消费 | ②时间戳（离线消费） |

### 1.8 `lifecycle` / `state_machine` —— 三个真状态机 + 一堆无关同名

| 命中 | 位置 | 语义 | 分类 |
|---|---|---|---|
| `CpLifecycleStatus { Pending, Closed }` | recursive_tower.rs:461-464；字段 `CpScanOwnership.lifecycle` :446（「只能从 Pending 单调闭合为 Closed」:445）；推进 `advance_cp_lifecycles` :1697（闭合 :1839）；回退 `invalidate_cp_lifecycle_dirty_dependencies` :1437 | c_p（完整父级 c）对象生命周期：开放窗口 Pending → 合法第三类对象生成 Closed | ③真生命周期状态机（c_p 对象域；二态无 `Invalidated` 终态，回退是 dirty 维护而非证伪） |
| `HeldLegState` 四态 | persistent.rs:50-60（见 §1.3） | 持仓腿 LivePresent/LiveDetached/Closed/Invalidated | ③真生命周期状态机（持仓域） |
| `CenterEvent::{Formed, Extended, Terminated}` | rust/src/trading/types.rs:284,298；产生点 trading/center_book.rs:133,159-164；生命周期测试 center_book.rs:348,403,444 | 中枢事件：形成→延伸→终结（三买确认） | ③真生命周期（trading 域中枢对象） |
| `MoveStatus { Active, Completed }` | rust/src/theta_v0/classifier/decompose.rs:34-39（「链尾块——尾关系/尾中枢仍可变」/「前缀不可变」） | 走势块暂定/完成二态 | ③雏形（结构块层，最接近「活假设→完成」） |
| `lifecycle_diff` 计数 | p92:221-236、p116:322-337、p123:507-522、p124_merge.rs:704-719、p124_shard_replay.rs:640-655（全量 vs 增量 `cp_ownership` lifecycle 对账差） | 对账计数器 | ④无关同名 |
| `run_state_machine` | rust/src/bin/pi_bsp_timing.rs:284,536-537 | π-θ 回测执行状态机 | ④无关同名 |
| `t58_fires_on_lifecycle_break` | rust/src/trading/unified_necessity.rs:2147 | 测试名 | ④无关同名 |
| `located_state_machine_sides_alternate` | formal/Tlayers/Signal.lean:398-443；引用 `strict_hybrid_state_machine_strategy.md`（Origin/MainTheorem.lean:8 等 5 处） | located 流交替性定理名 | ④无关同名（定理命名承自策略文档） |

## 2. 对照 E2E-D5 / E2E-N5：「活假设→完成复核→反超失效」雏形判定

教义锚原文核对（本 worktree `docs/chanlun/text/blog/`）：

- **061:26**（061-第61课.md:26）：「65开始的走势，由于没实际走出来，所以在和55-60比较时，都**可以先假设是进入背驰段**。而当走势实际走出来，**一旦力度大于前者，那么就可以断定背驰段不成立**，也就不会出现背驰。在**没有证据否定背驰之前，就要观察**从65开始的一段其内部结构中的背驰情况」——活假设（Provisional）→观察→力度反超证伪（Invalidated）。
- **024:24**（024-第24课.md:24）：「**C 段的走势类型完成时**对应的MACD柱子面积……比A段对应的面积要小，这时候就构成标准的背弛」——完成时结算（Confirmed）；024:28「把已经出现的面积乘2，就可以当成是该段的面积」——未完成段外推（first_provable 的教义根）。

### 2.1 `NestCandidateEvent.divergence_confirmed` 的时变语义：终态快照布尔，非生命周期

- 字段本体是 `bool`（level_view.rs:487），事件 `#[derive(Copy)]`（:479）——**值语义、无 revision、无修订史、无 state 字段**。
- 每个 as_of 快照由 provider 重算：盘背写入点 level_view.rs:770-776（`segments_diverge_or` 力度或关系，027:32）；映射失败静默置 `false`（p104_trend_div_funnel.rs:6 注记 level_view.rs:574 同口径）。趋势域 R1 注释（level_view.rs:526-527）自述是「全合取确认的首个全成立时点 t*（确认时点语义）」——即 judged-at-prefix 的一次性判定。
- 同一身份跨 prefix 可翻转：p92 兜底对账显式比较 `event.divergence_confirmed == target.divergence_confirmed`（p92_nest_replay_postruling.rs:507-510）；p95 审计 `bonly_flips`/`flip_obs`（p95_carriedonly_ab.rs:473）。**翻转被观测、被计数，但无 `Invalidated` 状态承载**——新快照直接覆盖旧值，旧修订不保留，恰是 E2E §1:81「旧修订保留，禁止用删除模拟失效」的反面。
- 判定：divergence_confirmed 是「终态几何 + prefix 首证钟」缝合口径（E2E §4.1:160 自述该缝合事实见 nest-tower-native-gap-audit §3），**不是**「活假设→完成→证伪」雏形；它是单 prefix 判定 + 快照重算。

### 2.2 CKPT 滚动窗：快照审计侧信道，非事件生命周期

- `P92_CKPT`/`P116_CKPT`/`P124_CKPT` 每 K bar 做一次**全量快照装配**并 dump（p92:484-485「只写不判」；p116:586-587；p123:775-776；p124_shard_replay.rs:953-955）。
- CKPT 行格式 `CKPT caliber=… as_of=… exec=… top=… side=… bucket=… ids=…`（p92:948）——**无 state 字段、无五钟、无 revision**；`CKPT_STATS` 只有计数（p92:961）。
- 判定：CKPT 是跨 as_of 的证书集合对账通道（存在性 diff），不承载任何状态转移语义。与「活假设滚动观察」无关。

### 2.3 turn_class 暂定态：分类层挂起，状态机自认遗留

- `NestTurnClass`（turn_class.rs:76-90）四值：`NestedConfirmed`（情况一终态分类）、`XiaozhuandaCandidate`（:82 自述「永远不是『小转大确认』」）、`ExecEvidenceOnly`（诚实判负）、`DeferOrphan`（:87-89）。
- `DeferOrphan` 注释原文（:87-88）：「039:34 defer 域……挂起观察而非丢弃（**defer 时序重判状态机列遗留项，本关只交付分类口径**）」——代码自认时序重判状态机**未实装**。
- 判定：`XiaozhuandaCandidate`/`DeferOrphan` 是「候选/挂起」的**分类标签**（终态快照 partition），有时变语义的需求注释，但无 Provisional→Confirmed/Invalidated 转移机制。

### 2.4 结构/对象层的二态碎片（最接近的雏形，均不在背驰事件域）

| 雏形 | 位置 | 活假设侧 | 完成侧 | 证伪侧 |
|---|---|---|---|---|
| `MoveStatus` | decompose.rs:34-39 | `Active`（链尾块仍可变） | `Completed`（前缀不可变） | 无 |
| Lean `ParseState` | formal/Strict/Parse.lean:114-117 | `active : Option ActiveTail`（`provDir` 临时方向 :91，仅预警不交易 :94） | `confirmed : List ConfirmedMove`（:77 交易只用 confirmed） | 无（未来分支集由 `OpenTailSystem.BranchPred` 承载，:95-96，未建形式映射 :337） |
| `CpLifecycleStatus` | recursive_tower.rs:461-464 | `Pending` | `Closed`（advance_cp_lifecycles :1697） | 无终态；dirty 回退 Closed→Pending（:1437,:1461）是依赖维护 |
| `HeldLegState` | persistent.rs:50-60 | `LivePresent`/`LiveDetached` | `Closed` | `Invalidated`（:59，结构或风险显式作废）——**全仓唯一含证伪终态的状态机**，但在持仓域 |

### 2.5 小结（任务②）

代码里**已有**「活假设→完成」的二态雏形（结构块 `MoveStatus`、Lean `ConfirmedMove/ActiveTail` 类型分离、c_p `Pending/Closed`）；**已有**唯一含证伪终态的四态机（持仓域 `HeldLegState`）；但**`NestCandidateEvent` 背驰事件域的「活假设→完成复核→反超失效」三阶段链全缺**——无 Provisional 态、无修订史、无 Invalidated 路径、无 ReviseEvent，turn_class 注释自认时序重判状态机是遗留项（turn_class.rs:87-88）。

## 3. 状态机完成度判定

**判定：部分缺 X——事件域（E2E-D5 宾语）接近全缺，邻域有不可复用的二/四态碎片。**

| 组件 | 状态 | 证据 |
|---|---|---|
| `state: Provisional｜Confirmed｜Invalidated｜Unresolved` 枚举 | **全缺** | §1.1-§1.4：`Provisional` 零命中；`Unresolved` 仅审计计数；`Confirmed/Invalidated` 全是别语义 |
| `ReviseEvent_at` 修订协议（revision/supersedes_revision/转移表/终态禁复活） | **全缺** | `ReviseEvent` rust/formal 零命中；E2E §1:66/:81/:83 仅文档 |
| 转移函数 `advance(prior_state, as_of, CallHashes, Delta)` | **全缺** | 无对应物（`advance_cp_lifecycles` 是 c_p 对象域二态推进，recursive_tower.rs:1697） |
| 终态纪律（`Confirmed/Invalidated` 对同 key 终态、禁删除模拟失效） | **全缺且现状相反** | divergence_confirmed 快照覆盖 + Copy 值语义（§2.1） |
| 事件五钟字段 | **缺 4/5** | 见 §4 |
| 结构完成判定（structure_end 的判据基础） | **已有** | `MoveStatus::Completed`（decompose.rs:38）、`CompletedFreezeEvent`（level_view.rs:478 引用）、段终 `SegmentTermination`（segment.rs:200-205）、Lean `ConfirmedMove`（Parse.lean:79） |
| 暂定/候选表达 | **已有（分类层）** | `ActiveTail.provDir`、`XiaozhuandaCandidate`/`DeferOrphan`（§2.3） |
| 含证伪终态的状态机范式 | **已有（异域）** | `HeldLegState::Invalidated`（persistent.rs:59）——可作范式参考，不可直接复用（持仓域语义） |

## 4. 五钟各钟既有载体盘点

E2E §4.1:144 定义事件五钟：`observed_at / first_provable_at / structure_end_at / confirmed_at / invalidated_at`。

| 钟 | 既有载体 | 判定 | 证据 |
|---|---|---|---|
| `observed_at` | **无独立字段**；`judge_at` 由「prefix 首次观察」写入（level_view.rs:477），`b_center_start` 同款「prefix 首次观察快照纪律」（:496-497） | **缺**（语义粘在 judge_at 上，未分离） | level_view.rs:477-478,:496-497,:712,:786 |
| `first_provable_at` | **零载体**；仅 exit.rs:154-156 v1 预留注释（`reverse_nest_cert_base` 接口预留，「E2E-L 五钟，roadmap:75」）与 exit.rs:180 | **缺** | exit.rs:154-156,:180 |
| `structure_end_at` | **判据已有、钟字段缺**：`MoveStatus::Completed`（decompose.rs:38）、`CompletedFreezeEvent`、段终 `FirstKindConfirmed/SecondKindConfirmed`（segment.rs:200,205）；冻结事件的 `judge_at`（level_view_store.rs:27）是最接近的载体 | **部分**（完成判据在，事件上无独立 structure_end 钟） | decompose.rs:34-39；segment.rs:200-205；level_view_store.rs:27,294 |
| `confirmed_at` | **单钟载体 = `judge_at`**：事件级（level_view.rs:489）+ 证书级向量（nest.rs:417，CERT 首证钟主键 p124_merge.rs:30）；趋势域注释自述「确认时点语义 t*」（level_view.rs:526-527）。注意 bin 终态装配会回填（p92:202 等），与 E2E §1:83「confirmed_at 只在首次进入终态时写入」的纪律存在口径差 | **部分**（单钟在，但与 observed 未分离、有终态回填口径） | level_view.rs:489,:526-527；nest.rs:411-437；p92:202；p116:894-906 |
| `invalidated_at` | **零载体**：事件域无 Invalidated 路径（§1.3）；唯一 `Invalidated` 在持仓域且无时间戳字段（persistent.rs:59）；p95 翻转只计数不记钟（p95_carriedonly_ab.rs:473） | **缺** | §1.3、§2.1 |

## 5. 遗留问题与后续挂点

1. `judge_at` 一钟兼多职（首次观察 + 首证 + bin 终态回填），接入五钟前须先裁决三语义分离——回填点清单：p92:202、p116:303/:894/:906、p123:488、p124_merge.rs:681、p108:356。
2. `NestCandidateEvent` 为 `Copy` 值语义（level_view.rs:479），承载 revision/supersedes 需改事件身份与存储模型——E2E §6.1 `StateKey` 重算纪律（§1:83）目前无对应物。
3. `DeferOrphan` 的「defer 时序重判状态机」注释遗留项（turn_class.rs:87-88）与 E2E-D5 `Unresolved→后续前缀继续修订」语义同源，实装时可合并设计。
4. 持仓域 `HeldLegState` 四态（persistent.rs:50-60）是全仓唯一含 `Invalidated` 终态的状态机范式；其「显式作废 ≠ 快照找不到」（LiveDetached 区分，:53-54）与 E2E-D5「候选身份消失 → Invalidated」（§1:81）语义不同，移植时须注意。
5. Lean 侧 `ConfirmedMove/ActiveTail`（Parse.lean:79-117）已把「当下可确定 / 未来未定」在类型层切开，是 Provisional 语义的形式化锚点；但 `ActiveTail ↔ OpenTailSystem.current` 形式桥接未建（Parse.lean:337,:494），形式化侧同样缺修订/证伪机。
