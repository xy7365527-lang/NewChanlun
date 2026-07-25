# formal/ ↔ rust/src/theta_v0/ 全对应核查

> 审计对象：当前文件系统中的 `formal/` 与 `rust/src/theta_v0/`。  
> 审计方式：只读源码对拍；允许并实际运行 `cargo test --test …`。未执行任何 git 操作，未修改源码。  
> 执行日期：2026-07-25；文件名沿用任务指定的 `20260724`。

## 0. 口径与总判定

### 0.1 证据等级

- `【确证】`：Lean 声明、Rust 可执行实现或真实测试输出提供直接证据。
- `【推断】`：由直接证据组合得到，但仓内没有显式的一对一声明或最终处置记录。
- `【未核实】`：当前证据不足；不得写成“已一致”。

注释中“port Lean”“bit-exact”“一致”等自述均未单独作为一致性证据；只有声明体、实现分支、断言和本次测试结果进入判定。

### 0.2 清单边界

- 【确证】主入口闭包为 `Formal.lean` 加其 8 个 import，共 9 件；入口的实际 import 位于 `formal/Formal.lean:20-30`，库存报告亦记为闭包 9 件（`chanlun/review-results/lean-island-inventory-20260724.md:12-19`）。
- 【确证】“①留”12 件来自五档处置表（`chanlun/review-results/lean-island-disposition-20260724.md:17-32`）。
- 【推断】处置表当前仍列 7 个“前瞻保留候选”（`chanlun/review-results/lean-island-disposition-20260724.md:44-46,122-123`），没有另列“最终 2 件”清单。本报告按任务给定的“2 件”基数，取表内被明确判为与 #59 最直接衔接的 `Origin/SegmentAutoConstruct` 与 `Origin/BspEventBridge`（同文件 `:88-95`）。若编排者另有后续二选名单，这两行应以该名单复核。
- 【确证】接线 3 件按任务指定及接线报告入口：`Origin/Pipeline`、`Origin/LedgerBridge`、`Origin/CanonicalQuotientTower`（`chanlun/review-results/lean-wiring-mechanics-20260724.md:3-4`）。

### 0.3 总数

- 【确证】本报告逐件列出 26 件：主入口闭包 9 + ①留 12 + 前瞻保留 2 + 接线 3。
- 【推断】按“能否定位到当前 Rust 可执行符号”计，23/26（88.5%）有直接或复合对应；3/26 无等价 Rust 语义对象。
- 【推断】按更严格的“单个或紧邻符号级直接对应”计，15/26（57.7%）；另有复合/部分/机械对应 8/26（30.8%），无对应 3/26（11.5%）。这些比例是本次审计分类，不是形式化证明覆盖率。

## 1. 覆盖图

“直接”表示存在当前生产符号可逐语句比较，不表示比较结果必然一致；漂移结论见第 2 节。“复合/部分”表示一个 Lean 对象需由多个 Rust 阶段拼接，或只有声明的一部分被实现。

### 1.1 主入口闭包 9 件

