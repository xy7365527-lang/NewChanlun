# Master 出场级别动态爬梯消融 — 判决（2026-06-11）

> 任务：BTC 上 HoldTrend 只救 +14.5pp，根因诊断为"出场基准级别太低"。
> 假设：出场级别随趋势发展动态上升（爬梯）能接近 BH 暴露但保留出场能力。
> 实现：`ExitMode::Climb { hold_trend: bool }`（rust/src/trading/）。
> 数据：analysis/data_cache/master_exit_climb_backtest.json（脚本同名 .py）。

## 1. 结论：Climb 被四标的否证（L3），且 BTC 进入爆仓域

| 标的 | BH | V2oa25 基线 | _ht（在册正结果） | _cl Δ(base) | _clht Δ(base) |
|------|----|----|----|----|----|
| OKLO | +307% | +1623.1% | **+176.9pp** | −155.5pp | −262.1pp |
| BRN | +87% | +103.8% | **+495.0pp** | −103.3pp | +54.9pp |
| CL | +28% | +40.8% | **+25.0pp** | −34.0pp | −25.4pp |
| BTC | +1380% | −7.9% | **+14.5pp** | **−289.7pp（复利−297.6%，MDD−120.6%）** | −302.0pp |

- _cl 4/4 全负；_clht 3/4 负（BRN +54.9pp 唯一正臂，但 ≪ _ht 的 +495pp——被 HoldTrend 严格支配）。
- **HoldTrend 保持四标的全正的新默认候选地位不变**；Climb 轴关闭。

## 2. BTC 三问的回答（任务预注册观察面）

1. **爬到多高**：全程仅 4 次爬梯事件；exit reason 直方图显示 670 笔中 576 笔仍在
   segment 级出场，recL2/recL3 出场合计 9 笔。爬梯触发稀少的根因 = 判据要求
   `trend_state[max_ladder]`，而 max_ladder 是全局塔高（顶层结构跨年，Trend 态
   确认极慢——"最高级别σ短窗口冻结"先例的另一面）。
2. **出场次数减少多少**：1462 → 670 笔（−54%）。机制：每次爬梯事件把一笔交易
   变成超长持仓（avg_hold 2536→6258 bar），单笔吞掉基线中数百笔的时间窗口。
3. **能否接近 BH 暴露但保留出场能力**：暴露确实升到 0.91、出场能力确实保留，
   但结果是**两头落空的最坏组合**——既没有像 HighestOnly（+1340.6%，伪装
   buy-hold）那样搭上 BH，又让 master 持仓穿越完整的高级别下跌段（MDD −120.6%
   = 单笔 pnl < −100%：超长持仓窗口内 voice 短差累计亏损 + 持仓深熊穿越使
   total_value 转负——shared_position 爆仓算术同模式）。

## 3. 概念判决：出场判据的级别不可高于入场级别（卖点滞后放大）

HoldTrend 与 Climb 的范畴区分——为什么前者全正后者全负：

- **HoldTrend 不改变出场判据的级别**：触发仍是 entry 级 sell1（事件），只加一个
  父级别趋势衰竭的**状态门**。出场点最多推迟到"父级别衰竭证据出现"，仍锚定在
  entry 级转折附近。
- **Climb 改变判据本身的级别**：sell1 @ 更高级别 = 等待高级别转折确认。高级别
  sell1 不在乎入场成本——它出现时高级别下跌段已走完大半。爬得越高，"卖点滞后"
  放大得越狠（recL3 的 type1 卖点滞后 = recL3 级别的回撤深度）。
- 这与 HighestOnly 否证是**同一根因的两种剂量**：HighestOnly 是一次性跳到顶层
  （sell1 几乎不来 = 伪装 buy-hold，碰巧吃满 BTC 单边）；Climb 是有条件的部分
  升级（sell1 偶尔来 = 既丢 BH 又吃高级别回撤）。
- BRN 旁证：_cl 暴露反而暴跌（0.77→0.30）且复利归零——高级别出场点位于深跌处，
  出场后错过的不是回撤而是行情。级别错配在两个方向上都付代价。

## 4. 结果包六要素

1. **结论**：见 §1-§3。Climb 轴关闭；master 出场侧的开放结论维持
   "HoldTrend（状态门）是唯一正确认方向，升级出场判据级别（事件源上移）被
   两种形态（HighestOnly/Climb）双重否证"。
2. **定义依据**：爬梯判据 = `max_ladder > exit_ladder ∧ trend_state[max_ladder]
   ∧ dir==Up`（17课趋势 ≥2 同向中枢，Cycle38 宿主趋势态先例）；出场 = 31课
   "历史性大顶"级别相对化的 sell1 读数；hold_trend 组合 = 41课衰竭门挂在动态
   `exit_ladder+1`。
3. **边界条件**：(a) 爬梯条件用的是**顶层** trend 态——若改为"扫描
   (exit_ladder, max_ladder] 间最高 Trend 层逐级爬"，触发频率会大增，结论可能
   不同（但 BTC 仅有的 4 次爬梯已 −290pp，更多爬梯按本机制只会更糟）；
   (b) 1min 数据、floor=segment、零摩擦回测；(c) 复利口径在单笔 pnl<−100% 后
   失义（BTC 数字应读作"爆仓"而非字面 −297%）。
4. **下游推论**：BTC 死因（出场基准级别太低）的修复方向不在出场侧级别升级——
   剩余开放轴是入场侧（entry_ladder 本身太低/41课门时效分辨率）或 voice 在
   超长持仓窗口的流血控制（t6 预逃逸槽先例）。
5. **谱系引用**：HighestOnly 伪装 buy-hold 否证（master出场模式消融，2026-06-11）；
   级别确认是状态范畴非事件范畴（递归建仓双否证 + HoldTrend 第四例）；
   shared_position 爆仓算术先例；最高级别σ短窗口冻结（顶层 Trend 态稀有的根因）。
6. **影响声明**：新增 `ExitMode::Climb`（config.rs/runner.rs/types.rs
   n_exit_climbs 计数器）+ 变体 V2oa25_cl/_clht + 脚本与 JSON。在册零接触
   （V2oa25 与 V2oa25_ht 两 cell 与 master_exit_mode_backtest.json 逐键对账
   PASS×4 标的；cargo 127 项测试全过）。Climb 代码保留为已否证消融臂
   （预注册判据可复查），不进入默认配置。

## 5. 守卫记录（L1）

- 在册对账：V2oa25、V2oa25_ht 两 cell × 4 标的 = 8 次逐键对账全 PASS
  （trend_flips 行 + Climb 改动对 Signal/HoldTrend 路径零泄漏）。
- 首笔入场 bar 同一性：四变体 × 4 标的全同（入场侧零改动）。
- cargo test 127 项全过；guard：Climb 无 trend_flips/dir_flips 行 fail-fast
  （方向 ≠ 趋势，不提供方向行代理降级）。
