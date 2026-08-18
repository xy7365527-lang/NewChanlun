# #893 S8-c 落地：成本口径 5 条重证的逐条处置（2026-08-18）

调研正本：`.chanlun/review-results/issue833-formal-chain-metric-dependency-2026-08-02.md` 丙段。
本票是 SPEC #847 S8-c 拆票，逐条处置丙段 5 条（丙-1／丙-2／丙-3／丙-4／丙-6；丙-5 = S8-a，
已由 #891 落地，不在本票）。

判据：重证通过（`lake build`）或明写「证不了」的精确缺口（本仓纪律：推不出来就明说，
不许拿漂亮数学填空）。缠论断言进 Lean 依 map #854 Notes N-2（具名 `Prop` + 显式前件，
不用 `axiom`）——**本票未引入任何新公理**（丙-2 的定理全部由既有 `twStep`/`cumNetCash`
定义推得，无 缠论原文断言作 hypothesis）。

## 一、丙-1 鞅不可能定理 —— 证不了，精确缺口

**原命题**（净值口径，`买卖点alpha2.pdf` p21 答复B §11 + p35 答复C §16 + `严格alpha.pdf` p10）：

> `E[ΔP|F_t]=0 ∧ N_t ∈ F_t ⟹ E[ΔN·ΔP]=0 ⟹ E[ΔR] = −E[ΔC] ≤ 0`

原证明一行（塔性质 + 可测性）：收益是净头寸对价格增量的**线性**泛函，
`E[ΔN·ΔP] = E[E[ΔN·ΔP|F_t]] = E[ΔN·E[ΔP|F_t]] = 0`。

**成本口径重述目标**：「价格是鞅 ∧ 手续费/资金费 ≥ 0 ⟹ `E[Δbasis] ≥ 0`（成本期望上不下降）」，
其中 `Δbasis = −realized/M`，`realized` 是进/出两个**停时** τ_entry/τ_exit 的函数。

**精确缺口（两条，缺一不可）**：

1. **载体缺口**：`formal/` 全仓 `import Mathlib` = 0，无概率空间／滤子／条件期望／停时的任何
   形式化。该命题在现役 L0 结构层里**连陈述都写不出来**（L0 是纯代数/结构层，零数据依赖）。
   ⟹ 既无法机器证明，也无法机器否证。
2. **即使补上测度论载体，原证明也不转移**：原证明依赖「收益是 ΔP 的线性泛函」；成本口径下
   `realized` 是**停时函数**（τ_entry/τ_exit），对 ΔP **非线性**。要复现结论必须用可选停时定理，
   而可选停时要求新增**一致可积**或**有界停时**假设——没有这两条之一，`E[停时处取值] ≠ E[初值]`，
   鞅性不向停时处传递。

**结论**：**既未证也未否，不默认照搬**。成本口径下「鞅市场里成本期望不降」这个命题当前
**证不了**；要证须先引入测度论载体 + 一致可积/有界停时假设，两者都不在本票范围（本票只做
有效域与重证，不引 Mathlib）。

## 二、丙-2 C22 成本口径版 —— 重证通过（`lake build`）

**落点**：`formal/Origin/TotalWealth.lean` §1.6（新增四定理），改挂 `TWEvent.closeShareLeg
profit`（调研建议的路径 (b)，已经在 canonical base 里，不扩 `Leg` 结构）。

四条定理（全部由 `twStep`/`cumNetCash` 定义推得，机器检查通过）：

- `closeShareLeg_strictly_increases_cumNetCash`：`profit > 0` ⟹ `cumNetCash` 严格上升
  （= cost_basis 严格下降，`cumNetCash` 是 cost_basis 符号载体）。
- `open_position_no_realized_pnl`：`shortDiff`/`openShareLeg` 不改 `cumNetCash`
  （未平仓腿不产生已实现盈亏——父腿不平仓则什么都不发生）。
