# Lean 孤岛清点报告（issue #222 / wayfinder #221 子票）

- 日期：2026-07-24
- 清点对象：`formal/` 全树，主查 worktree `/tmp/kimi-nest-mainline/formal/`（分支 `kimi-nest-mainline-20260717`，HEAD ec728bf6cf，138 个 .lean）；对照主仓 `main` 分支（139 个 .lean，多 `Research/CycleIsomorphism.lean`）。
- 方法：解析 `lakefile.toml` 7 个 lean_lib 的 roots/globs + 全树 `import` 图（剥离 `/- -/`、`--` 注释后正则提取），传递闭包求 build 可达集；sorry/admit/axiom 统计同样在剥离注释后的代码上做。主仓 `.lake/build` 既有构建产物（137 个 .olean）用于交叉验证 build 可达性（只读，未发起新构建）。
- 标注惯例：【确证】= 有机器/文本直接证据；【推断】= 由证据合理推出；【未核实】= 未及验证。

---

## 〇、总结论（四问速答）

1. **实际孤岛数**：编排者口径 45 在两种严格定义下均不复现。
   - **A 级（未接入主 build，真·孤岛）**：worktree **2 件**（`Phase2/Claim7_DivergenceNesting`、`Origin/ParityFixtureExport`）；main 分支 **3 件**（外加 untracked 的 `Research/CycleIsomorphism`）。【确证：import 闭包 + main 仓 .olean 清单双向一致】
   - **B 级（接入 build 但无任何消费方/死端 root）**：**38 件**（列于 lakefile roots、被 lake build 编译，但全树无任何模块 import 它们）。【确证】
   - A+B 合计 **40 件（worktree）/ 41 件（main）**。
   - 45 的最接近重构：41（main A+B）+ Tlayers 4 个 Accounting 子模块（`Earning/Forest/Ledger/Solvency`，仅经死端 root `Accounting` 可达）= **45**。【推断：编排者可能把「仅在孤岛锥内可达」的文件也计入】
   - 更彻底的结构事实：若按「消费方链」迭代剥离（无 import 者逐层移除），最终只剩 **9 个模块**（`Formal.lean` 主入口 + 其闭包 `Formal/*` 8 件 + Formal 根）。**其余 129/138 文件全部处在「孤岛锥」内**——整个 Origin/Strict/Tlayers/Foundation/Phase2 上层建筑是自产自销的封闭锥，唯一下游是彼此。【确证：剥离算法】这是否算「孤岛」是定义问题，留 #225 裁定。
2. **证明完成度**：40 件 A+B 孤岛**全部零 sorry / 零 admit / 零 axiom**（剥离注释后统计；注释中提及 sorry 字样不计），合计 579 个 theorem/lemma。【确证】完成度不是处置理由——它们是「证完了但没人用」。
3. **rust 对应物**：40 件中 **14 件有 rust 生产侧直接契约锚/引用**（其中 2 件是 bit-exact parity 机器耦合的活消费），**8 件被 rust 注释明确宣布为 legacy/已重锚取代**，其余 18 件 rust 侧无痕迹（纯探索或超前形式化）。
4. **可接线性初判**（证据呈堂，不拍板）：见 §四。

---

## 一、A 级孤岛（未接入主 build）

