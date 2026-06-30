# 经济正条件 L2 实装设计（端点价差分解诊断）

> **存在论位置**：本文档是 [economic-positive-condition-chain.md](economic-positive-condition-chain.md)
> 路径级可捕获价差判据 **Ab − ηin − ηout − Ce/qe > 0** 的 L2 可证伪实装设计（任务 #88）。
> 概念层产出，不含实装代码——代码由后续工位。
>
> **认识论等级**：本设计目标是把唯一可否证环节（可捕获价差判据前件）落到真实数据逐信号分解 = **L2**。
> 证明链本身 L0（economic-positive-condition-chain.md），本设计不重证。

---

## 1. theta_v0 backtest 现有的量（调查结果）

| 判据所需量 | 现状 | 位置 |
|-----------|------|------|
| τin（入场确认时刻） | **已有** = `TradeRecord.entry_bar`（BSP 确认 bar，F_i 可测） | `backtest/metrics.rs:200`；取价口径 `l3_delta_r_alpha.rs:129` |
| τout（出场时刻） | **已有** = `TradeRecord.exit_bar` | `backtest/metrics.rs:202` |
| Pτin（实际入场价） | **已有** = `bars[entry_bar].close × tick`（close 口径） | `runner.rs:641`, `l3_delta_r_alpha.rs:179` |
| Pτout（实际出场价） | **已有** = `bars[exit_bar].close × tick` | `l3_delta_r_alpha.rs:203` |
| Ce（成本） | **已有** = `qty × px × fee_rate`（双边，commission+slippage+tax bps） | `rebuild_cost_series` @ `l3_delta_r_alpha.rs:758`；fee_rate @ `:122` |
| qb（仓位） | **已有** = `TradeRecord.qty` | `metrics.rs:206` |
| εb（方向） | **已有** = `TradeRecord.long`（true=多 / false=空） | `metrics.rs:211` |
| μ(z,a) 估计器 | **已有** = `MuClass{level, delta}`（+ `UClass` 投影桶） | `backtest/mu_estimator.rs:69` |
| **Pλb（理想起点价）** | **缺失** ❌ | 见 §3 数据缺口 |
| **Pρb（理想终点价）** | **缺失** ❌ | 见 §3 数据缺口 |
| λb/ρb（笔端点 bar 索引） | **部分可达** = `Order.source_index`（=触发结构对象 bar，笔尾 ρ），但 λ 未挂 trade | `strategy/exec.rs:155`, `mod.rs:403` |

**结论**：判据右半（执行损耗 ηin+ηout+Ce/qe 所需的成交价 + 成本）**全部已有**；
判据左半（理想端点价差 Ab=εb(Pρb−Pλb)）**算不出来**——因为 `TradeRecord` 不携带信号源笔的端点价。

---

## 2. 笔端点价的可达性（链路存在但未连）

源笔端点价 `Stroke.start_price`/`Stroke.end_price` **存在**（`theta_v0/types.rs:93-94`），
且 `Order`/`VoiceDecision` 携带 `source_index`（触发该订单的结构对象 bar 序号，
`strategy/exec.rs:155`），carrier 携带 `(level, ρ_c)`（`coverage.rs:500`）。

**链路理论上可达**：Order → source_index → 信号源笔（carrier stroke）→ (start_price, end_price)。
**但当前 `TradeRecord` 只记 entry_bar/exit_bar/qty/long/forced_close，未携带源笔端点价**
（`metrics.rs:198-216`）。Trade 由 `runner.rs` 的 `track_position_transition` 从持仓符号翻转构造
（`runner.rs:651`），此时已脱离 Order 的 source_index 上下文——端点价信息在 fill 侧丢失。

---

## 3. 数据缺口（诚实 gap，不接线头）

**缺口**：回测记录了**成交价 Pτin/Pτout**（两套时刻 entry_bar/exit_bar 的 close），
但**没有记录理想端点价 Pλb/Pρb**。判据 Ab=εb(Pρb−Pλb) 的左半算不出来。