| # | Lean 模块与核心声明 | 当前 Rust 对应物（文件:符号） | 覆盖判定 |
|---|---|---|---|
| 1 | `Formal.lean`：8 个理论模块的汇总入口（`formal/Formal.lean:20-30`） | `rust/src/theta_v0/mod.rs:32-46,60-65,74-89`：生产模块树入口；没有与 `Formal.lean` 同语义的运行时对象 | 【确证】**无对应**；只有架构入口类比，不能当成定理对应物。 |
| 2 | `Formal/TrendTrichotomy.lean`：`Direction`、`TrendKind`、三分性（`:41-78,90-107`） | `rust/src/theta_v0/types.rs:30-41` `Direction`；`:127-132` `MoveKind` | 【确证】**直接对应**；枚举层可逐值对拍。 |
| 3 | `Formal/RecursiveConstruction.lean`：`Move`、层级/区间、窗口递归构造（`:37-58,293-337`） | `rust/src/theta_v0/classifier/descend.rs:65-119` `RMove`/`descend`；`classifier/recursive_tower.rs:103-115,290-315` `LeveledMove`/`compose_level` | 【确证】**直接对应**；窗口策略本身存在漂移，见 2.10。 |
| 4 | `Formal/CenterTrichotomy.lean`：`CenterRelation` 与 `classify`（`:32-49,73-98`） | `rust/src/theta_v0/classifier/center.rs:63-77,209-220` `CenterRelation`/`classify_relation` | 【确证】**直接对应**。 |
| 5 | `Formal/BSPLabels.lean`：`BSPLabelSet`、端点情形到标签（`:27-62,181-227`） | `rust/src/theta_v0/types.rs:173-180` `BspBits`；`classifier/bsp.rs:55-109` `EndpointSituation`/`endpoint_to_bsp` | 【确证】**直接对应**；Rust 用位集承载标签集合。 |
| 6 | `Formal/RStarNonSpecial.lean`：`TowerLevel`、`ValidTower`、`operate`、终止向上闭包与非顶级性（`:42-99,118-163`） | 最近邻仅为 `rust/src/theta_v0/classifier/recursive_tower.rs:103-115,290-315` `LeveledMove`/`compose_level` | 【确证】**无对应**；Rust 没有 `ValidTower`、`terminated`、`operate` 对顶级性不敏感等同一契约对象。 |
| 7 | `Formal/ConstitutiveLadder.lean`：合并 K 线→分型→笔→线段（`:44-177`） | `rust/src/theta_v0/types.rs:49-110` `Bar`/`Fractal`/`Stroke`/`Segment`；`parser/mod.rs:83-109,138-143,275-289` `ParseLayer`/`parse_layer` | 【确证】**复合对应**；Rust 由 parser 多阶段实现，而非一个 ladder 对象。 |
| 8 | `Formal/DivergenceNesting.lean`：`Nested`、`NestingChain`、带嵌套一类点（`:144-169,273-306`） | `rust/src/theta_v0/classifier/nest.rs:39-119,155-234` `NestInterval`/`LevelNode`/`Chi`/`NestCertificate` | 【确证】**复合对应**；结构层直接，证书装配与生产门控另有更多状态。 |
| 9 | `Formal/OperationalSemantics.lean`：操作原子/路径、账户、执行、状态投影（`:61-180,218-298,347-397`） | `rust/src/theta_v0/types.rs:285-292` `StrictAction`；`closed_loop/state.rs:177-203` `AssemblyState`；`closed_loop/transition.rs:433-518` `transition_adapter` | 【确证】**复合对应**；Lean 单模块语义被 Rust 的类型、状态与迁移三层拆分。 |

### 1.2 “①留”12 件

