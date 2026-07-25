# 多空双开端到端修复设计：分层语义判定 + 载体裁定材料 + 执行层改造方案（只设计不实装）

- 工位：深度调研文档工位（wave kimi-nest-mainline-20260717），只写本 .md；rust/src 未改一行；无 git mutation；主仓只读。
- 纪律：090（声明=能力，照实否定合格，禁简化/禁补丁）/ v3 硬禁令（不引入概率/统计推断作决策基础；不用回测验证策略；不假设 EMH——本文全部为机制论证与结构恒等，无统计推断）。
- 输入在案：
  - 设计语义源：`docs/formal-chain/多空对冲.pdf`（16 页）、`docs/formal-chain/子声部.pdf`（27 页），2026-07-19 经 pdftotext 逐页直读，引用带页码；
  - 实装审计：`chanlun/review-results/netting-vs-voice-execution-audit-20260719.md`（fill 按净额 ΔN 执行、声部旁路只读、churn 机制）；
  - 邻接设计：`chanlun/review-results/multi-level-native-execution-design-20260719.md`（LEE 级别事件驱动）、`chanlun/review-results/p120-nested-voice-dual-ledger-design-20260718.md`（recognize 路径 DualLedger 施工图——**本文核实其已实装**，§2.1）；
  - 同 wave 姊妹文档（一致性核对后吸收，§1.0/§2.4/§7-0）：`dual-open-design-semantics-20260719.md`（分层语义判定）、`dual-open-implementation-audit-20260719.md`（实装审计+双开区间实证）——三方结论的分歧点原样并列于 §7-0，不擅自和稀泥；
  - 载体语义源：`docs/nested_fugue_accounting.md`（C2/C3 依据）、`docs/canonical-coverage-pdf-package.md:150`（规格 §9 锚）、`formal/Strict/StrategyFamily.lean:561-589`（结构允许定理），均本文直读核对。
- 代码锚 = worktree `/tmp/kimi-nest-mainline` 2026-07-19 现行工作树直读核对（注意：runner.rs 有在途未提交改动，行号以今日直读为准，漂移时以锚点周围语义注释重定位）。

## 0. 结论速览

| 任务 | 一句话结论 |
|---|---|
| ① 语义判定 | **分层判定**（§1）：**语义核心无条件**——多空双开 = 父仓保持 ∧ 反向子腿存在的分账本声部结构（语法层 L0，Lean `long_short_both_open_allowed` 证结构可表达，Strict/StrategyFamily.lean:561-589）；**物质化载体是裁定项**——文档链存在三候选：C1 hedge-mode 真实双仓（FULL 规格首选，canonical-coverage-pdf-package.md:150 §9 锚 `Q⁺>0∧Q⁻>0`）/ C2 净额+内部记账（nested_fugue_accounting.md §10 降级路径，当前 coverage/overlay 实装）/ C3 嵌套赋格会计（父真实减仓+子冻结-capital 账本空头，nested_fugue_accounting.md:228）。本文工程论证**推荐 C1**（§1.4 四点论据 + p120 已实装先例），但照实登记：C2/C3 各有文档依据，且当前实装既不是完整 C1 也不是完整 C3（§1.4 三载体对照表）——载体选择列裁定项 0（§7）。 |
| ② 与声部独立执行的关系 | **同一改造，不是两个改造**（C1 分支下）。netting 审计 §7 的声部独立执行臂（每声部独立账本+独立 fill）在仓位层自动兑现真双开（每声部单向 σ_v，父多子空并存即 (Q⁺,Q⁻)）；p120 已实装的 DualLedger（dual_ledger.rs:40-49）是同语义的账户层聚合。真双开 = 声部独立执行的仓位层推论 + DualLedger 的账户层读数，共享同一 substrate。 |
| ②' 与 LEE 的关系 | **组合，非二选一，也不需"每级每方向独立仓位槽"**。LEE 的 Ledger_ℓ 内部就是声部簿（级内赋格，LEE §E）；方向独立由声部簿自动承载。最终形态 = (ℓ, voice) 二维键控 hedge-mode 簿，每格单向，方向间不抵消、级别间不抵消（LEE 级别封闭不变量）。合并次序：声部独立执行先行（本方案），LEE 按自身 M1-M4 在其上加 level 路由（M1 级别账本旁路 bit-exact，可并行）。 |
| ③ bit-exact 协议 | 必须不变：决策层全量（typed_ledger/TW/step_trace.sep_legs/分类层/净额臂全部产出）+ 三条恒等（Σ_v Δq_v≡ΔN′；Σ_v pnl_v==Σ N′ΔP 臂内对账；事件空集时嵌入恒等逐字节）。设计性改变：订单流、N′ 轨迹、费用路径、结算粒度。**明文禁设**：跨臂毛价格 PnL 相等（sizing 事件化后 N′≠N，PDF §11 恒等只在同一持仓轨迹下成立）。 |
| ④ 裁定点 | **裁定项 0（载体三选一 C1/C2/C3）为前置裁定**——§3 方案是 C1 分支设计，§4 是 C2 分支设计，C3 只有要点素描（§7-0，未展开为方案，090 不伪造能力）；其余裁定点：margin/毛敞口口径、两条路径验收次序、sizing 哲学张力、venue hedge-mode 前提、LEE 合并时机（§7）。 |

---

## 1. 设计语义判定：分层结构 + 载体裁定

### 1.0 三层判定框架（与姊妹文档对齐后的综合）

同 wave 姊妹文档 `dual-open-design-semantics-20260719.md`（设计语义调研）与 `dual-open-implementation-audit-20260719.md`（实装审计）与本文独立取证后三方汇合，判定收敛为**三层**——语义核心无条件成立，物质化载体是裁定项，经济后果是无条件定理：

| 层 | 判定 | 依据 |
|---|---|---|
| **语法层 L0（无条件）** | 多空双开 = **父仓保持 ∧ 反向子腿存在的分账本声部结构**；声部树不禁止多空声部并存（结构可表达） | Lean `long_short_both_open_allowed`（Strict/StrategyFamily.lean:561-589：证「结构允许」，其 docstring 诚实标注**不证**「双开总可行」——保证金/容量属 EmpiricalDomain）；子声部.pdf p16-17（§5/§6/§7）；M13/M14；FULL 规格 §9「开平双开 a_v≤a_p(v)、同单位数 a_v=1⟹q_v=q_p(v)」锚 `Q⁺>0∧Q⁻>0`（canonical-coverage-pdf-package.md:150 对照表行） |
| **载体层（裁定项，三候选）** | **C1 hedge-mode 真实双仓**（两方向真实仓位共存于 position book）；**C2 净额+内部记账**（净额单仓+声部旁路）；**C3 嵌套赋格会计**（父真实减仓释放现金 + 子空头=冻结 capital 上的账本腿，一笔物理交易两个账本身份，空头非逐市无独立 MtM 负债） | C1：FULL §8/§9 规格首选（canonical spec §9 锚）+ 多空对冲.pdf p12 §10.2 + p120 已实装 DualLedger；C2：nested_fugue_accounting.md §10（:240-244「逐仓独立 position **或**净额+内部记账（如果交易所不支持同标的多仓位）」明文两读）；C3：nested_fugue_accounting.md:228（「空头持现金，无独立 MtM 负债」）+ §8.4（空头取冻结 capital 非逐市，递延到回补结算）+ feedback_recursive_nested_fugue.md:14（「物理上是一笔交易，会计上是两层记账」） |
| **经济层（无条件定理）** | **任何载体下**，同单位数双开在账户净值层不产生正价格收益；可度量产出只剩两类：(a) 诊断型声部毛捕获 score（语法覆盖，非 NAV）；(b) w≠1 或父 reduce 造成 ΔN≠0 时的 overlay 增量 | 多空对冲.pdf p4（§9 抵消定理严格版）/ p12（§10.2 hedge mode 不改变价格 PnL 抵消）/ p14（§11 最终结论）/ p2-3（定理 1 净额不可识别）/ p5（定理 2 可见性充要条件）/ p7-9（§6.2/§8） |

