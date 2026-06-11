# Master 出场状态驱动消融判决 — exit_mode 单轴 × OKLO/BRN/CL/BTC

**任务**（2026-06-11 编排者）：master 出场从事件驱动（entry 级 confirmed sell1
即出）改为状态驱动（49课"利润最大化"持币不动 + 41课"大级别走势没有衰竭时
不做反向"）。动机：BTC 超强单边（BH +1380%）上基线 −7.9%——master 频繁
出场丢掉趋势主体段利润。

**认识论等级：L3**（四标的真实 1min 数据跨资产：OKLO 强趋势股 / BRN+CL
期货 / BTC 全历史单边；守卫面 L1 = cargo 127 项测试 + O0≡P5 基线对账）。

---

## 1. 结论

**HoldTrend（sell1 ∧ 父级别趋势衰竭才出）4/4 标的全正——状态驱动出场方向
L3 确认；HighestOnly（只认 max_ladder 卖点）3/4 负 + BTC 上的"+1348.5pp"
是伪装 buy-hold——作为操作模式被否证。BTC 死因未被 HoldTrend 解决
（仅 +14.5pp）：问题不在单次出场时点，而在出场基准级别本身太低。**

| 标的 | bars | BH | V2oa25 基线 | V2oa25_ht（HoldTrend） | V2oa25_ho（HighestOnly） |
|------|------|----|------------|------------------------|--------------------------|
| OKLO | 447K | +307.1% | +1623.1%（195笔） | **+1800.0%（Δ+176.9pp，170笔）** | +563.4%（Δ−1059.7pp，3笔） |
| BRN | 2.42M | +87.4% | +103.8%（910笔） | **+598.8%（Δ+495.0pp，764笔）** | +84.5%（Δ−19.3pp，6笔） |
| CL | 5.53M | +28.2% | +40.8%（2037笔） | **+65.8%（Δ+25.0pp，1898笔）** | −20.4%（Δ−61.2pp，5笔） |
| BTC | 4.63M | +1380.4% | −7.9%（1462笔） | +6.6%（Δ+14.5pp，1109笔） | +1340.6%（Δ+1348.5pp，2笔） |

细分观测面（暴露 = 持仓 bar/总 bar；holds = 趋势门拦截出场的 bar 数；
sd = voice 短差总次数）：

| 标的 | 变体 | 胜率 | MDD | 暴露 | holds | sd |
|------|------|------|-----|------|-------|----|
| OKLO | 基线→_ht | 56.4→58.2 | −43.9→−43.9 | 0.72→0.76 | 28 | 611→704 |
| BRN | 基线→_ht | 60.4→60.7 | **−59.9→−47.0** | 0.77→0.81 | 186 | 2434→2599 |
| CL | 基线→_ht | 59.2→59.2 | −72.1→−72.9 | 0.74→0.76 | 404 | 4721→4816 |
| BTC | 基线→_ht | 57.5→58.0 | −74.9→−74.4 | 0.80→0.85 | 202 | 6029→6417 |

### 1.1 HoldTrend：4/4 全正，且非纯暴露伪影

- 增量与暴露增量**不成比例**：BRN +495.0pp 收益增量对应暴露仅 +3.7pp；
  若是纯暴露伪影，增量应 ≈ BH×Δ暴露 ≈ +3.2pp。增量主体来自**砍掉的
  146 笔出场恰是亏损段中段的假衰竭出场**（出场→低位重入的往返损耗）。
- 胜率 4/4 微升（出场被门拦截的腿，后续以更好价位出场）；BRN MDD
  显著改善（−59.9→−47.0），其余近不变。
- voice 短差 4/4 增加（611→704 / 2434→2599 / 4721→4816 / 6029→6417）：
  "sell1 出现但 master 不出 → voice 做短差"的设计意图有直接观测证据。
- **诚实边界**：BTC 的 +14.5pp 与暴露增量（+4.4pp×BH 量级）无法区分，
  BTC 上不能声明 HoldTrend 有效。

