# #844 多重赋格资产盘点：存量 vs map #787 在案裁定 vs #839 六条 R

**票**：[#844](https://github.com/xy7365527-lang/NewChanlun/issues/844)（`wayfinder:task`，Part of #787，blocking #839）
**日期**：2026-08-01
**基线 HEAD**：`59907fece5`（`docs(teach): 新课「纵向造级别 vs 横向读级别」`）——开工前已核，非 `origin/main` 祖传线
**产物**：本报告。**零代码改动、零生产文件改动。**
**纪律**：本票只产读数与对账，**不裁任何东西**。发现冲突点名到行、两边各说什么照抄，不选边。裁定归 #839 与主图 #787。

---

## 0. 方法与覆盖面

### 0.1 检索路径

| 面 | 做法 |
|---|---|
| A tracker | `gh issue view` 逐票读 **body + comments**（resolution 在评论里，只读 body 会漏）；`gh issue list --search` 做关键词反查（已用 `level_risk.rs` 做控制组验证搜索确实生效） |
| B 文档 | `git show --stat 5dc24734fa` 列全「赋格系列」22 个文件 → 逐份读；再对 `赋格/fugue/声部/voice/多重/多空双开/独立反向/递归正则/申克/休止符/巴赫` 九词在 `analysis/ docs/ .chanlun/ chanlun/ rust/` 全仓反查 |
| C 代码 | `voice.rs`/`level_risk.rs`/`CampaignBook`/`MAX_LEVEL`/`TStage`/四族目录逐件读 + 调用点 grep，在本 HEAD 上独立复核 #753/#762 的「现役」结论 |

### 0.2 覆盖面照实（090）

- **B 面的真实规模远超票面预估**：`赋格|fugue` 在仓内命中**数百个文件**，跨至少 6–7 代架构（早期 `fugue_v1` 族 60 个文件 → Version I → `organic_fugue` v1/v2（即 5dc24734fa 那批）→ `nested_recursive_fugue`/`positional_fugue`/`isolated_fugue` 21 个 → `spiral` 15 个 → `fugue_v3` → `recursive_t` → 现役 `theta_v0` voice 相关 85 个 `.rs`）。
- **本次逐份精读 = 21（赋格系列，去掉我自读的 1 份）+ 约 43 份新查出者 + 5 张票的全文**。**其余数百个命中只做了路径与关键词命中确认，未逐份精读**——它们在表 1-C「已定位未展开」段按族群列出，**不给内容描述**（不编造）。
- **本报告不是穷举**。任何「未查到」都写成「未查到 + 检索式」，**没有一处写成「不存在」「无冲突」「不受影响」**。

---

## 表 1：存量清单

### 1-A tracker 侧（票 / spec / map）

| 资产 | 什么 | 在哪 | 状态 | 一句话结论 |
|---|---|---|---|---|
| **#135** | 🗺️ 按级别管理出场：多级别仓位的独立出场机制 | issue | CLOSED 2026-07-26 | **本线的总图**。其 Decisions 段是一份现成的裁定索引（17 条），票面 A 段列的 #156/#157/#141 全在它底下 |
| **#156** | [spec] 多重赋格多空双开：完整端到端（声部生命周期事件流缝） | issue | CLOSED 2026-07-29 | 17 条 User Story + 唯一新缝＝声部生命周期事件流；**明文裁定「跨级反向且过开仓门者开为独立声部，不再一律定型为对冲腿」** |
| **#157** | [grilling] 独立反向声部准入语义 | issue | CLOSED | 裁毕落 **ADR-0002 修正案一**：涌现天花板上移 bar 上不得有在世反向根（涌现互斥哨兵）；**其评论区带一份 344 行「会计层考古」**，是本次最重要的线索来源 |
| **#141** | 嵌套多重赋格的出场优先序 | issue | CLOSED | 两问全结：**各听各的哨子 + S3 父死子从（唯一连坐）**；「子仓削减母仓」判为**混账词法伪问题**，随 #272 每级一本账消解 |
| **#272** | [grilling] 短差重定义与点级别定账：每级一本账 | issue | CLOSED | **教义升格**：短差＝减仓回补（S6 开空腿废止夺名）；**点级别定账**（信号级别＝操作级别＝账级别）；**每级一本双向账**；「对冲」判为多余身份 |
| **#273** | 短差机制名分盘点 | issue | CLOSED | **重大事实**：生产 `CloseShortDiff` 全部产自散装证书分类链，**生产正跑已被废止的形态** 204/194 笔 |
| **#140** | 多空双开的出场表达 | issue | CLOSED | 三问全确认 ADR 0001 读法：共存＝各级别各记各账；唯一级联＝S3 父死子从 |
| **#278** | 🗺️ 多空双开与反向操作对齐 | issue | CLOSED | 散装域处置 + 对冲腿名分 + 狭义短差 |
| **#134** | L1 C3 硬门裁定 | issue | CLOSED | **级别归属终版**：级别身份是**事件**（C3 坐实＝升格时刻）而非静态标签 |
| **#261/#262/#264/#269/#270** | 翻向守卫事件化线 | issues | CLOSED | 1-bar prune 判为**接线缺陷非教义**；修后 L3 pnl +3026→+11053 ＝基线 100.6%，长持恢复 |
| **#803** | [task] 三阶段是单例还是自相似 | issue | CLOSED 2026-07-30 | **★★ 与 #839 R2 正面相撞，见表 2 冲突 X1** |
| **#753 / #762** | 前代族名分核定 / 三族联合名分标记 | issues | CLOSED | 「三族全改判现役」——本 HEAD 独立复核仍成立，但**判据须精确到位**，见表 1-C 尾注 |
| **#567 / #595** | π 观测面口径调研 / 旧锚 voice 夹具零信号 | issues | CLOSED | 净额影子账 vs 声部执行账的读数盘点 |
| **#837 / #838** | 跨级反向频次探针 / 成因票 | issues | CLOSED | 62.6%–72.0% bar 在场；根因＝`net_target_units` 标量净持仓坐标 |
| **#834 / #835 / #836 / #833** | 跨区仲裁 / 分区比例 / 总账口径 / 目标函数 | issues | **OPEN** | #839 的四张下游票，全部 `blocked_by` #839 |

### 1-B 理论文档侧

**「赋格系列」commit `5dc24734fa`（2026-06-12）带进来的全部 22 个文件**（票面要求「须先列全」）：

```
analysis/_counter_seg_level_dist.py                (226)  统计脚本
analysis/_trend_counter_seg_stats.py               (232)  统计脚本
analysis/concurrent_fugue_deep_think.md            (607)
analysis/fractal_recursive_fugue.md                (170)
analysis/fugue_v2_full_backtest.md                 (117)
analysis/fugue_version_i_equal_split_comparison.md (121)
analysis/fugue_version_i_results.md                 (42)
analysis/lesson38_40_concurrent_research.md        (255)
analysis/m1_vi_rust_results_on.json               (1360)  数据
analysis/m1_vi_rust_results_on.md                   (87)
analysis/multi_level_fugue_backtest.py             (310)  回测驱动器
analysis/multi_level_fugue_cross_asset.md          (134)
analysis/organic_fugue_design.md                   (517)
analysis/organic_fugue_rust_design.md              (895)
analysis/organic_fugue_v2_design.md               (1481)
analysis/recursive_empty_domain_research.md        (282)
analysis/recursive_fugue_depth1_experiment.md      (222)
analysis/recursive_fugue_ultimate_design.md        (478)
analysis/recursive_regularization_theory.md        (258)
analysis/sublevel_confirmation_recursive.md        (148)
analysis/trend_counter_segment_research.md         (246)
analysis/trend_counter_voice_design.md             (246)
```

**承重最重的六份（含票面漏列的三份）**：

| 文档 | 行 | 是什么 | 最重要的命题（带行号） | 名分（文中自称） |
|---|---|---|---|---|
| `analysis/recursive_regularization_theory.md` | 258 | 递归正则化 × 多重赋格的概念层严格化 | **命题 2.1**（`:49`）「走势是申克式的，操作是赋格式的……N 个独立的同构 FSM，每个 FSM 绑定一个级别和一份筹码……同级别分解的唯一性（39课）就是声部不共音的结构基础」；**命题 3.4/3.5**（`:90`/`:104`）可操作带 L\*≤L≤L\*\*；**命题 4.1**（`:133`）「子声部的激活条件是父声部处于域内震荡相位」；**命题 5.2**（`:171-178`）闭腿完备性 (a)(b)(c) 三分；**命题 5.1**（`:167`）3买与止损符号相反 | `:3`「纯概念层分析。无代码产出」 |
| **`analysis/bidirectional_nested_accounting.md`** | 534 | **★★ 双向嵌套递归赋格的完整会计体系 L0 形式化**（票面未列） | `:21-22` 每层一本带极性账本、物理执行＝各层净额之和；`:105-106`「空头的『成本』：B_s 是**单位收入基**……『降成本』在空头侧的精确含义 ＝ **抬高开空均价**」；`:133-142` **非对称定理**：空头无 EarningShares 镜像（L0 不可构造）；`:159-165` **同股数定理 M=N**；`:316-319` **无 FLAT**：`{+Q}∪{+Q−q}∪{−Q}∪{−Q+q}`，`0 ∉ 状态空间`；`:354` **保证金恒等式 posted＝Σ nₖ/L_maxₖ 逐层全额计提**；`:357` 按净额计提会「删除层间独立性」 | `:532`「纯文档（零代码、零在册判决改动）」 |
| **`docs/three_phase_unified_design.md`** | 255 | **★★ 三阶段会计总体性重构，编排者 2026-06-20 裁决**（票面未列，不在 `analysis/` 而在 `docs/`） | `:3`「把三阶段会计从 per-instance 上移到根账本（TRoot）总体口径」；`:10`「**三阶段是总体性的，不是 per-layer**……phase 是全局状态」；`:60` 依据 `chan99/0033:11`「级别的意义只和买卖量有关」⟹「实例只决定买卖『量』，不决定成本/阶段」；`:28`「**永远在市场，永远有方向**……不存在空仓真空期」；`:26`「绩效＝Σ\|涨跌幅\|」 | `:5`「**纯设计，未改代码**」 |
| `analysis/organic_fugue_v2_design.md` | 1481 | 有机赋格 v2：v1 矛盾诊断与逐点修正（C1-C7 + D1-D2 + K1-K13） | `:57`「**L\* 分辨率下界**（命题 3.4 + E6 实测爆仓）：段振幅 < 信号粒度+摩擦的层级……floor=segment 的依据」；`:43` 主升段休止符；`:220`「也是休止符（中途不参与短差）的根据」 | `:3`「设计稿（不含实现）」 |
| `analysis/organic_fugue_design.md` | 517 | 有机赋格 v1 框架 | `:14-16` 统一原则；`:28-29`「正确架构只有**一个** FSM 类型（LevelOperatingUnit）……实例化 N 份」；**`:198`/`:334` 分区比例公式 `frac[k] = A_k / Σ_j A_j`**（A_k＝该层存活中枢相对振幅） | `:3`「设计稿（不含实现）」 |
| `analysis/concurrent_fugue_deep_think.md` | 607 | 并发赋格深度思考（voice 存在论/级别身份/量分配/协调） | `:29`「不是一个 voice 做更多事，是更多 voice 各做自己的事」；**`:255-261` 声部数量上界**「期货上低级别声部先验关闭，OKLO 型标的**最多 2-3 个有效声部**……N 是数据决定的有限集」；`:550`「voice 不是机器，是一个有门的账本」 | `:5`「纯设计思考，零代码」 |

**同批其余 15 份（含判决与实测）**——全部已读，摘最承重一条：

| 文档 | 行 | 一条 |
|---|---|---|
| `lesson38_40_concurrent_research.md` | 255 | `:51`「『各级别各自独立操作』是原文直说」；`:23`「38课的串行程式是**一个声部的内部语法**」 |
| `recursive_fugue_ultimate_design.md` | 478 | `:296-301` 三范畴终止条件表（存在论/经济/结构）；`:323` `depth(voice k) = k − FIRST_BSP_LADDER` |
| `recursive_empty_domain_research.md` | 282 | `:26-27` 递归深度公式（代码实证）「entry=e ⇒ 可用递归深度 ＝ **e−3**」 |
| `recursive_fugue_depth1_experiment.md` | 222 | `:38` `MAX_REV_DEPTH: usize = 4`（代码硬上界）；`:80-83` 级联不变式「父腿闭合先平全部子树腿（最深优先）」 |
| `trend_counter_voice_design.md` | 246 | `:12`「**架构惊喜**……N-voice 容器、per-level SlotKey 槽隔离、共享账本 **已经存在**」；`:37`「voice 之间无显式协调，耦合只通过共享账本发生」 |
| `trend_counter_segment_research.md` | 246 | `:11` 范畴修正「38课『向下段先卖后买』的操作对象**不是线段**，是同级别分解下的该级别走势类型段」 |
| `fractal_recursive_fugue.md` | 170 | `:119-120` BRN 成立「递归赋格链上的第一张有效票」；`:118` OKLO 否证 |
| `sublevel_confirmation_recursive.md` | 148 | `:12`「『次级别确认完整递归』假说在 confirmed 事件层**被否证**」 |
| `multi_level_fugue_cross_asset.md` | 134 | `:33-34` **判决1**：floor=segment 9/9 全胜，结论升 L3 |
| `fugue_version_i_equal_split_comparison.md` | 121 | `:42` **均分配额「没有改善，部分恶化」** |
| `fugue_v2_full_backtest.md` | 117 | `:67-102` bar 级每级 FSM 贡献巨负（OKLO −1,048,873） |
| `m1_vi_rust_results_on.md` | 87 | `:42-64` ES bi/segment 转正而 bar 仍负 |
| `fugue_version_i_results.md` | 42 | `:28-32` 每级贡献度 bar −1,048,873 / bi −13,110 / segment −6,858 |
| `organic_fugue_rust_design.md` | 895 | `:1` 「类型系统作为递归正则化的编译期守卫」；实装对照见表 1-C |
| 2 个 `.py` + 2 个数据文件 | — | 一次性统计脚本 / 回测驱动器 / 原始结果 |

**22 个之外新查出的（精读过的 43 份里最相关的）**：

| 路径 | 行 | 一条 |
|---|---|---|
| `docs/adr/0001-graded-exit-and-shortdiff-doctrine.md` | 329 | `:12` S1「声部＝一次建仓……出场即声部终结」；`:40-42` S6「短差基线表示＝子级反向对冲声部」；**`:95` 修1「S6……自本修正案起废止」** |
| `docs/adr/0002-reverse-root-admission.md` | 32 | 反向开仓只有短差（挂父名下）或反向根（自立根）两种身份，「证书即准入，不另设门」 |
| `docs/theta-v0-grammar-audit.md` | 79 | `:26`「多声部（声部树）」当前是**退化（单声部 depth=0，无嵌套对冲）**；`:48`「独立反向双开」列为 alpha 来源缺失项 |
| `chanlun/review-results/p120-nested-voice-dual-ledger-design-20260718.md` | 369 | θ 线施工图：方案 A `recognize_nested` + 方案 B `DualLedger` |
| `docs/nested_fugue_accounting.md` | 330 | 同一笔物理交易在子层记 P&L、父层记降成本；§11.1 登记 `child.P&L ≡ father.cost_reduction` GAP |
| `docs/three_stages_accounting_design.md` | 69KB | `cost_basis`（净现金口径，可为负）与 `basis`（加权入场价，恒正）**必须物理分离** |
| `analysis/nested_recursive_fugue_design.md` | 376 | `:16-19`「一个趋势级别的反向线段在次级别，可能就是一个直接的空单；在高级别的视角看，只是一个短差」 |
| `analysis/unified_recursive_voice_fsm_design.md` | 310 | 「净额等价定理」「子 voice 反向＝会计描述非独立机制」 |
| `analysis/positional_fugue_design.md` | 352 | 26/44 课「级别↔量」直接形式化 |
| `analysis/bidirectional_nested_fugue_research.md` | 511 | v1 经 30 工位对抗审查判 1 fatal + 12 major，v2 两轴重构（暴露轴 × 载体轴） |
| `docs/architecture/complete_fugue_design.md` | 780 | 38 课操作骨架 S1-S14 逐条形式化 |
| `.chanlun/genealogy/settled/542-spawn-allocation-sigma-invariant.md` | 88 | **配额比例 σ-不变**：`f = m/p_units` 必须级别无关 ⟹ **`f = 1/λ`**；旧全局 `θ_sub/θ_total` 归一化是隐性违反；已实装 `SUB_SPAWN_FRAC` |
| `.chanlun/genealogy/settled/349-fugue-state-machine-definition.md` | 210 | 赋格状态机形式化：状态集＝降成本 FSM 五态 + 声部维度 V(t)，转移含「赋格分裂」 |
| `.chanlun/genealogy/settled/265` / `267` | 234 / 162 | 帕萨卡利亚模型（多重赋格发生在层 2 内部）／「赋格 ＝ 同一满仓仓位上不同级别短差的嵌套 + 小转大后新旧循环并存」 |
| `.chanlun/genealogy/settled/532` / `534` / `538` / `539` / `540` / `543` / `553` | 128–277 | master 出场 vs voice REV 腿概念分离／49 课二相分离／双重会计身份／清仓 regime 门控／同一-差异双重性／操作＝path 非硬编码循环／cascade 翻转 L3 否证 |
| `.chanlun/genealogy/pending/676` / `693` | 149 / 142 | **π^bsp 子声部恒＝0 是实装逻辑根因**「多声部对冲层从未执行」／A11 声部执行层三分冻结 |
| `.chanlun/review-results/issue831/837/838-B/840/841-*.md` | 280–553 | #831/#837/#838/#840/#841 的报告实体，本次已交叉引用 |

**「已定位、未展开读」（照实，不给内容描述）**：`analysis/fugue_*.{md,py}` 早期族 **60 个文件**；`nrf_v*`/`nested_recursive_fugue` 族 **12 个**；`positional_fugue`/`isolated_fugue` 族 **9 个**；`spiral` 族 **15 个**（含 `docs/spiral_engine_v2_architecture.md` 785 行）；`.chanlun/genealogy` 赋格/声部相关 **34 条中未读 21 条**；`docs/recursive_t_architecture_v2.md`（740）、`docs/reading_b_t_dual_design.md`（468）。

### 1-C 代码侧

| 路径:行 | 是什么 | 名分（判据） | 一句话 |
|---|---|---|---|
| `rust/src/theta_v0/strategy/voice.rs`（542 行） | Θ_voice 声部树：`VoiceSide`(:27)、`VoiceState{depth,b,q,exit,enter_ok}`(:138)、`ActState`(:148)、`depth_weight()`(:206)、`within_max_depth()`(:218)、`root_sel()`(:283) | **现役**：`strategy/interp.rs:55` 非测试 `use`；另 4 处生产消费；`:302-542` 14 个测试 | 一 depth 一份权重；上界＝`config.voice.max_depth` default **3**；`root_sel` 只做**根方向**消歧（双触发→Flat），**不做资金仲裁** |
| `rust/src/spiral/voice.rs`（204 行） | Voice 森林骨架：`VoiceStatus{Active,PendingRecovery,Closed}`(:27)、`SpiralVoice`(:45)、`SpiralForest`(:139)、`active_root_count()`(:153) | **现役**（`spiral/mod.rs:35-46` GUARD-ROLE） | 文件自陈 `:9-12`「Step 0 空骨架……当前不声明任何操作能力」，**该自陈在本 HEAD 已过时**——真正的开/关生命周期在 `spiral/accounting.rs:101/:190/:215/:307`；**无显式 `MAX_VOICES` 常量** |
| `rust/src/theta_v0/strategy/level_risk.rs`（123 行） | `level_weight()`(:52)/`level_weights_sum_le_one()`(:67) | **现役但 default 关闭**：被 `coverage/sizing.rs:247/:283` 消费，施加点 `backtest/fill.rs:5195`；但 `config.rs:228` `level_weights: Vec::new()` + `:229` `enforce_level_cap: false` | **`w_ℓ` 生产默认不生效**，见表 3 R4 |
| `rust/src/theta_v0/strategy/oscillation_campaign.rs:899-900` | `CampaignBook{campaigns: BTreeMap<(u32, VoiceSide), OscillationCampaign>}` | **现役**：`backtest/fill.rs:1349/1447/4456/6348`、`backtest/runner.rs:446` | 一 campaign ＝某（级别, 侧）从空仓→有仓到全平的一次生命周期；内含 `TwState`+`LedgerComp`+`ShortDiffAccount` 三本并置账本 |
| `rust/src/theta_v0/strategy/ledger.rs:88` | `enum TStage{CostReduction, CapitalRecovered, EarningShares}` | 现役，θ_v0 生产链核心 | 挂在 `TwState`(:154) → `OscillationCampaign`(:477) → `CampaignBook`，即**按（级别, 侧）** |
| `rust/src/theta_v0/closed_loop/state.rs:181,184` | `AssemblyState{..., tw_state: TwState, ...}` 单数字段 | 现役 | **全局单例** |
| `rust/src/recursive_t/t_engine.rs:124` | 第二个 `enum TStage`（同名不同物） | 现役 | `TPositionEngine` 单字段 ⟹ **全局单例** |
| `rust/src/recursive_t/rec_engine.rs:596-600` | `enum RecStage`（同概念第三次独立定义，改了名） | 现役 | `TRoot.stage`(:941) 单字段 ⟹ **全局单例** |
| `rust/src/recursive_t/rec_engine.rs:81` | `pub const MAX_LEVEL: usize = 8` | 现役 | 注释 `:76-80` 自陈 ＝ `MAX_LADDER(11) − BASE_LADDER(3)`，**本仓派生值** |
| `rust/src/trading/types.rs:28 / :65` | `MAX_LADDER = 11` / `MAX_REV_DEPTH = 4` | 现役 | 硬编码上界 |
| `rust/src/theta_v0/config.rs:135,156,157` | `max_depth` default `3`；`depth_weights` default `[0.60, 0.30, 0.10]` | 现役，**当前生产默认唯一生效的资金分配轴** | #841 已实测 `depth` 就是级别轴（相对坐标） |
| `rust/src/fugue_v3/mod.rs:111,115` | `LAMBDA = 3.0`；`MOBILE_FRAC = 1.0/LAMBDA` | 现役 | 即谱系 542 的 `f = 1/λ` 形式 |
| `rust/src/spiral/`(12 文件/3581) `fugue_v3/`(11/2342) `recursive_t/`(16/17383) `trading/`(31/34393) | 四代码族 | **现役**（本 HEAD 独立复核） | 见下方尾注 |
| `src/newchan/trading/`（15 文件/3443，**Python，与 `rust/src/trading/` 同名不同物**） | Python 交易层 | 现役但**未见任何票号覆盖它的名分裁决** | 被 5 个 `analysis/*.py` + 19+ 个 `tests/test_*.py` 直接 import |

**尾注：#753/#762「三族全改判现役」在本 HEAD 复核成立，但判据须精确到位。**
「现役」的判据是「有非测试 python 调用者 + 族内/跨族编译期硬依赖」——**不是**「被 `theta_v0`(π，唯一生产回测入口) 调用」。独立 grep 确认：`rust/src/theta_v0/` 下**没有一处真实 `use` 语句**引用 `spiral`/`fugue_v3`/`recursive_t`/`trading`（唯一命中是 `classifier/retrace_ledger/mod.rs:32-33` 的文档注释）。四族之间以及经 PyO3 被 `analysis/`（103 个文件、145 处调用）/`trading_system/` 调用而现役，**与 π 是两条互不相交的生产线**。
`recursive_t/` 内部也非铁板一块：`backtest.rs`/`backtest_run.rs` 两文件为 `deprecated 待退役`（`recursive_t/mod.rs:49-58`）。

**代码 ↔ 设计稿对应（表 C 摘要）**：`rust/src/trading/mod.rs:3-4` 直接点名 `analysis/organic_fugue_v2_design.md` + `organic_fugue_rust_design.md` 为设计来源，实装对应**高度成立**——`MasterExitSignal`/`VoicePhase`/`VoiceUnit`/`RevTranche`/`RevLeg`/`RevOpenVerdict`/`FatigueState`/`SlotKey`/`LadderMask`/`CenterBook`/`OrganicLedger`/`SizeAllocator`/`run_organic` 等 16 组两侧同名。**明确未被采纳的四组**：`struct Ladder` newtype、`CenterRef`、`LevelOperatingUnit`/`LouState`/`LadderCtx`、`LongCtx`/`RunnerState`/`ExitReason`（`runner.rs:98` 实装用扁平 `state: u8` 而非类型安全枚举）——**设计稿的类型安全提案未被采纳**。全仓共 **16 个 `.rs` 文件在注释里引 `analysis/*.md` 为设计源（涉 20 份不同文档）**。

---

## 表 2：与 map #787 在案裁定的重合面

标记：✅ 一致 ｜ ⚠️ 冲突（点名到行，两边照抄，**不裁**）｜ ➕ 补充（存量给了在案裁定没有的东西）

### 2-1 与 ADR 0010「重」三要件

| # | 在案裁定 | 存量 | 判 |
|---|---|---|---|
| A1 | `docs/adr/0010:21-23`「**重 ＝ ⟨标的 S，操作级别 ℓ，专属筹码 Q⟩**，三要件缺一不可」 | `analysis/recursive_regularization_theory.md:49`「N 个独立的同构 FSM，每个 FSM **绑定一个级别和一份筹码**」 | ➕ **三要件里的两件（级别+筹码）在 2026-06-12 已被独立推出**，缺的正是标的维——而标的维恰是 ADR 0010 从 `031:30` 补上的那一件 |
| A2 | `0010:25`「**一层有没有自己那份专属筹码**，决定它是独立的一重，还是别人的从属层」 | `recursive_regularization_theory.md:59` 仓位约束 3：「**L 声部的可操作筹码 ≤ L+1 声部卖出所释放的部分**；更高声部持有的底仓对 L 不可见、不可动」<br>`analysis/bidirectional_nested_accounting.md:302`「**子腿预算基 ＝ 旧父腿释放的敞口**（`open_sub` 语义）——父腿消灭 ⟹ 预算基消灭 ⟹ 腿的存在前提消灭」 | ⚠️ **X2 冲突**：ADR 0010 说钱是**分配下去的、各自专属**（N 重平行）；两份文档说钱是**父层释放出来的、父死子亡**（N 重嵌套有拓扑父子）。两种下 `040:20`「每一重都对应着一定的资金与筹码」都能读通，但资金拓扑相反 |
| A3 | `0010:31-35`「『声部』在缠论全语料 0 命中——它是本仓自造词……**本仓的『声部』用在了与赋格原义相反的东西上**」；`CONTEXT.md:21`「**不得由赋格比喻推理声部的性质**」 | `recursive_regularization_theory.md:34` 声部＝「一个操作级别上的独立资金/筹码层（独立 FSM）」——**该文档的「声部」是持久的、绑一份筹码，即 ADR 0010 的「重」**；但 `:31-43` 整节是一张十三行「赋格要素 ↔ 缠论对应」表，`:45-49` 明写「赋格在操作者一侧」 | ⚠️ **X3 冲突（方法论层）**：两种读法后果不同——(i) 该文档用的是赋格**原义**，故 ADR 0010 的错位诊断对它不适用，它反而是错位的**独立发现者**；(ii) 该文档的整个 §2 就是「由赋格比喻推理性质」，正是 `CONTEXT.md:21` 禁止的做法。**不裁** |
| A4 | `0010:52`「重是声部的容器……**声部死而重不死**」 | `docs/adr/0001:12` S1「声部一经建立不可变；**出场即声部终结**」 | ✅ 一致（ADR 0010 正是为补这个缺口而立） |
| A5 | `0010:53`「**三阶段（`TStage`）的实例单位应随重走，即逐 `(标的, 级别)`**」 | 见下 **X1**，本报告最重的一条 | ⚠️ **X1 冲突** |

### 2-2 ⚠️ X1（★★ 最重）：三阶段的实例单位，两侧各有原文、结论相反

**在案一侧（2026-08-01）**：

> `docs/adr/0010:53`：「**三阶段（`TStage`）的实例单位应随重走，即逐 `(标的, 级别)`** —— 而仓内三份实装（`CampaignBook` 按 `(级别, 方向)`／闭环 `tw_state` 全局单例／`rec_engine` 单 campaign）**无一如此**（#840 R2）。」
> #840 resolution R2：「三阶段实例单位错配：原文逐**标的** vs 仓内三份（按级别×侧／全局单例）⟹ 无法表达「A 已归零、B 还在降成本」」，承重引文 `031-第31课.md:30`「资金管理中**针对每只股票**的最大原则」。

**存量一侧（2026-06-20 / 2026-07-30，两份，均带原文锚）**：

> `docs/three_phase_unified_design.md:11`：「**三阶段是总体性的，不是 per-layer**：`cost_basis`/`phase` 在 `TRoot` 追踪，不在每个 `TInstance`。所有级别的短差利润汇聚降低**同一个**总体成本。phase 是全局状态。」标题下 `:3` 自陈「编排者裁决 2026-06-20。本文推翻 `recursive_t_architecture_v2.md` §8.7『每实例独立 cost_basis/phase』」，`:5`「**纯设计，未改代码**」。
> 同文 `:18` 原文锚：「第31课:24（=chan99/0033:21）……⟹ **阶段边界（成本穿 0）是改写全部级别买卖规则的单一全局事件**。chan99 `0033:11`『**级别的意义其实只有一个，基本只和买卖量有关**……在持股成本变成负数前，仓位是一直不变的』⟹ **级别只决定『量』(units)，成本/仓位/阶段是整只持仓的总体属性**」；`:60` 复述为「实例只决定买卖『量』(units)，不决定成本/阶段」；`:228` 结论表复述「§8.7『每实例独立』被推翻」。
> #803 resolution（CLOSED 2026-07-30）：「**维持**：三阶段的删除**不是工程妥协而是原文裁定**——`75ecfc9ee0` 溯源第 31 课「三阶段总体 + 成本穿 0 全局门」、chan99 `0033:11`「级别只决定量」；加 flat `t_engine.rs:105` 的 `TStage` **本就是 GLOBAL，per-instance 才是偏离**。」

**两边共识与分歧点**：两边**都同意**三阶段不该按（级别×侧）分 —— 分歧只在**级别维**：ADR 0010 从「重」三要件推出应带级别（逐 `(标的, 级别)`），#803/`three_phase_unified` 从 `chan99/0033:11` 推出**级别只管量、不管阶段**（逐标的、跨级别全局）。

**另一条须点名的措辞事实**：#840 resolution 写「**补 #803**：#803 从**代码侧**查出三阶段是单例，本票从**原文侧**证明这个单位错了——应逐标的」。而 #803 的 resolution 原文是「三阶段的删除**不是工程妥协而是原文裁定**」并给了两处原文锚。⟹ **#840 把 #803 描述成了代码侧发现，而 #803 自称是原文裁定。** 本票只报这个措辞差，不判谁对。

**#803 顺带指出的另一件（与本票主题同形）**：「**★ 而真正每级一份、真正自相似的那个『仓位生命周期』在仓里确实存在——载体是 `LegPair` + `leg_open_units` 显式分配规则（`rec_engine.rs:1026` / `:2002`，模块头 `:149` 逐字「每级别一对 LegPair」），不是三阶段。**」并给结论「**问题重心应从『拆不拆资金池』（已经是拆开的）移到『已经拆出来的 `LegPair` 那套为何没人当它是答案』**」——这与 map #787 立图根因是同一句话。

### 2-3 与 ADR 0011 分层（纵向造级别 / 横向读级别）

| # | 在案裁定 | 存量 | 判 |
|---|---|---|---|
| B1 | `0011:15`「纵向＝造级别（构造），横向＝读级别（操作）。两者依赖单向」；`0011:21` 横向「**否**——一次性切分，不自我调用」 | `recursive_regularization_theory.md:47`「市场走势本身的正确音乐学模型不是赋格，而是**申克分析**……这正是『中枢_{L+1} ＝ 三个连续走势类型_L 的重叠』的递归定义」；`:49`「**走势是申克式的（层级是构成关系），操作是赋格式的（声部是并行独立的程式）**」 | ➕ **同一分层的另一套语言，早 7 周**。ADR 0011 的「纵向构造 / 横向读法」＝ 该文档的「申克式走势 / 赋格式操作」。**该文档还给了这条分层一个 L0 论证**（`:47`「高级别声部没有独立于低级别音符的自由度」）与一条工程推论（`:51`「把『市场的多重赋格』实现为单一全局信号流的串行处理是**范畴错误**」） |
| B2 | `0011:39`「**必须支持多个同时存在**——多重赋格 ＝ N 个重各挂塔的一层」 | `organic_fugue_design.md:28-29`「正确架构只有**一个** FSM 类型（`LevelOperatingUnit`）……实例化 N 份」 | ✅ 一致 |
| B3 | `0011:31-35` 不变式 ＝ **分解唯一性**（`038:18`/`038:28`） | `recursive_regularization_theory.md:40` 声部不共音 ←「『两个中枢不可能共用一个次级走势』（27课答疑）——**同级别分解的唯一性是声部独立的前提**」；`:49`「同级别分解的唯一性（39课）就是声部不共音的结构基础」 | ➕ **同一不变式，且给出了它的操作学后果**（唯一性 ⟹ 各声部的域互不侵占）。ADR 0011 只到「不变式是唯一性」，未展开为什么这对多重并行是必要的 |
| B4 | `0011:51` 裁定五「『避开下跌类型』这条操作纪律在本仓不存在……本仓多空双开」 | `recursive_regularization_theory.md:57` 相位约束「**L 声部的节奏相位是父段方向的函数**——phase(L) = sign(当前 L+1 段方向)。父段向下 → L 声部只允许『先卖后买』序」；`:38` 不协和「**低级别开腿逆着高级别段方向——必须在高级别段完成前解决**」（锚 27 课「99% 转化为三卖」） | ⚠️ **X4 冲突**：在案裁定说跨级反向持仓是**目标形态**（#787 Notes 前提第三次订正、#842 R-4、#837 实测 62.6%–72.0%）；该文档说低级别方向**必须服从**父段方向，逆父段开腿是「不协和、需在父段完成前解决」。两条不能同时全真。**不裁** |
| B5 | `0011:111` 未决：「`038:36`『向下段的运作刚好相反，是先卖后买』，单向市场里是减仓再补回，双向市场里是开空再平空——两者不是同一件事。**归 #839 / #834**」 | `analysis/bidirectional_nested_accounting.md:153-157` 同股数进出三形态表（短差／削减-回复 hold26／**方向翻转**），并给 `:159-165` **同股数定理 M＝N** 的三步推导；`:316-319` 无 FLAT 状态空间 | ➕ **这条「未决」在存量里有一份 534 行的现成答案**，见表 3 |

### 2-4 与 #827 五问三结论 / #828 五问

| # | 在案 | 存量 | 判 |
|---|---|---|---|
| C1 | #827 结论（A）「规则不按级别分，**按角色分**」——中间产物→延伸，最终产物→不延伸；溯源标注「**推断，非原文明文**」 | **未查到**任何存量文档处理「同一层在不同重的视图里延伸/不延伸冲突」这个问题。检索式：`grep -rn "延伸.*不延伸\|中间产物\|最终产物" analysis docs .chanlun` | **未查到**（这是本次盘点里 #827 唯一没有存量重合的结论） |
| C2 | #827 结论（B）目标形态「**旁路结果不回流主干**」 | `recursive_regularization_theory.md:51` 同型警告：「把『市场的多重赋格』实现为**单一全局信号流的串行处理**（先算大级别、再算小级别、合并成一个决策）是**范畴错误**——它把操作者侧的赋格压扁成了走势侧的单声部」 | ➕ 同一条纪律的对偶表述（一条禁「旁路回流主干」，一条禁「主干压扁旁路」） |
| C3 | #828 问 1-2「三层的耦合假设必须一致吗／不一致谁服从谁」（结构耦合 × 会计独立 × 分解空白） | `recursive_regularization_theory.md:53-61` §2.3「和声约束」正是对这一问的直答：三重约束（相位 L0 / 域 L0 / **仓位 L0+L2**），并在 `:61` 给判据「赋格的对位法则要求各声部不协和在终止式协同解决，**缠论里这不是规约而是定理**——区间套定理保证各级别背驰段嵌套……**类比在这一点上不是隐喻而是同构**」 | ➕ 存量给出的答案是「**结构耦合与会计独立不冲突，因为耦合发生在解决点（终止式）而非持仓期**」——#828 的裁定我未能取到 resolution（该票 CLOSED，但 body 里只有问题，未取到评论；**照实：未核**） |
| C4 | #828 问 3「仓位属于哪一级靠什么定」 | #134 已裁「级别身份是**事件**（C3 坐实＝升格时刻）而非静态标签」 | ✅ 一致，且 #134 早于 #828 |

### 2-5 与 #840 定轴表 / #842 R-1…R-15

| # | 在案 | 存量 | 判 |
|---|---|---|---|
| D1 | #840 定轴表：**多重赋格 ＝ 纵向（级别轴，向上）**，H-B 否证「同层分段」读法 | `lesson38_40_concurrent_research.md:51`「『各级别各自独立操作』是原文直说」；`recursive_regularization_theory.md:29` 引 40 课「每一重都对应着一定的资金与筹码，而相应对应着不同的节奏与波动」 | ✅ 一致（存量从未把多重赋格读成同层分段） |
| D2 | #840「**份额只挂纵向，节奏只挂横向**」 | `recursive_regularization_theory.md:33` 主题（subject）＝ 同级别分解操作程式「同一程式在各级别**移位**再现」；`:34` 声部＝ 级别上的资金/筹码层 | ✅ 一致（程式＝节奏挂横向；筹码挂声部＝级别） |
| D3 | #842 **R-4**「多重赋格 ＝ N 个级别的同级别分解**同时**运行 ⟹ 多空双开是它在双向市场的自动结果」 | `analysis/nested_recursive_fugue_design.md:16-19`「一个趋势级别的反向线段在次级别，可能就是一个直接的空单；在高级别的视角看，次级别直接的开单对他只是一个短差」 | ✅ 一致（同一机制，早 7 周） |
| D4 | #842 **R-6**「一重 ＝ 乙：双向循环 + **永远在场**」；净发现「乙使『空仓』不可达」 | `docs/three_phase_unified_design.md:28`「**永远在市场，永远有方向**：走势向上就持多、向下就持空，**不存在空仓真空期**（空仓＝丢失该段绩效）」（编排者 2026-06-20 裁决）<br>`analysis/bidirectional_nested_accounting.md:316-319`「master FSM 状态空间不是 {LONG, SHORT} 两点集，是 `{+Q}∪{+Q−q}∪{−Q}∪{−Q+q}`；**0 ∉ 状态空间**——零仓位只在翻转 bar 内作为执行中间值出现（测度零过渡）」 | ✅ **一致，且早 7 周并已形式化到状态空间层**。R-6 是编排者 2026-08-01 当场裁定；同一形态 2026-06-20 已由同一裁定人裁过一次，且存量给了精确状态空间与例外路径穷举（`:321-322` 强平／配额判 0／创世前） |
| D5 | #842 **R-12**「**各重保证金逐仓分开**（`038:30` 专属筹码 + `040:22` 独立）——顺带修掉 #838 的 G1」；代价明写「保证金占用上升、可用杠杆下降」 | `analysis/bidirectional_nested_accounting.md:354`「**posted(t) ＝ Σₖ nₖ/L_maxₖ（每层按各自极性的 D_struct 全额计提）**；物理净额头寸的交易所维持保证金 ≤ posted；差额 ＝ 机动池增厚，**不得**折算为可部署名义」；`:357` 理由「按净额计提会使层 k 的存活依赖层 j 的反向持仓（层 j 平仓瞬间层 k 强平距离突变）——**删除层间独立性**」 | ✅ **一致，且存量给了公式 + 理由 + 代价条款**。#834 的 G1（「一个 60 天双边满仓的对冲在保证金账上是免费的」）正是 `:357` 那句话描述的病 |
| D6 | #842 **R-9**「缠师禁的是『钱的非长期性』不是杠杆；永续的对应物是强平；他给的办法是『把操作级别降到足够低』⟹ **杠杆与操作级别绑定**」 | `analysis/bidirectional_nested_accounting.md:332-350` §6：`L_max = 1/(D_struct + mm)`，`D_struct_short(j,t) = (P_neg_short(j,t) − c(t))/c(t) + θ_q(j−1,t)`，P_neg 镜像表（MoveDown→ZD_new／Osc→ZG／MoveUp→无定义）+ **五定理逐条镜像核验** | ➕ **R-9 的「杠杆与级别绑定」在存量里已有具体公式**：`D_struct` 就是级别的函数（P_neg 取该级别中枢边界），级别越低 D_struct 越小 ⟹ L_max 越大。R-10 挂账的「数值只能测」在此仍成立，但**形式已在** |
| D7 | #842 **R-1/R-11**「重 ＝ ⟨标的, 操作级别, 筹码⟩；只做第一利润最大定理」 | `recursive_regularization_theory.md:137-144` §4.3「49 课利润最大定理的精确读法」：「『肯定能做出来』＝ **存在性**；『绝对不会丢失筹码』＝ **守恒性**」，`:144`「**存在性 + 守恒性 ≠ 正期望**……它是关于**策略空间内的偏序**的命题，不是关于绝对收益的命题」 | ➕ 存量把「利润最大定理」拆成了三条可分别检验的命题，并标出缠师未给证明。R-11「只做第一定理」不冲突，但**第一定理本身的读法在存量里已被收窄** |
| D8 | #842 **R-3**「选级别是可算的最优化：硬约束 ＝ 交易成本 + 交易误差 ≪ 该级别买卖点间平均波幅，选满足约束的最低级」（`035:30`） | `recursive_regularization_theory.md:90` 命题 3.4「声部 L 可操作，当且仅当其域的期望震荡振幅超过信号分辨粒度与执行摩擦之和：**amp(D_L) > ρ**……存在最低可操作级别 **L\***」+ `:92-96` OKLO L2 校准（ρ ≈ 1% 量级；同中枢配对中位间隔仅 32 bars；bar 级 churn 17,424 次净毁 −11,973；floor≥segment 才进入正收益域） | ✅ **同一判据，且存量带 L2 校准与已实装的 floor 参数**。R-3 是形式，存量是形式 + 一次实测标定 |

---

## 表 3：与 #839 剩余六条 R 的对应

**R 的准确表述取自 #840 resolution「真分歧八条」段**（R1 已由 ADR 0010 解，R5 已由 #841 改写为 R5′）。

| R | 存量回答了没有 | 答案是什么（带出处到行） | 够不够格直接采信 |
|---|---|---|---|
| **R2** 三阶段实例单位错配 | **回答了，但答案与 R2 的诊断相反** | `docs/three_phase_unified_design.md:11`「三阶段是总体性的，不是 per-layer……phase 是全局状态」；`:18` 原文锚 `chan99/0033:11`「级别的意义其实只有一个，基本只和买卖量有关」；#803 resolution「三阶段的删除**不是工程妥协而是原文裁定**……flat `t_engine.rs:105` 的 `TStage` **本就是 GLOBAL，per-instance 才是偏离**」 | **不够格直接采信任一侧** —— 见表 2 X1。两侧都带原文锚且互斥（`031:30` 逐标的 vs `chan99/0033:11` 级别只管量），**须先裁哪条原文管这件事**。另：`docs/three_stages_accounting_design.md`（69KB）**未查到**任何关于实例单位的表述（检索式 `grep -n "每只股票\|逐标的\|每个标的\|单例\|实例单位"`，零命中） |
| **R3** `049:60` 级别门控仓内缺失 | **部分回答，但是另一套机制** | `recursive_regularization_theory.md:90` 命题 3.4 L\*（**振幅/分辨率比**门）+ `:98`「不是小级别信号更不准，而是**小级别域的振幅低于操作点的分辨率**——三元组在 L\* 以下结构性退化」；`organic_fugue_v2_design.md:57`「L\* 分辨率下界……floor=segment 的依据」；`concurrent_fugue_deep_think.md:259-260`「**期货上低级别声部先验关闭**」；已实装为 floor 参数（`multi_level_fugue_cross_asset.md:33-34` floor=segment 9/9 全胜升 L3） | **不够格顶替 R3**。`049:60` 的门是「小级别走不完三阶段（成本归不了 0）」，存量的门是「小级别振幅低于摩擦，短差期望翻负」——**两个不同判据、不同后果**（前者禁阶段推进，后者禁开腿）。代码侧独立复核确认：`ledger.rs` 全文件 `grep -n "level"` **零命中**，`advance_to`(:112)/`stage_progression`(`closed_loop/transition.rs:362`) 签名均无 level 参数 ⟹ **`049:60` 那个门在仓内仍是「未查到」**（检索式已列在附录），不写「不受影响」 |
| **R4** `w_ℓ` default 关闭 + `level_risk.rs:39-42` 注释归属错 | **回答了，从第三个角度** | `level_risk.rs:39-42` 逐字：「`w_ℓ` 全属 **Θ_risk**——级别之间怎么分配资金上限是风险配置的选择……**不是**缠论结构可导出的量。缠论只定义「有哪些级别」，不定义「每个级别该给多少资金」」<br>存量反证：`recursive_regularization_theory.md:59` 仓位约束 3「**L 声部的可操作筹码 ≤ L+1 声部卖出所释放的部分**；更高声部持有的底仓对 L 不可见、不可动」——这是一条**缠论结构导出的级别间资金约束**（虽是上界不是数值），并带 L2 反面物证 `:51`「取消槽隔离后 I_bar0 从 +279% 变为 −99.88% 爆仓」 | **够格作为「注释须重写」的第二条独立依据**（#840 已从原文侧给了第一条：分配的**存在性**有原文 `038:30`/`040:20`/`040:22`）。但**不够格直接定 `w_ℓ` 的值**——存量给的是结构上界不是数值。且这条约束本身与 ADR 0010「专属筹码」冲突（X2） |
| **R6** 重数上界／比例／大小资金分界值原文全无 | **回答了三块中的两块，且比例一块有三套互斥答案** | **上界**：`recursive_regularization_theory.md:104` 命题 3.5「短差声部存在于 **L\* ≤ L ≤ L\*\*** 的带内，L\* 由**振幅/分辨率比**决定，L\*\* 由**域寿命/操作寿命比**决定……两个边界都是**操作者-市场耦合量**，因标的、因 regime、因资金规模而移动（40课）」；**N 的实测上界** `concurrent_fugue_deep_think.md:255-261`「**OKLO 型标的最多 2-3 个有效声部**……N 是数据决定的有限集」；`recursive_empty_domain_research.md:26-27`「可用递归深度 ＝ **e−3**」<br>**比例（三套，互斥）**：(a) `organic_fugue_design.md:198`「`frac[k] = A_k / Σ_j A_j`，A_k ＝ 该层存活中枢相对振幅」；(b) 谱系 `542`「配额比例必须 **σ-不变（级别无关）**⟹ **f ＝ 1/λ**」，已实装 `SUB_SPAWN_FRAC`，且明判 (a) 那类随级别变的归一化是「隐性违反 σ-不变性」；(c) `fugue_version_i_equal_split_comparison.md:42` 均分「**没有改善，部分恶化**」（否证）<br>**大小资金分界**：#842 R-5「N 重的存在理由 ＝ 单一级别装不下钱（容量溢出）」+ 命题 3.5「因资金规模而移动」 | **上界：够格作为机制采信，数值不够格搬运** —— 文档自陈边界条件 `:255`「(i) L2 数据全部来自 OKLO 1min（单标的、强单边 regime、swing 代理标签）」。<br>**比例：一条都不够格直接采信** —— (a) 与 (b) 互斥（振幅比例随级别变 vs σ-不变常数），(b) 已实装但 λ 值 `542` 自陈「未独立测」，(c) 是否证不是构造。**这三条互斥本身就是 #835 该裁的东西，本票不裁。**<br>另：`MAX_LEVEL=8`(`rec_engine.rs:81`)、`MAX_LADDER=11`(`types.rs:28`)、`MAX_REV_DEPTH=4`(`types.rs:65`)、`max_depth=3`(`config.rs:135`)、`MOBILE_FRAC=1/3`(`fugue_v3/mod.rs:115`)、`depth_weights=[0.60,0.30,0.10]`(`config.rs:157`) 六个硬编码上界，**无一带「原文给出此值」的注释**，`rec_engine.rs:81` 反而自陈是派生值 |
| **R7** 立体性 vs 区间套定义层未分名（同一「次级别」两用途：配仓 vs 定位） | **给了一套坐标，但不是按「配仓/定位」二分** | `recursive_regularization_theory.md:73-77` 定义 3.1 三元组 `T_L = (D_L, O_L, F_L)`：**D_L（域）＝ 中枢区间**（配仓的载体）、**O_L（操作点集）＝ 经区间套向 L−1 收缩定位的点**（定位）、**F_L（失效边界）＝{3买,3卖}**；`:79` 命题 3.2 自相似性「**O_L 的定位调用 T_{L−1}**」；`:81` **命题 3.3 失效边界的双重读法**「一个事件，两种读法——**行动由哪个声部的读法主导，决定行动的符号**」 | **够格作为候选坐标，不够格作为分名方案** —— 它把「次级别」拆成了三分量（域/操作点/失效边界）而不是二分（配仓/定位），且 R7 要的是**定义层的两个名字**。代码侧现状：两个用途**已分实现未分名**——定位 `theta_v0/classifier/descend.rs:65,:115`（`RMove`/`descend`，纯读取不改仓位），配仓 `recursive_t/t_engine.rs:612`（`sink`，真下单）+ 守卫 `prove_guards.rs:260`（`prove_sink_descends` 断言 `sub < parent`） |
| **R8** `TStage` 命名三重撞车 | **未查到** | 检索式：`grep -rln "TStage" analysis/ docs/` 命中 10 个文件，逐一核后**无一处讨论命名撞车问题**；`gh issue list --search` 亦未查到专门票 | **未查到，且未做穷举检索**。代码侧读数（本 HEAD 独立核）：**字面标识符 `TStage` 只有 2 处独立定义**（`theta_v0/strategy/ledger.rs:88`、`recursive_t/t_engine.rs:124`，`grep -rn "enum TStage" rust/src/` 精确命中仅此二），第三个同概念实装改了名叫 `RecStage`（`rec_engine.rs:596`）。**原文侧三处撞车**（#840 已列，本次逐字复核属实）：`108:38`「三阶段中的哪一段」＝ 底/中/顶（`108:36`）、`064:496` 牛市三阶段、`091:40` 病的三阶段（未病-欲病-已病）。**⟹ 两种计数口径（标识符名字数 2 / 概念实装站点数 4 / 原文用法数 3），不可混为一谈** |

### 3-x 票面点名要核实的三处，核实结果

| 票面预判 | 核实 |
|---|---|
| 「命题 3.4 可操作下界 L\* 与命题 3.5 可操作带 L\*≤L≤L\*\* 对 **R6**」 | **成立**。原文在 `recursive_regularization_theory.md:90`（命题 3.4）与 `:104`（命题 3.5），逐字为「L\* 由**振幅/分辨率比**决定（3.4），L\*\* 由**域寿命/操作寿命比**决定」——与票面转述一致 |
| 「命题 4.1 + 休止符激活规则『子声部的激活条件是父声部处于域内震荡相位』对 **#834 跨区仲裁**」 | **原文属实**（`:133` 逐字命中），**但它与在案裁定冲突**：#842 **R-6**「一重 ＝ 乙：双向循环 + **永远在场**」+ #842 净发现「乙使『空仓』不可达」。休止符要求子声部在父趋势腿**强制静默**（`:43`「趋势腿中低级别声部强制静默」），乙要求永远在场。**⚠️ X5，不裁**。<br>另：该文档自己也留了同向的顶层例外（`:135`「顶层持仓声部（cantus firmus）除自身级别的一卖/背驰外**永不卖出**——保证基线暴露」），即顶层「永远在场」在存量里也成立，冲突只在**子声部**层 |
| 「命题 5.2 闭腿完备性对 **#839 第 4 问**」 | **部分成立**。命题 5.2（`:171-178`）给的是**单向框架下**的穷尽三分：(a) 延伸→域内低吸；(b) 上破 3买→强制回补 deadline；(c) 下破 3卖→本域闭腿取消、延至新域阶梯。`:199` 明写「**3卖不是短差开腿**——3卖是声部退场」⟹ 它的循环是单向（降成本式）。**它给的是 #839 第 4 问要被改写的那个骨架，不是改写后的答案。**<br>**★ 但改写后的答案在存量里另有一份**：`analysis/bidirectional_nested_accounting.md:296-312` §5.2「翻转 ＝ 会计断面（cascade settle 定理）」给出四步时序，`:316-319` 给出双向状态空间 `{+Q}∪{+Q−q}∪{−Q}∪{−Q+q}`，`:362-376` §7 给了 BTC 一涨一跌的逐 bar 数字推演 |

### 3-y ★★ 存量对 #839 三大问（Q2/Q3/Q6）的现成答案——票面完全未列

这三问是 #839 body 里标 ★/★★ 的「这条不定，下面全部无从谈起」级问题。`analysis/bidirectional_nested_accounting.md`（534 行，2026-06-12）逐条给了答案：

| #839 问 | 存量答案（到行） |
|---|---|
| **Q2 ★ 做空腿的名分**：(a) 第二台镜像机器／(b) 同一台机器的另一半／(c) 别的 | **(a) 与 (b) 分层并存**：`:21-22`「每层一本**带极性的账本**（多头书 ＝ 在册 `OrganicLedger` 逐字；空头书 ＝ 新类型 `ShortBook`），**物理执行 ＝ 各层净额之和**」；`:87`「同一工具双开**否决**在册；净额执行是默认形式；本文全部『多空并存』均指**虚拟账本层**」；`:70-78` 三条理由（会计需要段归属／守恒律是极性专属的／符号污染禁令） |
| **Q3 ★★「筹码」和「成本为 0」在双向市场怎么定义** | `:105-106`「空头的『成本』：**B_s 是单位收入基**——它越高，空头越有利。多头风险解除方向 ＝ cost ↓，空头 ＝ proceeds ↑。**『降成本』在空头侧的精确含义 ＝ 抬高开空均价**」；`:117` 统一公式「**basis ← basis − d·π/units**，d ∈ {+1 多, −1 空}」；`:133-142` **非对称定理（L0）**「多头书的二相守恒律结构在空头书**无镜像**」两支证明（相变不动点不存在：空头损失无界 c→∞／金额守恒算术未定义）；`:107-110` 空头的「挣股数」**被 31 课禁加仓直接否决**；`:492` §10 矛盾登记第 2 条「空头 earning 相……**已获载体层扬弃**（凸性载体买入认沽上两支前提同时消解）⟹ **§2.4 定理有效域收窄为线性载体专属**」 |
| **Q6 杠杆与强平（P6 废除后的空白）** | `:332-340` §6.1 P_neg 镜像表 + `D_struct_short` 公式；`:342-350` §6.2 五定理逐条镜像核验；`:352-358` §6.3 **保证金恒等式** `posted(t) = Σₖ nₖ/L_maxₖ` + 「差额 ＝ 机动池增厚，**不得**折算为可部署名义」；`:167-171`「49 课『满仓』空头侧：满仓 ＝ 名义配额打满，方向无关」+「[镜像] 中枢向下移动时应满空仓、**停止反弹短差回补**」 |

**该文档自己登记的边界（`:514-518` §11.3，必须一并搬运，否则是断章）**：「(i) **全部空头侧条款 ＝ 镜像推导（L0 无原文锚）**，阶段 1 夹逼门是实装先决——门否证则本文降格为『已关轴的会计预案存档』；(ii) 净额等价的有效域 ＝ 同一工具 + 无强平路径；(iii) M=N 定理依赖 `26:34` 单位数量纲读法；(iv) 非对称定理依赖『损失无界』」。**并有一条 L2 真实张力在册**（`:495` §10 第 5 条）：「反手做空 ES/QQQ **全层全形态为负**……双向的严格形式 ＝ **S1-S4 条件轴**（per 标的×层级 准入门），**不是全局极性开关**」。

---

## 表 4：真空白（盘完之后确实没人碰过的面）

| # | 面 | 检索式 | 结论 |
|---|---|---|---|
| V1 | **多重赋格下同一层的「延伸 vs 不延伸」角色冲突** | `grep -rn "延伸.*不延伸\|中间产物\|最终产物" analysis docs .chanlun` | **未查到存量**。#827 结论（A）是 2026-08-01 当场推断，**在本仓无前身**。它同时是 ADR 0011 自标最承重的一条 |
| V2 | **`TStage` 命名撞车的处置** | `grep -rln "TStage" analysis/ docs/` 10 命中逐一核 + `gh issue list --search` | **未查到**任何文档或票讨论此问题（R8） |
| V3 | **多标的（N 个标的 × N 个级别）的重管理** | `grep -rn "多标的\|跨标的.*仓位\|per-symbol" analysis docs .chanlun` 只命中**回测跨标的复验**（`multi_level_fugue_cross_asset.md`）与**选股层**，**未查到**任何关于「同时持有多个标的、每标的各自 N 重」的仓位/账本设计 | **真空白**。ADR 0010 的标的维（`031:30`）在本仓**无任何实装或设计文档**对应——所有赋格设计稿都是单标的的 |
| V4 | **`analysis/` 理论文档与 ADR / `.chanlun/definitions/` 的名分关系** | 见下节附带一问 | **真空白，且无既存规矩** |
| V5 | **`src/newchan/trading/`（Python 交易层，15 文件 3443 行）的名分** | `gh issue list --search` + `grep -rn "src/newchan/trading" .chanlun docs` | **未查到**任何票号覆盖它的名分裁决。#753/#762/#763 核的是 `rust/src/` 四族，不含它 |
| V6 | **`recursive_t` 的 `rec_engine` vs `t_engine` 谁是当前默认生产路径** | `grep -rn "rec_engine\|t_engine" rust/src/recursive_t/mod.rs` + GUARD-ROLE | **未核实**。两者在 GUARD-ROLE 索引里都判「现役」，**未见二者互斥关系的裁决文档**；`t_engine.rs:7` 自陈「当前 flat 实现」。#840 的 090 第 7 条也留了同一条 |

**不是空白但容易被误当空白的两处（照实标出，防下游重复劳动）**：

- **跨声部仲裁**：存量有明确立场——`organic_fugue_v2_design.md:480-484` C2「真实的资源冲突只有一个……**冲突解决应发生在冲突所在层（账本预算），不是概念层**」；`trend_counter_voice_design.md:37`「voice 之间**无显式协调**，耦合只通过共享账本发生」；`concurrent_fugue_deep_think.md:191`「不需要消息传递，**事件总线已经是通信**」。三份一致，即 #834 候选 (b)「各区完全独立、不仲裁」。
- **多声部对冲层的实测状态**：`docs/theta-v0-grammar-audit.md:26`「多声部（声部树）」当前是「**退化（单声部 depth=0，无嵌套对冲）**」；谱系 `676`「π^bsp **子声部恒＝0** 是实装逻辑根因（P3 出场优先 + 跨 carrier 错配），**多声部对冲层从未执行**」；谱系 `693`「否定『A11 诊断层完成』可以支撑『多重赋格/多空双开策略已回测』的声称」。⟹ **本仓的多重赋格在 π 上从未真正跑起来过**，这一条已在册三处。

---

## 附带一问：`analysis/` 下这批理论文档的名分是什么

**本票只报现状，不裁。**

### 现状事实

1. **AGENTS.md 的两条权威声明都不覆盖它。** `AGENTS.md:11-17`「缠论权威链（递减）」只列三层（博文／chan99／思维导图），全是**语料**；`AGENTS.md:19-21`「缠论教义正本」裁定「**`.chanlun/definitions/`（一概念一份）是缠论教义的唯一正本入口**」，并把权威分为三段（原文管语义／Lean 管边界／**生产代码没有发言权、只有否决权**）。**`analysis/` 在这两套里都没有位置**——检索式 `grep -n "analysis/" AGENTS.md CONTEXT.md`，**零命中**。
2. **`docs/agents/` 五份纪律文档也不覆盖它。** 检索式 `grep -rn "analysis/" docs/agents/*.md`，**零命中**。`delivery-discipline.md:16` 第 6 子句只管**本票报告**入仓路径（`chanlun/review-results/`），不管 `analysis/`。
3. **★ map #787 自己把它归类成了「脚本」。** `#787` 正文「在内」疆域表逐字：

   | 区 | 内容 |
   |---|---|
   | `analysis/` | **脚本（658 文件）** |

   实测：`analysis/` 654 个条目里 **`.md` 309 个、`.py` 312 个、`.json` 17 个** —— **接近一半是文档，不是脚本**。
4. **代码侧却把它当设计正本引用。** 全仓 **16 个 `.rs` 文件在注释里引用 `analysis/*.md`（共 20 份不同文档）为设计来源**（检索式 `grep -rlo "analysis/[a-z0-9_]*\.md" rust/src/ | sort -u | wc -l` ＝ 16；`grep -rho ... | sort -u | wc -l` ＝ 20），其中 `rust/src/trading/mod.rs:3-4` 逐字：「设计：`analysis/organic_fugue_v2_design.md`……类型系统参考：`analysis/organic_fugue_rust_design.md`」。`.chanlun/implementation-index-20260723.md:118` 也把 `docs/three_phase_unified_design.md:45-52` 与 `docs/three_stages_accounting_design.md` 列进「formal-chain 数学出处」栏。
5. **tracker 侧对它们零引用（已用控制组验证搜索生效）。**

   | 检索式 | 结果 |
   |---|---|
   | `gh issue list --state all --search "level_risk.rs"`（控制组） | **7 张票命中**（#349/#844/#841/#840/#310/#839/#787）⟹ 搜索确实工作 |
   | `gh issue list --state all --search "bidirectional_nested"` | **0 命中** |
   | `gh issue list --state all --search "recursive_regularization_theory"` | **0 命中** |
   | `gh issue list --state all --search "organic_fugue"` | **0 命中** |
   | `gh issue list --state all --search "three_phase_unified"` | **0 命中** |
   | `gh issue list --state all --search "申克"` | 仅 #844（本票） |
   | `gh issue list --state all --search "休止符"` | 仅 #844 + #787 |
   | `grep -rn "recursive_regularization_theory\|organic_fugue_v2_design" .chanlun/ docs/ AGENTS.md CONTEXT.md` | **0 命中** |

### ⟹ 现状可归纳的三句（事实陈述，非建议）

- **有既存规矩，但它不覆盖 `analysis/`**：#793 立的是「`.chanlun/definitions/` 是教义正本入口」，`analysis/` 既不是语料也不是 definitions，落在规矩之外。
- **两条引用路径互相不知道**：代码引它当设计正本（16 个 `.rs` 文件 / 20 份文档），tracker 与教义文档零引用（0 处），而 map #787 把整个目录标成「脚本」。
- **#753/#762 核的是代码族的名分，不覆盖文档**——本票核实：那两票的产物是 `mod.rs` 里的 GUARD-ROLE 块，对象是 `.rs` 目录，`analysis/*.md` 不在其内。

**名分要不要立、怎么立，回主图裁。**

---

## 090 照实（本票的未尽事项，逐条）

1. **不是穷举。** `赋格|fugue` 命中数百文件，本次逐份精读约 64 份（21 + 43），其余按族群列出并明标「未展开读」，**未给内容描述**。
2. **#828 的 resolution 未取到。** 本报告表 2-4 C3 只用了 #828 的 **body**（五问），**未核**其结案裁定。若 #828 已裁「三层耦合谁服从谁」，本报告该格的判定可能需要重看。
3. **R6 的「原文全无」我没有独立复跑 110 课全量扫描。** 沿用 #840 resolution 090 第 2 条的结论（该票称已逐条读过 `成本为0|退本金|收回本金` 等全部命中），本次只独立复核了其中 7 条具体引文。
4. **`depth_weights = [0.60, 0.30, 0.10]` 在语料中的对应物：未查到，且未做穷举检索。**（#840 090 第 6 条已把这条作废，理由是 depth 就是级别轴故对应物即 `040:20` 三处——本票不复议）
5. **`049:60` 级别门控在代码里「未查到」，不是「不存在」。** 已跑的四条检索式：`grep -n "level" rust/src/theta_v0/strategy/ledger.rs`（0 命中）／`grep -n "fn advance_to" -A 8`（签名仅两个 `TStage` 参数）／`grep -n "fn stage_progression" -A 6 closed_loop/transition.rs`（无 level 参数）／`grep -rniE "level_gate|operating_level|level_match|gate_by_level|level_permit" rust/src/theta_v0/`（命中全是资金**帽**或诊断探针局部变量）。**未做全仓穷举。**
6. **表 1-C「设计稿未被采纳的四组类型」只核了当前 HEAD 的文件集**，未做 git 历史追溯确认它们是否曾存在后被重构删除。
7. **本报告没有做任何回测、没有跑任何代码**，全部读数来自静态读文件 + grep + `gh` 查询。凡引「实测」二字者均转引自在案报告，已标出处。
8. **本票不裁。** 表 2 的五处冲突（X1–X5）、表 3 R6 比例的三套互斥答案，全部只做点名与两侧照抄，**未给倾向性表述**。

---

## 附：本报告点名的五处冲突速查

| 编号 | 冲突面 | 在案一侧 | 存量一侧 |
|---|---|---|---|
| **X1** ★★ | 三阶段实例单位 | `docs/adr/0010:53` 逐 `(标的, 级别)`；#840 R2；承重 `031:30` | `docs/three_phase_unified_design.md:11,:18`（编排者 2026-06-20 裁决）+ #803 resolution；承重 `chan99/0033:11` |
| **X2** | 各重的钱从哪来 | `docs/adr/0010:21-25` **专属筹码**、有没有专属筹码定独立性 | `recursive_regularization_theory.md:59` + `bidirectional_nested_accounting.md:302` **子腿预算基＝父腿释放的敞口，父死子亡** |
| **X3** | 能不能由赋格比喻推理 | `CONTEXT.md:21`「**不得由赋格比喻推理声部的性质**」；`docs/adr/0010:47` | `recursive_regularization_theory.md:31-49` §2 整节即赋格↔缠论十三行对照表 + 由其推出 命题 2.1 |
| **X4** | 低级别方向要不要服从父段 | `docs/adr/0011:51` 裁定五 + #842 R-4 + #787 Notes「跨级双向持仓是目标形态」 | `recursive_regularization_theory.md:57` 相位约束 `phase(L)=sign(父段方向)` + `:38` 不协和须在父段完成前解决 |
| **X5** | 子声部要不要休止 | #842 **R-6**「一重＝乙：双向循环 + **永远在场**」+ 净发现「乙使空仓不可达」 | `recursive_regularization_theory.md:133` 命题 4.1「**子声部的激活条件是父声部处于域内震荡相位**」+ `:43` 休止符「趋势腿中低级别声部**强制静默**」 |

---

**报告完。裁定归 #839 与 map #787。**