| # | Lean 模块与核心声明 | 当前 Rust 对应物（文件:符号） | 覆盖判定 |
|---|---|---|---|
| 10 | `Origin/ParityFixtureExport.lean`：机器值与 JSON fixture 导出（`:14-20,40-45,94-168`） | `rust/tests/theta_v0_{center,classifier,buy,lean}_parity.rs`，分别消费两份 fixture（详见第 3 节） | 【确证】**机械/部分对应**；它是跨语言锁，不是生产符号。 |
| 11 | `Origin/CenterConstruct.lean`：reference 中枢构造与见证（`:351-398,476-490,518-634`） | `rust/src/theta_v0/classifier/ref_v1.rs:112-175,178-200` `centers_ref_v1`/区间适配 | 【确证】**直接对应**。 |
| 12 | `Origin/MainTheorem.lean`：六阶段闭环、账本及总定理（`:83-122,185-201,221-231,264-270`） | `rust/src/theta_v0/strategy/ledger.rs:633-655,685-704` 账本迁移，加 `closed_loop/` 多模块 | 【确证】**复合/部分对应**；Rust 没有一个可与总定理整体逐项对拍的对象。 |
| 13 | `Origin/EngineBridge.lean`：`RustEngineContract` 与装配（`:23-72,82-85`） | `rust/src/theta_v0/closed_loop/conformance.rs:68-113` `ThetaV0Contract` | 【确证】**直接对应**；仅接口/适配层。 |
| 14 | `Origin/ForceConformance.lean`：全域力度保真命题（`:93-102,261-275,360-375`） | `rust/src/theta_v0/classifier/force_conformance.rs:68-138` adapter/measure/divergence/faithfulness | 【确证】**直接对应**；但 Rust 只执行具体计算，未实现 Lean 全域命题的证明对象。 |
| 15 | `Origin/MutexFinalTheorem.lean`：语法元素全覆盖、唯一次序/角色（`:208-224,497-513`） | `rust/src/theta_v0/strategy/coverage.rs:2145-2238,2257-2293,2383-2398,2780-2801` 活跃腿、候选 Γ、`pi_theta` | 【确证】**直接对应**；量化域存在漂移，见 2.6。 |
| 16 | `Origin/ParentDirContainer.lean`：父方向容器及唯一性（`:33-46,68-110,116-129`） | `rust/src/theta_v0/strategy/coverage.rs:443-465,1017-1023,1268-1300` `attached_dir`/`parent_sign`/`classify_vertical` | 【确证】**直接对应**。 |
| 17 | `Origin/TrendSixState.lean`：六态与 signal bits（`:82-124,252-291`） | `rust/src/theta_v0/classifier/level_state.rs:32-87`；`classifier/six_state.rs:65-85` | 【确证】**直接对应**。 |
| 18 | `Origin/RMoveCompose.lean`：组合见证及合法性（`:115-125,142-165,197-215`） | `rust/src/theta_v0/classifier/rmove_compose.rs:68-110,123-169` | 【确证】**直接对应**。 |
| 19 | `Origin/RecursiveLevelSystem.lean`：`lift`、自相似与样例系统（`:86-104,123-174`） | `rust/src/theta_v0/classifier/recursive_tower.rs:290-315,716-858` `compose_level`/`detect_centers_windowed_resume`/`compose_level_resume` | 【确证】**直接对应**；窗口分解漂移，见 2.10。 |
| 20 | `Origin/CenterComplete.lean`：完整中枢构造（`:82-83,146-148`） | `rust/src/theta_v0/classifier/center.rs:91-100,119-120,154-173` `compute_z*`/`dir_alternates`/`center_from_segments` | 【确证】**直接对应**。 |
| 21 | `Origin/SellClosedLoop.lean`：卖侧识别与账本闭环（`:101-104,126-129`） | `rust/src/theta_v0/closed_loop/sell.rs:149-155,162-199` `recog_chanlun_sell`/卖侧迁移 | 【确证】**直接对应**。 |

### 1.3 前瞻保留 2 件

| # | Lean 模块与核心声明 | 当前 Rust 对应物（文件:符号） | 覆盖判定 |
|---|---|---|---|
| 22 | `Origin/SegmentAutoConstruct.lean`：特征序列、扫描与自动线段（`:69-87,105-159,206-227,325-359`） | `rust/src/theta_v0/parser/segment.rs:113-160,169-273,324-393` `feature_elements`/终止扫描/`divide_segments_with_tail` | 【确证】**直接对应**；本节只确认对象存在，未将动态扫描当作 Lean 的 bit-exact 实现。 |
| 23 | `Origin/BspEventBridge.lean`：`BridgesTo`、label soundness、一类点判定（`:101-132,179-186`） | `rust/src/theta_v0/classifier/bsp.rs:55-109` `endpoint_to_bsp`；`closed_loop/buy.rs:116-139` 买点识别 | 【确证】**复合/部分对应**；Rust 没有显式 `BridgesTo` 关系对象。 |

### 1.4 接线 3 件

| # | Lean 模块与核心声明 | 当前 Rust 对应物（文件:符号） | 覆盖判定 |
|---|---|---|---|
| 24 | `Origin/Pipeline.lean`：候选、事件 fold、结构输出与账户（`:120-146,166-196,209-235`） | 最近邻为 `rust/src/theta_v0/parser/mod.rs:83-109,138-143,275-289` `ParseLayer`/`parse_layer`，账户另在 `closed_loop/` | 【确证】**复合/部分对应**；不存在同边界的一站式 Rust pipeline。 |
| 25 | `Origin/LedgerBridge.lean`：R/TW 双账本及阶段接线（`:80-145,185-203`） | `rust/src/theta_v0/closed_loop/state.rs:177-203` `AssemblyState`；`closed_loop/transition.rs:208-230,433-518` 双账本迁移 | 【确证】**复合/部分对应**；当前真实对应是闭环双账本，不是同名旧候选。 |
| 26 | `Origin/CanonicalQuotientTower.lean`：`CanonicalForm`、商单点与 canonical tower（`:90-109,131-137,178-203,261-328`） | 最近邻仅为 `rust/src/theta_v0/classifier/recursive_tower.rs:103-115,290-315` 的具体 tower | 【确证】**无对应**；Rust 没有 `CanonicalForm`/等价关系/商唯一性对象。 |