### 1.2 HighestOnly：否证（伪装 buy-hold）

- 笔数坍缩到 2-6 笔，暴露 ≈1.00，OKLO/BRN/BTC 的 sd=0（voice 整体失活，
  机制：master 不出场 ⇒ 无 FLAT 窗口 ⇒ 但 voice 失活另有原因——单笔
  超长持仓中 REV 腿仍可开，CL 例外 sd=4831，OKLO/BRN/BTC 失活与
  earning/锚域相关，未深挖）。
- BTC +1340.6% vs BH +1380.4%：**Δ(BH)=−39.8pp，全部"收益"就是 BH 本身**
  （2 笔、暴露 0.9997）。回测基准不可证伪陷阱先例：与 buy-hold 同义反复
  的配置无信息增量。_ho 的 MDD=0.0%/−1.6% 是 trade 级口径在 n<10 时的
  伪象，不是真实风险。
- OKLO −1059.7pp：丢掉中途 recL3 之下全部结构性出场+重入复利。
- **判决：HighestOnly 不是出场模式，是"删除出场"。否证。**

### 1.3 BTC 死因再定位（任务动机未解决）

HoldTrend 把 BTC 出场从 1462 砍到 1109（−24%），但只救回 14.5pp
（vs BH 缺口 −1374pp）。门拦截率 202/(202+1109)≈15%——41课门的保守语义
（父级别**正面延续证据**成立才拦截：相邻同向段创新高 ∧ 无盘整背驰；
段对不可定义 ⇒ 放行出场）在 4.6M bar 的单边行情中大部分 sell1 时点上
证据不可定义。死因不在单次出场时点，而在**出场基准级别 = entry 级
（segment 级 sell1 在 BTC 上 1254 次）本身太低**——每次出场→重入循环
平均回报为负（57.5% 胜率 × 小赢 vs 大输的 payoff 不对称）。
_ho 证明纯粹升到 max_ladder 又删除了全部操作。开放轴见 §4。

## 2. 定义依据

- **49课**："利润最大化"要求趋势中持币不动——HoldTrend 的存在论依据。
- **41课**："大级别走势没有任何衰竭迹象时参与反向小级别买卖点是刀口舔血"
  ——趋势衰竭判据 = `TrendExhaustion::up_unexhausted(entry_ladder+1)`
  （相邻同向 Up 段创新高 ∧ 当前段窗口内无盘整背驰 = 正面延续证据；
  `rust/src/trading/trend_exhaustion.rs` 在册实现，本任务仅消费）。
- **31课**"历史性大顶"的级别相对化——HighestOnly 的（被否证的）依据。
- **17课**趋势定义（≥2 同向中枢）：衰竭判据的"段"来自 D3 方向行
  （bar 末状态，close 分辨率——漏单非错单声明同 Fractal 子腿先例）。

## 3. 边界条件（结论何时翻转）

1. **配置域**：floor=2（segment 级入场）、V2oa25 默认门（θ 自适应 q=0.25 +
   成本门下界 + rev_l41_gate）、零摩擦、close 成交、1min 数据。换 floor
   或加摩擦需重验（_ht 减笔数 ⇒ 摩擦下相对优势只会扩大，方向稳健）。
2. **HoldTrend 4/4 全正**翻转条件：若在衰竭判据更敏感的标的（高噪声
   横盘主导）上门恒不拦截（holds≈0），增量退化为零——不会变负
   （门只延迟出场到衰竭确认，最坏 = 基线 + 延迟损耗）。CL +25.0pp 已是
   弱档样本。
3. **HighestOnly 否证**翻转条件：无。其 BTC"成功"与 buy-hold 同义反复，
   不可证伪（先例：project_backtest_benchmark_falsifiability）。
4. **earning_reaction 组合**：exit_mode≠Signal × earning_reaction 被 guard
   显式拒绝（出场升级语义建立在事件驱动上，状态驱动形态未设计）——
   V4 族变体不可直接换出场模式。
