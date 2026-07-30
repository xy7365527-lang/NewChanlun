# 实现索引：formal-chain 数学 → Lean → Rust/Python 既有实现 → 状态

- 日期：2026-07-23；任务：issue #192；性质：只读考证合成（未修改任何代码文件，无 git 操作）。
- 数据来源（在其上合成，不重复考古）：
  - `.chanlun/review-results/buw-parts/` 六块 orphan 档案（trading 15 / spiral 4 / theta-v0 17 / formal-lean 45 / analysis-py 10 / process 9）
  - `.chanlun/review-results/accounting-layer-math-design-20260723.md`（下称【数学原设计】）
  - `.chanlun/review-results/formal-chain-recursion-accounting-20260723.md`（下称【formal-chain 全卷】）
  - `.chanlun/review-results/accounting-layer-impl-history-20260723.md`（下称【#187 报告】）
  - 补查：`docs/formal-chain/INDEX.md`、`formal/` 目录树、`rust/src` 目录树与少量一手 grep（均标注「本次一手」）。
- 从六块档案直接继承的状态不重新核实；本索引新补的条目均一手核实；不确定标「未验证」。

## 出票查重用法（出票者必读）

1. **出票先查本索引**：任何新「建成」票（新建账本/解释器/证书/引擎/账本机制）在切票前，先在本索引按能力节查目标能力。
2. **命中「已存在」**（任何一行 Rust/Python 实现状态 ≠ 「无」）：票**必须改为接线/采用**，并在票面引用本索引的对应能力节编号；禁止再写「新建 XX」。
3. **未命中**：才允许写「新建」，并须在票面引用本索引路径声明「已查重，未命中」。
4. **命中「缺口清单」**：说明该能力已知缺什么（数学有/实现无，或反之）——票应写成「补缺口」而非「新建」，并引用缺口条目号。
5. 本索引不含处置建议（采用建议栏留空）：接不接、怎么接是裁决（#174/#179 线），不是本索引的活。

## 状态词表

- **生产**：被 `run_theta_v0_pi` 生产 π loop 或其直接调用链消费。
- **仅测试**：全部消费者在 `#[cfg(test)]` 内，生产零调用。
- **死码**：除自身测试外全仓零引用。
- **deprecated**：入口带 `#[deprecated]` 标记（多为 F-01 结构确认前视隔离）。
- **仅旁路**：只读 overlay / bit-exact 对照基线 / 一次性 harness，不改生产行为。
- **否证域**：实验臂经 L2/L3 判决否证退役（设计内生命周期），代码可跑但不在任何生产候选上。
- **Lean 有锚**：rust 侧存在该模块的契约锚/镜像/parity fixture（锚定强度 L3 机器耦合 / L2 镜像文件 / L1 注释锚，见 formal-lean 档案 §3）。
- **Lean 孤岛**：形式化完、CI 保绿，但无 Rust 锚且无 Lean 下游消费。

---

## 能力 1：分账本 / 双腿不净额（P^sep，禁止只记净额）

| 字段 | 内容 |
|---|---|
| formal-chain 数学出处 | 推导完全互斥分类.pdf p.4–5/19 §5–6（`P_sep=∏_v(R≥0 e_v⁺⊕R≥0 e_v⁻)`，双开不相消，Net 另行定义）；买卖点.pdf p.4/15 §6；递归完全分类买卖点.pdf p.8/20 §10；多空对冲.pdf p.1–2/16 定理 1（净额 NAV 不可识别，ker N 非平凡）；完整的策略.pdf p.1/11 §1（四账本必须分离）；ADR 0001:44-45（分腿记录硬约束） |
| Lean 模块 | `formal/Origin/SeparateLedger.lean`（C25/C26，**有锚** rust separate.rs:7-10）；`formal/Origin/VoiceEat.lean`（C29，**有锚**）；`formal/Origin/NetValueImpossibility.lean`、`SeparateEat.lean`、`SeparateCoverTheorem.lean`、`SeparateCoverRecursive.lean`、`SeparateFinalTheorem.lean`、`SeparateStrategyWellDefined.lean`、`SeparateStrategyTarget.lean`（支撑层，coverage.rs:2 端口声明点名）；`formal/Origin/VoiceCoverTheorem.lean`（**孤岛**，781 行，同批 VoiceEat 被锚它无人锚） |
| Rust 实现 | ① `rust/src/theta_v0/ledger/separate.rs:73,122,139,192`（Leg/SepPosition/net/eat_sep）——**零生产消费者**（死码级，spec R2 行交付，`89c23de3ac`）；② `rust/src/theta_v0/strategy/ledger.rs:821` SplitLegLedger（父腿+短差腿两独立坐标，事件面仅 OpenShortDiff/CloseShortDiff :832-835，`nest()` :932-938）——**仅测试**（#149 `fcd18cb389`、#151 `744da5ff81`）；③ `rust/src/theta_v0/backtest/dual_ledger.rs:42,130` DualLedger/apply_fill_dual（p120 方案 B，入口 `run_theta_v0_dual` runner.rs:372）——**deprecated**（`640609071d`，F-01 前视）；④ `rust/src/theta_v0/strategy/overlay_state.rs:102,170` OverlayState（hedge-mode 逐声部 P^sep 簿，bin/theta_overlay.rs:64 驱动）——**仅旁路**（bit-exact 承诺 runner.rs:1546-1550）；⑤ `rust/src/trading/ledger.rs:370,469` ShortBook/DirectionalBook（双向会计首实装，`89da391d1d`）——**死码**（全 git 史零消费者）；⑥ `rust/src/theta_v0/strategy/oscillation.rs:525-531` OscillationBook（母子腿分表，pan_div.rs:51-55「唯一子腿账本」）——**生产已接线但出口净额**（runner.rs:1076,1410,1427,1491-1544，子腿投影叠回净目标后走同一 schedule_order 净订单） |
| Python 实现 | 无独立实装；harness 经 `nr.run_positional_rust` / PyO3 驱动 Rust 侧 |
| 谱系引用 | issue #149/#151（T5/T6，CLOSED）、#68（C1 双仓接线，部分）、#105/#106/#111（p120 遗留）、#174（OPEN，接线裁决扣留）、#176（接线代价备忘 CLOSED）、#152 终验A 条目3（判「偏离」）；谱系 627（两引擎线登记）；p120 施工图 `chanlun/review-results/p120-nested-voice-dual-ledger-design-20260718.md`；设计正本 `analysis/bidirectional_nested_accounting.md` |
| 采用建议 | （留空——处置是裁决） |

