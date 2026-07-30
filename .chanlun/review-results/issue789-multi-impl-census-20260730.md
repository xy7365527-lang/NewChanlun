# #789 重复实现普查：七概念在仓内各实现了几遍、是否等价

- 日期：2026-07-30；票据：[#789](https://github.com/xy7365527-lang/NewChanlun/issues/789)（parent map [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787)）
- 性质：只读勘察，零改动。本报告是 worktree 内唯一写入物。
- **勘察基准树：`git archive main` 解包快照 `/tmp/nc-main-789`（main = `f6d000fed2`）**。
  说明：本 worktree 的 HEAD 是 `19b4015927`（2026-04-20），彼时 `rust/`、`formal/`、`analysis/`
  尚未入仓，worktree 内看不到疆域主体，故全程在 main 快照上勘察。
  已实测 `/tmp/nc-main-789` 与主仓工作目录逐文件一致（`diff -rq` 仅差 `.DS_Store`）。
- 直接采信不重做的在案结论：[#743](https://github.com/xy7365527-lang/NewChanlun/issues/743) 勘察底座
  `.chanlun/review-results/arch-survey-e2e-fable-20260729.md`、[#764](https://github.com/xy7365527-lang/NewChanlun/issues/764)
  顶层散件名分（14 现役 + 2 单列）、[#761](https://github.com/xy7365527-lang/NewChanlun/issues/761)–[#763](https://github.com/xy7365527-lang/NewChanlun/issues/763) 四族改判现役。

---

## 0. 一句话结论（本票对 map #787「甲层要不要重裁」的直接回答）

**甲层必须重裁。** 七个概念全部存在**教义层分歧**（同一个概念在仓内被理解成了不同的东西），
而不只是工程层的不同写法。最硬的三条：

1. **中枢的核心区间取哪几段，仓内有两个不相容的答案**，且这个分歧已被形式化层用 fixture
   反例钉死：同一组输入，`a_center_v0.py` 给 `(11,18)`、`ref_v1` 给 `(12,15)`
   （`formal/Origin/CenterConstruct.lean:494/497`，`native_decide` 已裁）。
2. **单点核心（ZD==ZG）成不成立，Rust 与 Lean 相反**，且这条分歧被 Rust 源码
   **自己写进了文件头登记在案**（`rust/src/theta_v0/classifier/center.rs:11-13`：
   「★与 Lean `Origin.CenterComplete` / `centerHolds` 的 `≤`（弱，单点核心成立）在端点分支上
   不一致——Lean 侧未随 #321 跟进」）。这不是我的推测，是仓内的自陈。
3. **区间套（本图五词之一）在仓内有四种互不相通的坐标系**——时间、`source_index`、bar 区间、
   以及 Lean 侧「纯蕴含关系无区间」——且**全仓零对拍**。五词教义里最核心的一个词，
   代码层从未被统一过，也从未被验证过一致。

反过来，**「工程层分歧」确实也大量存在且已被治理得不错**（增量 vs 全量、Python↔Rust
逐位移植九组等价测试在 CI 常跑），但它们覆盖的恰好是**已经统一了的那条线**（族 I），
遮住了未统一的那几条线。这正是 map #787 立图前提「没有人知道现在到底已经有什么」的实证。

---

## 1. 计数汇总

区分两个量：**代码实体数**（有独立文件/函数体的实现）与**独立教义口径数**
（判据实质不同、同一输入会给出不同结构的口径数）。后者才是甲层裁决的依据。

| 概念 | 代码实体数 | **独立教义口径数** | 已对拍覆盖 |
|---|---|---|---|
| 笔 | 4 | **2 套构造 + 1 套 Lean 谓词（第三种间隔算法，无构造）** | Py↔Rust 已对拍（族 I）；theta_v0 那套**未对拍**；Lean **未对拍** |
| 线段 | 9 | **5** | Py↔Rust 已对拍（v1 族）；theta_v0 静态批处理臂**未对拍**；Lean 两套**未对拍** |
| 中枢 | 13+ | **7** | Py↔Rust 已对拍；Lean↔Rust 有 parity 测试但 **CI 从不执行** |
| 走势类型 | 8+ | **5** | Py↔Rust 已对拍；Lean↔Rust **未对拍**（fixture 无任何走势类型字段） |
| 买卖点 | 12+ | **6** | Py↔Rust 已对拍；Lean↔Rust 有 29 个 parity test 但 **CI 从不执行**；卖侧**无对拍对象** |
| 区间套 | 10+ | **6–8**（坐标系就有 4 种） | **全仓零对拍** |
| 背驰 | 10+ | **8** | Py↔Rust 已对拍（两条口径）；Lean↔Rust 只对拍了一个布尔位，**力度算法本身未对拍** |

**总计：七概念合计约 40 套独立教义口径、70+ 代码实体。**

订正在案事实一条：#743 勘察（2026-07-29）记录的孤儿 `rust/src/theta_v0/classifier/pipeline.rs`
及其「第二份 `classify_impl`」，**在当前 main 已不存在**——全仓 `fn classify_impl` 唯一定义在
`rust/src/theta_v0/classifier/mod.rs:446`。该件已在 #743 收口后消失，判据双源风险的那一条已解。

---

## 2. 七概念 × 实现位置矩阵

区代号：**顶层** = `rust/src/*.rs` 顶层散件（#764 判现役）；**Θ** = `rust/src/theta_v0/`（π 现役主线）；
**前代** = `spiral/`+`fugue_v3/`+`recursive_t/`+`trading/`（#761–#763 判现役）；
**Py** = `src/newchan/`；**Lean** = `formal/Origin/`。

### 2.1 笔

| # | 位置 | 区 | 活着吗 | 定义口径要点 |
|---|---|---|---|---|
| 1 | `src/newchan/{a_inclusion,a_fractal,a_stroke}.py` | Py | 现役（5+ 模块 import） | 新笔=`merged_gap≥2 ∧ raw_gap≥3`；支持老笔 `min_strict_sep=5`；gap 不足 `j+=1`；锁定态**回写改写上一笔终点** |
| 2 | `src/newchan/bi_engine.py` | Py | 现役（gateway 消费） | 同 1 的增量化，复用 1 的全部判据函数 |
| 3 | `rust/src/{fractal,stroke,bi_engine}.rs` | 顶层 | 现役（PyO3 `PyBiEngine` 导出、orchestrator 消费） | 自述「逐位等价移植自 a_stroke.py」，实核对属实 |
| 4 | `rust/src/theta_v0/parser/{inclusion,fractal,stroke}.rs` | Θ | 现役（`parse_layer` 唯一入口，15+ bin 消费） | **只有新笔，旧笔禁用**；`gap_ok = b.source_index − a.source_index > 3`；gap 不足 `i+=2`；`collapse_consecutive` 预坍缩、**不回写** |
| 5 | `formal/Origin/SegmentFeatureSeq.lean:362-391` `IsNewStroke` | Lean | **全仓零引用**（仅谓词，无构造） | `noSharedBar ∧ barsBetween ≥ 3`；`ChanlunElements.strokesOf` 是纯接口字段，**Lean 侧无笔构造实现** |

### 2.2 线段

| # | 位置 | 区 | 活着吗 | 口径要点 |
|---|---|---|---|---|
| 1 | `src/newchan/a_segment_v0.py` | Py | 构造函数仅 `ab_bridge_newchan.py`+测试；但其 `Segment` 类型被 8 个生产模块复用 | **三笔重叠法**，不用特征序列，恒 3 笔，无终结判据 |
| 2 | `src/newchan/a_segment_v1.py` | Py | 现役主路径 | **特征序列增量状态机**：三笔重叠定起点、分型+两种情况定终结、71 课假设转折点、`extend_mode=strict` 缺口封闭降级、`TAIL_WINDOW=7`、`MAX_SECOND_SEQ_SCAN=50` |
| 3 | `rust/src/segment.rs` | 顶层 | 现役（PyO3 + orchestrator 增量） | 2 的逐位移植 |
| 4 | `rust/src/theta_v0/parser/segment.rs::divide_segments_with_tail` + `feature_seq.rs` + `second_kind.rs` | Θ | 现役（唯一生产入口） | 2/3 同教义；两处例外：第二特征序列扫描窗口**默认无限**（显式拒绝复制 Python 的 50 笔性能启发式）、价格域整数 tick |
| 5 | `rust/src/theta_v0/parser/segment.rs::analyze_termination` 及静态原语链 | Θ | **生产零调用者**（仅本文件 `#[cfg(test)]`） | 包含处理用**外包络并区间（非方向性）**、缺口**双侧对称**、无 TAIL_WINDOW/无最小笔数/无缺口封闭降级 |
| 6 | `formal/Origin/SegmentConstruction.lean` `segmentsOf` | Lean | 现役（**`Pipeline.lean:95` 端到端管线用的就是这一版**） | **遇第一根反方向笔即切段**，完全不用特征序列 |
| 7 | `formal/Origin/SegmentAutoConstruct.lean` `segmentsOfComplete` | Lean | **零 import** | 完整判据；第二特征序列取「同序列分型之后的剩余元素」且只找**对偶分型** |
| 8 | `formal/Origin/SegmentFeatureSeq.lean` + `SegmentFeatureComplete.lean` | Lean | 谓词层，与 6 的实际构造未接线（文件自承） | `Overlaps` 闭区间、`HasGap = ¬Overlaps` |
| 9 | `src/newchan/{a_feature_sequence,a_fractal_feature}.py` | Py | **仅 `tests/test_segment_v1.py`** | 特征序列分型用**双条件**（与 K 线分型同判据），全仓唯一 |

（另：`core/recursion/segment_engine.py`、`rust/src/segment_layers.rs` 为纯缓存/增量包装，无自有口径。）

### 2.3 中枢

| # | 位置 | 区 | 活着吗 | 口径要点 |
|---|---|---|---|---|
| 1 | `rust/src/{zhongshu,level}.rs` ≡ `src/newchan/{a_zhongshu_v1,a_zhongshu_level}.py` | 顶层/Py | 现役 | 全三段 `max(3lo)/min(3hi)`；**严格** `zg>zd`；**无方向判据**；延伸闭区间无上限；续进 `i=max(break−2, seg_end)` |
| 2 | `rust/src/theta_v0/classifier/center.rs` + `recursive_tower.rs` | Θ | 现役主塔 | 全三段 + **方向交替（仅 L0；L≥1 走几何路径无方向）**；严格；**≥9 段升级重切**（第 33 课），子中枢继承 seed 核心**不复验 seed 判据**；续进 `i=j` |
| 3 | `rust/src/recursive_t/center.rs` | 前代 | 现役 | 全三段；严格；无方向；续进 **`i=last+2`**（跳过突破段，自述让 c 段命中率从 21%→100%） |
| 4 | `src/newchan/a_center_v0.py` | Py | 现役（`ab_bridge`/`a_recursive_engine`/`a_topology`/analysis 脚本） | 门用全三段，但**发布区间用第 1、3 段** `(max(s1.low,s3.low), min(s1.high,s3.high))`；要求 **s1.dir==s3.dir**（不查 s2）；独有 **`sustain_m` 确认阈值**（settled vs candidate）；续进 `i=seg1+1` |
| 5 | `src/newchan/a_ph_zhongshu.py` | Py | 现役（PH 管线 + 5 个 analysis 脚本） | 输入是 **H0 persistence barcode 的 merge bars**；成员数 **`policy.min_members` 可配置**；重叠判据**可关**（policy 开关） |
| 6 | `formal/Origin/{CenterConstruction,CenterComplete,CenterStates,CenterFull}.lean` | Lean | 现役（Pipeline 消费） | 全三段；成立条件 **`≤`（弱，单点核心成立）**；`CenterComplete` 含方向交替 |
| 7 | `rust/src/theta_v0/classifier/ref_v1.rs` | Θ | 在编译树、**生产零调用**（自陈） | `RefZhongshu` 参照件，`theta_v0_center_parity.rs` 专用；文件头断言 `center.rs` 用「前两段」，**与 center.rs 现状矛盾**（陈旧谱系记录） |

消费/账本层（无自有构造口径，不计入 7）：`center_lifecycle.rs`、`trading/center_book.rs`、
`level_view/`、`level_state.rs`、`a_zhongshu_force.py`、`a_level_fsm_*.py`、
`core/recursion/zhongshu_engine.py`。`spiral/`、`fugue_v3/` **无自有中枢定义**（只透传 zd/zg）。

### 2.4 走势类型

| # | 位置 | 区 | 活着吗 | 口径要点 |
|---|---|---|---|---|
| 1 | `rust/src/moves.rs` + `level.rs` ≡ `src/newchan/a_move_v1.py` | 顶层/Py | 现役 | 趋势 ⟺ **≥2 中枢**；同向 = **核心分离** `c2.zd > c1.zg` |
| 2 | `rust/src/theta_v0/classifier/decompose.rs` | Θ | 现役主塔 | 走势 = 中枢关系链的 **maximal 等标签 run**；盘整 = `LevelExpansion` run（可含任意多中枢）；同向 = **外缘分离** `next.dd > prev.gg` |
| 3 | `rust/src/recursive_t/trend.rs` | 前代 | 现役 | 外缘分离；**盘整方向取首尾单元中点净位移**（全仓唯一） |
| 4 | `src/newchan/a_trendtype_v0.py` | Py | 现役 | 关系**四值**（多出 `higher_center`）；GG/DD 缺失时 fallback 走 **`high`/`low` 双升双降**（第三种同向判据，且函数名 `_centers_relation_by_zg_zd` 与实现不符） |
| 5 | `formal/Origin/TrendCompleteClassification.lean` | Lean | 现役 | `chooseTrend complete hasTwoCenters up`，趋势 ⟺ `hasTwoCenters`（同 1） |
| — | `src/newchan/a_xiaozhuan_da.py` | Py | 现役（`orchestrator/xiaozhuan_da_orchestrator.py`） | **全仓唯一的「小转大」实现**；Rust 三族 + Lean 全无对应 |

（`level_state.rs` `RLevel` 六态 / `TrendSixState.lean` 是**位置态**不是走势类型，`six_state.rs:1-13` 已明确区分。）

### 2.5 买卖点

| # | 位置 | 区 | 活着吗 | 口径要点 |
|---|---|---|---|---|
| 1 | `rust/src/buysellpoint.rs` ≡ `src/newchan/a_buysellpoint_v1.py` | 顶层/Py | 现役 | 一类 = **遍历 Trend 背驰产出**，confirmed 用 `force_c/force_a ≤ 0.9` **面积比阈值**；二类锚 = **一类点的价**；三类回抽 `> zs.zg`；**盘整背驰被显式排除、不产一类点** |
| 2 | `rust/src/theta_v0/classifier/bsp.rs` | Θ | 现役（signal/nest/six_state 消费） | 输出 **bit-vector**（2B/3B 可共存）；`is_first` **判据里无背驰**；`is_third` **无 firstRetrace 项** |
| 3 | `rust/src/theta_v0/classifier/signal.rs` | Θ | 现役（真生产者，5754 行） | 一类 = 破**最后一个中枢**核心 ∧ 037:20 破 b 包络极值 ∧ trend ∧ A/C 可配对；三类严格 `>zg`（`retest==zg` 判不成立）；盘整背驰定位与力度判定分家 |
| 4 | `rust/src/theta_v0/classifier/rmove_compose.rs` + `descend.rs` | Θ | 现役 | 二类 = **次级别第一类 + 回拉不创新低/新高**（买卖点定律一，真下钻）；归属 `OwnerRef::Type1Anchor`（v2 教义修订） |
| 5 | `formal/Origin/{BspClassification,SellPointRecog,BspConstruction,BspEventBridge}.lean` | Lean | 现役 | `IsType1 = brokeCenter ∧ IsDivergence`（**背驰在判据里**）；`IsType2 = afterTypeOne ∧ ¬brokeCenter`；`IsType3Buy` **含 `firstRetrace`**；已证互斥三分**失败** |
| 6 | `rust/src/recursive_t/divergence.rs` | 前代 | 现役 | 自带产 BSP 路径，几何门 = ≥2 中枢 + c 创新高/新低（不判「破核心」） |
| — | `src/newchan/nesting/` | Py | 现役（pipeline 消费） | `BSPType` 只作**状态标签 + 否定判定**，无识别判据 |

身份/账本层（不重判判据）：`bsp_bridge.rs`（episode 身份桥）、`retrace_ledger/`（三类点身份账本，
自陈「消费入口已立、驱动链未接」）、`trading/third_point_book.rs`。
`trading/{nested_interval,recursive_nested}_fugue.rs` 是**操作语义**分歧（BSP→仓位映射），
模块头明说「不改信号层」，不计入判据处数。

### 2.6 区间套

| # | 位置 | 区 | 活着吗 | 口径要点 |
|---|---|---|---|---|
| 1 | `rust/src/theta_v0/classifier/nest.rs` `n_delta` | Θ | 现役（候选过滤门唯一判据源） | 坐标 = **时间**；`is_sub` **闭区间含端点**；top-down；停在**执行级 e**；`Cand^δ_ℓ` 显式标 **[需人工确认]，spec 无定义式** |
| 2 | `rust/src/theta_v0/strategy/nest.rs` `chi_bool` | Θ | 现役 | 同为 time 坐标闭包含，但**末级必须恰为 ev**、级别严格递减（比 1 严）；递归骨架不同（无 `child_interval` 概念） |
| 3 | `rust/src/theta_v0/backtest/econ_positive.rs` `build_nest_certificate` / `_bottomup` | Θ | 现役（前者生产，后者对照基线） | **top-down 与 bottom-up 两种装配方向同仓并存**；top-down 允许 **partial chain** |
| 4 | `rust/src/theta_v0/classifier/cand_sub.rs` + `chain_cert/` | Θ | 在编译树，**纯产出零消费**（自陈） | 坐标 = **`source_index`**；模块头明说与 `NestCertificate`「不共享类型、不互相调用、不互为验证」 |
| 5 | `rust/src/theta_v0/classifier/interval_necessity.rs` | Θ | **零调用**（grep 坐实） | 「点 ∈ 离开窗口闭区间」——全仓最接近 point-contain 的一处，但是必要条件检查器、verdict 不回写 |
| 6 | `rust/src/recursive_t/divergence.rs` `nest_chain_complete`/`d_top` | 前代 | 现役 | 窗口套窗口；**必须逐层降到 a0（k=0）**，任一层断即 false（最严格） |
| 7 | `src/newchan/a_nested_divergence.py` | Py | 现役（`nested_pipeline`/`convergence`/`multi_tf_adapter`） | 坐标 = **bar 区间**；降到 **level 1** 停；中间级取 `matched[-1]`；level≥2 力度 = **振幅×组件数（无 MACD）** |
| 8 | `src/newchan/nesting/` | Py | 现役 | **完全不同的对象**：`N : (SearchSpace, Level) → (Target, BSP)`，**横向 = 搜索空间收缩（配置→角→板块→标的）** 与纵向级别下钻并列 |
| 9 | `formal/Origin/NestingCertificate.lean` / `IntervalNestCertificate.lean` | Lean | 现役 | **两套递归骨架**（Nat 级别差递归+平移不变 vs 列表链递归），文件头互相声明「刻意不合并」 |
| 10 | `formal/Origin/Divergence.lean` `NestNecessary` | Lean | 现役 | **纯蕴含关系，根本没有区间**：`IsDivergence outer → IsDivergence inner`，并证非充分 |

（`nest_index.rs` 索引层、`nest_lifecycle.rs` 活假设 sidecar（明令禁入证书真值路径）不计入判据。）

### 2.7 背驰

| # | 位置 | 区 | 活着吗 | 力度是什么 |
|---|---|---|---|---|
| 1 | `rust/src/theta_v0/classifier/divergence.rs` | Θ | 现役主线 | MACD 段面积 `Σ|hist|`，严格 `<`；**同文件另有 `segments_diverge_or`（面积 ∨ DIF 峰）**；`DivergenceGauge` 四档把力度参数化为 5 proxy 支配序（含 `price_speed` 几何速度） |
| 2 | `rust/src/divergence.rs` ≡ `src/newchan/a_divergence_v1.py` | 顶层/Py | 现役 | **三维度 OR**（面积 ∨ DIF 峰 ∨ HIST 峰）+ T4 零轴穿越前提；无 MACD 时 fallback = **振幅 × 时长** |
| 3 | `src/newchan/a_divergence.py` | Py | 现役 | Move 级 MACD 面积 |
| 4 | `rust/src/recursive_t/divergence.rs` | 前代 | 现役 | **中枢嵌套深度**（`Σ inner_zhongshu_count`），level=0 退化为几何振幅；**零 MACD**；MACD 面积仅作可选第二判据，三模式 Structural/And/Or |
| 5 | `src/newchan/a_divergence_topo.py` | Py | 仅测试 | **persistence 到对角线的 Wasserstein-1 距离** |
| 6 | `src/newchan/a_geometric_momentum.py` | Py | 仅测试 | persistence 泛函；自带 521 号声明「背驰的动量闸必须 MACD」 |
| 7 | `rust/src/theta_v0/classifier/cand_predicate.rs` | Θ | 在编译树 | `DivCand^δ` **四条件结构谓词** + 面积比较 |
| 8 | `formal/Origin/Divergence.lean` + `ForceInterface.lean` | Lean | 现役 | 力度是**抽象标量**（still-MISSING-C，不实装 MACD）；`MacdConformsTo` 是**未证 Prop** |

盘整背驰比较哪两段，三种：`rust/src/divergence.rs` 取**最后两个同向离开段**；
`recursive_t/divergence.rs` 取**进入段 vs 离开段**；`signal.rs` 只定位不判力度、力度另由面积门确认。

---

## 3. 等价性状态（对拍硬证据）

### 3.1 三个对拍机制的实测状态

| 机制 | 对拍双方 | 断言强度 | 当前环境可跑 | **CI 是否执行** | 最近提交 |
|---|---|---|---|---|---|
| `tests/test_rust_*_equivalence.py` 九组 | Python `src/newchan` ↔ Rust 顶层散件（PyO3 `newchan_rust`） | 逐字段 `==`，浮点无容差（真 bit-exact）；`test_rust_macd_equivalence.py` 的批量 pandas-ewm 那条是容差 `rel=1e-11/abs=1e-13` | ✗ 本机无 `newchan_rust`/`newchan`/`pytest` | ✅ **每次 push/PR 真跑**（`ci.yml:97` `pip install ./rust` → `:101` `pytest -m "not slow"`） | 2026-06-08 ~ 2026-07-29 |
| `scripts/check_fixture_drift.py` | Lean 导出器 `#eval` ↔ 仓内 fixture JSON | canonical JSON 递归字段级 diff | ✅（lake/lean 在 PATH，未实跑以免污染只读快照） | ✅ 独立 `fixture-drift` job | 2026-07-29 |
| `rust/tests/theta_v0_*_parity.rs`（center/buy/classifier/lean，共 34 个 `#[test]`） | Rust 实装 ↔ fixture（Lean 机器导出） | `assert_eq!` 逐字段 | 需 cargo | ❌ **全仓 `.github/workflows/` 没有任何一处 `cargo test`**，只有 `cargo check --all-targets` | 2026-07-29 |

**要害（本节最重要的一条）**：`fixture-drift` job 在 CI 里绿，只证明「Lean 源改了 fixture 有跟着重落盘」。
**fixture ↔ Rust 实装** 那半环的验证腿是 `rust/tests/theta_v0_*_parity.rs`，而 CI 里**从来没跑过 `cargo test`**。
也就是说：**Lean→fixture 这半环闭合，fixture→Rust 那半环在 CI 里是断的。**

补充两条：
- **真实行情对拍从未在 CI 跑过**。所有 L2 真实数据用例依赖 `.cache/BZ_1min_2024_raw.parquet`
  （`.gitignore:37` 排除）且带 `@pytest.mark.slow`，被 CI 的 `-m "not slow"` 先一步摘掉。
  **全部对拍证据实际来自确定性合成正弦数据**（bi 6000 bar / recursive 12000 bar）。
- `analysis/segment_refsem_cert.py`（线段参考语义认证 harness）**事实上已停用**：最近提交
  2026-06-26，依赖的 `_xcheck_oklo_strokes.json` 被 gitignore、快照内不存在，不在 pytest 收集路径。
  且它文件内记录的**上次结论是失败**（theta_v0 406 段 vs 参考 237 段，70% 过分段），
  没有证据表明此后重跑过。

### 3.2 七概念的对拍覆盖逐条

| 概念 | Py↔Rust | Lean↔Rust | 状态判定 |
|---|---|---|---|
| 笔 | ✅ `test_rust_bi_equivalence.py`，3 种 mode × 6000 bar 逐 bar 逐字段 `==`，CI 常跑 | ❌ fixture 无笔字段 | **族 I 已对拍；theta_v0 那套与族 I 之间未对拍；Lean 未对拍** |
| 线段 | ✅ `test_rust_segment_equivalence.py` 逐字段 bit-exact（含 break_evidence 嵌套） | ⚠ 仅原语（`theta_v0_lean_parity.rs` 对 `gap_overlap` fixture 测 `Interval::{gap,overlaps}`） | **v1 族已对拍；线段划分整体与 Lean 未对拍；theta_v0 静态批处理臂未对拍** |
| 中枢 | ✅ `test_rust_zhongshu_equivalence.py` 12 字段 bit-exact | ⚠ `theta_v0_center_parity.rs` 有逐字段 `assert_eq!`，**但 CI 从不执行** | **已对拍但 Lean 腿无 CI 看守**；且 a_center_v0/PH 两套口径**完全未对拍** |
| 走势类型 | ✅ `test_rust_move_equivalence.py` 逐字段 | ❌ **两份 fixture 无任何走势类型字段** | **Lean↔Rust 未对拍**（测不出来，照实记） |
| 买卖点 | ✅ `test_rust_bsp_equivalence.py` 逐字段 | ⚠ `theta_v0_buy_parity.rs`(10 测)+`theta_v0_classifier_parity.rs`(19 测) 覆盖 B1/B2/B3，**CI 不执行**；**卖侧 Rust 实装已 #181 退役删除 ⇒ 无对拍对象** | **部分已对拍、无 CI 看守；卖侧未对拍** |
| 区间套 | ❌ **九组测试没有任何一组跑 nest** | ❌ `check_fixture_drift.py` 的 FIXTURES 字典只有 parity/center 两项，区间套**未导出进任何 fixture**；`nest_isolation_guard.rs` 是源码文本扫描守卫，不比较任何数值 | **全仓零对拍。测不出等价性。** |
| 背驰 | ✅ `test_rust_divergence_equivalence.py`（fallback 口径）+ `test_rust_macd_equivalence.py`（在线/面积/峰值是 `struct.pack` 逐位；批量 ewm 是容差） | ⚠ fixture 里只有 `type1_is_divergence: true` **一个布尔位**；`classifier_parity.rs` 文件头自陈「Lean `divPair` 是外部参数（still-MISSING-C 无 MACD 引擎），rust 领先 Origin」 | **力度算法本身与 Lean 未对拍**；且第 4/5/6/7 那四套力度口径**互相之间完全未对拍** |

---

## 4. 分歧归类：**教义层分歧逐条**（本报告的核心输出）

判据（票面给定）：**同一个概念被理解成了不同的东西 = 教义层**；
**同一个理解的不同写法（数据结构 / 增量 vs 全量 / 语言 / 缓存）= 工程层**。
每条给理由，说明为什么它落在教义层而不是工程层。

### 中枢（4 条，最重）

**T-1. 中枢核心区间取哪几段——「全三段」vs「第 1、3 段」**
- 全三段 `max(3lo)/min(3hi)`：`rust/src/zhongshu.rs:185-186`、`level.rs:118-119`、
  `a_zhongshu_v1.py:142-143`、`recursive_t/center.rs:95-96`、`theta_v0/classifier/center.rs:10`、
  `formal/Origin/CenterConstruction.lean:92/99`。
- **第 1、3 段**：`src/newchan/a_center_v0.py:145-150` `_zseg_interval(s1, s3) = (max(s1.low,s3.low), min(s1.high,s3.high))`，
  `:260` 调用；Lean 对位 `formal/Origin/CenterConstruct.lean:490-491` `legacyV0Interval`。
- **理由（为什么是教义层）**：`CenterConstruct.lean:494/497` 已用 `native_decide` 在同一 fixture 上
  钉出两个不同的值 `(11,18)` vs `(12,15)`。同一输入产出不同的中枢边界，
  下游的三类点判据、背驰 A/C 段划分、走势类型关系判定全部跟着变。这不是写法差异，
  是「中枢是什么」的答案不同。

**T-2. 单点核心（ZD == ZG）成不成立——Rust 严格 `<`，Lean 弱 `≤`**
- Rust 全族从严：`zhongshu.rs:251` `if zg <= zd { continue }`、
  `theta_v0/classifier/center.rs:219/257` `if zd >= zg { None }`、`recursive_t/center.rs:98`。
- Lean 仍是 `≤`：`formal/Origin/CenterConstruction.lean:116`
  `centerHolds := decide (computeZD ≤ computeZG)`、`CenterComplete.lean:148`。
- **理由**：这条**已被 Rust 源码自己登记在案**——`theta_v0/classifier/center.rs:11-13` 原文：
  「★与 Lean `Origin.CenterComplete` / `centerHolds` 的 `≤`（弱，单点核心成立）在**端点分支**上
  不一致——Lean 侧未随 #321 跟进」。**这是一条仓内自认的、已知未闭合的教义分歧**，
  不需要我来判定。边界情形下 Lean 认中枢成立、Rust 认不成立。

**T-3. 中枢成立要不要方向判据——三种答案，且 theta_v0 内部自身分裂**
- **要求方向交替**（`s1.dir≠s2.dir ∧ s2.dir≠s3.dir`）：`theta_v0/classifier/center.rs:6-8`、
  `formal/Origin/CenterComplete.lean:146-148`。
- **要求 s1、s3 同向（不查 s2）**：`a_center_v0.py:252-259`。
- **完全不查方向**：`zhongshu.rs`（全文无 direction 进判据）、`level.rs:118-124`、
  `a_zhongshu_v1.py:142-145`、`recursive_t/center.rs:95-98`、`CenterConstruction.lean:116`
  （`CenterComplete.lean:79` 自述「`centerHolds` **完全忽略**这个维度」）。
- **理由**：更要命的是**同一个引擎内部也分裂**——`theta_v0/classifier/mod.rs:431-435` 的
  `is_l0` 分支：L0 走 `detect_centers_complete`（含方向交替），L≥1 走 `detect_centers_geometric`
  （不含）。**同一条递归塔上，「中枢」在不同级别是两个不同的概念**。
  这是纯粹的定义分歧，不可能用写法差异解释。

**T-4. 中枢延伸有无段数上限、会不会升级重切**
- **有，≥9 段整窗重切**：`theta_v0/classifier/recursive_tower.rs:57-70`
  （`UPGRADE_TOTAL_SEGMENTS = 9`，第 33 课）+ `:769-793`；且 `:771-772` 明写重切出的子中枢
  「**不复验 seed 判据**——延伸段仅保证各自触及核心，任意 3 段的三段交/方向交替均无保证」。
- **无上限，永远延伸**：`zhongshu.rs:193-197`、`a_zhongshu_v1.py:104-107`、
  `recursive_t/center.rs:101`、`a_center_v0.py:209`、`formal/Origin/CenterStates.lean:180`。
- **理由**：同一段数据，第 9 段处族 II 产 3 个中枢、其它族产 1 个。中枢**个数**不同 ⇒
  走势类型（趋势 ⟺ ≥2 中枢）跟着翻转。这是「中枢什么时候该拆成更高级别」这一教义问题的
  两个不同答案，不是缓存或增量策略。

（另有 **T-4b：中枢结算后从哪继续扫，四种答案**——`i=max(break−2, seg_end)`（族 I/Lean）/
`i=last+2`（`recursive_t/center.rs:70`，自述让 c 段命中率 21%→100%）/ `i=j`（`recursive_tower.rs:742`）/
`i=seg1+1`（`a_center_v0.py:298`）。续进锚直接决定后续中枢集合，且 `recursive_t` 的注释明说
换锚后 c 段命中率翻了 5 倍——这是**结果层面可观测的**定义分歧，不是实现细节。）

（**T-4c：中枢有无「候选/确认」中间态**——`a_center_v0.py:179` 独有 `sustain_m` 可调阈值
（默认 2），其它所有实现都是几何成立即成立、零参数。中枢成立带可调参数 vs 不带，是定义分歧。）

（**T-4d：中枢成员数是不是固定 3**——`a_ph_zhongshu.py:329` 用 `policy.min_members` **可配置**，
且输入不是笔/段而是 persistence barcode 的 merge bars、坐标是「出生价/死亡价」。这是把「中枢」
搬到了另一个数学对象上，属最彻底的教义分歧。）

### 区间套（3 条，本图五词之一，分歧最深且零对拍）

**N-1. 区间套的坐标系有四种，互不相通**
- **时间**：`theta_v0/classifier/nest.rs:71-74` `is_sub(inner,outer) = inner.start_time ≥ outer.start_time ∧ inner.end_time ≤ outer.end_time`。
- **`source_index`**：`theta_v0/classifier/cand_sub.rs:38-40` `interval_is_sub`；且 `chain_cert/mod.rs`
  模块头明说与 `NestCertificate`「不共享类型、不互相调用、不互为验证」。
- **bar 区间**：`src/newchan/a_nested_divergence.py`（`_level_move_to_bar_range`）。
- **根本不是区间**：`formal/Origin/Divergence.lean:172-173`
  `NestNecessary outer inner := IsDivergence outer.pair → IsDivergence inner.pair`——
  区间套在 Lean 这一支被理解成**一个纯蕴含关系**，并证明了它非充分。
- **理由**：坐标系不同 ⟹ 「A 套在 B 里」这句话在四处是四个不同的命题，无法互相翻译
  （时间与 source_index 在包含合并下不是单调对应）。这是概念本体的分歧。

**N-2. 区间套下降到哪一级停——四个不同的停止级**
- 停在**执行级 e**，且**partial chain 合法**（上级找不到包含段就 break）：
  `theta_v0/classifier/nest.rs` + `backtest/econ_positive.rs:1040-1045`。
- 末级**必须恰为 ev**，多余下级或链反向即 false：`theta_v0/strategy/nest.rs:129-133`。
- **必须逐层降到 a0（k=0）**，任一层断即 false：`recursive_t/divergence.rs:552-568` `nest_chain_complete`。
- 降到 **level 1** 停（因为 level 1 才有完整 MACD）：`a_nested_divergence.py:368-405`。
- 另：`nest_index.rs` 的 exec 循环**从 lvl 1 起扫，L0 被设计性跳过**。
- **理由**：「区间套要套到哪一级」是缠论区间套定义的构成部分（第 62–65 课的核心争点）。
  四个答案对同一行情会给出不同的确认/不确认结论。且这四处**没有任何一处与另一处对拍过**。

**N-3. 「区间套」这个词在 `src/newchan/nesting/` 指的是完全不同的东西**
- `src/newchan/nesting/nesting_operator.py:20-24`：`NestingType{HORIZONTAL, VERTICAL}`。
  **横向 = 搜索空间收缩（配置 → 角 → 板块 → 标的）**，与纵向的级别下钻**并列为同一算子 N 的两个实例**；
  约束是「搜索空间基数单调不增」而非价格/时间区间包含。
- **理由**：这不是同一概念的不同写法，是**同一个词命名了两个不同的算子**。而且这套是现役
  （`pipeline.py:17-24`、`pipeline_backtest.py:24`、`topology/multi_tf_pipeline.py:24` 都在消费）。

（**N-3b：装配方向 top-down vs bottom-up 同仓并存**——`econ_positive.rs:994` vs `:1142`
`build_nest_certificate_bottomup`（后者明标为对照/迁移基线）；Lean 侧
`NestingCertificate.lean`（Nat 级别差递归 + 平移不变定理）与 `IntervalNestCertificate.lean`
（列表链递归）两套骨架，文件头**互相声明「刻意不合并」**。这条介于教义与工程之间：
两套骨架若被证明等价则是工程层，但**仓内没有这个等价证明**，且 memory 在案的
「bottom-up 区间套 ≡ point-contain descend，BTC 三窗 bit-exact 差异 0」只覆盖 descend 那一对，
不覆盖这两套证书骨架。照 090 记为**未证等价**。）

（**N-3c：`Cand^δ_ℓ` 的判据式，三种状态**——`nest.rs:155-157` 标 **[需人工确认]，spec 中仅作符号
出现、无独立定义式**；`cand_predicate.rs` 给出四条件合取定义；`econ_positive.rs:1052-1054`
对 Type2/3 让它 **per-rung 恒 true**。同一个符号在仓内是「无定义 / 四条件 / 恒真」三个东西。
这是教义层的**空洞**，比分歧更严重——仓内自己承认判据式缺失。）

### 背驰（2 条）

**D-1. 背驰的「力度」是什么——五种不相容的量**
1. **MACD 段面积** `Σ|hist|`：`theta_v0/classifier/divergence.rs:231/247`。
2. **三维度 OR**（面积 ∨ DIF 峰 ∨ HIST 峰）+ T4 零轴穿越前提：`rust/src/divergence.rs:309-321`。
3. **振幅 × 时长** fallback：`rust/src/divergence.rs:188-197`、`a_nested_divergence.py:118-134`。
4. **中枢嵌套深度**：`rust/src/recursive_t/divergence.rs:72-81` `leg_strength`——
   `Σ inner_zhongshu_count`，level=0 才退化为几何振幅。**零 MACD**（已亲手核对源码）。
5. **拓扑 Wasserstein-1 / persistence**：`a_divergence_topo.py:96`、`a_geometric_momentum.py`。
- 外加 `theta_v0/classifier/divergence.rs:549-561` 的 `DivergenceGauge` 四档，把力度参数化为
  5 个 proxy 的支配序（含 `price_speed` 几何速度）——即力度**本身是可切换的**。
- **理由**：这条**被仓内源码自己定性为教义 gap**——`theta_v0/classifier/divergence.rs:256-269`
  原文：「背驰仅由 MACD 段面积判定，无独立走势力度判据，**与缠师第 17 课原文相悖**」。
  另 `a_geometric_momentum.py` 自带 521 号声明「**不可能**是纯拓扑动量，背驰的动量闸必须 MACD」——
  两处自陈互相矛盾。这是「力度是什么」这一五词教义问题在代码里的直接投影。

**D-2. 盘整背驰比较哪两段——三种**
- **最后两个同向离开段**：`rust/src/divergence.rs:504-505`。
- **进入段 vs 离开段**：`recursive_t/divergence.rs` `judge_consolidation_divergence`
  （`leg_strength(leave) < leg_strength(enter)`）。
- **只定位不判力度**：`theta_v0/classifier/signal.rs:1089` `locate_pan_div_structure`，
  力度另由 `judge_pan_div` 的面积门确认。
- **理由**：比较对象不同 ⇒ 同一盘整段一处报背驰、另一处不报。是「盘整背驰是什么」的定义分歧。

### 买卖点（3 条）

**B-1. 第一类买卖点：背驰是不是判据里的合取项**
- **在判据里**：`formal/Origin/BspClassification.lean:99` `IsType1 = brokeCenter ∧ IsDivergence divPair`。
- **不在判据里**：`theta_v0/classifier/bsp.rs:78-80` `is_first = below_last_center ∧ ¬after_first_buy ∧ ¬left_center`；
  `bsp.rs:159-160` 明说 `struct_break_dir` 的置位「**与 macd_c_lt_a（背驰）无关**」。
- **背驰是生成源，且用比值阈值**：`rust/src/buysellpoint.rs:195-226`——遍历 `DivKind::Trend`
  背驰产点，`confirmed = force_c/force_a ≤ TYPE1_CONFIRM_RATIO(0.9)`，**不是布尔背驰而是面积比阈值**。
- **理由**：第一类买卖点与背驰的关系，是五词教义里「背驰 → 买卖点」那条边的直接内容。
  仓内三种答案：判据含背驰 / 判据不含（外部预校验）/ 背驰是生成源且阈值化。

**B-2. 第二类买卖点的锚是什么——四种**
- **一类点的价**（同级别价格比较）：`rust/src/buysellpoint.rs:267-271`、`a_buysellpoint_v1.py:217-250`。
- **次级别第一类 + 回拉不创新低/新高**（买卖点定律一，真下钻）：
  `theta_v0/classifier/rmove_compose.rs:144-170` + `descend.rs:183-192`。
- **`afterTypeOne ∧ ¬brokeCenter`**（回抽未再破中枢）：`formal/Origin/BspClassification.lean:110`；
  `SellClosedLoop.lean`/`SecondSellClosedLoop.lean` 明确声明「只用本级别可观测判据，**不冒充次级别递归**」。
- **`after_first_buy ∧ is_pullback_end`**（无 `¬brokeCenter`）：`theta_v0/classifier/bsp.rs:82-84`。
- **理由**：「二买是不是必须靠次级别第一类来定义」正是买卖点定律一的内容，是教义问题。
  Lean 侧甚至明文拒绝走次级别递归。归属载体也跟着分歧：`bsp.rs:149-153` v2 修订为
  `OwnerRef::Type1Anchor`，自陈旧的「载次级别中枢 c1」是「归属层混载确认层对象，**级别错配根因**」。

**B-3. 第三类买卖点：「第一次回抽」是不是判据的一部分**
- **是**：`formal/Origin/BspClassification.lean:113-118` 有 `firstRetrace = true` 合取项。
- **不是**：`theta_v0/classifier/bsp.rs:86-88` `is_third = left_center ∧ retrace_not_reenter`，无该项。
- **隐式承担**：`rust/src/buysellpoint.rs:418` 用「break 后首个反向段」隐式实现。
- `recursive_t/divergence.rs` 自陈：「『第一次回抽』严格序数（第 38 课）**未单独编码**——
  对抗核验 minor，记为有效域边界」。
- **理由**：第 38 课的序数约束在不在判据里，决定了同一次离开中枢后能产几个三类点。

（**反向发现，值得记账**：三类点的回抽判据 **全仓一致用 ZG/ZD（核心），无一处用 GG/DD**——
`buysellpoint.rs:429`、`a_buysellpoint_v1.py:356`、`signal.rs:580`、`BspClassification.lean:117`、
`recursive_t` 的 `types.rs:163`。且严格 `>`（`signal.rs:541-543` 明记 `retest == zg` 判不成立）。
**这是七概念里唯一一处全仓口径完全一致的判据**，可以直接当既定写进甲层正本。）

### 笔与线段（4 条）

**S-1. 线段的构造范式，三种互不相容**
- **三笔重叠法**：`a_segment_v0.py:147-163`——连续三笔交集非空即一段，恒 3 笔，无终结判据。
- **特征序列增量状态机**：`a_segment_v1.py` / `rust/src/segment.rs` / `theta_v0 divide_segments_with_tail`
  ——三笔重叠只用来找起点，终结靠特征序列分型 + 第一/第二种情况。
- **方向反转朴素切分**：`formal/Origin/SegmentConstruction.lean:63-83`——扫到第一根反方向笔就切，
  完全不看特征序列。**且 `Pipeline.lean:95` 的端到端管线用的就是这一版**；
  Lean 侧唯一接近特征序列的 `SegmentAutoConstruct.segmentsOfComplete` **全仓零 import**。
- **理由**：`feature_seq.rs:15-22` 已实测记录过量级差——无状态批处理 406 段 vs 参考 237 段，
  **70% 过分段**。同一份笔序列，段数差一个量级。这是「线段是什么」的三个答案。
  这条也是本次普查最大的一条断链：**形式化层的端到端管线跑的不是生产用的那套线段定义。**

**S-2. 特征序列的包含处理是不是方向性的**
- **方向性**（`effective_up` 决定 max/max 还是 min/min）：`a_segment_v1.py:201-227`、
  `rust/src/segment.rs:337-348`、`theta_v0/parser/feature_seq.rs:186`、`SegmentFeatureSeq.lean:153-168`。
- **外包络并区间**（无条件 `lo=min, hi=max`，代码注释自称「中性实现」）：
  `theta_v0/parser/segment.rs:196-208`（`analyze_termination` 静态臂）。
- **理由**：「特征元素当 K 线做包含处理」这句话被理解成了两件事。
  `feature_seq.rs:20` 归因显示实测只带来 Δ=2 段/1.2%，但**幅度小不改变它是定义级分歧**——
  且该静态臂**生产零调用者、从未与生产臂对拍过**，1.2% 是单点观测不是等价证明。

**S-3. 第二特征序列是什么**
- **新建序列**：从触发笔之后取**与段方向同向**的笔重构，存在**任意分型（顶 OR 底）**即确认
  （`a_segment_v1.py:370-399`、`segment.rs:416-448`、`feature_seq.rs:229`）。
- **同序列剩余段**：直接把首条特征序列里分型之后的剩余元素当 revSeq（`SegmentAutoConstruct.lean:177-199`，
  `:174` 注释自承是「**代理**」），且只找**对偶分型**（向上段找底分型）。
- **理由**：前者由同向笔构成、后者由反向笔构成——**根本不是同一条序列**，判据也不同
  （任意分型 vs 对偶分型）。

**S-4. 笔的「至少 N 根独立 K」被算成三种量**
- `a_stroke.py:176-181`：`merged_gap ≥ 2` **且** `raw_gap = merged_to_raw[cand][0] − merged_to_raw[start][1] − 1 ≥ 3`
  ——两端 merged 区间之间的**净空原始 K 数**，双条件（已亲手核对）。
- `theta_v0/parser/stroke.rs:60-63`：`b.source_index − a.source_index > 3`——两端点**锚 index 之差**，
  单条件（已亲手核对；注释自述「两极值 K 间按原始 K 计数排除两端至少 N 根」，但 source_index
  是合并段起点，与净空计数不是同一个量）。
- `formal/Origin/SegmentFeatureSeq.lean:374`：`noSharedBar ∧ barsBetween ≥ 3`，
  `barsBetween` 语义 = 顶最高 K 与底最低 K 之间（不含）**不考虑包含关系**的根数。
- **理由**：三个量在有包含合并的区间上给出不同的笔集合。同一句「至少 3 根」被算成三种东西。
  另配套三条同源分歧：**旧笔存废**（族 I 支持 `mode=strict` 老笔 min_gap=5；theta_v0 头部
  「启用新笔，**旧笔禁用**」）、**gap 不足时的回退**（`j+=1` 不动起点 vs `i+=2` 放弃起点）、
  **是否回写改写前一笔**（`_extend_prev_stroke` 原地改写 vs `collapse_consecutive` 配对前坍缩，
  而 theta_v0 的 `mod.rs` 头部明文写着「已确认结构不可回写重分解」）。

### 走势类型（3 条）

**M-1. 「盘整」是什么——三种定义**
- **恰好 1 个中枢**：`rust/src/moves.rs:126-142`（`zs_count >= 2` 才 trend）、`a_move_v1.py:134-137`、
  `formal/Origin/TrendCompleteClassification.lean:66`（`hasTwoCenters`）。
- **一段 `LevelExpansion` 关系 run**（可含任意多个中枢）：`theta_v0/classifier/decompose.rs:3-5`。
- **方向断裂即收尾**，盘整方向另取首尾单元中点净位移：`recursive_t/trend.rs:47-77`。
- **理由**：同一批 5 个外缘互相重叠的中枢，族 I 切成 5 个盘整、族 II 产 1 个盘整块。
  走势类型的**个数与边界**都不同。

**M-2. 「同向」判据——核心分离 vs 外缘分离 vs 双升双降**
- **核心分离** `c2.zd > c1.zg`：`moves.rs:63-65`、`level.rs:176-178`、`a_move_v1.py:71-78`。
- **外缘分离** `next.dd > prev.gg`：`theta_v0/classifier/center.rs:274-283`、`recursive_t/center.rs:55`、
  `a_trendtype_v0.py:113-129`、`formal/Origin/CenterStates.lean:215/221`。
- **双升双降** `c_next.high > c_prev.high ∧ c_next.low > c_prev.low`：`a_trendtype_v0.py:131-139`
  （fallback 分支，且用的是 `high/low` 不是函数名声称的 `zg/zd`）。
- **理由**：三者接受集互不包含（因 `dd ≤ zd ≤ zg ≤ gg`，外缘分离 ⟹ 核心分离，反之不成立）。
  同一对中枢，一处判趋势、一处判盘整。

**M-3. 「级别」是递归级别还是从数据涌现的簇**
- **递归级别**（`level_id = 下级 + 1`）：`rust/src/level.rs:46/106`、`a_zhongshu_level.py`、
  `theta_v0/classifier/mod.rs:512`、`a_recursive_engine.py:152`。
- **persistence log-gap 递归分裂簇**：`src/newchan/a_level_detection.py:282` `detect_levels`，
  再经 `CalendarPeriodNamer:77`（L2 经验阈值）映射到「30 分钟/日线」这类时间周期名。
- **理由**：这是**五词之一「级别」的两个不同定义**——一个来自结构递归，一个来自数据统计分布。
  `a_ph_zhongshu.py:282` 的「同层」用的是后者，其「≥3 同级重叠」里的「同级」与缠论递归级别
  不是同一个概念。这条直接命中 map #787 待裁的「统一递归算子 T / 每级别一个 T 实例」。

**M-4（空洞，不是分歧）**：**「小转大」全仓只有一处实现**——`src/newchan/a_xiaozhuan_da.py:142`。
Rust 三族 + Lean 五文件**全无对应概念**。教义层的缺席，比分歧更需要甲层裁一刀。
（`theta_v0/classifier/turn_class.rs` 有 `XiaozhuandaCandidate` 四类 partition，但自陈
「纯只读派生，永远只是必要条件（044:30 纪律）」，不是小转大的判据实现。）

---

## 5. 工程层分歧摘要（同一理解的不同写法，不影响甲层裁决）

| 类别 | 实例 | 为什么是工程层 |
|---|---|---|
| **全量 vs 增量** | `zhongshu_from_segments` ↔ `IncrementalBiZhongshu`；`compose_level` ↔ `detect_centers_windowed_resume`；`decompose` ↔ `decompose_resume`（`decompose.rs:23-27` 自述「单一来源，全量 = 空 state 的 resume」）；`compute_macd` ↔ `MacdState`；`bi_engine` checkpoint 增量 | 判据函数复用同一份；多处自述 bit-exact 并有测试守 |
| **语言复刻** | `a_stroke.py`↔`stroke.rs`、`a_segment_v1.py`↔`segment.rs`、`a_zhongshu_v1.py`↔`zhongshu.rs`、`a_move_v1.py`↔`moves.rs`、`a_buysellpoint_v1.py`↔`buysellpoint.rs`、`a_divergence_v1.py`↔`divergence.rs` | 文件头显式承诺逐位等价，且**九组 CI 常跑的等价测试实测守住**——这是仓内唯一被持续验证的等价链 |
| **数值域** | f64（族 I/Python）↔ 整数 tick i64（`theta_v0/types.rs`）；`slice_max/slice_min` 专为复刻 numpy 约简顺序 | 消除浮点歧义，判据不变 |
| **数据结构** | `Rc<Vec<RMove>>` 免深拷贝；`MoveLookup` 区间表 O(1) 替代 Python O(N²)；`partition_point` 二分替代线性扫；`Rc<Vec<Segment>>` O(1) clone | 纯复杂度优化 |
| **缓存/索引** | `tower_cache.rs`、`nest_index.rs`、`level_view_store.rs`、`judge_first_cached`、`segment_layers.rs`（线段尾部可修订故需 `stable_count` 守门） | 不改判据 |
| **账本/身份化重写** | `retrace_ledger/`（append-only 日志折叠出状态）、`chain_cert/`（Hasse 覆盖边）、`cand_event/book.rs`、`bsp_bridge`（episode 折叠 `BridgeKey`）、`nest_lifecycle.rs`（活假设 sidecar，明令禁入证书真值路径）、`trading/third_point_book.rs`、`center_lifecycle.rs` | 同判据的持久化/身份化，均自陈不参与判定 |
| **事件化包装** | `core/recursion/{zhongshu,move,buysellpoint,segment}_engine.py` 全量重算 + diff 出 DomainEvent | 零自有口径 |
| **操作语义（非识别判据）** | `trading/nested_interval_fugue.rs`（min_trade_ladder 交易 floor）、`recursive_nested_fugue.rs`（「根永不平多」把一类点重解释为开空） | 模块头明说「不改信号层，只改 BSP → 仓位的映射规则」 |
| **透传/消费** | `spiral/ffi.rs`、`fugue_v3/ffi.rs` 原样搬运 `(cs, zd, zg)`；`spiral/`、`fugue_v3/` **零自有中枢/走势/笔/段定义** | 纯适配 |
| **相切边界 `<` vs `<=`** | #246/#277/#288 一次裁定后 `a_segment_v1.py:59`、`segment.rs:140`、`theta_v0 segment.rs:140`、Lean `Overlaps` **已同批统一**；`segment.rs:11-15` 声明「实测段端点零变化，仅 `break_evidence.gap_type` 标签翻转」 | **已收敛的历史分歧**，作为「教义分歧可以被裁掉」的正面样板 |
| **谓词重命名** | Lean `SegEndComplete` ↔ `FeatureConfirmed`（`SegmentFeatureComplete.lean:143` 自承 `Iff.rfl`） | 纯命名 |

**边界一条（记在这里但不当工程层用）**：第二特征序列扫描窗口——Python/顶层散件用
`MAX_SECOND_SEQ_SCAN = 50`，theta_v0 用 `second_seq_scan_window = 0`（无限），
且 `second_kind.rs:44-48` 明说「**不复制** Python 的性能启发式」。
`theta_v0/parser/segment.rs:76` 附 L2 实测「窗口 7→∞ 输出不变（237→237）」，
故**在该数据集上**是工程层；理论上是教义层。照 090 记为「单点实测一致，未证等价」。

---

## 6. 未对拍清单（090 照实：以下全部写「未对拍」，不推测成等价）

### 6.1 完全零对拍
1. **区间套（全部 6–8 套口径）**——九组 Python 等价测试无一组跑 nest；
   `check_fixture_drift.py` 的 FIXTURES 字典只有 `theta_v0_parity.json` / `theta_v0_center_parity.json`
   两项，区间套**未导出进任何 fixture**；`rust/tests/nest_isolation_guard.rs` 是源码文本扫描的
   依赖方向守卫，**不比较任何数值**。→ **区间套的实现等价性在本仓测不出来。**
2. **走势类型 ↔ Lean**——两份 fixture 无任何走势类型字段。
3. **卖侧买卖点 ↔ Lean**——fixture 里 `sell_*` 段仍在，但 Rust 卖侧实装已于 #181 退役删除，
   **当前无对拍对象**。
4. **背驰的力度算法本身 ↔ Lean**——fixture 只有 `type1_is_divergence: true` 一个布尔位；
   `classifier_parity.rs` 文件头自陈「Lean `divPair` 是外部参数（still-MISSING-C 无 MACD 引擎）」。
5. **五套力度口径之间**（MACD 面积 / 三维 OR / 振幅×时长 / 中枢嵌套深度 / persistence）——
   两两之间零对拍。
6. **`a_center_v0` 的首尾两段中枢口径 ↔ 全三段口径**——`CenterConstruct.lean` 有 fixture 反例
   证明**不等**，但**没有任何测试守着「生产不许用 v0 口径」**。
7. **`a_ph_zhongshu` 的 PH 中枢 ↔ 任何其它中枢**——零对拍。
8. **`theta_v0/parser/stroke.rs` 的笔 ↔ 族 I 的笔**——两套口径（S-4），**零对拍**。
   九组等价测试只覆盖 Python ↔ 族 I Rust，**不覆盖 theta_v0**。
9. **`theta_v0/parser/segment.rs::analyze_termination` 静态臂 ↔ 生产臂 `divide_segments_with_tail`**
   ——同文件两套，零对拍（静态臂生产零调用者）。
10. **Lean `segmentsOf`（Pipeline 实用）↔ 生产特征序列线段**——零对拍。
11. **Lean `NestingCertificate` ↔ `IntervalNestCertificate` 两套骨架**——文件头声明刻意不合并，
    **无等价证明**。
12. **`theta_v0/classifier/six_state.rs`、`force_conformance.rs`、`interval_necessity.rs`、
    `cand_sub.rs`、`chain_cert/`**——在编译树、**生产零调用**，其口径与生产口径零对拍。
13. **「小转大」**——全仓仅一处实现，无第二方可对拍。

### 6.2 有对拍机制但 CI 不执行（= 无持续看守，随时可能已经漂了）
14. **全部 Lean↔Rust parity 测试**：`theta_v0_center_parity.rs`、`theta_v0_buy_parity.rs`、
    `theta_v0_classifier_parity.rs`、`theta_v0_lean_parity.rs`（共 34 个 `#[test]`）+
    `nest_isolation_guard.rs` + `src` 内两个 `*_lean_fixture_bit_exact` 单测。
    **`.github/workflows/` 全仓无 `cargo test`**（只有 `cargo check --all-targets` + `cargo fmt --check`）。
    → **fixture→Rust 那半环在 CI 里是断的。**
15. **全部真实行情对拍**——依赖 `.cache/BZ_1min_2024_raw.parquet`（gitignore）且带
    `@pytest.mark.slow`，被 CI 的 `-m "not slow"` 摘掉。**现有对拍证据全部来自合成正弦数据。**
16. **`analysis/segment_refsem_cert.py`（线段参考语义认证）已停用**——最近提交 2026-06-26，
    依赖的 `_xcheck_oklo_strokes.json` 被 gitignore、不在仓内，不在 pytest 收集路径；
    且文件内记录的**上次结论是失败**（406 vs 237 段），无证据表明此后重跑。

### 6.3 仓内自陈的教义空洞（不是「未对拍」，是「没有可对的东西」）
17. **`Cand^δ_ℓ` 无权威定义式**——`nest.rs:155-157` 标 [需人工确认]，「spec 中仅作符号出现，
    无独立定义式」。
18. **`formal/Origin/BuySellPredicate.lean` 疑似编译不过**——`NestingCertificate.lean` 文件头
    逐字记载「经实测 `lake env lean Origin/BuySellPredicate.lean` **不通过**（line 117
    Decidable synth 失败 + line 164 free-variable 错误）」，该文件因此拒绝 import 它、
    在纯 core 重编码 `Conf`；**但它仍列在 `lakefile.toml` 的 Origin roots 里**。
    本次未在只读快照上跑 lake 复验（会写 `formal/.lake`），**只作事实登记，不判真伪**。

---

## 7. 对 map #787 的交付结论

**甲层（五词教义）必须重裁，不能当既定。** 依据是本报告第 4 节的 20 条教义层分歧，
其中五词各自的命中情况：

| 五词 | 教义层分歧 | 最硬的一条 |
|---|---|---|
| **递归** | ✅ | M-3：「级别」有递归级别与 persistence 涌现簇两个定义；`theta_v0` 塔内 L0 与 L≥1 的中枢定义不同（T-3） |
| **区间套** | ✅ **最严重** | N-1 四种坐标系 + N-2 四个停止级 + N-3 横向/纵向同名异物 + **全仓零对拍** + `Cand^δ` 判据式缺失 |
| **背驰** | ✅ | D-1 五种力度；且源码自陈「与缠师第 17 课原文相悖」 |
| **级别** | ✅ | M-3；T-4 中枢升级重切有无（≥9 段）直接决定级别边界 |
| **买卖点** | ✅ | B-2 二买锚四种，Lean 明文拒绝走次级别递归 vs 生产走定律一 |

同时给出两条**可以直接当既定写进正本、不必再议**的：
- **三类点回抽判据 = ZG/ZD（核心），严格 `>`**——全仓 5 处口径完全一致，无一处用 GG/DD。
- **相切边界 `<=`（闭区间含端点）**——#246/#277/#288 已一次性同批统一 Python/Rust/Lean 四处，
  是「教义分歧可以被一次裁掉」的正面样板，重裁流程可照此办理。

以及一条**建议顺带立案**（不属本票范围，仅登记）：CI 里没有 `cargo test`，
导致 34 个 Lean↔Rust parity 断言只编译不执行——甲层重裁之后若要靠 fixture 锁住新口径，
这条腿必须先接上，否则正本落盘也无从看守。

---

*本报告基于 `git archive main`（`f6d000fed2`）解包快照 `/tmp/nc-main-789` 的只读勘察。
七概念清单由三路并行只读勘察产出，教义层最承重的六条（中枢首尾两段区间、Lean `≤` vs Rust `<`、
`centerHolds` 原文、新笔间隔三种算法、`recursive_t` 零 MACD 力度）由本体亲手 Read/grep 复核。
worktree 内除本文件外零改动。*
