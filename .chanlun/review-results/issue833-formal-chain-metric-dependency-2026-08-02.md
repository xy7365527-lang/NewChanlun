# #833 换验收口径后形式化链的依赖清点（2026-08-02）

只读调研。对象：`docs/formal-chain/` 全 44 份 PDF + `formal/` 142 个 `.lean`（票面写 149，实测 `find formal -name "*.lean" | wc -l` = 142）。

## 零、先订正票面的页码/章节号

`docs/formal-chain/买卖点alpha2.pdf` 共 38 页，是**三段独立 ChatGPT 答复**首尾相接，每段各自从 §1 重新编号：

| 段 | PDF 页 | 章节范围 |
|---|---|---|
| 答复A | p1–p13 | §1–§15（Eat(e) 覆盖 vs alpha） |
| 答复B | p14–p25 | §1–§15（交易问题重定义 + 买卖点状态机） |
| 答复C | p26–p37（p38 空白） | §1–§19（全互斥策略与 alpha） |

对票面四条：

- **「§10 / §14 = 净值 Sharpe 判据出处」** —— 三段各有一份，票面没写是哪段。实际出处有四处：答复A §12（p9–10，L0/L1/L2/L3 四层，L3 判据 = `E[ΔN·ΔP − ΔC] > 0` 且 `ΔSharpe>0, ΔIR>0, ΔMDD<0, ΔUtility>0`）、答复A §14（p11）、答复B §10（p20–21）、答复C §11（p32）。**最完整、最像"判据定义"的是答复A §12（p9–10）**，其余三处是它的重述。
- **「§12–14 = 按声部证书求和那套」** = 答复B §12–§14 = **p22–p24**。
- **「p22–24 = 依赖级别间会计独立那套」** = **同一段**。票面这两条指向同一处推导，不是两处。
- **「p33–37 = 按净额状态那套」** = 答复C §12–§19 = p33–p37。准确说是"按**全互斥状态 Z_t** 分解"，不是按净额。

---

## 甲、只在净值口径下成立的推导

### 甲-1 `买卖点alpha2.pdf` p9–10（答复A §12）四层验收 L0/L1/L2/L3

断言：L2 = 「分账本腿确实进入账户净敞口」，指标 `Σ_t |ΔN_t| > 0`；L3 = 「真正有交易价值」，指标 `E[ΔN_t·ΔP_{t+1} − ΔC_t] > 0` + `ΔSharpe>0 / ΔIR>0 / ΔMDD<0 / ΔUtility>0`。

用到净值的那一步：L2 整条以 `ΔN ≠ 0` 为"可见性"门槛。

换成本口径后为什么不成立：闸门前持仓量锁死 M=N，降成本短差按定义是同价 free⇄holding 转换（`formal/Origin/TotalWealth.lean:159-161` 的 `TWEvent.shortDiff`），**ΔN 恒为 0 而 basis 严格下降**。L2 把"得分方式"直接判成"不可见"。L3 的 ΔSharpe 分母是净值曲线波动，成本口径下没有这条曲线。

### 甲-2 `买卖点alpha2.pdf` p21（答复B §10 末）`ΔN_t = 0 ⟹ ΔSharpe = 0`

这是本仓**全部"短差无价值"否证的推理引擎**。同一命题在 `多空对冲.pdf` p2–p3 被升级为「净额 NAV 不可识别定理」：两个分账本头寸过程只要净头寸相同、净额成本相同，则「净额收益序列、Sharpe、最大回撤等所有只依赖净额 NAV 的指标都相同」（p2 原文）。

换成本口径后：`ΔN=0` 的同价短差**正是**降成本的标准形态，成本口径下它非但不是零信息，它就是全部信息。这条从"定理"降为"关于某个不再使用的度量的事实"。

### 甲-3 `买卖点alpha2.pdf` p5–p7（答复A §5–§8）父子双开抵消定理

断言：父腿 `+Q`、子腿 `−H`，`H=Q ⟹ N=0 ⟹ NΔP=0`，扣成本 `−C < 0`；故"全覆盖不盈利"。