**两个 090 口径的禁句**（姊妹语义文档 :124 同款，本文重述）：声明「双开=两个方向的真实仓位必须同时存在」**只在 C1/逐仓载体内成立**；声明「双开在净额账户产生 NAV alpha」**在任何载体内都不成立**。

**术语约定**：本文以下「真双开」一律指 **C1**（hedge-mode 真实双仓，(Q⁺,Q⁻) 共存于仓位簿）；「账本双开」指 **C2**（净额执行+声部旁路记账）。

### 1.1 PDF 证据（逐页直读）

1. **多空对冲.pdf p11-12（§10.1）**：one-way/netting 账户「系统只保存 N_t=Σσ_v q_v……同一标的多空同时存在会被净额化」「若目标层或执行层只接受净目标 net_target_units=N_t，则声部层的多空腿不能独立出现在账户 NAV 中。**这正是你现在遇到的核心问题**」。——PDF 作者把净额压缩定性为**问题本身**，不是可接受状态。
2. **多空对冲.pdf p12（§10.2）**：hedge mode / separated position account「允许同一标的同时存在 Q⁺>0, Q⁻>0，position book 可以保存 (Q⁺,Q⁻) 而不是只保存 Q⁺−Q⁻」「**hedge mode 解决的是：腿的身份、保证金、执行、止损、归因可分开**」。——这五项全部是**执行层/仓位层**性质，不是观测层性质。同页同时声明「它不改变：同标的同单位数多空价格 PnL 抵消」——真双开的价值不在制造价格 PnL，而在腿身份/执行/归因的语义保真。
3. **多空对冲.pdf p14（§11 最终严格结论）**：「#5 的 depth>0 短差声部**可以在分账本语法层成立**；但它是否是账户净值 alpha，唯一取决于它是否改变净头寸过程并产生正的风险调整增量 PnL。」——分账本（voice ledger）双开是结构成立层；净头寸过程是经济有效层的判据。两层都真，互不替代。
4. **子声部.pdf p16（§5 position voice 严格定义）**：开仓 voice 是 position instance `v=(c,γ,σ,n)`，`posId(v)=H(c,γ,σ,n)`；同页（§14 简化的批判）指出 `posId_simp(v)=c`「会合并 entry signal、方向和 generation，**因此容易丢掉多实例语义**」。——设计语义要求每声部独立 position 实例，多实例语义不可压扁。
5. **子声部.pdf p16-17（§6 多重赋格父子关系）**：ShortDiff 要求 `σ_u=−σ_v`——「父多，子空；父空，子多」。父子声部是**同时 active 的反向 position instance**（§7 AncOK：`u∈A_i, d(u)>0 ⟹ p(u)∈A_i`）——两方向仓位必须同时存在于活动集，这是语法定理，不是观测选项。
6. **子声部.pdf p23（§16 新策略全定义）**：策略步骤 `p_i=LegTarget(A_i)`、`p*_i=LexArgmin_{p∈𝒦_Θ(x_i)} J_i(p,p_i)`、`O^order_i=Schedule_Θ(p*_i−p_{i−1})`——决策对象是**腿向量空间**（LegTarget(A_i) 由活动声部集产出），𝒦_Θ 是头寸空间可行集；净额标量是账户层可行化手段，不是设计对象。

### 1.2 教义/裁定链证据

- **M13（父仓保持）**：短差=父仓保持+次级别反向双开。净额账户的「先平后开」（runner.rs 注释链，p120 §1.3 直读：任一订单先平反向持仓再开新仓）使父腿的成本基与 round-trip 身份被子腿活动**物理摧毁**（部分减仓逐 fill 实现 PnL、父腿全平后翻转）——`bsp_consumption_redesign.md` §9.1 已裁定「净额翻转是空头不赚根因」（p120 §1.3 转引）。这是**语义违反**，不是观测缺口。
- **M14（P^sep 分账本）**：`P^sep=∏_v (R≥0 e⁺_v ⊕ R≥0 e⁻_v)`——多/空是两个独立坐标，p120 §4.1 将其兑现为 DualLedger（dual_ledger.rs:1-27 模块注释）。
- **实装自声明**：coverage.rs:1614-1624「净额化**丢失**双开的毛敞口信息……这是 Nautilus 净额兼容的必然降维，**非** bug。**完整毛分账本执行须 hedging 账户（v0 净额）**」——代码自己声明净额是 v0 兼容停损点，完整语义在 hedging 账户。
- **编排者先例**：p120 施工图（关⑤）选择真双开并已实装（§2.1 核实）——编排者裁定链的方向已是真双开。

### 1.3 对「C2（净额+旁路）即可、无需裁定」读法的驳斥

存在一个可能的读法：多空对冲.pdf §11 最终结论「唯一取决于它是否改变净头寸过程」+ nested_fugue_accounting §10 允许净额降级 ⟹ C2 是默认载体，净额单仓+声部旁路即可，连裁定都不需要。驳斥四点：

1. **§11 结论的论域是经济有效层，不是载体层语义**。PDF 自己的三层判定表（p10-11 §9：声部生成层 depth>0 active / 净额可见层 ‖ΔN‖₁ / 经济有效层 Π^overlay、IR、回撤）把「结构是否存在」与「NAV 是否看见」显式分层；§11 结论回答的是第三层，且以第一、二层可表示为前提。用第三层判据反推载体选择是论域错位。
2. **净额执行在机制上摧毁父腿身份**（§1.2 M13）：先平后开不是「看不见腿」，是「腿被拆掉」——typed ledger 的声部 round-trip 与真实成交脱节（netting 审计 §3 表「PnL 实现」行：声部平仓一次性实现 vs 每次减仓 fill 都实现一段）。C3（父真实减仓+冻结-capital 双层记账）虽也物理减仓父腿，但有严格的会计守恒与冻结-capital 语义兜底；当前 C2 实装两者皆无（§1.4 对照表）。
3. **费用语义不同且不可恢复**（netting 审计 §3）：PDF §11 线性恒等只覆盖价格 PnL，不覆盖费用；佣金 ∝ 周转名义，净额层周转（含 churn）与声部层周转（生命周期必需）差两个数量级（netting 审计 §4.2 量级论证）。账户层的真实费用路径只有在载体语义定稿后才能正确归因到腿。
4. **§10 的「两读」是部署条件分支，不是默认项**：nested_fugue_accounting.md:240-244 原文是「每个 voice level 可以对应一个独立的逐仓 position，**或者**用净额+内部记账（**如果交易所不支持**同标的多仓位）」——C2 是 venue 受限时的降级，规格首选是 C1；把降级路径当默认语义是因果倒置。

### 1.4 本文推荐 C1 的工程论据 + 当前实装的三载体对照

**推荐 C1（hedge-mode 真实双仓）的四点论据**（工程判断，最终载体仍列 §7 裁定项 0）：

1. **规格首选**：FULL §8/§9 规格层的双开定义锚在 `Q⁺>0∧Q⁻>0` 同存（canonical-coverage-pdf-package.md:150 对照表 §9 行）；多空对冲.pdf p12 §10.2 把 hedge mode 的五项收益（腿身份/保证金/执行/止损/归因可分开）全部定位在执行层/仓位层——这些是结构保真的物质载体，不是观测增强。
2. **M13 父仓保持的完整兑现**：C1 下父腿物理保持（p120 §4.3：开空不触多腿）；C2 下父腿被先平后开拆解；C3 下父真实减仓是**另一套设计语义**（嵌套赋格会计的降成本齿轮），需要冻结-capital 双层记账配套才完整——而配套不在当前实装中（见下对照表）。
3. **编排者先例已落地**：p120 施工图（关⑤）选择 C1 并已实装（dual_ledger.rs + recognize_nested + nautilus 接线，§2.1 核实）——裁定链的既有方向是 C1；推翻它需要新裁定，不是默认。
4. **代码自声明**：coverage.rs:1624「完整毛分账本执行须 hedging 账户（v0 净额）」——当前净额路径自己声明是 v0 兼容停损点。

