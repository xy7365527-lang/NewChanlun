# #603 交付：身份桥接放宽实装（档2 pending 槽队列 + 档1 暂认中枢严格同锚）

> 角色：实装执行层（全程前台单线程，未派 Task/子代理/后台任务）
> 基线：`kimi-nest-mainline-20260717` @ `6ebe18eda1`（#613 收口后）
> 票据：#603（#599 编排者裁定 2026-07-28 的实装票；map #597 线）
> 产物：`/tmp/wt603-{pre,post}-{20k,100k}.{stdout,p116,dump,stderr}`、`/tmp/wt603-post-m8-{p3fold,wf7,wf8}`
> 探针（仓外、不提交）：`/tmp/wt603_{cov,tier2,own,final}.py`

## 0. 结论摘要（先说最重要的偏差）

两档均已按裁定语义实装、护栏全绿、测试门唯一红 #491。**但两条验收指标一成一不成，且不成的那条
连带推翻了 #599 对缺口成因的机制归因**：

| 票面验收 | 目标 | 实测 | 判定 |
|---|---|---|---|
| 1. L1 覆盖率 | 77.2% → **94.1%**（挽回 34） | 77.2% → **78.7%**（挽回 **3**，全部来自档1） | **未达成**（档2 挽回 0） |
| 2. ForceOvertake 纠误 | 35 → **33**（2 例误计） | `force_overtake_claimed=2`，两例正是 #599 点名的 15984/71448 | **达成** |

**归因（实测，非推断）**：#599 §3.1 把 33 只未覆盖信号判给「档2 槽被占」，其判据是「该信号活跃期内
存在 `frontier_start ≠ own_c_start` 的诊断行」——那只证明「槽当时指向别处」，**没有证明「若排队则
该候选会被观测到」**。本票把队列真做出来之后逐只复核，33 只里没有一只是因为「槽位不够」而丢失的：

| 实测桶 | 只数 | 队列为何救不了 |
|---|---:|---|
| C 段**从未**进过队列 | 18 | parser 的线段确认是**批量**的：一次 `append` 同时确认多条链式段，中间段从未单独当过 pending 槽 ⟹ 它从来没有进过任何槽，队列深度再大也保存不到它 |
| 占槽期 B 恒异于完成 B | 17 | 该候选确实占过槽，但活窗侧 `nearest_confirmed_center_idx` 给出的 B 在它整个存活期内**不变**（实测 c=17170 那只：占槽 12 次 B 恒 16283，完成信号 B=16724）。活窗与完成两路径的 centers 集合来自不同投影（活窗 = `tower[level]` run 分组 seeds；完成 = level_view 侧），**不是** #599 §1.1 说的"稀疏采样盲区"这种时序差——延长观测不改变 B |
| 占槽期全 MISS | 11 | `structure_not_locatable`：结构定位失败，与槽容量无关（#599 §6 已把 locate_fail 判出范围，实测该桶比 #599 的 3 只大得多） |
| **合计** | **46** | |

**由此产生的待裁事项（§7 遗留 1）**：档2 在 BTC 100k 上零挽回，但它是编排者已裁采纳的机制，本票不
替裁——保留实装并照实登记；是否回退，附新证据上呈。

---

## 1. 档2：pending 槽队列（`ActiveFrontierQueue`）

### 1.1 语义与深度选择

`nest_lifecycle.rs` 新增 `ActiveFrontierQueue`（深度常量 `ACTIVE_FRONTIER_QUEUE_DEPTH = 3`）。
队列成员**全部**由既有唯一构造点 `active_segment_frontier`（parser `tail`）产出——#523 永禁清单
「禁止从 confirmed segments 回放重建」逐字未动，本档只是把曾真实出现过的槽快照多留几个 bar。

