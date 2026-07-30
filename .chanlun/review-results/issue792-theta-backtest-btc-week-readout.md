# theta_backtest BTC 2024-01-01..2024-01-07 实测读数（issue #792，2026-07-30）

命令：`./target/release/theta_backtest BTC 2024-01-01 2024-01-07`（release + backtest_bin feature）
数据：`analysis/data_cache/btc_1m_full.json`（314MB，gitignore 内，磁盘实存）
退出码：0。原始日志 26108 行（含 nautilus 逐行 INFO 与 banner），本文件仅留读数。

```
=== S_Θ 回测结果（theta_v0::run_theta_v0_pi）===
品种            : BTC
日期窗          : 2024-01-01 .. 2024-01-07
bar 数          : 10080
不可交易占比    : 0.00%
初始 NAV        : 422986100.00
年化基数(years) : 0.0192
--- 订单/交易 ---
订单数          : 3136
成交交易笔数    : 2482
--- 指标（含浮盈口径，runner §3）---
strat_return    : -0.1025
buy&hold_return : 0.0385
CAGR            : -0.9965
Sharpe          : -0.3749
MaxDrawdown     : 0.1120
win_rate        : 0.2752
--- 认识论等级（formalization-validity-domain 231号）---
等级: L2（真实数据 + 非空订单流）——结果可被否证。
★L2 ≠ 证明 Θ 盈利：单标的单窗，否证鲁棒性需 L3（多标的多窗）。
--- ⑥ 真实 Nautilus BacktestEngine（生产引擎贯通验证）---
2026-07-30T18:43:29.579098000Z [INFO] TRADER-001.BacktestEngine: =================================================================
```

## 第⑥段：真实 Nautilus BacktestEngine 结论

```
等级: L2（真实数据 + 非空订单流）——结果可被否证。
2026-07-30T18:43:30.520342002Z [INFO] TRADER-001.nautilus_backtest::engine: Iterations: 10_080
2026-07-30T18:43:30.520342003Z [INFO] TRADER-001.nautilus_backtest::engine: Total events: 6_655
2026-07-30T18:43:30.520342004Z [INFO] TRADER-001.nautilus_backtest::engine: Total orders: 2_754
2026-07-30T18:43:30.520342005Z [INFO] TRADER-001.nautilus_backtest::engine: Total positions: 177
2026-07-30T18:43:30.524983001Z [INFO] TRADER-001.nautilus_backtest::engine: Expectancy:                     -4772.11
2026-07-30T18:43:30.524984002Z [INFO] TRADER-001.nautilus_backtest::engine: PnL (total):                    -230553.08
2026-07-30T18:43:30.524984003Z [INFO] TRADER-001.nautilus_backtest::engine: PnL% (total):                   -23.06
2026-07-30T18:43:30.524984004Z [INFO] TRADER-001.nautilus_backtest::engine: Win Rate:                       0.01
2026-07-30T18:43:30.524986000Z [INFO] TRADER-001.nautilus_backtest::engine: PnL (total):                    0.25
2026-07-30T18:43:30.524986001Z [INFO] TRADER-001.nautilus_backtest::engine: PnL% (total):                   0.00
2026-07-30T18:43:30.525003002Z [INFO] TRADER-001.nautilus_backtest::engine: Profit Factor:                  3.49
引擎迭代次数    : 10080
总订单数        : 2754
总持仓数        : 177
等级: L2（真实 Nautilus 引擎 + 非空订单流）——生产引擎贯通验证通过。
```
