# Lean 孤岛五档处置表（issue #235 / wayfinder #221 子票）

- 日期：2026-07-24；执行：AFK 处置 subagent（机械档自判，边界件只呈证据，裁定归 #236 grilling / 编排者终审）。
- 判据：#225 已裁五档树 —— ①留（rust 活消费方在跑）②接（rust 已实装、可近期接线，进 #59 补充清单）③前瞻保留（Lean 零 sorry + rust 未实装 + 对得上 #59 Destination 组件）④归档（有价值但不在路线图）⑤删（只收误导性内容，谱系保留硬约束）。
- 输入：清点报告 `lean-island-inventory-20260724.md`（40 件）；worktree `formal/`、`rust/`（只读）；#59 Destination 组件清单（N2 力度门 / N3 塔内原生 / 活假设状态机 / LEE 级别原生执行 / χ 量纲 / C1 双开 / 5a5b 线形化 / 买卖点生命周期）。
- 标注：【确证】= 证据行号在案；【推断】= 合理推出；【供裁定】= 边界件，本表不拍板。
- 取证方法：rust 侧 `grep -rn --include="*.rs"` 双向核对（worktree `/tmp/kimi-nest-mainline/rust/`）；Lean 侧 diff/grep/head；import 图由 tomllib+正则重算（脚本一次性运行，只读）。

## 〇、对清点报告的三处纠正（090：声明=能力）

1. **Strict/Parse 已被 rust 宣布 legacy**（清点报告称「被引用，未被宣布取代」——不成立）：`parser/mod.rs:3-5`「契约重锚（legacy Strict/Parse.lean → Origin.ChanlunElements canonical）…重锚到 `Origin.ChanlunElements.ElementPipeline`」、`parser/tail.rs:3`「契约重锚（legacy Strict/OpenTail + Strict/Parse §6 → …）」。【确证】→ 归④归档。
2. **Origin/SellClosedLoop 在本 worktree 有活消费方**（清点报告称「rust 卖侧 port closed_loop/sell.rs 已于 #181 下线」——本 worktree 不复现）：`closed_loop/mod.rs:57` `pub mod sell;` 在编译；`sell.rs`（400+ 行活 port，sell.rs:1,15-25）被 `buy.rs:431`（`use super::super::sell::{sell_decision_ledger_delta, SellDecision}`）与 `backtest/econ_positive.rs:49`（`use ...closed_loop::sell::{sell_decision_of, SellDecision}`）消费；全树无「形式化留存」「#181 下线」注释痕迹。【确证】→ 归①留。#181 下线若是 main 分支后续状态，请编排者核对口径。
3. **OriginAdapters/StrictPipeline 的 rust 引用是中性活引用，非 legacy 宣布**（清点报告 §三将其列入「已宣布 legacy」组）：`strategy/ledger.rs:13`「接口别名见 `OriginAdapters/StrictPipeline.lean` `FullDefinitionIface`」——ledger.rs 的 legacy 宣布（:3-5）针对的是 Strict/Tlayers/Foundation 生态整体，StrictPipeline 作为重锚适配器被引用而非被取代。【确证】→ 边界件（见 §四）。

## 一、五档处置总表（40 件逐件）

### ①留（12 件：rust 测试/fixture/生产引用在跑）

