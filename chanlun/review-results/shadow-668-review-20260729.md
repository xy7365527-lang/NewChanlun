# #670 影子评审——#668（N4）事件↔BSP 桥接对象实装（两轴）

- ticket：#670（影子评审，blocked-by #668；母裁定票 #666 四裁 + 2026-07-29 supersede；map #529）。
- 评审对象：分支 `ticket-668c`，尖端 `803b355bea`，diff 基 `1b7ba1a1a5..803b355bea`
  （5 commit / 9 文件 / 1677+ 行；生产改动 = 新模块 `rust/src/theta_v0/classifier/bsp_bridge.rs`
  + `bsp_bridge/tests.rs` + `classifier/mod.rs` +3 行声明）。
- 评审器：claude opus 5，工位 `/tmp/wt-668c`，全程前台单线程，**未派发任何子代理/后台代理**；
  未改动被评审仓库任何代码（复核探针为仓外独立工程 `/tmp/probe668`，源码见 §五附录）。
- **结论：FAIL**（2 条 HIGH 直接命中裁定②的前置条件与裁定⑦的验收门，1 条 HIGH 命中裁定③；
  对象骨架与纪律面大体合规，但「键 = 身份」这个定形前提实测不成立 ⟹ 不宜按现状放行）。
  发现计数：**HIGH 3 / MED 2 / LOW 3**。不关票。

---

## 一、结论摘要

裁定①④⑤（一等对象独立、老对象零改动、零消费接线、Absent 非证伪、终态不复活、append-only、
幂等）与裁定⑧（golden 未动、文书名实）**实测合规**，机器落点逐条查到（§四）。

不合规集中在**键与配对判据**这一条线上，三条 HIGH 是同一根因的三个面：

> 桥接的配对判据用了 `event.interval.1`（C 段右端）做等值匹配。而 C 段右端是 N1 生长纪律
> 明文**不入身份**、随 `as_of` 单调增长的坐标（`cand_event` 里它有专名 `growth_revision`）。
> 用一个非身份、会漂移的坐标当配对键，导致：
> ① 每 episode 至多只有一个一类点能配上（实测 300k：29 个一类 bit 只有 11 个配上，
>    而 29 个**全部**落在某个同级同向同中枢指纹的 Trend 候选区间内）——该写的边没写；
> ② 真值表的「唯一」是这条过窄判据的算术必然：被它挡在检验域外的 18 个点，正是会撞键的
>    那些点。把它们纳入检验域，300k 窗撞键 5 组、最坏一组 4 点共键；
> ③ 验收对拍的参照集是同一判据的逐行复写，因此对 ①② 完全盲，且 20k 窗是 0-vs-0 空对空。

## 二、Standards 轴（与 N1 `cand_event` / N3 `chain_cert` 同族先例的纪律一致性）

### 合规项（实查逐条）

| 纪律 | 落点 | 判定 |
|---|---|---|
| append-only（旧 revision 不改写不删除） | `BspBridgeBook::append`（bsp_bridge.rs:415），`edges` 只 push | ✓ |
| 钟一次写不后移 | `make_revision`（:422-427）：`observed_at` 取 prior 优先、`invalidated_at` 用 `.or()` 不覆盖 | ✓ |
| 终态不复活 | `apply`（:404）`prior.status.is_terminal()` ⟹ 直接 return，同 `ChainCertificateBook::apply` 挡法 | ✓ |
| 同 as_of 重跑零 Delta | `BridgeProjection`（:159）排除三只钟 + revision 计数，`apply`（:408）投影相等即跳过 | ✓ |
| 查簿命中才写 / Absent 非证伪 | `resolve_bridge` 三分支全走 `?`，`observe`（:329-333）查无 `continue`，不产占位 | ✓ |
| 老对象零改动 | `CandidateEvent`/`BspPoint`/`classify_impl`/`classify_with_tower_events` 签名与字段零 diff | ✓ |
| 纯产出零消费（同 N3 先例） | 全仓生产引用面 = `mod.rs:107` 声明一处；`ChainCertificateBook` 同样只在 mod.rs 测试面 + 诊断 bin 被调 —— 先例一致 | ✓ |
| 折叠而非新判据 | `latest_candidates`/`trend_index_by_interval_end` 纯折叠，零 `judge_*`/`extract_*` 改动 | ✓ |
| 文书格式对齐先例 | ADR-0008 = 标题 + 问题陈述 + `## Considered Options` + `## Consequences`，与 0003/0004 同型（本仓 ADR 无 Status 段，非缺项）；CONTEXT.md 两词条含 `_Avoid_`，风格对齐 N0-N3 | ✓ |
| CI 门 | `cargo check --all-targets` 绿（本仓 CI 无 fmt/clippy 门） | ✓ |

