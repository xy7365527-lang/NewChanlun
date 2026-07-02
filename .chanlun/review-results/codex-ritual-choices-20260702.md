# codex-ritual-choices-20260702

裁决效力来源：编排者 2026-07-02 明令「要裁决的全问 codex」（本日明令委托）。

对 `.chanlun/review-results/ritual-queue-20260702.md` §1 选择类 10 条，逐条读谱系原文后，
以 `.venv/bin/python -m newchan.codex decide` 模式（model=codex-cli, reasoning_effort=high）
请求裁决，每条给 codex 完整选项集+各选项后果+谱系上下文（context-file 全文见 `/tmp/codex-ritual-ctx/*.md`，
原始 codex CLI 完整交互另自动持久化于 `.chanlun/review-results/codex-decide-20260702-{1544,1545,1546}.md`
——注：10 次调用落在同一分钟时间戳窗口内导致文件名碰撞覆盖，本文档中的判决全文取自各次调用独立捕获的 stdout 日志，
与被覆盖前的持久化文件内容一致，无信息损失）。

本轮 10 条中，codex 均给出了明确的技术裁决（无整条 UNDECIDABLE）；634号和claim10号各有一个
**子问题**被 codex 明确标注 UNDECIDABLE（均为'是否需要编排者/治理层拍板'这类价值判断，
codex 如实标注未强行给出技术裁决无法覆盖的答案）。

## 判决摘要表

| 谱系号 | 一句话判决 |
|---|---|
| 576-ledger | C：双层并置（LedgerState + TWState，命名明确的单向有损投影，禁止暗示双向同构） |
| 665 | 两命题逻辑独立（P⇏Q，¬P⇏¬Q） |
| 663 | 判据B胜出（μ_net(z,a)>0为主判据），但须分层：p<0.05降级为证据强度/仓位治理门槛，不作可交易性定义 |
| 637 | 迁移到口径B（全三段交集） |
| 646 | 确认（CONFIRM）：范畴错误/消解成立 |
| 658 | 维持设计A（gid-scoped独立闭包） |
| 634 | 概念分离成立（真常驻⊥平台硬墙 vs 伪常驻=ceremony周期重复spawn） |
| 673-fix | 接口级拆分（cand_delta_type1_extreme/type2_completion/type3_retest… |
| maimai#4 | 第一部分：确认 |
| claim10 | 应修正xianduan.md:169-170以第67课原文语义为准（当前为镜像反转） |

---

## 576-ledger — 账本 R=Π-A-W vs 缠论取本金三阶段 TW 不同构（machine-checked L0）

**谱系原文**：`.chanlun/genealogy/pending/576-ledger-r-vs-tw-three-stage-semantic-alignment.md`

**待裁问题**：

系统承载哪个/哪些会计范畴——A（只留R=Π-A-W）/ B（只留TW三阶段）/ C（双层并置，显式声明不同构、单向有损投影）三选一。Lean 已 machine-checked 证明两者不同构。

**codex 判决摘要**：

C：双层并置（LedgerState + TWState，命名明确的单向有损投影，禁止暗示双向同构）

**codex 完整回复（含决策/推理链/替代方案/边界条件/风险，model=codex-cli）**：

**决策**：选 **C：双层并置**。

系统应同时承载两个不同会计范畴，但必须在代码层显式分离：

- `LedgerState / R=Π-A-W`：财务分配、储备、提款、闭环装配视角。
- `TWState / free+holding+withdrawn + stage`：缠论第31课“取本金”三阶段操盘语义。
- 两者只能有**命名明确的单向/有损投影**，不能提供或暗示双向同构转换。

**推理链**：

Lean 已验证两者不同构，所以技术上不能把它们压成一个“更通用账本”。强行统一会制造错误抽象。

选 C 的核心理由是：这两个模型回答的问题不同。`R=Π-A-W` 管的是利润如何被分配、资本化、提取、形成储备；`TW` 管的是持仓生命周期中本金是否回收、阶段是否不可逆切换、后续是否进入增股数阶段。它们不是字段命名差异，而是状态空间、操作可逆性、守恒律有效域都不同。

代码层建议采用：

- 不可变事件流作为物理事实来源。
- 两个独立 projection：`LedgerProjection` 与 `TWProjection`。
- `TWStage` 必须是状态机字段，不允许退化成由当前数值重算的派生标签。
- 投影函数必须在类型名/文档中标注损失，例如 `forgetStageToLedgerView`，避免叫 `toLedgerIso` 这类误导名称。
- 测试/Lean 定理继续保留“不同构”作为架构护栏。

**替代方案**：

A 被拒绝：只保留 `R=Π-A-W` 工程最简单，但会放弃对缠论第31课“取本金三阶段”的忠实表达。对于缠论形式化项目，这等于把核心操盘语义降级为外部叙述，代码不再承载该定义。

B 被拒绝：只保留 `TW` 能忠实缠论原文，但会丢掉当前财务账本闭环、储备/分配/提款语义，以及已有 `HybridAssembly` 接缝。`TW` 不自然表达利润分配账本，强行替代会让财务层变得拧巴。

不选“第四种统一模型”：除非能给出 machine-checked 的新结构，证明它不是伪装同构，也不丢失 stage、守恒律切换和外生价格维度。当前条件下没有这个模型。

**边界条件**：

应推翻 C 的情况只有几类：

