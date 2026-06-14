# type1 覆盖率漏检诊断（OKLO L1 主走势层）—— 缺口机制分解 + 修复方向

> 任务（2026-06-13 编排者）：诊断为什么信号层 type1 覆盖率只有 44-49% 而非 100%。
> 前提公理："走势终完美 → 每个笔端点=次级别走势完美=次级别 type1 BSP → 覆盖应 100%"。
> 44-49% 意味漏检过半 type1，怀疑是 bug。要求：**不放松确认适应 44%（声明膨胀），诊断漏检原因**。
>
> **认识论等级：L2**（OKLO 447738 bars 真实数据，单标的；跨标的复验升 L3）。
> **配套**：`signal_layer_bsp_coverage_audit.md`（审计原数据 48.6%）、`recursive_structure_fundamental.md` §6.3（L0 概念判决）。

---

## 0. 判决（先行）

| 子命题 | 真假 | 决定性依据 |
|---|---|---|
| type1 覆盖 ~42-49% < 100% | **真** | OKLO 全量：fallback 44/105=41.9%，macd 45/105=42.9% |
| 这是引擎 bug（漏检本应存在的 type1） | **否** | 缺口根因 = `force_c ≥ force_a`，**ratio 中位 5.73**（非边界、非测量误差） |
| 放松 `TYPE1_CONFIRM_RATIO`(0.9) 能补 | **否（声明膨胀）** | 阈值边界缺口仅 3（fallback）；补全需放到 ratio≤5.73 = 把 5.7× 力度放大判为"衰竭" |
| 接通 MACD 力度能补 | **否** | MACD 路径覆盖 41.9%→42.9%（**+1**）；MACD 只增 candidate，confirmed 不变 |
| C 段边界 / 中枢检测 bug | **否** | 复刻判据零失配（DIV=65 == 实际 trend_div=65）；C_empty/settled_zs<2 = 0 例 |
| 任务公理"走势完美 ⟺ type1" | **过强，被 66% 否证** | 190 settled trend move 中 125（66%）力度未衰竭（加速）反转，由 type3 标记 |
| 信号层在升跌完备意义上成立 | **真（100%）** | 16 个 δ=3 漏配反转在 δ=10 内全有 confirmed BSP，无空白反转 |
| 真实修复轴 | **交易层 voice 清仓判据 type1→sell_any** | 漏掉 66% 的 type3 顶 = RNF 踏空归因（审计 §4） |

**一句话**：type1 不是"走势完美"的充要标记，是其**严格子集**（趋势**衰竭型**完成）。OKLO 是超强单边牛股，66% 的趋势是**加速型**反转（C 段力度中位是 A 段 5.7 倍），结构上只能由 type3（中枢突破）标记。44-49% 是缠论真实形态，**不是 bug**。

---

## 1. 诊断方法

引擎：`RecursiveOrchestrator`（`.venv` newchan_rust），逐 bar 驱动 OKLO 447739 bars 到终态。
口径：主走势层（ladder3 = 审计 L1），取 `current_moves` + `current_buysellpoints` + `current_trend_divergences`（fallback）。
反转：相邻 **settled** move 反向衔接（`direction` 不同）。反转点坐标 = 前 move 的 `seg_end`（component/segment 索引，与 type1 的 `seg_idx=div.seg_c_end` 同坐标系，避开 bar/stroke 合并坐标陷阱）。
匹配：confirmed type1 BSP `seg_idx` 与反转点 δ=3 容差（复刻审计 present-within-δ）。
对照：`enable_macd_divergence` = false（价格振幅 fallback）vs true（MACD 三维度）。
脚本：`analysis/_type1_coverage_diag.py`；数据 `analysis/data_cache/type1_coverage_diag_OKLO.json`。

机制分桶（每个反转恰好一桶，仅用 BSP 存在性，`detect_type1` 对每个 trend div 都产出一个 type1 BSP，confirmed=`force_c/force_a≤0.9`）：

| 桶 | 定义 | 缠论含义 |
|---|---|---|
| **C0_covered** | 反转处有 confirmed type1 | 趋势衰竭型完成（真背驰）✓ |
| **C1_threshold** | 有 type1 BSP 但 unconfirmed（trend div 存在，force 比 >0.9） | 背驰候选但力度衰竭不足——**放松阈值才能救** |
| **C2_no_div** | 趋势 move(zsc≥2) 无 type1 div（`force_c≥force_a`） | 加速型反转（力度未衰竭）= 中枢突破 |
| **C3_consol** | 盘整 move（zsc<2），结构上无趋势背驰 | 盘整完成（OKLO 为 0） |