**深度 3 的选择依据照实登记**：#599 §3.2 对 33 只实例统计链深，分布 1 跳 25 / 2 跳 7 / 3 跳 1，
未观测到 ≥4。裁定采纳「默认 3」= 覆盖实测全部链深。`formalization-validity-domain` 标注：
**L2，仅 BTC 100k 单窗**，不是规格常量。补记：本票实测该分布与「队列能否挽回」**无因果关系**
（§0 归因），故深度选择在本数据集上不可证伪——depth=1/2/3 的挽回量同为 0。

### 1.2 三处必需的正确性约束（每一处都由实测钉出，不是保守冗余）

| 约束 | 不设时的实测后果 | 判据 |
|---|---|---|
| **槽的粒度是段，不是快照值** | `ActiveSegmentFrontier` 的 `extreme_at` 随 bar 推进，逐值去重会把同一候选段的不同延展阶段当成并行候选各占一槽：L1 `Supersedes` 19814 → **30281**，且同一候选在同一 prefix 有多个"当下状态"（违反活假设状态机的时间语义） | `observe` 按 `start_index` 去重，同起点新快照原地替换并移到队尾 |
| **历史候选的活窗右端冻结** | 右端继续随 `as_of` 延展 = 把该段终结之后的行情算进它的力度：`ForceOvertake` 35 → **43** | `LifecycleWindowStem::frozen_end`，取候选快照的极值结构点 `extreme_at`（沿用「禁用 as_of 冒充结构点」纪律） |
| **过期快照守卫** | 快照拍摄于该段还是 pending 时，parser 确认时可能**截短**（实测 (15738, extreme_at=15862) 的真实段是 (15738,15821)）；继续用它 ⟹ `ForceOvertake` 35 → **53**，涨出来的 13 只全是本来 `Confirmed`/`NeverConstituted` 的身份被活窗判负 | 候选起点在 confirmed 侧已有对应段时，右端须逐位相同才继续用；不同 ⟹ 落显式原因码 `stale_candidate_redivided`（BTC 100k 命中 2382 次） |

三处修完后，`ForceOvertake` 回到基线 40（L1 35）、`Supersedes` 24569 → 24577（+8）。

### 1.3 有效域登记（L2 不外推）

L2 的 C 腿仍**只**由当下槽派生（`queue.current()`）：历史槽重扫 L1 层窗口会拿过期的行进中单元
冒充当下（#523 同类错配），且 #599 的量化只覆盖 L1 完成信号。L2 读数逐项零回归（§4）。

---

## 2. 档1：暂认中枢回溯认领（严格同锚）

### 2.1 判据

新增 `bridge_by_center_upgrade`：`level/side/kind/seg_a/seg_c_full.0` 五项全等 ∧
`old.b_center_start < new.b_center_start`（**严格单调前进**）。与 `bridge_identity` 在
`b_center_start` 上互斥（相等 vs 严格小于）⟹ `advance` 第 1 步先桥后认领，匹配不重叠。

**跨锚零实装**（#599 §4.2 教义否定，编排者采纳）：`seg_a` 全等是硬约束，函数不提供任何放开它的
分支；`issue603_center_upgrade_rejects_cross_anchor_and_backward_center` 正面锁定跨锚/B 反向/
C 左端不同三种拒绝，diff 自证无此路径。

**`bridge_match` 语义不动**：认领走独立的 `center_upgrade_match`，不进
`bridge_entry`/`terminal_bridge_hit`/`completion_signal_seen` 三处判定——认领是「两个身份之间的
关联」，不是「它们是同一个身份」；塞进桥语义会让终态前身把新身份一并吸收（禁复活的适用面被误扩）。

### 2.2 两形态（与 #599 §3.3 预测的偏差 + 为何只能如此）

| 前身状态 | 本票实装 | #599 §3.3 的预测 |
|---|---|---|
| 仍 `Provisional` 且非倒退 | **迁移**：`remove` + 五钟继承 + `CenterUpgraded{from}` 链留痕（与 `Supersedes` 同款） | 一致 |
| 已终态 | **认领留痕**：新身份独立建仓（`Observed` 后紧跟认领修订 + `superseded_from`），**前身条目一个 bit 不动** | 预测「两条历史合并为同一身份、entries 285→283、前身从反超桶移出」 |