## 2. 语句级漂移检测

### 2.1 汇总

- 【确证】15 件抽核结果：**一致 7、漂移 4、无法判定 4**。
- 【确证】一致：`CenterConstruct`、`EngineBridge`（接口层）、`ParentDirContainer`、`TrendSixState`、`RMoveCompose`、`CenterComplete`、`SellClosedLoop`。
- 【确证】漂移：`MutexFinalTheorem`、`RecursiveLevelSystem`、`Pipeline`、`LedgerBridge`。
- 【确证】无法判定：`ParityFixtureExport`（只缺当前 Lean→fixture 新鲜度）、`MainTheorem`（整体）、`ForceConformance`（全域）、`CanonicalQuotientTower`（无实例）。

### 2.2 逐件对拍

| # | 对拍对象 | Lean 声明 vs 当前 Rust 行为 | 判定 |
|---|---|---|---|
| 1 | `Origin/ParityFixtureExport` | Lean 定义并 `#eval` JSON（`formal/Origin/ParityFixtureExport.lean:94-168`）；Rust 四个测试真实读取已提交 fixture（`rust/tests/theta_v0_center_parity.rs:60-76`；其余见第 3 节）。本次 Rust↔fixture 全绿，但当前 Lean 源未能在不写构建产物的前提下重新导出 fixture。 | 【未核实】**无法判定当前 Lean 源→fixture 新鲜度**；不能由绿测反推源码当下仍一致。 |
| 2 | `Origin/CenterConstruct` | Lean reference 的 overlap/extend/settle 构造与固定见证位于 `formal/Origin/CenterConstruct.lean:351-398,476-490,518-634`；Rust `centers_ref_v1` 对同字段、区间和 settled/break 状态执行（`rust/src/theta_v0/classifier/ref_v1.rs:112-200`），中心 parity 又逐字段断言（`rust/tests/theta_v0_center_parity.rs:65-76`）。 | 【确证】**一致**（已提交见证域、逐字段）。 |
| 3 | `Origin/MainTheorem` | Lean 把六阶段、账本、closure obligations 合成一个总命题（`formal/Origin/MainTheorem.lean:83-122,185-201,221-231,264-270`）；Rust 有账本与闭环阶段，但没有承载“动态合同全同余/总闭合”的单一可执行断言（`rust/src/theta_v0/strategy/ledger.rs:633-655,685-704`）。 | 【未核实】**无法判定整体一致性**；局部对应不能推出总定理对应。 |
| 4 | `Origin/EngineBridge` | Lean 的 `RustEngineContract` 要求分类、风险、调度、迁移适配（`formal/Origin/EngineBridge.lean:23-72`）；Rust `ThetaV0Contract` 提供相同层次的实际入口（`rust/src/theta_v0/closed_loop/conformance.rs:68-113`），并有状态分类/适配测试（`:131-229`）。 | 【确证】**一致（接口形状与适配层）**；不外推到所有业务语义。 |
| 5 | `Origin/ForceConformance` | Lean 的目标是有限域乃至全域 `ForceConformsAll`（`formal/Origin/ForceConformance.lean:93-102,261-275`），文件自身仍把全域实现列为未验证（`:360-375`）；Rust 实现具体 adapter、measure、divergence 与 faithful 判定（`rust/src/theta_v0/classifier/force_conformance.rs:68-138`），测试只覆盖样例（`:150-245`）。 | 【未核实】**无法判定全域一致性**；样例绿不能提升为全称。 |
| 6 | `Origin/MutexFinalTheorem` | Lean 结论量化到每个 syntax element，并要求唯一次序/角色（`formal/Origin/MutexFinalTheorem.lean:497-513`）；Rust 实际从 previous active legs 与 `buckets.open` 生成被激活候选，再只对这些候选分类/投影（`rust/src/theta_v0/strategy/coverage.rs:2145-2238,2257-2293,2383-2398,2780-2801`）。 | 【确证】**漂移**：Rust 执行量化域是“活跃/新开候选”，小于 Lean 的“所有语法元素”；这不否定 Lean 抽象定理，但否定二者当前全对应。 |
| 7 | `Origin/ParentDirContainer` | Lean 要求子项携父方向且容器唯一（`formal/Origin/ParentDirContainer.lean:33-46,68-110,116-129`）；Rust 创建子腿时写入父 `eps`，读取 `parent_sign` 并据此垂直分类（`rust/src/theta_v0/strategy/coverage.rs:443-465,1017-1023,1268-1300`）。 | 【确证】**一致**。 |
| 8 | `Origin/TrendSixState` | Lean 六态及 bits 映射（`formal/Origin/TrendSixState.lean:82-124,252-291`）与 Rust 的 level state、six-state 分类（`rust/src/theta_v0/classifier/level_state.rs:32-87`；`classifier/six_state.rs:65-85`）分支一一可定位。 | 【确证】**一致**。 |
| 9 | `Origin/RMoveCompose` | Lean 要求组合见证满足连续/方向/层级条件（`formal/Origin/RMoveCompose.lean:115-125,142-165,197-215`）；Rust 枚举候选并只返回首个通过完整合法检查的见证（`rust/src/theta_v0/classifier/rmove_compose.rs:68-110,123-169`）。 | 【确证】**一致**；“返回首个合法见证”是合法细化，不改变存在性契约。 |
| 10 | `Origin/RecursiveLevelSystem` | Lean `lift` 由固定三项窗口组成（`formal/Formal/RecursiveConstruction.lean:317-337`），RLS 再以该 lift 表达层间自相似（`formal/Origin/RecursiveLevelSystem.lean:86-104,123-174`）；Rust 从运行时 detected windows 建上层，短序列可形成单窗，长序列扫描/续扫并可用非固定边界组合（`rust/src/theta_v0/classifier/recursive_tower.rs:290-315,716-797,815-858`）。 | 【确证】**漂移**：固定、不重叠的三项 canonical windows 与生产动态窗口不是同一 lift。 |
| 11 | `Origin/CenterComplete` | Lean 完整性谓词及构造（`formal/Origin/CenterComplete.lean:82-83,146-148`）要求三段交替且核心非空；Rust 明确计算 `zg/zd`、检查方向交替并在核心成立时构造（`rust/src/theta_v0/classifier/center.rs:91-100,119-120,154-173`）。 | 【确证】**一致**。 |
| 12 | `Origin/SellClosedLoop` | Lean 卖侧识别与账本变化（`formal/Origin/SellClosedLoop.lean:101-104,126-129`）在 Rust `recog_chanlun_sell` 与卖侧 transition 中逐分支实现（`rust/src/theta_v0/closed_loop/sell.rs:149-199`），且本次卖侧 parity 10 项全绿。 | 【确证】**一致（fixture 覆盖域）**。 |
| 13 | `Origin/Pipeline` | Lean 输入含 strokes/candidates/account，内部生成事件并 fold，输出 segments/centers/BSP/account（`formal/Origin/Pipeline.lean:120-146,166-196,209-235`）；当前 Rust `parse_layer` 输入 raw bars，只产 merged/fractals/strokes/segments/tail，不 fold account（`rust/src/theta_v0/parser/mod.rs:83-109,138-143,275-289`）。 | 【确证】**漂移**：若把同名 `parse_layer` 当接线对应物，其输入、阶段和输出边界均不同；跨 `parser + closed_loop` 的复合等价尚无整体锁。 |
| 14 | `Origin/LedgerBridge` | Lean 同时定义 R/TW 状态，却无条件证明闭环后 `tw` 保持相等（`formal/Origin/LedgerBridge.lean:80-126`）；Rust `AssemblyState` 同时持有两账本（`rust/src/theta_v0/closed_loop/state.rs:177-203`），迁移计算 realized PnL（`closed_loop/transition.rs:208-230`）、调用 `TwEvent::Realize`（`:451-455`）并写回两账本（`:506-518`）；测试直接断言 TW 分别变化 `+10/-6`（`:922-952`）。 | 【确证】**漂移**：当前生产迁移中 TW 可随已实现盈亏变化，反例否定 Lean 的无条件 `tw_preserved_in_loop`；至多在零成交/零实现盈亏子域保持。 |
| 15 | `Origin/CanonicalQuotientTower` | Lean 声明 `CanonicalForm`、等价关系、`QuotientSingleton`、代表元唯一及到 RLS 的映射（`formal/Origin/CanonicalQuotientTower.lean:90-109,131-203,261-328`）；Rust concrete tower 只有 `LeveledMove` 与组合过程（`rust/src/theta_v0/classifier/recursive_tower.rs:103-115,290-315`），没有可实例化的 `Can/Φ/Setoid/quotient`。 | 【未核实】**无法判定**；缺少共同对象，不能把 concrete tower 的存在写成商唯一性一致。 |