**横切警示**：P^sep/M14 同概念被独立实装三次（separate.rs 06-28 / OverlayState 07-04 / DualLedger 07-19）互不引用，且均未回溯 ShortBook（06-12）——本仓「找不到前人成果而重造」的头号标本（作者知悉度未验证）。

## 能力 2：通道解释器 / 消歧次序表（P1–P8 / P1–P10 / ≺_Θ 全互斥）

| 字段 | 内容 |
|---|---|
| formal-chain 数学出处 | 买卖点.pdf p.6–8/15 §8–9（≺_Θ 全序 + `Σ1[C_j]=1` 互斥化定理）；完整的策略.pdf p.4–5/11 §7（P1..P10+P0，P2–P4 为 TW 会计谓词）、p.9/11 §14；买卖点alpha2.pdf p.17/38 §5（P1..P8）；缠论的全互斥定义策略.pdf p.18–20/27（P1..P8 + 假设 4 固定优先级是全定义载荷）；子声部.pdf p.22/27 §15（P1..P7 变体）；递归完全分类买卖点.pdf p.8–9/20 §11–12（解释器自相似、≺_Θ 平移不变）；ADR 0001:47-54。**对立面**：`docs/necessity_derivation.md:517-530` T₂₁（去全局互斥是定理）——两份原始数学打架，见缺口 G10 |
| Lean 模块 | `formal/Origin/MutexFinalTheorem.lean` + `MutexElement.lean`/`MutexExhaustive.lean`/`MutexRecursive.lean`（**有锚**，coverage.rs:1-4 端口声明，M29 兑现）；`formal/Origin/CausalEndpointImpossibility.lean`（**孤岛**，两不可能定理之一，goal 验收止于 lake build 零 sorry）；`formal/Origin/LexArgmin.lean`（支撑层） |
| Rust 实现 | ① `rust/src/theta_v0/strategy/mutex.rs`（671 行，P1..P10 全互斥 + μ(z,a) 估计器，`ff88d81bc9`）——**仅测试**（channel.rs:52-53 自声明 D1 测试 oracle；例外：`MutexClass` 枚举被 oscillation.rs:16,467-471 借作标签映射）；② `rust/src/theta_v0/strategy/channel.rs`（1529 行，P1–P8 全互斥通道解释器：`step_voice`:500、`run_voice`:513、`run_voice_ledgered`:523、`run_short_diff_replay`:650；P7 记录桶 VoiceLedger :309；P8 占位恒 false :28）——**仅测试**（#147 `1dba02e86c`/#149/#150 `bbd6b1e39e`；外部引用 coverage.rs:5166、exit.rs:369 均在 `#[cfg(test)]`）；③ 生产散装等价物：interp fold P1..P10——`coverage.rs:3030 pi_theta_step_traced` ← runner.rs:1453——**生产**（#152 终验A 条目1 判「行为近似、无结构对应」的结构偏离） |
| Python 实现 | 无；机器证明记录 `docs/formal-chain/proofs-full-strategy-20260703.md` §B（mutex.rs:296 2^10=1024 谓词穷举 Σ=1、:574 十二场景桶级对拍） |
| 谱系引用 | 母 spec #139（「落进生产 π loop」切票时丢失）；T 线 #147–#151（CLOSED，验收全测试级）；#152 终验A（判偏离）；#174（OPEN，「接线 vs 让步」扣留）；#124 真统一重分解（mutex.rs:4）；goal `g-strict-mutex-classification-strategy`（2026-06-29 被 SUPERSEDE） |
| 采用建议 | （留空） |

## 能力 3：子树清仓（AncOK，祖先闭合 ⇒ 子树剪除）

| 字段 | 内容 |
|---|---|
| formal-chain 数学出处 | 推导完全互斥分类.pdf p.7/19 §8（`A_{t+1}=AncOK[(A_t∖D_t)∪O_t]`）；递归完全分类买卖点.pdf p.10/20 §13；买卖点2.pdf p.3/14 §4；级别容器2.pdf p.3/12 §4（「是约束，不是生成规则」）；完整的策略.pdf p.5/11 §8；ADR 0001 S3（:22-25） |
| Lean 模块 | `formal/Origin/AncestorClosure.lean`、`ActiveSet.lean`（支撑层，coverage.rs:2 端口声明 M16/M17）；`formal/Origin/AncestorLifespan.lean`（**孤岛**，task #24 gap-B 真证，commit message 明示「rust MR2 未做」） |
| Rust 实现 | ① `coverage.rs:828 ancestor_close_by_id`（:2290 消费，测试 `b36aa3975e` 2026-06-28 已入仓）——**生产**；② `exit.rs:184 cascade_exit_decisions` → nautilus/strategy.rs:170、runner.rs:3320——**生产**；③ `exit.rs:286 subtree_close` / `:326 step_active_set_with_subtree_close`（`ced46f4f62`，#148 T4）——**仅测试**（全部调用点在 exit.rs:366 起 `mod tests` 内；#152 条目4 判「对齐（有保留）」：不变量生产两条路径均兑现，T4 专用函数零生产调用） |
| Python 实现 | 无 |
| 谱系引用 | #148（CLOSED，验收全测试级）；**#179**（2026-07-23 编排者亲裁「接线进生产，结构对应是硬要求」）；#183（OPEN，实施票，硬约束「归一即删除散装等价物」）；#152 条目4；task #24 |
| 采用建议 | （留空） |

## 能力 4：级别容器与级别键（Cell 递归 / C_ℓ 统一递归单元）