- 产品明确收缩为“纯财务回测账本”，不再声称覆盖缠论第31课语义，则改选 A。
- 产品明确收缩为“纯缠论取本金形式化”，不需要提款/分配/储备闭环，则改选 B。
- 未来发现两套语义之一没有任何下游使用场景，维护成本超过价值。
- 出现新的形式化证明，能给出严格保语义的共同上层结构，并且不抹平不可逆 stage 与守恒律切换。

**风险**：

- 双模型会增加维护成本，尤其是测试矩阵和文档解释成本。
- 用户可能误把投影当同构，所以命名和文档必须非常克制。
- 两个 projection 对同一交易事件的解释可能产生表面冲突，需要在 UI/API 层标明当前使用的是财务视角还是取本金视角。
- 如果转换层写得过“聪明”，容易重新滑回伪统一模型。这里应保持简单、有损、显式。

---

## 665 — 选择偏差消除 ≠ 可交易 alpha（逻辑独立两命题）

**谱系原文**：`.chanlun/genealogy/pending/665-selection-bias-removal-vs-tradeable-alpha-logically-independent-663-664-peer.md`

**待裁问题**：

P=选择偏差已消除 与 Q=存在可交易alpha 是否有逻辑蕴含关系？与663/664/645同簇统一裁。

**codex 判决摘要**：

两命题逻辑独立（P⇏Q，¬P⇏¬Q）；混淆二者=non sequitur推理谬误

**codex 完整回复（含决策/推理链/替代方案/边界条件/风险，model=codex-cli）**：

**决策**：两命题逻辑独立。  
`P = 选择偏差已被消除` 不蕴含 `Q = 存在可交易 alpha`；`¬P` 也不蕴含 `¬Q`。工程/研究文档必须把它们写成两个独立结论：`bias-clean evidence` 与 `alpha-positive evidence` 分开验收。

**推理链**：

选择偏差消除只说明某个污染源被排除，形式上是 `¬H_bias`。但 `¬H_bias` 下至少有两类可能：

1. `H_alpha`：无偏后仍有正期望、扣成本后可交易；
2. `H_null`：无偏后是零 alpha 或负 alpha。

所以 `¬H_bias ⊬ H_alpha`。  
同理，`H_bias` 存在只说明当前检验流程不能作为干净证据，不说明真实世界不存在 alpha；它只能否定“这个证据有效”，不能否定“alpha 本体存在”。

因此：

- `P ⇒ Q`：否。
- `¬P ⇒ ¬Q`：否。
- 更精确地说，P/Q 是方法论清洁性与经济正期望两个不同层级的命题。

把“我们消除了选择偏差”写成“我们找到了 alpha”的证据，属于**非逻辑推论**，更具体是**排除单一替代解释即肯定目标假设**的错误；可称为 `non sequitur`、`false dichotomy / 排中错误`，或“把证无伪因误写成证有真因”。

**替代方案**：

拒绝方案一：`除偏差成功 = alpha 成立`。  
理由：无偏估计仍可能估到零或负收益，尤其在 holdout LCB≤0、多窗不稳健、block bootstrap 不显著时。

拒绝方案二：`未除偏差 = alpha 不存在`。  
理由：偏差污染的是当前证据链，不是 alpha 的本体存在性。结论应是“该流程不能证明 alpha”，不是“alpha 被证明不存在”。

拒绝方案三：让单标的 beta-residual 退化事实改变 P/Q 逻辑关系。  
理由：`corr(raw, B_market)=1.0`、残差恒为 `-ce` 只说明单标的同期 beta 扣除法在该有效域退化，并加强当前 alpha 否证；它不改变“除偏差”和“可交易 alpha”之间的逻辑独立性。

**边界条件**：

若后续把 `P` 重新定义为“所有已知偏差均消除，且无偏估计满足扣成本后 μ>0、稳健性通过”，那它已经不再是单纯的选择偏差消除，而是把 Q 的判据并入 P，逻辑关系会被定义性改写。

若发现 train/holdout 泄漏，当前 `P` 应撤销，但这仍不推出 `¬Q`，只推出“现有除偏差证据无效”。

若引入多标的独立市场因子或因果预测 beta，使 residual 检验不再退化，则可重新评估 Q；但这仍是 Q 的独立验证，不是 P 的自然推论。

**风险**：

最大风险是文档措辞滑坡：把“排除了 winner’s curse/selection bias”写成“策略已验证有效”。建议固定表述为：

“本检验仅支持选择偏差污染源被排除；可交易 alpha 需由独立的 μ>0、扣成本、稳健性与容量/执行性检验证明。”

单标的 beta-residual 退化还带来一个工程风险：若继续使用 `raw - B_market` 作为 BTC 单标的 alpha 残差，会得到形式上干净但信息量为零的指标，应标注为有效域外。

---

## 663 — 判据错误：统计显著 p<0.05 ≠ 可交易 μ>0

**谱系原文**：`.chanlun/genealogy/pending/663-criterion-error-statistical-significance-mistaken-for-tradeability-mu-positive-vs-sign-test.md`

**待裁问题**：

判据A（p<0.05）vs 判据B（μ(z,a)>0）——哪个才是可交易性的正确判据？收窄 Le Cam 有效域的方式随判据选择而不同。

**codex 判决摘要**：

判据B胜出（μ_net(z,a)>0为主判据），但须分层：p<0.05降级为证据强度/仓位治理门槛，不作可交易性定义

**codex 完整回复（含决策/推理链/替代方案/边界条件/风险，model=codex-cli）**：

