# #210 Resolution：P1 重议实装现状与产量实效核查（事实账，只呈事实不拍板）

- 日期：2026-07-24 ｜ 票据：issue #210（wayfinder:research；parent map #126「塔侧跨级链增产」）
- 性质：事实核查——只读分析 + 既有产物对拍 + 一个新增分析脚本。rust 源码零改动、git 零 mutation、主仓零读写。
- 工作面：worktree `/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`）。本评论全部数字可由文内所列脚本/命令重算。
- 090 分项：每条结论标【确证】/【推断】/【未核实】。

---

## 总结论（四问一句话版）

1. **P1 hunk 已按裁定文实装到位且测试全绿**（62/62），唯一落点 `terminal_bits_in_book` CWindow Trend 分支；但有两处 doc 注释与一处审计 bin 的 RULE 串仍写旧口径（声明≠能力，090 待清），且裁定影响清单的「旧口径对照读数保留」未实装（旧测试被改写而非保留）。
2. **P1 对产量实效为「小」**：m8 三窗证书装配量 +7~+8 张/窗（同事件供给下 +8~9%），wf7 typed_found 仅 +2（#164：71→73）；#128 审计 T1 单项预期 ~15-20%、裁定文引用的 ~25-35%（实为 T1+T4 合并情景）均未兑现。增产没起来的机制在当前 dump 装置下**不可分账**（无逐事件背书失败原因字段），候选解释按既有证据排序见 Q2。
3. **不同源**：#205 的 L1 首缺（879/1852）经本票分解 **100% 是 missing_cert（脚在 L1 层有登记、键域内双向零证书）**，且 879/879 首缺恰为链顶（top==1）——这是塔侧证书供给缺席；#133 的 L1 异常是交易漏斗信号门/Xzd C2∧C3 硬门（admission 侧，有意设计）。两线共享的结构背景仅「高级别一类点稀缺」，proximate 机制互不相同。另：#133 票面「1699 events」= wf7 tower_events.jsonl 的 L1 结构事件行数（551 new_center + 1147 extend + 1 level_upgrade），口径混杂（#133 评论已指出）。
4. 雾①：一/二/三类生产点 center 构造时**恒 Some**（各自判定中枢；signal.rs 三构造器不变量），owner=B 判定载体对二三类**存在**；但「二三类点的判定中枢 start_index == 事件 b_center_start」的实际命中率**全库无测量**——这正是 P1 预期增产 vs 实测 +8 之间差额的首要悬案。雾②：`level_origin` 全树**恒 0**（写入点不存在，doc 声称的 stamping 路径未实装），消费面仅 `PartialEq` + `Debug`（digest 面）；改语义 = 补写入点 + PartialEq 从恒真变有效 + GOLDEN 翻转（#110 线已有先例在案）。

---

## Q1 P1 重议实装现状

### 1.1 hunk 范围【确证：git diff 直读】

hunk 在**未提交改动**中（`git status`：`M rust/src/theta_v0/classifier/nest.rs`，diff +25/-29），与同文件内另两条工作线改动（T5b #208 的 `NestEventIdentity` 加 `Hash` derive、#110 的测试构造器补 `level_origin: 0`）交织。P1 专属改动 = 两处：

**(a) 生产核 `terminal_bits_in_book` CWindow Trend 分支**（`rust/src/theta_v0/classifier/nest.rs:573-580`）：

- 删除 `(point.bits.buy1 || point.bits.sell1)` 限定；保留 `b_center_start.is_some_and(|b| point.center.is_some_and(|c| c.start_index == b))`（owner=B 身份合取）。
- 共享过滤的 `c_start <= source_index <= turn_source`（窗口）与 `bits.confirm_side(side)`（方向谓词）**逐字未动**；`Consolidation` 分支（`true`，同向全合法）未动；`min_by_key(source_index)` 最早性未动；`Exact` 分支未动。
- 调用点零限定纪律保持：唯一生产消费点 `nest_index.rs:97`（证书索引构建器）不加额外过滤；lib 内无第二判定 fork（研究 bin p92/p116/p117/p123/p124/p125 全部委托 lib 单一来源）。