**为什么终态前身不能合并**：#599 点名的 2 例（15984/71448），其 wrong-B 前身在完成信号到账前
**已各自独立走到 `Invalidated{ForceOvertake}`**（dump 实证：15857 / 71403）。要兑现「两条历史
合并成一条」，必须把一个已终态条目迁移并重置为 Provisional 继续走完成路径 = **复活**，与
E2E §1:83「终态吸收、禁复活」+ 模块头 090 登记「终态钟只写一次、禁删除模拟失效」直接冲突。

本票的解法**不是折中**：账本层一个 bit 不改（三条不变量全保），纠误改在**口径层**——
`LifecycleSettlementStats::force_overtake_claimed_count` 由账本自足反查（认领方 entry 的
`superseded_from` + `CenterUpgraded` 修订），`force_overtake_count - force_overtake_claimed_count`
即纠误后的反超数。实测 `force_overtake=40 force_overtake_claimed=2` ⟹ L1 侧 35 − 2 = **33**，
与票面「35→33」逐位对上。

### 2.3 档1 纠误 2 例逐条登记（票面验收 3）

| # | 完成信号 | 候选身份 | 曾被误计的前身 | 前身终局 | 认领后终局 | 形态 |
|---|---|---|---|---|---|---|
| 1 | `as_of=15984` | `seg_a=(15191,15441)` `c=15738` `b=14709` | 同锚同 C、`b=14193`，`seg_c_full=(15738,15857)`，`Observed@15789`→`FirstProvable@15789`→`ForceOvertake@15857` | **ForceOvertake**（账本留档不改） | **Confirmed**@15984 | 认领留痕 |
| 2 | `as_of=71448` | `seg_a=(71236,71266)` `c=71341` `b=71033` | 同锚同 C、`b=70396`，`seg_c_full=(71341,71403)`，`Observed@71367`→`FirstProvable@71367`→`ForceOvertake@71403` | **ForceOvertake**（账本留档不改） | **NeverConstituted**@71448 | 认领留痕 |

两例的「误计」性质（#599 §5-1 口径正确性修复）：前身的反超判定作出时，B 参照是**暂认中枢**
（更早、更远的那只）；更近的中枢确认后，同一候选的诚实终局由完成路径给出（Confirmed /
NeverConstituted）。前身那条反超是「更精确的 B 出现之前的暂时状态」，故从反超分母剔除。

**第 3 条认领（不在 #599 点名之列，本票新发现）**：`as_of=66981`、`seg_a=(66433,66646)`、
`c=66897`、`b` 从 65627 升到 66077。其前身在认领时仍是 `Provisional`（`Observed@66923`），故走
**迁移**形态 ⟹ 新身份继承 `observed_at=66923 < 66981` ⟹ 这一只被真正挽回（§3 覆盖率 +1 的来源）。

---

## 3. 验收 1：覆盖率对拍（照实，不预设凑数）

L1 完成信号 202 只。两种口径分列（**必须分列**——档1 的认领链会改 `b_center_start`，而
`b_center_start` 是覆盖率判据五元 key 的一部分）：

| 口径 | pre（`6ebe18eda1`） | post | Δ |
|---|---:|---:|---:|
| **身份键口径**（只认以该完成信号五元 key 记的 `Observed` 行） | 156/202 = **77.2%** | 156/202 = **77.2%** | **0** |
| **候选口径**（沿 `CenterUpgraded` 认领链上溯最早观测钟——裁定「回溯认领」的语义口径） | 156/202 = 77.2% | **159/202 = 78.7%** | **+3（+1.5pp）** |

+3 的构成：15984 / 66981 / 71448，**全部来自档1**；档2 贡献 **0**。

与票面 94.1% 的差额 = 31 只，逐桶归因见 §0（18 只从未进队列 / 17 只 B 不更新 / 11 只结构不可定位，
桶间有重叠，合计 46 只未覆盖）。**未做任何凑数处理**：探针判据、原始 dump 与逐只 as_of 清单全部
在案（`/tmp/wt603_final.py`）。

