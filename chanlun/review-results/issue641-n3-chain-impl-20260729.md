# #641（N3）级别链证书塔对象——实施报告

日期：2026-07-29　票：#641（父票 #529）　裁定：#636 Resolution（comment-5119573446）
基座：`ticket-553`（654ee7a9c3，N1/N2 四票链尾）　交付分支：`ticket-641b`　工位：`/tmp/wt-641b`

> **修订登记（2026-07-29 修复轮）**：本报告在 `4fa468711d` 交付后，按 #641 地板裁定
> （comment-5121572134）与两套 N3 取舍裁定（comment-5121793896）追加了 **§九 取舍裁定执行节**，
> 并就地改写了受裁定影响的 5 处：§一.3 第二条边界条件、§二「Closed / Invalidated / 路径扩展」
> 三行、§三② 测试清单、§六 D2 / D3 / K2 / S8 四行。§五 的 BTC 读数**未改**（修复轮三窗逐字段复现，
> 见 §九 9.3）；§七 的 10 条登记**未改**（本轮新增登记接在 §九 9.4）；§八 的三元组是交付当时的
> 快照，现行值见 §九 9.5。

---

## 〇、必须先读：工位冲突登记（照实，未自决）

**票面指定的工位分支 `kimi-nest-mainline-20260717` 上没有 N1/N2 基座。** `cand_event.rs`（#550/#551
候选事件对象 + E2E-O 状态机）与 `cand_sub.rs`（#552 C⊆C 谓词）只存在于 `ticket-550→551→552→553`
这条链上，`git merge-base --is-ancestor ticket-553 kimi-nest-mainline-20260717` 判否。#641 的验收
③明写「事件流 FNV golden 锁」「p123 事件 dump **扩一路**（N1-T4 口径）」，两者都要求基座在场；在
主线上做只能重写一遍 N1/N2（090 禁重复实装）或 merge 四条分支（票面禁 merge）。故按前四票惯例
（`/tmp/wt-NNN` + `ticket-NNN`）取链尾 `ticket-553` 为基座。

**其后发现同一张票有第二个 session 并发在做。** 本工位最初开在 `/tmp/wt-641`（分支 `ticket-641`）；
工作过程中该路径/分支被另一 session 用于同一张 #641，并已提交两枚 commit：

| commit | 内容 | 规模 |
|---|---|---|
| `6ec57e760b` | `#641 N3-T1 级别链几何构造层` → `cand_chain.rs` | 767 行 + mod.rs 6 行 |
| `442d9589b7` | `#641 N3-T2 链谱系簿 + 终态状态机` → `cand_chain_book.rs` | 1072 行 + mod.rs 3 行 |

且 `6ec57e760b` 的 mod.rs 6 行里**有 2 行是本实装的模块声明**（`pub mod chain_cert;` 及其 doc），
被其 `git add` 一并吞入。

**本实装的处置**：不在 `ticket-641` 上叠第二套实装、不回退对方 commit、不删对方文件。把本侧产出整体
移到独立分支 `ticket-641b`（基座同为 `ticket-553`），`/tmp/wt-641` 的对方工作面原样不动（本侧遗留在
那里的未提交改动亦未强行清理——清理会与对方可能正在进行的编辑竞态，属不可逆动作，留给编排侧裁）。

**待裁（本实装不自决）**：两套 N3 实装（`cand_chain.rs`+`cand_chain_book.rs` vs `chain_cert/`）如何
收口——保留哪一套、是否合并、`ticket-641` 上被吞的 2 行如何处理。这是「选择」类，超出实装权限。

---

## 一、结果包（六要素）

1. **结论**：新建塔内原生级别链证书对象 `TowerChainCertificate`（模块
   `rust/src/theta_v0/classifier/chain_cert/`），节点 = N1 候选事件身份键，边 = N2 `C⊆C` 谓词的
   **覆盖关系**，谱系走 E2E-L 三态（`Open`/`Closed`/`Invalidated`）+ skip edge + append-only 修订；
   缺失/证伪/跳过全留痕；纯产出零消费。
2. **定义依据**：#636 Resolution 五条（形态/终态语义/skip 语义/N7 朝向/命名）；ADR-0006（Sub = C⊆C
   同口径）；ADR-0007（append-only + 终态不复活 + `extends_lineage_key`）；#246（相切算包含，全域）；
   E2E-L 冻结文本（roadmap:64、原型 §5:188、原型 §6.1 状态机段，git 锚 `640609071d`）；027:46（逐级
   收缩至最低级别）；032:227 + 044:16（逐级非必然、命名的跨级路径）。
3. **边界条件**（结论在什么条件下翻转）：
   - 若裁定改判「链段有效性看边的级别形态」（skip 边不同权），`ChainEdge::is_segment` 与终态判定翻转；
   - ~~若裁定给回 E2E-L 的 `CloseFloor_at` / `UnresolvedFloor`（本裁定未携带，见 §六对照表 D3），
     「只剩链头存活且不可扩展 ⟹ 可 Closed」这一条照实后果翻转~~ ——**该条件已于 2026-07-29 兑现**
     （#641 comment-5121572134 地板裁定），翻转已落地，见 §九 9.1①②；剩余的 floor 级别分量仍留
     fog（翻转条件未消失，只是缩小到「若裁定再给回**级别**地板」）；
   - 若裁定要求 skip 边携带 44 课「小转大」替代必要条件（次级别中枢第三类买卖点），本实装的 skip 边
     语义偏宽，须加证据字段（#636 裁定③已明确**留 fog**，本票不做）；
   - 若「边 = 覆盖关系」被改判为「边 = 全部包含对」，链数按路径长度组合放大，且在场且套得住的中间级
     会被记成「跳过」。
4. **下游推论**：N7 若「只收 Closed 谱系」，其可消费量 = 本对象 `status == Closed` 的 head 集合；
   本报告 §五给出该集合在 BTC 上的实测规模与形态分布，可直接作为 N7 立案的容量口径。链的伤疤
   （证伪节点、skip 边、事实边）全部随对象可见，判断留给消费端。
5. **谱系引用**：ADR-0005（nest 独立对照身份 ⟹ `NestCertificate` 不是本对象的范本，命名必须分家）、
   ADR-0006、ADR-0007、#535 四类非法近似红线、#246、#636 研究报告
   `issue636-chainline-skip-edge-research-20260728.md`（32/44/100 课与 E2E-L 一手锚）。