**决策**：选择 **判据B：μ(z,a)>0 作为可交易性的主判据**。  
但代码层不要把它写成“只要样本均值大于 0 就满格通过”，而应分层实现：

`可交易性判据 = 净条件期望 μ_net(z,a) > 0`  
`证据强度判据 = p 值 / 置信区间 / 交叉验证 / 样本量标签`

也就是说：`p<0.05` 只能作为“证据强度”或“发布/放大仓位”的门槛，不能作为“是否有交易价值”的定义门槛。

**推理链**：

1. 交易对象是收益分布，不是假设检验结论。  
   策略真正关心的是条件动作 `a` 在结构状态 `z` 下的净期望收益是否为正：`E[R | z,a] - cost > 0`。这才对应“做了是否长期赚钱”。

2. `p<0.05`回答的是另一个问题：当前样本能否以传统频率派标准拒绝 `μ=0`。  
   它是证据强度判据，不是收益定义。低频高级别信号天然样本少，用它做硬门槛会系统性淘汰低频但大幅度的信号。

3. Le Cam 的有效域应收窄。  
   Le Cam 下界适合约束“短窗内区分 ±Δ 是否显著可辨”的问题，不应外推成“长窗累积正期望不可交易”。这是两个不同任务：一个是显著性检验，一个是净期望估计与累积收益验证。

4. 代码结构上应保留二者，而不是二选一丢掉一个。  
   最稳妥的数据结构是不可变评估结果，例如：
   - `mu_net`
   - `sample_count`
   - `total_pnl`
   - `cost_model`
   - `p_value: Option<f64>`
   - `evidence_grade`
   - `tradability = mu_net > 0`

   这样领域概念清楚：`tradability` 不依赖 `p_value`，但风控、仓位、发布等级可以依赖 `evidence_grade`。

**替代方案**：

拒绝判据A：`p<0.05 才算有效`。  
理由是它把“统计显著”误当成“可交易”，会对高级别低频信号产生结构性偏见。它可以用于论文式确认、模型推广、参数冻结或放大仓位，但不应定义 alpha 是否存在。

也拒绝“裸 μ̂>0 即无条件实盘”的极端版本。  
`μ̂>0` 是方向判据，但必须净掉手续费、滑点、冲击成本，并通过因果回测、长窗累积、跨时段或跨标的稳定性来控制假阳性。

**边界条件**：

应推翻或降级判据B的情况：

1. `μ_net <= 0`，即扣除成本后条件期望不为正。
2. 全历史长窗累积净值为负，或收益完全由极少数不可复现事件贡献。
3. 出现明显前视、幸存者偏差、参数挖掘、重复试验污染。
4. 跨时段、跨标的、滚动窗口中符号不稳定，`μ̂`方向频繁翻转。
5. 资金容量、滑点、成交约束使理论正期望在真实执行中消失。
6. 目标从“是否可交易”变成“是否可公开证明/是否允许大仓位部署”，此时可以引入 `p<0.05` 或更严格的证据门槛作为上层治理条件。

**风险**：

主要风险是假阳性增加：低样本 `μ̂>0` 可能只是运气。解决方式不是回到 `p<0.05` 硬门槛，而是把代码判据分层：`μ_net>0` 判方向可用，`evidence_grade` 控制信任等级、仓位、是否进入生产。

结论：663号应裁为 **判据B胜出，但必须工程化为“正净期望主判据 + 证据强度分层”**。`p<0.05` 保留为辅助证据，不再作为可交易性的定义。

---

## 637 — 中枢核心区间 ZG/ZD 三口径分离（两段/三段/端点）

**谱系原文**：`.chanlun/genealogy/pending/637-zhongshu-core-interval-caliber-separation-two-segment-vs-three-segment-vs-endpoints.md`

**待裁问题**：

主 tower 当前用口径A（前两段），rust 引擎参考实现用口径B（全三段，标注 canonical）。C（首尾段）已被 Lean 反例拒绝。主 tower 应否迁移到 B？

**codex 判决摘要**：

迁移到口径B（全三段交集）；口径A保留为显式命名的legacy/compat计算器，不再作主tower默认

**codex 完整回复（含决策/推理链/替代方案/边界条件/风险，model=codex-cli）**：

**决策**：主 tower 应迁移到 **口径B：全三段**。  
`ZG/ZD` 的 canonical 定义应为前三段走势类型区间的共同交集：

```text
ZD = max(d1, d2, d3)
ZG = min(g1, g2, g3)
```

口径A保留为显式命名的兼容/派生口径，例如 `TwoSegmentCore`、`legacyAInterval`，但不再作为主 tower 的默认中枢核心区间。

**推理链**：

1. “中枢核心区间”这个领域概念本质上来自“三个连续次级别走势类型的重叠部分”。代码结构应忠实表达这个三段共同约束，而不是只存前两段交集再隐含假设第三段不收窄区间。

2. B 对 A 是语义收紧，不是任意改口径。  
   当第三段贯穿 A 的核心区间时，A = B；当第三段更窄时，B 才会收窄。也就是说 A 正确依赖一个额外不变量：

   ```text
   d3 <= ZG_A && ZD_A <= g3
   ```

   如果主 tower 没有把这个不变量作为类型或证明条件表达出来，就不应把 A 当 canonical。

3. 仓库已有 rust engine reference 标注 B 为 canonical。主 tower 继续用 A，会导致同一概念在跨 Lean/rust、主塔/reference、分类器/买卖点模块之间长期漂移。这个成本会持续放大，比一次迁移的回归成本更危险。