### 2.3 四项漂移的处置含义

1. 【确证】`LedgerBridge` 是最强直接反例：Lean 的无条件 TW 保持与 Rust 可达迁移冲突，应先收窄 Lean 定理前提或重建 bridge，不能靠注释/接线表宣称一致。
2. 【确证】`RecursiveLevelSystem` 的差异在构造规则本身，不是命名差异；若要锁生产塔，Lean 需形式化动态窗口扫描/续扫及其确定性。
3. 【确证】`Pipeline` 当前只能做复合接线设计，不能把 `parse_layer` 视为 Lean pipeline 的逐语句移植。
4. 【确证】`MutexFinalTheorem` 的 Rust 对应只覆盖进入 Γ 的活跃候选；要声称全对应，必须先证明“所有相关 syntax element 都进入该候选域”。

## 3. 机械锁有效性

### 3.1 测试发现与 fixture 消费

在 `rust/` 下执行 `ls tests/`，得到：

```text
econ_oddeven_diagnosis.rs
fixtures
theta_v0_07b_gating.rs
theta_v0_buy_parity.rs
theta_v0_center_parity.rs
theta_v0_classifier_parity.rs
theta_v0_lean_parity.rs
theta_v0_perf_profile.rs
```

- 【确证】parity 集成测试共 4 个：`theta_v0_buy_parity`、`theta_v0_center_parity`、`theta_v0_classifier_parity`、`theta_v0_lean_parity`。
- 【确证】`theta_v0_center_parity` 读取 `tests/fixtures/theta_v0_center_parity.json`（`rust/tests/theta_v0_center_parity.rs:60-62`），并对 `zd/zg/gg/dd/start/end/count/settled/break_*` 逐字段断言（`:65-76`）。
- 【确证】其余三个测试均读取 `tests/fixtures/theta_v0_parity.json`：classifier（`rust/tests/theta_v0_classifier_parity.rs:99-102`）、buy（`rust/tests/theta_v0_buy_parity.rs:116-119`）、lean/sell（`rust/tests/theta_v0_lean_parity.rs:110-112`）。