方法学自查（commit `c36b168477`，「锚不得含本点自身右端」）**方向正确且值得记功**：三类锚
（`leave_interval` + `retest_interval.0`）确实是 BSP 侧独立、已闭合的坐标，三类键不空洞。
问题只在一类/二类（见 HIGH-1）。

### MED-1 ——「不需要 `resident` 补集」的论证有洞，端死边死存在漏判路径

`bsp_bridge.rs:388-391` 与报告 §四 声称：BSP 点一次置位即冻结事实 ⟹ 每次全量重扫
`classification` 天然覆盖历史点 ⟹ 不需要 `chain_cert` 式的 `resident` 补集。

该论证只覆盖 **BSP 侧**的单调性，未覆盖**事件侧**：`observe` 每次不仅重扫点，还**重新解算配对**。
一旦配对不再解出（同一 `CandidateKey` 的新 revision 把 `interval.1` 生长过本点 —— 这在
`cand_event` 是一等形态，`event_probe` 里专有计数 `growth_revision`，见 cand_event.rs:413），
簿内那条边就**永不再被观察**：

- 该边停在 `Open`，此后 N1 事件转 `Invalidated` 时**不会**同步终态 ⟹ 裁定④「任一端 Invalidated
  ⟹ 边判死」在此路径失效；
- 若新右端恰好也有一个一类点，则因 `BridgeKey` 不含 BSP 侧分量（HIGH-1 附带后果），`apply` 会把它
  判成「同 key 载荷变了」而追加一条 revision，**静默把边改指到另一个 BSP 点**。

现状读数看不到此路径：fresh-full 单次 `advance` 下每个 Trend key 只有 1 个 revision
（复核探针 `keys_with_multi_rev=0`，100k / 300k 均为 0）。但报告 §十一.5 同时声称
「`advance` 的 revision/幂等/终态协议本身与调用次数无关，通用正确」——**该声称在此点不成立**，
且无任何测试覆盖。

先例对照（同族两票都有跨 as_of 平价锁，N4 没有）：
- N1：`classifier::tests::candidate_event_stream_per_segment_full_replay_equals_incremental`
- N3：`classifier::tests::chain_certificate_book_incremental_equals_full_replay`

### MED-2 —— 测试密度低于同族先例，且缺口正对着上述问题

| 对象 | 实现行数 | 测试行数 | 测试数 |
|---|---|---|---|
| N3 `chain_cert` | 1198 | 941 | 24 |
| N4 `bsp_bridge` | 443 | 245 | 7 |

7 例的风格与夹具原则合规（N1 事件侧一律经真实 `CandidateEventBook::advance` 产出，不手搓
`CandidateEvent` 字面量 —— 同 `chain_cert::tests` 先例）。但缺的用例恰好是能暴露 HIGH-1/MED-1 的：

1. 同 key 多 revision（C 段生长）后配对丢失 → 边停 `Open` 不跟随失效；
2. 同 `BridgeKey` 改指另一 BSP 点；
3. 同一 C episode 内多个一类点（实测存在，见 HIGH-1）；
4. `trend_index_by_interval_end` 键冲突（LOW-2）；
5. 多级别正例 —— 现有 7 例全部只在 `level 0` 产边，`level 1` 仅出现在
   `edges_for_bsp_point(1, 40).is_empty()` 一条 negative 断言里。

### LOW-2 —— `trend_index_by_interval_end` 静默覆盖，无留痕