| 文件 | 规模 | sorry | 主题 | 状态证据 |
|---|---|---|---|---|
| `Phase2/Claim7_DivergenceNesting.lean` | 334L / 14 定理 | 0 | 背驰/区间套完全分类（claim7） | 【确证】lakefile 注释称其为「与 solo `Formal/DivergenceNesting` 字节相同、为防命名空间碰撞而排除」——**但该注释已过期**：`Formal/DivergenceNesting` 后来新增 `import Formal.RecursiveConstruction` 及约 60 行「标准第5部分后半 Ext(t) 互斥穷尽分支」内容，两文件已漂移（diff 258 行）。Claim7 现为**过期副本**。 |
| `Origin/ParityFixtureExport.lean` | 169L / 0 定理 | 0 | parity fixture 机器导出脚本（`#eval` 输出 JSON） | 【确证】**有活消费方，是假孤岛**：rust `tests/theta_v0_buy_parity.rs`、`tests/theta_v0_classifier_parity.rs` 经 `include_str!("fixtures/theta_v0_parity.json")` 消费其 `#eval` 机器导出（631 铁律「机器产非手填」）。不入 lib roots 是合理的（`#eval` 脚本 + `import Lean.Data.Json`，属工具非库）。注：fixture 中 sell_* 段对应的 rust 卖侧 port `closed_loop/sell.rs` 已于 #181 下线，sell 段是否仍被任一测试读取【未核实】。 |
| `Research/CycleIsomorphism.lean`（仅 main，**untracked**） | ~?L | 0（自声明） | issue #144「周期同构猜想」条件化证明 + 显式反例 | 【确证】`git status` 显示 `?? formal/Research/`（未跟踪）；import `Origin.{ChanlunElements,VoiceTree,OperationRole18}`；无 sorry/admit/axiom（头部自声明）。**在编目分支上不存在，在 main 上是未提交的新研究**。 |

## 二、B 级孤岛（接入 build 但无消费方，38 件死端 root）

全部零 sorry/admit/axiom【确证】。按主题分群（括号为行数/定理数）：

### Phase2 交叉验证群（teammate 独立形式化，与 solo `Formal/*` 平行）
| 文件 | 规模 | rust 对应/状态 |
|---|---|---|
| `Phase2/Claim5_ConstitutiveLadder.lean` | 825L/24 | solo 对应物 `Formal/ConstitutiveLadder` 在主入口闭包内；rust 无直接锚 |
| `Phase2/Claim6_OperationalSemantics.lean` | 376L/13 | 同上（solo `Formal/OperationalSemantics` 被 `Strict.Op` 消费） |
| `Phase2/Claim10_SegmentV1.lean` | 514L/13 | 【确证】rust `parser/segment.rs` 注释：「契约重锚 legacy `Phase2/Claim10_SegmentV1` → `Origin.SegmentConstruction`+`SegmentFeatureSeq`+`SegmentFeatureComplete`」——**已被宣布为 legacy 并取代** |

### Tlayers T-框架群
| 文件 | 规模 | rust 对应/状态 |
|---|---|---|
| `Tlayers/Accounting.lean`（root） | 76L/2 | 聚合 root，独占子树 5 件（自身 + `Accounting/{Earning,Forest,Ledger,Solvency}`，task #49 会计层 T₃₃–T₄₆ E-set）；rust `fugue_v3/accounting.rs`、`spiral/accounting.rs` 为概念对应【推断：命名层】；rust `strategy/ledger.rs` 注释称 legacy `Tlayers/Accounting/TotalWealth.lean`「降为待清理 reference」（该文件已不存在，注释或指更早状态）【确证注释存在/未核实其时效】 |
| `Tlayers/Operational.lean` | 847L/54 | T₁₇–T₃₂ 递归操作层 E-set（task #48）；rust 无直接锚 |
| `Tlayers/Payoff.lean` | 479L/17 | payoff 层=成本阶段状态机（第31课）；rust 无直接锚 |
| `Tlayers/Spiral.lean` | 432L/26 | 几何螺旋层 T₄₇–T₅₉（task #50）；rust `src/spiral/` 整目录为概念对应【推断：命名层，未逐定理对拍】 |

### Strict 标准内核 Layer2 群（task #57 时代）
| 文件 | 规模 | rust 对应/状态 |
|---|---|---|
| `Strict/Decomp.lean` | 478L/19 | 【确证】rust `parser/canonical.rs`：「契约重锚 legacy `Strict/Decomp` → Origin 行为商」——**已宣布 legacy 并取代** |
| `Strict/LevelState.lean` | 537L/16 | 【确证】rust `classifier/level_state.rs`：「契约重锚 legacy `Strict/LevelState` → `Origin.CenterStates`+`Origin.RecursiveLevelSystem`」——**已宣布 legacy 并取代** |
| `Strict/Center.lean` | 458L/7 | 中枢三态+位置三态 Layer2（task #59）；独占子树含共享 `Claim9_CenterPosition`；rust 无直接锚（头部自引用文件名 `CenterStrict.lean` 与实际不符，小瑕疵） |
| `Strict/Parse.lean` | 543L/8 | π_Θ 第2步唯一递归解析（task #76）；rust `parser/tail.rs`、`parser/mod.rs` 提及 `Strict.Parse`【确证：被引用，未被宣布取代】 |