**当前 coverage/overlay 实装 vs 三载体对照**（照实：现状不是任何一个载体的完整兑现）：

| 载体 | 关键语义 | 当前实装是否满足 |
|---|---|---|
| C1 真实双仓 | (Q⁺,Q⁻) 共存于仓位簿；fill 分腿；realized 按腿 | **否**——仓位簿是单标量 `units`（runner.rs:1065）；(Q⁺,Q⁻) 只在只读旁路 OverlayState（runner.rs:1555-1567） |
| C2 净额+内部记账 | 净额执行 + 声部旁路记账 | **是（账面）**——但附两个未声明后果：①churn 费用路径（netting 审计 §5，48.7× fill 放大）不是 C2 语义的必然部分，是「每 bar 重定目标+单订单出口」的实装附加物；②父腿身份在净额层被拆（§1.3-2）——姊妹实装审计评「无偏离」是按「C2 本义=账本双开+净额执行」口径（其 §4），该口径把 churn 与父腿拆解归为净额执行固有语义；本文登记两口径分歧，不擅自判死（§7-0 裁定材料） |
| C3 嵌套赋格会计 | 父真实减仓 + 子冻结-capital 账本空头 + 双层记账守恒 + 空头非逐市 | **否**——TW ShortDiff 划转按 \|units\| 净额名义（runner.rs:1234 `basis_now=(units.abs()·entry_cost.abs())`，本文直读），不是冻结-capital 模型；空头逐 bar 计入 MtM（`cum_price_pnl += units·Δpx`，runner.rs:1187），与 C3「空头非逐市、递延到回补结算」（nested_fugue_accounting §8.4）不符 |

**推论**：载体裁定不是「现状 vs 改造」的二选一——三个候选中现状只账面符合 C2 且带有 C2 语义外的 churn 附加物。**即使裁定 C2 或 C3，churn 修复（sizing 事件化，netting 审计 §6.1）与观测补全（§4）仍然必要**；C1 与 C2/C3 的真正分歧只在仓位簿与 fill 层（§3）。

### 1.5 判定边界（090：声明=能力）

- C1 推荐 ≠ 任何 NAV alpha 承诺。多空对冲.pdf p12 明文：hedge mode「不改变同标的同单位数多空价格 PnL 抵消」；p9 §8：ShortDiff 对冲腿的经济实质是「风险转换/暂时降敞口/路径风险管理」，其绩效指标是回撤/波动/尾部风险改善，「不一定是 standalone Sharpe」。本设计只修复**结构保真度**（腿身份贯穿到持仓与结算），不声明任何收益改善。
- 载体判定不依赖「双开有 alpha」——它是语义判定（M13/M14/PDF §10.2/规格 §9），有效性量测是另案（且不得以回测定优劣，只能验不变量与结构一致性，同 LEE §F-2 纪律）。
- **对姊妹实装审计一处越界主张的照实登记**：`dual-open-implementation-audit-20260719.md` §4 称「把双开解读为仓位层真双开与 StrategyFamily.lean:561-569 docstring 直接矛盾，属解读错误」——本文直读 Lean 原文（Strict/StrategyFamily.lean:561-589）后的核对结论：该 docstring 证的是**声部树结构可表达性**并诚实标注**不证可行性**（保证金/容量属 EmpiricalDomain），它既不支持也不排除任何账户载体；FULL 规格 §9 把双开锚在 `Q⁺>0∧Q⁻>0`（canonical-coverage-pdf-package.md:150）反而是 C1 的规格依据。两方引用的 Lean 文本同一、读法相反——此分歧原样列 §7-0 裁定材料，本文不擅自判死。

---

## 2. 实装审计现状：两条路径的真双开落差

### 2.1 recognize 路径：真双开已实装（p120 施工图已兑现，本文核实）

p120 是 2026-07-18 的施工图；本文 2026-07-19 直读 worktree 核实其**已落地**：

- `rust/src/theta_v0/backtest/dual_ledger.rs`（526 行）已存在：`DualLedger{cash, q_long, q_short, cost_long, cost_short}`（dual_ledger.rs:40-49）、`net_units/gross_units/equity`（:57/:62/:67）、`apply_fill_dual`（:130，分腿不先净额、realized 只出平仓腿）、`compatible_leg_orders`（:233，嵌入恒等重放器）、8 个 D 组单测（:306 起，含 D8 逐字节对拍锁）。
- `plan_and_fill_mtm_dual`（runner.rs:3190）+ `run_theta_v0_dual`（runner.rs:372）已接线：recognize_nested + DualLedger 分腿成交 + cascade 退出 + 毛闸门（`risk::gross_units_ok` 单源，runner.rs:3312-3316）。
- 可观测物证已存在：`FillOutputDual.leg_log: Vec<LegFillRec>`（runner.rs:3119-3140，逐腿成交日志：bar/depth/leg/close/成交手数/realized/双腿后态）。
- 生产因果接线在 `nautilus/strategy.rs:229-246`（`recognize_current` → `classify_with_tower` + `recognize_nested`，per-bar 窗口因果）。
- **有效域声明（照实）**：`run_theta_v0_dual` 带 `#[deprecated]`（runner.rs:366-369：结构确认前视，同 F-01 口径，产出禁用于 L2/L3 声明；因果验收依赖关①②串行链落地后的重放）。即 recognize 路径的真双开**引擎已实装、因果验收未闭环**。

### 2.2 coverage/overlay 生产路径（m8 E2E 主路径）：净额单仓单向 + 声部只读旁路

netting 审计 §1 已全链核实，本文复核关键锚点（2026-07-19 直读，行号一致）：

- **账户态 = 三个标量**：`cash/units/entry_cost`（runner.rs:1064-1066，`:1065`「p_t = 净 lot，有符号：正多/负空/0空仓」）——全账户单标量持仓，无腿维度。
- **唯一订单出口**：`p̃=net_target_units(&legs)`（coverage.rs:2307，自声明 :1614-1624）→ p*→`schedule_order(p*−p_t)`（coverage.rs:2692-2727）→ 每 bar 至多一张净额订单（runner.rs:1856-1863）→ 延迟成交计数（runner.rs:1191-1208，`:1204` n_orders_executed）。
- **声部簿 = 只读旁路**：`OverlayState`（overlay_state.rs:101-113）是 hedge-mode 逐声部簿（VoiceBook :59-79 / ClosedVoice :81-95），`ov.step` 产出只用于 ΔN 守恒 debug_assert（runner.rs:1555-1567 注释「只读旁路」），**不改 cash/units/trade_pnls**。
- **双开在净额上湮灭的对照测试**：overlay_state.rs:308-327 `hedged_two_voices_net_zero_but_book_nonzero`——父多 10+子空 10 ⟹ 净 N=0、order=0（净额账户不下单），账本记两声部。**账本双开、执行净额**——这正是任务书「voice 账本双开 vs 净额单仓单向」的实装坐实。
- **后果（netting 审计已证）**：fill 密度 5.4-12.4%/bar vs 声部事件率 ≈0.4%/bar（14-31×）；三窗 75629 张净额 fill vs 1554 声部 round-trip（48.7×）；Comm+Slip 是毛价格 PnL 的 11×/133×/14×（netting 审计 §5.2 表）。

### 2.3 观测层缺口（两路径共有/各有）