### 3.2 本次真实运行

所有命令均在 `rust/` 下运行。

| 命令 | 退出状态 | 测试尾行（原样） | 判定 |
|---|---:|---|---|
| `cargo test --test theta_v0_center_parity` | 0 | `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s` | 【确证】当前 Rust 对已提交 center fixture 逐字段 bit-exact。 |
| `cargo test --test theta_v0_classifier_parity` | 0 | `test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s` | 【确证】当前 Rust classifier 对已提交 parity fixture 全绿。 |
| `cargo test --test theta_v0_buy_parity` | 0 | `test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s` | 【确证】当前 Rust 买侧对已提交 parity fixture 全绿。 |
| `cargo test --test theta_v0_lean_parity` | 0 | `test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s` | 【确证】当前 Rust 卖侧/闭环对已提交 parity fixture 全绿。 |

- 【确证】本次未发现失败或陈旧到无法通过当前代码的 parity 测试：合计 43 passed、0 failed。
- 【确证】因此机械锁对“**当前 Rust 实现 ↔ 仓内已提交 fixture**”有效。
- 【未核实】机械锁尚不能证明“**当前 Lean 源 ↔ 仓内 fixture**”新鲜：接线报告明确记录 fixture 需手工 `#eval`/复制，仓内未发现自动生成与漂移检查脚本（`chanlun/review-results/lean-wiring-mechanics-20260724.md:15-21,24-32,108-115,127-132`）。
- 【确证】本次尝试只编译导出入口而不触发构建写入时，`lake env lean Origin/ParityFixtureExport.lean` 终止于 `Origin/ParityFixtureExport.lean:40:0: error: unknown module prefix 'Origin'`；center 导出入口同样在 `Origin/CenterConstruct.lean:47:0` 缺 `Origin` 模块前缀。由于任务禁止写报告外文件，未运行会生成 `.lake/build` 产物的补建。
- 【推断】当前锁是“单向 golden consumer 锁”，不是“Lean 源变更即自动再生/失败”的闭环锁；未来若要称双向机械锁，应把 Lean 导出、fixture diff 和四个 Rust 测试放入同一只读 CI 产物链。

