# Canonical「完全分类+全定义策略」对照 rust theta_v0 **实装**覆盖矩阵

> 反膨胀完整性核查（编排者质询「**实装**有没有对照结果包做」——核心是 rust 引擎实装，非 Lean spec）
> 逐条核 · 标缺口 · 不声明全覆盖 · 区分定义域（canonical 完整策略）vs 有效域（rust 实装子集）
> 审计日期：2026-06-27 · 只读审计，未改任何代码

---

## 【5层真封后更新 — 2026-06-27 最终验收】

> 下方正文是**补全前** rust 矩阵（完整 39%/部分 39%/缺 7%）。本节是本轮 rust 6 模块改动（signal/recursive_tower/exec/runner/risk/intent）真封后的 rust 侧缺口更新（git 真相核 pub fn）。

| 旧 rust 缺口 | 旧状态 | 真封 rust 模块:函数 | 新状态 |
|---|---|---|---|
| **D2** 杠杆 G_t/N_t/L^G/L^N | ❌ 完全缺 | `strategy/risk.rs`（本轮 +228 行：名义头寸 + 杠杆比，对齐 Lean `LeverageCapital`）| ✅ 已实装 |
| **D5** 风险投影 J_Θ LexArgmin | ❌ 缺/简化（硬编码三路 min）| `strategy/intent.rs`（本轮 +200 行 LexArgmin，对齐 Lean `LexArgmin.lean` 多分量 LexKey）| ✅ 已实装 |
| **C3** 根声部状态机 RootSel | ◐ 压缩 8 态 | `backtest/runner.rs`（退出决策生成器 §9 closePred）+ `strategy/risk.rs`(GlobalRiskClose) | ✅→根方向递归接入 |
| **B4** 第二类买卖点 B2/S2 | ◐ 谓词备而未产 | `classifier/signal.rs`（+259 行 `extract_second_signals` 入口）+ `classifier/recursive_tower.rs`(+416 行塔升级)+`rmove_compose.rs:find_second_type_structure`（次级别第一类构成）| ✅ 已接入（签名层诚实边界）|
| **E5** 平仓闭环 trades=0 | 运行形态缺口 | `backtest/runner.rs:241`（持仓声部台账 + 退出判定逐 bar §9 closePred）+ `strategy/exec.rs`(+157 行 ConflictKey depth 配对)| ✅ 退出决策生成器接入 |
| **中枢生成** parity 空缺 | ❌ 无 parity | `classifier/center.rs`(+187 行) + `recursive_tower.rs` 塔升级（迁主塔 A→B）| ✅ 真中枢判据接入 |

**真封后 rust**：D2/D5 两个硬缺全填补；B2/S2 第二类接入（签名层诚实边界=§10.2 买卖点定律一，第二类由次级别第一类构成，非遗漏）；E5 退出决策生成器接入修复 trades=0 结构根因。
**剩余诚实 still-MISSING（rust 侧，非结构缺口）**：L2 真实回测验证（trades 实测）、parser 全层 bit-exact parity、性能优化、账户层 RiskChrome 完整实装、Θ 参数（w_v/λ/ν/ζ/保证金阈值）L2 经验校准——这些是 goal 自陈不证的 L2/L3 经验层。

## 0. 范围与对照链

**canonical 源（权威文本，定义域）**：
- `~/Downloads/newchanlun-formal/strict_hybrid_state_machine_strategy.md`（S_Θ，619 行，17 节）
- `~/Downloads/newchanlun-claude-code-result-package/FULL_USER_FORMULA_SOURCE.md`（1619 行，22 节完全分类+全定义策略）

**rust 实装（有效域）**：`/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/`（~13700 行）

**Lean↔rust bit-exact 桥**：`rust/tests/theta_v0_{buy,classifier,lean}_parity.rs`（**实测 36 个 #[test]**，非 33）

### 对照链的关键结构（与 `canonical-coverage-classification.md` 的区别）

```
canonical 文本（S_Θ / FULL_USER_FORMULA）
   │
   ├──[已有 docs/canonical-coverage-classification.md 覆盖]──> formal/Origin/*.lean（Lean spec）
   │
   └──[本文档覆盖]──> rust/src/theta_v0/（rust 实装）
                          │
                          └──[parity 桥]──> formal/Origin/*.lean（bit-exact 对齐）
```

**关键**：rust 的契约锚**不直接锚 canonical 文本**，而锚 `formal/Origin/*.lean`（reference-theta-v0.md
冻结的 Θ v0 规格，codex gpt-5.5 编排者全权代理裁决 2026-06-25）。因此「rust 实装对照 canonical」=
两跳：rust ↔ Origin Lean（parity bit-exact）+ Origin Lean ↔ canonical（已由 classification 文档核）。
本文档核第一跳 + 标 canonical 机制在 rust 的实装状态。