**(b) 三个单测改写**（同文件 `mod tests`，旧语义测试被**改写**而非保留）：

| 旧测试（拒） | 新测试（收） | 断言要点 |
|---|---|---|
| `c_window_trend_type2_type3_rejected` | `c_window_trend_type2_type3_accepted` | 同向二类 owner=B ⟹ 合法，取 @120 buy2 |
| `c_window_trend_later_type1_rescues_earlier_illegal` | `c_window_trend_earliest_same_direction_owner_b` | 不再跳过二类取后至一类；取最早同向 owner=B 点 |
| `c_window_kind_partition_trend_subset_of_pan` | `c_window_kind_trend_pan_same_behavior` | Trend 域与 Pan 域同构 |

### 1.2 测试状态【确证：本票实跑】

- `cargo test --lib theta_v0::classifier::nest`：**62 passed, 0 failed**（2026-07-24 本票运行），含上表三个新测试与 owner=B 机制既有测试（`c_window_trend_type1_owner_b_hits`、`c_window_trend_type1_owner_not_b_rejected`、`c_window_trend_owner_extension_start_index_judges` 等 16 个 c_window 测试全绿）。
- 既线失败 `extract_signals_bit_exact_digest_guard` 属 #110 工作线 GOLDEN 漂移，与本 hunk 无关（本票未复核，按既有登记引用）。

### 1.3 与裁定文对拍（`chanlun/escalate/p1-resupercede-trend-all-three-types-20260721.md`）

| 裁定要求 | 实装 | 判定 |
|---|---|---|
| Trend 域移除 `(buy1\|\|sell1)` 限定，同向一/二/三类全合法（与 P2 同构） | §1.1(a) | 【确证】到位，无截断 |
| owner=B 身份合取保留 | 合取逐字保留 | 【确证】 |
| 窗口 `[c_start, t*]`、最早性确定性、confirm_side 方向谓词不变 | 共享过滤零改动 | 【确证】 |
| 证书索引 Trend 域二三类进入 ⟹ typed_found 显著上升（文引「#128 审计预期 ~25-35%」） | 未兑现（Q2；wf7 typed_found +2） | 【确证：未兑现】且**预期引用有漂移**——#128 审计（`cert-index-caliber-audit-20260721.md` §4.3）T1 单项放宽预期为 **~15-20%**，~25-35% 是 T1+T4（值桥）合并情景；裁定文把合并数字挂在 T1 单项上 |
| wf7 ✗ 可能改善 | T3 链 wf7 确认 39、T5a 33（方向退役后净降 -6）；未见改善 | 【确证：未改善】（T3/T5a 报告） |
| 旧口径对照读数保留（bit-exact 回归锁验证变化量） | **未实装**：旧语义测试被改写，未保留旧口径 shadow/锁 | 【确证：漏项】 |

**090 待清（声明≠能力，本票只登记不修改）**：

- `terminal_bits_in_book` 函数 doc（nest.rs:539-540 区域）仍写「`Trend`：`confirm_side(side) ∧ (buy1|sell1) ∧ owner=B`」；`terminal_bits_at_event` doc 仍写「Trend = 破 B 一类点 ∧ owner=B」——注释与代码行为相反。
- 研究 bin `p125_endorsement_audit.rs:392` 的 P125_RULE banner 仍写「关③P3收紧=Trend：一类∧owner=B」（该 bin 是当时审计口径的历史记录，是否算待清由编排者定）。

### 1.4 Q1 裁定输入建议（只列选项与事实依据）