4. 性能不是反对理由。A 是两个区间求交，B 是三个区间求交，都是常数时间，差异不可作为架构选择依据。

5. 最简单的长期结构是：一个 canonical `ZhongshuCoreInterval`，默认 B；A 作为有名字的 legacy/compat 计算器，而不是同名概念的另一套默认实现。

**替代方案**：

- **维持 A 作为主 tower**：拒绝。它依赖第三段不收窄核心区间的隐含条件，一旦 fixture 出现 `g3 < min(g1,g2)` 或 `d3 > max(d1,d2)`，A 会给出过宽核心区间。
- **A/B 并列长期共存且都叫 ZG/ZD**：拒绝。这会把概念差异外包给调用方，制造隐性 bug。
- **恢复或考虑 C**：拒绝。C 已被 Lean 反例证伪，只能保留为反例 fixture，不应进入生产路径。

**边界条件**：

只有在补充出强约束时才应推翻迁移到 B 的决定：

1. 原始缠论依据明确证明主 tower 所谓 `ZG/ZD` 在当前抽象层只应由前两段确定，第三段只用于确认中枢存在而不参与核心收窄。
2. 主 tower 的所有下游算法明确依赖“前两段核心”而非“三段共同核心”，并且该概念应改名为独立领域对象。
3. 系统能在类型、构造器或定理层保证第三段永远贯穿 A 区间，使 A 与 B 在主 tower 有效域内恒等。

**风险**：

- 迁移到 B 会收窄部分中枢区间，分类器、背驰判断、买卖点识别可能出现回归差异。
- 历史 fixture、parity 测试和文档中凡是默认 A 的地方需要重标口径。
- 若下游把“过宽的 A 区间”当作稳定行为使用，迁移会暴露隐藏耦合。

建议迁移方式：主 tower 默认改 B，同时保留 A 的显式 API 和回归 fixture，用口径名隔离旧行为。

---

## 646 — §9 等单元计数公理 vs rust depth-weight 不等——定义冲突

**谱系原文**：`.chanlun/genealogy/pending/646-section9-equal-unit-count-axiom-vs-rust-depth-weight-unequal-definition-conflict.md`

**待裁问题**：

本条谱系文件自身已记录一次先前 codex CLI 会话给出的裁决（C：范畴错误/消解）。本次请求独立审计该裁决是否站得住，而非重新做A/B开放选择。

**codex 判决摘要**：

确认（CONFIRM）：范畴错误/消解成立——leg.units(sizing层资本敞口) ≠ q_v(voice层单位计数)，depth_weight不等权应保留

**codex 完整回复（含决策/推理链/替代方案/边界条件/风险，model=codex-cli）**：

**决策**：确认（CONFIRM）：范畴错误，消解成立。

代码层选择：保留 `depth_weight` 不等权；不要把 `leg.units = base_units * depth_weight` 解释成 §9 的 `q_v`。应把两者建模为不同领域量：`q_v` 是 voice-state 的单位计数，`leg.units` 是 sizing/target 层的资本加权目标敞口。

**推理链**：

§9 的约束形式是条件式：`a_v = 1 => q_v = q_parent`。它约束的是 active voice 的单位计数恒等。这个恒等在 voice 层成立，进而在特定 notional 投影下可得到对冲抵消。

`depth_weight` 做的是另一件事：把 `base_units` 投影成不同 depth 的资本目标敞口，例如 `[0.6, 0.3, 0.1]`。这是 sizing 层的风险/资金分配参数，不是 voice 层的单位定义。

所以原冲突成立需要一个隐藏前提：`leg.units === q_v`。但谱系给出的定义恰好否定这个前提。若 `leg.units` 是 capital-weighted exposure，`q_v` 是 voice-unit count，那么“voice 单位相等”和“资本敞口不等权”可以同时为真。前者是治理公理，后者是下游投影策略。

因此 b1/b2 不是同一对象的两个互斥定义，而是两个不同投影空间里的陈述。原“不可同真”不成立。

**替代方案**：

把 `depth_weight` 改成等权：拒绝。这样会把 sizing 层强行并入 §9 的 voice 层，牺牲资本配置自由度，且解决的是误读出来的问题。

把 §9 扩展为约束所有下游敞口相等：拒绝。这会把 voice-state 公理扩大成 portfolio sizing 公理，概念边界过宽。

把 `leg.units` 直接当作 `q_v`：拒绝。除非源码或规格明确说 `leg.units` 是 active voice 的规范单位计数，否则这是范畴混淆。

**边界条件**：

若源码显示 `leg.units` 被直接写回或等同为 voice-state 的 `q_v`，此消解应推翻。

若 `depth_weight` 在 §9 判定之前参与生成 active voice 的单位数，使同一 active voice 中出现 `q_v != q_parent`，此消解应推翻。

若规格明确规定 `leg.units` 就是 §9 所称“手数”，而不是 target exposure / order sizing 输出，此消解应推翻。

若 BSP 决策被提升进 voice-state sizing，并且提升后的字段同时承担 `q_v` 与 capital exposure 两种职责，应重新裁决，优先拆类型或拆字段。

**风险**：

最大风险是命名误导：`leg.units` 这个名字容易让人误读成 §9 的 `q_v`。代码层最好用不同类型或字段名隔离，例如 `VoiceUnits` vs `TargetExposureUnits` / `SizedLegUnits`。