索引键 = `(event_level, side, parent, interval.1)`，**不含** `seg_a` / `c_start` /
`previous_center_start`。两个同 `(level, side, parent, 右端)` 的 Trend 候选会经 `BTreeMap::insert`
静默互相覆盖 ⟹ 静默错配，无 `debug_assert`、无计数留痕。实查无实害：全 revision 口径冲突数
= **0**（100k / 300k 均为 0，见 §五 `idx_all_interval_end_collisions`）。

### 注释照实性（并入 HIGH-2 一并处置）

`bsp_bridge.rs:199-205` 头注断言：「一类点的 `source_index`、三类点 `leave_interval.1`
**都恰好落在**这个坐标（C 段右端）上 —— 同一结构门产出」。实测一类只有 **38%** 落在
（300k：11/29；100k：1/3）。该断言与本票自己报告 §六 的覆盖率表**互相矛盾且未调和**。

## 三、Spec 轴（逐条对 #666 四裁 + supersede）

| 裁定 | 要求 | 判定 |
|---|---|---|
| ① 载体形态 | 独立桥接对象，老对象零改动 | **✓** `BspBridgeEdge`/`BridgeKey`/`BspStructuralKey` 自立户口；老对象与 classify 签名零 diff |
| ② 键 = v2（原式 + 锚段坐标），须先过真值表再造对象 | 三窗 ambiguous=0 | **✗ HIGH-1**：实查通过是检验域窄化的产物 |
| ③ 双向产出 + 主路径内联 + 查簿命中才写 | 两方向都立 | **✗ HIGH-2**（只落一向）；「命中才写」✓；「内联」口径见下注 |
| ④ 端死边死留痕不复活 | Invalidated ⟹ 边终态 | **⚠ MED-1**（机器落点齐，但存在漏判路径且无测试） |
| ⑤ 零消费接线 | p92 / runner 零 diff | **✓** 实查两文件 diff 为空；全仓生产引用面仅 mod.rs 声明 |
| ⑥ 对拍 cmp=0 | 边查询 ≡ source_index 等值拼，逐条 | **✗ HIGH-3**：参照集非独立 + 20k 空对空 + 对拍对象换了 |
| ⑦ golden 未动 | 不为变绿静默重锚 | **✓** `rust/tests/` 零 diff，138 ignored 前后不变 |
| ⑧ 文书 | ADR-0008 + CONTEXT.md 两词 | **✓ 格式/内容在位**；但三处文书含须撤的断言（HIGH-1）+ 报告与分支名实不符（LOW-1） |

> 裁定③「主路径内联」口径注（不判违规）：实装不是嵌进 `classify_impl` 循环体，而是事后对已产出的
> `Classification` + `CandidateStreams` 做全量重扫。这与 N1 词条（CONTEXT.md:124「classify_impl
> 主路径按级内联产出（非 sidecar 事后扫）」）不同，但与 N3 `chain_cert` 现状**一致**（同样不接生产
> 路径）。关键区别在于是否重跑判据 —— `bsp_bridge` 只读既有输出字段、零重判，符合「禁 sidecar 事后
> 补扫」的实质。故按先例判合规，仅登记口径差异。

---

### HIGH-1 [Spec 裁定②] v2 键唯一性实查空洞：被排除出检验域的样本，正是会撞键的样本

**机制。** 一类的锚 **全部由事件键派生**：

```rust
// bsp_bridge.rs:289-290
let event_key = *trend_index.get(&(level_idx, side, parent, point.source_index))?;
let anchor = vec![event_key.seg_a, (event_key.c_start, event_key.c_start)];
```

于是一类的 `BspStructuralKey` = `(rule_version, level, parent, side, class, f(event_key))`，**不含
任何从 BSP 点自身结构读出的分量**（二类同理，锚 = 一类锚坐标一段）。同一 C episode
（同 `seg_a` + 同 `c_start`）内的多个一类点，锚**完全相同** ⟹ 键相同。这正是 v1 报告点名的
「一类点多段递进背驰亦撞」那一类，v2 并没有修掉它。