1. **「实装确认到位，关闭实装面，仅清 doc」**：依据 §1.1-1.3（代码逐项对拍全中、62/62 绿）；待清 = 两处 doc + 旧口径对照读数锁的缺项。
2. **「补旧口径 shadow 锁再关」**：依据裁定文影响清单第 4 条明确要求「旧口径对照读数保留」，现未实装；若认为该条仍有效则需补。
3. **「预期数字修正归档」**：依据 #128 审计 §4.3 原文（T1 单项 ~15-20% vs 裁定文引 25-35%）；影响 map #126 Destination 验收口径的设定。

---

## Q2 各级证书产量实测与 P1 实效

### 2.1 NEST_GATE_INDEX 对拍（pre-P1 vs post-P1）【确证：既有 stderr 存档直读】

pre-P1 基线 = `/tmp/v4_C.out`（2026-07-21 14:20 运行件；同日 `cert-index-caliber-audit-20260721.md` 代码直读确证当时 Trend 域仍只收一类 ⟹ v4_C 为 pre-P1 树）。post-P1 = T3（2026-07-23，`/tmp/t3_{baseline,after}` 双侧一致）与 T5a（2026-07-24，`/tmp/t5a_after`；INDEX 内容 13 项与 T3 逐项相同，仅 index_builds 增加）。

| 窗 | 时点 | base_events | assembled | indexed | rungs_0/1/2p | single_level_share |
|---|---|---|---|---|---|---|
| p3fold | pre-P1 (v4_C) | 773 | 101 | 95 | 91/2/2 | 0.9579 |
| p3fold | post-P1 (T3/T5a) | **773** | **109 (+8)** | 109 (+14) | 102/3/4 | 0.9358 |
| wf7 | pre-P1 (v4_C) | 878 | 81 | 76 | 73/1/2 | 0.9605 |
| wf7 | post-P1 (T3/T5a) | **878** | **88 (+7)** | 88 (+12) | 83/1/4 | 0.9432 |
| wf8 | pre-P1 (v4_C) | 710 | 55 | 55 | 55/0/0 | 1.0000 |
| wf8 | post-P1 (T3/T5a) | 724 (+14) | 63 (+8) | 63 (+8) | 63/0/0 | 1.0000 |

读法（090 分项）：

- 【确证】p3fold/wf7 **事件供给完全相同**（base_events 773/878 双侧一致），装配量 +8/+7（+7.9%/+8.6%）——供给不变下背书面放宽带来的增量，量级 = **小**。wf8 事件供给变了（+14），不可同口径归因【确证：不可比】。
- 【推断】+7/+8 归因 P1 hunk：worktree diff 中唯一改动终端背书面的代码即 P1 hunk；但 `nest_index.rs` 全文件 untracked、无 git 历史，#164 登记的「装配演进」（76→88）与 P1 之间的精确分账**不可复原**【未核实：装配演进内容】。佐证 dedup 口径亦变：v4_C p3fold assembled 101→indexed 95（6 张撞键），T3 109→109（零撞键）——身份键/装配规则在两时点间有未登记变化。
- 【确证】链结构几乎不变：single_level_share 仍 ≥0.936，三窗 rungs_2p 合计 8 张——跨级链稀薄依旧（对照 #128 审计「~300× 密度差是教义结构」论断）。

### 2.2 消费侧命中（typed_found）对拍【确证：#164/T3 已发表数字】

wf7（同一 corpus 逐位对读）：v4_C typed_found **71** → #164 重跑 **73**（±2 登记为「nest_index 装配演进」，含 P1）→ #112 multi 桥 **90**（+17 与 P1 无关）。即 **P1（含同期装配演进）对候选侧命中的贡献 = +2/1651**；命中率 4.3%→4.4%。#128 审计 T1 单项预期 ~15-20% **未兑现**；裁定文引用的 ~25-35%（T1+T4 合并）更未兑现。#164 结论「约束已从级别方向转移到值桥（`seg_c_full.1 == source_index` 精确端点匹配）」与本读数一致。

### 2.3 按级别分解的装置现状【未核实 + 确证混合】