---

## 2. 核心数据（OKLO 447K，fallback vs MACD）

| 指标 | fallback（价格振幅力度） | MACD（三维度） |
|---|---:|---:|
| settled moves / 方向反转 | 190 / 105 | 190 / 105 |
| **type1 覆盖** | **44/105 = 41.9%** | **45/105 = 42.9%** |
| 任意 BSP 覆盖（δ=3） | 89/105 = 84.8% | 88/105 = 83.8% |
| 任意 BSP 覆盖（δ=10） | **105/105 = 100%** | — |
| C0_covered | 44 | 45 |
| **C1_threshold（阈值可救）** | **3** | **20** |
| **C2_no_div（力度未衰竭）** | **58** | **40** |
| C3_consol | 0 | 0 |
| 缺口实际由 type3 标记 | 45/61 | 42/60 |
| confirmed type1 / type2 / type3 | 60 / 58 / 429 | 58 / 103 / 429 |
| all type1 BSP（含未确认） | 65 | 122 |

注：本文 41.9% 与审计 48.6% 的差异源于匹配口径——审计 present-within-δ 含多锚（seg_idx/move_seg_start/center_seg_start），本文仅 seg_idx 单锚。量级与结论一致（均 <50%，type3 主导）。

---

## 3. 根因：走势级 force 分解（决定性）

对 190 个 settled trend move 完整复刻 `detect_trend_divergence` + `compute_force` + `trend_extreme_seg`（**零失配**：复刻 DIV=65 == 实际 `current_trend_divergences`=65）：

| 走势级判定 | 计数 | force_c/force_a |
|---|---:|---|
| 全 settled move 是 trend（zsc≥2） | 190/190 | 零 consolidation |
| **DIV 产生（force_c < force_a，背驰）** | **65（34%）** | 中位 <0.5，60 个 confirmed(≤0.9) |
| **NO_DIV（force_c ≥ force_a，未衰竭）** | **125（66%）** | **min 1.15 / 中位 5.73 / max 238** |

NO_DIV 的 125 个 ratio 分布：`[1.0,1.2): 2 · [1.2,1.5): 15 · [1.5,2.0): 10 · [2.0,3.0): 8 · [3.0,∞): 90`。

**这是压倒性的结构证据**：125 个趋势反转处，C 段（趋势最后一段）力度是 A 段的**中位 5.73 倍**，90 个 >3 倍。趋势的最后一段是**最强**的一段——这是**加速突破型反转**，力度毫无衰竭迹象。`detect_trend_divergence` 在 `compare_and_build` 的 `check_three_dim`（fallback 下 = `force_c<force_a`）处正确地拒绝它们——**这正是引擎的正确行为**，不是漏检。

之前"65 个 div 全 ratio<1"是**幸存者偏差**：只有 force_c<force_a 的才成为 div，被拒的 125 个根本不进入 `current_trend_divergences`。

---

## 4. 为什么不是 bug：三条独立证据

**证据 A（阈值）**：阈值边界缺口（C1_threshold）fallback 仅 3、macd 仅 20。NO_DIV 的 force 比中位 5.73 —— 放松 `TYPE1_CONFIRM_RATIO` 从 0.9 到 1.0 只多救 2 个（[1.0,1.2)），要救一半缺口需放到 5.73 = 把"力度放大 5.7 倍"判为"力度衰竭"。**这是 `no-patch-mentality` 禁止的声明膨胀**（把非背驰当背驰），任务已明令禁止。

**证据 B（MACD）**：接通 MACD 三维度力度，type1 覆盖 41.9%→42.9%（**+1**）。MACD 让更多 trend move 产生 div（all type1 65→122，type2 58→103），但这些新增 div 的 force 比仍 >0.9（unconfirmed，C1 3→20），confirmed type1 覆盖几乎不变。**换言之：这些反转处无论用价格振幅还是 MACD 面积，力度都没衰竭到确认阈值——是真·非背驰。** 力度衡量退化（fallback vs MACD）不是覆盖低的原因。

**证据 C（C 段结构）**：复刻判据零失配，C_empty(c_start>search_end)=0、settled_zs<2=0、force_a≤0=0、a_start≥n=0。**B2 修复（C 段越界极值）已闭合 C 段空缺口**——没有任何 trend move 因 C 段定义错误而漏 div。缺口纯粹是 force_c≥force_a。

