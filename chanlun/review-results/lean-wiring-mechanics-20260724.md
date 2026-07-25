# Lean 接线机制核查（issue #238 / wayfinder map #221 子票）

- 日期：2026-07-24；执行：AFK research subagent（只呈事实与方案对照，裁定归 #236 grilling）。
- 输入：#236 裁定三件接线候选 `Origin/Pipeline.lean`、`Origin/LedgerBridge.lean`、`Origin/CanonicalQuotientTower.lean`；先例 `Origin/CenterConstruct.lean`、`Origin/ParityFixtureExport.lean`。
- 工作面：worktree `/tmp/kimi-nest-mainline/formal/`、`/tmp/kimi-nest-mainline/rust/`（只读）；主仓只读；无 git mutation；未发起 lake 全量构建（读既有产物与源码）。
- 标注：【确证】= 证据行号在案；【推断】= 合理推出；【未核实】单独列 §五。
- 姊妹篇（同工作线）：`lean-island-inventory-20260724.md`（清点 40 件）、`lean-island-disposition-20260724.md`（五档处置）。本报告不重复处置裁定，只答票体三问。

## 一、先例链机制清单（两条既有接线链，全链逐步）

### 1.1 ParityFixtureExport 链（独立导出脚本模式）

五步链【确证】：

1. **Lean 侧 def**：`formal/Origin/ParityFixtureExport.lean`（168L）定义 `fixtureJson : Json`（:94-161）。全部字段来自真函数求值——`decisionLedgerDelta`/`recogChanlun`/`chanlunTransition`/`sellTransition`/`classifyPosition` 真跑后序列化，枚举名由 match 真求值结果产出（631 铁律「机器产非手填」，:14-20）。imports = 5 个 Origin 库模块 + `Lean.Data.Json`（:40-45）。
2. **#eval 出口**：文件末尾 `#eval IO.println NewChanlun.Origin.ParityFixtureExport.fixtureJson.compress`（:168），stdout 输出单行压缩 JSON（裸 fixture，非 String 字面量，:165-167 注释）。
3. **落盘**：`lake env lean Origin/ParityFixtureExport.lean` stdout 重定向 → `rust/tests/fixtures/theta_v0_parity.json`（:28-30 自述命令）。【确证命令自述；落盘动作本身无脚本/Makefile/CI 证据——全仓 grep（.md/.sh/.py/.toml/.yml）仅 `docs/canonical-coverage-rust-impl.md:173` 一句描述 → 手工一次性命令【推断】】
4. **rust 消费点**：三个测试文件 `tests/theta_v0_buy_parity.rs:118`、`tests/theta_v0_lean_parity.rs:111`、`tests/theta_v0_classifier_parity.rs:100`，均 `include_str!("fixtures/theta_v0_parity.json")` + serde `Deserialize` 镜像 struct（编译期嵌入，fixture 变更触发重编译）。
5. **断言对象**：rust **生产模块**（`closed_loop/buy.rs`、`closed_loop/sell.rs`、classifier 位置分类）输出 == fixture 中 Lean 导出值，逐字段 bit-exact。