## 4. Rust 主线的重要形式化缺口

### 4.1 缺口总表

| 缺口 | Rust 当前行为证据 | Lean 最近邻与缺失面 | 是否值得形式化 |
|---|---|---|---|
| #214 四桶测量装置 | `EndorsementProbe` 定义与分类结果位于 `rust/src/theta_v0/classifier/nest.rs:564-592,654-773,806-827`；`NestEndorsementInstrument` 及聚合在 `classifier/nest_index.rs:65-187,258-307`；输出 `NEST_GATE_FAIL/LEVEL` 在 `backtest/fill.rs:1638-1681`。现有读数给出四桶、level/kind 计数且干预为零（`chanlun/review-results/endorsement-failure-instrument-readings-20260724.md:28-56,69-101`）。 | Lean 仅有 `LevelNode/NestCertificate/Chi` 等结构（`formal/Strict/Nest.lean:202-222,298-303`；`formal/Origin/IntervalNestCertificate.lean:375-440`），没有四桶守恒、计数归因或 no-intervention 性质。 | 【确证】**值得，中高优先**：形式化“每次失败恰落一桶/总数守恒/测量 sidecar 不改变判定”；具体 `eprintln!` 文本格式不值得形式化。 |
| #218 两族判同 | `OwnerRef` 两族在 `rust/src/theta_v0/classifier/bsp.rs:41-46`；终态核对中枢 `(zd,zg)` 与点锚分别执行于 `classifier/nest.rs:654-760`，无法解析即 fail-closed。读数显示 c2 `0/40`、c3 `23/124`，并发现 9 个带相同但起点不同的碰撞及结果变化（`chanlun/review-results/owner-attribution-fix-readings-20260724.md:19-26,106-118,164-170`）。 | 当前 Lean 没有 `OwnerRef` 两族和这两种身份等价关系；结构性 Nest 定义不足以表达 owner 归因。 | 【确证】**值得，高优先**；先形式化“两族不混用、缺 owner/无法判同即拒绝”的和类型与 fail-closed 法则。【推断】`(zd,zg)` 是否为最终中枢身份仍需裁定，因现有碰撞已改变结果；裁定前不宜把该具体等价关系固化成最终教义。 |
| `nest_index` 证书索引 | `NestCertificateIndex` 的 identity key/get/instrument 在 `rust/src/theta_v0/classifier/nest_index.rs:199-237`，构建和装入在 `:258-307`。 | Lean 有证书结构/判定，但没有按事件身份索引、重复键处置、lookup soundness/completeness。 | 【确证】**值得，高优先**：证明 build→get 声音性、目标证书不漏、重复身份的确定性，以及索引读数不改变证书集合。 |
| `NEST_GATE_FAIL` / `NEST_GATE_LEVEL` 行族 | `rust/src/theta_v0/backtest/fill.rs:1653-1681` 把 trend 四桶、owner 子计数、点级二维计数、扫描数和 base/assembled/indexed level 一次输出。 | Lean 没有对应的测量 schema 或跨层计数约束。 | 【确证】**只形式化数据不变量，中优先**：字段和、层级单调/包含关系、同一运行快照的一致性值得；日志字符串、分隔符和打印顺序不值得。 |
| `exit_decision_for_nested_cert` | `rust/src/theta_v0/strategy/exit.rs:124-135` 通过证书查询闭包注入嵌套退出，`:150-183` 的实际布尔短路保证同向候选不查证书；调用点在 `backtest/fill.rs:1926-1951,2331-2357`。查询实现只在 `ChainVerdict::Pass` 时返回真（`backtest/admission.rs:1327-1358`），NoChain/断链拒的可执行断言在 `backtest/runner.rs:3629-3639`。 | 最近邻 `formal/Origin/SubVoiceOpenClose.lean:114-121` 只把 `chiParentDir` 当外部 Bool；没有证书 lookup miss=false、无 fallback、reverse short-circuit 和 action 决策边界。 | 【确证】**值得，高优先**：它直接决定交易退出，应证明“只有匹配证书可触发、lookup miss 不触发、`reverse_signal ∧ cert` 的短路纪律、不同索引视图不产生双重退出”。 |
| `NestLifecycleBook` 活假设状态机（V3 重建件） | 身份键/bridge identity 在 `rust/src/theta_v0/classifier/nest_lifecycle.rs:78-138`，事件状态与原因在 `:144-188`，entry/revision 在 `:210-272`，book 在 `:496-538`，推进/消失处理在 `:560-749`，运行不变量检查在 `:752-839`。 | 最近邻仅是结构 ancestor/lifespan（`formal/Origin/AncestorLifespan.lean:110-136,173-198`）与 `NestingChain`（`formal/Formal/DivergenceNesting.lean:160-169`）；没有 live hypothesis、revision、吸收态、时钟或 bridge identity 状态机。 | 【确证】**最值得，最高优先**：先形式化状态迁移合法性、终态吸收、观测时钟单调、revision append-only、消失/失效规则及 bridge identity 唯一性。【未核实】当前片段本身不能证明完整生产接线已经闭合，故不应把 V3 文件存在写成“生产状态机已形式化可替代”。 |

