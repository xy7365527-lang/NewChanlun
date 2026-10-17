# #1528 replay_compare 接入设计（2026-10-18 实施口径）

## 问题与边界

旧 `replay_compare` 只驱动 `MinimalEngine`；`ReplayComparator` 主要校对旧 D4/p123 快照协议，不能证明生产分类、π_Θ、订单、持仓与账本一致。生产入口 `theta_v0::backtest::runner::run_theta_v0_pi` 返回 `RunResult`，但尚无逐 bar 业务事件 tape。

## 当前组件实现

默认 CLI `replay_compare SYMBOL [MAX_BARS|YYYY-MM-DD]` 走 `run_theta_v0_pi`。旧 `MinimalEngine` 只在 `--engine minimal DATA_PATH P123_DUMP_PATH MAX_BARS` 显式诊断模式执行，不作为生产验收替代或严格判据的失败兜底。

两次独立生产 runner 使用相同数据、配置、年化窗口与初始净值。只比较九项选定结果：equity curve、daily returns、两类 trade PnL、交易记录的选定字段、订单数、R 分解、闭环终态及稀疏 turn-class rows。f64 序列与 R 分解逐位比较；闭环终态和稀疏分类旁路目前按 `Debug` 文本逐字节比较。每项输出 SHA-256 收据，但判等使用原始编码。`metrics`、`prices`、`fee_rate`、`theta_return_mtm`、`strict_nest_sidecar` 等公开字段并未纳入本阶段；更没有逐 bar 事件 tape。

## 终态缺口

生产逐 bar classification、π_Θ signal、order、fill、position、ledger 事件流没有统一出口；当前只能覆盖公开的权益、收益、交易字段、订单数、R 分解、稀疏分类旁路与闭环终态。详见 `goal1323-resume-20261017/1528-phase2-tape-requirements.md`。正式完整历史回放、来源唯一性、恢复和回归锁留在 #1323/#1304 全图验收链，不能凭此组件结案。