真值表之所以判 `ambiguous_keys=0`，是因为 `resolve_bridge` / 探针在取锚**之前**先要求
`point.source_index == interval.1`。一个 Trend 候选只有一个当前右端 ⟹ **每 episode 至多一个点
进入检验域**，其余全部记入 `anchor_seg_unresolved` 被丢弃。0 是这条排除规则的算术必然，
不是键的区分力。

**实查（复核探针，300k 窗）。**

| 读数 | 值 | 含义 |
|---|---|---|
| `b1_bits` | 29 | 一类 bit 总数 |
| 生产口径有键的点 | 11 | 其余 18 个（**62%**）`anchor_seg_unresolved`，键**无定义** |
| `groups_production` / `ambiguous_production` | 11 / **0** | 复现真值表结论（检验域 = 11 个点） |
| `hit_episode_range` | **29** | 29 个点**全部**落在某 Trend 候选的 `[c_start, interval.1]` 内（同 level/side/parent） |
| `points_in_multiple_episodes` | **0** | episode 归属**唯一**，不存在任意指派 |
| `groups_episode` / `ambiguous_episode` | 22 / **5** | 把 18 个纳入后：**5 组撞键** |
| `worst_group_size` | **4** | 最坏一组 4 个点共用一把键 |

撞键实例（`level=1`，被破中枢指纹 `(145672, 916600000000, 957398000000)`，`Short`，
`seg_a=(144020,147967)`，`c_start=148038`）：`source_index ∈ {148528, 148864, 149213, 149699}`
—— 四个卖 1 点，同一把 v2 键。另 4 组见 §五 `PROBE670_COLLIDE`。

**两个独立层次的判定**（请编排者分开裁）：

1. **不依赖任何我构造的规则**：v2 键对 62% 的一类 bit **没有定义**（键存在与否取决于该点是否恰好
   坐在候选的当前右端上）。裁定②问的是「同一走势同一点类唯一性」—— 一把只对 38% 的样本存在的键，
   不构成 BSP 点的结构身份。这一层单独就足以判前置条件未满足。
2. **依赖 episode 归属规则**（`c_start ≤ si ≤ interval.1`，同 level/side/parent；此规则是我按裁定②
   叙事「一类锚 = 背驰确认段」的自然外推，非生产规则，如实标注）：纳入后 300k 窗 5 组撞键。

**附带后果（与 MED-1 交叉）**：`BridgeKey` 不含 BSP 侧独立分量，而 `bsp_level`/`bsp_source_index`
被明文定为「载荷，不是身份分量」（bsp_bridge.rs:96-100, 140-143）⟹ 增量路径下同一 `BridgeKey`
可在不同 `as_of` 指向不同的 BSP 点，`apply` 会判「载荷变了」追加 revision 而**静默改指**。

**须撤的文书断言**（三处口径一致地说反了）：报告 §六 与 §一（「配对覆盖率与键唯一性两者正交」）、
ADR-0008 Consequences（「这不影响本票裁定②的真值表判据（唯一性已证成立）」）、CONTEXT.md 新词条
`_Avoid_`（「拿配对覆盖率低反推键公式错误」）。实测二者**由同一 join 判据耦合**：正是因为覆盖率低
（排除 62%），才得出唯一性成立。CONTEXT.md 那条 `_Avoid_` 尤其危险 —— 它把唯一有效的反推路径
写成了禁令。

---

### HIGH-2 [Spec 裁定③ / 接线] 一类 62% 应有的边未写；报告的归因方向被证伪

dispatch 明文：配对覆盖率不在评审范围（#688），**但若低覆盖率实为接线 bug（该写边没写）而非
候选域结构性错位，照 HIGH 报**。本条落在该口径内。

**实查。** 复核探针两窗：

| 窗口 | 一类 bit | 生产口径命中 | 落在某候选 episode 区间内 | 归属歧义 |
|---|---|---|---|---|
| 100,000 | 3 | 1（33%） | **3（100%）** | 0 |
| 300,000 | 29 | 11（38%） | **29（100%）** | 0 |

