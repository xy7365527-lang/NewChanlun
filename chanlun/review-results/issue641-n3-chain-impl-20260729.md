# #641（N3）级别链证书塔对象——实施报告

日期：2026-07-29　票：#641（父票 #529）　裁定：#636 Resolution（comment-5119573446）
基座：`ticket-553`（654ee7a9c3，N1/N2 四票链尾）　交付分支：`ticket-641b`　工位：`/tmp/wt-641b`

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
   - 若裁定给回 E2E-L 的 `CloseFloor_at` / `UnresolvedFloor`（本裁定未携带，见 §六对照表 D3），
     「只剩链头存活且不可扩展 ⟹ 可 Closed」这一条照实后果翻转；
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
| 路径扩展走 `extends_lineage_key` 新 key | `ChainKey::proper_prefix()` → `TowerChainCertificate.extends` | 加子节点 = 新 `ChainKey`，旧链不被改写 |
| 链段有效性一律由 C⊆C 在**存活端点**间判定 | `evaluate()`：先取 `alive_positions`，边只在相邻存活端点之间建 | 证伪/查无节点被跨过，不参与判定 |
| 谓词判过的 skip 边与相邻边**同权** | `ChainEdge::is_segment()` 只读 `predicate_holds`，不读 `kind` | 单测 `skip_edge_and_adjacent_edge_are_equally_segments` 直接断言两者相等 |
| 判不过记为**事实边**，不构成链段 | 边照实留在 `edges` 里、`is_segment()` 为假；`fact_edge_count()` 可读 | 单测 `predicate_failure_becomes_fact_edge_and_invalidates_chain` |
| 证伪节点留痕但**不判死上级** | 中间节点证伪 ⟹ 两侧存活端点间新长出一条 skip 边，链继续由谓词裁 | 单测 `falsified_middle_node_is_crossed_and_does_not_kill_the_head`（链保持 `Open`） |
| `Closed` = 链头 Confirmed + 不可再扩展 + 全链段判过 | `evaluate()` 三合取 | `extendable` = 最低存活端点内部还有存活候选 |
| `Invalidated` = 谓词判不过 **或** 链头 Invalidated；不复活 | `evaluate()` 优先臂 + `apply()` 终态挡回 | 单测 `terminal_status_never_revives`（几何恢复也不复活） |
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
`head_only_survivor_closes_by_vacuous_segment_condition`、`isolated_candidates_are_counted_not_silently_dropped`、
`single_node_path_is_not_a_chain`。

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
| D2 | `Closed` 条件 = **全部被消费节点** `Confirmed` ∧ `CloseFloor_at` 非 `UnresolvedFloor`（:85） | `Closed` = **链头** `Confirmed` ∧ 不可再扩展 ∧ 全链段谓词判过 | **裁定改写**。裁定把「全部节点确认」放宽为「链头确认」（因「证伪节点不判死上级」），另加「全链段谓词判过」 |
| D3 | `CloseFloor_at` 三型地板 `FormalFloor / QuasiFloor / UnresolvedFloor`，且「节点高于 L1 而当前前缀没有可证子事件 ⟹ 只能给 `UnresolvedFloor`，`Consume_at` 必须拒绝」（:184） | **不实装 floor 分类**；「不可再扩展」= 本次 `as_of` 下最低存活端点内部无存活候选 | **裁定未携带**。#636 裁定②的三条件里没有 floor 分量。**照实后果**：全部下级被证伪、只剩链头存活时链段集合为空 ⟹「全链段谓词判过」真空成立 ⟹ 链可 `Closed`；按 E2E-L 该情形是 `UnresolvedFloor`、不可 Close。本实装按裁定字面落地，不自行补 floor 规则。机器载体 = `head_only_survivor_closes_by_vacuous_segment_condition`（该测试同时锁住「链头未确认时真空条件本身不足以 Closed」）。**待编排侧裁** |
| S1 | skip edge：允许 `parent_level > child_level + 1` 的**显式**边；记录事实、不自动补中间节点、不得为缺失级别伪造证书（:188） | `ChainEdgeKind::Skip` + `SkippedLevel` 逐级记数；模块内不构造 `CandidateKey` | **一致**（并加严：把「缺」与「断」分三格记，原型只要求「记录事实」） |
| S2 | skip 边是否需携带替代必要条件（44 课小转大：次级别中枢第三类买卖点）——原型**未回答** | **不携带** | **一致于裁定③**（明确留 fog）。照实：本实装的 skip 边比 44 课教义门槛**宽** |
| K1 | `LineageKey = ["LIN1", rule_id, rule_version, selection_policy_id_or_null, selection_policy_version_or_null, ordered_EventKey_path, ordered_EdgeKey_path, localization_tail_key_or_null]`；不含 Floor/status/clocks/revision（:250） | `ChainKey = { rule_version, path: Vec<CandidateKey> }`；不含 status/clocks/revision | **裁剪同构**。`ordered_EdgeKey_path` **不入键**——边的形态（Adjacent/Skip、跨过哪些节点）是当时节点状态的派生量，入键会让同一条路径在证伪前后变成两个身份，与「证伪节点留痕但不判死上级」冲突。`selection_policy` 恒 null（不选支）；`localization_tail_key` 不实装（类背驰尾端属 E2E-F，不在 #636 范围） |
| K2 | `extends_lineage_key` = 唯一最长 proper-prefix key；只在规则与政策 ID/版本完全相同时才指（:253） | `ChainKey::proper_prefix()`（去 leaf，≥2 节点才有）；`rule_version` 相同 | **一致**。向上多出新 root 不构成 prefix 关系 ⟹ `extends = None`（原型的路径扩展也只指向下加子节点/加尾端） |
| S8 | `s4_lineages.jsonl.node_event_keys[]` **只存 EventKey 引用**，节点几何由 validator 回联事件 head 取（:314） | `nodes` 只存 key + 当次状态；**节点区间不进链投影** | **一致**。后果已机器锁：节点 C 段右端生长而拓扑与谓词结论不变时链侧零 Delta（`revisions_are_append_only_and_node_growth_alone_is_not_a_chain_revision`） |
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