---

## 5. 公理修正：走势完美 = 三类之一，非 type1

任务公理链的断裂环节：

```
走势终完美（缠论定理，真）
  → 每个笔端点 = 次级别走势完美（真，递归代数）
  → 次级别走势完美 = 次级别 type1 BSP（✗ 过强，此环断裂）
```

缠论中，走势（趋势/盘整）的完成有**多种**方式，type1 只是其一：
- **type1（趋势背驰）**：力度衰竭终结趋势 —— OKLO 占 34%
- **type3（中枢突破）**：趋势加速突破后被反向中枢终结（不背驰）—— OKLO 占 66%
- type2 / 线段破坏(settle)：其它完成判据

`recursive_structure_fundamental.md` §6.3（L0，已结算 537 号双重性）逐字判决：
> Move 边界 ⊋ type1 BSP（Move 也经 type2/3/settle 完成），故"对应"是"每个 k-1 type1 BSP 落在 level-k Move 边界"（**单向**），非"每个 Move 边界都是 type1 BSP"。

即 **type1 ⊆ 走势完美点**（单向包含），不是双向等价。本 L2 诊断把这条 L0 概念判决**实证化**：OKLO 上双向等价被 66% 否证，且否证机制（force_c/force_a 中位 5.73）精确量化。

**升跌完备性（任意类型）成立**：16 个 δ=3 漏配反转在 δ=10 内全部有 confirmed BSP（每个 move 段区间含 2-5 个 BSP），**无空白反转**。信号层在"每个走势完美点都有某类 BSP"意义上 100% 构成性成立——交易层可建于其上。

---

## 6. 修复方向（严格，非声明膨胀）

| # | 方向 | 动作 | 等级 |
|---|---|---|---|
| 1 | **信号层 type1 检测：不改** | 42% 是结构形态。放松阈值=声明膨胀（禁）；MACD 不提升覆盖（+1）；C 段已由 B2 闭合。零改动。 | 已结算 |
| 2 | **公理修正** | "走势完美" 操作化 = type1∨type2∨type3∨settle（升跌完备性），**非 type1 单独**。type1 = 趋势衰竭型完成的严格子集（OKLO 34%）。 | L2 已证 |
| 3 | **交易层 voice 清仓判据：type1 → sell_any** | RNF/URS 清仓绑 type1（仅覆盖 34% 顶），真顶 66% 是 type3 中枢突破 ⟹ 强牛 type3 真顶不触发清仓 = 踏空。放宽到任意类型反转。**待预注册回测**（编排者优先级：先信号层后交易层，本文闭合信号层）。 | 开放轴（L3 待测） |

**关键反模式（已避免）**：把 `TYPE1_CONFIRM_RATIO` 从 0.9 上调以"提升覆盖率"—— 这会把 force_c/force_a∈(0.9, 5.73] 的加速反转误判为背驰，污染所有在册回测的 type1 信号，是 090 号声明膨胀 + 161 号务实补丁。**诊断的价值是缩小有效域边界**：type1 的有效域 = 趋势衰竭型反转（OKLO 34%），不是全部走势反转。

---

## 7. 边界条件（结论翻转条件）

- **单标的**：OKLO 是超强单边牛股（+932%），趋势加速型反转占比异常高（66%）。震荡标的 type1/type3 比例不同，但 type1 < 100% 是结构必然（趋势可不背驰终结）。**跨标的（CL/BTC/ES）复验升 L3**——预测：所有标的 type1 覆盖 <100%，且覆盖率 ∝ 趋势衰竭型反转占比（震荡标的更低）。
- **若某标的出现 C_empty>0 或 settled_zs<2>0**：则该标的存在 B2 未覆盖的 C 段边界残留，需独立处理（OKLO 上为 0）。
- **若放松阈值后回测净正**：不构成"该放松"——净正可能来自暴露增加而非择时（`backtest_benchmark_falsifiability` 不可证伪陷阱），且违背 type1=背驰的定义。须随机门控对照。
- **递归层（ladder4+，L2）覆盖更高（审计 recL2 88.9%）**：大级别趋势背驰更清晰（加速型反转在高级别被聚合），但递归层强制 fallback force（`level_buysellpoints` 的 `divergences_from_moves_v1(..., None)`），不可开 MACD——其 type1 覆盖同样非 100%，同根因。

