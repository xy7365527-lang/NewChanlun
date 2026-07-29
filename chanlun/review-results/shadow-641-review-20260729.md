# 影子评审：#641 N3 级别链证书 A 侧（两轴，新上下文禁自评）

票：#653（父票 #529）　日期：2026-07-29　评审工位：Opus 影子评审（前台单线程，无子代理）
对象：分支 `ticket-641b` @ `d5e55925a6`（= `4fa468711d` 初装 + `46c4c30dd3` 报告 + `d5e55925a6` 修复轮收口）
基座：`ticket-553`（`654ee7a9c3`）
性质：**只读评审**——未改工位任何文件（`git status --porcelain` 空），只写本报告。

## 结论：**PASS WITH CONDITIONS**

裁定符合性、读数复现、护栏、测试门四条硬轴全部通过且逐项独立实测复现；两条 MEDIUM 属
「声明与实际不一致」（090），均不影响判定路径、不影响已交付读数，故列为收编条件而非阻断。

| 轴 | 判定 | 依据 |
|---|---|---|
| Spec — #636 五条 | **符合** | §一.1 逐条 |
| Spec — 地板合取（comment-5121572134） | **符合** | §一.2，实证等价于 `!edges.is_empty()` |
| Spec — 取舍裁定（comment-5121793896）补三件 + 移植三件 | **符合** | §一.3 / §一.4 |
| 读数复现（BTC 三窗） | **逐字段全中** | §二，本评审独立复跑 |
| 护栏（p116/stdout ×4、m8 ×6、链 dump 差异面） | **全通过** | §三，p123 20k 三方 SHA 相等 |
| 测试门 | **通过** | §四，2081/1(#491，基座同红)/135；+4 新测试全绿；零新增红 |
| Standards（结构/命名/文档/错误处理/体量/fmt） | **符合** | §五 |
| 永禁清单 + #449 方向 | **未命中 / 符合** | §六 |

**收编条件（两条，均可一行/一段修，不必重跑读数）**：

- **MED-1**：`chain_probe::floor_blocked` 在「事实边判死」场景假计数（实证）——探针条件补 `&& all_segments`。
- **MED-2**：报告头部修订登记「§五 的 BTC 读数**未改**（逐字段复现）」在 `extends` 一格不成立——
  §五 5.4 的 `extends` 列（5/12/33）现行实测为 0/0/0，须就地标注 supersede 指向 §九 9.3。

---

## 一、裁定符合性逐条

### 1.1 #636 Resolution 五条（comment-5119573446 + 2026-07-29 核定/supersede 登记）

| # | 条款 | 判定 | 代码/测试锚 |
|---|---|---|---|
| 1 | 节点 = N1 候选事件，身份键沿用，不新造身份 | **符合（类型层）** | `ChainKey.path: Vec<CandidateKey>`（`chain_cert/mod.rs:281-284`）；全模块 grep 无 `CandidateKey {` 构造点（测试夹具除外） |
| 1 | 边 = N2 `C⊆C` 谓词 | **符合** | `build_edge` 唯一判定来源 `candidate_is_sub(child, parent)`（`mod.rs:659`） |
| 1 | nest 对照侧不动（ADR-0005） | **符合** | `grep -nE "nest\|Nest" chain_cert/*.rs` → 仅 2 处文档正文（`mod.rs:10-11`），零类型、零调用 |
| 2 | 谱系照实：缺失留痕 | **符合（加严）** | `SkippedLevel{level, alive_at_level, inside_parent}` 三格（`mod.rs:164-171`）；测试 `skipped_level_separates_absent_level_from_broken_level` |
| 2 | 谱系照实：证伪留痕 | **符合** | `ChainNodeStatus::Falsified` 原样留在 `nodes` + `ChainEdge.crossed_nodes` 指名（`mod.rs:143-156, 255-256`） |
| 2 | 谱系照实：跳过留痕 | **符合** | `ChainEdgeKind::Skip` 由两存活端点级别差派生（`mod.rs:641-645`），非人工标记 |
| 2 | **禁伪造中间级证书** | **符合** | 同第 1 条（类型层成立） |
| 2 | 链段有效性一律由 `C⊆C` 在**存活端点**间判定 | **符合** | `evaluate()` 先算 `alive_positions`，边只在相邻存活端点间建（`mod.rs:572-582`） |
| 2 | `Closed` = 链头 Confirmed + 不可再扩展 + 全链段判过（+ 地板补） | **符合** | `mod.rs:606` 四合取 `head_confirmed && !extendable && has_segment`（`all_segments` 在前一臂已保证） |
| 2 | `Invalidated` = 谓词判不过 **或** 链头 Invalidated；不复活 | **符合** | 两支恰对应（`mod.rs:596-605`）；`apply()` 入口终态挡回（`mod.rs:871-879`）；`terminal_status_never_revives` |
| 2 | **不含连坐支**（2026-07-29 supersede 的原型文本） | **符合，且有反事实锁** | `ChainInvalidationCause` 恰两档（`mod.rs:118-130`）；`invalidation_cause_has_only_the_two_ruled_branches` 断言中间节点证伪 ⟹ 链仍 `Open`、成因 `None` |
| 3 | 谓词判过的 skip 边与相邻边**同权** | **符合** | `is_segment()` 只读 `predicate_holds`，不读 `kind`（`mod.rs:265-270`）；`skip_edge_and_adjacent_edge_are_equally_segments` 直接断言两者相等 |
| 3 | 判不过记**事实边**、不构成链段 | **符合** | 边照实留在 `edges`、`fact_edge_count()` 可读；`predicate_failure_becomes_fact_edge_and_invalidates_chain` |
| 3 | 证伪节点**留痕但不判死上级** | **符合** | 两侧存活端点间新长出 skip 边，链继续由谓词裁；`falsified_middle_node_is_crossed_and_does_not_kill_the_head`（链保持 `Open`） |
| 3 | 44 课小转大证据路径留 fog | **符合（并登记后果）** | skip 边不携带任何替代证据；报告 §七登记 2 明写「本实装的 skip 边比 44 课教义门槛宽」 |
| 4 | N7 只收 Closed（本票纯产出零消费） | **符合** | 独立 grep：`backtest` / `strategy` / `trading` / `bsp.rs` **零引用**；全仓引用点 = 2 个诊断 bin + `classifier/mod.rs:94` 模块声明 + `#[cfg(test)]` 测试段 |
| 5 | 命名独立（与 `NestCertificate` 分家） | **符合** | `TowerChainCertificate` / `ChainKey` / `ChainCertificateBook`；模块头 `mod.rs:8-13` 专段说明分家理由；不共享类型、不互相调用 |

### 1.2 地板条款（#641 comment-5121572134）

**判定：符合。**

- 落地：`mod.rs:593` `has_segment = edges.iter().any(is_segment)`，进 `Closed` 臂（`mod.rs:606`）。
- **语义等价性核验**：`all_segments`（空集真空真）在前一臂已判过；故 `has_segment` 唯一起作用的场合
  恰是 `edges` 为空，等价于对比评审 M1 建议的 `all_segments && !edges.is_empty()`。
- **「链头独活」覆盖完整性核验**：`edges` 为空 ∧ `head_alive` ⟹ `alive_positions` 只有一个元素
  ⟹ 该元素必是链头（否则 `head_alive` 假、已在第一臂判 `Invalidated`）。即「零链段」与「链头独活」
  在本实装下互为充要，无漏格。
- 「空集不真空」机器载体三重：单测 `head_only_survivor_stays_open_by_floor_conjunct`（断言
  `Open` + `closed_at == None` + `floor_blocked > 0`）+ 读数格 `closed_with_zero_segments`（三窗实测 0）
  + 主缝 golden 内 `closed_with_zero_segments == 0` 断言。
- 反事实锁 `floor_conjunct_does_not_block_a_chain_that_has_a_segment` 有效：它断言 L2→L1 两节点链
  （`leaf_level == 1`，未降到 L0）可 `Closed`，故若误把地板写成级别地板会当场变红。**该锁真实有效**。
- ⚠ 探针一侧有假计数，见 **MED-1**（不影响判定）。

### 1.3 补三件（取舍裁定第 3 条）

| # | 判定 | 核验 |
|---|---|---|
| ① 地板合取 | **符合** | 见 §1.2 |
| ② 测试翻转改名 | **符合（090 口径正确）** | 旧名 `head_only_survivor_closes_by_vacuous_segment_condition` 已消失（全仓 grep 零命中），新名 `head_only_survivor_stays_open_by_floor_conjunct`；断言由 `Closed` 翻为 `Open`；旧文档注释整段替换为裁定字面。**改名理由成立**：旧名把被裁定否掉的后果写进名字，留名即声明与实际不一致；新旧名对照已在报告 §九 9.1② 登记，票面点名的那一枚可追溯 |
| ③ `extends` 修复 | **符合，三重载体齐** | `ChainKey::proper_prefix()` 已删（全仓零命中），改私有 `ChainKey::prefix(len)`（`mod.rs:317-322`）+ 簿侧 `resolve_extends()`（`mod.rs:824-832`）；解析点从纯函数 `evaluate` 挪到落簿处 `apply`（`mod.rs:880`）——「纯函数」声明因此为真；`ChainObservation` 已删 `extends` 字段。负控 `extends_is_none_when_the_structural_prefix_never_materialized` 通过；命中侧探针断言 `(1,0)`；主缝非真空锁 `extends_not_materialized > 0`。**幽灵 5/12/33 全清**：本评审三窗独立实测 `extends_some=0/0/0`（见 §二） |

**反事实锁独立核验**（`extends` 负控）：该测试构造三节点链一次成型、真前缀从未落簿，断言
`extends == None` 且探针 `(0,1)`。若回退到纯结构派生，`extends` 会写成 `Some([L2,L1])` ⟹ 当场变红。**锁有效**。

### 1.4 移植三件（取舍裁定第 4 条）

| # | 判定 | 核验 |
|---|---|---|
| P1 成因两档 | **符合** | `ChainInvalidationCause` 恰 `HeadInvalidated` / `PredicateFailed` 两档；**与 `invalidated_at` 同步一次写入**——`make_revision` 中两者同款 `prior.and_then(..).or(..)`（`mod.rs:991-996`），核验其不变量 `cause.is_some() ⟺ status == Invalidated` 成立（`evaluate` 只在 `Invalidated` 两臂给 `Some`，终态挡回保证 prior 不会带旧 cause 进非终态）；**不进 projection**——`ChainProjection`（`mod.rs:363-372`）无该字段 ✓。连坐支 `NodeInvalidated` / `NodeAbsent` 均未移植 ✓ |
| P2 `digest()` | **符合（口径同源），一处顺序脆弱** | `summarize()` / `digest()` 为簿内方法（`mod.rs:903-976`）；主缝 golden 改调 `book.digest()`（`classifier/mod.rs:4419`）；bin 自写循环整段删除改读 `summarize()`。**同源声明核**：报告 §9.5 三个 digest 值 = bin 打印值 = 本评审实测值，逐字相同（§二）。⚠ 顺序脆弱见 **LOW-1** |
| P3 `PredicateBreach` 五档 | **符合，分档口径经对照核验** | `PredicateBreachReason` 五档（`mod.rs:198-210`）；**与 `interval_is_sub` 的合取式逐项对照**：`!deg(child) ∧ !deg(parent) ∧ parent.0 <= child.0 ∧ child.1 <= parent.1`（`cand_event.rs:158-162`）↔ `ChildIntervalDegenerate` / `ParentIntervalDegenerate` / `LeftOverhang` / `RightOverhang`(+`BothEndsOverhang`)，**求值序一致、无口径漂移** ✓。`(false,false)` 的 `unreachable!` 论证成立（两端不越界 + 两区间非退化 ⟹ 谓词真，与调用前提矛盾）。级别分支不设档、用 `unreachable!` + 推导链——推导链三步（覆盖边构造 ⟹ 路径级别严格递减 ⟹ upper/lower 位序）**核验成立**。⚠ 耦合无锁见 **LOW-2** |

---

## 二、读数复现（BTC 三窗，本评审独立复跑）

命令：`CARGO_TARGET_DIR=/tmp/kimi-nest-target-653`，release，
`issue550_event_battery /private/tmp/kimi-nest-mainline/analysis/data_cache/btc_1m_full.json <bars> <every>`。

| 窗口/节拍 | 链数 | rev | Open | Closed | Inval | 边 | adj | skip | skip 占比 | fact | `closed_w_zero_seg` | `inval_head`/`_pred` | `extends_some` | 幂等 Δ | digest |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 20k / 2000 | 18 | 18 | 0 | 18 | 0 | 23 | 14 | 9 | 0.3913 | 0 | **0** | 0/0 | **0** | 0 | 11721078737383519109 |
| 100k / 5000 | 89 | 99 | 15 | 74 | 0 | 101 | 60 | 41 | 0.4059 | 0 | **0** | 0/0 | **0** | 0 | 6645041198801530824 |
| 300k / 25000 | 293 | 480 | 55 | 237 | **1** | 325 | 191 | 134 | 0.4123 | 0 | **0** | **1**/0 | **0** | 0 | 2200354287177549197 |

留痕分解实测：20k `alive=41 / falsified=0 / absent=0 / missing=0 / broken_out=0 / broken_in=9`；
100k `190/0/0/0/6/35`；300k `618/1/0/0/73/96`。路径长分布 20k `{2:13,3:5}`、100k `{2:77,3:12}`、
300k `{2:260,3:33}`；链头级别 20k `{1:4,2:14}`、100k `{1:36,2:53}`、300k `{1:131,2:116,3:46}`。

**判定**：

1. **三窗与报告 §五 / §9.3 / §9.5 逐字段相同**（含三个 digest 逐位相同）——读数可复现 ✓。
2. **地板合取零变化声明成立**：`closed_with_zero_segments = 0`（三窗），链数/三态/边形态/skip 占比
   与交付快照 §五 完全一致 ⟹「地板合取在 A 的数据面上零变化」为真 ✓。
3. **300k 唯一 `Invalidated` 死因机器直读**：`invalidated_head=1 / invalidated_predicate=0`，
   佐以 `nodes_falsified=1 ∧ crossed_nodes=0 ∧ fact_edges=0` ⟹ 死因 = **链头证伪**，
   与裁定②第二支对应 ✓。该归因不再依赖推理，由 P1 成因分档直接给出 ✓。
4. **幽灵 `extends` 已全清**：三窗 `extends_some = 0/0/0`（对照 §五 5.4 的 5/12/33）✓。
5. **幂等在真实数据上成立**：三窗 `idempotent_replay_delta = 0` ✓。
6. **300k 耗时** `real 2m45s`（报告称 2m57s，同量级，机器负载差异）；500k 未复跑（沿用报告登记，
   瓶颈在既有电池三节，本评审未验证该归因）。

---

## 三、护栏

| 面 | 判定 | 证据（本评审实测） |
|---|---|---|
| p123 stdout / P116 dump（20k、100k）pre↔post | **cmp=0 ×4** | `20000.p116` `fcc8016a9a01a109`、`20000.stdout` `bd9ac1d655f9d615`、`100000.p116` `8a7327feb3b9ba29`、`100000.stdout` `d8b69c180c23c5e3` |
| **产物可信度（三方核）** | **通过** | 本评审用**当前 HEAD 树**独立 release 复跑 p123 20k：`p116` 与 `stdout` 与实施侧 pre、post **三方 cmp=0**，SHA 分别 `fcc8016a9a01a109` / `bd9ac1d655f9d615`，与报告 §八声明的前 16 位逐字相同 ⟹ 实施侧产物确来自本树，非自报 |
| m8 三窗 `trades.jsonl` / `tower_events.jsonl` | **cmp=0 ×6** | `p3fold` `2da686833581d535`/`83f45a36ab422a92`；`wf7` `3371f1e62153e8aa`/`aa96b3e836be5a43`；`wf8` `006c31f54cd72d8e`/`1d8dff0d29e8925f` |
| 跨票一致声明 | **成立** | 上述六个 SHA 与 `chanlun/review-results/issue527-panlive-l1-provider-20260728.md:143-145` **逐字相同** ✓ |
| p123 链 dump 差异面 | **成立** | 20k：19 行中 5 行差异；100k：100 行中 12 行差异；**抹掉 `extends=` 字段后两侧 diff 为空行数 = 0**（两窗）⟹「差异全部且仅在 `extends=`」为真 ✓ |
| p123 stderr | 唯一差异 = `prefix_s`（耗时），非判定内容 | 本评审复跑对照 |
| 生产面零引用 | **成立** | 独立 grep，见 §1.1 第 4 条 |
| nest 零触碰 | **成立** | 见 §1.1 第 3 条 |

---

## 四、测试门（本评审自跑）

```text
工位 ticket-641b @ d5e55925a6：cargo test --lib
  → 2081 passed / 1 failed / 135 ignored（合计 2217）
  唯一红 = theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard（#491）

基座 ticket-553 @ 654ee7a9c3（git archive 到 /tmp/wt653-base 独立构建）：
  → 2058 passed / 1 failed / 135 ignored（合计 2194）  ※见下方 flaky 说明
  唯一红 = 同一枚 #491 digest guard

增量 = 2217 − 2194 = 23 枚，恰为 21 chain_cert::tests + 2 classifier::tests（零第三方混入）
cargo test --lib chain_cert  → 23 passed / 0 failed（含 2 枚 classifier 主缝）
cargo test --bin p123_fast_replay → 9 passed / 0 failed
```

**判定**：

- **唯一红 = #491 digest guard，基座同红** ✓。因果面独立核实：本分支 diff **未触及** `signal.rs`
  及任何生产文件（`git diff 654ee7a9c3..HEAD --name-only` 仅 6 个文件，无 signal/bsp/backtest/strategy/trading）。
- **零新增红** ✓。**+4 新测试全绿** ✓（`floor_conjunct_does_not_block_a_chain_that_has_a_segment`、
  `invalidation_cause_has_only_the_two_ruled_branches`、`extends_is_none_when_the_structural_prefix_never_materialized`、
  `summarize_and_digest_are_book_internal_readouts`）。
- 报告 §9.5 自报 `2081 / 1 / 135` 与 FNV golden `9935805022530767834`、三个 BTC digest 全部实测吻合 ✓。
- 报告 §9.4 登记 5（#491 在 main 已收口、ticket 链未同步；「基线已全绿」的前提对 main 成立、对本基座不成立）
  **照实、不代过** ✓。
- ⚠ **INFO-1**：首次基座全量跑得 `2057 passed / 2 failed`，多出的
  `theta_v0::classifier::divergence::tests::incremental_macd_append_is_o1_scaling` 为**负载敏感 flaky**
  ——单独复跑 `ok`。该枚在并发编译/测试负载下会假红，与本票无关；登记以免后续核验被误导。

---

## 五、Standards 轴

| 项 | 判定 | 依据 |
|---|---|---|
| 模块结构 | **符合** | `chain_cert/` 目录，生产（`mod.rs`）与测试（`tests.rs`）分文件；`#[cfg(test)] mod tests;` |
| 体量 | **符合** | `mod.rs` 1170 行（其中 357 行为文档/注释）、`tests.rs` 822 行；同层对照 `cand_event.rs` 1417、`nest.rs` 3148 ⟹ 未越同类上限。（对比评审提到的「几何层/谱系层未分家」属可移植重构，非缺陷） |
| 命名 | **符合** | 与 `NestCertificate` 全面分家；模块内命名自洽（`ChainStatus`/`ChainEdgeKind`/`ChainNodeStatus`/`ChainInvalidationCause`/`PredicateBreachReason`） |
| 文档 | **符合（高于均线）** | 模块头 73 行语义总纲逐条对裁定 + 地板条款专段 + 构造口径论证；每个 `pub` 项有 doc；两处 `unreachable!` 均带**可核对的推导链**而非「应该不会发生」 |
| 错误处理 | **符合** | 编程错误当场失败（`ChainKey::new` 的 ≥2 断言 + 单测 `single_node_path_is_not_a_chain`）；bin 侧 `Result`+`?` 传播，env 解析失败不默认化（`P123_CHAIN_DUMP_EVERY` 非法 ⟹ `Err` 带原始串）；dump 写失败传播口径与 `EventDump` 同款并已登记 |
| 探针纪律 | **符合（一处失真）** | `chain_probe` 常开、调用点 `#[cfg(test)]` ⟹ 生产零开销；含「恒 0 断言」`birth_invalidated` 作构造口径锁。⚠ `floor_blocked` 见 MED-1 |
| `rustfmt` | **干净** | 本评审自跑 `rustfmt --check --edition 2021`：`chain_cert/{mod.rs,tests.rs}` + 两个 bin **全部干净**；`classifier/mod.rs` 未单文件校验（沿用 #552 §七登记 5 的既有处置，理由成立：仓库整体非 fmt-clean） |
| commit 卫生 | **符合** | 三个 commit 逐个可编译（本评审在 `d5e55925a6` 全量跑通；`4fa468711d` 为完整实装、`46c4c30dd3` 纯文档）；message 逐条对裁定并点名锚 |
| 工位干净 | **符合** | `git status --porcelain` 空 |

---

## 六、永禁清单 + #449 方向

| 项 | 判定 | 依据 |
|---|---|---|
| #449 判据层禁引 nest 产物（裁定：**维持**，收窄保留，ADR-0005） | **符合** | `chain_cert` 零引用 nest 类型；本模块**纯产出零消费**，不回写/过滤/改判任何历史锚点的 `Classification`/`BspBits`；生产面零引用 |
| 禁伪造中间级证书（roadmap:64） | **未命中** | 类型层不可构造 `CandidateKey`；且边取覆盖关系而非传递闭包，避免「把在场且套得住的中间级说成跳过」这一镜像违规（机器锁 `edges_are_cover_relation_not_transitive_closure`） |
| 禁删除模拟失效 / 禁改写旧 revision | **未命中** | `certificates: Vec<..>` 只 `push`，无删除路径；`revisions_are_append_only_and_node_growth_alone_is_not_a_chain_revision` 断言旧 revision 原样 |
| 禁终态复活 | **未命中** | `apply()` 入口挡回 + `terminal_status_never_revives`（几何恢复也不复活）+ `on_append` 的「终态→Open」`unreachable!` |
| 禁静默丢格 | **未命中** | 孤立候选单列计数（`isolated_roots`，`isolated_candidates_are_counted_not_silently_dropped`）；被跳过级别三格分解；`Absent` 与 `Falsified` 不混同 |
| 禁靠注释声明条款 | **基本未命中** | 全部裁定条款有单测/探针/读数格载体。唯一例外见 MED-1（探针本身失真，条款本体仍由单测锁住） |
| 禁静默采样/截断 | **未命中** | 推进节拍 `advance_every` 随读数显式印出；`chain_paths` 无上限无采样，复杂度照实登记（§七登记 7） |
| 090 照实 | **基本符合** | §七 10 条 + §9.4 8 条登记覆盖了未做项、超时、口径后果与权限边界；#491 基线红照实不代过。两处措辞瑕疵见 MED-2 / LOW-2 |

---

## 七、新发现（分级 + 锚）

### MED-1　`floor_blocked` 探针在「事实边判死」场景假计数（声明≠实际）

**锚**：`rust/src/theta_v0/classifier/chain_cert/mod.rs:614-617`（探针条件）
vs `mod.rs:1061-1066`（探针文档）。

探针文档写的是「★地板条款拦下的 `Closed`：链头存活且 `Confirmed`、不可再扩展，但**链段集合为空**
⟹ 判 `Open` 而非 `Closed`。非零 = 地板合取真被走到」。但触发条件

```rust
if head_alive && head_confirmed && !extendable && !has_segment { on_floor_blocked(); }
```

**缺 `&& all_segments`**：当链有事实边（`all_segments == false`）时，链在前一臂已判
`Invalidated(PredicateFailed)`——地板合取根本没参与判定——探针却仍 +1。

**实证**（本评审在 `/tmp/wt653-probe`，工位 archive 副本，工位零改动）：构造「轮1 链头未确认 ⟹ Open；
轮2 链头转 `Confirmed` + 子端点右端越出父区间」，实得

```text
status=Invalidated cause=Some(PredicateFailed) segments=0 fact_edges=1 extendable=false floor_blocked=1
```

**影响**：判定路径零影响（探针仅 `#[cfg(test)]`，不参与任何判据）；BTC 三窗 `fact_edges=0`
⟹ 实测 `floor_blocked=0`，报告 §9.4 登记 6「探针在真实数据上恒 0」仍为真。但该探针是地板条款的
**机器载体之一**（报告 §9.1① 三重载体的中间一重），其「非零 = 地板合取真被走到」这条声明在有事实边
时会假阳性；报告 §9.4 登记 6 末句「若将来某窗口出现非零，属真实触发，须照实记而不是当异常」会因此被误读。

**修法**：探针条件加 `&& all_segments`（或直接用 `status == ChainStatus::Open` 作条件）。一行。
现有单测 `head_only_survivor_stays_open_by_floor_conjunct` 的 `floor_blocked > 0` 断言不受影响
（该场景无事实边）。

### MED-2　报告头部修订登记「§五 读数未改／逐字段复现」在 `extends` 一格不成立

**锚**：`chanlun/review-results/issue641-n3-chain-impl-20260729.md:9-10`（修订登记）
vs `:230-234`（§五 5.4 表格 `extends` 非空列 = 5 / 12 / 33）vs `:390`（§9.3 照实读第 2 条「5/12/33 → 0」）。

修订登记称「§五 的 BTC 读数**未改**（修复轮三窗逐字段复现，见 §九 9.3）」，而 §9.3 自己的表述是
**限定字段的**——「三窗链数/三态/边形态/skip 占比与 §五逐字段相同」，并明写 `extends_some` 由
5/12/33 归零。本评审三窗独立实测 `extends_some = 0/0/0`，即 **§五 5.4 的 `extends` 列是现行代码
跑不出来的数**，而 §五 表格没有任何就地 supersede 标注。

**影响**：正确值与成因在同一份报告的 §9.3 有完整说明，故不构成事实错误；但单读 §五（例如 N7 立案时
按 §五 取容量口径）会取到过期数，且修订登记的「未改／逐字段复现」措辞把这一格也包了进去。属 090
「声明与实际不一致」。

**修法**：§五 5.4 该列就地加删除线或脚注「★修复轮后为 0，见 §九 9.3」；头部修订登记把「未改」收窄为
「除 `extends` 列外未改」。纯文档，不必重跑读数。

### LOW-1　bin 中 `digest` 与 `summarize` 不同源于同一簿状态（顺序脆弱）

**锚**：`rust/src/bin/issue550_event_battery.rs`，`print_chain_summary` 内
`let summary = book.summarize();` → `let replay = book.advance(..).len();` → 末行 `digest={}` 打印 `book.digest()`。

`summarize()` 取自幂等重跑**之前**的簿，`digest()` 取自重跑**之后**的簿。当前三窗
`idempotent_replay_delta = 0` ⟹ 两者同态、报告数无误。但一旦幂等破，`digest` 会静默包含重跑追加的
revision，而同一行打印的 `summary` 不含——「报告数 / golden / bin 读数三处同源」（§9.2 P2）的声明
会在最需要它的那一刻失真，且没有任何断言会报警。

**修法**：把 `digest` 与 `summarize` 一并在 `advance` 之前取（或对 `replay != 0` 显式告警）。

### LOW-2　`breach_reason` 与 `interval_is_sub` 的口径耦合无机器锁

**锚**：`chain_cert/mod.rs:686-725` vs `cand_event.rs:153-162`。

`breach_reason` 的文档称「分档不是重判……本模块不复算任何不等式，只读同一组端点值」，但代码实际
**逐项复写**了与 `interval_is_sub` 相同的四个不等式。本评审逐项对照确认**当前口径与求值序完全一致**、
分档正确（`RightOverhang` 一档另有单测断言）。风险在耦合无锁：若将来 `interval_is_sub` 口径变动
（相切规则、容差、半开区间），`breach_reason` 会静默漂移成错误归因，而没有任何测试会红。

**修法**（任一）：把分档表述改为「与 `interval_is_sub` 的合取式逐项对应（口径变更须同步）」并加一枚
一致性测试（对随机/边界端点组，断言 `!candidate_is_sub` ⟺ `breach_reason` 命中的那一项确为假）。

### INFO-1　基座 `incremental_macd_append_is_o1_scaling` 负载敏感假红

见 §四。本票无关，登记以免后续「基线红数」核验被误导。

### INFO-2　链头 `Absent` ⟹ 终态判死且不复活（消费端风险）

**锚**：`mod.rs:596-600`（`!head_alive` ⟹ `Invalidated`）+ `mod.rs:149-155`（`Absent` 可达域）。

`Absent` 只在「终态窗口投影」（每次 fresh book）驱动下可达；该驱动下一次身份查无就把链永久判死
（终态不复活）。报告 §9.4 登记 1 已照实登记「裁定②只授权『链头 `Invalidated`』一支，故不为 `Absent`
另开档；若认为应独立成档属新裁」。**本评审同意不自决**，仅提示：若 N7 采用 fresh-book 驱动，须知这条
判死路径的语义与因果簿驱动不同。

### INFO-3　`CONTEXT.md` 词表条目仍未收编

§七登记 1 + §9.4 登记 8 已备好条目文本（含地板条款与 `extends` 簿内口径的补字），文件只在 `main`，
本票权限外。**待编排侧在 main 上落**。

---

## 八、090 照实登记（本评审自身的限度）

1. **未复跑的面**：500k 窗口（沿用报告登记，未验证「瓶颈在既有电池三节」这一归因）；
   m8 六个产物与 p123 100k 产物为**抽对**（cmp + SHA 对照实施侧产物），未逐个独立复跑——
   但 p123 20k 已用当前 HEAD 树独立复跑并与 pre/post **三方 cmp=0**，故实施侧产物的来源可信度已锁。
2. **本评审的临时验证副本**：`/tmp/wt653-base`（基座 archive）、`/tmp/wt653-probe`（HEAD archive +
   一枚临时探针测试）、`/tmp/wt653-verify-20000.*`（p123 独立复跑产物）。
   **工位 `/tmp/wt-641b` 零改动**（`git status --porcelain` 空，评审前后一致）；
   `/tmp/wt-641`、侧 ref `ticket-641-kimi-line` **零接触**。
3. **B 侧未复核**：本评审按票据只作对照引用，未复跑 B、未核对比评审的 B 列读数。
4. **未评审的面**：`chain_paths` 的 DFS 在病态输入（宽扁包含格）下的规模上界（报告 §七登记 7 已照实
   登记「无上限、无采样、无截断」，本模块不在热路径）；`digest()` 对 `format!("{:?}")` 的内存占用
   （480 revision 量级无问题，更大簿未测）。
5. **测试门的红**：`extract_signals_bit_exact_digest_guard`（#491）在本工位基座与交付面同红，
   收口在 main 侧、跨票，**本评审不建议在本票内动** `signal.rs`。

---

## 九、收编建议（供编排者裁）

1. **MED-1 / MED-2 收编后即可 PASS**：两条都是一行代码 / 一段文档，不改判定、不必重跑读数与护栏。
   若编排者认为探针失真与文档措辞不构成阻断，可直接判 PASS 并把两条转为跟进项。
2. **LOW-1 / LOW-2 建议一并收**（同一修复轮，成本极低），但不作条件。
3. **INFO-3（`CONTEXT.md` 词表）归口 main 线**，与本票交付面无关。
4. **floor 完整口径（`CloseFloor_at` 三型级别地板）**：本评审确认实装**未越权自补**级别分量，
   反事实锁有效，留 fog 处置正确——维持取舍裁定第 5 条的另裁安排。