### L 级标注（`formalization-validity-domain.md` 强制）

| L 级 | 含义 | 本文档用途 |
|------|------|-----------|
| **L0** | 纯结构/定义，machine-checked，不依赖数据 | rust 结构判定（分型/中枢/买卖点谓词） |
| **L1** | 合成/golden 一致性，bit-exact 对齐 Lean fixture | parity 测试、conformance |
| **L2** | 真实数据假设检验，可证伪 | backtest OOS（见 `theta-v0-l2-results-v0.md`） |

状态 ∈ {**完整** / **部分** / **桩** / **缺**}。

---

## 1. 覆盖矩阵（逐条核，按 canonical 章节）

### A. 解析层 Θ_parse（canonical §4 自相似递归核 + §5 走势分类的解析输入）

| # | canonical 机制（出处）| rust 模块:函数 | Lean 对照(parity?) | 状态 | L级 |
|---|---|---|---|---|---|
| A1 | K线包含合并 | `parser/inclusion.rs:102` `process_inclusion` | Origin.ElementPipeline.mergeBars（**无 parity 测试**）| 完整 | L0 |
| A2 | 分型（顶/底严格） | `parser/fractal.rs:34` `detect_fractals` | Origin.ChanlunElements.fractalsOf（**无 parity**）| 完整 | L0 |
| A3 | 笔（新笔定义，第81课） | `parser/stroke.rs:75` `build_strokes` | Origin.strokesOf（**无 parity**）| 完整 | L0 |
| A4 | 线段（67课特征序列法） | `parser/segment.rs:323` `divide_segments_with_tail` + `feature_seq.rs:200` `FeatureSeqState` | Origin.segmentsOf（**无 parity**；L1 对 Python a_segment_v1 交叉验证）| 完整 | L0/L1 |
| A5 | 古怪线段（第77/78课，无特征分型） | `parser/segment.rs` `SegmentTermination::NoFractal` + `second_kind.rs:142` `resolve_second_kind` | Origin **动态划分有意非形式化**（无 Lean bit-exact，对 67课博文忠实）| 完整（不产假线段）| L1 |
| A6 | canonical 分解（破平局键）| `parser/canonical.rs:95` `canonical_endpoints` + `:75` `gauge_fix` | Origin.minGauge（**优化层 native port 缺**，L0 段层退化为恒等）| 部分（L0 段层完整，跨层 gauge 原语备而未用）| L0 |
| A7 | 未完成尾部 D^active（§4 `D=(D^confirmed,D^active)`）| `parser/tail.rs:216` `build_tail` | Origin.OpenTail（**无 parity**）| 部分（3/5 类型：PendingFractal/Stroke/Segment；AliveCenter/PendingMove 移交 classifier）| L0 |

**反膨胀核**：parser 7 步流水线代码无 TODO/桩/unimplemented，但 **A1-A4 完全无 parity 测试**——rust 解析层
与 Lean 无 bit-exact 交叉验证（仅 classifier 层起有 parity）。古怪线段动态划分 Origin 有意非形式化，
rust 对博文忠实但非 Lean bit-exact（设计边界，非遗漏）。

### B. 分类层 Θ_level + Θ_signal（canonical §5 走势分类 + §6 区间套）

| # | canonical 机制（出处）| rust 模块:函数 | Lean 对照(parity?) | 状态 | L级 |
|---|---|---|---|---|---|
| B1 | 中枢 zd/zg/dd/gg（前三连续次级走势）| `classifier/center.rs:` `center_from_segments`(完整判据)/`center_from_window`(几何) | Origin.CenterComplete.CenterConfirmedComplete + centerHolds（位置态有 parity，**中枢生成逻辑无 parity**）| 完整 | L0 |
| B2 | 走势类型 τ∈{⊥,P,U,D}（§5）| `classifier/level.rs:` `classify_move` → `MoveOutcome`(Trend/Consolidation/HigherCenterCandidate) | Origin.TrendCompleteClassification.chooseTrend（**无独立 parity**）| 完整 | L0 |
| B3 | 位置 r∈{⊥,I,U0,U1,D0,D1}（§5）| `classifier/level_state.rs:` `RLevel`(6态) + `rlevel_of` | Origin.CenterStates.classifyPosition（**有 parity**：`classifier_parity.rs:116-169`）| 完整 | L0/L1 |
| B4 | 信号位 b=(B1,B2,B3,S1,S2,S3)∈{0,1}^6（§5，非互斥 2B/3B 可重合）| `types.rs:172` `BspBits`(6 独立 bool) + `classifier/bsp.rs` `is_first/is_second/is_third` + `endpoint_to_bsp` | Origin.BspClassification.IsType1/2/3 + no_exclusive_trichotomy（**第三类有 parity**，1/2 类谓词有但不产出）| **部分**（见下）| L0/L1 |
| B5 | 区间套递归证书 N^δ_{ℓ↓e}（§6）| `classifier/nest.rs:` `Chi`+`nest_valid` + `descend.rs:` `descend`+`sub_level_type1` | Origin.SubLevelDescent.{Chi,descend,NestCertificate}（**无 parity 测试**）| 完整（结构，**无 parity**）| L0 |
| B6 | Sel_Θ 固定选择器（§6 多候选唯一选定）| `classifier/nest.rs:` `sel_order`(三键字典序)+`select_best` | Origin.SubLevelDescent.selOrder（**无 parity**）| 完整 | L0 |
| B7 | 背驰度量（§6 Confirm/Candidate 力度）| `classifier/divergence.rs:` `segment_macd_area`+`is_divergence` + `force_conformance.rs` ForceMeasure | Origin.Divergence.IsDivergence + ForceInterface（**rust 半 L1，Lean↔rust bit-exact 未验=L2 假设**）| 部分（MACD 面积比真算；EMA 首值/常数 `[L3经验待标定]` 占位）| L1 |

