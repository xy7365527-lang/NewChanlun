# 降成本 → 成本归零 → 挣股数：缠师原文完整逻辑链与代码实现

> 日期：2026-06-06
> 触发：编排者指出降成本不止"降成本"，还有成本归零后"挣股数"阶段——用利润买回更多股数，持仓量反而增加。
> 认识论等级：原文考据 L0（一级/二级权威原文引用）；代码实现 L0（公式正确性 + 回归测试）；重跑 L2。

---

## 一、原文完整逻辑链（三级权威链溯源）

降成本不是孤立操作，而是一条**两阶段**资金管理链：**降成本 → 成本归零 → 挣股数（增筹码）**。

### 阶段划分的原文依据

**第53课（一级权威，博文）** — 挣股数的最明确定义：
> "由此，不难明白，为什么本ID说**0成本后就赚筹码**的意思，有些股票，根本就不需要钱在里面搞，到后来，大把人为你抬轿子。知道大牛市最金贵的是什么？**筹码！**"
> "……加快**成本变0或增加筹码**的过程。"（`docs/chanlun/text/blog/053-第53课.md:34,101,103`）

**第81课（一级权威，博文）** — 降成本与增筹码并列：
> "如果没技术，就死拿着等停牌，有的，就拿部分打短差，**降成本、增筹码**。"
> "在来回震荡中**减低成本、增加筹码**。"（`docs/chanlun/text/blog/081-第81课.md:288,322`）

**《股市技术理论》编纂版（二级权威）** — "挣股票的阶段"最完整表述：
> "特别**挣股票的阶段**，一般一个股票，盘整的时间都占一半以上。如果一个股票在上涨后出现大型盘整，只要超大级别卖点没出现，这个盘整会让你的股票**不仅把抛掉的全挣回来，而且比底部的数量还要多，甚至多很多**。一旦股票再次启动，你就拥有**比底部还多的但成本为0的股票**，这才是最大的黑马……一个合理的持仓结构，就是拥有的**0成本股票越来越多**，一直游戏到大级别上涨结束以后。"（`docs/chanlun/mineru/.../full.md:1591`）

**第38课（一级权威，博文）** — 分区卷钱使成本不断减少（阶段1 机制）：
> "就如同开了一个**分区卷钱的机械**……这样不断地机械操作下去，**成本就会不断减少**。"
> "用本ID减成本的方法，最终的结果就是**钱越来越多，而筹码不见少**。"（`docs/chanlun/text/blog/038-第38课.md:30,52`）

**第26节杂史（二级权威）** — 负成本终态：
> "基本的**0成本筹码**，然后反复拉抬都变成纯负数的……满手都是**负成本的筹码**。"（`docs/chanlun/text/chan99/0047-...md:124`）

### 严格逻辑链（综合上述原文）

| 阶段 | 触发 | 操作 | 持仓量 | 成本 |
|------|------|------|--------|------|
| **阶段1 降成本** | 持仓后中枢震荡 | 高抛低吸短差，利润下调成本 | 不变 | entry → 0 |
| **临界** | 成本归零（利润累计=本金） | — | — | = 0 |
| **阶段2 挣股数** | 成本已归零 | 短差利润**买回比抛出更多的股数** | **增加**（新增股 0/负成本） | ≤ 0 |
| **终态** | 超大级别卖点出现才清仓 | 全部清仓 | 比底部多很多 | 负 |

**核心区分**：
- 阶段1：利润 → **降 cost_basis**（钱拿出来，股数不变）。
- 阶段2：cost_basis 已 0 → 利润 → **买更多股数**（持仓量增加，成本维持 0/负）。

---

## 二、现有代码是否实现了挣股数？——否（修复前停在阶段1）

审计前的全部赋格回测脚本（no_addon / with_short / v3 等）**只实现了阶段1**：

- `cost_basis -= profit / total_shares`（下调成本）+ `cumulative_recovered += profit`。
- 到达 `cumulative_recovered >= own_capital`（≈成本归零）后转入 `PRINCIPAL_RECOVERED` / `PRINCIPAL_WITHDRAWN` 状态，但该状态下**仍然只下调 cost_basis（继续变负），从不增加 total_shares**。
- `total_shares` 仅在"加仓"路径（addon 版）改变，与挣股数无关。

**结论**：`PRINCIPAL_RECOVERED` 这个名字暗示"本金回收"，但其行为只是"成本继续变负"，**没有把利润转化为更多筹码**——挣股数阶段（53/81课的核心）**完全缺失**。这是一处操盘流程不完整（spec-execution-gap）。