换成本口径后不成立：抵消的前提是**父腿也按市价结算**。成本口径下父腿不平仓就不产生已实现盈亏，basis 不动；子腿平仓的已实现盈亏直接冲减父腿 basis。"子腿赚多少父腿亏多少"退化为"子腿赚多少 basis 降多少，父腿什么也没发生"。

### 甲-4 `formal/Origin/NetValueImpossibility.lean` 全文件（244 行，C22/W4）

文件自己在 `:10-11` 和 `:34` 明写有效域是"**组合净值坐标**"。核心对象 `Leg.G = q·ε·(pRho − pLam)`（`:99`）是**未平仓的方向化毛收益**（mark-to-market），不是已实现盈亏。受影响的定理：

- `:141 net_value_cancels` —— `G_parent + G_child = 0`
- `:152 child_gain_is_parent_loss` —— `G_child = −G_parent`
- `:161 general_net_zero`
- `:182 reverse_stroke_short_positive_long_negative`
- `:225 same_unit_forbids_net_profit`
- `:237 net_value_eat_every_stroke_impossible`

换成本口径后为什么不成立：按 T₃₃（`formal/Tlayers/Accounting/Ledger.lean:78 trade_dual_view_same_source`），父子两腿是**同一物理量 m 的两个会计视图**，不是两个可加的净值分量；子腿平仓的已实现盈亏走 `costReduction` 分量（`formal/Tlayers/Accounting/Earning.lean:117 childPnL`）冲减父腿成本。代数恒等 `G_p + G_c = 0` 仍是真的整数算式，但它算的量在新尺子下不参与打分。

**这是本次换口径影响最大的一条。**

### 甲-5 `formal/Origin/SeparateFinalTheorem.lean` §3「C40 防火墙」及顶点有效域声明

- `:274 net_profit_every_stroke_impossible`（真引用甲-4）
- `:84-86` 文件头："C40/W4 净收益不可能定理是**防火墙**——它封死了把 L0 声部级覆盖膨胀为 L2 实盘每笔盈利的有效域越界"
- `:429-435` 诚实边界表同款表述

这道"防火墙"整体建在净值坐标上。换尺子后它不再是有效的边界声明——它防的那个越界方向（"L0 覆盖 ⟹ 净账户每笔盈利"）在新口径下不是待防的命题。

### 甲-6 `买卖点alpha2.pdf` p22–p24（答复B §12–§14）声部证书求和 + 正收益定理

断言：`X_γ = δ(P_τ − P_t) − C_{t:τ}`；`μ_{ℓ,δ,I} = E[X_γ | ℓ, δ, I_γ]`；若 `E[X_γ|F_t] ≥ η > 0` 则 `E[R] = E[Σ_{γ∈T} X_γ] ≥ η·E[|T|] > 0`；方差有限则 `Sharpe(R) > 0`（p23 末）。

两处只在净值口径下成立：

1. **`R = Σ_{γ∈T} X_γ` 的可加性**（票面说的"级别间会计独立"）。成本口径下持仓量锁死 M=N，跨级别不是各算各的，而是**共享同一个 N**（T₃₄ 股数守恒，`Ledger.lean:123 spawn_conserves` / `:149 spawn_close_roundtrip`）。独立加总与 T₃₃/T₃₄ 直接冲突。
2. 末步 `Var 有限 ⟹ Sharpe(R) > 0` 纯净值。

### 甲-7 `买卖点alpha2.pdf` p8/p11/p12（答复A §10/§14/§15）"goal 应改成什么"的结论

p12 原文："最终修复方向不是再证明一次 Eat(e)，而是把 goal 改成……`E[ΔN_t·ΔP_{t+1} − ΔC_t] > 0`"。这条**处方**被本次裁定直接推翻（新处方是成本下降速率）。p8 的"uncovered=0 不说明 3.净值收益为正 4.风险调整收益改善"同理：清单里的项在新口径下不是待证项。

---

## 乙、与口径无关的推导

### 乙-1 全部结构层 Lean（`Strict/*`、`Origin/*` 的分类/覆盖/递归/中枢/买卖点/全定义）

**证据：`formal/` 里 `Sharpe` 出现 0 次，`E[R]` 出现 0 次，`期望收益` 出现 0 次。** 净值/收益只作为"本文件**不**证"的排除项出现：

