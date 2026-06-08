# 审计修复记录（Opus 4.8 审计报告 → 修复）

> 修复者：Claude Opus 4.8
> 日期：2026-06-06
> 输入：`docs/architecture/opus48_audit_report.md`
> 范围：CRITICAL（A1/A2）全修；HIGH/MEDIUM 按审计与编排者指令分别"修复"或"标记"
> 认识论等级：代码修复 L0（公式符号正确性，纯代数验证 + 回归测试）；重跑结果 L2（真实数据，含否定性结果，见 `analysis/fugue_audit_fixed_backtest.md`）

---

## 修复总览

| 审计项 | 严重度 | 处理 | 状态 |
|--------|--------|------|------|
| A1 降成本只赚不赔（截断）| CRITICAL | 全脚本去截断 | ✅ 已修 |
| A2 空头降成本方向 | CRITICAL | 去截断 + 方向语义显式化 + 回归测试 | ✅ 已修 |
| A4 幸存者偏差 | HIGH | **标记不修**（标的选择问题，非代码 bug）| 📌 已标记 |
| A5 可证伪基准 | HIGH | 重跑报告加 BH 对比 + 夏普比率 | ✅ 已加 |
| B4 走势方向笔端点代理 | MEDIUM | 核查：σ 实为中枢定义，非端点代理 | ✅ 已核查（不构成证伪）|
| B3 PH settle 门控越界 | HIGH | 核查 521 号边界：确属越界，**escalate 类不打补丁** | ⚠️ 已标记上浮 |

---

## A1 — 降成本"只赚不赔"截断（CRITICAL，已修）

### 根因

降成本 trim 是一个高抛低吸 / 回补再卖的往返。每次往返的真实美元盈亏可正可负，
但全部回测脚本在累加前对其做了**非负截断**，使亏损的 trim 被静默丢弃：

- `full_system_backtest_v3.py:208`：`return max(0.0, profit - comm)`
- 其余 ~19 个脚本：`if profit > 0 and total_shares > 0:`（亏损 trim 不更新 cost_basis / cumulative_recovered）
- `fugue_with_short_backtest_1min.py:1101`：空头侧 `if profit > 0 and short_shares > 0:`

机制：`cost_basis` 只下调不上调 → 每次 trim 单向降低持仓成本 → 循环 N 次后单笔 PnL
被无界抬升（OKLO 第 7 笔价格跌 21% 却录 +8.20%，155 次 trim）。

### 修复（统一去截断，39 处 / 20 文件）

```python
# 修复前
if profit > 0 and total_shares > 0:
    cost_basis -= profit / total_shares
    cumulative_recovered += profit
# 修复后（亏损 trim 照实扣减：profit<0 → cost_basis 上升、recovered 下降）
if total_shares > 0:
    cost_basis -= profit / total_shares
    cumulative_recovered += profit
```

```python
# v3 修复前 _cr_profit_dollars 末行
return max(0.0, profit - comm)
# 修复后
return profit - comm
```

去截断后 `cumul_cr_dollars` / `cost_basis` 双向累加——价格逆向运动导致的亏损 trim
真实减少累计降成本。这是恢复回测可信度的前提（审计 A1 边界条件）。

### 边界条件（结论翻转点）

若去截断后 OKLO/BTC/HK700 收益仍 >> BH，则"截断是天文数字主因"判定翻转。
审计预期天文数字崩塌——由 `analysis/fugue_audit_fixed_backtest.md` 的 L2 重跑兑现。

---

## A2 — 空头降成本方向（CRITICAL，已修）

### 诊断与编排者裁定

审计指出 diagnosis 的"已修复"只改了利润公式的符号（`direction==1/-1` 分支），
未消除截断，导致空头在亏损 trim 上仍被记非负利润。编排者裁定（用户指令）：

> 空头减仓时：高卖低买才是正利润，高买低卖是亏损；触发逻辑要区分多空方向。

### 修复

1. **去截断**（同 A1）：空头侧 `if profit > 0 and short_shares > 0:` → `if short_shares > 0:`；
   v3 `_cr_profit_dollars` 去 `max(0.0, ...)`。截断移除后，公式 `(buy_price - sell_price)`
   （空头 = 再卖价 − 回补价）的负值正常计入——高买低卖（再卖低于回补）即为亏损。