即：**每一个一类点都存在一个同级别、同向、同被破中枢指纹、且区间覆盖本点的 Trend 候选**。
缺的不是候选（候选在场、可解、归属唯一），缺的是 join 判据 —— 桥接要求
`point.source_index == event.interval.1`（C 段**右端**等值），而这 18 个点落在 episode 内部。

**为什么这是判据错而不是数据事实**：C 段右端按 N1 纪律明文**不入身份**（`CandidateKey` 头注
「C 右端与 as_of 均不在键中」）且随 `as_of` 生长（`growth_revision`）。桥接头注自己也重述了这条
纪律（:95-100），却在配对判据上恰恰用了它 —— 于是「谁跟谁配对」这件事变成了 `as_of` 的函数。

**与裁定③「双向产出」的关系（这是规格面的根因）**：裁定③要求「事件出生回挂一类点、二/三类点
出生回挂事件，**两方向都立**」。实装只有 `observe` 一个方向：遍历 `classification` 的点 → 反查
`trend_index`。**没有**「遍历事件 → 回挂它覆盖的一类点」这一向；而后者正是能把这 18 条边写出来
的方向。报告 §三 把「三个点类各有一条解析路径」称为「双向」—— 那是三个点类，不是两个产出方向。

**报告 §六.1 的归因须撤**：报告把 38% 命中率归因于「`judge_first_cached`（BSP 抽取）与
`cand_event::observations_for_level`（候选扫描）两条路径的 trend-gate 对齐细节，根因未查，
可能与 `center_trend_gate` 的中枢级 vs 段级粒度有关」。实测证伪该方向：两条路径的产出**是**对齐的
（候选对每一个一类点都在场且区间覆盖），不对齐的是桥接自己的 join 判据。顺带：两条路径同级产出这一点
也已核实 —— `cand_event::observations_for_level(level_idx, ...)`（mod.rs:625）与该级 `bsp`
（mod.rs:651-658）在 `classify_impl` 同一轮循环里，输入分段同源
（`candidate_scan_inputs(is_l0, &l0.segments, &units)`），**不存在** nest 侧
`event_bsp_book_level` 那种 ℓ-1 移位。故同级 join 本身是对的，错的只是右端等值。

**不在本条范围**：三类近零覆盖（`trend_events` 42 vs `b3_points` 890）。那是
`judge_third_cert` 的纯几何离开判据与 `first_structural_gates` 三门不同构 —— 属候选域结构性
错位，归 #688，Absent 非证伪，本评审不判。

---

### HIGH-3 [Spec 裁定⑥] 验收对拍无信息量：参照集是同一判据的逐行复写，且 20k 空对空

**参照集非独立。** `reference_join`（battery:75-151）与生产模块逐行同构：

| 步骤 | `bsp_bridge.rs` | `p_issue668_bsp_bridge_battery.rs` |
|---|---|---|
| latest 折叠 | `latest_candidates`:188-196 | :76-81（逐字同构） |
| 索引 | `trend_index_by_interval_end`:206-220，键 `(event_level, side, parent, interval.1)` | :82-89，同键 |
| 一类 | `:289` 查 `point.source_index` | `:104` 同查 |
| 三类 | `:299` 查 `entry.leave_interval.1` | `:114` 同查 |
| 二类 | `:311` 查 `anchor_idx` + `resolve_second_class_anchor` 反查 | `:130-144` 同逻辑重写 |

两侧共享**全部**设计判断，包括错的那些。它能捕获字段誤植 / off-by-one，**不可能**捕获判据层错误
（HIGH-2 就在它眼前而它 cmp=0）。bin 头注与报告 §五 对此有如实披露（「共享同一 v2 键公式」
「本对拍验的是接线」）—— 披露在位，但**验收门本身失效**这件事没有上报。

**规模无信息量。** 20k：`reference_edges=0`、`produced_edges=0`、`cmp=0` —— **空对空**
（`trend_events=0`，该窗结构上不可能产出任何边）。三窗合计参与逐条比对的边 = **12 条**
（0 / 1 / 11）。#668 编排者抽验③把「对拍电池复跑 20k：cmp=0」记为通过的验收门 —— 该门在 20k
是重言式。