| 字段 | 内容 |
|---|---|
| formal-chain 数学出处 | 级别容器.pdf p.3/17 §3、p.12–13/17 §12.2（`Cell_{σ,ℓ}=Local_{σ,ℓ}×∏_w(1+Cell_{−σ,ℓ_w})`，Cell 含持仓/风险状态）；级别容器2.pdf p.10/12 §12（无 depth 特权）、p.5–6/12 §7（祖先链定理）；递归完全分类买卖点.pdf p.1–2/20 §2（`C_ℓ=(S_ℓ,Γ_ℓ,A_ℓ,P_ℓ,E_ℓ)` 每级自带头寸腿与资本状态）；最高级别走势类型.pdf p.1–2/13 §1–2；级别和sigma.pdf（级别-σ 桶键，wverify 出处，【formal-chain 全卷】未读） |
| Lean 模块 | `formal/Origin/RecursiveLevelSystem.lean`（**有锚**，classifier 生产链）；`formal/Origin/CanonicalQuotientTower.lean`（**孤岛**，commit 自述「对接 Origin RecursiveLevelSystem」未兑现）；`formal/Origin/ParentDirContainer.lean`、`VoiceThreeLevel.lean`；`formal/Origin/TrendSixState.lean`（对照层 six_state.rs:1-8 锚） |
| Rust 实现 | ① `rust/src/theta_v0/classifier/recursive_tower.rs`、`level_view.rs`、`level_view_store.rs`——classifier 生产链支撑（本次一手 grep：被 nest.rs:30-31、turn_class.rs:55、cand_predicate.rs:57、level_view_store.rs:7 等消费）；② `rust/src/theta_v0/classifier/level_state.rs` + `six_state.rs`（走势六态 r + 位向量 b canonical 对照层，`42dd08d3d3`）——**零消费者**（six_state 被 level_state 内部消费，econ_positive.rs:830 仅注释提及，注释声称的上游角色与代码事实不符，未裁决）；③ `rust/src/theta_v0/complete/`（完整状态 x_t 17 分量 + 外部事件 e 8 元组 schema，`a28a079fd7`，锚 `Origin/CompleteStateEvent.lean`）——**零引用死码**（自述「不替换闭环引擎 AssemblyState/MicroEvent」） |
| Python 实现 | 无 |
| 谱系引用 | cov-rust-impl 报告 E1/D7 覆盖缺口（complete/ 出处票据）；637 迁主塔批次 `cefb29df69`；task #127 |
| 采用建议 | （留空） |

**缺口提示**：生产执行账**无 (account, level) 键**——`AccountState.voice_qty` 按 depth 非 level（混桶审计条目5，经【#187 报告】§6 转引），「一类点后 Core(level)=0」类账本断言当前不可表达。见缺口 G3。

## 能力 5：区间套证书 / 买卖点确认（N^δ 递归证书，买卖同源镜像）

| 字段 | 内容 |
|---|---|
| formal-chain 数学出处 | 递归完全分类买卖点.pdf p.4–5/20 §6（`N^δ_{ℓ↓e}`，基例 `Conf⁺_e=∨B_i` / `Conf⁻_e=∨S_i`，递归步区间包含 + 严格下降终止，自相似平移不变）；买卖点.pdf p.2/15 §2；区间套.pdf p.1,3,7/15（rung=区间包含非端点相等，soundness 定理 1/2）；回测的问题.pdf（7-02 晚同源再裁决）；完整的策略.pdf p.2/11 §3；一类买卖点.pdf p.5/11 §7（趋势块 + 破界 + 背驰 ⇒ B1/S1）+ p.1–2/11（趋势门须作用 TrendBlock 非 AllTrend）；`docs/formal-chain/有效域定理-20260704.md` 定理 3 |
| Lean 模块 | `formal/Origin/SellPointRecog.lean`、`SellClosedLoop.lean`、`CenterStates.lean`、`BspClassification.lean`、`ThetaInstantiation.lean`（**parity fixture 机器耦合**，`ParityFixtureExport.lean` #eval 导出 `rust/tests/fixtures/theta_v0_parity.json`——**但宿主 run_theta_v0/run_theta_v0_dual 已 deprecated**，见缺口 G6）；`formal/Origin/BspConstruction.lean`、`SegmentConstruction.lean`、`SegmentFeatureSeq.lean`（**有锚**，classifier 生产）；`formal/Origin/IntervalNestCertificate.lean`、`NestingCertificate.lean`；`formal/Origin/SecondSellClosedLoop.lean`（**孤岛**，二类卖闭环扩展无人接）；`formal/Origin/SegmentAutoConstruct.lean`（**孤岛**，A‴ fallback 良构性缺口未关） |
| Rust 实现 | ① `rust/src/theta_v0/classifier/nest.rs`（nest_confirmed 硬门，interp.rs:1313-1315 在 close/open 前拦截）——**生产**（#146 T2）；② `rust/src/theta_v0/closed_loop/*`（sell/transition/buy/state 函数级 bit-exact，4 个 parity 测试活着）——**宿主 deprecated**（`run_closed_loop` 仅被 run_theta_v0 runner.rs:301 与 run_theta_v0_dual runner.rs:381 调用，均 deprecated）；③ `rust/src/theta_v0/classifier/interval_necessity.rs`（区间套必要条件塔内原生实现，`6075840687` task #106 条款 9）——**零消费者**；④ `rust/src/theta_v0/classifier/first_retrace_replay.rs`（D7 firstRetrace 重开复核原语，`e942e1d3c1` #76 D7 例2）——**零消费者**；⑤ `rust/src/theta_v0/classifier/force_conformance.rs`（ForceMeasure MACD 实例化，`4318b4efad` task #127）——**零消费者**；⑥ 旧 ladder：`rust/src/bi_zhongshu_bsp.rs`、`buysellpoint.rs`、`divergence.rs`——被 recursive_t/fugue_v3/segment_layers 消费（本次一手 grep），theta_v0 明文不复用（theta_v0/mod.rs:13-17） |
| Python 实现 | `analysis/segment_refsem_cert.py:53,57`（Parse.lean 口径注释，文档性引用非消费）；旧 Python 引擎谱系（analysis/ 各 backtest，多为 Rust harness） |
| 谱系引用 | #33（证书漏斗归因插桩）、#76（D7）、#106（条款 9）、#127（force-conformance）、#146（T2 已接生产）；637 迁主塔 A→B 批次 `cefb29df69`；631 parity fixture 兑现（acceptance #4）；`cert-bsp-binding-ruling-DRAFT-20260717.md` |
| 采用建议 | （留空） |