我未能在当前工作区核源码：代码图工具调用被取消，工作区为空。因此本裁决基于你给出的谱系全文；若实际 Rust 代码违反上述层次分离，应按边界条件重开。

---

## 658 — reducer acceptance rollup schema 未定义（sub→top）

**谱系原文**：`.chanlun/genealogy/pending/658-reducer-acceptance-rollup-schema-undefined-subgoal-to-toplevel.md`

**待裁问题**：

设计A（当前实现，gid-scoped独立闭包，为防MAJOR-2刻意设计）vs 设计B（sub-goal全PASS自动冒泡到top-level）。

**codex 判决摘要**：

维持设计A（gid-scoped独立闭包）；不采用隐式B冒泡；可选增强=显式声明式rollup（非默认）

**codex 完整回复（含决策/推理链/替代方案/边界条件/风险，model=codex-cli）**：

- **决策**：维持 **设计A：gid-scoped 独立闭包**。  
  子目标 `CHECK_PASS` 不应默认自动冒泡为 top-level goal acceptance passed。  
  可接受的增强是 **显式 rollup schema**，但不是隐式 B：只有 top-level acceptance 明确声明 `rollup_from` 映射时，reducer 才可把指定子目标验收结果汇总为顶层验收。

- **推理链**：
  1. 子目标完成与顶层验收不是同一领域概念。子目标表示分解任务通过，top-level acceptance 表示整体目标的验收事实；自动等同会混淆“工作分解”与“验收标准”。
  2. 当前 A 忠实保留事件作用域：`(sub_goal_id, acceptance_id)` 只关闭对应目标域内的验收项，符合不可变事件流里的局部事实建模。
  3. B 的“全部子目标 PASS => 顶层 PASS”只有在子目标集合被声明为穷尽且无额外整体验收时才成立；这是额外语义，不能默认推断。
  4. MAJOR-2 风险是结构性风险，不是实现细节。只靠 acceptance_id 同名或所有子目标通过来冒泡，会重新打开误闭合窗口。
  5. A 虽然要求顶层独立 `CHECK_PASS`，但它把责任边界放在 schema/goal 编写层，简单、可解释、失败模式明确。

- **替代方案**：
  - 拒绝 **B 隐式冒泡**：因为它把子目标 DAG 的完成状态当作顶层验收，默认假设子目标集合完整覆盖顶层目标。这个假设在 schema 中没有被表达，且会复活 MAJOR-2。
  - 不建议简单改 reducer 查询键：把 `gid` 查询改成任意 `sub_goal_id` 匹配不是修 bug，而是绕过未定义语义，会制造同名 acceptance_id 误闭合。
  - 可作为未来增强的方案：显式 rollup，例如 top-level acceptance 声明：
    `rollup_from: [{sub_goal_id, acceptance_id}]`，并支持 `all/any` 聚合。无声明时仍按 A。

- **边界条件**：
  - 如果项目 schema 明确规定：某类 goal 的 top-level acceptance 完全等价于一组子目标 acceptance 的穷尽汇总，可引入显式 rollup。
  - 如果 acceptance_id 被提升为全局唯一，且每个顶层 acceptance 与子目标 acceptance 有强类型映射，B 的风险会降低，但仍不应默认冒泡。
  - 如果 goal 系统转为纯任务树模型，而 top-level acceptance 不再代表独立验收事实，可以重新考虑自动 rollup。

- **风险**：
  - A 会增加 goal 编写成本，需要显式顶层验收事件，否则 goal 不会自动 terminated。
  - 使用者可能误以为“所有子目标 PASS”自然代表整体完成，需要文档和 schema 校验提醒。
  - 若未来加入显式 rollup，必须禁止同名隐式匹配，要求稳定 ID 映射，否则 MAJOR-2 会回归。

---

## 634 — 督导常驻 ⊥ 624号四象限③硬墙——工位分类冲突

**谱系原文**：`.chanlun/genealogy/pending/634-supervisor-persistent-vs-quadrant3-hardwall-conflict-event-vs-resident-station-taxonomy.md`

**待裁问题**：

编排者要求 teach-supervisor 常驻，624号发现 AGENT×持续监控象限无平台载体（硬墙）。'真常驻 vs 伪常驻'概念分离是否成立？是否需要改动已结算075号规范（触发020号阻断等待）？

**codex 判决摘要**：

概念分离成立（真常驻⊥平台硬墙 vs 伪常驻=ceremony周期重复spawn）；075号无需改核心分类，改在实现层加ActivationPolicy字段；020是否触发=UNDECIDABLE（治理权限问题）

**codex 完整回复（含决策/推理链/替代方案/边界条件/风险，model=codex-cli）**：

**决策**：概念分离成立，但代码层不应把“伪常驻”伪装成真正的 `resident`。技术选择是：

`teach-supervisor` 不实现真常驻；实现为 **ceremony 周期调度的重复 spawn**。

075 号规范不需要因此做核心分类修改。更合适的是在实现层增加独立的激活策略字段，例如：

```ts
ActivationPolicy =
  | EventTriggered(event)
  | CeremonyScheduled(phase)
  | Manual
```

`teach-supervisor` 归为 `CeremonyScheduled(every_ceremony)`，不是 `TrueResident`。

020 是否必须触发属于治理/流程裁决，我标为 **UNDECIDABLE**；但从代码层技术必要性看，这个改动不足以要求修改 075。