诚实标注：候选口径下 15984/71448 这两只，其**新身份自身**的 `observed_at` 仍是完成时刻（认领留痕
不继承钟）；它们算 covered 是因为「该候选以暂认中枢身份在 15789/71367 就已被观测到」——这正是
裁定「回溯认领」的字面语义，但与「新身份自己有更早的 Live」不是一回事，读数不可互相冒充。

---

## 4. 验收 2：反超 / 寿命 / 两类消失原因码分列重报 + L2 零回归

`P421_LIFETIME_SUMMARY`（BTC 100k，全级别）：

| 读数 | pre | post | 说明 |
|---|---:|---:|---|
| entries | 302 | **304** | +2：历史候选被观测产生的新身份 |
| first_provable | 208 | 208 | — |
| confirmed | 135 | 135 | — |
| **force_overtake** | 35(L1)+5(L2)=40 | 40 | 账本数不变（伪反超已由 §1.2 三处约束消除） |
| **force_overtake_claimed**（本票新增） | — | **2** | 纠误口径：L1 35 − 2 = **33** |
| never_constituted | 74 | 74 | — |
| **identity_vanished_refuted** | 29 | **31** | +2 |
| **identity_vanished_seam** | 23 | 23 | 不变（档2 的排队对象天然跨锚，与 `same_anchor` 判据不重叠——#599 §3.4 这一条推断成立） |
| flash_terminal | 75 | 74 | −1 |
| nonflash_count / median | 226 / 88.0 | 229 / **95.0** | 中位 +7.0；#599 §3.3 预测的「25–437 bar 提前量」未出现（那批实例根本没被挽回） |
| force_lifetime median | 64.5 | 64.5 | 不变 |

**L2 零回归**（#601/#613 面）：`l2:window=342`、`l2:center_not_consolidation=329`、
`l2:lower_frontier_not_absorbed=558`、`l2:structure_not_locatable=467`、`l2:tower_level_absent=520`
——逐项与 pre 相同；L2 覆盖率 17/38 = 44.7% 不变；L3 8 只全未覆盖，不变。

L1 原因码面（`P527_L1_LIVE_OUTCOMES`）：`l1:window` 610 → **1008**（队列候选真在产窗），
新增 `l1:stale_candidate_redivided=2382`，`l1:center_not_consolidation` 388 → 723、
`l1:structure_not_locatable` 818 → 1335（历史候选各自落码，非当下槽读数变化）。

---

## 5. 验收 4：字节护栏

| 面 | 窗 | cmp | SHA-256 |
|---|---|---|---|
| p123 stdout | 20k | **0** | `bd9ac1d655f9d615…`（与 #527/#601 记录逐字相同，跨票一致） |
| p123 stdout | 100k | **0** | `d8b69c180c23c5e3…`（同上） |
| p123 P116 dump | 20k / 100k | **0** | — |
| m8 `p3fold/wf7/wf8` trades | — | **0** | 与仓内 golden 逐字节相等 |
| m8 `p3fold/wf7/wf8` tower_events | — | **0** | 同上 |

**lifecycle dump 是本票本体，不作 cmp=0**（同 #527/#601 口径）。post SHA-256：
20k `794581bf8e4e9917…`、100k `8d7ad3fcd05284b8…`。dump 格式变化：诊断行新增 `cand_age=`，
`frontier_start=` 改记**产该行的候选**起点（队列化前恒 = 当下唯一槽，语义相容）；
`PAN_LIVE_RECOMPUTE` 新增 `queue=` 字段；`REV` 出现新修订种类 `CenterUpgraded`。