### Foundation 群
| 文件 | 规模 | rust 对应/状态 |
|---|---|---|
| `Foundation/CompleteClassificationLimits.lean` | 371L/11 | 【确证】头部自声明：「A′ 下本模块为**待重锚 legacy reference**；canonical base 已迁 formal/Origin/」——**自我宣布 legacy** |

### 适配器群
| 文件 | 规模 | rust 对应/状态 |
|---|---|---|
| `OriginAdapters/StrictPipeline.lean` | 104L/1 | A′ legacy→Origin 适配器骨架（task #96/#97）；rust `strategy/ledger.rs` 注释引用其 `FullDefinitionIface`【确证：被引用】；lakefile 自承认「尚不能无 sorry 实现的适配器不声明，只留桩」 |

### Origin canonical 群（24 件，A′ 主包死端）
| 文件 | 规模 | rust 对应/状态 |
|---|---|---|
| `Origin/CenterConstruct.lean` | 700L/29 | 【确证】**有活消费方（parity 机器耦合）**：其 `refZhongshusFromComponents` 的 `#eval` 机器导出 → `tests/fixtures/theta_v0_center_parity.json` → `theta_v0_center_parity.rs` bit-exact 断言。另被 `complete/state.rs`、`classifier/center.rs` 等引用 |
| `Origin/MainTheorem.lean` | 534L/23 | 【确证】rust `backtest/metrics.rs` 直接引用；L5 全局综合层 §21 最终总定理（封顶综合）；独占子树 6 件 |
| `Origin/EngineBridge.lean` | 92L/6 | 【确证】rust `closed_loop/conformance.rs`、`theta_v0/mod.rs` 直接引用（Strict.HybridAssembly ↔ Origin 桥，task #96 曾明确「延 Phase2」后接入） |
| `Origin/ForceConformance.lean` | 439L/12 | 【确证】rust `classifier/force_conformance.rs` 直接引用（task #131 MACD↔ForceMeasure 规约） |
| `Origin/MutexFinalTheorem.lean` | 632L/19 | 【确证】rust `strategy/coverage.rs` 直接引用（M29 互斥全定义策略最终定理，「goal 形式化顶点」）；独占子树 13 件（最大群） |
| `Origin/ParentDirContainer.lean` | 141L/4 | 【确证】rust `backtest/runner.rs:455`、`strategy/interp.rs:1221` 直接引用（σ_p(g) 父容器方向） |
| `Origin/TrendSixState.lean` | 410L/18 | 【确证】rust `classifier/six_state.rs` 契约锚（走势六态 r + 信号位向量 b） |
| `Origin/RMoveCompose.lean` | 499L/11 | 【确证】rust `classifier/rmove_compose.rs` 契约锚（port，第二类走势递归组装，task #51） |
| `Origin/RecursiveLevelSystem.lean` | 205L/11 | 【确证】rust `classifier/level_state.rs` 重锚目标之一（#89 窗口化递归，task #99）；独占子树 5 件 |
| `Origin/Pipeline.lean` | 677L/23 | S_Θ 端到端单管线 proven chain（raw→segmentsOf→centersOf→bspOf→recog→ledger 闭环）——**端到端的形式化主链**；rust 无直接锚【推断：theta_v0 整管线的形式规约】 |
| `Origin/SellClosedLoop.lean` | (在独占锥内) | 【确证】rust 卖侧 port `closed_loop/sell.rs` **已于 #181 下线**；rust 注释：「Lean `Origin/SellClosedLoop.lean` 形式化留存于 formal/」——**对应功能已退役，形式化留存** |
| `Origin/SecondSellClosedLoop.lean` | 516L/14 | 同上：第二类卖点闭环（task #129），rust 卖侧已下线【确证同据】 |
| `Origin/LedgerBridge.lean` | 259L/12 | A′ Phase2 双账本桥（task #101/#127）；rust `backtest/dual_ledger.rs` 为概念对应（M14 P^sep）【推断】 |
| `Origin/HybridBridge.lean` | 329L/7 | Origin FullDefinitionSystem 实例化（task #20 P5）；rust 无直接锚 |
| `Origin/VoiceCoverTheorem.lean` | 782L/28 | 主覆盖定理（声部主线顶点 C16–C21，五步归纳）；独占子树 7 件；rust 无直接锚 |
| `Origin/CenterComplete.lean` | 489L/17 | 完整中枢识别判据（task #118，637号 A→B 主塔迁移）；rust `classifier/center.rs` 等提及 CenterComplete【确证：被提及】 |
| `Origin/SegmentAutoConstruct.lean` | 471L/11 | segmentsOfComplete 全自动递归切分（A″ still-MISSING 判决）；rust 无直接锚 |
| `Origin/CanonicalQuotientTower.lean` | 341L/14 | PDF-only 级别递归「Can 商化基础」；rust `classifier/recursive_tower.rs` 概念对应【推断】 |
| `Origin/CausalEndpointImpossibility.lean` | 228L/5 | 不可能定理 II（C23：因果策略不可无延迟吃满事后端点）；rust 无直接锚 |
| `Origin/ClassificationGuardrail.lean` | 281L/9 | #91 分类极限+诚实降级重锚（task #100）；rust 无直接锚 |
| `Origin/ConcreteBehaviorQuotient.lean` | 506L/12 | 具体行为商（SG-A，acceptance #4 bit-exact 严格形式）；rust 无直接锚 |
| `Origin/GlobalProductClassification.lean` | 360L/5 | 全局乘积分类父子一致性 G_α（PDF-only 真缺口 2b）；rust 无直接锚 |
| `Origin/AncestorLifespan.lean` | 240L/6 | gap-B 祖先生命期不变量（MW7/MW8 前置，task #24）；rust 无直接锚 |
| `Origin/BspEventBridge.lean` | 381L/13 | 薄 bspOf↦厚 ChanlunEvent 真桥（task #128）；rust 无直接锚 |
| `Origin/RootSelDisambig.lean` | 411L/27 | 根声部方向选择+镜像反对称消歧+全局风控平仓；rust 无直接锚 |
| `Origin/SelfSimilarity.lean` | 529L/18 | §18/§19 自相似等变收口；rust 无直接锚 |

