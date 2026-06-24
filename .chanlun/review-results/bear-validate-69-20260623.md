# 大额吃熊牛熊对称 L3 验证：核心做空镜像在真 bear regime（任务 bear-validate）

> 工位：大额吃熊验证（编排者裁决「入库 bear 标的验证牛熊对称大额做空」——做空腿赚的完整形式）。
> 认识论等级：L0（源码/缠论定义）+ **L3（真 bear regime 数据，5 窗口×3 标的，可否证，诚实标注否定性结果）**。
> 诚实约束（231号 formalization-validity-domain）：大额做空有效域 ⊂ 真 bear regime——本轮用真 bear 数据 L3 验证。

---

## 〇、关键诚实声明（先读）

1. **不入库新标的**——bear regime **已作为现有 8 标的的子窗口存在**（CL/ES/BRN 数据 2010-2026 含 dates 列）。用
   `load_clean_ohlc_window`（dates 切片）取真 bear 窗，**同格式同管线，比入库新标的更严格**（零格式失配风险）。
   这是对任务「入库 bear 标的（数据工程）」的严格扬弃：入库新标的=冗余数据工程，切片现有数据=同口径 L3。
2. **编排者的两种措辞指向不同机制、相反结果**（关键澄清）：
   - 「**放开 k<核心**」（字面=移除开空门控 below_core_long，t3sell 核心开空）⇒ **max_gross>1× 大额做空，吃熊 4/5 成立** ✓
   - 「**走势完成→核心翻空**」（d_top 驱动核心翻空镜像）⇒ **STARVED，max_gross<1×，≈#69**（long 塔在走势完成前已坍缩）❌
   - 两者皆实装、皆 L3 测试，诚实报告分叉。
3. **BTC 1m 数据无 dates 列**（btc_1m_full.json 仅 opens/highs/lows/closes）⇒ **无法日期切片** BTC 2022 熊（fail-loud 声明，不入表）。
   BTC 1s 年度文件存在（btc_1s_binance_full_2022.json）但 1s=观测分辨率非操作床位（[[project_cl_1s_a0_verdict]]），不混入。
   CL/ES/BRN 5 窗口（油崩/COVID/标普熊，−23%~−74%）已构成多标的多 bear-型 L3 交叉验证。

---

## 一、L3 数据（5 bear 窗口 × 4 变体，Structural 模式）