**推理链**：

真常驻和伪常驻对应的是不同生命周期语义，不是同一个概念的两种命名。真常驻要求长期进程、持续监控、主动投递通道；当前平台没有 carrier，所以 624 硬墙仍然成立。

伪常驻的真实保证是：“每个 ceremony 周期都重新生成一次督导审查”。这是调度不变量，不是进程存活不变量。它能满足“每轮都在场”的观察需求，但不能满足“随时响应、不关停、24/7 监控”的字面需求。

因此，正确建模方式是把“工位职责”和“激活策略”分离。075 如果是工位分类规范，不应被平台 workaround 污染；平台能力差异应落在调度层、执行层或 adapter 层。

**替代方案**：

1. 修改 075，新增“event-triggered vs 常驻”主分类：不选。它把运行时调度策略上升成领域分类，影响面过大，也容易把平台限制固化进概念本体。

2. 把伪常驻直接称为常驻：不选。这会掩盖 624 的能力缺口，形成声明膨胀。

3. 尝试空转 agent / 长循环模拟常驻：不选。没有可靠主动投递通道时，空转没有完整语义，只增加复杂度和资源风险。

4. 把 ceremony 周期视为普通业务事件：不完全选。可以作为系统调度事件实现，但应命名为 `CeremonyScheduled`，避免和业务事件触发混淆。

**边界条件**：

如果平台提供长期 agent carrier、主动消息通道、后台任务或可靠 hook-spawn 能力，则可重新评估真常驻。

如果 075 当前已经在代码或规范中强制规定“所有工位只能 EventTriggered”，导致 `CeremonyScheduled` 无法表达，则需要对 075 做最小兼容性修订。

如果编排者明确要求的是“进程不退出、随时响应”，伪常驻不能满足，应直接返回硬墙，而不是降级解释。

如果系统中出现多个无单一业务事件、必须每轮执行的工位，并且调度策略已经反复散落实现，才值得把激活策略正式纳入规范。

**风险**：

主要风险是术语滑坡：团队以后把“伪常驻”简称为“常驻”，再次误导能力边界。

其次是调度遗漏：既然伪常驻依赖 Lead/ceremony scheduler，每轮 spawn 必须成为可验证的不变量，最好有日志或测试检查。

还有一个架构风险：如果把每个 workaround 都写进 075，规范会膨胀，领域分类会和平台实现细节耦合。

---

## 673-fix — Cand^δ_ℓ 按 bsp 类型分叉（Type1 Extreme / Type2 走势完备性）

**谱系原文**：`.chanlun/genealogy/pending/673-cand-delta-predicate-scope-misuse-type1-divergence-vs-type2-completeness-h2-overfiltering.md`

**待裁问题**：

bug本身已定理级坐实（codex#7 + H2真实信号验证，1473样本，三因子分解58.72%+23.83%+17.45%=100%）。待裁的是修复设计：函数内分支 vs 接口级拆分？Type2判据具体如何构造？Type3是否需要独立第三分支？

**codex 判决摘要**：

接口级拆分（cand_delta_type1_extreme/type2_completion/type3_retest三独立函数+薄dispatcher）；Type2判据=anchor+departure+ret的走势完备性谓词；Type3需独立第三分支（不可复用Type2）

**codex 完整回复（含决策/推理链/替代方案/边界条件/风险，model=codex-cli）**：

**决策**：采用**接口级拆分**，并保留一个很薄的兼容调度器。

核心形态：

```text
cand_delta_type1_extreme(...)
cand_delta_type2_completion(...)
cand_delta_type3_retest(...)

cand_delta(...) = match BspEvidence:
  Type1Evidence -> cand_delta_type1_extreme
  Type2Evidence -> cand_delta_type2_completion
  Type3Evidence -> cand_delta_type3_retest
```

不要把 Type1/Type2/Type3 的真实逻辑塞进同一个 `Cand^δ_ℓ` 函数内部靠 `if bsp_class` 混跑。`Cand^δ_ℓ` 可以继续存在，但只能作为 dispatcher，不能承载判据逻辑。

**推理链**：

Type1、Type2、Type3 不是同一谓词的参数变体，而是三种不同领域概念：

- Type1：背驰段 / Extreme / 同向前驱 / 创新极值。
- Type2：一类点之后的回调或反弹走势完成，且不破一类点保护位。
- Type3：中枢离开后的回抽或反抽完成，且不回到中枢区间。

H2 实证已经说明 Type2 被杀掉的主因不是单个 `extreme` 条件，而是 Type1 谓词的方向、前驱、极值三层前件整体不适用于 Type2。也就是说，不能通过“放松 Type1 条件”修 Type2；Type2 需要自己的证据结构。

建议把输入也拆成不可变证据对象，而不是只传 `bsp_class`：

```text
Type1Evidence(parent_move, div_leg, prev_same_dir_leg, ...)
Type2Evidence(anchor_type1, departure_move, return_move, ...)
Type3Evidence(center, breakout_move, retest_move, ...)
```

这样可以在类型层面防止 Type2 误拿 Type1 的 `s_prev/m1/m2` 语义。

Type2 判据建议构造为：

```text
Cand2^δ_ℓ(c) iff exists anchor, departure, ret:
  anchor = confirmed Type1^δ_ℓ before c
  departure is completed move after anchor, direction = δ
  ret is completed counter-move after departure, direction = -δ
  c is the terminal extreme interval of ret
  ret does not break anchor protection price
```