- `formal/Strict/ClassificationFamily.lean:41` / `:317` / `:388`：「收益 / 最优 / 阈值稳定 = L3 EmpiricalDomain，需真实数据，不由此 L0 声称」
- `formal/Strict/Chain.lean:326` / `:476`：「不证盈利 / 收益最优」
- `formal/Strict/StrategyFamily.lean:53` / `:664-665`
- `formal/Strict/RiskProj.lean:36` / `:521`：「本文件**不**证投影出的仓位盈利或风险调整后最优」
- `formal/Strict/CThetaWiring.lean:138`

这批文件从一开始就把目标函数排除在有效域外，换尺子对它们零影响。

### 乙-2 `formal/Tlayers/Payoff.lean` 全文（478 行）—— **本来就是成本口径的形式化**

第31课成本阶段三态 `CostStage = {reducing, zeroing, harvesting}`（`:75-78`），配套：`:87 stage_total`（三态穷尽）、`:114 harvesting_absorbing`、`:117 zeroing_irreversible`、`:124 costZero_monotone`（成本单调不增）、`:168 reducing_no_position_increase`（降成本阶段净股数变化 = 0）、`:179 harvesting_shares_nondecreasing`、`:206 netDelta_sign_determined_by_stage`、`:252 reducing_position_constant`。

`:417 payoffValue` 显式 opaque —— 连续净收益数值被主动标为 L2/L3 不裁定。

换尺子后这个文件**不但不受影响，还从配角升为主判据的结构侧**（"闸门前持仓量锁死 M=N" 就是 `:168 reducing_no_position_increase` 的内容）。

### 乙-3 `formal/Origin/TotalWealth.lean`（686 行）—— 成本口径的账本锚

`TWState`（`:94-103`）+ `TW = free + holding + withdrawn`（`:105`）+ 三阶段单向（`:187 stage_rank_monotone`、`:205 earning_no_regress`）+ OQ-9 gate（`:249 LegalEnterEarning`、`:296 oq9inv_preserved`、`:358 oq9_legacy_leg_unreachable_in_earning`）。`:178 twStep_preserves_tw` 只在同价 c 固定下成立，是纯转移恒等式。

`cumNetCash`（`:103` 注释明写「cost_basis 符号的结构载体」）是链上**唯一**已有的成本基载体。

### 乙-4 `formal/Tlayers/Accounting/Ledger.lean`（310 行）会计恒等式

`:78 trade_dual_view_same_source`（T₃₃ 物理一笔两视图）、`:87 trade_zero_iff_both_zero`、`:123 spawn_conserves` / `:140 close_conserves` / `:149 spawn_close_roundtrip`（T₃₄ 股数守恒）、`:291 costReduce_preserves_nBase` / `:306 rebase_preserves_forest`（T₃₈ 两范畴互不干涉）。全是纯算术恒等，不含目标函数。

**但 `:182 nav_neutral_on_cost_reduce`（T₃₅）要单独看**：算式仍真（降成本 spawn 前后 NAV 不变），可它证的恰恰是"**NAV 这把尺子看不见降成本**"。等式不变，读法要翻：从"降成本无价值"翻成"NAV 度量不到得分"。这条建议在 ADR 里点名。

### 乙-5 `formal/Tlayers/Accounting/Earning.lean`（168 行）

T₃₆ 构造不对称（`:49/:58/:68/:78/:93`）、T₃₇ 有条件恒等（`:133 child_pnl_eq_cost_reduction_conditional`、`:147` 反例）。`:109 CloseSettlement` 的四分裂 `{costReduction, earningExcess, shortfallLoss, freeResidual}` 是**已实现盈亏的去向分解**，本身就是成本口径的语言。

### 乙-6 `formal/Origin/CausalEndpointImpossibility.lean`（227 行，C23）

`:152 causal_cannot_eat_endpoint`、`:195 peekAhead_can_eat_endpoint`、`:222 causal_xor_endpoint`。纯因果可测性（`q_t ∈ F_t`），与目标函数无关。对应 PDF `买卖点alpha2.pdf` p4–p5（答复A §5）与 p14–p15（答复B §2）。

### 乙-7 `formal/Origin/CoverFeasibleTarget.lean` 的 `CoverObjective`（`:384-396`）