### 4.2 建议的形式化次序

1. 【推断】**P0：`NestLifecycleBook` 状态机核心不变量。** 它是跨时刻、可修订、可失效的状态载体，错误会污染后续所有证书/退出决策。
2. 【推断】**P0：证书索引 + `exit_decision_for_nested_cert`。** 先证明 lookup 的声音性/完整性，再证明交易动作只消费合法命中；两者应作为同一 action-boundary 形式化包。
3. 【推断】**P1：#218 两族判同。** 立即形式化族隔离和 fail-closed；具体中枢 identity 在处理 9 个 `(zd,zg)` 碰撞的裁定后再冻结。
4. 【推断】**P2：#214 测量守恒与 `NEST_GATE_*` schema 不变量。** 它对诊断可信度重要，但不应先于状态机和交易动作边界；日志呈现层不进入 Lean。

## 5. 最终回答

- 【确证】覆盖图已穷举任务给定的 26 件；23 件能在 Rust 定位直接/复合对应，3 件无等价语义对象。
- 【确证】15 件语句级抽核中，7 一致、4 漂移、4 无法判定；最明确的行为冲突是 `LedgerBridge.tw_preserved_in_loop` 对上 Rust 的可达 realized-PnL/TW 变化。
- 【确证】4 个 parity 集成测试真实运行全绿，共 43 passed、0 failed；它们确证当前 Rust 对已提交 fixture bit-exact。
- 【未核实】当前 Lean 源到 fixture 的新鲜度未被自动机械锁覆盖，因此不能把 43 个绿测升级成“当前 Lean 与当前 Rust 全对应”。
- 【推断】未来形式化的最高价值顺序是：`NestLifecycleBook` → 证书索引与 cert-gated exit → #218 两族判同 → #214 测量守恒。