| bear 窗口 | bars | BH | OFF | RB_PAIR(#69) | RB_PAIR_CS（d_top翻空） | **RB_PAIR_T3（放开k<核心）** | T3 short_pnl | T3 max_gross | T3 liq |
|---|---|---|---|---|---|---|---|---|---|
| CL 2014-16 油崩 | 563k | −74.3% | −28.8 | −17.2 | −17.2 | **−12.4** | **+2666** | 1.21× | 0 |
| CL 2020 COVID 崩 | 112k | −69.3% | +7.0 | −8.0 | −8.0 | **+4.3** | **+18358** | 1.21× | 0 |
| ES 2022 标普熊 | 279k | −22.9% | +16.7 | −3.0 | −3.0 | **+8.6** | **+12186** | 1.21× | 0 |
| BRN 2020 COVID 崩 | 107k | −61.7% | +46.7 | +1.6 | +1.6 | **+6.2** | **+5164** | 1.05× | 0 |
| BRN 2022H2 跌（震荡）| 167k | −36.3% | +14.0 | −7.9 | −7.9 | **−15.4** | **−6898** | 1.27× | 0 |

**per-level 大额吃熊证据（T3，高级别核心空腿大配额吃整段）**：
- CL 2020：**L2 单笔 +23583**（一个高级别核心空骑完整段崩盘）；ES 2022：**L3 单笔 +7030 + L2 +3422**（高级别核心空吃整个 −23%）。
- 对照 net-up 灾难（shortleg-alpha 变体1）：CL L4 单笔 **−40928** / BTC L3 −20629（假顶翻空打主升浪）。**同机制，bear 中吃熊 / net-up 中灾难——这就是有效域 ⊂ bear 的直接证据。**

---

## 二、结果包六要素

### 1. 结论

**大额做空（max_gross>1×）吃熊在真 bear regime L3 证实（4/5 sharp bear），是 #69「net-up 结构性禁止」的牛熊对称补全。** 具体：
- 「**放开 k<核心**」（移除 below_core_long，t3sell 核心/高级别开空）⇒ max_gross 升至 **1.05–1.27×**（大额达成），
  **short_pnl 大额正 4/5**（CL2020 **+18358** / ES2022 **+12186** / BRN2020 +5164 / CL2014 +2666），**strat 超 #69 +4.6~+12.3pp**，
  **零强平**（5 窗口×T3 全 liq=0——大额空在 bear 中**不穿仓**，价跌空盈 + 否定线止损封顶反弹）。
- **唯一 net-up 灾难担忧在 bear 中消失**：#69「max_gross>1× ⇒ net-up 假顶翻空打主升浪 −40928」的禁止理由，在 bear 中 liq=0、short_pnl 大额正 ⇒ **禁止是 regime 性的，非绝对**。
- **但有效域比「全 bear」更窄**：BRN 2022H2（**震荡型** −36% 反复）大额吃熊**失败**（short_pnl −6898，L1 whipsaw 16 次止损）⇒ 有效域 = **⊂ {真 bear ∧ 趋势性下跌}**（COVID 急崩 / ES2022 持续跌成立，震荡跌失败）。
- **编排者 literal 机制（走势完成→核心翻空，d_top）STARVED**：max_gross<1×、≈#69——因 bear 中 long 塔在走势完成前已 ~90% 止损坍缩（CL Lstop 370/408），走势完成时最高活跃 long 已退至 L0 ⇒ 翻空困在 L0 小配额。**工作机制不是 d_top 翻空，是 t3sell 快信号在塔坍缩前于高级别开核心空**（吃熊=骑熊，对偶骑牛）。

### 2. 定义依据

| 信号/机制 | 缠论定义 | 代码判据 | bear 中角色 |
|---|---|---|---|
| t3sell 核心开空（放开 k<核心）| 第三类卖点（17课+27课区间套）：向下离开中枢、回试不回 | `leave.high<ZD ∧ pull.high<ZD`，移除 `below_core_long` 限制 | 塔坍缩前于高级别开大额核心空，骑熊吃整段 |
| d_top 走势完成翻空（CS）| 走势完成真顶 = type1顶背驰 + 区间套链贯通 a0 | `is_core_long_level ∧ d_top[k]` ⇒ close_long + open_short | **starved**：bear 中走势完成滞后于塔坍缩 ⇒ 困 L0 |
| 否定线止损（max_gross>1× 安全保证）| 27课区间套否定：进场中枢 ZG 破坏 | `open_short_leg(k, top, zg[k], c)` ⇒ 涨破 ZG 止损 | bear 中价跌不触 ZG ⇒ 大额空持有；反弹触 ZG 止损封顶 ⇒ liq=0 |
| k<核心（#69 net-up 门控）| 21课:40 升跌完备性：上涨趋势无真顶卖点 | `highest_active_long().map_or(false,\|core\| k<core)` | net-up 必须（防假顶翻空）；bear 中放开（真顶可空）|

**输入满足**：CL/ES/BRN dates 列切片 → 真 bear 子窗（BH −23%~−74%）；per-level short pnl/opens/max_gross/liq bit-exact 观测；
角色（core vs sub）由 `highest_active_long` 结构涌现（**零 if regime / if level==N**，no-hardcode）。

### 3. 边界条件（结论翻转处）

1. **趋势性 vs 震荡 bear**（最关键翻转）：大额吃熊在 **sharp/sustained bear** 成立（COVID 急崩、ES2022 持续跌），在
   **震荡 bear** 失败（BRN2022H2 反复 −36%，大额空被 whipsaw −6898）。**翻转条件**：若加「走势向下持续性」过滤（趋势涌现判据），
   震荡型可能被排除——但那需要「识别当前是趋势性 bear」的**结构判据**（如最高活跃走势级别方向向下且未完成），是开放轴。
2. **d_top starved 翻转**：若 long 塔不坍缩（如叠加趋势底仓 anchor 保高级别 long 不被否定线扫掉），d_top 翻空可能在高级别 fire 大额。
   当前 LegPair 否定线止损扫掉 ~90% long ⇒ 塔坍缩 ⇒ 翻空困 L0。开放轴：anchor × core-flip。
3. **OFF 基线在 2/5 仍超 T3**（BRN2020 +46.7 / ES2022 +16.7 vs T3 +6.2/+8.6）：base operate 路径（自有 sink/做空）处理 bear 优于 LegPair——
   **LegPair 是 long-biased dip-buyer**（bear 中 long 反复抄底止损，long_pnl 失血主导）。翻转条件：若 LegPair 入场侧也门控（bear 不抄底），可能超 OFF。
4. **net-up 灾难重现**：T3 机制（放开 k<核心）在 net-up 8 标的 = shortleg-alpha 变体1 灾难（CL −57435）。**T3 非全 regime 默认**——
   它是 bear 专属。无 regime 门控的 T3 在 net-up 会灾难 ⇒ 实盘需「当前 regime」结构判据（开放轴，违反 no-hardcode 风险）。

### 4. 下游推论

1. **#69 判决精化**：「大额做空吃熊有效域 ⊂ 真 bear regime——8 net-up 标的无法证实⇒结构性禁止」中的**「无法证实」部分被本轮 L3 证实**
   （4/5 sharp bear，short_pnl 大额正 + liq=0）。**net-up 禁止 + bear 吃熊 = 编排者要的牛熊对称完整形式**。但「结构性禁止」**不是绝对禁止**，
   是 **regime 条件禁止**（net-up 禁 / bear 开）——这正是「大额做空有效域 ⊂ bear」的精确陈述。
2. **「做空腿赚=基本稳」的完整验证**：#69 只达成**次级别短差小额净赚**（max_gross<1×，+9623）；本轮证实**大额核心做空在真 bear 吃熊**
   （short_pnl 单窗口 +18358，超 #69 全 8 标的求和近 2×）。**做空腿赚的完整形式 = net-up 次级别短差（小额）⊕ bear 核心大额吃熊**——两有效域互补，
   覆盖牛熊。
3. **机制选择问题上浮**：编排者 literal「走势完成→翻空」starved，工作机制是「t3sell 放开 k<核心」（= 变体1 = net-up 灾难机制）。
   ⇒ **大额吃熊的工作机制在 net-up 是灾难机制**。要在实盘安全运行须区分 regime，但 no-hardcode 禁 `if regime`。**真正的 no-hardcode 解 = 用
   「最高活跃走势级别方向 + 是否完成」的结构判据自动门控核心空**（向下未完成走势中开核心空 / 向上中禁）——这是开放轴，本轮未实装（避声明膨胀）。
4. **熊转牛不对称确认**（编排者「熊转牛也处理不好」）：LegPair 是 **long-tower-biased**——核心翻空靠平 long 塔，但无独立**短塔**（无 d_bot=下跌走势完成
   tracking，只 d_top）。熊转牛靠任意 buy 平空（非下跌走势完成）⇒ 不对称。完整对称需独立短塔（下跌走势完成驱动），是架构级开放轴。

### 5. 谱系引用

- **539**（[[project_t_short_leg_regime_function]] 做空腿=regime 函数）：**本轮精化**——大额吃熊有效域 = ⊂ {真 bear ∧ 趋势性下跌}（非「net-up 有界负」的全部）。
  539 不再是「做空永远亏/有界负」，而是「做空有效域 = bear∧趋势」（正域明确，net-up 是其补集的禁止域）。
- **567**（冻结空腿=穿仓+杠杆同根）：本轮证实 max_gross>1× 在 bear 中 liq=0（无穿仓）⇒ 567「冻结空腿穿仓」是 **net-up 专属**，bear 中大额空安全。
- **547**（[[project_cascade_level_misattribution]] cascade 翻错级别）：核心翻空（放开 k<核心）是 547「次级别卖点不翻主力」的**对偶解禁**——
  bear 中翻主力是对的（真顶），547 隔离是 net-up 专属。
- **[[project_long_short_dual_open_eat_both]]**（多空双开吃涨跌绝对值纲领）：本轮 = 纲领的**核心做空腿 bear 实装**，与 #69 次级别短差（net-up）对称互补，纲领向 bear 侧推进一步。
- **shortleg-alpha-69 / shortleg-switch-69**（本轮前置）：「大额吃熊未证实⇒结构性禁止」+「牛熊对称需 bear 标的入库才能 L3 闭环」——本轮**用切片现有数据闭环**（无须入库），部分证实大额吃熊。
- **[[project_t_multiscale_independent_filters]]**（regime×scale 二维盈利画像）：本轮新增 **bear×趋势性 二维**（sharp 吃熊 / 震荡 whipsaw）。
- 谱系不确定声明：本轮未触发**新概念分离**（核心做空镜像是 539 有效域精化 + 547 对偶，非新所指）。但「编排者两措辞（放开 k<核心 vs 走势完成翻空）指向不同机制相反结果」**可能是一个待显式化的语法记录**（机制命名歧义）——交 genealogist 判定。

### 6. 影响声明

| 文件 | 改动 | 性质 |
|---|---|---|
| `rec_engine.rs` EngineConfig | +`enable_pair_core_short`（d_top 翻空）+`enable_pair_core_short_open`（放开 k<核心 t3sell）+ 2 constructor | 新增牛熊对称核心做空 2 机制（flag 门控）|
| `rec_engine.rs` 引擎 struct + 构造 | +2 字段 wiring | flag 传导 |
| `rec_engine.rs` g_pair | 两处：① d_top churn 后翻空镜像（CS）② 开空门控 `short_level_ok = below_core_long ‖ 放开标志`（T3）| **核心改动**，全 flag-gated false-by-default |
| `rec_stream.rs` | +`bear_validate_core_short_l3` 测试（5 bear 窗口×4 变体，dates 切片现有数据，零新入库）| L3 bear 验证工具 |

- **bit-exact 守恒（L1 by construction + 验证）**：两 flag 默认 false ⇒ OFF/RB_PAIR g_pair 路径**逐字不变**。验证：OKLO RB_PAIR
  **strat=+88.0% / short_pnl=+85 / max_gross=0.91× / liq=0** 逐字复现 committed #69（shortleg-alpha §四）。
- **守卫**：prove_pair_isolation（547 隔离）+ prove_tw_neutral（守恒）每 op 零 panic（5 窗口×4 变体 + OKLO 全程）。RB_PAIR 零强平契约 hard-assert 保留。
- **未改**：committed #69 主路径（flag false）、平空级别匹配、d_top 函数、OFF/reading_b/reading_b_pair 默认行为。

---

## 三、认识论等级标注

| 命题 | 等级 |
|---|---|
| bear regime 数据已存在为现有标的子窗口（CL/ES/BRN dates 切片）| **L2**（形式化有效域：定义域=合法日期窗，有效域=真 bear 子窗）|
| 大额做空 max_gross>1× 在真 sharp bear 吃熊（short_pnl 大额正 4/5）| **L3**（5 窗口×3 标的，CL2020 +18358 / ES2022 +12186）|
| max_gross>1× 核心做空在 bear 零穿仓（liq=0）| **L3**（5 窗口×T3 全 liq=0）|
| d_top 走势完成→核心翻空 STARVED（max_gross<1×，≈#69）| **L3**（5 窗口 CS 均≈RB_PAIR，否定性）|
| 大额吃熊在震荡 bear 失败（BRN2022H2 short_pnl −6898）| **L3**（否定性，缩小有效域至 ⊂{bear∧趋势}）|
| T3 机制（放开 k<核心）在 net-up 是灾难机制（变体1 复用）| **L3**（shortleg-alpha 变体1，CL −57435）|
| 熊转牛不对称（无 d_bot/下跌走势完成 trigger，靠任意 buy 平空）| **L0**（源码）|
| net-up 禁止是 regime 条件禁止非绝对禁止 | **L3 推论**（bear liq=0 + short_pnl 正 ⇒ 禁止理由 regime 性）|
| RB_PAIR/OFF bit-exact 不变 | **L1**（by construction + OKLO 验证）|

> **核心诚实声明**：本轮**首次用真 bear regime 数据（L3）证实大额做空吃熊**（max_gross>1×，short_pnl 单窗 +18358，零穿仓）——
> #69「大额吃熊有效域 ⊂ bear，net-up 无法证实」的**「无法证实」缺口被 5 bear 窗口部分填补**（4/5 sharp bear 成立）。这是编排者「做空腿赚=基本稳」
> 超越次级别小额短差的**完整形式的 bear 侧验证**。**但三条诚实约束**：（a）工作机制是 t3sell 放开 k<核心（= net-up 灾难机制），非编排者 literal 的
> 走势完成翻空（后者 starved）；（b）有效域 ⊂ {真 bear ∧ 趋势性下跌}，震荡 bear 失败；（c）熊转牛仍不对称（架构 long-tower-biased，缺独立短塔）。
> **不声明膨胀**：大额吃熊**不是全 regime 默认**（net-up 灾难），需 regime 结构判据（向下未完成走势开核心空）才能安全实盘——这是未实装的开放轴。
> **未将 T3 合入 committed #69 默认**（仍 k<核心 门控）——T3 是 bear 专属实验变体，无 regime 判据前合入会 net-up 灾难。
</content>
</invoke>