LexArgmin 的目标函数 `J_t` 第一坐标是 `quadDev`（对目标头寸的二次偏离，`:369-370`），**不是收益**。整套风险投影/全定义/∃!O 机制（`:402 coverKey_inj`、`:419 toStrategyObjective`）与打分尺子无关。

### 乙-8 `formal/Origin/FullDefinitionStrategy.lean:184-203` `LedgerState`

`R = Π − A − W` 由 `inv` 字段类型层强制（`:189`），`:198 ledger_invariant_preservation`。按 `rust/src/theta_v0/closed_loop/transition.rs:80-96` 的口径注释，**Π 已经是已实现利润口径、A 是成本基**（该处明写旧版"让 Π 被成本基污染——利润与成本基混称"是 bug，现已按 `Allocate`(成本基) / `Realize`(利润) 分开）。所以 LedgerState **本来就不是市值净值账本**，它是收益表账本，与成本口径兼容。

### 乙-9 `买卖点alpha2.pdf` p1–p4（答复A §1–§4）+ p15–p20（答复B §3–§9）+ p27–p32（答复C §2–§10）

Eat(e) 结构覆盖不变量、父子生命期包含 ⟹ uncovered=0、`G_e>0` 是端点定向恒真、买卖点证书 γ、声部 v、全互斥解释器 `C_j = P_j ∧ ⋀_{k<j} ¬P_k`、AncOK 活动集更新、目标头寸 `Q_Θ`、风险投影、`∃!O` 全定义定理。全部是**操作语义/全定义性**，一个收益量都不含。

### 乙-10 `买卖点alpha2.pdf` p33 (答复C §13) 细分类不劣定理 `V(Z) ≥ V(Y)`

证明只用"细分类策略集合 ⊇ 粗分类策略的模拟版本 + 取 sup"。对**任何**目标泛函都成立，换尺子照搬即可。同页 §12 的分解式 `E[R] = Σ_z P(Z=z)·μ(z,π̄(z))` 是全期望公式，形式上也照搬（但 μ 的被估对象要换，见丙-4）。p34 §14 正 alpha 定理与 p34–35 §15 反向不可能定理同理，是纯凸组合代数。

### 乙-11 `formal/Origin/CovariantCapital.lean`（537 行）

`:427 phaseOf_scale`（三阶段判定在正标度下不变）、`:397/:407/:414` 各阶段 scale 不变、`:122 dimensionless_invariant`。这套"资本约束必须写成无量纲比例"的等变性（出处 `docs/formal-chain/绝对资本.pdf` p7–p9）对成本口径同样适用，且是加分项——成本下降速率天然是无量纲的。

---

## 丙、需要重证的

### 丙-1 鞅不可能定理

出处：`买卖点alpha2.pdf` p21（答复B §11）+ p35（答复C §16）+ `严格alpha.pdf` p10（第二条定理）。

原命题：`E[ΔP|F_t]=0 ∧ N_t ∈ F_t ⟹ E[ΔN·ΔP]=0 ⟹ E[ΔR] = −E[ΔC] ≤ 0`。

成本口径下的重述：「价格是鞅 ∧ 手续费/资金费 ≥ 0 ⟹ `E[Δbasis] ≥ 0`（成本期望上不下降）」。

难点：原证明一行就完（塔性质 + 可测性），因为收益是净头寸对价格增量的**线性**泛函。成本口径下得分量是 `Δbasis = −realized/M`，而 `realized` 是**停时** τ_γ 的函数（进/出各一次），不是 ΔP 的线性泛函。要复现结论必须用可选停时定理，就得补一致可积或有界停时假设。**这条现在既没被证也没被否——不能默认它照搬成立。**

### 丙-2 C22 净值抵消 → 成本口径版

出处：`formal/Origin/NetValueImpossibility.lean` 全文件。

重述方向：「同单位父多子空双开，成本口径下：子腿平仓的已实现盈亏 > 0 ⟹ basis 严格下降；父腿不平仓则不产生已实现盈亏。故双开在成本坐标下**不抵消**。」