## 能力 6：顶点与分型（买卖点语法 / 走势分类 / 中枢构造）

| 字段 | 内容 |
|---|---|
| formal-chain 数学出处 | 买卖点.pdf p.1/15 §1（6-bit 信号向量 `b_ℓ∈{0,1}⁶`，64 类穷尽不强行互斥）；递归完全分类买卖点.pdf p.4/20 §5（级别间同一归一化语法）；一类买卖点.pdf（全 11 页）；关于背驰.pdf（INDEX 登记，【formal-chain 全卷】未读——背驰专项出处未验证）；最高级别走势类型.pdf（全 13 页） |
| Lean 模块 | `formal/Origin/ChanlunElements.lean`、`TrendCompleteClassification.lean`、`CenterComplete.lean`、`CenterConstruct.lean`、`CenterConstruction.lean`、`CenterFull.lean`、`Divergence.lean`、`PromQual.lean`（**有锚**，637 迁主塔后 classifier 生产链）；`formal/Origin/BspClassification.lean`、`CenterStates.lean`（parity fixture，宿主 deprecated）；`formal/Origin/RootSelDisambig.lean`（**孤岛**，637+L3 声部根状态机层，同批主塔被锚这层无人接）；`formal/Origin/CenterAutoAssign.lean`（**孤岛**，仅被孤岛 Pipeline import）；`formal/Formal/DivergenceNesting.lean`（**孤岛**但为 Claim7 单一权威源——防重造正面案例）；`formal/Phase2/Claim7_DivergenceNesting.lean`（**最接近死码**：与 Formal 侧字节相同撞命名空间被排除出 roots，CI 不 build） |
| Rust 实现 | ① `rust/src/theta_v0/classifier/*`（bsp/center/decompose/descend/divergence/signal/turn_class/rmove_compose/cand_predicate 等）——**生产**（IncrementalClassifier 线，637 迁主塔 `cefb29df69` 等）；② `rust/src/theta_v0/parser/*`——**生产**；③ 旧 ladder：`rust/src/fractal.rs`、`stroke.rs`、`segment.rs`、`zhongshu.rs`、`moves.rs`、`bi_engine.rs`、`ph.rs`、`segment_layers.rs`——被 recursive_t/fugue_v3/spiral/signal 消费（本次一手 grep），**旧 Python 等价 ladder，theta_v0 不复用**（theta_v0/mod.rs:13-17 明文）；④ `rust/src/theta_v0/classifier/voice_eat.rs`（C06 声部吃到 Eat(v,b)，`89c23de3ac` WIP 扫入）——**零消费者** |
| Python 实现 | 旧 Python 引擎谱系（`analysis/` 顶点/段/中枢实现，2026-06-10 起被 Rust 移植旁路降格为 bit-exact 对照基线，见 analysis-py 档案组 1） |
| 谱系引用 | 637（迁主塔）；task #127（A′ 收口轮 rust 全重锚 Origin）；task #57/#62（Strict 工位，孤岛）；A′ 裁定 task #96/#97（legacy 降级）；谱系 528/529（K4 顶点映射非正典裁定——1min/K4 族隔离依据，原文未读，转引自 k4_1min_lib.py:5-24） |
| 采用建议 | （留空） |

## 能力 7：会计恒等式与守恒（T₃₃–T₄₆，一股不多一股不少 / NAV 中性 / 零强平零破产）

| 字段 | 内容 |
|---|---|
| formal-chain 数学出处 | `docs/necessity_derivation.md:701-979` 第 3 部分「会计层定理（操作原子 → 守恒律）」（D3.1 双重记账语义 :706-708、D3.2 NAV :710-711、T₃₃–T₄₆ :713-979）；完整的策略.pdf p.6/11 §10（`R=Π−A−W`、`TW=free+holding+withdrawn` 双恒等式）；`docs/formal/full-definition-strategy-v1.md:56,66`（R=Π−A−W 立为合法态不变量）；14 定理与 Lean file:line 全对照表见【数学原设计】问题 6 |
| Lean 模块 | `formal/Tlayers/Accounting.lean`（root）+ `Accounting/Ledger.lean`（T₃₃/₃₄/₃₅/₃₈）+ `Accounting/Earning.lean`（T₃₆/₃₇）+ `Accounting/Forest.lean`（T₃₉/₄₀/₄₂/₄₄）+ `Accounting/Solvency.lean`（T₄₁/₄₃/₄₅/₄₆）——14/14 定理进 Lean E-set、lake build PASS、零 sorry/admit/axiom（`formal/Tlayers/ACCOUNTING_STATUS.md:6-25`）；**孤岛**（A′ 裁定 legacy，rust 侧四种锚定形式 grep 零命中；与 spiral/accounting.rs 概念同域但无镜像关系）。Origin 侧重锚：`Origin/TotalWealth.lean`（有锚，见能力 8）、`Origin/FullDefinitionStrategy.lean` LedgerState（有锚 LedgerComp） |
| Rust 实现 | ① `rust/src/theta_v0/strategy/ledger.rs:146-611` TwState/tw_step——**生产**（见能力 8）；② `rust/src/theta_v0/strategy/ledger.rs:613-706` LedgerComp（R=Π−A−W，锚 Origin.FullDefinitionStrategy.LedgerState :9-13）——closed_loop/transition 层消费，**宿主 deprecated**；③ `rust/src/trading/nested_fugue.rs`（pop_tail/unwind_to/守恒律 §1-§8 + Voice 双层记账）——**否证域**，原语被 isolated_fugue/nested_interval_fugue/positioning_chain_fugue/recursive_nested_fugue/unified_recursive 五模块逐字复用；④ `rust/src/trading/isolated_fugue.rs:157,212,256`（nav/settle/close_voice）→ `unified_necessity.rs:191` 复用——有消费者但**不在 theta_v0 生产线**；⑤ `rust/src/spiral/accounting.rs:33,54,98,199,212,296`（bit-exact 复用 unn 会计原语 R5/R6）——**仅 PyO3 出口**；⑥ 运行时数值守恒守卫 N8/A4/N1（25M bar 零违反，L2，unn 线 + theta_v0 runner 侧）——unn 线不在生产，theta_v0 侧生产（行号未逐行核对，沿【数学原设计】未验证声明） |
| Python 实现 | `analysis/organic_fugue.py:316` OrganicLedger——**仅旁路**（bit-exact 对照基线）；`analysis/fugue_version_i.py:738` _SharedFugue——**仅旁路**（常量面被 99 处 import 但引擎本体仅基线消费） |
| 谱系引用 | task #49（t-accounting，编排者裁决 T₃₃–T₄₆ 全进 E-set）；task #96/#97（A′ 降级）；`4318b4efad`（删 Tlayers TotalWealth.lean，「Origin.TotalWealth 已 native 接管」）；谱系 538（NRF 会计双重性 + 资金守恒解锁，编排者裁决结算）、534、539；ACCOUNTING_STATUS.md:30-47（首版 vacuous 守恒被 codex 6/6 FAIL 删除重写的失败教训） |
| 采用建议 | （留空） |