（注：上表「独占子树」= 孤岛锥内仅经该死端 root 可达的模块数，剥离算法产出；另有 49 个模块被多个死端 root 共享保活——它们是「锥内公共依赖」，若全部死端被处置也会连带成孤岛。）

## 三、与 rust 生产侧 / 端到端的对拍汇总

rust 生产侧 = `rust/src/theta_v0/`（parser→classifier→strategy→closed_loop→ledger→nautilus/backtest 端到端管线）+ legacy 顶层模块（`segment.rs`/`zhongshu.rs` 等）+ `spiral/`、`fugue_v3/`、`recursive_t/` 引擎。

**机器耦合活消费（bit-exact parity，631 铁律）【确证】**：
- `Origin/ParityFixtureExport.lean`（A 级）→ `theta_v0_parity.json` → buy/classifier parity 测试。
- `Origin/CenterConstruct.lean`（B 级）→ `theta_v0_center_parity.json` → center parity 测试。

**rust 注释直接引用/契约锚（12 个 formal 文件被引，其中孤岛 9 件）【确证】**：
EngineBridge、MainTheorem、ForceConformance、MutexFinalTheorem、ParentDirContainer、TrendSixState、RMoveCompose、RecursiveLevelSystem、CenterConstruct（上表）+ 非孤岛的 SeparateLedger/VoiceEat/LeverageCapital/CompleteStateEvent/TotalWealth/ForceInterface/FullDefinitionStrategy/ChanlunElements/IntervalNestCertificate。