- NEST_GATE_INDEX 行**无按级别分解**（stats 仅 base_events/assembled/indexed/rungs_0/1/2p 六项，`nest_index.rs:26-39`）；`events_by_level` 各级事件数与各级 indexed 张数**当前不落任何 dump/stderr**【确证：装置缺口】。逐级别「P1 实效零/小/大」在现有产物下给不出——需新增装置（按 base 事件 level 分解 assembled/indexed 计数）。
- 可替代的级别视角（链查询侧，非索引生产侧）= #205 首缺交叉表 + 本票 Q3 分解（§3.1）。

### 2.4 为什么「放开了产量却没起来」——候选解释与证据状态

裁定文引用的预期基础：Trend 族 465 案中 type1 仅 95（20.4%）、370 案 type2/3 被排除（源：`pan-terminal-endorsement-ruling-20260718.md:32`，经 `bsp-type-usage-audit-20260719.md:126` 转引）。**注意该普查是 p125 v3 全量 4,613,599 bar 的读数，不是 m8 三窗（~30 万 bar/窗）语料**【确证：出处】；370→证书的换算不能平移到 m8 窗。

实测 +8（wf7，同供给）与预期差 ~一个数量级。候选机制（按既有证据排序，**均无法在当前 dump 下分账**——无逐事件背书失败原因字段）：

| # | 候选机制 | 支持/反对证据 | 状态 |
|---|---|---|---|
| C1 | **owner=B 合取掐住二三类**：二类点 center=c1（次级别离开走势所破中枢）、三类点 center=其判定中枢，二者的 `start_index == 事件 b_center_start` 不构造性保证；若不命中则 P1 放开的点仍被 owner 合取裁掉 | 机制上成立（signal.rs 构造器语义词读）；#128 审计 §4.2 估 owner=B（T3）贡献「低」——但那是**一类点**语境的估计，二三类未测 | 【未核实：全库无测量】——首要悬案 |
| C2 | **Trend 事件占确认事件比例低**：P1 只放宽 Trend 域；若 m8 窗 878 个确认事件以 Pan 为主，放宽天花板本就低 | p115 语料读数 trend 47 / pan 418（`p115-predicate-caliber-impl-20260717.md:146`）显示 Pan 可占 ~90%；但非 m8 语料【推断】；#128 审计称「强单边窗 Trend 主导」未经 m8 实测 | 【未核实：m8 窗事件 kind 分布不落 dump】 |
| C3 | 窗口/方向谓词掐住：type2/3 点落在 `[c_start, t*]` 外或不同向 | 机制存在；无量级数据 | 【未核实】 |
| C4 | 预期基数错误：465/95 是 4.6M bar 普查，m8 窗 Trend 事件绝对量小 | §2.4 首段【确证：出处口径不同】 | 【确证：口径不同成立；量级贡献未分账】 |

### 2.5 Q2 裁定输入建议

1. **「补装置再裁」**：给索引构建/终端背书加逐事件失败原因计数（Trend 域分 owner 不中等 / 窗外 / 异向 / 无点四桶；并按 base 事件 level 分解 assembled/indexed）——一次 m8 重放即可把 C1/C2/C3 分账。依据 §2.3 装置缺口 + §2.4 不可分账现状。
2. **「接受小产量为既成事实，把 map #126 增产战线转向值桥/表示层」**：依据 #164（约束已转移到值桥）+ §2.2（P1 对命中贡献 +2）+ T5a（方向退役后塔产不随迁增产，ADR 裁定 2 同向）。
3. **「先修预期再验收」**：依据 #128 审计 §4.3 原文（T1 单项 ~15-20%、25-35% 为 T1+T4 合并）与 465/95 的 4.6M bar 口径——map #126 Destination 的验收线需按 m8 语料重设。

---

## Q3 #133 L1 异常 × #205 首缺分布的合流核查