---

## 8. 结果包（六要素）

1. **结论**：type1 覆盖 ~42-49%（OKLO L2）不是引擎 bug，是缠论结构形态。根因 = 190 settled trend move 中 125（66%）反转时 `force_c ≥ force_a`（C 段力度中位是 A 段 5.73 倍，加速型反转），`detect_trend_divergence` 正确拒绝其为背驰；这些反转由 type3 标记（中枢突破型完成）。任务公理"走势完美 ⟺ type1"过强——type1 是走势完成的严格子集（趋势衰竭型），被 66% 实证否证。信号层零改动。

2. **定义依据**：缠论升跌完备性定理（"任何向上/向下必从三类买卖点之一开始并结束"）；趋势完成的双路径（背驰 type1 / 中枢突破 type3，第24/49课）；`recursive_structure_fundamental.md` §6.3 / 537 号双重性（Move 边界 ⊋ type1，单向包含）。代码依据：`divergence.rs:307-318`（`check_three_dim` fallback = `force_c<force_a`）、`divergence.rs:336-344`（`force_a≤0` / 三维度过滤）、`buysellpoint.rs:208-212`（confirmed = `force_c/force_a≤0.9`）、`moves.rs:124`（zsc≥2=trend）。

3. **边界条件**：见 §7。核心翻转条件——(a) 若某标的 C_empty>0 / settled_zs<2>0，则存在 B2 未覆盖的 C 段 bug（OKLO=0）；(b) 若跨标的 type1 覆盖恒 <100% ∧ 任意 BSP 恒 100%，坐实"走势完美=三类之一"；(c) 若 NO_DIV 的 force_c/force_a 在某标的集中于 (1.0,1.1)（而非 OKLO 的中位 5.73），则该标的接近"勉强不背驰"，阈值敏感性需重审（但仍不应放松——定义是背驰）。

4. **下游推论**：(a) 信号层 type1 引擎零改动（无 bug）；(b) 升跌完备性意义上信号层构成性成立（任意 BSP 100%，无空白反转）——交易层可建于其上；(c) 交易层 voice 清仓/降成本判据应从 type1-only 放宽到 sell_any（任意类型反转），否则结构性漏掉 66% 的 type3 顶 = RNF 踏空（审计 §4，待 L3 预注册回测）；(d) 递归层（ladder4+）同根因，覆盖同样 <100%。

5. **谱系引用**：537 号（信号层同一-差异双重性，本诊断的概念模板）、526 号（操作性/存在性意义区分）、`recursive_structure_fundamental.md` §6.3（L0 单向包含判决，本文是其 L2 实证化）、`signal_layer_bsp_coverage_audit.md`（审计原数据 48.6%，本文加机制分解）、`project_bsp_gap_root_cause` / `project_c_segment_fix_b2`（C 段修复，本文确认其闭合 C_empty=0）、`project_signal_layer_duality`（segment/move 双重性，待结晶谱系）、RNF 踏空报告（清仓绑 type1 的 regime 二难）。**本诊断未发现新概念分离**——是 537 号双重性在 type1 覆盖率上的 L2 实证；建议作为 537 号的实证增强登记（谱系工位裁决）。

6. **影响声明**：新增 `analysis/type1_coverage_diagnosis.md`（本文）+ `analysis/_type1_coverage_diag.py`（诊断脚本）+ `analysis/data_cache/type1_coverage_diag_OKLO.json`（数据）。**引擎/交易层零代码改动**。对认识的影响：(1) type1 覆盖 <50% 定级为结构形态非 bug（根因 force_c≥force_a 中位 5.73× 实证）；(2) 排除三个 bug 假设（阈值 / MACD / C 段边界）；(3) 修复轴定位到交易层 voice 清仓判据（type1→sell_any）。否定性结果：任务公理"走势完美 ⟺ type1"被 66% 否证，type1 有效域收窄为"趋势衰竭型反转"——有效域边界收窄，价值高于确认性结论（`formalization-validity-domain.md`）。

---

*认识论等级：L2（OKLO 447738 bars 单标的真实数据，fallback+MACD 双跑，复刻判据零失配）。跨标的复验（CL/BTC/ES）升 L3。本诊断对交易层 PnL 影响沉默——清仓 type1→sell_any 的净效应须独立 L3 回测（编排者优先级：先信号层，本文闭合信号层侧）。*