| 件 | 活消费方证据 |
|---|---|
| `Origin/ParityFixtureExport`（A 级） | `tests/theta_v0_buy_parity.rs:62,118`、`tests/theta_v0_lean_parity.rs:111`、`tests/theta_v0_classifier_parity.rs:100` 均 `include_str!("fixtures/theta_v0_parity.json")`，fixture 由其 `#eval` 机器导出（631 铁律）【确证】 |
| `Origin/CenterConstruct` | `tests/theta_v0_center_parity.rs:61` 消费 `theta_v0_center_parity.json`，`CenterConstruct.lean:613,632-634` `#eval` 机器导出入口（fixture meta.source 自证）；另 `classifier/ref_v1.rs:4-256` 密集契约锚、`complete/state.rs:31,214`【确证】 |
| `Origin/MainTheorem` | `backtest/metrics.rs:52,568,913`（§22 多空镜像生产引用）【确证】 |
| `Origin/EngineBridge` | `closed_loop/conformance.rs:1,4,54-207`、`closed_loop/mod.rs:42,49-53`（U5 bit-exact 逐态 conformance 已实装）【确证】 |
| `Origin/ForceConformance` | `classifier/force_conformance.rs:22,26`（task #131 MACD↔ForceMeasure 规约）【确证】 |
| `Origin/MutexFinalTheorem` | `strategy/coverage.rs:2,36`（M29 互斥全定义策略顶点）【确证】 |
| `Origin/ParentDirContainer` | `backtest/runner.rs:362`（parentDirOfContainer 无持仓门控）【确证】 |
| `Origin/TrendSixState` | `classifier/six_state.rs:1,11-25`（六态 canonical 对照 + parity 断言）【确证】 |
| `Origin/RMoveCompose` | `classifier/rmove_compose.rs:1,31-132`（task #51 port）、`classifier/signal.rs:55,825`【确证】 |
| `Origin/RecursiveLevelSystem` | `classifier/mod.rs:7`、`classifier/level_state.rs:3`（重锚目标）、`classifier/descend.rs:7,52`【确证】 |
| `Origin/CenterComplete` | `classifier/center.rs:5,115,138,243-321`、`classifier/mod.rs:13,293,3058`（完整判据口径 B 在跑）【确证】 |
| `Origin/SellClosedLoop` | `closed_loop/sell.rs:1-407` 活 port（`mod.rs:57` 编译），被 `buy.rs:431`、`backtest/econ_positive.rs:49,619` 消费【确证；纠正清点报告，见 §〇.2】 |

### ②接候选（5 件：rust 侧已实装同构功能，进 #59 补充清单的动作【供裁定】）

| 件 | rust 同构证据 |
|---|---|
| `Origin/Pipeline`（S_Θ 端到端单管线 proven chain） | `parser/mod.rs:14-20`：rust `parse_layer` 声明为 `ElementPipeline.parse` 七段顺序复合（mergeBars→…→bspOf→tailOf）的 bit-exact 实装【确证同构声明】 |
| `Origin/MainTheorem`（封顶综合；已列①，接线=挂主入口/被更多消费） | `backtest/metrics.rs:52,568,913` 已引用 §22【确证】 |
| `Origin/LedgerBridge`（A′ 双账本桥，task #101/#127） | `backtest/dual_ledger.rs:1-3`（M14 `P^sep` 分腿头寸簿已实装，关⑤方案 B）【确证 rust 实装存在；模块级对应为推断】 |
| `Origin/CanonicalQuotientTower`（级别递归 Can 商化基础） | `classifier/recursive_tower.rs:1-9`（递归塔 UnitRange→RMove::Compose 升级已实装，task #53）【确证 rust 实装存在；模块级对应为推断】 |
| `Origin/SegmentAutoConstruct`（segmentsOfComplete 全自动递归切分） | `parser/segment.rs` 段切分已实装（契约锚 `Origin.SegmentConstruction` 族，segment.rs:3）【推断：构造层同构，字面无锚】 |

### ③前瞻保留候选（对上 #59 组件的无痕迹件；定档【供裁定】，见 §四对位表）

`Origin/BspEventBridge`（买卖点生命周期接线）、`Origin/SegmentAutoConstruct`（N3，亦列②）、`Tlayers/Operational`（N3，弱）、`Tlayers/Payoff`（三阶段 treasury，弱）、`Origin/AncestorLifespan`（买卖点生命周期，弱）、`Origin/SelfSimilarity`（N3，弱）、`Origin/RootSelDisambig`（出场侧，弱）。

### ④归档（6 件 + 1 群连带：rust 注释已宣布 legacy/被取代）