**⚠边界声明作废（B1 `centerHolds` parity——中枢成立落点 ZG==ZD）**：#290 裁定 B（2026-07-26 用户裁决，
#314 登记）。Lean `Origin.CenterConstruction.centerHolds` 为 `ZD ≤ ZG`（**弱**，单点交集成立；同
`ChanlunElements.Center.valid`），Python 全家族（`a_zhongshu_v1._scan_zhongshu`、`a_center_v0._try_init_center`）为**严格**
（`zg <= zd` 跳过 ⟹ 单点**不**成立）。**该落点的 Lean↔Python 对齐声明在本边界上作废**——两侧语义相反，
**此边界不作机械锁用**（B1 的「位置态有 parity」不覆盖成立谓词的端点分支）。依据：原文对单点中枢
**未涉及**（17 课定义/20 课公式无端点口径；22 课 Q&A `022-第22课.md:514` 缠师答的是级别谬误，未答
单点之问），不擅自发明 ⟹
维持 Python 严格口径。下游真空照实：单点中枢若成立，其下游（定理二分支/三类买卖点/092 监视器 Z 值）全落原文真空。
**输入域实测（ZG==ZD 输入，落码口径）**：OKLO **0 次**、BZ2024 **2 次**（均在 L1）；Python 两版
（v0 `a_center_v0._try_init_center` / v1 `a_zhongshu_v1._scan_zhongshu`）**均判不成立** ⟹ 单点中枢
成立分支及其下游真空**在 Python 链上不激活**。
⚠**不可推广到 Rust**：Rust θ v0 **生产链是弱口径**——`rust/src/theta_v0/classifier/center.rs` 的
`center_from_segments` / `center_from_window` 两处均为 `if zd > zg { return None }`（单点 `zd==zg`
通过），同文件 doc 明写「`ZD_B=ZG_B` 单点核心合法」。故该分支在 Rust 生产链上**可达且判成立**，
下游真空是**激活的**。此矛盾（Lean 弱 / Python 严格 / Rust 弱）**超出 #290 裁定 B 的有效域**，
已另立票 **#321**（成立口径三方矛盾，grilling）。
**与 #290 裁定 A 无交集**（裁定 A 改的是重叠/延伸判定：`a_center_v0._has_overlap`、
`a_level_fsm_newchan.overlap` 两谓词严格 `<` → 含端点 `<=`，对齐中心定理一 `020-第20课.md:56` 与 Lean
`CenterExtension`/`CenterBroken`；实测 golden 锁不波及）。登记模板：#249/#288 先例。裁定书链：
`chanlun/review-results/center-tangency-doctrine-20260726.md`、
`.chanlun/review-results/center-tangency-blast-radius-20260726.md`。

**#321 后续裁定（2026-07-26 用户裁决，落码）**：上段「Rust 生产链是弱口径」的悬案已裁决——**从严**，
与 #290 裁定 B / Python 严格口径三方对齐。`rust/src/theta_v0/classifier/center.rs` 的
`center_from_segments` / `center_from_window` 两处判据均由 `if zd > zg { return None }` 改为
`if zd >= zg { return None }`（单点 `zd==zg` 不再通过），`types.rs` `Center` doc 同步更正。理由：原弱
口径本是实现者自选、无教义依据（#290 裁定 B 是首个教义裁定，Lean/Rust 此前均只是实现者自选未经裁定
的弱口径），原文（17 课定义/20 课公式）未涉及端点口径，22 课 Q&A（`022-第22课.md:514`）单点中枢之问
缠师答的是级别谬误（未答单点之问）在案，不擅自发明 ⟹ 统一向严格看齐。