**对拍对象换了。** 裁定⑥要求对拍**现役** `source_index` 等值拼（#666 正文点名 p92 / runner）。
实查现役 p92 侧的事件↔BSP 绑定 = `nest::terminal_bits_at_event` → `terminal_bits_in_book_core`
（nest.rs:658-778），其判据是：

- 账本级别 **ℓ-1 移位**（`event_bsp_book_level`，nest.rs:493）；
- 窗口 `[interval_b.0, turn_source]` 闭区间内的**最早合法点**（`min_by_key(source_index)`），
  **不是** `source_index` 精确等值 —— 精确等值那条是 `TerminalMatch::Exact`，明文「诊断常数臂，
  不作生产默认」；
- 合法性 = `confirm_side` ∧ owner 两族精确判同（一/三类按 `(zd,zg)` 带判同；二类按身份锚判同）。

与桥接的「同级 + 右端等值 + 指纹三元全等」是**不同判据**。该现役拼缝全程未被引用。
若因跨对象族（`NestCandidateEvent` vs N1 `CandidateEvent` 是两套事件体系）导致对拍不可直接执行
——这是个真实困难——正确处置是**上报不可执行、请编排者改验收门**，而非以自身复写替代后交
「cmp=0 验收通过」。

---

### LOW-1 [文书] 报告与被评审分支名实不符（重贴后未更新）

1. §七 测试计数 `2533 → 2540`：在 `ticket-668c` 上**不可复现**。实测 `cargo test --lib --release`
   = **2555 passed / 0 failed / 138 ignored**；`cargo test --lib`（debug）= **2557 / 0 / 138**
   （与 #668 编排者抽验⑤ 的 2557 一致 —— 抽验跑的是 debug 臂）。差值来自报告所依据的
   `ticket-668b` 基底（含 ticket-667 侧线）。净增 7 这个**相对**读数仍成立（新增 7 个单测）。
2. §十 commit hash 列（`03b0a3339a` / `1209166ee9` / `00fa6fc3ea` / `3e55c261c9`）在
   `ticket-668c` 上**均不存在**；重贴后为 `76673bbc90` / `550a7a6fd7` / `c36b168477` /
   `5d46d1fc62` / `803b355bea` 五个。090 纪律要 hash 溯源，重贴后的报告须补映射表。
3. §一 声称工位起点 = 「干净 main 尖端 `9b68eb1967`」，与 #668 编排者抽验④「`ticket-668b`
   起点为 ticket-667 侧线，与 dispatch 指定不符，v1 同犯」**直接矛盾**。执行器当时以为自己在干净
   main 上 —— 这条错误自述本身是可复发的偏航信号，建议在 #668 评论里订正登记。

### LOW-3 [文书] ADR 编号跳号，0005-0007 在仓内无实体

`docs/adr/` 现有 `0001`-`0004` + `0008`。CONTEXT.md 引用的 **ADR-0006**（#537，行 116/120）与
**ADR-0007**（#540，行 124）在仓内**无对应文件**（全仓 `find` 零命中）。0008 的编号占位假定
0005-0007 存在，读者按号索引会落空。非本票新增的问题（N1/N3 只在 CONTEXT.md 里挂了号、没落盘
ADR），但本票是第一个把 0008 实体落盘的票，建议顺手在 #529 下登记补号或改为按票号命名。

---

## 四、复跑读数（全部本工位实测，`/tmp/wt-668c`，`CARGO_TARGET_DIR=/tmp/wt668c-target`）

### 零消费 / golden / CI 门

```
git diff 1b7ba1a1a5 803b355bea -- rust/src/bin/p92_nest_replay_postruling.rs \
                                  rust/src/theta_v0/backtest/runner.rs   → 空
git diff --stat 1b7ba1a1a5 803b355bea -- rust/tests/                     → 空
cargo check --all-targets                                                → 绿（CI rust-check 同款）
cargo test --lib                    → 2557 passed; 0 failed; 138 ignored
cargo test --lib --release          → 2555 passed; 0 failed; 138 ignored
```

### v2 真值表探针（本票 `p_issue668_bsp_key_truth`）