| 件 | legacy 宣布证据 |
|---|---|
| `Phase2/Claim10_SegmentV1` | `parser/segment.rs:3`「契约重锚（legacy Phase2/Claim10_SegmentV1 → Origin.SegmentConstruction+SegmentFeatureSeq+SegmentFeatureComplete）」【确证】 |
| `Strict/Decomp` | `parser/canonical.rs:3`「契约重锚（legacy Strict/Decomp → Origin 行为商…）」【确证】 |
| `Strict/LevelState` | `classifier/level_state.rs:3`「契约重锚（legacy Strict/LevelState → Origin.CenterStates+Origin.RecursiveLevelSystem）」【确证】 |
| `Strict/Parse` | `parser/mod.rs:3-5`、`parser/tail.rs:3`【确证；纠正清点报告，见 §〇.1】 |
| `Foundation/CompleteClassificationLimits` | 头部自声明（:1）「A′ 下本模块为待重锚 legacy reference；canonical base 已迁 formal/Origin/」【确证】 |
| `Tlayers/Accounting`（root；连带独占子树 `Accounting/{Earning,Forest,Ledger,Solvency}` 4 件） | `strategy/ledger.rs:21,38`「从 legacy `Tlayers/Accounting/TotalWealth.lean` 重锚到 `Origin.TotalWealth`…降为待清理 reference」+ `ledger.rs:3-5` 总宣布「legacy Strict/Tlayers/Foundation 降为待重锚 reference」【确证】 |

### ⑤删候选（1 件，证据呈堂；终审归编排者）

| 件 | 误导性证据 |
|---|---|
| `Phase2/Claim7_DivergenceNesting`（A 级） | lakefile.toml:15-16 注释声称「与 solo `Formal/DivergenceNesting` 字节相同…避免双份漂移」——**实测已漂移 258 行**：solo 版 584 行（新增 `import Formal.RecursiveConstruction` @:32 + §5 后半 `Ext(t)=⊔ⱼBⱼ(t)` 互斥穷尽分支集 :335-389），Claim7 版 333 行无此内容。lakefile 注释与事实不符，Claim7 现为过期副本，留存会使读者误认其为权威同步副本【确证漂移事实；删除裁定归编排者，谱系保留硬约束在案】 |

## 二、删候选补充说明

- 清点报告曾将 `Origin/SellClosedLoop`/`SecondSellClosedLoop` 列为删除待定（依「rust 卖侧 #181 下线」前提）。本 worktree 复核：**SellClosedLoop 消费方在跑（§一①），删除前提不成立**；`SecondSellClosedLoop` rust 侧双向 grep 零 hit【确证】，其处置（删/归档/前瞻）列入边界件【供裁定】，不擅自列删。

## 三、归档机械复核结论

§一④ 各件逐件复核注释原文（行号在案），全部成立、无反证；归档动作（降出 build roots、保留 reference）本身未执行——本票只处置定档，改动归后续实装票。

## 四、边界件（一律【供裁定】，只呈证据）

### 4a. rust 无痕迹 17 件 × #59 Destination 组件对位

无痕迹核实：17 件逐件 `grep -rln`（模块名/文件名两式）于 `rust/src`+`rust/tests` 全部零 hit【确证，2026-07-24 复核】。