### 3.1 #205 首缺 L1 的底质分解（本票新增，脚本 `p1-l1-firstgap-decompose-20260724.py`，断言锚定 #205 交叉表后分解，退出码 0）【确证】

人口 = #205 同口径（top≥1 多级候选 1852 = p3fold 614 / wf7 725 / wf8 513）。

**(a) first_gap=L1/missing 879 例的 L1 级底质×见证**：

| 底质 | 见证 | 例数 | 占比 |
|---|---|---|---|
| missing_cert | agree（双向皆无证） | 839 | 95.4% |
| missing_cert | certs_only_opposite（证在异向键域） | 40 | 4.6% |
| missing_existence | （任何见证） | **0** | 0% |

- **879/879 首缺恰为链顶（top==1）**；首缺级键域内有证（certs>0）者 **0/879**。
- 候选级分布：L0 候选 194、L1 候选 685。
- 对照：L2/L3/L4 首缺 missing 同型——L2 512 例中 missing_cert 451+61、L3 249 中 248+1、L4 128 全 missing_cert agree。

**(b) 全 1852 候选的 L1 环底质（不论是否首缺）**：missing_cert agree 970（52.4%）、missing_existence agree 683（36.9%）、**closed 仅 76（4.1%）**、missing_existence 异向 74（4.0%）、missing_cert 异向 49（2.6%）。

读法：879 例的「L1 missing」**100% 是 missing_cert**——脚在 L1 层的存在性登记**有**（top==1 由存在性定义，构造性保证），缺的是该级键域内的**证书**（95.4% 双向皆无）。即 L1 首缺集中 = 「脚攀到 L1、但 L1 事件侧零产证」——塔侧**证书供给**缺席，不是存在性/登记缺席。（missing_cert 主导部分为构造性——首缺在链顶而链顶存在性恒真；但「证在不在」不是构造性的，95.4% 双向零证是实测事实。）

### 3.2 #133 异常面的事实边界【确证：#133 评论 + 本票复核】

- #133 评论的 L1 漏斗（300K 窗 level-hole-dx + V4 dump）：L1 bsp 提取 228（Type1=0、Type2=209、Type3=19）→ 信号门后 19（滤除 91.7%）；主因 = level==1 小转大 Xzd 通道的 C2∧C3 硬门（通过率 1.42%，L2+ C2-only ~80%，codex #44/#55/#179 有意设计）。**该漏斗在 admission/交易侧**。
- 票面「1699 events 产 1 笔交易」口径核查【本票新增，确证】：wf7 `tower_events.jsonl` 的 level==1 行数恰 = **1699**（构成：new_center 551 + extend 1147 + level_upgrade 1）——这是塔**结构事件**（中枢生命周期）行数，不是 nest 候选事件、不是交易候选；与 #133 评论的实测口径（tower 总计 1696 / L1 938 / 18 trades）不一致系三方口径各异。「1699」这个数字本身可复现，但其语义不是「可交易事件」。

### 3.3 同源判定【确证 + 推断】

**不同源。** 两线唯一的共享结构背景 = 高级别账本稀疏、一类点稀缺（#133 截断①：L1 账 228 点零一类——缠论定义在高级别的自然表现）。proximate 机制互相独立：

- #205 的 L1 首缺 = nest 证书**生产侧**：L1 事件（其终端背书按级别移位读 **L0 账本**——点最密、有一类）零产证。L1 账本无一类这一点**不直接**解释 L1 环缺证；它影响的是读 L1 账本的 **L2 事件**（P1 后该账本的 209 个二类点已可合法背书，是否命中待 Q4① 的 owner 测量）。
- #133 的 L1 异常 = **交易准入侧**门设计（Xzd C2∧C3），与 nest 证书索引/链闭合并无机制交点（T3 §3.4 已确证准入主体是 Xzd 回退通道：xzd_pass 609 vs nest_pass 71）。
- 【推断】若一定要找合流点：两侧都以「高级别结构稀疏」为背景条件，且 P1 对两侧本应同时增产——但 #133 侧 P1 不接线（Xzd 门不看 nest 背书类型），#205 侧实测 +8（Q2）。「L1 异常解释首缺集中」**不成立**。