- coverage/overlay 路径：**无逐 bar q_v 轨迹落盘、无双开区间 dump、无 G/T/L 周转计数器**（netting 审计 §4.2 照实缺席：OverlayState 数据够算但不累计；OverlayRunResult 只带终态 overlay 簿，runner.rs:652-679；wverify 只消费 closed_voices 角色计数，wverify_run.rs:1281-1287）。
- recognize 路径：leg_log 已提供逐腿物证（§2.1），但无双开区间聚合视图（leg_log 是 fill 事件流，区间须派生）。
- **口径红线（090）**：观测 dump 的 voice-capture score（PDF p7 §6.2 `C^voice=ΣG_e`）是**诊断型语法覆盖 score，不是自融资 NAV**（多空对冲.pdf p7-8：「voice-capture score > 0 ⇏ self-financing NAV alpha > 0」）；落盘字段名与文档必须带诊断标签，禁冒充账户收益（overlay_state.rs:26-27 已有同款声明：pnl_v 是净额 PnL 的逐声部分解，不是独立 self-financing NAV）。

### 2.4 姊妹实装审计的双开区间实证（转引，口径照实）

`dual-open-implementation-audit-20260719.md` §2（数据 /tmp/m8_opsem_fixed/trades.jsonl，518 笔中 units>0 的 454 笔为主口径；pnl 逐 bar 归属=线性均摊近似）：

- **双开客观存在于账本层**：双开 bar 37750 个（占持仓 bar 19.09%，141 个连续段，最长 3373 bar）；最大并发声部 3。
- **净额化率**：双开 bar 上账本毛敞口均值 559.9 / 账户净敞口 249.0 ⟹ **净/毛=45.3%**——平均逾一半毛敞口在仓位层抵消；59.3% 的双开 bar 毛敞口>2×|净额|；**11.9% 的双开 bar |净|/毛<10%**（账户近空仓而账本双开满仓）。
- **双开配对结构（关键）**：共存反向声部对 125 对中 **0 对是赋格父子对冲**（父声部+子 ShortDiff 腿同时持仓）——36 对双方皆独立根、89 对一方带结构父容器（父容器是 Compose 身份，非交易声部）；247 笔 depth≥1 交易的 parent_id 全部不在账本内。即**当前双开 = StrategyFamily §5 多独立根型的跨时间持仓重叠，不是嵌套赋格父子交替双开**；同一决策点双触发由 `root_sel` 消歧为 Flat（voice.rs:265-272），双开只能跨时间形成。
- **双开区间 pnl 贡献为负**（两口径一致：−488.64 / −2772.10，均摊近似）——**这是账本归因读数，不是策略评估结论**（姊妹审计同款声明）；按 v3 禁令不作策略择优输入，仅作载体裁定的结构观察材料。
- 对 C1 设计的含义：C1 落地后，多独立根重叠型双开同样在 (Q⁺,Q⁻) 簿上共存（声部簿不区分根/子）；父子对冲型双开（0 例现状）的出现与否取决于嵌套声部产出（recognize_nested 因果验收 / coverage M29 腿），与载体裁定正交。

---

## 3. 修复方案（分支①：C1 hedge-mode 真实双仓，端到端）

**分支前提**：本节是裁定项 0 选 C1 时的设计。若裁定 C2，走 §4（观测补全）+ churn 修复（sizing 事件化，netting 审计 §6.1/§7.2 的 sizing 部分单独成关——注意 §7.2 伪码中 sizing 冻结与声部独立 fill 可分离，前者不依赖载体选择）；若裁定 C3，§7-0 仅有要点素描，须另立设计工位。

### 3.1 合并判定：与声部独立执行是同一改造，不是两个改造

三份文档的表面三个改造，实质是**一个 substrate 的三次落地**：

| 文档 | 对象 | 粒度 | 状态 |
|---|---|---|---|
| p120 §4（方案 B） | recognize 路径账户层 | (q⁺,q⁻) 双坐标 + (depth,leg) 腿标记（单脊柱） | **已实装**（§2.1），因果验收未闭环 |
| netting 审计 §7 | coverage/overlay 路径声部层 | ElementId 键控 VoiceBook（多声部，每声部单向） | 设计，未实装 |
| LEE §C | 级别维度 | (ℓ, 声部簿) 分组 + 事件钟 | 设计，M1-M4 迁移路径已排 |

合并论证（为什么是一个改造）：

1. **语义同一**：三者都兑现 PDF §10.2 (Q⁺,Q⁻)——p120 在账户坐标层，netting 审计在声部层，LEE 在级别分组层。每声部单向（σ_v∈{+1,−1}，overlay_state.rs:65-66）⟹ 声部簿的账户聚合 **Q⁺=Σ_{v:σ_v=+1} q_v、Q⁻=Σ_{v:σ_v=−1} q_v** 就是 DualLedger 的 (q_long, q_short)——**同一事实的两个粒度，必须单源派生**（声部簿为源，账户坐标为派生只读，禁双账本双写）。
2. **验收协议同一**：p120 §4.4 嵌入恒等（compatible_leg_orders 重放逐字节对拍，dual_ledger.rs:233 + D8 测试）与 netting 审计 §7.4 的「bit-exact 必须不变」清单是同一协议的两个表述；LEE M1/M2 的「加性细化 bit-exact」是同一协议的级别版。
3. **机制根因同一**：netting 审计 §5（每 bar 重定目标+净额单订单出口）与 LEE §B（统一 bar tick+净额混合）诊断的是同一个扁平化；修复也必须同点（执行层账本+事件化 sizing），只换账本不换 sizing 治不了 churn（netting 审计 §6.1，OverlayState 现状即反例）。

**结论：立项应为「真双开执行 substrate」一个改造**——吸收 netting 审计 §7 的声部独立执行臂设计作为主体，复用 p120 已实装的 DualLedger/apply_fill_dual 语义与嵌入恒等验收，LEE 作为后续 level 路由扩展按自身 M1-M4 接入。若立两个改造必然双源分叉（090 禁）。

### 3.2 账本层设计：声部簿为源，账户坐标派生

- **源账本**：每声部一行 `(voice_id=ElementId, σ_v, q_v, entry_cost_v, role_v, parent_v)`——直接升级 OverlayState 的 VoiceBook（overlay_state.rs:59-79）为**可写执行簿**（现状只读旁路，runner.rs:1555-1567）；加仓/减仓在**声部内净额**（同一 voice_id 的 q_v 是有符号单坐标：开仓后加/减/平都作用同一 q_v——「方向内净额」的精确定义）。
- **方向间不抵消**：不同 voice_id 的 q_v 永不互相湮灭——包括同方向不同声部（各自独立 round-trip）与反方向父子声部（父多子空并存即双开）。账户聚合 Q⁺/Q⁻ 只是 Σ 派生读数，**不参与 fill 决策**。
- **账户层**：DualLedger（dual_ledger.rs:40-49）作为账户坐标保留，但其 (q_long,q_short) 改为**从声部簿派生**（每 bar 末 `q_long=Σ_{σ=+}q_v, q_short=Σ_{σ=−}q_v`），不再独立维护——消灭 p120 单脊柱（depth↔腿 1:1）与 coverage 多声部（ElementId 键控）之间的粒度差。p120 §7 L3（多孩子分叉需 VoiceId 键控账户）随之解决：VoiceId 键控已在声部簿层兑现，账户层无需升 HashMap。
- **现金/费用单源**：cash 只有账户层一份（声部簿不复制现金）；fee 逐 fill 计账户层、按 voice_id 归因（新字段 `fee_v`，**禁复用 pnl_v 冒充费后**——netting 审计 §7.3 同款纪律；pnl_v 维持价格 PnL 口径，overlay_state.rs:77）。
- **realized 语义**：只在声部平仓（q_v→0 或部分平仓）时按该声部成本基产 realized——A′ 结算源（TW Realize 唯一合法资金源）按腿延伸，与 p120 §4.3/dual_ledger.rs:130 口径一致。

### 3.3 fill 设计：声部事件驱动