6. **影响声明**：新增 `chain_cert/{mod.rs,tests.rs}`；`classifier/mod.rs` 加模块声明 + 一个只读取数口
   `TowerCache::candidate_streams()` + 两枚测试；两个诊断 bin 各扩一路只写不判的旁缝。**生产面零引用**
   （backtest / strategy / trading / bsp 全零，见 §三③）。既有 Classification / 塔快照 / BSP / 订单流
   逐字节不动（p123 双跑 SHA 相等，见 §三③）。

---

## 二、语义逐条对 #636 裁定

| 裁定条款 | 实装载体 | 说明 |
|---|---|---|
| 节点 = N1 候选事件（身份键沿用） | `ChainKey.path: Vec<CandidateKey>` | 本模块**任何路径都不构造** `CandidateKey`；节点只能来自事件流查得的 key ⟹ 「禁伪造中间级证书」在类型层成立 |
| 边 = C⊆C Sub 包含（N2 谓词） | `ChainEdge.predicate_holds = candidate_is_sub(child, parent)` | 唯一判定来源，不重写任何区间不等式 |
| 谱系照实：缺失留痕 | `ChainEdge.skipped_levels: Vec<SkippedLevel>` | 逐级记 `alive_at_level` / `inside_parent`，把「**缺**（该级无候选）」「**断-在父外**」「**断-在父内接不上**」分三格，不混同 |
| 谱系照实：证伪留痕 | `ChainNodeStatus::Falsified` + `ChainEdge.crossed_nodes` | 证伪节点原样留在 `nodes` 里（状态照实为 `Invalidated`），并记录它被哪条边跨过 |
| 谱系照实：跳过留痕 | `ChainEdgeKind::Skip` | 由两存活端点级别差派生，非人工标记 |
| append-only、旧 revision 保留 | `ChainCertificateBook.certificates: Vec<..>` 只 push | 无删除路径；`heads()` 另给每 key 最新 revision |
| 路径扩展走 `extends_lineage_key` 新 key | `ChainCertificateBook::resolve_extends()` → `TowerChainCertificate.extends`（**修复轮改**：由纯结构派生改为查簿命中） | 加子节点 = 新 `ChainKey`，旧链不被改写；结构真前缀未物化 ⟹ 不写指针，见 §九 9.1③ |
| 链段有效性一律由 C⊆C 在**存活端点**间判定 | `evaluate()`：先取 `alive_positions`，边只在相邻存活端点之间建 | 证伪/查无节点被跨过，不参与判定 |
| 谓词判过的 skip 边与相邻边**同权** | `ChainEdge::is_segment()` 只读 `predicate_holds`，不读 `kind` | 单测 `skip_edge_and_adjacent_edge_are_equally_segments` 直接断言两者相等 |
| 判不过记为**事实边**，不构成链段 | 边照实留在 `edges` 里、`is_segment()` 为假；`fact_edge_count()` 可读 | 单测 `predicate_failure_becomes_fact_edge_and_invalidates_chain` |
| 证伪节点留痕但**不判死上级** | 中间节点证伪 ⟹ 两侧存活端点间新长出一条 skip 边，链继续由谓词裁 | 单测 `falsified_middle_node_is_crossed_and_does_not_kill_the_head`（链保持 `Open`） |
| `Closed` = 链头 Confirmed + 不可再扩展 + 全链段判过（+ **≥1 有效链段**，#641 地板裁定补） | `evaluate()` **四**合取 | `extendable` = 最低存活端点内部还有存活候选；第四合取项见 §九 9.1① |
| `Invalidated` = 谓词判不过 **或** 链头 Invalidated；不复活 | `evaluate()` 优先臂 + `apply()` 终态挡回；成因落 `invalidation_cause`（**恰两档**） | 单测 `terminal_status_never_revives`（几何恢复也不复活）、`invalidation_cause_has_only_the_two_ruled_branches` |
| 否则 `Open` | 同上 | — |
| 纯产出零消费 | 生产面零引用（§三③） | — |
| 命名独立（与 `NestCertificate` 区分） | `TowerChainCertificate` / `ChainKey` / `ChainCertificateBook` | 不共享类型、不互相调用 |
| 44 课小转大证据路径**不做**（留 fog） | skip 边**不携带**任何替代必要条件证据 | 该事实随对象登记（§七 登记 2） |

### 本实装做出的一个判定（非新裁，须编排侧确认）：边 = 包含序的覆盖关系

`C⊆C` 传递：`c ⊆ m ⊆ p ⟹ c ⊆ p`。若把全部包含对都当链的边，一条 `p→m→c` 的下降会同时产出
`p→c` 这条传递闭包边——链数按路径长度组合放大，且 `p→c` 把**在场且套得住**的中间级候选 `m` 记成
「跳过」，伪造的是「中间级缺失」这一事实本身（roadmap:64「不得为缺失的中间级别伪造证书」的镜像
违规）。

故边取覆盖关系：`(c,p)` 是边 ⟺ `c ⊆ p` 且不存在存活 `m` 使 `c ⊆ m ⊆ p`（级别严格居中）。教义依据
= 027:46 逐级收缩 + chan99 §六「第一种情况（完全契合、逐级监控）最普遍」——**能逐级就必须逐级**；
skip 边只在中间级**真缺**或**真断**时出现。机器锁 = `edges_are_cover_relation_not_transitive_closure`。

链的构造口径 = 覆盖关系图中的**极大路径**（root 无存活父、leaf 无存活子），对应 027:46「从大级别
转折点出发……反复进行下去，直到最低级别」。单节点不成链（`ChainKey::new` 当场失败），孤立候选
数单列计数不静默丢。路径分叉时**不选支**（`selection_policy = null`，E2E-L 允许），全部分支各成一条链。

---

## 三、验收五条逐条

### ① 语义按裁定

见 §二逐条表。全部由 17 枚 `chain_cert::tests` + 2 枚 `classifier::tests` 语义锁承载，无「靠注释声明」
的条款。

### ② 单测全覆盖（票面点名七项）

| 票面要求 | 测试 | 结果 |
|---|---|---|
| 谓词两侧 | `predicate_failure_becomes_fact_edge_and_invalidates_chain`（判不过）/ 全部建链测试（判过） | PASS |
| skip 记录与同权 | `skip_edge_and_adjacent_edge_are_equally_segments`、`skipped_level_separates_absent_level_from_broken_level` | PASS |
| 证伪留痕 | `falsified_middle_node_is_crossed_and_does_not_kill_the_head`、`falsified_head_invalidates_chain_but_keeps_node_traces`、`absent_node_is_recorded_separately_from_falsified` | PASS |
| 终态不复活 | `terminal_status_never_revives`、`downward_extension_creates_new_key_and_leaves_closed_chain_untouched` | PASS |
| 幂等（同 as_of 零 Delta） | `same_as_of_replay_is_idempotent`；真实数据侧 `ISSUE641_CHAIN idempotent_replay_delta=0` | PASS |
| 双路径逐字节一致 | `chain_certificate_book_incremental_equals_full_replay`（跨步增量链簿 ≡ 每步全历史重建链簿，逐 revision `PartialEq`） | PASS |
| 相切边界（#246） | `touching_endpoints_still_form_a_segment`（左切/右切/同区间三格） | PASS |