```
ISSUE668_TRUTH_V2 bars=20000 levels=3 bsp_points_total=54 bit_instances=54 distinct_keys=37
                  anchor_unresolved=17 anchor_seg_unresolved=0 ambiguous_keys=0
```
与报告 §一 20k 行**逐位一致** ✓（distinct=37 / unresolved=17 / ambiguous=0）。

### 对拍电池（本票 `p_issue668_bsp_bridge_battery`），三窗

```
bars=20000   trend_events=0  pan_events=41  b1=0  b2=17  b3=37  hit_by_class={}
             reference_edges=0  produced_edges=0  missing=0 extra=0 cmp=0     ← 空对空
bars=100000  trend_events=7  pan_events=227 b1=3  b2=88  b3=257 hit_by_class={"Buy1":1}
             reference_edges=1  produced_edges=1  missing=0 extra=0 cmp=0
bars=300000  trend_events=42 pan_events=698 b1=29 b2=280 b3=890
             hit_by_class={"Buy1":4,"Sell1":7} b1_by_level={0:21,1:8} b1_hit_by_level={0:8,1:3}
             reference_edges=11 produced_edges=11 missing=0 extra=0 cmp=0
```
三窗全部与报告 §五 / §六 表**逐位一致** ✓（cmp=0 / 覆盖率细分行均复现）。

### 复核探针（仓外工程 `/tmp/probe668`，本评审新写，未入仓）

```
PROBE670     bars=100000 trend_keys=7  trend_revs=7  keys_with_multi_rev=0 idx_all_interval_end_collisions=0
PROBE670_B1  b1_bits=3  hit_latest_only=1  hit_any_revision=1  hit_episode_range=3  points_in_multiple_episodes=0
PROBE670_KEY groups_episode=3  ambiguous_episode=0 worst_group_size=1 groups_production=1  ambiguous_production=0

PROBE670     bars=300000 trend_keys=42 trend_revs=42 keys_with_multi_rev=0 idx_all_interval_end_collisions=0
PROBE670_B1  b1_bits=29 hit_latest_only=11 hit_any_revision=11 hit_episode_range=29 points_in_multiple_episodes=0
PROBE670_KEY groups_episode=22 ambiguous_episode=5 worst_group_size=4 groups_production=11 ambiguous_production=0
PROBE670_COLLIDE level=0 fp=(181542,1469160000000,1480011000000) side=Long  seg_a=(181720,182233) c_start=182297 points={182331,182478}
PROBE670_COLLIDE level=0 fp=(225525,1156432000000,1199000000000) side=Long  seg_a=(225308,225996) c_start=226281 points={226342,226586}
PROBE670_COLLIDE level=0 fp=(225525,1156432000000,1199000000000) side=Long  seg_a=(225308,225996) c_start=226725 points={226777,227011}
PROBE670_COLLIDE level=0 fp=(233342,1093100000000,1105000000000) side=Short seg_a=(233466,234019) c_start=234038 points={234256,234332}
PROBE670_COLLIDE level=1 fp=(145672,916600000000,957398000000)  side=Short seg_a=(144020,147967) c_start=148038 points={148528,148864,149213,149699}
```

三条读数值得单独指出：

- `keys_with_multi_rev=0`（两窗）：fresh-full 单次 `advance` 下每个 Trend key 只有一个 revision。
  这意味着 MED-1 的漏判路径**在现有三窗读数里不可观测** —— 它是协议层的洞，不是当前数据里的错。
  如实标注：MED-1 为**结构可达 / 未实测**。
- `idx_all_interval_end_collisions=0`（两窗）：LOW-2 的静默覆盖当前无实害。
- `hit_episode_range` 等于 `b1_bits`（3/3、29/29）且 `points_in_multiple_episodes=0`：HIGH-2 的
  「候选在场且归属唯一」是**实测事实**，不是推断。

## 五、附录：复核探针核心逻辑（仓外，`/tmp/probe668/src/main.rs`）

载入器与 `classify_with_tower_events` 调用逐字复用本票两个 bin 的写法（略）。决定性部分：