难点（具体、可执行）：现有 `Leg` 结构（`:91-96`）只有 `q / ε / pLam / pRho` 四个字段，**没有区分"元素生命期结束"与"腿平仓"**。要写成本口径版必须先把平仓事件显式化。两条路：(a) 扩 `Leg`；(b) 改挂 `Origin/TotalWealth.lean:120-130` 的 `TWEvent.closeShareLeg profit` —— 那里已经有 profit 参数并落进 `cumNetCash`（`:165`）。**建议走 (b)**，因为 (b) 已经在 canonical base 里。

### 丙-3 `gSep_pos`（声部级毛收益为正）

出处：`formal/Origin/SeparateEat.lean:283 gSep` / `:287 gSep_pos`；顶点消费方 `formal/Origin/SeparateFinalTheorem.lean:225 final_profit`。

`gSep = s_e · ε_e · (P_ρ − P_λ)` 与甲-4 的 `Leg.G` 同型——**未平仓的方向化毛收益**。成本口径下 basis 只在已实现落袋时动，所以"每个元素毛收益为正"翻不成"每个元素都降了成本"。

重述方向：「每个元素在其规范腿**平仓时刻**的已实现盈亏为正」。难点：`gSep` 的 `pRho` 是**元素端点**，不是平仓价；把它当平仓价用等于假设"腿在元素结束的瞬间平掉"，这个假设现在没有形式化承载。要重证得先钉死"规范腿的平仓时刻"这个语义——而这正是 `Origin/CertGatedExit.lean` / typed exit 那条线的对象（未在本次调研中展开）。

### 丙-4 三类买卖点边际收益 μ 的被估对象

出处：`买卖点alpha2.pdf` p22（答复B §12）`μ_{ℓ,δ,I} = E[X_γ | ℓ(γ)=ℓ, δ(γ)=δ, I_γ=I]`。

重述：`μ^basis_{ℓ,δ,I} = E[Δbasis_γ | ...]`，其中 `Δbasis_γ = realized_γ / M`。

难点两条：
1. 量纲从"元"变成"元/股"。M=N 锁死时同一 campaign 内 M 是常数，桶内可比；跨 campaign / 跨标的不可比，池化前必须归一（本仓已有先例：`project_l3_cross_symbol_btc_idiosyncratic` 记的"原始-$ 拼接=尺度伪影"）。
2. 原推导的"级别间会计独立"在成本口径下由 T₃₄ 股数守恒**接管**：`Σ units = N_base` 恒定意味着级别间不是独立而是共享同一个 N。跨级别求和的合法性要重新论证，不能沿用 p23 的 `R = Σ_{γ∈T} X_γ`。

### 丙-5 ★ 两个账本锚都写不出 `basis`

这条是结构性障碍，优先级最高：

- `formal/Origin/TotalWealth.lean:94-103` `TWState` 七个字段 = `free / holding / withdrawn / notionalIn / stage / openLegacyLegs / cumNetCash` —— **没有持仓量（股数）字段**。`holding` 是市值摘要（`:103` 注释自陈"用 Int 承载市值摘要"），不是股数。
- `formal/Origin/FullDefinitionStrategy.lean:184-189` `LedgerState` 四字段 = `Pi / A / W / R` —— **也没有持仓量字段**。

裁定三的 `basis ← basis − 已实现利润/持仓量` 里的分母 **M 在 Origin canonical base 里不存在**。`Tlayers/Accounting/Ledger.lean:42-46` 的 `VoiceLedger` 有 `units`，但那是 legacy 层（`Origin/TotalWealth.lean:8-12` 明写 Origin 版是 native 重证、**不** import legacy）。

重证前必须先做的：给 TWState 或 LedgerState 加持仓量分量，并重证 TW 守恒/OQ-9 那批定理在扩维后仍成立。**这是本次换口径唯一的硬前置工程量。**

### 丙-6 C42 顶点的有效域声明（文档层，非数学层）

出处：`formal/Origin/SeparateFinalTheorem.lean:80-90` + `:429-435`。现文写「C42 = L0 分账本覆盖，与全窗 L3（8 品种 8/8 否证、无 alpha）不矛盾，因为 L3 = 净账户实盘扣成本盈利」。

问题：被引为对照的"全窗 L3 8/8 否证"本身是净值口径的产物。换尺子后这段的第二半要改写成「成本口径下**未测**」，而不是继续拿旧否证当边界。难点不在数学，在于要重跑一遍 8 品种的成本口径测量才能填回去。