### 3.4 Q3 裁定输入建议

1. **「两线分账，关闭合流假设」**：依据 §3.1（879 例 100% missing_cert、双向零证 95.4%）+ §3.3（漏斗不交叉）。
2. **「L1 环增产归入 Q2 装置线」**：L1 环缺证的可分账测量 = §2.5 选项 1 的同一装置（逐事件背书失败原因 + 按级分解），无需另立 L1 专项。
3. **「#133 票面口径修正归档」**：1699 = 塔结构事件行数（§3.2），后续引用 #133 时应以其评论实测（228 bsp / 91.7% / 1.42%）为准。

---

## Q4 两团雾的事实备齐

### 4.1 雾①：Trend 域二三类放开后的 owner 判定现状

**谁在产 owner【确证：源码直读】**：二三类点**有 owner 来源，且构造恒填**——

- `make_first_point`（signal.rs:539）：`center = Some(last_center)`（被破的最后中枢，a+A+b+B+c 的 B 同型对象）。
- `make_second_point`（signal.rs:590）：`center = Some(c1)`（次级别第一类离开走势所破的中枢，B 口径核心区间）。
- `make_third_point`（signal.rs:566）：`center = Some(c)`（判定中枢；含三类 bit 恒 Some 是旧不变量）。
- nest.rs `TerminalEndorsement.owner_center_start` doc：生产一/二/三类点 center 均恒 Some；`None` 仅剩非生产合成形状。

**判定式**：`point.center.start_index == event.b_center_start`（start_index 判同；事件侧 B 身份由 `NestCandidateEvent.b_center_start` 快照供给；`b_center_start=None` ⟹ 无合法点，诚实判负——`c_window_witness_absent_returns_none` 等测试在绿）。

**缺了会怎样【确证：机制】**：center=None 或 start_index 不等 ⟹ owner 合取假 ⟹ 该点在 Trend 域不合法（`is_some_and` 短路径负）；窗口内无其他合法点则该事件终端背书 None ⟹ 基例不产证。

**关键未测量【未核实】**：二三类点的判定中枢 `start_index` 与事件 `b_center_start` 的**实际相等率全库无任何测量**——dump 不落逐点 center 字段、不落逐事件 owner 判定结果。这是 P1 预期（370/465 量级）与实测（+8）之间差额的**首要候选机制**（Q2 §2.4 C1）：二类点的 c1 是「次级别离开走势所破的中枢」，对落在趋势 c 段窗口内的二类点，c1 是否就是本事件 B 中枢，构造上不作保证。该测量所需装置 = §2.5 选项 1 的同一组计数器。

**选项（不拍板）**：(a) 维持 owner=B 合取不变，先补测量装置看 C1 量级；(b) 若 C1 坐实大量owner 不等，再议 owner 判据是否对二三类另立口径（如判定中枢包含/同源判定）——属新裁定，超本票；(c) 认定 owner=B 是身份铁律不可动，则 P1 增产天花板即现状，map #126 转向。

### 4.2 雾②：`level_origin` 现语义与消费面

**现语义【确证：bsp.rs:115-120 doc + 全树 grep】**：doc 声明 = 「该点被提取时所在的塔级别下标」（产出级别；0 = L0 线段层），「classify stamping 路径写入（classify_impl / classify_with_tower_incremental memo-miss 终装点，构造器默认 0）」。

**实装现状【确证：全树 grep + git diff】**：