- `double_open_close_child_not_cancelled`：开两条腿（父+子）后闭合一条（profit p）⟹
  `cumNetCash` 恰增 `p`（**无 −p 抵消项**）。
- `double_open_close_child_profit_strictly_increases`：`p > 0` ⟹ 严格增（成本坐标下双开
  **不抵消**）。

**对照**：净值坐标 `net_value_cancels`（`NetValueImpossibility.lean`）证 `G_parent+G_child=0`
（mark-to-market 抵消）；成本坐标下子腿已实现 profit 不被父腿浮亏抵消（父腿不平仓则不进
`cumNetCash`）。两条定理并置、互不越界——净值版管净值 Sharpe 口径，成本版管成本轨迹口径。

## 三、丙-3 `gSep_pos` —— 证不了（须先钉死「规范腿平仓时刻」语义），精确缺口

`gSep` 的 `pRho`（`priceDelta`）是**元素端点**不是平仓价；把它当平仓价用 = 假设「规范腿在
元素结束的瞬间平掉」，该假设**无形式化承载**。「规范腿平仓时刻」语义是 `CertGatedExit` /
typed exit 那条线的对象，本仓未展开。

⟹ 成本口径重述「每个元素在其规范腿**平仓时刻**的已实现盈亏为正」**重述前须先钉死平仓时刻
语义**——当前证不了，不得默认 `gSep_pos` 在成本口径下照搬成立。

**落点**：`formal/Origin/SeparateEat.lean` `gSep_pos` doc 内加缺口标记（见该文件 #893 S8-c 注）。

## 四、丙-4 μ 的被估对象 —— 量纲改「元/股」+ 跨级别求和改挂 T₃₄

两条分置：

1. **量纲 元 → 元/股**：`Δbasis_γ = realized_γ / M`。M=N 锁死时同一 campaign 内 M 是常数
   （`formal/Tlayers/Payoff.lean` `reducing_no_position_increase` 已 machine-check「闸门前
   持仓量锁死 M=N」），桶内可比；跨 campaign／跨标的不可比，池化前必须归一（先例：
   `project_l3_cross_symbol_btc_idiosyncratic` 的「原始-$ 拼接=尺度伪影」）。这是**估计层纪律**，
   不是 L0 可证命题——本票只写清楚，不产读数（参数归 #853）。
2. **跨级别求和合法性**：原推导 `R = Σ_{γ∈T} X_γ` 依赖「级别间会计独立」，成本口径下由 T₃₄
   股数守恒**接管**——`Σ units = N_base` 恒定意味着级别间是**共享同一个 N**，不是独立加总。
   该守恒已由 `formal/Tlayers/Accounting/Ledger.lean` 的 `spawn_conserves` / `close_conserves`
   / `spawn_close_roundtrip` machine-check ⟹ **已证、可直接引用，无需新证**。跨级别求和不再用
   p23 的独立求和式，改按 T₃₄ 守恒口径。

## 五、丙-6 C42 有效域声明文档层同步 —— 文档改（`lake build` 绿）

`formal/Origin/SeparateFinalTheorem.lean` 文件头「全窗 L3 8/8 否证」的对照段改写：

- 标注该否证是**净值口径**产物（旧验收尺子）；
- 成本轨迹口径（ADR 0015）下，8 品种 L3 测量**未重跑** ⟹ C42 在成本口径下的经验对照是
  「**未测**」，不得继续拿净值口径的 8/8 否证当成本口径边界；
- 补这一格须重跑 8 品种成本口径测量（归 #853 参数，本票不产读数）。

同步改其余四处「全窗 L3 8/8 否证」援引（`theorem_final` doc / `FinalTheoremVerdict` doc /
交付总结 / 谱系），逐处补「净值口径」限定。

## 收尾

- `lake build`（删 `.olean` 强制重编）exit 0，150 jobs。
- `python3 scripts/check_fixture_drift.py` exit 0（两 fixture 无漂移）。
- 未产任何验收读数（判据参数归 #853）。
