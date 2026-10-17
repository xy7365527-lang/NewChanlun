# #1528 / #1320-B：生产回放有限摘要接入与组件验证

状态：组件级实现与验证完成；未达到 #1304、#1323 的完整历史回放验收，#1528 不关闭。

## 接入

`replay_compare ES [MAX_BARS|YYYY-MM-DD]` 默认通过 `load_by_symbol` 加载生产数据，调用 `run_theta_v0_pi` 两次。旧 `MinimalEngine` 仅在 `--engine minimal DATA_PATH P123_DUMP_PATH MAX_BARS` 显式诊断模式执行。生产模式使用相同 `Dataset`、`ThetaConfig::default()`、年化窗口与初始净值，各自独立运行。

逐位比较公开 `RunResult` 的 equity curve、daily returns、trade PnL、forced-close PnL、交易记录选定字段、订单数、R 分解、终态 closed-loop、稀疏 turn-class rows；逐项输出 SHA-256。哈希用于收据，实际判等使用 f64 的 IEEE-754 bit pattern 或编码文本的完整比较。

## 组件验证（2026-10-18，macOS）

- `CARGO_TARGET_DIR=/Volumes/AgentStorage/cargo-target-1320b cargo check --features backtest_bin --bin replay_compare`：exit 0；仅有仓内既有 warning。
- `rustfmt --check --edition 2021 rust/src/bin/replay_compare.rs`：exit 0。
- `git diff --check`：exit 0。
- `replay_compare ES 600`：exit 0；600 bars，0 orders，0 closed trades，所有已公开字段逐位相等。
- `replay_compare ES 20000`：两个独立进程各 exit 0；两份完整 stdout 经 `cmp` 比较 exit 0；20,000 bars，0 orders，0 closed trades。原始输出为 `1528-es-20000-run1.txt` 与 `1528-es-20000-run2.txt`。

ES 输入 `analysis/data_cache/es_1m_databento_10y.json` SHA-256 为 `19bcf944b1da8b6b22de7ccb7acfa6101ddf13db8069cd6a0d47d635aced4aa4`；测试二进制 SHA-256 为 `804ae6676d613b760aa922778e43cca1b4f6f651e97ee01415edc92a30444161`。

## 尚未证明

20,000 bar 样本未产订单或平仓交易；空交易记录的相等不能证明交易路径。当前 `RunResult` 不暴露逐 bar 分类、信号、订单、成交、持仓和账本事件 tape，因而 CLI 未做这些维度的全量对拍，也未做完整历史数据或恢复验收。所需生产 tape、来源唯一性与回归锁见 `1528-phase2-tape-requirements.md`；正式输入与全图验收仍受上游 blocker 约束。