## 能力 8：财富三阶段账 TW（free + holding + withdrawn，CostReduction→CapitalRecovered→EarningShares）

| 字段 | 内容 |
|---|---|
| formal-chain 数学出处 | 完整的策略.pdf p.6/11 §10（`TW_t=free+holding+withdrawn`、TStage 三态、五事件桥 {ShortDiff, RecoverCapital, Withdraw, EnterEarning, BuyCore}、Ready/EnterReady 最严格触发）；目前的缺口.pdf p.7–10/21 Q3–Q4（裁定选 C：R 与 TW 双层并置 S_C=S_R×S_TW；η 递推、barrier `η⋆=L^wc+κQ`，κ 为政策参数）；绝对资本.pdf p.1–4/12（三选二不相容定理 + 协变资本单位 U_ℓ 修复方案 A）；`docs/three_stages_accounting_design.md`（cost_basis 与 basis 物理分离）；`docs/three_phase_unified_design.md:45-52`（单一全局入口 `TRoot::account_campaign`）；`docs/recursive_t_architecture_v2.md:432-440` §8.7（**单一根现金池裁决 2026-06-20**：现金 fungible 非级别专属，仓位隔离但共享根 free） |
| Lean 模块 | `formal/Origin/TotalWealth.lean`——**有锚**（TwState 镜像，task #127 native 接管）；`formal/Origin/LedgerBridge.lean`——**孤岛**（task #93/#101 双账本桥，task #127 后降为 compatibility 层，文件头自述「不定义任何 TW 结构」）；`formal/Origin/CovariantCapital.lean`（对应绝对资本.pdf 协变修复）；`formal/Origin/LeverageCapital.lean`（支撑层）；`formal/Tlayers/Accounting/TotalWealth.lean`——**已删除**（`4318b4efad`，-721 行） |
| Rust 实现 | ① `rust/src/theta_v0/strategy/ledger.rs:146-611` TwState/TwEvent/tw_step（锚 Origin.TotalWealth :15-33）——**生产已接线**（#124 裁定 4：runner.rs:52 import、:1118-1132 初始化、:1230/1235 ShortDiff、:1253 Realize、:1674-1811 OpenShareLeg/CloseShareLeg、:1816-1821 消费 step_trace.tw_event）；② `rust/src/theta_v0/backtest/treasury.rs`（S1 Treasury 薄适配层，`bf1854d719` task #9）——**零消费者**（头部自述「μ 层转发来的已兑现摆动」——μ 层从未转发）；③ DualLedger 内 TW 账（#68 接线）——deprecated 路径内；④ recursive_t 线 T 树会计（`rust/src/recursive_t/`，§8.7 裁决的实装谱系）——旧 ladder，theta_v0 不复用 |
| Python 实现 | 无独立实装 |
| 谱系引用 | #124 裁定 4（TW 接生产）；task #90（不同构裁定后双账本并置）、#93/#101（桥）、#127（native 接管）；#68（dual 内 TW）；task #9（S1 treasury）；2026-06-20 根账本裁决（已结算） |
| 采用建议 | （留空） |

## 能力 9：做空 / 反向腿（ε=−1 空头腿坐标，Side×Role 两轴分离）

| 字段 | 内容 |
|---|---|
| formal-chain 数学出处 | 推导完全互斥分类.pdf p.4–5/19 §3–6（Side×Role 分离、ε_e=−1 ⇒ 空头腿、双开不相消）；多空对冲.pdf p.3/16 §3（双开抵消定理：父仓保持+同单位反向双开 ⟹ PnL=0、扣成本 <0）、p.6–8/16（毛捕获 ≠ NAV alpha）；`docs/necessity_derivation.md:578-590` T₂₅（方向交替递归）、:751-771 T₃₆（earning 多空构造不对称：多头可构造、空头 L0 不可构造，凸性载体是期权 DTE）；买卖点alpha2.pdf p.15–17/38 §4.1–4.3（多空双开语义镜像）；`.chanlun/tower-construction/tower_mirror_duality.md:26-33`（τ 价格反射 Z₂ 协变，独立空腿逻辑 = 违反孤儿不可能定理） |
| Lean 模块 | `formal/Tlayers/Accounting/Earning.lean`（T₃₆ `earning_asymmetry` :68、`short_earning_no_construction` ¬∃ :93、T₃₇ 关闭结算四分裂 :109-166）——**孤岛**；`formal/Origin/VoiceEat.lean`（C29，有锚）；`formal/Origin/SubVoiceOpenClose.lean` |
| Rust 实现 | ① `rust/src/trading/ledger.rs:370,469` ShortBook/DirectionalBook（双向会计首实装 `89da391d1d`，设计正本 `analysis/bidirectional_nested_accounting.md` §2.2）——**死码**（全 git 史零消费者；同 commit 引擎臂选了 LayerState::Short 载体）；② `rust/src/trading/positional_fusion.rs:238,1247,1399,1508` LayerState::Short——**否证域**（fusion 臂族）；③ `rust/src/trading/dual_voice.rs`（D1 双书 0/8 全否证，「字面做空死域第四次复现」）——**否证域**；④ `rust/src/trading/recursive_nested_fugue.rs`（「平多≠开空」0/8）——**否证域**；⑤ theta_v0 生产：`runner.rs:1054-1055` 有符号 units（负=空）——**生产但无腿级身份**（净额标量）；⑥ `rust/src/theta_v0/ledger/separate.rs` 空头腿坐标 e_v⁻——**零消费者**；⑦ `rust/src/trading/unified_necessity.rs` T14 根翻空接入（`b5355deb08`）——不在 theta_v0 生产线 |
| Python 实现 | `analysis/` 各否证 harness（dual_voice_backtest.py、unified_recursive_system_backtest.py 等，一次性）；`trading_system/` NT 链路做空支持未验证 |
| 谱系引用 | 谱系 534（NRF 降成本 vs hold26 regime 分离）、538、539（清仓判据 v5 否证）、627（两引擎线有效域分离）；`analysis/dual_voice_results.md`（0/8 + N1 5/8）；谱系 541-544（spiral 群论线）；#187 报告 §1.2 谱系 A/B |
| 采用建议 | （留空） |