5. trade 级 MDD 口径在 n<10 时不可比较（_ho 行的 MDD 无意义）。

## 4. 下游推论

1. **V2oa25_ht 是新默认候选**：4/4 正增量 + BRN MDD 改善 + voice 短差
   增加（架构意图实现）。是否升级默认 = 选择类决断（编排者/Gemini decide），
   本报告不自决。
2. **状态范畴第四例**：级别确认（递归建仓）/确认深度（BSP 形态）/
   开放轴（方向状态）之后，**出场时机也是状态范畴非事件范畴**——sell1
   事件只是必要条件，趋势态结束才是充分条件。事件驱动→状态驱动的
   系统性替换方向获得出场侧证据。
3. **BTC 开放轴**：出场基准级别动态升级（持仓时长/盈利幅度驱动的级别
   爬梯，版本I动态级别归属先例）介于 entry 级（太低，BTC 死因）与
   max_ladder（删除出场，_ho 否证）之间。或：衰竭判据从"正面延续证据
   缺失即放行"改为"正面衰竭证据成立才放行"（对偶语义——但这会在段对
   不可定义的 warm-up 期锁死出场，需要显式设计）。
4. voice 短差在 _ho 下三标的失活（sd=0）而 CL 不失活——voice 开腿与
   master 持仓相位的耦合存在未刻画的依赖，若推进 §4.3 需先诊断。

## 5. 谱系引用

- **master入场侧递归建仓否证**（project_master_recursive_entry_falsified）：
  "级别确认是状态范畴"第三例——本任务是同一区分的出场侧应用（第四例）。
- **回测基准不可证伪陷阱**（project_backtest_benchmark_falsifiability）：
  _ho=伪装 buy-hold 的判决依据。
- **41课门先例链**：rev_l41_gate（REV 主腿）/ sub_l41_gate（Fractal 子腿）
  / FatigueGate（C7 点态门）——本任务把同一 TrendExhaustion 追踪器
  接到第三个消费点（master 出场），零新数据依赖。
- **θ自适应深度门**（project_theta_adaptive_gate）：V2oa25 基线定义。
- 不确定是否有"出场模式"专属谱系——据查无，本报告可作首条。

## 6. 影响声明

**代码改动（纯 Rust 链路，cargo test 127 项全过）**：
- `rust/src/trading/config.rs`：新增 `ExitMode` 枚举（Signal/HoldTrend/
  HighestOnly）+ `OrganicConfig.exit_mode` 字段（默认 Signal）+ 变体
  `V2oa25_ht`/`V2oa25_ho` + 单轴测试。
- `rust/src/trading/runner.rs`：guard 两条（HoldTrend 要求 D3 行 fail-fast；
  exit_mode≠Signal × earning_reaction 组合显式拒绝）+ TrendExhaustion
  实例化条件扩展 + master RIDE 出场判据按模式分派 + reason 后缀区分。
- `rust/src/trading/types.rs`：计数器 `n_exit_trend_holds`（py_items 导出）。

**O0≡P5 / 基线零接触守卫**：exit_mode=Signal 路径逐位不变——
OKLO/BRN 的 V2oa25 cell 与在册 cycle38_voice_backtest.json 对账 PASS
（n/total_compound/max_dd 逐键一致）；voice 短差（V2oa25 REV+域腿）
代码零接触。

**新增文件**：`analysis/master_exit_mode_backtest.py`（增量续跑，
BT_SYMBOLS/BT_FORCE 环境变量）+
`analysis/data_cache/master_exit_mode_backtest.json`（首跑误持久化逐 bar
years 数组致 149MB，已修复为 8.7KB——脚本同步修正）。

**未改动**：voice/VoiceUnit/SubLou/Cycle38 全部在册路径；earning 升级
出场路径（guard 拒绝组合，代码不变）；Python 信号层。