⚠边界声明作废：中枢**成立**落点 ZD==ZG —— Lean `centerHolds` 为 `ZD ≤ ZG`（弱，单点成立）/ Rust
`center_from_segments`/`center_from_window` 严格（不成立，#321 裁定）；**该落点的 Lean↔Rust 对齐声明
在本边界上作废，此边界不作机械锁用**（不得据本实装断言 Lean 侧行为，亦不得据 Lean `centerHolds`
反推本实装期望值）；其余落点对齐声明不受影响。Lean 侧仍弱（对方线地盘）——跟进留对方线。裁定书：
GitHub issue #321（2026-07-26 用户裁决）。

**反膨胀核（B4 三类买卖点完整性——编排者最关心）**：
- **bit-vector 6 位结构完整**（`BspBits` 每位独立可置，2B/3B 可共存，符合 `no_exclusive_trichotomy`）。
- **但 v0 实际只产第三类**（`signal.rs:9-26` 诚实声明）：`extract_third_for_center` 只判第三类（confirmed
  结构严格可判定，零经验时序）。**第一/二类谓词存在**（`bsp.rs is_first/is_second`）但 **signal.rs 不调用**
  ——第一类需「趋势末段背驰确认」（多级别时序定位段边界），第二类需第一类后回调时序，超出单中枢局部可判定范围。
- **次级别层第一/二类有结构实装**（`descend.rs sub_level_type1` + `second_type_via_subLevel_type1`），但未接入
  signal 主提取流。诚实分类：**第三类=完整产出；第一/二类=谓词备而未产**（非桩，是结构边界）。

**B5/B6 区间套**：结构完整（多级递归 descend + Sel_Θ 字典序）但 **完全无 parity 测试**（Origin.SubLevelDescent
无对应 fixture）——这是「实装了但未 bit-exact 对照 Lean」的典型缺口。

### C. 策略层 Θ_voice + Θ_phase + Θ_ledger（canonical §8-10）

| # | canonical 机制（出处）| rust 模块:函数 | Lean 对照(parity?) | 状态 | L级 |
|---|---|---|---|---|---|
| C1 | 声部树 T=(V,p)，σ_v=σ_r(-1)^depth（§8）| `strategy/voice.rs:98` `voice_side` + `VoiceState` + `act_state`(4态) | Origin.VoiceTree.{voice_side,actState,targetPos}（**无独立 parity**，单元测试覆盖）| 完整（架构支持多层）| L0 |
| C2 | 声部 4 公理（祖先闭合/同单位/父仓保持/每父一活子，§8）| 分散：`voice.rs within_max_depth/depth_weight` + `strategy/mod.rs plan_orders` parent_cap | Origin.VoiceTree 公理（**无单一 parity**）| 部分（4 公理分散在多模块，无单一全局检查）| L0 |
| C3 | 根声部状态机 RootSel_Θ + 根方向递归（§9）| **压缩为** `strategy/intent.rs:103` `action_priority`(risk×phase 摘要) | Origin.FullDefinitionStrategy（RootSel_Θ/tilde_σ **未显式实装**）| **部分**（压缩为 8 态摘要，未显式根方向递归）| L0 |
| C4 | 三阶段资本账本 R=Π-A-W（§10）| `strategy/ledger.rs:272` `LedgerComp`+`inv_holds` + `ledger_step` | Origin.FullDefinitionStrategy.LedgerState（**有 parity**：`buy/lean_parity` ledger delta）| 完整 | L0/L1 |
| C5 | TW 三阶段 + OQ-9 gate（§10 ReadyReturn/增单位）| `strategy/ledger.rs:122` `TwState` + `tw_step` + `is_legal_from`(OQ-9) | Origin.TotalWealth.{TWState,LegalTransition}（**有 parity**：transition ledger）| 完整 | L0/L1 |
| C6 | Phase I/II/III 五态互斥分类（§10/§12）| `closed_loop/state.rs:` `Phase`(3态) | Origin.CapitalPhase（**5 态 → 3 态摘要**，对齐诚实延后 `state.rs:138-143`）| **部分**（III_repair/protected/accretive 三子态压缩为 PhaseIII）| L0 |
| C7 | 增单位 ΔQ=LotFloor(a/m_unit)，α 收敛（§10）| `strategy/risk.rs:` `size_position` lot 取整 | Origin（**LotCost/α 收敛比未显式实装**）| 部分（lot 取整有，α 收敛/m_unit 阈值判定缺）| L0 |

**反膨胀核（C3/C6/C7）**：
- **C3 根声部状态机是关键缺口**：canonical §9 的 `RootSel_Θ(1,1)` 镜像反对称消歧 + 「先平根仓下周期才反向」
  递归 **在 rust 无对应代码**——rust `action_priority` 是 (RiskMode×Phase) → {Buy,Reduce,Add,Close} 的 8 态映射，
  Origin Lean 侧有 `action_priority_complete_unique` 证 10 级唯一，rust 只是其在 (risk,phase) 摘要上的投影。