**禁止的接线头**（no-patch-mentality）：
- ❌ 用成交价反推端点价（Pλb≈Pτin）——这恰好抹掉 ηin（入场滞后损耗 = εb(Pτin−Pλb)），
  使判据退化为同义反复 Xe>0⟺Xe>0，零信息增量（L0→L1 退化，formalization-validity-domain）。
- ❌ 用笔的 high/low 当端点价——笔端点价是分型确认价（`Stroke.start_price/end_price`），
  非区间极值，二者口径不同。

**唯一严格修复**：`TradeRecord` 新增两个字段 `ideal_entry_px`/`ideal_exit_px`（= 信号源笔的
λb/ρb 端点价），在 Order→fill 链路中沿 source_index 把源笔 (start_price, end_price) 透传到 trade。
这是**有源数据**（笔端点价真实存在），不是构造，符合严格性。

---

## 4. L2 可证伪测试设计（确定性分解，非统计检验）

依据 663（缠论原生 = 逐信号 μ̂>0，不做统计功效检验）+ economic-positive-condition-chain.md §「与663咬合」，
本测试是**逐信号确定性分解**，信息量大于统计符号检验：

### 逐信号四项分解（每笔 trade）
对每笔 trade e，记录源笔端点价后计算：

    Ab   = εb(Pρb − Pλb)                  # 结构理想价差（源笔端点）
    ηin  = max(0, εb(Pτin − Pλb))         # 入场滞后损耗（不利滑移，≥0）
    ηout = max(0, εb(Pρb − Pτout))        # 出场损耗（≥0）
    Ce/qe = px × fee_rate × 2             # 单位成本（双边，已有口径）
    captured = Ab − ηin − ηout − Ce/qe    # 可捕获价差（判据左边）

判据：captured > 0 ⟺ 该笔路径级正收益（economic-positive-condition-chain.md §「可捕获价差判据」）。

### 聚合诊断「钱去哪了」（确定性，非概率）

    Σ Ab          = 结构给的理想总价差
    Σ ηin         = 入场滞后吃掉多少
    Σ ηout        = 出场滞后吃掉多少
    Σ Ce          = 成本吃掉多少
    Σ captured    = 剩下的可捕获 alpha

**否定性结果的价值**（formalization-validity-domain）：若 Σ(ηin+ηout+Ce) ≥ Σ Ab（执行吃光结构价差），
则**否证「该信号集可交易」**——这缩小有效域边界，比确认性结果信息量大。这正是
`project_underperform_bh_osc_bleed`（跑输 BH 的失血）的**确定性归因**：失血到底是结构无价差（Ab 小），
还是执行吃光（η 大），还是成本（Ce 大）——三者可分离定位。

### 与统计检验的区别
现有 `metrics.rs` 的 bootstrap/随机对照是**概率检验**（H0:收益≤0）。本分解是**确定性恒等式**：
Σ Ab、Σ η、Σ Ce 加起来精确等于 Σ trade gross。它回答的不是「显著吗」而是「钱去哪了」——
确定性分解，无概率推断（符合 dialectical-trading-system 禁概率推断作决策基础）。

---

## 5. 落点（具体文件/函数）

| 改动 | 文件 | 说明 |
|------|------|------|
| 1. `TradeRecord` 新增 `ideal_entry_px`/`ideal_exit_px` | `theta_v0/backtest/metrics.rs:198` | 源笔 λb/ρb 端点价；旧测试 mk_trade 需补默认 |
| 2. Order→fill 透传源笔端点价 | `theta_v0/backtest/runner.rs:651` 附近 + `Order`/`VoiceDecision` 携带源笔 (start_price,end_price) | 沿 source_index 把 carrier stroke 端点价透传 |
| 3. 新增逐信号分解函数 `decompose_capturable_spread` | 新文件 `theta_v0/backtest/econ_positive.rs` | 输入 RunResult + bars，输出每笔 {Ab,ηin,ηout,Ce,captured} + 聚合 |
| 4. L2 诊断报告 | 同上 | Σ 分解 + 「钱去哪了」归因 |