**#533 golden 重锚**（按 `rust/tests/issue533_p123_byte_guardrail.rs:44-47` 纪律：已审阅、故意、
同 PR 说明并引用本票 #603）：只重锚 **dump 面** 三处——`issue533_p123_2000_dump.golden.txt` 全文、
`issue533_p123_{20000,100000}.sha256` 的 `dump` 行；两个 sha256 文件的 **`stdout` 行一个字符未改**，
即 stdout 面零漂移由 golden 自身自证。重锚后护栏门 2 passed / 0 failed。

---

## 6. 验收 5：测试门

- `cargo test --lib`（`CARGO_TARGET_DIR=/tmp/kimi-nest-target-603`）：**2045 passed / 1 failed / 137 ignored**，
  唯一红 = `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`（**#491**，既线）。
- `cargo test --bin p123_fast_replay`：4 passed / 0 failed。
- 新增测试（票面点名三项全覆盖）：
  | 测试 | 覆盖 |
  |---|---|
  | `issue603_frontier_queue_depth_behaviour` | **队列深度行为**：段粒度去重、同起点快照原地更新、FIFO 淘汰、`None` 不清队且当下槽为空、回到既有起点移回队尾、depth=1 退化回单槽 |
  | `issue603_center_upgrade_claims_provisional_predecessor_by_migration` | **同锚暂认回溯认领**（迁移形态）：五钟继承、`superseded_from` 链、不产生终局 |
  | `issue603_center_upgrade_leaves_terminal_predecessor_untouched` | **同锚认领**（留痕形态）：终态前身逐位不动、新身份不被吸收、`force_overtake_claimed_count` 口径 |
  | `issue603_center_upgrade_rejects_cross_anchor_and_backward_center` | **跨锚拒绝** + B 反向拒绝 + C 左端不同拒绝 + 与 `bridge_identity` 互斥 + 端到端零认领修订 |
  | `lifecycle_window_stem_freezes_right_edge_for_stale_candidate`（p123 bin） | 历史候选右端冻结、逐 bar stem 逐位相同 |
- 零新增编译警告（`p123_fast_replay.rs` / `nest_lifecycle.rs` 两文件均无警告输出；lib 的 37 条为既有）。

---

## 7. 遗留（不替裁，上呈）

1. **档2 在 BTC 100k 上零挽回，成因与 #599 的归因不同**（§0）。裁定是在「33 只属槽被占」这一
   归因下作出的，该归因经本票实测不成立。是否保留 depth=3 队列（当前状态：机制在案、护栏全绿、
   口径扰动为 entries +2 / vanish_refuted +2 / nonflash 中位 +7.0）、回退、或转向真正的成因，
   归编排者。**本票不自行删除已裁定的机制**。
2. **真正的两条成因线索**（本票实测钉出，均超出 #603 Scope，建议另开票）：
   (a) **parser 批量确认跳过的中间段**（18 只）——这些段从未当过 pending 槽，任何"槽容量"方案
   都够不着；要救只能改 provider 的候选发现口径（而非观测容量）。
   (b) **活窗与完成两路径的 centers 集合口径差**（17 只）——活窗侧 B 在候选整个存活期内不更新
   （实测 c=17170 占槽 12 次 B 恒 16283，完成信号 B=16724）。#599 §1.1 把它归为"稀疏采样盲区"
   的时序差，实测显示是**两路径投影不同源**，不是采样密度问题。
3. **历史候选截断口径的有效域**：「`end_index ≤ 候选起点` 的段 ≡ 该候选占槽时的 confirmed 集合」
   依赖段账本对该前缀不再回缩重划；古怪线段导致的尾段跨 bar 重划会破坏该等价。与 #613 F1 注记
   点名的「L1 confirmed 尾段可重划」是同一条待查线索。
4. **`force_overtake_claimed` 只在 stderr 诊断面**，未进任何真值/证书路径；下游若要用纠误后的
   反超率，须显式减去该项（口径不自动生效）。
5. **档3 未实施**（裁定判出范围），本票 diff 无相关路径。

---

## 8. 结果包六要素