- 该字段**整字段属未提交的 #110 工作线**（HEAD 的 bsp.rs/signal.rs 无此字段；diff 全为 `+`）。
- 全树约 50 处构造点**全部写 `level_origin: 0`**；`rust/src` 内**不存在任何写入/stamping 点**（`level_origin =` 赋值零命中）——doc 声称的 stamping 路径**未实装**（090：声明≠能力，在案）。即**全库每个 BspPoint 的 level_origin 恒为 0**。
- 消费面清单（全树 grep 完备）：**仅两处**——(i) `PartialEq` 逐字段比较（bsp.rs:176；因全恒 0 当前恒真、实为 no-op）；(ii) `Debug` derive 进 bit-exact digest 面（#110 线 GOLDEN 翻转已登记，`extract_signals_bit_exact_digest_guard` 既线失败即此族）。`projection.rs:53` 仅 doc 提及同源，无代码读。不进 `BspBits`/`class_index`/分桶 key/止损判据/owner 判定式（owner 判定用 `center.start_index`，与 level_origin 无关）。

**改「所属走势级别」的影响面【确证：由消费面直推】**：

1. **需新补写入点**：现无任何 stamping 代码；「所属走势级别」语义要求在每个账本终装点写入该账本级别下标（classify_impl / classify_with_tower_incremental 两处终装点，mod.rs:445/2105 注释位已预留 #110 投影层 stamping 话语）。
2. **PartialEq 从 no-op 变有效**：同坐标不同级别点将不等 ⟹ 去重/相等语义变化（bsp.rs doc 自述设计意图即「同坐标不同级别的点不等」——目前该意图悬空）。
3. **GOLDEN digest 翻转**（Debug 面）：#110 线已有先例与登记流程（诚实重算），属受控代价。
4. **零其他消费**：nest 终端背书、证书索引、链判定、Xzd/admission 均不读 level_origin（grep 完备）——改语义对塔侧判定链**无直接机制影响**，影响集中在相等/去重/digest 面与未来的级别归属消费方。

**选项（不拍板）**：(a) 维持恒 0 现状并把 doc 降级为「保留字段」（最小改动，消除 090 声明差）；(b) 按 doc 本意补 stamping（产出级别），影响面 = 上表 1-3；(c) 改义为「所属走势级别」再补 stamping——注意与 (b) 的差：同一物理点在多级账本重复出现时，(b) 各账各值、(c) 归一宿主级；两者对 PartialEq/去重的语义后果不同，且 (c) 需要先定义「所属走势」的归属判据（现无此概念实装）。

---

## 090 总分项

- **确证（本票新测）**：§1.1 hunk 直读；§1.2 测试实跑（62/62）；§2.1 v4_C/T3/T5a INDEX 行对拍；§3.1 首缺底质分解（脚本断言锚定 #205 交叉表，退出码 0）；§3.2 的 1699 构成分解；§4.1 构造器 owner 语义词读；§4.2 level_origin 全树 grep + git diff（HEAD 无此字段、全树恒 0、消费面两处）。
- **确证（引用既有发表物）**：#205 resolution 交叉表与 94.2%；T3/T5a 报告 INDEX 13 项不变、typed_found 112/90/58、链确认 39/33；#164（71→73→90、1.03%、值桥约束转移）；#133 评论漏斗数字；#128 审计 §4.3 预期分档。
- **推断（已标注）**：+7/+8 归因 P1（nest_index.rs untracked 无历史，「装配演进」内容不可复原）；C2（Trend 事件占比低）的 m8 适用性。
- **未核实**：逐级别 indexed 分解（装置缺口）；owner=B 对二三类的实际命中率（C1，全库无测量）；m8 窗确认事件的 kind 分布（不落 dump）；`extract_signals_bit_exact_digest_guard` 既线失败本票未复核（按 #110 工作线登记引用）。
- **产物**：本事实账 + 分析脚本 `chanlun/review-results/p1-l1-firstgap-decompose-20260724.py`（输入 = T3 合并 dump，只读；断言锚定 #205 已发表交叉表后分解，退出码 0）。rust 源码零改动、git 零 mutation、主仓零读写。
