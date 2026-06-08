# PH persistence 分层赋格 FSM 回测 — QQQ 5min 2年

> 一次 OnlineMergeTree 双树跑完，所有尺度 settle 同时产出；按 ATR 自适应 τ 分层为
> L0/L1/L2，驱动三个并行 CostReductionFSM 声部。**persistence 是级别本身，不是过滤器。**

- 代码：`scripts/ph_fugue_backtest.py`、引擎 `src/newchan/trading/ph_fugue.py`、单测 `tests/test_ph_fugue.py`（84 passed）
- 数据：`analysis/data_cache/qqq_5m_2y.json`（QQQ 5min，96168 根，2024-05-29 → 2026-05-29，含盘前盘后，源 alpha_vantage）
- 结果：`analysis/data_cache/ph_fugue_backtest_qqq.json`
- **认识论等级：L2**（单标的单时段，结论可否证；管线正确性 L0/L1 由单测覆盖）

---

## 1. 结论

**三层赋格 FSM 在 QQQ 2024–2026 牛市 regime 下全部大幅亏损，且赋格层数越多亏得越多。这是一个强否定性结果（缩小了有效域边界）。**

| 组 | 策略 | 总收益 | 最大回撤 | Sharpe | 开仓数 | 短差数 | 小转大 | 胜率 |
|----|------|-------:|--------:|-------:|------:|------:|------:|-----:|
| **A** | 纯 PH 三层赋格（L0+L1+L2） | **−25.92%** | 40.89% | −0.68 | 1102 | 764 | 190 | 33.7% |
| **B** | PH 赋格 + 缠论买卖点门控 | **−22.28%** | 34.34% | −0.71 | 923 | 637 | 155 | 34.0% |
| **C** | 单层 settle baseline（仅 band2 进出） | **−9.24%** | 32.31% | −0.03 | 43 | 0 | 0 | 42.9% |
| — | **buy & hold**（自预热起） | **+64.46%** | — | — | — | — | — | — |

排序：**buy&hold ≫ C(单层) > B(门控赋格) > A(纯赋格)**。即：**交易越频繁、声部越多，相对 buy&hold 的损失越大。**

**级别分层本身有效**（这是确认性的一面）：6569 个信号按带分布为
`band0:5087 / band1:1176 / band2:306 ≈ 4.3 : 1` 的自相似比例——persistence 阈值
确实把 settle 事件分成了清晰的级别层级（与缠论分形递归一致）。**失败不在分层，在用法。**

### 失败机制（决定性诊断：信号前向收益）

对每个 (band, side) 信号测其后 12 根 bar 的平均前向收益，对比随机基线（牛市漂移 +0.007%）：

| 信号 | 含义 | 后 12bar 平均收益 | 后 156bar | 判定 |
|------|------|----------------:|---------:|------|
| band2 **BUY**（底确认→我们**买入**） | sublevel settle | −0.007% | +0.160% | ≈ 漂移，**无 edge** |
| band1 BUY | | −0.020% | +0.043% | ≈ 漂移 |
| band0 BUY | | −0.002% | +0.068% | ≈ 漂移 |
| band2 **SELL**（顶确认→我们**清仓/减仓**） | superlevel settle | **+1.169%** | **+1.323%** | **反预测！卖在大涨前** |
| band1 SELL | | +0.662% | +0.785% | 反预测 |
| band0 SELL | | +0.221% | +0.353% | 反预测 |

**机制精确定位**：在单边牛市里，superlevel settle（"顶确认" = 价格跌破前谷，因果确认）
之后价格**强烈均值回归向上**，且 **band 越大反弹越强**（L2 卖出后 12bar +1.17%，是
基线的 167 倍）。我们用它来清仓主仓 + 减短差 = **系统性地卖在反弹之前**。BUY 侧≈市场
漂移（无超额 edge），SELL 侧在此 regime 下**极性是反的**。这就是 −9% 到 −26% 失血的来源。

赋格放大失血：最细的 L0 声部交易最频繁（852 次开仓 + 0 短差，全是主带 whipsaw），
realized −9865；L1 −10808（含 639 次短差，多在牛市里高买回）；L2 −5720。
**层数 = churn 倍数 = 反预测 SELL 信号的暴露倍数。**

---

## 2. 定义依据