- **fill 触发 = 声部生命周期事件**（与 netting 审计 §7.2 伪码同源）：开仓（opened）、平仓（closed/silent_drops/risk_exits/overlay_closes）、可选结构 resize（默认空集）。无事件 bar ⟹ 零订单。
- **每声部独立 fill**：`VoiceOrder{voice_id, side=σ_v, qty, kind=Open/Close/Resize}`——订单携声部身份（现状 Order 三字段无身份，coverage.rs:2726；VoiceOrder 为新类型，不改 Order）。
- **成交语义复用**：`apply_fill_dual`（dual_ledger.rs:130）的分腿段规则逐字复用，腿键从 (depth,leg) 升到 voice_id；平仓 clamp 不借机开反向（close_only 语义保留）；现金约束沿用（开多需 cash≥含费成本；做空保证金未建模声明沿用，dual_ledger.rs:20-26）。
- **延迟成交/exec 机制不变**：VoiceOrder 进独立 `pending_voice` 队列，同一 exec_index 延迟成交框架（runner.rs:1191-1208 同型）——执行时序因果不变。

### 3.4 sizing 设计：事件化冻结 + 独立 sizing

- **开仓时刻冻结**（netting 审计 §6.1）：`q_v = round(base_units(entry_bar)×w_depth×w_dir / lot)×lot`，开仓时定死；`base_units` 从「每 bar 协变量」（runner.rs:1279）降级为「开仓时刻快照」。**不改 coverage.rs:1528/1552**（决策层目标两臂共享）——冻结发生在执行臂落簿时。
- **独立 sizing**：每声部 sizing 只依赖自身 (entry_bar, w_depth, w_dir, lot) 与父约束（parent_cap，risk.rs:257——p120 已激活，runner.rs:3177-3178）；存续期默认不重定；可选结构事件 resize（级别/角色变化）为显式参数，默认关。
- **sizing 哲学张力如实登记**（不裁）：w=[0.60,0.30,0.10] depth 权重 + β=0.5 parent_cap vs M15 同单位数 κ=1 vs 026:80 的 f=1/3 机动份额——p120 §7 L4 已登记；真双开落地使该张力表面化（双开两腿 sizing 独立后，w=0.30/0.10 非同单位双开 vs κ=1 同单位双开的费用/保证金后果真实化），列 §7 裁定材料。

### 3.5 风控设计：独立风控 + 双读数

- **毛闸门**：`risk::gross_units_ok`（risk.rs:572）单源复用（p120 已接线，runner.rs:3312-3316）——Σq_v ≤ γ̄·base_units，超限拒当步边际开仓（fail-closed，不缩放既有腿）。strict §11「净约束不能替代毛约束」在真双开下首次真实生效（毛敞口 = Q⁺+Q⁻ 真实化）。
- **净敞口降级为派生只读**：N_derived = Q⁺−Q⁻（= Σσ_v q_v），喂 `k_theta_risk_gate`（runner.rs:1281 同口径）与 cap 判据——与 netting 审计 §7.1「N_derived 只读派生」一致；风控门保持**每 bar 生效**（LEE M3 风险点同款：事件门控只门控结构交易，不门控风控）。
- **margin/accrual 口径（诚实近似声明）**：maint_margin/funding/borrow 现状以 |N| 为基；双腿共存时 |net| 基是近似（p120 §4.5 表同款声明；per-leg 精化 borrow 基=空腿名义/funding 基=毛额列遗留，涉成本数字变更须独立裁定——§7 裁定材料①）。
- **双读数分列（090）**：`n_orders`（净额臂）与 `n_voice_fills`（声部臂）两读数分列，禁互相冒充（netting 审计 §7.5 同款；OverlayRunResult.n_overlay_fill_events 的声明口径 runner.rs:656-659 同步修订）。

### 3.6 与 LEE 的合并关系（任务①第二问：每级每方向独立仓位？）

- **不需要"每级每方向独立仓位槽"这个对象**。LEE 的 Ledger_ℓ 是「在该级开仓的声部」的账本（LEE §C 支柱1：键=formation_level=ℓ 的 BSP 开出的声部）——账本内部就是声部簿，方向由 σ_v 承载。正确形态 = **(ℓ, voice) 二维键控的 hedge-mode 簿**：级别是声部的路由索引（BspKey.formation_level 已有，roadmap:64），不是与方向平行的仓位维度。每格单向；方向间不抵消（声部簿语义）；级别间不抵消（LEE 级别封闭不变量：跨级影响必须显式跨级消息落账，不得经净额隐式传导）。
- **合并次序**：本方案（声部独立执行，单级平坦簿）先行 ⟹ LEE M1（级别账本旁路，按 formation_level 分桶只读镜像，bit-exact）可**并行**（它不改主路径）；LEE M2（订单归因 Σ_ℓΔq_ℓ==ΔN）在本方案落地后自然成立（Σ_ℓΣ_v Δq_v 重排）；LEE M3（事件门控）是声部事件驱动的级别稀疏化——本方案的 fill 触发集是 LEE M3 触发集的超集（LEE 额外要求同 bar 无 ℓ 级事件时该级零订单；本方案的声部事件本身就是级别事件，二者在「事件=结构事件」处合一）。**结论：本方案是 LEE 的执行层底座，LEE 是其级别路由扩展——同一改造的分期，非两个改造。**
- **跨级赋格边界沿用 LEE §E**：跨级影响走谱系显式边（ProjectionKey/Closed 谱系），不走净额隐式抵消——真双开 substrate 恰好提供了「不经净额传导」的物理保证（净额已不再驱动订单）。

### 3.7 前/后伪码

**前（coverage/overlay 生产臂现状，runner.rs:1279/:1462-1476/:1555-1567/:1856-1863）**：

```
每 bar i:
    base_units = equity_nav / px                    // 每 bar 重算（runner.rs:1279）
    (next_active, p*, order, step_trace) = pi_theta_step_traced(...)
        // legs: units_v = base_units×w_depth×w_dir（coverage.rs:1528/1552）每 bar 漂移
        // p̃ = net_target_units(&legs)（coverage.rs:2307）← 先求和消维，双开湮灭
        // order = Schedule_Θ(p* − p_t)（coverage.rs:2692）← 唯一净额出口，无 voice_id
    ov.step(sep_legs) 只读旁路（runner.rs:1555-1567） // 声部簿记两声部但不下单
    if order.qty > 0: pending[exec_index].push(order) // 每 bar 至多一张净额订单
成交 bar i:
    apply_order(o, px, fee_rate, &mut cash, &mut units, ...) // 单标量；先平后开（父腿身份被拆）
```

**后（声部执行臂 `pi_theta_fill_loop_voice`，净额臂逐字节不动）**：

```
每 bar i:
    (next_active, _p*, _忽略净额order, step_trace) = pi_theta_step_traced(... 同参 ...)
        // 决策轨迹与净额臂同源同参 ⟹ typed_ledger/TW/sep_legs bit-exact
    voice_orders = []
    for leg in step_trace.opened:                       // 事件①开仓
        q_v = round(base_units_i×w_depth(leg)×w_dir(leg)/lot)×lot   // ★开仓时刻冻结
        if q_v ≥ lot ∧ gross_units_ok(Σq+q_v, base_units_i, γ̄):     // 毛闸门单源
            voice_orders.push(VoiceOrder{voice: leg.id, side: σ_v, qty: q_v, kind: Open})
    for leg in step_trace.closed+silent_drops+risk_exits+overlay_closes:  // 事件②平仓
        if book = books.get(leg.id):
            voice_orders.push(VoiceOrder{voice: leg.id, side: −σ_v, qty: book.q, kind: Close})
    for ev in structural_resize_events:                 // 事件③结构 resize（默认空集）
        ...
    for vo in voice_orders: pending_voice[exec_index].push(vo)   // 无事件 ⟹ 零订单
成交 bar i:
    for vo in pending_voice[i]:
        book = books.entry(vo.voice)                    // 声部内净额（同一 q_v 单坐标）
        fill = apply_fill_dual_vo(vo, px, fee_rate, &mut cash, &mut book)  // 复用 dual_ledger.rs:130 段规则
        n_voice_fills += (fill.executed_qty > 0); fee_v += fill.fee  // 费用按声部归因
        if book.q == 0: settle → ClosedVoice（realized 冻结，TW Realize 按腿延伸）
    Q⁺ = Σ_{σ=+}q_v; Q⁻ = Σ_{σ=−}q_v                    // DualLedger 派生只读（§3.2）
    N_derived = Q⁺ − Q⁻  // 只读派生，喂风控门/cap；不再驱动订单
    assert Σ_v Δq_v == ΔN_derived                       // ΔN 守恒新形式（臂内）
    assert Σ_v pnl_v ≈ Σ_t N_derived,t·ΔP_t  (eps)      // PDF §11 臂内对账
```