2. **方向语义显式化**：`full_system_backtest_v3.py:_cr_profit_dollars` 增加 docstring
   明确多空两个方向的 sell-leg / buy-leg 角色与不变式（高卖低买为正、高买低卖为负）。

### 为什么不另改触发逻辑（区分多空方向已满足）

`_try_open_trim` / `_try_close_trim` 已按 `direction` 分支（多头开仓于 `cr_trigger_up`、
空头开仓于 `cr_trigger_down`），利润公式 `_cr_profit_dollars` 已按 `direction` 反号。
不变式"高卖低买为正、高买低卖为负"在**去截断后**即对多空两侧成立——这由回归测试锁定：

`tests/test_audit_cr_profit_fix.py`（6 用例全过）：
- `test_short_resell_high_cover_low_is_profit`：空头再卖高于回补 → 正
- `test_short_resell_low_cover_high_is_loss_not_clamped`：再卖低于回补 → 负（不截断为 0）
- `test_adverse_trims_accumulate_negative`：50 次逆向 trim 累计 < 0（提款机反例）

截断是唯一使空头"亏损 trim 记正利润"的根因；移除即满足编排者不变式。
**未在已正确的触发分支上叠加额外补丁**（no-patch-mentality）。

---

## A4 — 幸存者偏差（HIGH，标记不修）

标的池（QQQ/OKLO/HK700/BTC，2024–2026 普涨）是标的选择问题，非代码 bug。
按编排者指令**标记不修**：重跑报告显式声明此局限，并以 BH 对比 + 夏普暴露之
（普涨段 BH 本身极高，FSM 须显著超越 BH 才有 alpha 证据）。无下跌 / 横盘 / 退市标的，
策略在熊市 / 震荡 regime 的表现仍未知——这是 M1 交付物缺口，记入待办。

---

## A5 — 可证伪基准（HIGH，已加）

重跑报告 `analysis/fugue_audit_fixed_backtest.md` 对每个标的并列：
- **BH%**（买入持有基准）
- **FSM%**（修复后复利）
- **超额 = FSM − BH**（正才有 alpha 嫌疑）
- **夏普比率**（per-trade：mean(pnl)/std(pnl)，标注未年化）

判据（兑现 ROADMAP M1 §可证伪基准）：FSM ≤ BH → 无 alpha 证据（与 buy-hold 同义反复）。
**注**：随机门控对照（ROADMAP M1 交付物 3）本次仍缺失，记入待办——夏普 + BH 对比
是必要不充分的可证伪手段，随机基线才能分离"缠论信号 vs 随机择时"。

---

## B4 — 走势方向定义（MEDIUM，已核查，不构成证伪）

核查 `src/newchan/a_move_v1.py`：

- `Move.direction` 对**趋势**由同向中枢贪心分组方向定义（`_group_consecutive`，中枢中心位移）；
  对**盘整**填 `break_direction`（第 31 课"盘整概念上无方向"的技术性标识）。
- **结论**：`Move.direction` 是**中枢定义**（缠论正典），不是笔端点代理。与设计声明
  "走势方向用中枢定义"一致，无定义冲突。

v3 `_extract_sigma` 取 `moves[-1].direction` 作 σ——读的是中枢定义的方向。审计 B4 提到的
笔端点 `strokes_now[si].p1`（v3:614）仅用于 PH 降成本 trim 的时机（521 号 PH 拓扑层），
**不参与走势方向 σ**。故 B4 不证伪重跑的选股方向。

残余（审计自身 hedge）：取"最近一个 move"的瞬时方向作当前 σ 是**级别粒度**选择
（最近 move 若为小级别盘整则 σ=0），非端点代理；属可调粒度，记入待办，不影响本次修复正确性。

---

## B3 — PH settle 门控越界（HIGH，已标记上浮，不打补丁）

核查 521 号谱系（`.chanlun/genealogy/settled/521-pure-topo-momentum-impossible.md`，已结算 L0 定理）：

> 纯拓扑（PH）框架内**不存在**编码"动量衰竭 / 背驰"的拓扑不变量；动量闸**必须** MACD。
> 下游：架构应为"双闸门 PH（级别 + 中枢结构）+ MACD 动量闸"。

`full_system_backtest_v3.py` 的 FSM 用 PH rank-1 settle 翻转（`flip_long`/`flip_short`）作
**入场 / 出场门控**（v3:691-696 入场、714-721 止损），**无 MACD 动量闸**。按 521 号，
PH settle 给"候选时机"（candidate 层），不给"背驰确认"（confirmed 层须 MACD）——
以 PH settle 单独作门控属**越界**。