**复用现有**（ponytail 阶梯2）：fee_rate/成本口径复用 `rebuild_cost_series`（`l3_delta_r_alpha.rs:758`），
成交价口径复用 `bars[bar].close × tick`，不重造。新增的只有「源笔端点价透传 + 四项分解」。

---

## 6. 建议的下一步工位

1. **数据缺口修复工位**（前置，阻塞其余）：实装改动 1+2（TradeRecord 端点价 + Order 透传）。
   这是 §3 缺口的唯一严格修复，无此则 Ab 算不出。
2. **分解函数工位**（依赖 1）：实装改动 3（`decompose_capturable_spread`）+ bit-exact 单测（合成数据 L1）。
3. **L2 诊断工位**（依赖 2）：在真实数据（BTC 基线 / project_t_engine_btc_baseline）上跑分解，
   产出「钱去哪了」归因——可能否证某些信号集的可交易性（L2 否定性结果）。

---

## 结果包六要素

1. **结论**：经济正条件判据 Ab−ηin−ηout−Ce/qe>0 的**右半（成交价/成本/方向/仓位/μ估计器）现有量全部已有**，
   **左半（理想端点价 Pλb/Pρb）缺失**——`TradeRecord` 不携带源笔端点价。唯一严格修复 = `TradeRecord`
   新增 ideal_entry_px/ideal_exit_px + Order→fill 沿 source_index 透传源笔端点价。L2 测试 = 逐信号确定性
   四项分解 + 「钱去哪了」聚合归因（非统计检验）。

2. **定义依据**：判据来自 economic-positive-condition-chain.md §「可捕获价差判据」（PDF p4-5 §5）。
   现有量映射见 §1 表（每项标文件:行号）。εb=`TradeRecord.long`、τin/τout=`entry_bar/exit_bar`、
   Pτin/Pτout=`bars[bar].close×tick`、Ce=`rebuild_cost_series` 口径。

3. **边界条件（结论翻转条件）**：
   - 若实测发现 Order 链路无法可靠回查源笔端点价（如多 carrier 同 bar 共享 id，coverage.rs:482 的
     simplification ceiling），则缺口修复需先解决 carrier-stroke 唯一映射——此时落点 2 的工作量上升。
   - 若 close 口径成交价与引擎 plan_orders 的成交价不一致（runner.rs:47 注「引擎稳定后二者口径对齐」），
     则 ηin/ηout 计算需统一到引擎成交价口径，否则分解混入口径偏差。

4. **下游推论**：
   - 数据缺口修复后，`project_underperform_bh_osc_bleed`（跑输 BH 失血）可被**确定性归因**——
     失血源（结构无价差 / 执行吃光 / 成本）可分离定位，不再只能概率检验。
   - 分解函数是 663「逐信号 μ̂」用法的路径级实装；为 μ_estimator（MuClass）提供 per-class 的 captured 分布。

5. **谱系引用**：
   - 判据 L0 证明：economic-positive-condition-chain.md（本文档的理论前件）。
   - μ>0 充要：strict-alpha-theorem-chain.md 定理3。
   - 663（逐信号 μ̂>0 非统计显著）：本测试是其路径级确定性实装。
   - 645（π^bsp 因果择时 ≠ π^cov 覆盖）：分解服务 π^bsp。
   - 关联 `project_underperform_bh_osc_bleed`（失血 L2 归因目标）、`project_t_engine_btc_baseline`（L2 数据基底）、
     `project_zero_lookahead_backtest`（确认滞后 = ηin 的来源）。

6. **影响声明**：本文档**不改任何代码**，是 L2 实装设计。指出 1 个数据缺口（理想端点价缺失）+ 3 处落点
   （TradeRecord 字段 / Order 透传 / 分解函数）+ 3 个下一步工位（缺口修复→分解函数→L2 诊断，有数据依赖串行）。
   新增落盘文件 `.chanlun/proofs/economic-positive-impl-design.md`。