另加（非票面点名，属构造口径与照实纪律的机器载体）：`edges_are_cover_relation_not_transitive_closure`、
`closed_requires_confirmed_head_and_no_extension`、`resident_open_chain_becomes_extendable_and_appends_a_payload_revision`、
`revisions_are_append_only_and_node_growth_alone_is_not_a_chain_revision`、
`head_only_survivor_stays_open_by_floor_conjunct`（**修复轮改名 + 翻转**，原
`head_only_survivor_closes_by_vacuous_segment_condition`，见 §九 9.1②）、
`isolated_candidates_are_counted_not_silently_dropped`、`single_node_path_is_not_a_chain`。

修复轮另加 4 枚（见 §九）：`floor_conjunct_does_not_block_a_chain_that_has_a_segment`、
`invalidation_cause_has_only_the_two_ruled_branches`、
`extends_is_none_when_the_structural_prefix_never_materialized`、
`summarize_and_digest_are_book_internal_readouts`。

### ③ 护栏

| 护栏项 | 证据 | 结果 |
|---|---|---|
| 既有 Classification/塔快照/BSP/订单流逐字节不动 | p123 三跑 SHA256：基座二进制（`/tmp/wt-553` 构建，654ee7a9c3）/ 本分支关灯 / 本分支开灯，`P116_DUMP` 均 = `fcc8016a9a01a1098736b9ee3b348614296bb97aec817a5c172c9a0723427a40`，stdout 均 = `bd9ac1d655f9d615a5d9b3495b3a92465fb1fae13ce5dadfa28033c47d375b6c`（`P116_MAX_BARS=20000`） | PASS |
| 事件流 FNV golden 锁 + 非真空锁 | `chain_certificate_book_classify_fnv1a_golden`：golden = `9771189513849272089`；非真空断言 = heads 非空 ∧ 至少一条链带边 ∧ `birth_open+birth_closed>0` ∧ `skip_edges>0`；构造口径锁 `birth_invalidated == 0` | PASS |
| nest 对照侧零触碰（ADR-0005） | 本包不引用 `nest.rs` 任何函数；不改 `NestCertificate`/`NestCandidateEvent`/`CandDeltaEvent`/typed 链 | PASS |
| p123 事件 dump 旁缝扩一路（env-gated 只写不判，N1-T4 口径） | `P123_CHAIN_DUMP=<path>` + `P123_CHAIN_DUMP_EVERY=<K>`，独立 sink 独立文件；关灯 ⟹ `observe` 首行返回（不建簿、不推进、不格式化）；单测 `chain_dump_disabled_writes_nothing_and_never_advances_the_book` / `chain_dump_cadence_advances_on_beat_and_on_last_bar` / `chain_dump_meta_and_line_format_are_stable` | PASS |
| 零消费接线 | `grep -rn chain_cert src/`：引用点仅两个诊断 bin + `mod.rs` 的 `#[cfg(test)]` 测试段；`src/theta_v0/backtest`、`src/theta_v0/strategy`、`src/trading`、`classifier/bsp.rs` **零引用** | PASS |

### ④ BTC 实测读数

见 §五（照实登记，未预设任何数值）。

### ⑤ 测试门

```text
CARGO_TARGET_DIR=/tmp/kimi-nest-target-641b cargo test --lib
基座 ticket-553（654ee7a9c3，净树）：2058 passed / 1 failed / 135 ignored（合计 2194）
本分支 ticket-641b               ：2077 passed / 1 failed / 135 ignored（合计 2213，+19 = 17 chain_cert + 2 classifier::tests）
唯一红                            ：theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard = #491（基座同红）
bin 侧                            ：cargo test --bin p123_fast_replay → 9 passed / 0 failed（含新增 3 枚）
```

**他工位污染核**：测试清单逐条 diff（`cargo test --lib -- --list`）确认本分支相对基座的增量恰为
上述 19 枚，无第三方测试混入。该核对正是发现 §〇 并发冲突的手段——在 `/tmp/wt-641` 上同一比对给出
+42（多出的 23 枚来自对方的 `cand_chain` / `cand_chain_book`）。

---

## 四、红绿记录（照实）

**不是严格 RED-first**：本轮先写模块体、再写测试文件，首跑即得真实红。照实登记如下，不追认 TDD。