| 件 | 主题 | #59 组件对位 | 对位证据/理由 |
|---|---|---|---|
| `Phase2/Claim5_ConstitutiveLadder` | 构造阶梯（teammate 平行形式化） | **对不上** | solo 对应物 `Formal/ConstitutiveLadder` 在主入口闭包内活着；Phase2 版为交叉验证副本 |
| `Phase2/Claim6_OperationalSemantics` | 操作语义（平行） | **对不上** | solo 版 `Formal/OperationalSemantics` 被 Strict.Op 消费（活） |
| `Tlayers/Operational` | T₁₇–T₃₂ 递归操作层 E-set | **弱对位 N3 塔内原生** | 递归操作层语义与「横切⑪ 塔内原生化长线」同域，但无组件级对应物 |
| `Tlayers/Payoff` | 成本阶段状态机（第31课） | **弱对位三阶段 treasury** | #59 Destination 末句「三阶段 treasury 重验落盘」；成本阶段=TW 三阶段同域 |
| `Strict/Center` | 中枢三态+位置三态 Layer2 | **对不上** | 中枢态语义已被 `Origin.CenterStates` 重锚覆盖（level_state.rs:3 语境）；头部自引用文件名瑕疵（CenterStrict.lean）在案 |
| `Origin/HybridBridge` | FullDefinitionSystem 实例化（task #20 P5） | **弱对位「全定义策略」主线** | 8 组件无直接对应物；互斥全定义顶点 MutexFinalTheorem 已①留 |
| `Origin/VoiceCoverTheorem` | 声部主覆盖定理 C16–C21 | **对不上** | 声部主线顶点，#59 组件无声部覆盖对应物 |
| `Origin/SegmentAutoConstruct` | segmentsOfComplete 全自动递归切分 | **对位 N3 塔内原生**（亦列②接候选） | 塔内原生段构造；#59「横切⑪ 塔内原生化长线」 |
| `Origin/CausalEndpointImpossibility` | 不可能定理 II（C23） | **对不上** | 元理论不可能性结果，非执行组件 |
| `Origin/ClassificationGuardrail` | #91 分类极限+诚实降级重锚 | **对不上** | 91 证书语义已在 V1 级别标定裁定中结算，非 Destination 组件 |
| `Origin/ConcreteBehaviorQuotient` | 具体行为商（SG-A） | **对不上** | canonical.rs:3 锚的是 `Origin.BehaviorQuotient`+`FiniteTraceQuotient`，非本件【确证】 |
| `Origin/GlobalProductClassification` | 全局乘积分类 G_α（PDF-only 缺口 2b） | **对不上** | 无对应组件 |
| `Origin/AncestorLifespan` | gap-B 祖先生命期不变量（MW7/MW8 前置） | **弱对位买卖点生命周期接线** | 「生命期」语义近生命周期，但对象为谱系祖先非买卖点 |
| `Origin/BspEventBridge` | 薄 bspOf↦厚 ChanlunEvent 真桥（task #128） | **对位买卖点生命周期接线** | 买卖点事件化是生命周期接线的结构前置 |
| `Origin/RootSelDisambig` | 根声部方向+镜像消歧+全局风控平仓 | **弱对位出场侧** | #59 Not yet specified「出场侧 NEST_GATE_EXIT 未接线」；平仓≠出场门，语义有距 |
| `Origin/SelfSimilarity` | §18/§19 自相似等变收口 | **弱对位 N3 塔内原生** | 自相似等变=级别递归理论基石，非直接组件 |
| `Research/CycleIsomorphism`（仅 main，untracked） | 周期同构猜想（#144） | **对不上** | 研究性；且未入版本控制，先补登记再处置【确证 untracked】 |

对位统计：对得上（含弱）7 件；对不上 10 件。

### 4b. 其他边界件

| 件 | 双向证据 |
|---|---|
| `OriginAdapters/StrictPipeline` | 被 `strategy/ledger.rs:13` 中性引用（接口别名来源，①方向）vs 其为 A′ legacy→Origin 适配器骨架（104L/1 定理），重锚两端已落 Origin（ledger.rs:16-38），适配器使命弱化（④方向）【供裁定】 |
| `Origin/SecondSellClosedLoop` | rust 双向 grep 零 hit【确证】；卖侧语义存续（sell.rs 在跑但只 port SellClosedLoop）；删/归档/前瞻归裁定【供裁定】 |
| `Tlayers/Spiral`（T₄₇–T₅₉ 几何螺旋层，task #50） | ②方向：`rust/src/spiral/` 整目录存在（accounting/closure/engine/ffi/mod 5 件，清点报告标「概念对应【推断：命名层，未逐定理对拍】」）；④方向：`strategy/ledger.rs:3-5` 总宣布「legacy Strict/Tlayers/Foundation 降为待重锚 reference」覆盖 Tlayers 生态【确证双向；接线 vs 随群归档归裁定】 |