**recognize 路径（已实装，本方案不动其引擎）**：`plan_and_fill_mtm_dual`（runner.rs:3190）已是「后」形态的单脊柱版；本方案对 recognize 路径的增量只有两点（均列遗留，不在本方案实装范围）：①因果验收重放（依赖关①②串行链，p120 §2 依赖层）；②账户层 (q_long,q_short) 切派生读数以与声部簿单源（§3.2——recognize 路径现状是 DualLedger 独立维护，两路径统一 substrate 时对齐）。

### 3.8 落点（行号锚，只设计不实装）

| 改动 | 落点 | 说明 |
|---|---|---|
| 新臂 `pi_theta_fill_loop_voice` | runner.rs `pi_theta_fill_loop_overlay` 旁（:1041 区域） | 共享 classify/风控/TW/opsem 上游；分叉点在 :1462 之后，消费同一 step_trace、忽略净额 order；`overlay: None` 路径逐字节不变（净额臂回归锁） |
| `VoiceOrder` 类型 | types.rs Order 旁 | = Order + voice_id + kind；不改 Order（coverage.rs:2726） |
| 声部执行簿 | 升级 overlay_state.rs:101-113 OverlayState | 只读旁路→可写执行簿；加 `fee_v` 字段（禁复用 pnl_v 冒充费后）；VoiceBook/ClosedVoice 形状（:59-95）保留 |
| 账户坐标派生 | dual_ledger.rs:57-67 同型函数 | Q⁺/Q⁻/N_derived 从声部簿 Σ 派生；DualLedger 独立维护路径（recognize 侧）统一时对齐 |
| sizing 冻结 | 新臂落簿点 | 只消费 step_trace.opened 当 bar 的 base_units 快照；不改 runner.rs:1279、不改 coverage.rs:1528/1552 |
| pending/fill 循环 | runner.rs:1191-1208 同型 | 独立 pending_voice 队列 + per-voice apply_fill 包装（复用 dual_ledger.rs:130 段规则）；净额臂原队列不动 |
| 观测 dump | FillOutput 装配（runner.rs:1997-2007 旁）+ wverify 报告层 | §5.4 验收计数器 + §4 双开区间/贡献 dump |
| 声明口径修订 | runner.rs:656-659（n_overlay_fill_events 注释） | 双读数分列（净额 fill 数 / 声部 fill 数），090 三态一致 |

---

## 4. 分支②对照：若裁定 C2（净额+内部记账），观测 dump 补全设计

本文推荐 C1（§1.4），分支②是裁定项 0 选 C2 时的对照方案。若编排者裁定 C2（净额单仓+声部旁路为合法载体），则当前实装账面符合（§1.4 对照表），偏离只在观测层与 churn 附加物，补全设计如下（全部为只读派生，不改执行路径，天然 bit-exact；churn 的 sizing 事件化修复见 §3 分支前提注，可独立成关）：

1. **双开区间 dump**：从 OverlayState 逐 bar 簿派生 `DualOpenInterval{t_open, t_close, long_voices: Vec<ElementId>, short_voices: Vec<ElementId>, q⁺_path, q⁻_path, gross_path, net_path}`——区间 = 同一 bar 上 ∃v,w: σ_v=+1∧σ_w=−1∧q_v,q_w>0 的极大连续段。OverlayState 已有全部数据（books 逐 bar 状态），缺逐 bar 快照落盘（netting 审计 §4.2：数据够算但不累计）——新增只读 recorder，不动 step 语义。
2. **双开贡献 dump**：每区间三量——`hedge_netted_away = G−T`（区间声部毛周转 − 净周转，量对冲节省）、`voice_shred = G−L`（切碎量）、`voice_capture = Σ G_e`（PDF p7 §6.2 诊断型 score，字段名强制带 `_diagnostic` 后缀 + 报告注明「非自融资 NAV」，§2.3 口径红线）。
3. **计数器落地**：netting 审计 §7.4 的四个验收量（gross_voice_turnover/net_turnover_filled/hedge_netted_away/voice_shred）+ 断言 `n_orders ≥ L/2` 型 sanity——全部为 OverlayState 旁路累计，零执行路径改动。
4. **与 C1 方案的关系**：这些 dump 在 C1 方案下**同样需要**（§5.4 验收计数器同源）——观测补全不是分支②专属，是两个分支的公共底座；差别只在执行层动不动。因此观测 dump 可先行落地（独立小关，bit-exact 零风险），载体裁定后只决定执行层走不走 §3。

---

## 5. bit-exact 影响分析与验收协议

### 5.1 必须不变（新臂 vs 净额臂同窗同参对跑）

决策层与旧臂全量（与 netting 审计 §7.4 一致，逐条可断）：

1. `typed_ledger` 全序列逐行一致（决策同源直接推论）；
2. TW 账本事件序列与终态 `tw_final`（runner.rs:2005 区域；A′：TW Realize 资金源按腿延伸后总量在嵌入重放下相等，§5.3-3）；
3. `step_trace.sep_legs` 逐 bar 序列（决策层产，执行臂不反馈）；
4. 分类层全部（classifier 零触碰，构造性）；
5. **净额臂自身全部产出**（`overlay=None`/现有臂路径，runner.rs:1039-1040 注释的既有承诺——新臂并列新增，旧路径零改）；
6. 三条恒等（臂内，逐 bar 可断）：
   - **ΔN 守恒新形式**：Σ_v Δq_{v,t} ≡ ΔN_derived,t（overlay_state.rs:13-18 三恒等的执行版）；
   - **PDF §11 臂内对账**：Σ_v pnl_v == Σ_t N_derived,t·ΔP_t（|diff|<eps，f64 结合律容差，overlay_state.rs:331-350 同型测试）；
   - **嵌入恒等**：声部事件空集（或 nested 未触发数据）下，新臂经兼容重放与净额臂 cash/equity/realized/fee/trade_pnls **逐字节相等**（p120 §4.4 定理 + dual_ledger.rs:233 compatible_leg_orders + D8/E4 对拍模式复用）。

### 5.2 设计性改变（非回归，须重出报告，新旧读数禁混排）

1. **订单流**：fill 时点/数量——事件驱动上界 `n_voice_fills ≤ 2×n_voices + n_resize_events`（构造性可断，netting 审计 §6.2）；量级读数（非承诺）：订单数降 ≥24×、佣金降 1-2 个数量级，对冲毛额化部分抵消（netting 审计 §6.3 区间论证）。
2. **N_derived 轨迹**：sizing 事件化冻结 ⟹ N′_t ≠ 净额臂 N_t（净额臂每 bar 重定目标）。**这是设计意图**：N′ 是事件驱动持仓的净投影，N 是每 bar 重定目标的净投影；两者都是合法对象，禁互称 bug。
3. **费用路径**：cash/equity_curve 改变（佣金 ∝ 周转，周转从「净周转+churn」变「声部毛周转」）；r_decomp 的 Comm+Slip 行改变；守恒断言（runner.rs:1990-1994 区域）在新臂必须同样成立（Σ_v ledger_delta_v == Σ_v(price_pnl_v − fee_v)，残差容差同款）。
4. **结算粒度**：trade_pnls 按声部 round-trip 结算（条数≈声部数），不再按 fill 切碎（F-06 部分减仓逐 fill 产 TradeRecord 的形态在声部簿内保留为腿级事件，账户层贸易配对按 (voice_id) 键）。
5. **风控生效域**：毛闸门首次真实约束（毛敞口真实化）；极端路径下新臂可能拒单而净额臂成交——差异属 fail-closed 设计意图，须逐例归因报告。