---

## 三、修复后的实现（2026-06-06，必须实现非预留）

### no_addon（`fugue_no_addon_backtest_1min.py`，长仓干净基线）

1. `CostSt` 新增状态 **`EARNING_SHARES`**。
2. 阶段1→阶段2 转换：`cumulative_recovered >= own_capital`（成本归零）→ `cost_state = EARNING_SHARES`（原 `PRINCIPAL_RECOVERED`）。
3. `EARNING_SHARES` 状态短差平仓：
   ```python
   profit = (active_trim_sell_price - c) * active_trim_shares
   if c > 0:
       total_shares += profit / c   # 挣股数核心：利润买回更多股数（可正可负，无截断）
   ```
   **不再下调 cost_basis、不再累加 cumulative_recovered**。
4. PnL 改为**全额记账**（阶段1 与旧公式数学等价，阶段2 正确）：
   ```python
   pnl_pct = (cumulative_recovered + total_shares * price - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100
   ```

### v3（`full_system_backtest_v3.py`，组合，含期货/杠杆/多空）

1. 新增 `earn_legs: list[(notional$, 买入价)]`。
2. `_try_close_trim` 在 `PRINCIPAL_WITHDRAWN`（成本归零）状态：短差利润不再累加 `cumul_cr_dollars`，而是 `earn_legs.append((profit, c))`——按当前价买入额外仓位。
3. `_close_trade` 增加挣股数额外仓位的退出盈亏：
   ```python
   pnl += _earn_pnl_dollars(price) / INITIAL_CAPITAL * 100   # Σ direction×(exit−c_k)/c_k × notional_k
   ```

### 会计等价性验证

`tests/test_audit_cr_profit_fix.py`：
- `test_full_accounting_equivalent_to_legacy_phase1`：阶段1 全额记账 ≡ 旧 `(price−cost_basis)/entry`。
- `test_earning_shares_increases_position`：挣股数提高总收益（额外股数 × 退出价）。
- `test_earning_shares_state_present`：源码结构断言。
- **实证等价**：OKLO 去截断 A1-only 与挣股数版均为 **+114.08%**——因 OKLO 去截断后成本极少归零，挣股数几乎不触发，故两版相同，反证阶段1 公式等价。

---

## 四、重要发现：去截断后挣股数很少触发

A1 去截断后，降成本短差**有赚有赔**，`cumulative_recovered` 很难累计到完整本金 → **成本很少归零** → 挣股数阶段**很少激活**。

这与原文不矛盾：缠师的挣股数依赖**高精度短差**（"技术高的就能把成本降更低，筹码增得更多"，编纂版）和**大牛市普涨**背景。在 PH 门控、无 MACD 动量闸（B3，521号）、单一普涨时段的当前回测设置下，短差精度不足以稳定把成本打到 0，故挣股数罕见触发。**这是对"当前信号质量不足以支撑完整操盘流程"的 L2 否定性证据**，不是实现缺陷。

---

## 结果包（六要素）

**1. 结论**：缠师降成本逻辑链是两阶段——降成本（成本→0）+ 挣股数（成本归零后利润买更多筹码，持仓量增加）。原代码仅实现阶段1，挣股数缺失；现已在 no_addon + v3 实现 `EARNING_SHARES` / `earn_legs` 阶段。

**2. 定义依据**：第53课"0成本后就赚筹码"、第81课"降成本、增筹码"、编纂版"挣股票的阶段……比底部还多的成本为0的股票"、第38课"分区卷钱成本不断减少"、第26节"负成本筹码"。

**3. 边界条件**：挣股数仅在成本归零（`cumulative_recovered ≥ INITIAL_CAPITAL`）后激活；去截断后成本罕归零 → 挣股数罕触发。若短差精度提高（如加 MACD 动量闸）使成本稳定归零，则挣股数将显著激活，结论翻转点在此。

**4. 下游推论**：完整操盘流程（降成本+挣股数）已可被回测；但当前信号质量（PH 门控无动量闸）不足以稳定进入挣股数 → M2/M4 接入前须先提升短差精度。

**5. 谱系引用**：521号（PH 无动量，B3）、`spec-execution-gap`（挣股数缺失=声明-能力缺口）、memory `project_costreduction_moneyprinter_bug`。

**6. 影响声明**：新增 `EARNING_SHARES` 状态（no_addon）+ `earn_legs`（v3）+ 全额记账 PnL + 3 个回归测试；新建本文档。重跑结果见 `analysis/fugue_audit_fixed_backtest.md`。不改引擎核心 `src/newchan/`。