## 五、锥内共享保活模块处置（报告口径 49 件）

名单重算：tomllib 解析 lakefile roots + import 闭包，得共享保活模块 **43 件**；加上裸名 import 解析差漏的 `Phase2/Claim9_CenterPosition`（被 Strict.Center:40、Strict/LevelState:72 共享）与 `Accounting/{Earning,Forest,Ledger,Solvency}`（被 Accounting.lean:42-45 裸名共享）= **48 件**，与报告 49 口径吻合【推断：残余 1 件同为裸名解析差，倾向 Strict.Nest，其归④不改变分群结论】。

- **①留：Origin.* 共享件 33 件**（SourceAxioms×25、CompleteClassification×23、ChanlunElements×23、TrendCompleteClassification×17、FullDefinitionStrategy×14、CenterStates×10、Divergence×9、BspClassification×8、VoiceTree×7、AncestorClosure/ClassifierFamily/RiskProj/SeparateStrategyTarget/StrategyFamily 各×5、ConstraintSystem/LexArgmin/SeparateLedger/ThetaInstantiation 各×4、ActiveSet/BehaviorQuotient/CandidateSet/CenterConstruction/CenterFull/NestingCertificate/OperationRole18/RThetaInterp/SubLevelDescent/TotalWealth 各×3、BspConstruction/SegmentConstruction/SegmentFeatureSeq/WellFoundedRank 各×2）——理由：lakefile.toml:74-88 载编排者裁定「Origin = A′ 方向唯一 canonical base」，共享层是 canonical 链的公共依赖；其中 22 件另有 rust 生产注释直接引用（ChanlunElements 5 文件、TotalWealth 11、FullDefinitionStrategy 9、BspClassification 6、CenterStates 5、VoiceTree 5、SubLevelDescent 5、RiskProj 5 等）【确证：lakefile 裁定文本 + rust grep 计数】。
- **④归档：Strict.* 共享件 10 件**（Classification×10、RiskProj×4、Causal/Chain/ClassificationFamily/Fugue/HybridAssembly/HybridStep/OpenTail/StrategyFamily ×3）——理由：`strategy/ledger.rs:3-5` 总宣布 legacy Strict 生态降 reference，且逐件重锚注释在案（bsp.rs:3 Strict/BSP→Origin.BspClassification；nest.rs:3 Strict/Nest→Origin.SubLevelDescent；transition.rs:4 Strict.HybridAssembly→Origin canonical；tail.rs:3 Strict/OpenTail→Origin.ChanlunElements.OpenTail）【确证】。同生态的 Strict.Trend/Recursive/BSP/Op/Nest 等非共享件若在 49 差额内同归④。注意：归档=保留 reference，锥内被①留件（EngineBridge/LedgerBridge）import 的 HybridAssembly 等在消费方存续期间不可物理移除。
- **④归档（随消费方）：`Phase2/Claim9_CenterPosition`**（两个消费方 Strict.Center、Strict/LevelState 均归④/边界）【推断】；**`Accounting/{Earning,Forest,Ledger,Solvency}` 4 件**（随 root `Tlayers/Accounting` ④）【确证 ledger.rs 注释语境】。
- 无「二轮待定」遗留。

## 六、供裁定清单汇总（归 #236 grilling / 编排者）

1. Claim7 删除终审（证据 §一⑤）。
2. ②接 5 件是否进 #59 补充清单（证据 §一②）。
3. ③前瞻 7 件定档（对位表 §4a；其中弱对位 5 件尤其需要拍板）。
4. 对不上 10 件的归档/前瞻分流（§4a）。
5. StrictPipeline、SecondSellClosedLoop、Tlayers/Spiral（②接线 vs ④随群归档）、CycleIsomorphism（先补登记）四边界件。
6. §〇.2 SellClosedLoop「#181 下线」口径核对（若 main 分支确已下线，本表①判定需随口径回改）。