| 轮次 | 命令 | 结果 | 处置 |
|---|---|---|---|
| 1 | `cargo test --lib chain_cert`（首跑） | **15 passed / 2 FAILED** | 两红均为**测试预期写错**，非实装缺陷 |
| 1a | `isolated_candidates_are_counted_not_silently_dropped` | 实得 `isolated_roots=2`，预期写了 1 | 夹具两条区间互不相含 ⟹ 两个都是孤立根；订正预期为 2 |
| 1b | `revisions_are_append_only` | 实得 2 枚 revision，预期写了 3 | 中间那步只是**节点 C 段右端生长**，链拓扑与谓词结论未变 ⟹ 链侧幂等跳过。这是 E2E-L §S8「`node_event_keys[]` 只存 key 引用」的正确后果（否则每根 bar 的活段生长都会给每条链刷一次修订，链簿沦为事件簿的影子）。测试改写为同时锁住这条口径，并另用「可扩展性翻转」制造真载荷变化 |
| 2 | `cargo test --lib chain_cert` | **17 passed / 0 failed** | — |
| 3 | `cargo test --lib chain_certificate_book`（golden 占位 0） | 1 passed / 1 FAILED | 取实测摘要 `9771189513849272089` 落 golden；同时按实测探针补 `skip_edges>0` 非真空断言 |
| 4 | `cargo test --lib chain` | **86 passed / 0 failed** | — |
| 5 | `cargo test --bin p123_fast_replay chain_dump`（首跑） | **3 passed / 0 failed** | — |
| 6 | 电池 500k 首跑（链簿自跑一条独立逐 bar 流） | **timeout 585s 中止** | 追加的第二遍全量分类是瓶颈；改为把链簿挂到 #550 既有对拍循环上（只读其已产出的事件流），100k 读数逐字段不变（89 链 / 74 Closed / skip_ratio 0.4059）证明改挂载口径中性 |
| 7 | 电池 500k 重跑（挂载后） | **仍 timeout 585s 中止** | 瓶颈在**既有** #550/#551/#552 三节（500k 上电池整体超过单命令 10 分钟上限），非链侧。照实登记，见 §七 登记 5 |
| 8 | `cargo test --lib` 全量（格式化后复跑） | 2077 / 1(#491) / 135 | — |

---

## 五、BTC 实测读数（照实，未预设）

数据：`/private/tmp/kimi-nest-mainline/analysis/data_cache/btc_1m_full.json`（软链主仓 data_cache）。
命令：`issue550_event_battery <json> <max_bars> <chain_every>`（release，`CARGO_TARGET_DIR=/tmp/kimi-nest-target-641b`）。

> **推进节拍是显式口径，不是静默采样**：`chain_every` 随读数一并印出（`advance_every=`）。链的覆盖边
> 计算是 O(存活候选²)，逐 bar 推进在十万级窗口不可行；末根必推。

### 5.1 链规模与三态分布

| 窗口 | 节拍 | 推进次数 | 候选身份（存活） | **链数（head）** | 链 revision | **Open** | **Closed** | **Invalidated** |
|---|---|---|---|---|---|---|---|---|
| 20k | 2000 | 10 | 46 | 18 | 18 | 0 | 18 | 0 |
| 100k | 5000 | 20 | 268 | 89 | 99 | 15 | 74 | 0 |
| 300k | 25000 | 12 | 832 | 293 | 480 | 55 | 237 | **1** |

### 5.2 边形态与 skip 占比

| 窗口 | 边数 | 相邻边 | **skip 边** | **skip 边占比** | 事实边（谓词判不过） | 被边跨过的节点 |
|---|---|---|---|---|---|---|
| 20k | 23 | 14 | 9 | **0.3913** | 0 | 0 |
| 100k | 101 | 60 | 41 | **0.4059** | 0 | 0 |
| 300k | 325 | 191 | 134 | **0.4123** | 0 | 0 |

### 5.3 留痕分解（节点 / 被跳过级别）

| 窗口 | 节点存活 | 节点证伪 | 节点查无 | 跳过级：**缺**（该级无候选） | 跳过级：**断-在父外** | 跳过级：**断-在父内接不上** |
|---|---|---|---|---|---|---|
| 20k | 41 | 0 | 0 | 0 | 0 | 9 |
| 100k | 190 | 0 | 0 | 0 | 6 | 35 |
| 300k | 618 | **1** | 0 | 0 | 73 | 96 |

### 5.4 路径形态

| 窗口 | 路径长 2 | 路径长 3 | 链头级别分布 | `extends` 非空（路径扩展链） | 幂等重跑 Delta |
|---|---|---|---|---|---|
| 20k | 13 | 5 | {1:4, 2:14} | 5 | **0** |
| 100k | 77 | 12 | {1:36, 2:53} | 12 | **0** |
| 300k | 260 | 33 | {1:131, 2:116, 3:46} | 33 | **0** |

### 5.5 读数照实解读（只说数支持的）

1. **skip 边不是边缘现象**：三个窗口的 skip 占比稳定在 0.39–0.41，即约四成链段是跨级的。若按 (a)
   「逐级必须相邻」口径，这四成链段全部作废——#636 裁定③选 skip 合法，在数据上不是无关紧要的选择。
2. **跳过的成因全部是「断」，一例「缺」都没有**：`skipped_level_missing = 0`（三窗）。即被跳过的中间
   级别在窗口里**都有候选存在**，只是没有一个同时套住两端（`broken_inside`：该级有候选落在父区间内
   却接不上子端点；`broken_outside`：该级候选整个落在父区间外）。这是本实装分三格记数才读得出的事实
   ——若只记一个布尔「跳了一级」，会被误读成「中间级别不存在」。
3. **事实边（谓词判不过）在 head 面上为 0**：三窗均 0。含义是「活着的链没有一条是靠谓词判不过而死的」；
   300k 那 1 条 `Invalidated` 伴随 `nodes_falsified=1` 且 `crossed=0`，即死因是**链头被证伪**，不是谓词
   断——与裁定②「Invalidated = 谓词判不过**或**链头 Invalidated」的第二支对应。
4. **`Closed` 占绝对多数（20k 100%、100k 83.1%、300k 80.9%）**，且 20k 窗口全部链一出生即 `Closed`。
   成因可直接从上游读出：BTC 上候选身份几乎全是 Pan 域（100k：268 中 261 为 Pan，Pan 观察恒 `Confirmed`）
   ⟹ 链头恒 `Confirmed`；而极大路径的 leaf 按构造无存活子 ⟹ `extendable=false`。两条 `Closed` 条件在
   构造时就同时满足。**这是读数，不是评价**——它对 N7「只收 Closed 谱系」的容量口径直接相关：可消费
   链数 ≈ 链总数的八成以上，而其中链头几乎都是盘整背驰候选。
5. **路径长度上限是 3**：三窗都没有出现 4 节点及以上的链，即便 300k 窗口的塔有 4 级（`stream_levels=4`,
   `levels={0:666, 1:136, 2:27, 3:3}`）。
6. **幂等在真实数据上成立**：三窗 `idempotent_replay_delta = 0`。

### 5.6 未取到的读数（照实）

- **500k 窗口未完成**：两次尝试均在单命令 585s 上限中止（`real 9m45`）。瓶颈是**既有**电池三节
  （#550 逐 bar 双通道对拍 + #552 的 O(n²) 相邻级扫描 + #551 的 fresh-full 分叉重算）在 500k 上的总耗时，
  不是链侧——证据：把链簿从「自跑一遍全量分类」改挂到既有循环上后，100k 总耗时 12.0s → 11.1s，读数
  逐字段不变；300k 全跑 2m57s 通过。500k 读数留空，不外推、不用 300k 的数冒充。

---

## 六、与 E2E-L 原型的语义对照表

原型锚：`git show 640609071d:chanlun/review-results/doc-divergence-endtoend-prototype-20260718.md`
（§5:188 skip edge 定义、§6.1:85 谱系状态机、§6.2:250 LineageKey、§6.3 S8 联结口径）。

| # | E2E-L 原型 | 本实装 | 关系 |
|---|---|---|---|
| D1 | 谱系四态 `Open / Unresolved / Closed / Invalidated`（:85） | 三态 `Open / Closed / Invalidated` | **裁定改写**。#636 裁定②只给三态、且「否则 Open」；原型的 `Unresolved`（证据不足）在本实装并入 `Open` |
| D2 | `Closed` 条件 = **全部被消费节点** `Confirmed` ∧ `CloseFloor_at` 非 `UnresolvedFloor`（:85） | `Closed` = **链头** `Confirmed` ∧ 不可再扩展 ∧ 全链段谓词判过 ∧ **≥1 有效链段** | **裁定改写 + 地板条款补齐**。裁定把「全部节点确认」放宽为「链头确认」（因「证伪节点不判死上级」），另加「全链段谓词判过」；第四个合取项由 #641 comment-5121572134 补裁（见 D3） |
| D3 | `CloseFloor_at` 三型地板 `FormalFloor / QuasiFloor / UnresolvedFloor`，且「节点高于 L1 而当前前缀没有可证子事件 ⟹ 只能给 `UnresolvedFloor`，`Consume_at` 必须拒绝」（:184） | **不实装 floor 三型分类**；「不可再扩展」= 本次 `as_of` 下最低存活端点内部无存活候选；**另加「≥1 有效链段」合取项** | **部分对齐（2026-07-29 修复轮改写本行）**。原实装按 #636 裁定字面落地，「全链段谓词判过」在空集上真空成立 ⟹ 链头独活可 `Closed`，与原型的 `UnresolvedFloor` 相悖（旧对照结论：不一致）。#641 comment-5121572134 已裁定补上「至少一条有效链段」合取项：链头独活 = **永远 Open**，本实装已落地（`evaluate` 的 `has_segment` 合取 + `chain_probe::floor_blocked` + 单测 `head_only_survivor_stays_open_by_floor_conjunct`）⟹ 与原型 `UnresolvedFloor` 在**这一格**上对齐。**仍不一致的剩余面**：原型的三型地板是**级别地板**（`L1=FormalFloor`、`L0=QuasiFloor`，节点高于 L1 且无可证子事件才给 `UnresolvedFloor`），本实装无级别分量——一条 L2→L1 的两节点链只要有链段就可 `Closed`，按原型它在 L1 应是 `FormalFloor`（可闭合，此格恰巧同判），但一条 L3→L2 的链按原型只能给 `UnresolvedFloor`，本实装仍判 `Closed`。该剩余面由取舍裁定 comment-5121793896 第 5 条**明确留 fog 另裁**，本轮不动。反事实锁 = `floor_conjunct_does_not_block_a_chain_that_has_a_segment`（若误把地板写成级别地板，该测试当场变红） |
| S1 | skip edge：允许 `parent_level > child_level + 1` 的**显式**边；记录事实、不自动补中间节点、不得为缺失级别伪造证书（:188） | `ChainEdgeKind::Skip` + `SkippedLevel` 逐级记数；模块内不构造 `CandidateKey` | **一致**（并加严：把「缺」与「断」分三格记，原型只要求「记录事实」） |
| S2 | skip 边是否需携带替代必要条件（44 课小转大：次级别中枢第三类买卖点）——原型**未回答** | **不携带** | **一致于裁定③**（明确留 fog）。照实：本实装的 skip 边比 44 课教义门槛**宽** |
| K1 | `LineageKey = ["LIN1", rule_id, rule_version, selection_policy_id_or_null, selection_policy_version_or_null, ordered_EventKey_path, ordered_EdgeKey_path, localization_tail_key_or_null]`；不含 Floor/status/clocks/revision（:250） | `ChainKey = { rule_version, path: Vec<CandidateKey> }`；不含 status/clocks/revision | **裁剪同构**。`ordered_EdgeKey_path` **不入键**——边的形态（Adjacent/Skip、跨过哪些节点）是当时节点状态的派生量，入键会让同一条路径在证伪前后变成两个身份，与「证伪节点留痕但不判死上级」冲突。`selection_policy` 恒 null（不选支）；`localization_tail_key` 不实装（类背驰尾端属 E2E-F，不在 #636 范围） |
| K2 | `extends_lineage_key` = 唯一最长 proper-prefix key；只在规则与政策 ID/版本完全相同时才指（:253） | `ChainCertificateBook::resolve_extends()`：**簿内**最长真前缀（从 `len-1` 向下试到 2 节点，取第一个有 head 的）；`rule_version` 相同 | **一致（2026-07-29 修复轮改写本行）**。旧实装是纯结构派生（去 leaf，不问是否物化），BTC 100k 实测 12 条 `extends` **全部**指向从未落簿的链（幽灵前缀）；取舍裁定 comment-5121793896 第 3 条判改为查簿命中才写。向上多出新 root 不构成 prefix 关系 ⟹ `extends = None`。**不按状态过滤**：`Closed`/`Invalidated` 的前缀同样算数（「那条链曾经存在」是谱系事实）。负控 = `extends_is_none_when_the_structural_prefix_never_materialized`，命中侧 = `downward_extension_creates_new_key_and_leaves_closed_chain_untouched` |
| S8 | `s4_lineages.jsonl.node_event_keys[]` **只存 EventKey 引用**，节点几何由 validator 回联事件 head 取（:314） | `nodes` 只存 key + 当次状态；**节点区间不进链投影**；唯一例外 = 事实边的 `PredicateBreach` 携带判定时读到的两个区间 | **一致（例外有界，2026-07-29 修复轮加注）**。后果已机器锁：节点 C 段右端生长而拓扑与谓词结论不变时链侧零 Delta（`revisions_are_append_only_and_node_growth_alone_is_not_a_chain_revision`）。修复轮移植的「事实边指名见证」把两端点区间写进 `ChainEdge::breach`——这是**唯一**几何入链载荷的路径，且有界：谓词判不过 ⟹ 链当次即 `Invalidated`（终态）⟹ 每条链至多一条 revision 携带几何，不构成 B 侧那种「节拍收细即退化为事件簿影子」的形态（B 是**每条边**恒带 `parent_interval`/`child_interval`）|
| R1 | 修订协议：同 key 业务载荷投影变才 `revision=n+1` + `supersedes_revision=n`；投影相同零输出；`Closed/Invalidated` 终态；终态后规则版本变须新 key（:83/:85） | `ChainProjection` + `apply()` + `CHAIN_RULE_VERSION` | **一致** |
| C1 | `Consume_at` 只接收 `Closed` 谱系（:85） | **不实装消费**（N7 另票） | 一致于裁定④「N7 只收 Closed 谱系」，本票纯产出 |

---

## 七、090 照实登记

1. **`CONTEXT.md` 词表收编（验收④后半）未落地——文件不在本分支上**。`CONTEXT.md` 只存在于 `main`
   （前例：「背驰段候选事件」条是由 #540 裁定侧在 main 上加的，不是 #550 实装票加的）。在 `main` 工作区
   落词条超出本票权限（不 push/merge、不动他 worktree 已检出面）。**待编排侧收编**，条目文本备好如下：

   > **级别链证书（tower chain certificate）**：
   > 塔内原生链谱系对象（2026-07-28 #636 裁定，N3 = #641）：节点 = 背驰段候选事件（身份键沿用），
   > 边 = C⊆C 包含序的**覆盖关系**（能逐级必逐级；中间级真缺/真断才出 skip 边，skip 与相邻边同权），
   > 谱系照实（缺失/证伪/跳过全留痕、append-only、禁伪造中间级证书），终态三态（Closed = 链头
   > Confirmed + 不可再扩展 + 全链段谓词判过；Invalidated = 谓词判不过或链头 Invalidated，不复活；
   > 否则 Open）。路径扩展 = 新 key + `extends_lineage_key`。与 nest 对照侧的 `NestCertificate`
   > 是两个词（后者 = ADR-0005 独立对照实现、时间坐标、严格相邻装配）。
   > _Avoid_：与 NestCertificate 混称「证书」；把 skip 边当无门槛跳级（44 课小转大的替代必要条件尚未实装）

2. **44 课「小转大」替代必要条件未实装**：skip 边不携带「次级别中枢第三类买卖点」证据 ⟹ 本实装的
   skip 边**比 44 课教义门槛宽**。#636 裁定③明确留 fog、另票另裁；此处只登记后果，不擅补。

3. **`ChainNodeStatus::Absent` 的可达域**：由因果簿（`TowerCache` 正本，append-only）驱动时不可达；
   由终态窗口投影（`classify_with_tower_events`，每次 fresh book）驱动时可达。两种驱动都支持，故它是
   真实可达分支，非防御性死码。单测 `absent_node_is_recorded_separately_from_falsified` 走的是后者。

4. **`chain_probe.birth_invalidated` 恒 0 是构造口径的后果**：首次观察的路径来自覆盖关系图（每条边的
   `C⊆C` 按构造成立）且全节点存活 ⟹ 既不可能「谓词判不过」也不可能「链头 Invalidated」。已作断言锁
   （非零 = 构造口径被破坏），不是无人读的死计数。

5. **BTC 500k 窗口未完成**：两次 585s 上限中止，瓶颈在既有电池三节而非链侧（证据见 §五 5.6）。
   500k 读数留空，不外推。

6. **合成 classify 夹具未覆盖的分支**：`to_invalidated` / `fact_edges` / `falsified_nodes` /
   `payload_revision` 在 `chain_certificate_book_classify_fnv1a_golden` 的合成夹具上为 0，故 golden
   测试**不**断言它们非零（断言非零会把「合成数据规整」误报成缺陷）。这四条由 `chain_cert::tests` 的
   语义锁逐条覆盖；真实数据侧 300k 窗口给出 `Invalidated=1`、`nodes_falsified=1` 的非真空量度。

7. **复杂度未优化**：`chain_paths` 的覆盖边计算是 `Σ_p O(|⊆p|²)`，路径枚举是输出敏感 DFS，无上限、
   无采样、无截断。本模块不在任何逐 bar 热路径上；若将来接进逐 bar 循环须先改判据结构。

8. **`TowerCache::candidate_streams()` 是新增的公开只读取数口**：为让已调 `classify_with_tower_incremental`
   的既有驱动（p123）在不改既有调用点、不改既有返回值的前提下读到事件流。只读、不改簿、零行为影响。

9. **`rustfmt`**：本票四个文件（`chain_cert/mod.rs`、`chain_cert/tests.rs`、两个 bin）`rustfmt --check
   --edition 2021` 干净。`classifier/mod.rs` 未单文件校验——`rustfmt` 对它会连带拉入 `bsp.rs` 等既有
   非 fmt-clean 文件（仓库整体非 rustfmt-clean，#552 报告 §七登记 5 已在案），故沿用同一处置：只对本票
   新增/独立文件做校验，不全仓改写。

10. **`/tmp/wt-641` 上本侧遗留的未提交改动未清理**：清理会与并发 session 可能正在进行的编辑竞态
    （不可逆），留给编排侧处置。本侧产出已完整移到 `ticket-641b`。

---

## 八、测试指纹三元组 + 快照

```text
快照 commit（基座）：654ee7a9c3  ticket-553（净树）
交付分支           ：ticket-641b（代码 commit 4fa468711d）
cargo test --lib   ：passed=2077 / failed=1 / ignored=135     （唯一红 = #491，基座同红）
基座同命令         ：passed=2058 / failed=1 / ignored=135
cargo test --bin p123_fast_replay：passed=9 / failed=0 / ignored=0
干净快照口径       ：git status --porcelain 仅列本票四个文件 + chain_cert/ 新目录
FNV golden（链簿） ：9771189513849272089
p123 封印 SHA256   ：P116_DUMP  = fcc8016a9a01a1098736b9ee3b348614296bb97aec817a5c172c9a0723427a40
                     stdout     = bd9ac1d655f9d615a5d9b3495b3a92465fb1fae13ce5dadfa28033c47d375b6c
                     （基座二进制 / 本分支关灯 / 本分支开灯 三者相等，P116_MAX_BARS=20000）
```

> ★本节数字为 `4fa468711d` 当时的快照。修复轮（取舍裁定执行节，见 §九）后的现行三元组见 §九 5。

---

## 九、取舍裁定执行节（2026-07-29）

裁定锚：#641 comment-5121793896（两套 N3 取舍，编排者「照办」）+ comment-5121572134（地板条款）。
依据：对比评审 `/tmp/issue641-n3-comparison-20260729.md`（M1/M2/M3 + P1/P2/P3）。
执行面：`ticket-641b`，改 4 个文件（`chain_cert/{mod.rs,tests.rs}`、`classifier/mod.rs`、
`bin/issue550_event_battery.rs`）。**未动** `/tmp/wt-641`、未动侧 ref `ticket-641-kimi-line`、
未 push/merge、未改 `Cargo.toml`。

### 9.1 补三件（裁定第 3 条）

| # | 裁定条款 | 落地 | 机器载体 |
|---|---|---|---|
| ① | `Closed` 加「≥1 有效链段」合取（地板裁定） | `evaluate()` 新增 `has_segment = edges.iter().any(is_segment)`，`Closed` 臂改为 `head_confirmed && !extendable && has_segment`；模块头加「地板条款」专段 | 单测 `head_only_survivor_stays_open_by_floor_conjunct`（链头 `Confirmed` + 不可扩展 + 零链段 ⟹ 判 `Open`、`closed_at` 不落）+ 探针 `chain_probe::floor_blocked` + 读数格 `ChainBookSummary::closed_with_zero_segments`（三窗恒 0）+ 主缝 golden 测试内的 `closed_with_zero_segments == 0` 断言 |
| ② | 翻转 `head_only_survivor_closes_by_vacuous_segment_condition` 为「不可 Closed」 | 该测试**重写并改名**为 `head_only_survivor_stays_open_by_floor_conjunct`；原文档注释（「真空成立 ⟹ 链可 Closed」）整段替换为裁定字面。改名理由：原名把被裁定否掉的后果写进了名字，留名 = 声明与实际不一致（090）；新旧名对照在此登记，票面点名的那一枚即本枚 | 断言由 `status == Closed` 翻为 `status == Open` + `closed_at == None` + `floor_blocked > 0` |
| ③ | `extends` 改查簿命中才写 | 删 `ChainKey::proper_prefix()`（纯结构派生），改为私有 `ChainKey::prefix(len)` + 簿侧 `ChainCertificateBook::resolve_extends()`：从 `len-1` 向下试到 2 节点，取**第一个簿内有 head 的**前缀。解析点从 `evaluate()`（纯函数）挪到 `apply()`（落簿处）——承继是簿的事实，塞进纯函数会让「纯函数」这个声明变假。`ChainObservation` 相应删去 `extends` 字段 | **负控**单测 `extends_is_none_when_the_structural_prefix_never_materialized`（三节点链一次成型、真前缀从未落簿 ⟹ `extends == None`）+ **命中侧**在 `downward_extension_creates_new_key_and_leaves_closed_chain_untouched` 内加探针断言 `(extends_resolved, extends_not_materialized) == (1, 0)` + 主缝非真空锁 `probe.extends_not_materialized > 0` |

### 9.2 从 B 移植三件（裁定第 4 条，B 源锚 → A 形态）

| # | B 源锚 | A 侧形态 | 差异说明（不照抄签名） |
|---|---|---|---|
| P1 判死成因分档 | `cand_chain_book.rs:56-68` `ChainInvalidationCause`（四档） | `chain_cert::ChainInvalidationCause`（**两档**：`HeadInvalidated` / `PredicateFailed`）+ `TowerChainCertificate::invalidation_cause`（与 `invalidated_at` 同步一次写入）+ `ChainBookSummary` 两个分档计数 | **只留裁定②授权的两支**：B 的 `NodeInvalidated`（中间节点失效即判死全链）是 2026-07-29 核定 supersede 掉的连坐语义，**不移植**；B 的 `NodeAbsent` 亦不另开档（见 9.4 登记 1）。成因**不进** `ChainProjection`（生命史记账，与三只钟同侧，口径与 B 同）。反事实锁 = `invalidation_cause_has_only_the_two_ruled_branches`（中间节点证伪 ⟹ 链仍 `Open` 且成因为 `None`；若误移植连坐支当场变红） |
| P2 库内只读读数口 | `cand_chain_book.rs:539-611` `summarize` / `digest` / `heads`（自由函数，吃 `ChainLineageStreams`） | `ChainCertificateBook::summarize() -> ChainBookSummary` / `::digest() -> u64`（方法，吃簿本身）；`heads()` A 侧**原已有**，本轮不动 | B 按四档 `EdgeVerdict` 分桶，A 按本模块的边形态（`ChainEdgeKind`）+ 谓词两侧（链段/事实边）分桶，并加了 A 独有的三格 `SkippedLevel` 分解与地板监视格。`digest` 口径与主缝 golden 统一（golden 测试改调 `book.digest()`）⟹ 报告数、golden、bin 读数三处同源。`issue550_event_battery::print_chain_summary` 的自写循环整段删除，改读 `summarize()` |
| P3 事实边指名见证 | `cand_chain.rs:366-397` `fact_edge` 的 `min_by_key` 指名档 | `ChainEdge::breach: Option<PredicateBreach>`：`predicate`（恒 `CHAIN_SEGMENT_PREDICATE = "cand_sub::candidate_is_sub"`）+ `parent`/`child`（指名到候选身份键）+ 两端点判定时的区间 + `reason: PredicateBreachReason` | 语义位置不同：B 的指名见证挂在「被跳过级别」的事实边上（B 那里 child 可能缺失，故需另选见证）；A 的事实边**两端点本就在场**，缺的是「哪条谓词、判不过在哪一项」，故 A 的见证给的是**谓词构成的首个失败合取项**（`ChildIntervalDegenerate` / `ParentIntervalDegenerate` / `LeftOverhang` / `RightOverhang` / `BothEndsOverhang`）。级别分支的失败档**不设**——路径级别严格递减使其在调用点不可达，按本模块既有口径用 `unreachable!` + 推导链作载体（同 `on_append` 的禁止边），不留恒 0 的枚举变体。锁 = `predicate_failure_becomes_fact_edge_and_invalidates_chain` 内的六条见证断言 |

### 9.3 复测与护栏

**BTC 链读数（同 §五口径与节拍，release，`CARGO_TARGET_DIR=/tmp/kimi-nest-target-641b-fix`）**：

| 窗口/节拍 | 链数 | revision | Open | Closed | Invalidated | 边 | skip | skip 占比 | **`closed_with_zero_segments`** | **`invalidated_head`/`_predicate`** | **`extends_some`（旧→新）** |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 20k / 2000 | 18 | 18 | 0 | 18 | 0 | 23 | 9 | 0.3913 | **0** | 0 / 0 | **5 → 0** |
| 100k / 5000 | 89 | 99 | 15 | 74 | 0 | 101 | 41 | 0.4059 | **0** | 0 / 0 | **12 → 0** |
| 300k / 25000 | 293 | 480 | 55 | 237 | 1 | 325 | 134 | 0.4123 | **0** | **1 / 0** | **33 → 0** |

照实读：
1. **地板合取在 A 的数据面上零变化**——三窗链数/三态/边形态/skip 占比与 §五**逐字段相同**，`closed_with_zero_segments = 0`（与对比评审「A 侧违规可达但未触发」的实测一致）。对照：B 侧同一格是其 `Closed` 的 80–85%。
2. **`extends` 的 5/12/33 条全部撤下**——三窗归零，与对比评审「12/12 幽灵」的独立实测吻合（那 12 条正是本轮 100k 归零的 12 条）。这不是「功能退化」，是把一个**从未指向过真实对象**的指针如实置空；`extends` 的命中路径仍在（单测覆盖），只是 BTC 这三窗的节拍下从未发生「真前缀先落簿、再向下扩展」的序列。
3. **300k 那 1 条 `Invalidated` 的死因现在机器可读**：`invalidated_head=1 / invalidated_predicate=0`，即 §5.5 第 3 条原来靠 `nodes_falsified=1 ∧ crossed=0` 推出来的归因，现由 P1 的成因分档直接给出。
4. `idempotent_replay_delta = 0`（三窗），幂等未因本轮改动破坏。

**字节护栏（产物 `/tmp/wt641bfix-*`）**：

| 面 | 命令 | 结果 |
|---|---|---|
| p123 stdout / P116 dump（20k、100k） | `P116_MAX_BARS={20000,100000} P116_CKPT=0 P116_DUMP=…` | **cmp=0 ×4**（pre = `46c4c30dd3` 树经 `git archive` 另建的 release 二进制；post = 本轮） |
| m8 三窗 `trades.jsonl` / `tower_events.jsonl` | `M8_WIN_FILTER={p3fold,wf7,wf8} OPSEM_DUMP_DIR=… cargo test --release --lib m8_e2e_all_systems_oos -- --ignored` | **cmp=0 ×6**；SHA 前 16 位与 #527 报告 §7.3 六个数逐字相同（跨票一致） |
| p123 链 dump（本体面，给前后对照） | `P123_CHAIN_DUMP=… P123_CHAIN_DUMP_EVERY={2000,5000}` | 20k：19 行中 **5 行**差异；100k：100 行中 **12 行**差异（两侧行数相同）。**差异全部且仅在 `extends=` 字段（1→0）**——把该字段抹掉后两侧逐字节相同（`diff <(sed 's/extends=[01] //' pre) <(sed …post)` 为空）。即：地板合取在 dump 面上**零翻转**，唯一变化面就是 `extends` 修复 |

链 dump 的行格式**未改**（`cause=` 未加）：判死成因可由该行既有的 `alive/falsified/fact` 三格推出，加字段会动 `chain_dump_meta_and_line_format_are_stable` 的封印面而不增信息。此事实照实登记。

### 9.4 090 照实登记（本轮新增，接续 §七）

1. **`ChainInvalidationCause::HeadInvalidated` 覆盖两种链头形态**：`Falsified`（事件判 `Invalidated`）与 `Absent`（终态窗口投影驱动下查无）。裁定②只授权「链头 `Invalidated`」一支，故不为 `Absent` 另开第三档（B 有 `NodeAbsent`，不在授权范围）。**分辨不丢**——`nodes[0].status` 原样留痕，读的人从节点留痕即可分开。若编排侧认为「查无」应独立成档，属新裁，本轮不自决。
2. **`extends` 现在可因簿的历史而变**：一条 `Open` 链在其真前缀后来物化时会获得 `extends` ⟹ 载荷变 ⟹ 追加一条 payload revision。这是查簿口径的必然后果（承继是簿的事实），append-only 与终态不复活均不受影响（终态链在 `apply` 入口已挡回）。BTC 三窗未触发该序列。
3. **事实边的见证把几何写进了链载荷**（E2E-L §S8 的唯一例外，见 §六 S8 行）：有界——谓词判不过 ⟹ 链当次终态 ⟹ 每条链至多一条 revision 携带几何。
4. **`resolve_extends` 不按前缀状态过滤**：`Closed` / `Invalidated` 的前缀同样可被指向（「那条链曾经存在」是谱系事实）。B 侧 `resolve()` 排除 `Invalidated` 前缀，本轮**不照抄**——A 的既有语义锁 `downward_extension_creates_new_key_and_leaves_closed_chain_untouched` 正是「指向一条已 `Closed` 的前缀」，排除终态会与它冲突。差异在此登记。
5. **测试门的红不是全绿**：本工位基座 `ticket-553` 上 `extract_signals_bit_exact_digest_guard`（#491）**仍红**，与 `4fa468711d` 快照同红、与基座同红。#491 的收口是**在 main 上**做的 GOLDEN 诚实重锚（`f6cd18aebc` / 合并 `a559d2edba`，锚 `issue610-digest-guard-attribution-20260728.md`），main 侧现行值 `0xe371_3897_d9bf_978c` 取的是 #455 之后（`level_origin` 已删）的口径；kimi 线的实测值 `0xe6a2_63e3_43e4_3845` 正是 main 侧注释里点名的「该字段删除前那一版」。**该重锚不在本票权限内**（跨票、且属 main 线的口径决定），本轮不改 `signal.rs`。派发单「基线已全绿」的前提对 **main** 成立、对 `ticket-641b` 的基座**不成立**，照实登记不代过。
6. **`floor_blocked` 探针在真实数据上恒 0**：三窗均未触发（A 的地板违规「可达但未触发」）。故该条款的非真空证据只来自 `chain_cert::tests` 的语义锁，不来自 BTC——认识论等级 = L0/L1（合成夹具），不是 L2。若将来某窗口出现非零，属真实触发，须照实记而不是当异常。
7. **500k 窗口仍未跑**（§七 登记 5 原因未变，瓶颈在既有电池三节）。本轮未重试。
8. **`CONTEXT.md` 词表条目文本需随本轮更新**：§七 登记 1 备好的条目里「终态三态」一句应补「+ 至少一条有效链段（链头独活 = 永远 Open）」，并把「`extends_lineage_key`」注为「簿内最长真前缀」。仍待编排侧在 main 上收编，本票不动 main。

### 9.5 修复轮后的三元组

```text
基座               ：654ee7a9c3  ticket-553（净树）
交付分支           ：ticket-641b（代码 4fa468711d + 报告 46c4c30dd3 + 本修复轮 commit）
cargo test --lib   ：passed=2081 / failed=1 / ignored=135   （+4 = 本轮新增 4 枚）
  唯一红           ：extract_signals_bit_exact_digest_guard = #491（基座同红，见 9.4 登记 5）
  本票 chain_cert  ：21 passed / 0 failed（17 → 21）
  本票 classifier  ：2 passed / 0 failed（不变，内容加严）
cargo test --bin p123_fast_replay：passed=9 / failed=0 / ignored=0（不变）
FNV golden（链簿） ：9935805022530767834（旧 9771189513849272089，因 `invalidation_cause` /
                     `breach` 进 Debug + `extends` 改查簿而诚实翻转）
rustfmt --check    ：chain_cert/{mod.rs,tests.rs}、两个 bin 干净（`classifier/mod.rs` 同 §七登记 9 处置）
BTC 链簿 digest    ：20k=11721078737383519109 / 100k=6645041198801530824 / 300k=2200354287177549197
```
