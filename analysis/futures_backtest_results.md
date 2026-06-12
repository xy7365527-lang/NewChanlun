# 期货标的多级别赋格回测 — 数据确认 + 在跑中报告（2026-06-11）

## 结论

1. **期货数据可用性：已确认充足，无需新拉数据。** `analysis/data_cache/` 在册 7 个
   期货标的的 databento 10 年 1min 连续合约数据，且 `fugue_v2_full_backtest.SYMBOL_FILES`
   已全部注册：

   | 标的 | 文件 | bar 数 | 时间跨度 |
   |---|---|---|---|
   | BRN（ICE Brent，用户实盘核心） | brn_1m_databento_10y.json | 2,434,555 | 2018-12 → 2026-06 |
   | CL（WTI 原油） | cl_1m_databento_10y.json | 5,528,156 | 2010-06 → 2026-06 |
   | ES（E-mini S&P500） | es_1m_databento_10y.json | 5,589,928 | ~10y |
   | GC（黄金） | gc_1m_databento_10y.json | 5,544,556 | ~10y |
   | ZN / 6E / DX | 同命名模式 | — | ~10y |

2. **发现并修复 BRN 坏 bar（数据质量缺陷）。** `2024-02-19 17:41` 整根 bar
   OHLC=0.08（前后均 83.56，孤立 spike-and-revert，数据源坏 tick）。原 `load_ohlc`
   只删 nan/≤0 拦不住，该 bar 会制造一根 −99.9% 的伪巨型笔污染 BRN 全部缠论下游
   结构。已在 `fugue_v2_full_backtest.load_ohlc` 加孤立坏 bar 清洗（判据：close 相对
   前 bar 跳变 >50% 且后 bar 回到前 bar ±5% 内）。扫描确认：ES/GC/CL 零坏 bar，
   BRN 仅此 1 根（2,434,555 → 清洗后 2,418,058，nan 清洗删 ~16K + 坏 bar 1 根）。

3. **回测在跑中（两条 orphan 进程，增量落盘，跑完自动产出）。**
   配置：floor=2（segment），变体 O0（≡P5 四面对账守卫）+ V0 + V2of。
   - 油链（BRN→CL）：日志 `analysis/data_cache/_futures_mlf_oil.log`，
     产出 `multi_level_fugue_backtest_fut_oil.json` / `multi_level_fugue_tables_fut_oil.md`
   - 指数链（ES→GC）：日志 `_futures_mlf_idx.log`，产出 `*_fut_idx.json` / `*_fut_idx.md`
   - 耗时主因：增量信号引擎随结构数累积放缓（期货 10 年行情反复 ≫ OKLO 447K，
     实测 ~350 bar/s vs OKLO ~18K bar/s）。BRN 预计 ~1h，CL/ES/GC 各数小时。
   - 查看进度：`tail analysis/data_cache/_futures_mlf_oil.log`
   - 续跑/补跑：`BT_SYMBOLS=BRN,CL BT_FLOORS=2 BT_VARIANTS=V0,V2of BT_OUT_SUFFIX=_fut_oil
     PYTHONPATH=src .venv/bin/python analysis/multi_level_fugue_backtest.py`
     （增量续跑，已完成 cell 自动 skip；磁带指纹守卫防引擎语义漂移混表）

## 期货特殊性核查

| 维度 | 现状 | 判定 |
|---|---|---|
| 合约展期 | 10y 文件为连续合约拼接（K4 谱系结论：必须 .v.0 成交量加权 roll，.c.0 日历 roll 数据稀疏已弃用）。**未做后向价差调整（non-back-adjusted）**：roll 日价差 gap 以真实跳变形式留在序列中，引擎会将其读为价格结构的一部分 | 已声明，见边界条件 |
| 交易时段 | 全 24 个 UTC 小时均有 bar（BRN 实测 hour 覆盖 00–23）——Globex/ICE 非常规时段**已囊括**，符合用户要求 | ✓ |
| 保证金 | 回测按全额现金建模（与股票口径一致）。期货实盘是保证金交易，杠杆放大同一信号的资金曲线但不改变信号本身；复利%/胜率/笔数可比，maxDD 换算到保证金口径需乘杠杆倍数 | 已声明 |
| nan 缺失 bar | BRN 0.68% nan（非交易分钟），load_ohlc 删除（在册标准预处理） | ✓ |
| 坏 tick | BRN 1 根（已修，见结论 2） | ✓ 已修复 |

## 定义依据

- 多级别赋格矩阵定义：`analysis/multi_level_fugue_backtest.py` 模块 docstring
  （master=entry_ladder 数据驱动动态级别，voice 域=[floor, entry_ladder)）。
- O0≡P5 对账：`organic_fugue_design.md` 谱系（O0 逐位守卫，trades/trace/counters 四面）。
- V2of：配对修正 REV（仅震荡型 + θ_depth=1% + kind 配对闭腿，在册最优变体）。

## 边界条件（结论何时翻转）

- **坏 bar 判据**：若某期货品种存在真实的"单 bar 跌 >50% 且下一 bar 完全收复"
  行情（闪崩闪回），清洗会误删。实测扫描四标的仅 BRN 一根纯坏 tick 满足判据；
  CL 2020-04 负油价为连续多 bar 下跌（后 bar 不回归前 bar ±5%），不被误删。
- **roll gap 未调整**：若 roll 价差 gap 在某品种上系统性巨大（远月升贴水深），
  连续合约的"笔/段"结构会含非交易性跳变，回测复利%与可交易收益出现偏差。
  BRN/CL 近月流动性 roll gap 通常 <1%（1min 序列实测 |Δ|>1.5% 跳变均为行情事件），
  该偏差在油品上可忽略；对深 contango 品种（VX 类）不成立。
- **BRN 在册旧结果**：V2r/fugue_v2 等在册 BRN 结果建立在含坏 bar 的序列上。
  磁带指纹守卫会在下次重跑时抓住该漂移——旧 BRN 结果与新口径**不可同表比较**，
  需重跑后更新（坏 bar 在 2024-02，影响 2024 之后的结构链）。

## 下游推论

- 若期货 V2of 与股票同号（Δ(vs V0) 转正方向一致），REV 配对修正的有效域从股票
  扩展到期货资产类，多级别赋格结论升 L3（跨资产）。
- 若 BRN/CL 与 ES/GC 分化，资产类（趋势型油 vs 均值回归指数）是有效域边界变量。

## 谱系引用

- 形式化有效域规则（222/223/230/231号）：本次期货跑为 L2→L3 跨资产验证步。
- K4 数据源谱系：连续合约 .v.0 口径结论复用。
- 090号严格性：坏 bar 修复走通用判据（spike-and-revert 合取），非硬编码日期特例。

## 影响声明

- 改动 `analysis/fugue_v2_full_backtest.py::load_ohlc`：新增孤立坏 bar 清洗——
  **影响所有经此加载的标的**（OKLO/BTC/QQQ 等实测零坏 bar，输出不变；BRN −1 bar，
  2024-02 之后全部缠论结构链漂移，在册 BRN 旧结果作废待重跑）。
- 改动 `analysis/multi_level_fugue_backtest.py`：新增 BT_FLOORS/BT_VARIANTS/
  BT_OUT_SUFFIX 环境开关（默认行为不变，OKLO 在册结果不受影响）。
- 新增两条在跑进程的产出文件（`*_fut_oil.*` / `*_fut_idx.*`），跑完后需人工
  （或下个 session）将表格并入主报告。

## 认识论等级

数据可用性/时段覆盖/坏 bar：L2（真实数据实测）。回测结论：**尚无**（在跑中，
本报告不预判方向）。