**处理**：这是系统已结算定理（521）与 FSM 实现之间的**设计矛盾**，属 escalate 类
（no-workaround：不在错误前提上打补丁）。本次**不修**——修复须重构 FSM 引入 MACD 动量闸，
超出"修 A1/A2 + 重跑"范围，且是定义层选择，须 `/escalate`。

**对重跑结果的限定**：`analysis/fugue_audit_fixed_backtest.md` 的 FSM 仍是 **PH 门控** FSM
（无 MACD 动量闸）。按 521 号，其信号是 candidate 层，**不构成 confirmed 买卖点 alpha 证据**。
已含 MACD 闸的变体见 `fugue_complete_macd` / `fugue_persistence_macd`（同样已去截断，未在本次重跑）。

---

## 追加修复 — 挣股数阶段缺失（编排者追加要求，2026-06-06）

**问题**：降成本是两阶段链（降成本→成本归零→**挣股数**），原代码只实现阶段1。
第53课"0成本后就赚筹码"、第81课"降成本、增筹码"、编纂版"挣股票的阶段……比底部还多的成本为0的股票"。
原 `PRINCIPAL_RECOVERED`/`PRINCIPAL_WITHDRAWN` 状态到达成本归零后**仍只下调 cost_basis、从不增持**——挣股数（增筹码）缺失（spec-execution-gap）。

**实现**（必须实现，非预留）：
- no_addon：新增 `CostSt.EARNING_SHARES`；成本归零后短差利润 `total_shares += profit/c`（增持，无截断）；
  PnL 改全额记账 `(cumulative_recovered + total_shares×price − INITIAL)/INITIAL`（阶段1 与旧公式数学等价）。
- v3：新增 `earn_legs`；阶段2 利润转额外仓位 `earn_legs.append((profit, c))`，退出时 `_earn_pnl_dollars` 计入。
- 回归测试 3 个（全额记账等价 / 挣股数增收益 / 源码结构），9 用例全过。

**L2 否定性发现**：A1 去截断后短差有赚有赔，成本**很少归零** → 挣股数**很少激活**（OKLO 实测 0 笔）。
完整流程已实现，但当前信号质量（PH 门控无 MACD 动量闸，B3/521号）不足以稳定进入挣股数。
详见 `docs/architecture/cost_reduction_earning_shares.md`。

---

## 结果包（六要素）

**1. 结论**：A1/A2 截断 bug 全脚本修复（39 处 / 20 文件 + v3 max(0) 去除），由
`tests/test_audit_cr_profit_fix.py`（6 用例）锁定多空不变式；A5 BH+夏普已加；
B4 核查为非证伪（σ 是中枢定义）；B3 确认越界但属 escalate 类未打补丁；A4 标记不修。

**2. 定义依据**：38 课降成本（高抛低吸 / 向下段低买高卖）；`a_move_v1.py` 中枢定义走势方向
（第 31 课盘整无方向）；521 号（PH 纯拓扑无动量，动量闸须 MACD）。

**3. 边界条件**：去截断后若天文数字不崩塌则 A1 判定翻转（由 L2 重跑兑现）；
若重跑 FSM ≤ BH 则"无 alpha 证据"成立；B3 若 521 号边界被修订（PH 可作门控的条件）
则该项越界判定翻转。

**4. 下游推论**：修复后回测数字是"PH 门控 FSM、去截断、真实降成本"的 L2 基线；
M2 选股层 / M4 执行层接入前须先补随机门控对照（仍缺）+ MACD 动量闸（B3）。

**5. 谱系引用**：521 号（B3 边界）、526 号（递归存在论）、`formalization-validity-domain`
（L0-L3 标注）、`no-workaround` + `no-patch-mentality`（B3 不打补丁、A2 不叠补丁的依据）。
memory：`project_costreduction_moneyprinter_bug`、`project_trend_direction_proxy`、
`project_ph_settle_usage_boundary`。

**6. 影响声明**：修改 20 个回测脚本的降成本利润计算（去截断）；v3 增方向 docstring + JSON dump；
no_addon 增 BTC 标的 + JSON dump；新建 `tests/test_audit_cr_profit_fix.py`。
不修改引擎核心代码（`src/newchan/`）。重跑结果见 `analysis/fugue_audit_fixed_backtest.md`。