- **C6 五态压三态**：诚实标注，但意味着 III_repair/protected/accretive 的区分（决定是否增核心单位）在 rust 未落地。

### D. 风险/杠杆/执行 Θ_leverage + Θ_risk + Θ_execution（canonical §11-13, §19-20）

| # | canonical 机制（出处）| rust 模块:函数 | Lean 对照(parity?) | 状态 | L级 |
|---|---|---|---|---|---|
| D1 | 风险模式 μ∈{Insolvent..Normal} 按 M0-M4（§11）| `closed_loop/state.rs:` `RiskMode`(5态) | Origin.FullDefinitionStrategy.RiskMode（**无独立 parity**）| 完整（枚举），**部分**（M0-M4 优先级判定逻辑未见真实 E_t/MM_t 计算）| L0 |
| D2 | 杠杆 G_t=Σ\|n_v\|, N_t=\|Σn_v\|, L^G/L^N（§11）| **缺**（无 `total_notional`/`net_notional`/`leverage` 函数）| Origin（**无对应**）| **缺** | — |
| D3 | 全局安全可行集 K_Θ（§12，~14 条约束）| `strategy/risk.rs size_position`(三路 min) + `closed_loop transition.rs risk_adapter`(网格{0,1,2,3}) | Origin.RiskProj（**无 parity**）| **部分**（约束分散；IM/MM/StressLoss/后代全平/同单位明文检查缺）| L0 |
| D4 | 动作优先级 A1-A10 互斥分区（§16/§18）| `strategy/intent.rs:103` `action_priority`(8 态) | Origin.action_priority_complete_unique（**压缩投影，无 parity**）| **部分**（A4 未完成订单/A6 结构失效/A7 关闭反向子/A9 建立反向子 **无独立分支**）| L0 |
| D5 | 风险投影 J_Θ LexArgmin（§19）| `strategy/risk.rs:168` `size_position`(硬编码三路 min) | Origin.RiskProj（**无 parity**）| **缺/简化**（未实装 J=Σw(q'-q~)²+λTradeCost+νRiskPenalty+ζTurnover 加权字典序，硬编码三路 min）| L0 |
| D6 | 订单 Schedule_Θ 固定执行次序（§20）| `strategy/exec.rs:` `ConflictKey`(6 键字典序) + `fill_bar_index` | Origin（**无 parity**，全序隐含实装）| 完整（次序经 ConflictKey 字典序隐含）| L0 |
| D7 | 外部事件 e_{t+1}（Fill/Reject/Fee/Funding/Margin/Borrow/CorpAction，§20）| `closed_loop/transition.rs AssemblyEvent`(仅 `parse_event`) | Origin.Event（**仅 MicroEvent，外部事件 7 元组未处理**）| **部分**（仅价格/笔事件，成交/拒单/费用/保证金事件缺）| L0 |

**反膨胀核（D2/D5 是硬缺口）**：
- **D2 杠杆 G_t/N_t 完全缺**：canonical §11 的总名义/净名义头寸、总杠杆/净杠杆计算 **rust 无任何对应函数**。
  「同单位双开净杠杆低但总杠杆高，必须同时约束」这一 canonical 核心约束在 rust 未实装。
- **D5 J_Θ LexArgmin 缺**：`size_position` 是三个硬编码上界的 min，**不是**参数化加权目标的字典序最小化。
  canonical §19 的 J_Θ 四项加权（拟合误差/交易成本/风险罚/换手）在 rust 无对应。

### E. 闭环装配 S_Θ（canonical §1 单一总式 + §17 总定理）

| # | canonical 机制（出处）| rust 模块:函数 | Lean 对照(parity?) | 状态 | L级 |
|---|---|---|---|---|---|
| E1 | 完整状态 x_t（§3，~17 分量）| `closed_loop/state.rs:` `AssemblyState`(8 字段) | Origin.StrictState（**乘积态扩展**）| **部分**（17 分量中 7-8 完整字段 + 摘要计数；h_t/σ_r/E_t/Cash_t/完整 O_t/M_t 缺或摘要）| L0 |
| E2 | T_Θ 状态转移 x_{t+1}=T_Θ(x_t,π,e)（§1）| `closed_loop/transition.rs:` `transition_adapter` + `hybrid_step` | Origin.FullDefinitionSystem.{transition,hybridStep}（**conformance 有 parity**）| 完整（双账本写回每 bar 喂回）| L0/L1 |
| E3 | 六段数据流 rec→classify→intent→risk→schedule→T（§17）| `closed_loop/transition.rs:` `policy_output`(前5段)+`transition_adapter` | Origin.policyTheta（**conformance 有**：`conformance.rs` U5）| 完整 | L0/L1 |
| E4 | 总定理唯一性（§17 ∃! (D,c,u~,u*,O,x')）| `closed_loop/conformance.rs:` `ThetaV0Contract` step/classify 全函数确定唯一 | Origin.EngineBridge.RustEngineContract（**有 parity**）| 完整（L1 conformance）| L0/L1 |
| E5 | 买卖闭环（三类买卖点建仓→持仓→平仓）| `closed_loop/buy.rs` `recog_chanlun_buy` + `sell.rs` `recog_chanlun_sell` | Origin.ThetaInstantiation + SellClosedLoop（**买/卖各有 parity**：buy_parity 9 + lean_parity 16）| **部分**（第一/三类有；第二类缺；**sell_transition 游离 hybrid_step 主流程**）| L0/L1 |

**反膨胀核（E1/E5）**：
- **E1 状态不完整**：rust `AssemblyState` 8 字段覆盖 canonical 17 分量约 50%。缺：h_t（历史）、σ_r（根方向编码）、
  E_t（清算权益，杠杆/风险约束的输入）、Cash_t、完整 O_t 订单清单（仅计数）、完整 M_t 信号记忆（仅计数）。
- **E5 买卖闭环运行形态**：买卖点谓词在 buy.rs/sell.rs 都识别第一/三类，但 **hybrid_step 主驱动是 RiskMode×Phase→Buy
  的被动导出**；主动卖点 `sell_transition` **不在 hybrid_step 单步流程内**（`sell.rs` 工位定位标 MISSING）。
  这是 L2 回测 `trades=0` 的结构根因（见 §3）。

### F. 结构等变（canonical §7 镜像 / §4 自相似）

| # | canonical 机制（出处）| rust 模块:函数 | Lean 对照(parity?) | 状态 | L级 |
|---|---|---|---|---|---|
| F1 | 镜像等变 M²=id，U↔D/B_i↔S_i（§7）| 买卖对偶分散（`Side::flip`/`Direction::flip`/sell 是 buy 对偶）| Origin（**lean_parity 用买侧 fixture 对偶验卖侧**：`lean_parity.rs:271-287`）| 部分（对偶在 parity 验证层，无独立 mirror 算子 Rec_{MΘ}）| L0/L1 |
| F2 | 自相似递归核 N_{ℓ+1}∘F_ℓ=F_*∘N_ℓ（§4）| `classifier/mod.rs classify`(L0..Lmax 递归) + `descend.rs descend`(级别递减) | Origin.RecursiveLevelSystem（**无 parity 测试**）| 部分（级别递归结构有；规则自相似归一化算子 N_ℓ 未显式）| L0 |

---

## 2. parity 桥逐个核（rust↔Lean bit-exact，编排者要 bit-exact 对照验证）

**实测 36 个 #[test]**（9 buy + 11 classifier + 16 lean/sell），**非「33 parity」笼统声明**。

### 2.1 golden 来源真实性 = 真 bit-exact ✓

- golden 来自 `formal/Origin/ParityFixtureExport.lean` 的 **`#eval` 机器导出** → `fixtures/theta_v0_parity.json`。
- **非硬编码常数**：`631 点名修复`（`lean_parity.rs:271-287`）把买侧 delta 硬编码 `1` 改为 fixture 读，消转录漂移。
- **边界口径对齐**：codex 裁决 2026-06-27，rust 严格 `>`/`<` 与 Lean 严格对齐（`classifier_parity.rs:308-334`）。
- **负向探针**：`buy_parity.rs:409` `buy_recog_negative_probes_non_vacuous` 证 parity 可证伪（非 vacuous）。

### 2.2 parity 覆盖机制矩阵（哪些机制有 bit-exact 对照，哪些无）

| 机制 | 有 parity? | 说明 |
|------|-----------|------|
| 分型/笔/线段 | **❌ 无** | parser 已实装，**完全无 rust↔Lean 交叉验证** |
| 中枢生成 | **❌ 无**（位置态 ✓）| 中枢位置判定 r 有 parity，中枢构造逻辑无 |
| 第一类买卖点 | ✓（谓词 + 对偶）| 谓词有 parity，但 signal v0 不产出 |
| 第二类买卖点 | **❌ 无** | 未接入主流，无 parity |
| 第三类买卖点 | ✓ **完整** | buy/classifier/lean 三侧 bit-exact |
| 区间套/Sel_Θ | **❌ 无** | 结构实装但无 Lean fixture |
| 背驰力度 | **❌ 无**（半 L1）| rust 半实装，Lean↔rust bit-exact 是 L2 假设 |
| 声部树 | **❌ 无**（单元测试）| 无 parity，仅 rust 内部单元测试 |
| 账本 R=Π-A-W + TW | ✓ **完整** | ledger delta + transition bit-exact |
| 风险投影 J_Θ / sizing | **❌ 无** | strategy 实装但无 Lean 对标 |
| 动作 order/exec | **❌ 无** | fill 模拟 L1，无 Lean 侧 parity |
| hybrid_step / conformance | ✓（U5）| RustEngineContract step/classify 确定唯一 |

**parity 集中在**：第三类买卖点、账本、闭环 conformance。**parser 全层 + 区间套 + 声部 + 风险投影 + 执行
完全无 parity**——这些是「rust 实装了但未 bit-exact 对照 Lean」的空白区。

---

## 3. L2 回测有效域核（rust 实装跑的是 canonical 完整策略还是子集）

引用 `theta-v0-l2-results-v0.md`（8 品种 OOS 真实数据，修复 source_index bug 后干净 L2）：

| 证据 | 数值 | 含义 |
|------|------|------|
| decisions（决策点）| 百万级（如 BTC 9.3M）| 第三类买卖点提取大量产出 |
| ord（执行订单）| **1~25** | 实际成交订单极少 |
| **trades（完整开平配对）**| **全 8 品种 = 0** | **零完整交易**——`metrics.n_trades`，`trade_pnls` 闭合配对计数 |
| strat% | 混合（+327% ~ -12%）| mark-to-market **持仓浮动**，非实际成交盈亏 |

**有效域结论（区分定义域 vs 有效域）**：

1. **rust 实装跑的是「单声部 L0 第三类」**，**不是 canonical 完整策略**。证据链：
   - 单声部：`recognize` 实际产 depth=0 单声部（`mod.rs:410-413` 诚实声明：classifier 上级 bsp 留空 ⟹ L* = L0）。
   - L0：只在最低级别产买卖点（上级递归层无方向单元，第三类回试方向判据无法 bit-exact）。
   - 第三类：`signal.rs:9-26` 只产第三类，第一/二类谓词备而未产。

2. **trades=0 的结构根因**（非工程 bug，是实装子集的必然后果）：
   - recog 单帧只产**开仓侧**决策（`mod.rs:404-406`），平仓/止损由账户层后续驱动。
   - 主动卖点 `sell_transition` **游离 hybrid_step 主流程**（`closed_loop/sell.rs` 工位标 MISSING）。
   - ⟹ 开仓订单零星产生但无配对平仓 ⟹ 无完整 trade ⟹ trades=0，strat% 仅为持仓浮动。

3. **L2 否定性观察**（231 价值）：所有品种 strat% < bh%（未跑赢买入持有）。但因 trades=0，**此 strat% 尚不能
   作为「Θ 策略经验无效」的强证据**——它度量的是稀疏开仓的持仓浮动，非闭环交易系统的真实表现。真正的
   经验否证需先补全平仓闭环（E5）。

---

## 4. 总账：rust 实装覆盖 canonical 的比例 + 缺口清单

### 4.1 覆盖比例（按 canonical 机制条目，46 条核）

| 状态 | 条数 | 占比 | 条目 |
|------|------|------|------|
| **完整** | 18 | ~39% | A1-A4, B1-B3, B5-B6, C1, C4-C5, D6, E2-E4 |
| **部分** | 18 | ~39% | A5-A7, B4, B7, C2-C3, C6-C7, D1, D3-D4, D7, E1, E5, F1-F2 |
| **桩** | 0 | 0% | （无纯桩——代码诚实，缺口标为「部分/缺」非桩）|
| **缺** | 3 | ~7% | D2（杠杆 G_t/N_t）, D5（J_Θ LexArgmin）, + B 区无 parity 的隐性缺 |
| **诚实非形式化（设计边界）** | 7 | ~15% | A5 古怪线段动态划分、区间套等 Origin 有意不形式化部分 |

**核心比例判断**：
- **结构骨架覆盖 ~78%**（完整+部分），但「完整」仅占 ~39%。
- **rust 实装 ≠ canonical 完整策略**：实装的是 canonical 的**单声部 L0 第三类子集** + 完整账本/闭环框架。
- **bit-exact parity 覆盖远低于实装覆盖**：36 个 parity 集中在第三类/账本/conformance，parser 全层 + 区间套 +
  声部 + 风险投影 + 执行 + 杠杆 **无 parity** ⟹ 大量「实装了但未对照 Lean」的代码。

### 4.2 缺口清单（哪些 canonical 机制 rust 未实装/部分/无对照）

**硬缺（canonical 有，rust 完全无）**：
- **D2 杠杆 G_t/N_t/L^G/L^N**：总名义/净名义头寸、总/净杠杆计算无任何对应函数。
- **D5 风险投影 J_Θ**：加权目标 LexArgmin 未实装，`size_position` 是硬编码三路 min。

**部分（rust 有压缩/投影，未达 canonical 完整）**：
- **C3 根声部状态机**：RootSel_Θ(1,1) 镜像消歧 + 「先平根仓下周期反向」递归无对应，压缩为 (risk,phase) 8 态。
- **C6 资本五态压三态**：III_repair/protected/accretive 三子态压为 PhaseIII，丢失增核心单位决策粒度。
- **D4 动作 A1-A10**：A4/A6/A7/A9 无独立分支，10 级压缩为 8 态映射。
- **D7 外部事件**：仅处理价格/笔事件，Fill/Reject/Fee/Funding/Margin/Borrow/CorpAction 七元组未处理。
- **E1 完整状态**：17 分量覆盖 ~50%（h_t/σ_r/E_t/Cash_t/完整 O_t/M_t 缺或仅摘要）。
- **B4 第一/二类买卖点**：谓词存在但 signal 主流不产出（次级别层有结构实装但未接入）。

**实装了但无 bit-exact parity 对照（空白区）**：
- parser 全层（分型/笔/线段/包含）、区间套（nest/descend）、Sel_Θ 选择器、声部树、背驰力度、风险 sizing、执行 exec。

**运行形态缺口（导致 L2 trades=0）**：
- **E5 平仓闭环**：sell_transition 游离 hybrid_step，recog 只产开仓侧，无完整开平配对。

---

## 5. 结果包六要素

1. **结论**：rust theta_v0 实装 canonical「完全分类+全定义策略」的**单声部 L0 第三类子集 + 完整账本/闭环框架**，
   而非 canonical 完整策略。46 条机制核：完整 ~39% / 部分 ~39% / 缺 ~7% / 设计边界 ~15%。36 个 parity（非 33）
   集中在第三类/账本/conformance，parser 全层 + 区间套 + 声部 + 风险投影 + 执行 + 杠杆**无 bit-exact 对照 Lean**。

2. **定义依据**：canonical S_Θ §1-17 / FULL_USER_FORMULA §1-22 逐节核 rust `theta_v0/` 各模块 pub fn；rust 契约
   锚 `formal/Origin/*.lean`（reference-theta-v0.md 冻结 Θ v0）；parity 桥 `tests/theta_v0_*_parity.rs` 实测 36 test。

3. **边界条件**（结论翻转条件）：
   - 若 classifier 上级填充 bsp（多级别 + 第一/二类接入），则 recognize 自然产多声部，「单声部 L0 第三类」翻转。
   - 若 sell_transition 接入 hybrid_step 主流程 + recog 产平仓侧决策，则 trades=0 翻转，strat% 成为有效经验证据。
   - 若 D2/D5 实装（杠杆 + J_Θ），则「缺」条目下降，覆盖比例上升。

4. **下游推论**：
   - 本轮 L2 strat% 度量稀疏开仓持仓浮动，**不是闭环交易系统经验表现**——经验否证（231 价值）需先补 E5 平仓闭环。
   - canonical 完整策略的有效域检验（多声部、杠杆约束、J_Θ 优化）当前**不可能**，因实装是子集（有效域 < 定义域）。
   - parser 全层无 parity ⟹ rust 解析层与 Lean spec 的 bit-exact 一致性**未验证**，是潜在漂移风险点。

5. **谱系引用**：
   - **L2引擎不完整vs Θ否证**（记忆 `l2-engine-incompleteness-vs-theta-falsification`）：本核确证 trades=0 属「引擎产
     开仓不产平仓配对」（工程层 E5 缺口）而非「Θ 产信号不盈利」（经验否证）——分层诊断成立。
   - **第三类边界 reference↔Lean 矛盾**（记忆 `theta-v0-type3-boundary-reference-lean-conflict`）：本核确认 rust 已对齐
     Lean 严格 `<`（codex 2026-06-27 裁决，`classifier_parity.rs:308-334`），矛盾已结算为 Lean 口径。
   - 与 `docs/canonical-coverage-classification.md`（canonical→Lean）互补：本文档核 canonical→rust 第一跳。
   - 形式化有效域规则（231号）：rust 实装定义域（代数可施加全 canonical）≠ 有效域（实际跑单声部 L0 第三类）。

6. **影响声明**：本产出新增 `docs/canonical-coverage-rust-impl.md`（只读审计，未改任何 rust/Lean 代码）。
   不改变任何定义/实装。澄清编排者质询：rust 实装**不是** canonical 完整策略，是其子集，且 parity 对照覆盖
   远低于实装覆盖。影响范围：为下游提供 rust 实装缺口清单（D2/D5 硬缺、C3/C6/D4/D7/E1 部分、E5 平仓闭环、
   parser/区间套/声部/风险/执行 无 parity）。