**横切警示**：本线双向/做空引擎形态**全部否证**（dual_voice 0/8、rnf 0/8、fusion 系字面做空死域四次复现）——出「做空引擎」票前必须先读这批否证判决，不是空白地。

## 能力 10：短差机制（ShortDiff：父仓保持 + 次级别反向双开，显式独立机制）

| 字段 | 内容 |
|---|---|
| formal-chain 数学出处 | 递归完全分类买卖点.pdf p.7–8/20 §9（`ShortDiff(g)⟺σ_p(g)≠0∧δ_g=−σ_p(g)`，去根化定义）；买卖点.pdf p.3–4/15 §4–5（五角色互斥）；推导完全互斥分类.pdf p.6–7/19 §7.4（父仓 `(+Q,0)→(+Q,−Q)`；同级别反向 = 反手不是短差 §7.2/§17）；缠论的全互斥定义策略.pdf p.18/27（ShortDiffEntry/Exit 谓词）；买卖点alpha2.pdf p.15–17/38 §4.3；完整的策略.pdf p.5/11 §8（`σ_v=−σ_p(v)`）；ADR 0001 S4（次级别反父证书只走 P4 通道，:27-29）、S5（删除「净和视角短差自动涌现」——被多空对冲定理 1 证伪，:31-35）、S6（:37-40） |
| Lean 模块 | `formal/Origin/VoiceTree.lean`（有锚）；`formal/Origin/SubVoiceOpenClose.lean`、`VoiceThreeLevel.lean`、`SeparateEat.lean`（支撑层）；`formal/Origin/OperationRole.lean`、`OperationRole18.lean`（角色分类，锚定/支撑） |
| Rust 实现 | ① `rust/src/theta_v0/strategy/channel.rs` P4/P5 短差通道（`fcd18cb389` #149 T5）——**仅测试**；② `rust/src/theta_v0/strategy/ledger.rs:821` SplitLegLedger（短差腿独立坐标 + quota + `nest()`）——**仅测试**；③ 共用类型 `rust/src/theta_v0/strategy/voice.rs:48-58 short_diff_side`（channel.rs:65、ledger.rs:74 消费）；④ `rust/src/theta_v0/backtest/pan_div.rs:51-55` PanDivProductionState + `oscillation.rs` OscillationBook——**生产**（但出口合并净订单，runner.rs:1491-1544；混桶审计：「语义账户仍混桶」）；⑤ `rust/src/trading/nested_fugue.rs` pop_tail 逐层解栈/cost_pool/§8.1 逐 bar 守恒——**否证域**（nrf v4 严格会计 P1 3/8 历史最佳，`5e761a5ad5`）；⑥ `rust/src/spiral/accounting.rs:196-291` sigma_invariant_quota/try_spawn_cost_gated（f=1/λ 级别无关配额 spawn）——**仅 PyO3 出口** |
| Python 实现 | `analysis/nrf_v4_strict_accounting_backtest.py`（harness，`5e761a5ad5`，无消费者）；trading ShortBook 设计正本 `analysis/bidirectional_nested_accounting.md`（短差=父账本内降成本腿） |
| 谱系引用 | #149（T5）、#135/#151（T6 毛暴露）；谱系 534（NRF 降成本语义 trend regime 失效裁定）、538；#174/#176（接线裁决扣留）；混桶审计（exit-account-identity-audit-20260723，两类错账） |
| 采用建议 | （留空） |

## 能力 11：R 交易利润账（R_t = Π_t − A_t − W_t，与 TW 双层并置）

| 字段 | 内容 |
|---|---|
| formal-chain 数学出处 | 完整的策略.pdf p.6/11 §10（双账本会计恒等式）；目前的缺口.pdf p.7/21 Q3（裁定选 C：R 与 TW 双层并置，强行合并丢信息）；`docs/formal/full-definition-strategy-v1.md:56,66`（合法态不变量，来源标注 Tlayers/Accounting/Ledger） |
| Lean 模块 | `formal/Origin/FullDefinitionStrategy.lean`（LedgerState，**有锚** LedgerComp:9-13）；Tlayers 会计层 T₃₅ NAV 中性（Ledger.lean:170-209）——**孤岛** |
| Rust 实现 | `rust/src/theta_v0/strategy/ledger.rs:613-706` LedgerComp——closed_loop/transition 层消费（ledger.rs:45-48 自述），**宿主 deprecated**（F-01）；**生产 π loop 直接接的只有 TwState，R 账不在生产**（【#187 报告】§3） |
| Python 实现 | 无 |
| 谱系引用 | task #90（不同构裁定后双账本并置，ledger.rs:40-48）；#124；F-01（bughunt，2026-07-10，`d88190130e`） |
| 采用建议 | （留空） |

## 能力 12：毛暴露递归资金约束（quota ⊆ 父级额度，毛暴露 ≠ 净暴露）