- **persistence = 级别**（§7.5 因果 merge tree "按 prominence 组织的区间套"，区间套层数 = 级别层数）：
  settle 事件的 `persistence = death_price − birth_price` 直接经 ATR 倍数阈值
  `τ₀=1×, τ₁=3×, τ₂=8× ATR` 映射为 band ∈ {0,1,2}。这是把 PH 当级别用、不当过滤器用的
  操作化定义（`a_online_persistence.py` §7.5；`PersistenceBands.classify`）。
- **双树 buy/sell-side settle**：sublevel 树（喂 `close`）的 settle = 下跌波底部因果确认 →
  BUY 信号；superlevel 树（喂 `−close`）的 settle = 上涨波顶部因果确认 → SELL 信号
  （`a_online_persistence` 因果 settle 判据 = 缠论分型确认的拓扑形式化）。
- **赋格三声部**：267号操作方法论 v1（满仓满融降成本 + 小转大赋格）。每个 Lk 声部 =
  一个 `CostReductionFSM` 实例（主带 k 进出 + 子带 k−1 短差降成本），共用同一支旋律
  （自相似/分形）。band-1 SELL 同时是 L2 短差减仓与 L1 主清仓 = 赋格声部重叠。
- **缠论买点分类（B 组门控）**：一买=创新低+幅度衰减（背驰**代理**）、二买=不创新低
  (higher-low)、三买=回踩不破前低。**关键边界**：persistence = 幅度 ∈ ker(D)（§5），
  只能算"创新低/幅度"必要条件，**不能算 MACD 力度衰竭**（`a_online_persistence` 模块顶部
  边界）。故"背驰"是**幅度代理**，非真力度背驰——B 组只是必要条件过滤，不是充分确认。

---

## 3. 边界条件（结论在什么条件下翻转）

1. **regime 翻转**：本结论严格绑定 QQQ 2024–2026 **单边牛市**。SELL 信号反预测正是
   "牛市里跌破前谷=回调买点而非顶"的体现。在**震荡/均值回归 regime** 或**熊市**，
   superlevel settle 的极性可能恢复（顶就是顶）→ 策略可能转正。这是 L2→**L3 跨 regime
   验证**的必经检验，当前未做。
2. **信号极性翻转**：诊断显示若把 SELL 信号当作**买入**（抄回调）、BUY 信号当作减仓，
   极性反转后在本 regime 下大概率转正（band2 SELL 后 +1.17%/12bar 是强动量买点）。
   但这**改变了 267号 FSM 的语义**（sublevel→买），属于不同假设，未在本回测实现。
3. **交易成本**：本回测**零成本零滑点**。加入成本后 A/B（千次交易）会进一步恶化，C
   相对受影响小——结论方向不变，差距拉大。
4. **τ 标定**：k₀/k₁/k₂ = 1/3/8 来自 persistence 分布直觉（p90≈1.1×, p99≈5.5×ATR）。
   改变 k 会改变各带信号数与层间比例，但不改变"SELL 反预测"的机制——除非 regime 改变。
5. **盘前盘后**：含 extended-hours 薄量 bar（96168 根）。这些薄 bar 多落在 sub-τ₀ 噪声带
   被过滤，但贡献了部分 L0 churn。RTH-only（约 39658 根）会减少 L0 噪声，方向不变。
6. **复利 + 满仓无杠杆**：每次清仓后 `new_own_capital = 现金`（复利）；leverage=1.0
   （267号"满仓满融"的融资部分未启用，避免杠杆放大掩盖信号质量）。

---

## 4. 下游推论

1. **PH-settle 不能直接做反转择时**：因果 settle 天生滞后——它要求反转**已被后续价格
   确认**（sublevel 要价格涨过屏障、superlevel 要价格跌破前谷）。作为反转 entry/exit
   触发器，它结构性地"晚"，在趋势市变成高买低卖。这与模块自身边界注记（"分型需后续
   K 线确认"）一致——确认的代价是滞后。
2. **级别分层 ≠ 盈利信号**：persistence 成功分出了 L0/L1/L2 三个自相似层级（4.3:1），
   但"分对了级别"和"该级别上 settle 能赚钱"是两件事。下游任何依赖 PH 级别的策略都必须
   独立验证该级别信号的前向 edge，不能假设"级别正确 ⇒ 信号有效"（formalization-validity
   -domain：有效域 ≠ 定义域）。