买点对称展开：

```text
Type2 Buy:
  已有一买 anchor
  一买后有向上离开走势 departure，且已完成
  之后有向下回调走势 ret，且已完成
  ret.low >= anchor.low - tick_epsilon
  候选点 c 是 ret 的结束低点区间
```

卖点对称展开：

```text
Type2 Sell:
  已有一卖 anchor
  一卖后有向下离开走势 departure，且已完成
  之后有向上反弹走势 ret，且已完成
  ret.high <= anchor.high + tick_epsilon
  候选点 c 是 ret 的结束高点区间
```

这里的核心是 `completed move`，不是力度、背驰、创新极值。具体 `completed` 应调用项目里已结算的走势类型完成判定；如果项目尚未形式化“走势完备”，需要补一个独立谓词，例如 `is_completed_move(level, move)`，不要临时用 bar 数、MACD、振幅替代。

Type3 需要第三条独立分支。它最多复用 Type2 的“反向走势完成” helper，但不能复用 Type2 顶层谓词：

```text
Type3 Buy:
  存在中枢 Z
  向上离开中枢完成
  回抽走势完成
  回抽低点不回到中枢区间：ret.low > ZG，或按 tick 策略允许边界容差

Type3 Sell:
  存在中枢 Z
  向下离开中枢完成
  反抽走势完成
  反抽高点不回到中枢区间：ret.high < ZD，或按 tick 策略允许边界容差
```

**替代方案**：

拒绝“单函数内部 if/match 分支承载全部逻辑”。理由是它会继续共享变量、前件和 failure reason，容易让 Type1 的 `dir/prev/extreme` 语义再次污染 Type2。

拒绝“Type2 = Type1 去掉 extreme”。H2 已显示 82.55% 的失败发生在方向和前驱条件，说明 Type2 根本不是 Type1 的弱化版。

拒绝“Type3 复用 Type2”。Type2 的保护位是一类点极值，Type3 的保护边界是中枢区间边界；锚点不同，失效条件不同。

**边界条件**：

若 606/017 后续被形式化为 `Cand^δ_ℓ` 这个名字只允许表示 Type1 区间套候选，则应把 `Cand^δ_ℓ` 收窄回 Type1，并另建 `BspCandidate_type2/type3`，而不是扩展 `Cand` 语义。

若项目语言有强 ADT、trait、GADT 或 proof-object 风格，可以用一个多态接口实现，但语义上仍必须是三套独立证据和谓词。

若性能 profiling 证明函数分派在热路径有成本，可以内联 dispatcher，但不能合并判据。

**风险**：

最大风险是“走势完备性”未被项目内明确定义，导致 Type2 从过严变成过松。落地前至少需要确认：完成走势的级别关系、边界是否允许等价触碰、Type2 是否仅取一买/一卖后的第一次回调/反弹。

另一个风险是 Type3 边界处理：`不回到中枢` 对闭区间边界是否严格，需要按 tick size 和项目既有中枢定义统一，否则会产生边界样本分歧。

---

## maimai#4 — #4 结论过度泛化（三类→应第一类）+ 补 L2 确认层条款

**谱系原文**：`.chanlun/genealogy/pending/2026-07-02-source-tracing-type2-panzhengbeichi-vs-maimai-def4.md`

**待裁问题**：

maimai.md#4：'盘整背驰不产生三类买卖点中任何一类'——多篇一级权威博文显示该结论对Type2/3过度泛化。第一部分：收窄为仅Type1（是否为无争议的定理级更正）。第二部分：Type2/3的L2确认层条款具体措辞。与673号同批处理（maimai#4是673代码bug的定义层上游）。

**codex 判决摘要**：

第一部分：确认——定理级收窄，无价值判断残留，改为'盘整背驰不产生第一类买卖点'。第二部分：给出具体L2确认层条款文本（见下）

**codex 完整回复（含决策/推理链/替代方案/边界条件/风险，model=codex-cli）**：

**决策**：确认：定理级收窄，无需价值判断。`maimai.md #4` 应从“盘整背驰不产生三类买卖点中任何一类”改为“盘整背驰不产生第一类买卖点”。  
同时新增 L2 确认层条款：

> 第二、三类买卖点的结构位置仍由各自定义条款决定，盘整背驰不单独定义任何一类买卖点。对已满足 Type2/Type3 结构候选条件的次级别回抽/反抽，其 L2 确认可以由次级别以下的不创新高/不创新低，或盘整背驰导致的力度衰竭给出。第27课“多数第二、三类买点由盘整背驰构成”应解释为确认层证据，而非定义层替代。实现上，`trend` 背驰门只应用于 Type1；Type2/Type3 不得复用 Type1 的 `kind="trend"` 强制过滤，应接受盘整背驰作为确认层证据。

**推理链**：第27课直接说“多数第二、三类买点”由盘整背驰构成，这与“盘整背驰不产生三类中任何一类”正面冲突；但它不要求把盘整背驰升级成独立买卖点定义。因此最小修正是分层：Type1 触发层排除盘整背驰，Type2/3 确认层允许盘整背驰参与。这样保留原 #4 的有效内核，又切掉过度外延。

**替代方案**：拒绝继续维持“三类全部排除”，因为它会直接误导代码把 Type2/3 也接到 Type1 的趋势背驰门。也拒绝改成“盘整背驰产生第二、三类买卖点”，因为这会把确认层证据误写成定义层规则，破坏 Type2/3 的结构位置约束。