- **lake 挂载**：**不在任何 lib roots**（lakefile.toml 全文无 `ParityFixtureExport`【确证】）→ 不随 `lake build`；只能 `lake env lean` 单文件跑。inventory 判「不入 lib roots 是合理的（#eval 脚本 + import Lean.Data.Json，属工具非库）」。
- **消费形态**：**测试**（`cargo test` 自动发现 tests/*.rs），非生产路径消费；被测的是生产码。

### 1.2 CenterConstruct 链（库内嵌导出段模式）

1. **Lean 侧**：导出段**嵌在既有库文件内部**——`Origin/CenterConstruct.lean` §5.5.4（:582-634）：`refZhongshuJson`（:588）/`refV1FixtureJson`（:610）两个 Json def + 文件尾 `#eval IO.println refV1FixtureJson.compress`（:634）；import 追加 `Lean.Data.Json`（:51）。fixture 四 case：three_unsettled / four_extend / five_settled_break / v0_v1_interval（:604-630）。
2. **落盘**：手动 `lake env lean Origin/CenterConstruct.lean` 重定向 → `rust/tests/fixtures/theta_v0_center_parity.json`（:632-633 注释自述）。
3. **rust 消费点**：`tests/theta_v0_center_parity.rs:61` `include_str!` + serde 镜像（`FixtureZs`/`FixtureV0V1`/`CenterFixture`，:28-58），4 个 `#[test]`（parity_three_unsettled / four_extend / five_settled_break / v0_v1_interval，:113-180）。
4. **断言对象**：rust 生产模块 `theta_v0::classifier::ref_v1`（`ref_zhongshus_from_components`/`ref_v1_interval`/`legacy_v0_interval`）逐字段 == Lean 导出（`assert_zs_eq` 10 字段，:66-77）。

- **lake 挂载**：**在 Origin lib roots**（lakefile.toml:104 列 `Origin.CenterConstruct`；defaultTargets 含 `"Origin"`，:2）→ 随 `lake build` 编译，#eval 编译期执行（IO.println 进 build 日志）【#eval 编译期执行为 Lean 4 标准语义，未跑 build 亲验 → 推断】；但 **fixture 落盘不在 build 内**，仍需手动重定向。【roots 在案 = 确证】
- **★输入口径机制细节**：fixture 只携带**输出**（中枢全字段 + v0/v1 区间值）；**输入段是 rust 手编镜像**（`ref_seg0..ref_seg4_break`，theta_v0_center_parity.rs:93-107，与 Lean `refSeg0..refSeg4Break` 同价位人工对齐，CenterConstruct.lean:519-562）。即 631 机器耦合铁律只覆盖**输出侧**，输入侧靠双端人工同构。【确证】（ParityFixtureExport 链的输入侧是 Lean 见证 def 字段直接读值，输入也在机器产内——两链输入耦合度不同。）

### 1.3 两模式对比表

| 维度 | ParityFixtureExport | CenterConstruct |
|---|---|---|
| 文件性质 | 独立导出脚本（168L 纯工具） | 库文件内嵌导出段（§5.5.4，约 50L） |
| lakefile roots | 不在（不随 build） | 在（随 build 编译，#eval 进日志） |
| 导出物 | `rust/tests/fixtures/theta_v0_parity.json` | `rust/tests/fixtures/theta_v0_center_parity.json` |
| rust 消费点 | 3 个 parity 测试 | 1 个 parity 测试 |
| 被测 rust 对象 | closed_loop buy/sell + classifier 生产码 | classifier/ref_v1 生产码 |
| 输入耦合 | 输入=Lean 见证 def 字段，机器产 | 输出机器耦合；输入 rust 手编镜像 |
| 落盘 | 手动 `lake env lean … > fixture`（无脚本） | 同左 |

## 二、三件逐件评估

### 2.1 Origin/Pipeline.lean（S_Θ 端到端单管线 proven chain）

**现状**【确证】：

- 676L；已列 Origin roots（lakefile.toml:104）；主仓 `.lake/build/lib/lean/Origin/` 有 `Pipeline.olean`（已随 build 编译）；无下游 import（全 formal/ grep `import Origin.Pipeline` 零 hit，死端叶 root）。
- 内容：`OriginInput → OriginParse` 复合——构造链 `pipelineSegments`/`pipelineCenters`/`pipelineBsp`（:129-145）+ 闭环链 `candidateToEvent`/`pipelineEvents`/`chanlunTransitionFold`/`pipelineAccount`（:166-194）+ 整链 `originPipeline`（:231）+ total/unique/确定性/R=Π-A-W 贯穿定理（:247-365）+ §5 计算见证（`sampleInput` :418；`witness_pipeline_segments_nonempty`/`witness_pipeline_bsp_one`/`witness_pipeline_centers_empty`，:424-474）。
- ⚠ 现有 `sampleInput` 只 3 笔 → 2 段 → **0 中枢**（`witness_pipeline_centers_empty` 是诚实空见证，:470-474）——直接 fixture 化现有 sampleInput 的中枢段无信息量。
- rust 对偶：`parser/mod.rs:138 parse_layer` 声明为 `ElementPipeline.parse` **七段**顺序复合（mergeBars→…→bspOf→tailOf，:13-22）的 bit-exact 实装，生产码，被 p86/p92/p93/p102/p108/p109/p112 等 bin + `tests/theta_v0_perf_profile.rs` 消费。
- ⚠ **锚点口径事实**：parser/mod.rs 现锚 **Origin.ChanlunElements.ElementPipeline.parse**（七段批量），非 `Origin.Pipeline`；Lean `originPipeline` 是「3 段构造链 + 事件流闭环 fold」（吃已切好 `strokes`/`candidates`），rust `parse_layer` 是「7 段从 raw bars」。**段数口径与输入层级（Bar vs Stroke/BspCandidate）均不同**——rust 端无单一函数对应 `originPipeline` 全体（parse_layer + strategy/closed_loop 组合才覆盖）。【确证：Pipeline.lean §1-3 vs parser/mod.rs:13-22】

**形态选项 × 动什么 × 成本**：

| 形态 | Lean 侧 | rust 侧 | build 侧 | 成本 |
|---|---|---|---|---|
| (a) 契约锚注释 | 头部加 rust 互锚 + 口径差异声明 | parser/mod.rs 头部加 `↔ Origin.Pipeline.originPipeline` 锚 | 不动 | **小**（2 文件注释级） |
| (b) fixture 导出 + parity 测试 | ①设计胖 fixture 输入（≥4 段使中枢非空；现 sampleInput 中枢空）②加 Json 导出段（仿 §5.5.4，约 50-80L，`import Lean.Data.Json`）③跑 #eval 落盘新 fixture | ④新测试文件 + serde 镜像（约 150-200L）⑤输入镜像（bars 或 strokes，口径须先裁定） | 不动（已随 build） | **中偏大**（难点：双端输入层级不同——rust 测试从 stroke 层切入则绕开 mergeBars/strokesOf 两段，从 bar 层切入则 Lean 侧缺 bars→strokes 段；口径归 #236） |
| (c) build 接入闭包 | — | — | **零改动**（已在 roots + defaultTargets） | 无 |

**依赖闭包**：传递 18 件项目内模块，全部已在 Origin roots 内（BspClassification/BspConstruction/CenterAutoAssign/CenterConstruction/CenterFull/CenterStates/ChanlunElements/ClassifierFamily/CompleteClassification/Divergence/FullDefinitionStrategy/RiskProj/SegmentConstruction/SegmentFeatureSeq/SourceAxioms/StrategyFamily/ThetaInstantiation/TrendCompleteClassification）——**拉入 0 新件**。【确证：脚本重算，§三】

### 2.2 Origin/LedgerBridge.lean（双账本桥 vs dual_ledger.rs M14）

**现状**【确证】：

- 258L；已列 roots（lakefile.toml:104，注释 :93-96 载 task #101 桥 + #127 TW native 重锚）；`LedgerBridge.olean` 已编译；死端。
- 内容：**纯 compatibility/bridge 定理层**——11 个定理（`origin_ledger_inv_preserved` :80、`tw_threaded_in_loop` :109、`tw_preserved_in_loop` :124、`tw_stage_monotone_in_loop` :133、`tw_oq9_gate_preserved_in_loop` :142、`two_ledger_non_isomorphic_stage_collapses` :185、`dual_ledger_verdict_is_both_ends` :202 等），全部桥接 `Origin.TotalWealth` + `Strict.HybridAssembly` 已证定理；**无原生可执行 def、无具体 witness、无 native_decide**。imports = `Origin.FullDefinitionStrategy` + `Strict.HybridAssembly`（:58-59）。
- ⚠ **语义错位事实（接线裁定的关键）**：Lean 侧「双账本」= R=Π-A-W 单账本 ⊕ TW 取本金三阶段（#90 不同构两端并置，:30-52）；rust `backtest/dual_ledger.rs` 的「dual ledger」= M14 `P^sep=∏_v(R≥0 e⁺_v ⊕ R≥0 e⁻_v)` hedge-mode **多空分腿**头寸簿（:1-8）。**同名不同义**——两侧无可对拍的共同对象（Lean 无 M14 分腿结构的可执行 def）。disposition 表亦自标「模块级对应为推断」。【确证：LedgerBridge.lean:1-56 vs dual_ledger.rs:1-30】
- rust 对偶现状：dual_ledger.rs 生产码（`apply_fill_dual` :130、`compatible_leg_orders` :233）+ 模块内 `#[cfg(test)]` 9 个单测（:280-439+），含 D8 逐字节对拍锁（净额账本嵌入恒等）——rust 侧已有自洽测试网，**无 Lean fixture 消费**。

**形态选项 × 动什么 × 成本**：

| 形态 | Lean 侧 | rust 侧 | build 侧 | 成本 |
|---|---|---|---|---|
| (a) 契约锚注释 | 头部标注「rust dual_ledger.rs 是 M14 分腿簿，与本文件 R=Π-A-W⊕TW 双账本**非同一对象**」 | dual_ledger.rs 头部互锚（或明确不锚本件、锚向 TotalWealth 链） | 不动 | **小**（注释级，且唯一低失真形态） |
| (b) fixture 导出 + parity 测试 | **无可行对拍对象**——Lean 侧无可执行分腿 def，fixture 无物可导；若改为 R=Π-A-W⊕TW 导出，对应 rust 对象是 `strategy/ledger.rs`/TW 守卫（prove_tw_neutral），属 `Origin.TotalWealth` 链（已①留、11 处 rust 引用），非本票对偶 | — | — | **不可行**（须先实装 Lean M14 分腿结构 = 新形式化，成本大，非接线） |
| (c) build 接入闭包 | — | — | **零改动**（已在闭包） | 无 |

- ⚠ **依赖闭包特殊风险**：传递 17 件，含 **Strict.* 10 件**（HybridAssembly/HybridStep/Chain/Causal/Classification/ClassificationFamily/Fugue/OpenTail/RiskProj/StrategyFamily）+ `Formal.BSPLabels`。disposition §五裁定 Strict.* 共享件归④归档，但「锥内被①留件（EngineBridge/LedgerBridge）import 的 HybridAssembly 等在消费方存续期间不可物理移除」——**LedgerBridge 是把 Strict 锥锚在 build 内的两件锚件之一**：任何改 import/移文件的接线动作都牵动 Strict 锥处置；纯加注释不动 import 锥。【确证】

### 2.3 Origin/CanonicalQuotientTower.lean（Can 商化基础 vs recursive_tower.rs）

**现状**【确证】：

- 340L；已列 roots（lakefile.toml:104）；`CanonicalQuotientTower.olean` 已编译；死端。
- 内容：**全抽象参数化商化基础**——`Decomposition`(:90)/`LegalDecomp`(:108)/`CanonicalForm`（Setoid，:131)/`QuotientSingleton`(:178)/`canonical_representative_unique`(:199)/`existsUnique_decomp_of_quotient_singleton`(:243)/`CanonicalTower`(:261) + `toRecursiveLevelSystem` 对接（:309-332）。`Φ : PhiRule U` 是抽象规则族、`U : Type u` 泛型；**无具体实例化、无 native_decide witness**（codex 裁 (b)：商化是基础，∃! 是推论）。
- rust 对偶：`classifier/recursive_tower.rs`（生产码，`LeveledMove`/`RMove::Compose` 塔，task #53；被 7+ bin + `strategy/persistent.rs:36` 消费）。
- ⚠ **层级错位事实**：rust 塔头部自述契约锚 = **Origin.RecursiveLevelSystem.composeStep**（recursive_tower.rs:25-28「契约锚 Origin.RecursiveLevelSystem.composeStep + 走势分解定理二」），非 CanonicalQuotientTower；Lean 本件是 PDF-only **基础层**（商化 ⟹ ∃!），rust 塔是**具体窗口 compose 实例层**。基础 vs 实例，中间隔着 RecursiveLevelSystem（已①留且 rust 锚已在）。【确证】

**形态选项 × 动什么 × 成本**：

| 形态 | Lean 侧 | rust 侧 | build 侧 | 成本 |
|---|---|---|---|---|
| (a) 契约锚注释 | 头部加 rust 互锚 + 「本件是 RecursiveLevelSystem 实例的商化基础层」声明 | recursive_tower.rs 头部加基础层锚（现锚实例层保留） | 不动 | **小**（注释级） |
| (b) fixture 导出 + parity 测试 | **无可对拍对象**——全抽象无具体计算函数，导出需先实例化 Φ/U（新形式化工作，非接线） | — | — | **不可行**（前置形式化缺失；实例化 = 大） |
| (c) build 接入闭包 | — | — | **零改动**（已在闭包） | 无 |

**依赖闭包**：传递仅 4 件（ChanlunElements/CompleteClassification/SourceAxioms/TrendCompleteClassification），三件中最小，全部已在 Origin roots——**拉入 0 新件**。【确证】

## 三、依赖与阻塞（票体第三问）

1. **三件全部已列 lakefile.toml:104 Origin roots + defaultTargets 含 `"Origin"`（:2）+ 主仓 `.lake/build/lib/lean/Origin/` 三件 olean 均在**——**已接入主 build 闭包**。「接入主 build 闭包」**不需要动 lakefile.toml，更不需要动 Formal.lean**（Formal.lean 30L 仅聚合 Formal lib 的 8 个 `Formal.*` 模块，与 Origin lib 无涉；Origin 件的「主入口」就是 roots 列表本身）。【确证】
2. **死端核实**：全 formal/ grep `import Origin.(Pipeline|LedgerBridge|CanonicalQuotientTower)` 零 hit——无下游消费，三件是纯叶 root。接线不会「顺带拉活」其他孤岛件（无件依赖它们）。【确证】
3. **闭包实测**（传递 import 闭包，项目内模块数，tomllib/正则脚本重算）：
   - Pipeline：**18 件**，全部已在 Origin roots → 拉入 0 新件。
   - LedgerBridge：**17 件**（含 Strict.* 10 + Formal.BSPLabels），全部已在各 lib roots → 拉入 0 新件；但它是 Strict 锥留存的锚件之一（§2.2 风险在案）。
   - CanonicalQuotientTower：**4 件**，全部已在 Origin roots → 拉入 0 新件。
   【确证；与 disposition §五 48 件共享保活名单口径一致】
4. **fixture 再生工作流缺口**（两先例共同）：无脚本/Makefile/CI 固化「`lake env lean` → 重定向落盘」步骤，fixture 与 Lean 源的同步靠人跑命令——若三件中任何一件走 fixture 形态，此缺口照旧存在（或顺手脚本化，+1 小文件，不属本票裁定范围）。【确证：仓内无脚本证据】

## 四、逐件推荐（方案对照，裁定归 #236）

| 件 | 推荐形态 | 成本 | 事实层理由 |
|---|---|---|---|
| Pipeline | **(a) 契约锚先行**；(b) fixture 留作二期，且先裁定输入口径（Bar vs Stroke/七段 vs 3+2 段） | a=小；b=中偏大 | 双端段数/输入层级口径不一致在案；现 sampleInput 中枢空，fixture 需新设计胖输入；无单一 rust 函数对应 originPipeline 全体 |
| LedgerBridge | **(a) 契约锚**（必须注明同名不同义） | 小 | 无可对拍共同对象（Lean 无 M14 分腿 def）；fixture 形态不可行，除非先实装 Lean M14（新形式化，大）；且本件是 Strict 锥锚件，动 import 牵动归档处置 |
| CanonicalQuotientTower | **(a) 契约锚**（注明基础层 vs 实例层） | 小 | 全抽象无实例，fixture 无物可导；rust 锚已在实例层 RecursiveLevelSystem（①留件），本件补基础层锚即可 |

共同结论：**三件均无需动 lakefile/Formal.lean**（已在 build 闭包）；「接线不碰代码」对 (a) 成立（注释级），对 (b) 不成立（fixture 形态需动 Lean 导出段 + 新 fixture 文件 + rust 新测试文件——Pipeline 若走 (b)，触碰面 = 1 个 Lean 文件导出段 + 1 个新 fixture + 1 个新 rust 测试，与先例同构）。

## 五、未核实项

1. `#eval` 在 `lake build` 全量中的实际执行/输出形态未亲验（约束：不发起构建）。Lean 4 语义上 #eval 编译期执行，CenterConstruct 在 roots 故其 #eval 应随 build 跑、ParityFixtureExport 不在 roots 故不随——【推断】。
2. fixture 落盘命令的历史执行记录（谁跑的、何时、是否有仓外 regen 脚本）未核实——仓内无脚本证据。
3. `theta_v0_parity.json` 的 sell_* 段在卖侧 port 存续口径下是否仍被逐 key 读取（inventory 遗留未核实项，本票未展开）。
4. rust `parse_layer` 七段中 movesOf/tailOf 两段与 Lean 侧对应 def 的逐字段口径未逐行对拍（本票只核到模块级锚点）。