3. **赋格不是免费的**：多声部并行在**信号有 edge 时**才放大收益；信号反预测时，多声部
   只放大暴露 → 放大亏损。赋格的价值条件依赖底层信号质量，不是结构本身。
4. **缠论门控的有限作用**：B 组（−22.28%）比 A（−25.92%）略好，靠减少 churn（923 vs
   1102 次）——但它过滤的是**入场**（BUY 侧，本就≈漂移），没触及真正失血的 SELL 侧。
   说明：基于幅度代理的买点分类无法修复力度维度（MACD）缺失下的择时问题。

---

## 5. 谱系引用

- **§7.5 因果 merge tree / persistence 区间套**：`docs/persistence_theory.md` §7.5；
  实现 `src/newchan/a_online_persistence.py`（在线因果 settle 判据 = 分型确认的拓扑形式化）。
- **267号操作方法论 v1**（满仓满融降成本 + 小转大赋格）：`cost_reduction_fsm.py` 顶部；
  268a号结算修正（RESET 独立核算 own_capital、min_operable_level 外部参数）。
- **§5 / 239号 不可约维度**：persistence = 幅度 ∈ ker(D)，力度（MACD 动量）是独立维度——
  本回测缺力度维度，故缠论买点分类只能做幅度代理（必要条件），这是 B 组无法修复 SELL
  侧失血的根因。
- **formalization-validity-domain 规则（222/223/230号）**：本结果是有效域 ≠ 定义域的又一例——
  级别分层在定义域（全部 settle）上代数成立，但"settle 可盈利"的有效域在牛市 regime 下
  为空（甚至为负）。否定性结果缩小了有效域边界（规则明示：比确认性结果更有价值）。
- **概念分离提示**：本回测**未**发生新的概念分离。它揭示的是一个**经验否定**（假设被
  L2 数据否证），不是定义矛盾——FSM 与 PH 定义内部自洽，无需 `/escalate`。若后续要把
  "settle 极性随 regime 翻转"形式化为定义，则需新谱系条目（当前仅记为边界条件）。

---

## 6. 影响声明

**新增**
- `src/newchan/trading/ph_fugue.py`：赋格引擎（`PersistenceBands` / `DualTreeStream` /
  `VoiceLedger` / `FugueVoice` / `ChanlunBspClassifier` / `rolling_atr`）。
- `tests/test_ph_fugue.py`：13 个单测（band 分层 / 账本 MTM / 声部进出 / 短差 / 缠论分类 /
  **零前视前缀稳定性不变量**）。
- `scripts/ph_fugue_backtest.py`：A/B/C 三组回测运行器 + 信号前向收益诊断。
- `analysis/data_cache/qqq_5m_2y.json`：2 年 5min 紧凑缓存（从 21 年全量切片）。
- `analysis/data_cache/ph_fugue_backtest_qqq.json`：完整结果（含三层 equity 曲线、交易样本、诊断）。
- 本报告。

**修改**
- `src/newchan/a_online_persistence.py`：新增 `OnlineMergeTree.max_alive_persistence`
  属性（O(栈深) 读 LEVEL_UPGRADE 用）+ `track_dominant` 构造开关（关闭 `_record_dom`
  的 O(n²) `_component_extent`；默认 True 不破坏 `trend_health`/`level_switch_event`）。
  84 个相关单测全过，与 `current_barcode().alive_bars[0]` 等价性已验证。

**影响范围**：纯新增 + 一处非破坏性性能/接口扩展。未改动任何既有定义、未改动
`cost_reduction_fsm.py`（其 POSITION_OPEN 不接受 MAIN_LEVEL_SELL_POINT / LEVEL_UPGRADE
的状态表约束被声部层**遵守而非绕过**：主卖在 POSITION_OPEN 走 BUY_POINT_NEGATED 同出口，
小转大仅在 COST_REDUCING 触发——均忠于 267号语义）。

---

## 附：复现

```bash
.venv/bin/python scripts/ph_fugue_backtest.py      # ~30s，零前视 streaming
.venv/bin/python -m pytest tests/test_ph_fugue.py  # 13 passed
```

零前视保证：每根 bar 喂入双在线 merge tree，settle 因果实时确认（`update()` 只读过去），
τ 用滚动 ATR（窗口 270，只用 [0..i]），信号在当前 bar 收盘可交易——单测
`test_dual_tree_zero_lookahead_prefix_stability` 验证"喂第 i 根不改变 < i 根已发信号"。