| 字段 | 内容 |
|---|---|
| formal-chain 数学出处 | ADR 0001:44-45（「资金分配递归按毛暴露约束：子对冲额度 ⊆ 父级短差额度」）；递归完全分类买卖点.pdf p.11–12/20 §15（K_Θ 可行集含最大毛头寸、多空双开、TW 三阶段合法性）；完整的策略.pdf p.7–8/11 §11（K_Θ 约束清单）；绝对资本.pdf（资本协变）；`docs/necessity_derivation.md:444-483` T₁₈（配额 f=1/λ σ-不变，【数学原设计】标注未深入） |
| Lean 模块 | `formal/Origin/RiskProj.lean`、`CoverFeasibleTarget.lean`、`LeverageCapital.lean`、`CovariantCapital.lean`（锚定/支撑）；`formal/Strict/RiskProj.lean`（支撑层） |
| Rust 实现 | ① SplitLegLedger 毛暴露额度（`ledger.rs:836-848,858-879` typed 拒绝 GrossExposureExceeded）+ `nest()` 递归收缩（:929-940，测试 :1038-1074 验证 100→40→10→0）——**仅测试**；② #151 T6 单层 quota——**生产**（#152 audit :11：「单层 quota 接生产；多级 nest() 递归收缩仅在未接线账本」）；③ 生产净额单出口下的加法抵消补丁（runner.rs:1530-1544）——生产（【接线备忘】§4：换出口后才能退役） |
| Python 实现 | 无 |
| 谱系引用 | #151（T6，CLOSED）、#135；#176 备忘 §2-§3（多级 nest() 生产触发条件不存在：路由从不把 live lot 当 parent、子腿级 PanDivCert 信号源未见存在） |
| 采用建议 | （留空） |

## 能力 13（附）：净额主账 / 执行出口——当前生产实际形态（查重基线）

出票者需要知道「现在生产跑的是什么」，以免把「已存在的净额行为」当缺口、或把「教义模块」当已接线。

| 字段 | 内容 |
|---|---|
| formal-chain 数学出处 | 缠论的全互斥定义策略.pdf p.19–20/27（`N_{t+1}=Net(p_{t+1})` 净额是派生映射不是本体）；多空对冲.pdf p.1–2/16 定理 1（净额路线的数学证伪）；ADR 0001:42-45 |
| Lean 模块 | `Origin/MutexFinalTheorem`/`SeparateStrategyTarget` 等经 coverage.rs 锚定（M29）——但 coverage 五步表步 5「净额执行」把分账本塌缩成标量 units（【#187 报告】§3 混桶链） |
| Rust 实现（全部**生产**） | 主账：`runner.rs:1054-1056` cash/units/entry_cost 净额标量（units 有符号，正多负空）；fill 记账 `apply_order` runner.rs:3630；决策 `coverage::pi_theta_step_traced` runner.rs:1453 → `interp::interpret_with_close_triggers` coverage.rs:3030；`net_target_units` coverage.rs:1614-1619（短差空腿抵消父多腿）；单净订单出口 `schedule_order` coverage.rs:2692、runner.rs:1530-1534,1543；TW 并行入账（能力 8）；OscillationBook（能力 1⑥/10④）；typed ledger G4 归因层 runner.rs:1095-1100（只归因不改 fill）；OverlayState 仅 overlay 臂只读（runner.rs:1546-1550） |
| Python 实现 | `analysis/organic_signals.py`（生产信号层，:533-545 push_signal → UnnStream）；`trading_system/strategy/chanlun_strategy.py:39`（`trading_mode="fusion_tr"`，阶段 2 TODO 永远停在 TODO——PositionalStream 全 git 史从未作为代码存在）；signal_bridge.py:119（BSP 直读映射，生产至今跑骨架信号映射） |
| 谱系引用 | #124、#139、#152 终验A；trading_system/README.md:111,119（阶段 2 = PositionalStream，谱系断失未验证）；627（两引擎线） |
| 采用建议 | （留空） |

---

## 缺口清单（如实呈现，出票写成「补缺口」并引用条目号）

### A. 数学有、实现全无（或全部死在非生产线）

- **G1 每本账独立递归买卖机（信号轴）**：数学有——`C_ℓ=(S,Γ,A,P,E)` 每级自带账和钱（递归完全分类买卖点.pdf p.1–2/20 §2）、T₁₆/T₂₀/T₂₁（`docs/necessity_derivation.md:403-530`，自层 fire、去全局互斥是定理）。实现**结构性缺席**：unn/spiral 做到「多本账 + 递归开平仓」但信号是中央引擎每 bar 一份共享帧（`spiral/engine.rs:109`，【#187 报告】§4）；theta_v0 是「一个混合仓位池 + 全局 interp 仲裁」。无任何「账本实例自己订阅信号并自决开平仓」的机制。**边界**：现金轴不属此缺口——单一根 free 池是 2026-06-20 已结算裁决（`docs/recursive_t_architecture_v2.md:432-440` §8.7），「独立递归」不能扩展到现金独立池。
- **G2 R 交易利润账在生产缺席**：数学有（完整的策略.pdf §10，选 C 双层并置）、Lean 有锚、Rust 有 LedgerComp——但唯一消费挂在 deprecated 的 closed_loop 路径；生产 π loop 只接 TW 不接 R（【#187 报告】§3）。
- **G3 执行账 (account, level) 键缺席**：数学有（P_sep 腿坐标、posId=H(c,γ,σ,n)，子声部.pdf p.16/27 §5）；实现无——生产 fill 全落标量 units，`AccountState.voice_qty` 按 depth 非 level（混桶审计条目5）。后果：「一类点后 Core(level)=0」「FollowParent 错标 CloseShortDiff」两类错账不可被账本断言发现（【#187 报告】§6）。
- **G4 做空腿会计**：数学有（空头腿坐标、T₃₆ 不对称、τ 协变）、Lean 有（Earning.lean，孤岛）；Rust 四套（ShortBook/separate.rs/DualLedger/OscillationBook 空腿侧）全部死码/零消费者/deprecated/净额化；生产无腿级空账。
- **G5 多级毛暴露递归收缩**：数学有（ADR 0001:44-45 子对冲额度⊆父级短差额度）；实现只有单层 quota 接生产，多级 `nest()` 仅测试，且生产触发条件不存在（子腿级 PanDivCert 信号源未见存在，#176 备忘 §2）。
- **G6 Lean↔Rust parity 机器耦合宿主死亡**：5 个模块（ThetaInstantiation/SellPointRecog/SellClosedLoop/CenterStates/BspClassification）的 #eval→fixture→include_str! 机器耦合活着（测试仍跑），但耦合证明的 bit-exact 对象 run_theta_v0/run_theta_v0_dual 双双 deprecated（F-01）——「耦合活着，宿主死了」（formal-lean 档案 §5）。
- **G7 P7 记录桶数据面**：channel::VoiceLedger 仅测试，且生产 `parent_kappa` 无源（只能恒 Unknown，#175 备忘条目3）——即使接线也是降级记录。
- **G8 单根现金池唯一性未形式化**：§8.7 裁决（单根 free 池）未落成 Lean 定理——Lean `nav` 把 free 作参数（Ledger.lean:170），四模块内确认无「free 真相源唯一」结构定理（【数学原设计】问题 6 未证条 6）。
- **G9 Lean 孤岛定理无 Rust 对应物**（数学/Lean 有、实现无，按批次）：AncestorLifespan（rust MR2 未做）、SegmentAutoConstruct（A‴ 缺口）、CanonicalQuotientTower（对接声明未兑现）、SecondSellClosedLoop、VoiceCoverTheorem（781 行）、CausalEndpointImpossibility、RootSelDisambig、GlobalProductClassification、ConcreteBehaviorQuotient、SelfSimilarity、Pipeline（Lean 端到端 proven chain，rust 侧对应物挂 deprecated）、BspEventBridge、ClassificationGuardrail、HybridBridge、LedgerBridge，及 Tlayers/Strict/Formal/Phase2/Foundation 第一代 28 个 legacy 孤岛（A′ 裁定降级，重锚只做了被点名的几条）。
- **G10 优先级仲裁的原文张力（未裁决分歧，非实现缺口）**：策略 PDF 假设 4「解释器固定优先级」（缠论的全互斥定义策略.pdf p.20/27）vs necessity T₂₁「去全局互斥是定理」（necessity_derivation.md:517-530）——会计层数学对此中立（T₃₃–T₄₆ 不含仲裁结构），裁决定位在解释器层，扣留在 #174（OPEN）。