**rust 已明确宣布 legacy/重锚取代（处置证据最强）【确证】**：
`Phase2/Claim10_SegmentV1`、`Strict/Decomp`、`Strict/LevelState`、`Foundation/CompleteClassificationLimits`（自声明）、`OriginAdapters.StrictPipeline`（接口已重锚，`strategy/ledger.rs`「legacy Strict/Tlayers → Origin canonical」）、`Tlayers/Accounting` 群（同注释语境）。

**对应功能已从 rust 退役【确证】**：`Origin/SellClosedLoop`、`Origin/SecondSellClosedLoop`（+ 非孤岛 `Origin/SellPointRecog` 的 rust port 同于 #181 下线）。

**rust 侧无任何痕迹（纯探索/超前形式化）【确证：双向 grep 无hit】**：Claim5、Claim6、Operational、Payoff、Strict.Center、Origin.{HybridBridge, VoiceCoverTheorem, SegmentAutoConstruct, CausalEndpointImpossibility, ClassificationGuardrail, ConcreteBehaviorQuotient, GlobalProductClassification, AncestorLifespan, BspEventBridge, RootSelDisambig, SelfSimilarity}、Research/CycleIsomorphism（其 issue #144 本身即研究性）。

## 四、可接线性初判（初步证据，处置裁定归 #225 grilling）

- **接线（接入主入口或被消费）候选**：`Origin/Pipeline`（端到端形式主链，与 theta_v0 管线同构，是「已实现功能的形式化」的最直接候选）、`Origin/MainTheorem`（封顶综合定理，metrics.rs 已引用）。证据：均有 rust 侧对应物且无 sorry。【推断】
- **归档（保留为 reference、降出 build）候选**：5 件 rust 注释已宣布 legacy 的文件（Claim10/Strict.Decomp/Strict.LevelState/CompleteClassificationLimits/StrictPipeline）+ Tlayers Accounting 群。证据：契约锚已迁 Origin canonical，继续占 build roots 是漂移面。【确证注释存在；归档动作本身未执行】
- **删除候选**：`Phase2/Claim7_DivergenceNesting`（过期副本，与权威源已漂移，lakefile 注释与事实不符——留着反而会误导）。`Origin/SellClosedLoop`/`SecondSellClosedLoop`：对应 rust 功能 #181 已退役，删除与否取决于「退役功能的形式化是否保留作谱系」——这涉及不可篡改/谱系学保留原则，**明确留 #225**。【确证漂移与退役事实；删除建议仅为推断】
- **绝非孤岛（应摘帽）**：`Origin/ParityFixtureExport`、`Origin/CenterConstruct`——parity 机器耦合的活工具，rust 测试天天消费。【确证】
- **需先补登记再处置**：`Research/CycleIsomorphism`（untracked，不在编目分支）——它甚至还没进版本控制，清点口径需先由编排者确认它是否属于 #222 范围。【确证 untracked 状态】

## 五、未核实/限制项

1. worktree 无 `.lake` 构建产物，build 可达性以 import 闭包计算 + main 仓 137 个 .olean 清单交叉验证（两者一致：main 未构建的恰为 Claim7/ParityFixtureExport/CycleIsomorphism）；未发起全新构建（遵守约束）。
2. fixture `theta_v0_parity.json` 的 sell_* 段在 rust 卖侧下线后是否仍被测试读取——未逐 key 核对。
3. 「rust 无痕迹」结论基于 `formal/` 路径引用 + 模块名/概念关键词 grep；若某 rust 文件仅意译概念未留名，可能漏配（低估对应关系，不会高估）。
4. 编排者 45 口径的出处未见原文，§〇的重构（41+4=45）为推断。
5. Strict/Center 与 Claim9_CenterPosition 的共享保活关系、以及 49 个锥内共享模块的逐件定级（它们本身非死端），未逐一展开——如 #225 需要可补二轮。