1. **结论**：档2（pending 槽队列 depth=3）与档1（暂认中枢严格同锚回溯认领 + 跨锚零实装）均已按
   #599 裁定语义实装；BTC 100k 实测覆盖率 77.2% → **78.7%**（挽回 3，全部来自档1，档2 为 0），
   `ForceOvertake` 纠误 2 例在案（`force_overtake_claimed=2`，L1 35 → 33）；字节护栏 stdout/P116/m8
   六面 cmp=0，测试门唯一红 #491。**票面验收 1 的 94.1% 未达成，差额 31 只逐桶归因在案，并推翻了
   #599 对该差额的机制归因**。
2. **定义依据**：`nest_lifecycle.rs` `bridge_identity`/`same_anchor`（既有）与新增
   `bridge_by_center_upgrade`；`ActiveSegmentFrontier`/`active_segment_frontier` 数据源纪律
   （#523 永禁清单）；`parser/tail.rs:11-14`「不预判最终结果」；E2E §1:83 终态吸收/禁复活；
   模块头 090 登记 1（白名单工程桥）与 4（终态留档可查账）；#599 报告 §1.2/§3.2/§4.1-4.3。
3. **边界条件**（本结论何时翻转）：(a) 若段账本前缀会回缩重划，§1.2 的历史候选截断等价与过期
   快照守卫的判据同时失效；(b) 深度 3 的充分性在本数据集不可证伪（挽回量恒 0），跨标的/更长窗口
   须重测；(c) 若编排者裁定允许终态复活，则档1 可走「合并」形态，覆盖率与 entries 读数全部要重算；
   (d) 覆盖率两口径（身份键 / 候选）给出 156 与 159 两个数，任何下游引用必须指明口径。
4. **下游推论**：#527/#559/#580/#591/#599 已发布的 `entries=302`、`非闪现寿命中位=88.0`、
   `identity_vanished_refuted=29`、`flash_terminal=75` 四个读数在本票后变为 304 / 95.0 / 31 / 74；
   `ForceOvertake=35`（L1）账本数**不变**，但纠误口径为 33。#599 §3.3 预测的
   「entries 285→283」「覆盖率 +16.8pp」「寿命提前 25–437 bar」三项**均未发生**，引用须以本票为准。
5. **谱系引用**：#523（PanLive provider 接缝根因 + 永禁清单）；#527（L1 活窗实装）；#559（两类消失
   原因码裁定）；#580/#591（α/β 机制与 32893 现场）；#599（本票直接前置，档位语义与量化来源，本票
   订正其 §3.1 的档2 归因）；#601/#613（L2 活窗与整窗截断口径，本票保其读数零回归）；#533（字节
   护栏门与 golden 变更纪律）；#491（既线唯一红）；`formalization-validity-domain`（§1.1 深度选择、
   §3 覆盖率、§4 读数全部标 **L2 = BTC 100k 单窗真实数据**，不外推）；`no-patch-mentality`（§2.2
   拒绝「先合并着看看」的复活折中，给出账本不动 + 口径层纠误的严格形式）；`coding-style` 回放驱动
   性能缓存注记（`ActiveFrontierQueue` 就地推进）。
6. **影响声明**：改动 5 个文件——`rust/src/theta_v0/classifier/nest_lifecycle.rs`（认领判据 +
   `CenterUpgraded` 修订 + `ActiveFrontierQueue` + `force_overtake_claimed_count` + 4 个测试）、
   `rust/src/bin/p123_fast_replay.rs`（队列接线 + 逐候选扫描 + 右端冻结 + 过期快照守卫 + 诊断字段 +
   1 个测试）、`rust/tests/fixtures/issue533_p123_2000_dump.golden.txt` 与
   `issue533_p123_{20000,100000}.sha256`（**仅 dump 面**重锚，stdout 行未动）。受影响面 = lifecycle
   sidecar 账本与其诊断 dump。**未触**：p123 stdout / P116 dump / m8 trades / tower_events / 装配 /
   证书真值路径 / `Cargo.toml` / p409 / 他 session 未提交面。未关票、未改 map、未改 roster。