### B. 实现有、数学无（或蓝图出处未验证）

- **G11 OscillationBook（生产母子腿簿）**：生产已接线（能力 1⑥），但其 formal-chain 数学出处未见于本索引覆盖的语料——出处票据为 task #81/#82（pan_div/震荡线），蓝图直接对应**未验证**；#176 备忘 §1 记录它与 SplitLegLedger 不可直接替换（缺多 lot 生命周期与路由）。
- **G12 theta_v0/complete/ 17 分量 schema**：有 FULL §3/§20 出处与 Origin.CompleteStateEvent 锚，但自述「不替换闭环引擎」——为覆盖率报告补勾而建的零接线镜像（B+F）。
- **G13 一次性 prereg/报告 harness**：`backtest/treasury.rs`（S1，μ 层从未转发）、`backtest/capture_oos.rs`（S3）、`backtest/segment_gn.rs`（S2，「只产报告不定参」）、`backtest/highlow_mu.rs`（a3，`#[ignore]` 手动跑批）——全部零消费者；报告是否回喂决策未验证。
- **G14 spiral v2 群论引擎（8772 行）**：设计目标 = bit-exact 复现 unn（P&L 零增量是定理），**验收三判据无一条要求接生产**，验收通过即冻结；唯一分叉口 escalation（P-close 转 L3 alpha 重设计，2026-06-16 上浮）**至今无裁决记录**（`.chanlun/escalations/2026-06-16-spiral-pclose-l2-vs-l3.md`）。群论成果以谱系/概念形式被继承（recursive_t D∞ 齿轮引用），代码被绕过。
- **G15 旧引擎线判决「部分肯定」无承接**：nrf v4 严格会计 P1 3/8 历史最佳（`5e761a5ad5`）、hold26 有效域 4/8 白名单（`e8a2d93704`）、fusion_v P1 4/8（`06e29b7103`）、emht 五标的全正（`f204c0f464`）——2026-06-14 主线切 theta_v0 后集体零接续，无接线票据（analysis-py 档案组 2）。谱系 534 的 regime 分离裁定（趋势域 hold26 / 降成本 NRF）从未变成生产接线。

### C. 重造警示（C 类失守实证，出票必查）

- **G16 分账本四重实装互不引用**：ShortBook（06-12，死码）→ separate.rs（06-28，零消费者）→ OverlayState（07-04，旁路）→ DualLedger（07-19，deprecated）——后者均未回溯前者（【#187 报告】:67-68「互不知道对方存在」）；作者知悉度未验证。
- **G17 子树清仓两实装**：coverage AncOK（06-28 已接生产）vs exit.rs T4（07-22 仅测试）——T 线建成时生产已有行为近似物，#179 已裁「接线进生产、归一即删除散装等价物」（#183 OPEN）。

---

## 未验证 / 边界声明（090：声明 = 能力）

1. 本索引的状态判断绝大多数直接继承六块档案与【#187 报告】，未重新核实；继承源的未验证声明（trading 块 5 条、spiral 块 5 条、theta-v0 块 6 条、formal-lean 块 7 条、analysis-py 块 6 条、process 块 5 条、【#187 报告】7 条）继续有效，不再逐条复制。
2. 本次新做的一手核实仅限：rust/src 顶层旧 ladder 文件（fractal/stroke/segment/zhongshu/bi_engine/buysellpoint/moves/divergence/ph/segment_layers）的消费者 grep（recursive_t/fugue_v3/spiral 消费）；theta_v0/classifier 级别容器支撑层（recursive_tower/level_view/level_view_store）的消费者 grep（nest/turn_class/cand_predicate 等）；`formal/` 与 `docs/formal-chain/` 目录树（模块名与 PDF 名存在性）。
3. 「蓝图无 / 未验证」的判定基于【formal-chain 全卷】的覆盖声明——该卷未读 19 份 PDF（递归证明.pdf、推导完全分类.pdf、严格alpha.pdf、alpha 系列、gap 系列、anc.pdf 等），其中如藏有相关账本/机制表述，本索引未覆盖。
4. 未运行任何 cargo test / lake build / 回跑；全部状态为静态 grep + 档案继承。
5. Python 侧消费面以档案的 analysis/ + trading_system/ 覆盖为准；G:/、scripts/ 等目录未查（沿档案边界）。
6. 本索引不裁决任何「接/不接」；接线裁决在 #174（OPEN）、#179（已裁 T4）、#183（OPEN 实施票）一线。

*索引完。能力节 13 节（含附节「净额主账」），缺口 17 条。*