### 5.3 明文禁设的不变量（常见误设，验收协议预先排除）

1. **禁设「跨臂毛价格 PnL 相等」**：PDF §11 线性恒等 `Q⁺ΔP−Q⁻ΔP=(Q⁺−Q⁻)ΔP` 只在**同一持仓轨迹**下成立；sizing 事件化后 N′≠N ⟹ 两臂毛价格 PnL 不必相等。验收协议若误设此条，会把设计性改变（§5.2-2）误判为回归。恒等的正确用法是**臂内**（§5.1-6）。
2. **禁设「双开区间费用为零」**：真双开下对冲两腿毛额付费（netting 审计 §6.3 反向修正已诚实计入）；同 bar 反向事件的净额节省 (G−T) 被毛额化回来是设计代价，不是异常。
3. **禁设「TW 事件序列逐字节不变且资金源口径不变」**：TW 事件序列不变（§5.1-2），但 ShortDiff 通道的资金源从「净额 realized」延伸为「平仓腿 realized」（A′ 按腿延伸）——嵌入重放下总量相等，双开真实发生后口径是 M14 语义的忠实化（p120 §4.5 表「语义差」行同款声明），报告须标注。
4. **禁把 voice-capture score 当 alpha 验收**：§2.3 口径红线；任何 `C^voice>0` 读数不是 NAV alpha 证据（多空对冲.pdf p7-8）。

### 5.4 验收矩阵（落地时逐行执行）

| # | 验收 | 判据 | 性质 |
|---|---|---|---|
| V1 | 净额臂回归 | `cargo test --lib` 既有测试零非授权变红；现有臂全基线 diff=0 | 构造性（旧路径零改）+重放复验 |
| V2 | 决策同源 | 新臂 vs 净额臂：typed_ledger/TW/sep_legs 逐行 diff=0 | bit-exact |
| V3 | ΔN 守恒 | 逐 bar Σ_v Δq_v == ΔN_derived，零违例 | 构造性断言 |
| V4 | §11 臂内对账 | Σ_v pnl_v − Σ N′ΔP < eps（全程 max 残差落盘） | 线性恒等 |
| V5 | 嵌入恒等 | 事件空集夹具：新臂 == 净额臂逐字节（D8/E4 模式） | 对拍 |
| V6 | 事件上界 | n_voice_fills ≤ 2×n_voices + n_resize_events | 构造性上界 |
| V7 | 周转计数器 | G/T/L/hedge_netted_away/voice_shred 五量落盘（netting 审计 §4.2 缺席补全） | 观测 |
| V8 | 双开区间 dump | §4-1 区间表落盘；每区间毛/净路径可重放 | 观测 |
| V9 | R 分解守恒 | 新臂 Σ ledger_delta == Σ(price_pnl − fee)，残差容差同净额臂 | 账本恒等 |
| V10 | 现金守恒 | cash_final == nav0 + Σrealized − Σfee（E1 模式，合成夹具四步 1-cycle） | 对拍 |

---

## 6. 风险声明

1. **venue 前提**：q_short 是真空头（期货/永续域）；现货标的须部署层 gate=0（dual_ledger.rs:20-26 声明沿用）；venue 不支持 hedge-mode 时 (Q⁺,Q⁻) 不可共存——venue 适配是部署裁定（p120 §7 L7），引擎层不区分标的。
2. **保证金/accrual 近似**：双腿共存时 |net| 基 maint_margin/funding/borrow 是近似口径（§3.5）；per-leg 精化涉成本数字变更，须独立裁定，不得顺手改（p120 §7 L6 同款）。
3. **recognize 路径因果未闭环**：run_theta_v0_dual deprecated（runner.rs:366-369，前视口径）；nautilus recognize_current（:229-246）是因果接线但其嵌套产量验收依赖关①②串行链（p120 §2 依赖层）——真双开的 recognize 侧证据等级不得超此声明。
4. **churn 修复量级是机制论证非承诺**：订单降 ≥24× 是构造性上界（§5.2-1）；佣金降 1-2 个数量级是恒等式+显式假设区间的量级估算（费率 3bps 未标定，config.rs:222-235，禁作策略择优输入）。
5. **alpha 零承诺**：§1.5 判定边界；C1 修的是结构保真度，净值后果是 L2/L3 另案且不得以回测定优劣。
6. **双源风险**：若 recognize 路径 DualLedger（独立维护）与 coverage 路径声部簿（派生）长期并存而不统一（§3.2），两路径账户坐标口径分叉——统一 substrate 是本方案的一部分，不得分期成「各自为政」。
7. **LEE 并发风险**：LEE M1（只读级别镜像）可并行；LEE M3（事件门控）必须排在本方案验收之后（其订单流分叉叠加在本方案之上，双层分叉须逐层验收，禁合并评审）。

## 7. 编排者裁定材料

**裁定项 0（前置）：物质化载体三选一 C1/C2/C3**——语义核心（语法层 L0 分账本声部结构）三方无争议（§1.0）；有分歧的是载体。材料清单：

| 材料 | 内容 | 指向 |
|---|---|---|
| FULL 规格 §8/§9（canonical-coverage-pdf-package.md:150 对照表 §9 行 516-601） | 双开定义锚 `Q⁺>0∧Q⁻>0` 同存、同单位数 `a_v=1⟹q_v=q_p(v)` | C1（规格首选） |
| 多空对冲.pdf p11-12 §10.1-10.2 / p14 §11 | 净额压缩=「核心问题」；hedge mode 五项执行层收益；分账本语法层成立 | C1 倾向（但 PDF 不强制载体） |
| nested_fugue_accounting.md:240-244（§10）/ :228 / §8.4 | 「逐仓独立 position **或**净额+内部记账（如果交易所不支持）」明文两读；C3 冻结-capital 会计 | C2 为 venue 受限降级；C3 为赋格会计精化 |
| Strict/StrategyFamily.lean:561-589 | `long_short_both_open_allowed` 证结构可表达、**不证可行性**（EmpiricalDomain）——不指定载体 | 中性（三方共读，读法分歧见 §1.5 第 3 条登记） |
| p120-nested-voice-dual-ledger-design-20260718.md §4 + 已实装 dual_ledger.rs | 编排者先例：recognize 路径已按 C1 实装 | C1（推翻需新裁定） |
| coverage.rs:1624 自声明 | 「完整毛分账本执行须 hedging 账户（v0 净额）」 | C1（现状自认停损点） |
| 本文 §1.4 工程论据 + 三载体对照表 | 现状只账面符合 C2 且带 churn 附加物；不符合 C1/C3 的完整语义 | C1 推荐 |
| dual-open-design-semantics-20260719.md §2/§7 | 分层判定「既不是无条件真双仓，也不是无代价净额+旁路」 | 分层框架（本文 §1.0 已吸收） |
| dual-open-implementation-audit-20260719.md §3/§4 | 按 C2 本义口径评现状「无偏离」；其「真双开解读与 Lean docstring 矛盾」主张 | C2 口径（与本文 §1.5-3 的核对结论分歧，原样并列） |
| 实证读数（本文 §2.4 转引） | 双开 bar 19.09%、净/毛 45.3%、11.9% 近全对冲、0 对赋格父子、双开区间 pnl 归因负 | 结构观察（禁作策略择优输入） |