**边界条件**：若后续一级原文证明第27课语义只指“寻找精确点位的方法”，而非 Type2/3 成立确认，应把条款从“确认层证据”降级为“精确定位辅助”。若现有 maimai.md 已另有严格 Type2/3 定义与此条冲突，应以“不改定义层、只补确认层”为优先重写口径。

**风险**：最大风险是实现层把“允许盘整背驰确认”误解成“只要盘整背驰就生成 Type2/3”。应在数据结构上把 `Divergence(kind="consolidation")` 放进 `ConfirmationEvidence`，而不是放进 Type2/3 的核心构造条件。

---

## claim10 — xianduan.md:169-170 两情况终结语义镜像反转

**谱系原文**：`.chanlun/genealogy/pending/2026-06-25-claim10-xianduan-edition-vs-blog-termination-semantics.md`

**待裁问题**：

settled定义文档(v1.3)第169-170行与缠师原始博文(第67课)相比，两个case的终结语义疑似镜像反转。claim10自己的Lean形式化已采用与原文一致（即与settled文档相反）的语义。是否应修正settled文档？修正措辞？技术判断是否已足够作为编排者拍板依据？

**codex 判决摘要**：

应修正xianduan.md:169-170以第67课原文语义为准（当前为镜像反转）；给出具体修正措辞；技术判断已足够作为编排者拍板依据；'是否由编排者拍板'本身=UNDECIDABLE（治理权限问题）

**codex 完整回复（含决策/推理链/替代方案/边界条件/风险，model=codex-cli）**：

**决策**：应修正 `xianduan.md:169-170`，以第67课原文语义为准。当前 settled 文档若确实写成“第一种需发展为线段破坏、第二种直接线段破坏”，技术上应判为镜像反转。

**建议措辞**：

```md
- 第一种情况（特征序列第一、二元素无缺口）：特征序列顶/底分型形成后，原线段即在该分型的高点/低点结束；该极值点为线段终点。这里的直接终结以“特征序列分型已形成”为前提，不等同于裸笔破坏。
- 第二种情况（特征序列第一、二元素有缺口）：当前特征序列分型不能直接确认线段终结；须从该分型极值点开始的反向笔序列构成第二特征序列，并出现对应反向分型后，才确认原线段在前述极值点结束。向上线段对应“顶分型后等待第二特征序列底分型”，向下线段对称处理。
```

**推理链**：第67课原文把“无缺口”绑定到分型形成即终结，把“有缺口”绑定到额外确认条件；答疑中“第一种任何三笔其实都构成破坏 / 第二种并不是任何三笔都能构成破坏”也支持同一方向。第77、78课讨论的是“笔破坏后特征序列分型可能形不成”的古怪线段问题，不能反推成“第一种情况本身必须再发展为线段破坏”。代码层应把三层状态分开：`笔破坏`、`特征序列分型形成`、`线段终结确认`。

**替代方案**：拒绝保留 settled 现文。它会把第二种情况实现成过早终结，把第一种情况实现成过度等待，正好反转原文判定时序。也拒绝在代码里兼容两套语义作为同级模式；除非项目显式定义“编纂版算法口径”和“原文算法口径”并行，否则这会制造不可证明的分叉。

**边界条件**：若能找到更高优先级项目裁决明确说明 `xianduan.md` 的两情况命名与第67课原文命名相反，或能证明当前 settled 文档采用的是另一个已声明的形式化口径，此决策应重审。否则不应推翻。

**风险**：需要同步核查 `.chanlun/genealogy/settled/003` 及引用该镜像表述的下游说明，否则文档会继续互相污染。问题3中“是否由编排者最终拍板”属于治理权限，标记为 **UNDECIDABLE**；但就代码/文档层技术正确性而言，依据已经足够明确，可以作为编排者拍板的技术依据。  
本地 workspace 为空，未能直接读取仓库文件；判断基于你提供的原文摘录和谱系材料。

---

## 影响声明

本文档不修改任何定义文件、settled 谱系、或代码。产出仅为 codex 异质裁决记录，
供编排者/genealogist/Lead 在 /ritual 落盘时作为技术依据引用。以下条目的裁决落地
涉及的模块（仅列出裁决建议触及的范围，实际改动需另行 PR/commit）：

- 576-ledger：`rust/src/theta_v0` 账本子系统（若采纳C，需新增 LedgerProjection/TWProjection 双投影层）
- 665/663：alpha 研究报告措辞规范 + `perm_test`/`alpha` 判据代码（若采纳663判据B，需引入 mu_net/evidence_grade 分层字段）
- 637：`rust/src/theta_v0` 中枢核心区间计算（主 tower 迁移口径A→B，影响分类器/背驰/买卖点识别下游）
- 646：不改代码（确认消解，`depth_weight` 保留现状）；建议补充类型/字段命名以防未来误读
- 658：`scripts/goal_reducer.py`（维持现状A；若未来加显式rollup，需新增 schema 字段）
- 634：`.chanlun/genealogy/settled/075*`（实现层加 ActivationPolicy 字段，核心分类维持不改）
- 673-fix + maimai#4：`rust/src/theta_v0/backtest`（区间套候选谓词按 bsp 类型拆分为独立函数）+ `maimai.md`（#4条目收窄+补L2确认层条款）
- claim10：`.chanlun/definitions/xianduan.md:169-170`（改动权归编排者，本文档提供技术判断依据）