```rust
// 口径一（生产）：每 key 取最新 revision，再按 interval.1 索引
let mut idx_latest: BTreeMap<(u32, Side, Fp, usize), CandidateKey> = BTreeMap::new();
for event in latest.values() {                       // latest = 按 CandidateKey 折叠
    if event.kind != CandidateKind::Trend { continue }
    idx_latest.insert((event.event_level, event.key.side, fp(event), event.interval.1), event.key);
}
// 口径二（全 revision）：同 key 的每一次 interval.1 都进索引 —— 测「最新折叠是否丢配对」
// 口径三（episode 区间）：(c_start, 最新 interval.1) 闭区间 —— 测「候选是否在场且覆盖本点」
for (level_idx, level) in classification.levels.iter().enumerate() {
    for (set, side) in [(point.bits.buy1, Side::Long), (point.bits.sell1, Side::Short)] {
        if !set { continue }
        b1_total += 1;
        let Some(fp) = center_fp(level, idx) else { continue };
        let si = point.source_index;
        if let Some(k) = idx_latest.get(&(lvl, side, fp, si)) {      // 生产口径命中
            hit_latest += 1;
            groups_prod.entry((lvl, fp, side, k.seg_a, k.c_start)).or_default().insert(si);
        }
        if idx_all.contains_key(&(lvl, side, fp, si)) { hit_all_rev += 1 }
        let owners: Vec<_> = episodes.iter()                          // episode 归属
            .filter(|(el, es, efp, cs, e1, _)| *el == lvl && *es == side && *efp == fp
                                               && *cs <= si && si <= *e1)
            .map(|(.., k)| *k).collect();
        if !owners.is_empty() {
            hit_episode += 1;
            if owners.len() > 1 { multi_episode += 1 }
            for k in &owners {                                        // v2 键分组
                groups.entry((lvl, fp, side, k.seg_a, k.c_start)).or_default().insert(si);
            }
        }
    }
}
let ambiguous_episode = groups.values().filter(|s| s.len() > 1).count();
```

探针只读：零 `judge_*`/`extract_*` 调用，只读既有输出字段 + 三次纯折叠；不在被评审仓库内，
`git status` 全程干净（数据软链 `analysis/data_cache/btc_1m_full.json` 已 gitignore）。

## 六、处置建议（不动手修，交编排者裁）

1. **键公式回炉第三轮（裁定②）**：一类/二类的锚需要一个**从 BSP 点自身读出**的、已闭合的
   区分分量。可选面（不预判）：一类锚补 `struct_break_dir` + 本点所在段的**左端**
   （闭合坐标，非右端）；或把「episode 内第 k 次递进背驰」的序号语义显式化；或裁定「同 episode
   多一类点」在教义上是否本该塌缩为一个身份（若是，则问题移到 BSP 抽取侧而非键侧）。
2. **配对方向补齐（裁定③）**：补「事件侧 → 回挂其 C episode 覆盖的一类点」这一向，join 判据从
   「右端等值」改为「episode 区间覆盖」（与 N1 生长纪律一致：身份用 `(seg_a, c_start)`，右端只用于
   区间上界）。改完后 §五 的 `hit_episode_range` 就是新覆盖率上界（100k 3/3、300k 29/29）。
3. **验收门重定（裁定⑥）**：现役拼缝跨对象族不可直接对拍这一点须先上报裁；替代方案建议
   = 跨 as_of 平价锁（对齐 N1/N3 两条既有先例）+ 一条会因 HIGH-2 变红的负控用例，而非同构复写。
4. **文书订正**：撤「覆盖率与唯一性正交」三处断言（报告 §一/§六、ADR-0008 Consequences、
   CONTEXT.md `_Avoid_`）；订正 `bsp_bridge.rs:199-205` 头注的对齐断言；补 LOW-1 三项名实。
5. **保留**：对象骨架（`BspBridgeEdge`/`BspBridgeBook`/append-only+终态+幂等机器落点）、
   三类锚公式、零消费接线、方法学自查那条「本点自身右端不入锚」的纪律 —— 这些经查都成立，
   回炉时不必推倒。

不关票。