---

## 丁、查不动的地方

1. **`docs/formal-chain/chatgpt.com-推导完全分类-fpscreenshot (3).pdf`（46 页，12.7 MB）没读。** pymupdf 文本层提取为空（关键词扫描 44 份 PDF，只有这一份全零命中），是纯截图。没做 OCR。它是"完全分类推导原始对话（截图版，最新副本）"（`docs/formal-chain/INDEX.md:10`），有可能含目标函数相关内容，**未知**。

2. **逐页精读的 PDF 只有一份半。** 全 38 页逐页读完的只有 `买卖点alpha2.pdf`；`严格alpha.pdf` 读了 p8–p20，`多空对冲.pdf` 读了 p1–p3，`目前的缺口.pdf` 读了 p1/p3。其余 40 份只做了关键词定位（`Sharpe|净值|E[R]|E[ΔR]|风险调整|降成本|三阶段|绝对资本|持仓量|操作的频率`）与命中行摘录，**没有逐页读**。特别是 `alpha.pdf`(33p)、`alpha分离.pdf`(18p)、`alpha检验.pdf`(20p)、`多级别检验.pdf`(20p)、`经济正条件.pdf`(19p)、`缠论的全互斥定义策略2.pdf`(37p) 都有净值/成本命中但只看了命中行。

3. **`formal/` 142 个文件里精读了 8 个**（NetValueImpossibility / TotalWealth 前 215 行 / Ledger / Earning / Payoff 声明表 / SeparateFinalTheorem 三段 / CoverFeasibleTarget 片段 / SeparateLedger 片段）。其余 134 个的"与口径无关"判定**建立在 grep 上**：`Sharpe`=0 命中、`E[R]`=0 命中、`期望收益`=0 命中、`净值`只在 5 个文件命中且全部是甲-4/甲-5 或"不证"声明。补过的 grep 盲区：`wealth / Utility / MDD / drawdown / CVaR / profit / equity / pnl / nav / basis / 成本 / 收益` 都扫过。**仍可能漏**：若某文件用我没想到的词表达净值。

4. **没跑过 `lake build`。** 本报告所有 Lean 断言来自源码阅读，没有机器验证。特别是"某定理换口径后不成立"是**语义判断**，不是机器判定——Lean 里的算式当然还是恒真的，我说的"不成立"指的是它承载的结论在新口径下失去承重。

5. **没找到 `049-第49课.md` 与 `031-第31课.md` 的实际路径。** `find` 全仓扫描超时（120s）被切到后台且未返回结果，`chanlun/` 下没有。票面引的 `049-第49课.md:74`「真正能产生总体利润的，还与操作的频率有关」**我没有回原文核对**。相关的间接证据：`formal/Tlayers/Payoff.lean:14-23` 大段逐字引用了 `031-第31课.md:24,26,28,30` 与注 36/169，说明这批课文在仓内确实存在，只是我没定位到目录。

6. **「成本下降速率」这把尺子在 44 份 PDF 里一次都没出现。** 扫了 `成本下降|下降速率|操作频率|操作的频率|降低成本的速度|频率`，只命中：`级别和sigma.pdf` p5「频率 × 边际收益 × 可靠性」/ p6、`缠论的全互斥定义策略.pdf` p1/p4/p7（都是把频率当净值 alpha 的**因子**，不是独立判据）。

   最接近新口径的既有讨论是 `严格alpha.pdf` p13–p16（答复§5 情况C / §7 / §9 / §10）：那里明写「如果你用 alpha 作为唯一目标来评价"多声部短差/**三阶段资本管理**"，范式可能过窄」（p15），并把 L3 拆成"收益目标 `E[ΔR]>0`" vs "风险目标 `J(π)>J(π₀)`"两选一（p15–16），结语是「你现在真正要改的不是数学证明风格，而是**目标函数**」（p16）。

   **但它给的备选只有效用/风控 `J(π) = E[W] − λVar(W) − κE[DD] − ηCVaR`，没有给成本下降速率。** 新尺子是 PDF 链之外的第三条路。这不是缺陷，是事实——ADR 应当自己承担这条尺子的来源（缠师第31/49课原文），不要写成"PDF 链已经说了"。