**本文推荐 C1**（论据 §1.4）。若裁定 C2：执行层只走 §4 观测补全 + churn sizing 修复（独立成关）；若裁定 C3：须另立设计工位——要点素描：物理交易保持净额（父真实减仓释放现金），子空头 = 父冻结 capital 上的账本腿（非逐市、递延到回补结算、无独立 MtM 负债），须新建双层记账守恒（Σunits=N_base 会计守恒 + 空头冻结 capital 通道 + TW ShortDiff 划转从 |units| 基改为冻结-capital 基）——**本文不展开为方案**（090：声明=能力，C3 的完整设计超出本工位已核实材料）。

**语义判定复核摘要（本文判定）**：语法层无条件成立（§1.0 第一层）；载体层推荐 C1（§1.4）；对「C2 即可无需裁定」读法的驳斥见 §1.3；对姊妹审计 Lean 读法分歧的核对见 §1.5 第 3 条。

**其余需裁定的五点**：

1. **margin/accrual 口径**：毛敞口 Q⁺+Q⁻ vs 净 |N| 为基（CME-simple 是否按毛腿计）；per-leg 精化（borrow 基=空腿名义、funding 基=毛额）是否立项（netting 审计 §8 + p120 §7 L6 同点）。C2/C3 下载体裁定后此题同样存在（funding/borrow 现状按 |units| 净额名义，runner.rs:1214-1223）。
2. **两路径验收次序**（仅 C1 分支）：(a) coverage/overlay 生产臂先落地（本方案 §3，m8 报告路径直接受益）；还是 (b) recognize 路径因果重放先闭环（依赖关①②，p120 §2）；还是 (c) 统一 substrate 先行（§3.2 派生化改造，两路径后接）。本文倾向 (c)→(a)，(b) 随关①②节奏。
3. **sizing 哲学**：w_depth=[0.60,0.30,0.10]+β=0.5 vs M15 κ=1 同单位数 vs f=1/3 机动份额（p120 §7 L4 登记不裁项）——C1 落地使其表面化，须在 sizing 冻结语义定稿前裁定，否则 q_v 冻结值的口径悬空。
4. **venue hedge-mode 部署前提**（p120 §7 L7；C2 恰好是 venue 不支持时的降级路径——venue 事实核查是裁定项 0 的输入）。
5. **LEE 合并时机**：本方案验收后接 LEE M1-M4（推荐）；或 LEE M1（只读镜像）并行先行（bit-exact 零风险，可批）。禁 LEE M3 与本方案合并评审（§6-7）。

## 8. 锚索引

**PDF 锚**（pdftotext 逐页直读，2026-07-19）：多空对冲.pdf p1-2 §1（净额映射与核 ker N）/ p3-4 §3（§9 双开抵消定理精确含义）/ p7-8 §6.2-§7（voice-capture score 诊断型，非 NAV）/ p9 §8（alpha 还是风险转换）/ p10-11 §9（三层判定表）/ p11 §10.1（one-way/netting，「这正是你现在遇到的核心问题」）/ p12 §10.2-10.3（hedge mode (Q⁺,Q⁻)；三账户模型对照表）/ p13-14 §11（最终严格结论：分账本语法层成立 + ΔN 判据）。子声部.pdf p16 §5（position voice v=(c,γ,σ,n)）/ p16-17 §6-7（σ_u=−σ_v；AncOK 过滤器）/ p23 §16（全互斥解释器 + LegTarget 决策链）/ p24-25 §19（双视图架构最终建议）。

**代码锚**（worktree 现行工作树直读）：净额执行链 runner.rs:1064-1066（标量账户）/:1187（cum_price_pnl 逐 bar MtM）/:1214-1223（funding/borrow 按 |units| 净额名义）/:1234（TW ShortDiff 划转 |units| 基）/:1279（base_units 每 bar 重算）/:1462-1476（step_traced）/:1555-1567（overlay 只读旁路+ΔN 断言）/:1856-1863（单订单挂单）/:1191-1208（延迟成交+:1204 计数）；决策层 coverage.rs:1528/:1552（units=base_units×w）/:1614-1624（net_target_units 自声明「完整毛分账本执行须 hedging 账户」）/:2307（p̃）/:2692-2727（schedule_order）；声部簿 overlay_state.rs:13-18（三恒等）/:59-95（VoiceBook/ClosedVoice）/:101-113（OverlayState）/:308-327（双开净零对照测试）/:331-350（§11 对账测试）；方向消歧 voice.rs:265-272（root_sel，(1,1)→Flat）；真双开已实装 dual_ledger.rs:1-27（模块声明）/:40-49（DualLedger）/:57/:62/:67（net/gross/equity）/:130（apply_fill_dual）/:233（compatible_leg_orders）/:306-448（D 组 8 测）；runner.rs:372（run_theta_v0_dual，:366-369 deprecated 声明）/:3190（plan_and_fill_mtm_dual）/:3119-3140（FillOutputDual/leg_log）/:3177-3178+3312-3316（毛闸门）；nautilus/strategy.rs:229-246（recognize_current→recognize_nested）；口径字段 runner.rs:70（n_orders）/:652-679（OverlayRunResult，:656-659 声明）/:766（n_overlay_voices）；wverify_run.rs:1281-1287（closed_voices 角色计数）；risk.rs:257（parent_cap）/:572（gross_units_ok）；config.rs:222-235（fee 3bps）。

**文档锚**：netting-vs-voice-execution-audit-20260719.md §1（净额机制全链）/§3（设计对照表）/§4.1-4.2（分解恒等式+缺席声明）/§5（churn 机制）/§6.1（只换账本不治 churn）/§6.2-6.3（量级估算）/§7（声部独立执行设计）/§7.4（验收量）/§8（margin 待裁）；multi-level-native-execution-design-20260719.md §B（扁平化两根因）/§C（LEE 四支柱+不变量）/§D（M1-M4 迁移）/§E（级内/跨级赋格）/§F-2（优劣判据=结构保真度）；p120-nested-voice-dual-ledger-design-20260718.md §1.3（净额翻转根因）/§4（方案 B 双账本）/§4.4（嵌入恒等）/§4.5（下游对账表）/§7（L3/L4/L6/L7 遗留）；m8-opsem-vs-norders-recon-20260719.md（48.7× 同口径比）；dual-open-design-semantics-20260719.md §2/:115-124（分层判定+三表述）/§7/:180-186（总结论：语义=structure concurrency，载体二选一）；dual-open-implementation-audit-20260719.md §1（实装机制）/§2（双开区间实证：37750 bar/45.3%/0 对赋格父子）/§3-4（C2 口径「无偏离」评级+Lean 读法主张）；docs/nested_fugue_accounting.md:228（C3 冻结-capital）/:240-244（§10 两读）/§8.4（空头非逐市）；docs/canonical-coverage-pdf-package.md:150（规格 §9 锚 Q⁺>0∧Q⁻>0）；formal/Strict/StrategyFamily.lean:561-589（long_short_both_open_allowed：结构允许≠可行）。

---

## 签字位

- [x] 设计语义判定：分层结构（语法层无条件 / 载体层三候选 C1-C2-C3 / 经济层无条件定理）+ 推荐 C1 的工程论据 + 三载体对照表（§1）
- [x] 实装审计：recognize 路径 C1 已实装（p120 兑现核实）/ coverage 路径净额单仓单向 + 姊妹审计实证转引（§2）
- [x] 修复方案（C1 分支）：与声部独立执行=同一改造判定 + 前/后伪码 + 行号落点（§3）
- [x] LEE 合并关系：(ℓ,voice) 二维键控，无需「每级每方向仓位槽」，合并次序定（§3.6）
- [x] 分支②对照（C2）：观测 dump 补全设计（公共底座，可先行，§4）
- [x] bit-exact 验收协议：6 条必须不变 + 5 条设计性改变 + 4 条禁设误设 + V1-V10 矩阵（§5）
- [x] 风险声明 7 条（§6）
- [x] 裁定材料：裁定项 0（载体三选一，10 项材料表 + C3 要点素描不展开）+ 复核摘要 + 其余 5 点（§7）
- [ ] 编排者裁定（空位：裁定项 0 载体三选一 + §7 五点）
