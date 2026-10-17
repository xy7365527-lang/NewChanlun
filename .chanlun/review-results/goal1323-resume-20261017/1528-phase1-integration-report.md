# #1528 / #1320-B：生产回放有限摘要接入与组件验证

状态：组件级实现与验证完成；未达到 #1304、#1323 的完整历史回放验收，#1528 不关闭。

## 接入

`replay_compare ES [MAX_BARS|YYYY-MM-DD]` 默认通过 `load_by_symbol` 加载生产数据，调用 `run_theta_v0_pi` 两次。旧 `MinimalEngine` 仅在 `--engine minimal DATA_PATH P123_DUMP_PATH MAX_BARS` 显式诊断模式执行。生产模式使用相同 `Dataset`、`ThetaConfig::default()`、年化窗口与初始净值，各自独立运行。

比较 `RunResult` 中九项选定结果：equity curve、daily returns、trade PnL、forced-close PnL、交易记录选定字段、订单数、R 分解、终态 closed-loop、稀疏 turn-class rows；逐项输出 SHA-256。f64 序列和 R 分解（八个 f64 加一个 i64）按字段位模式判等；交易选定字段、闭环终态和稀疏分类旁路按编码文本逐字节判等。哈希只用于收据。`metrics`、`prices`、`fee_rate`、`theta_return_mtm`、`strict_nest_sidecar`、`untradable_ratio`、`is_l2` 等公开字段未比较。

## 组件验证（2026-10-18，macOS）

- `CARGO_TARGET_DIR=/Volumes/AgentStorage/cargo-target-1320b cargo check --features backtest_bin --bin replay_compare`：exit 0；仅有仓内既有 warning。
- `rustfmt --check --edition 2021 rust/src/bin/replay_compare.rs`：exit 0。
- `git diff --check`：exit 0。
- `replay_compare ES 600`：exit 0；600 bars，0 orders，0 closed trades，本比较器所选九项按各自编码比较相等；不是完整 RunResult 或全部字段的逐位一致性证明。
- `replay_compare ES 20000`：两个独立进程各 exit 0；两份完整 stdout 经 `cmp` 比较 exit 0；20,000 bars，0 orders，0 closed trades。原始输出为 `1528-es-20000-run1.txt` 与 `1528-es-20000-run2.txt`。

ES 输入 `analysis/data_cache/es_1m_databento_10y.json` SHA-256 为 `19bcf944b1da8b6b22de7ccb7acfa6101ddf13db8069cd6a0d47d635aced4aa4`；测试二进制 SHA-256 为 `804ae6676d613b760aa922778e43cca1b4f6f651e97ee01415edc92a30444161`。

## 尚未证明

20,000 bar 样本未产订单或平仓交易；空交易记录的相等不能证明交易路径。当前 `RunResult` 不暴露逐 bar 分类、信号、订单、成交、持仓和账本事件 tape，因而 CLI 未做这些维度的全量对拍，也未做完整历史数据或恢复验收。所需生产 tape、来源唯一性与回归锁见 `1528-phase2-tape-requirements.md`；正式输入与全图验收仍受上游 blocker 约束。

## 独立评审后修订（2026-10-18）

独立评审指出 `RDecomposition` 的 `Debug` 文本会把不同 NaN payload 都打印为 `NaN`，原实现不能称该项逐位；同时指出文档误称比较所有公开字段，以及不同长度的 f64 序列未报告前缀边界。修订后 R 分解的八个 f64 字段显式 `to_bits()` 编码、i64 桥字段精确编码，错误报告点名首个差异字段；长度差异报告共同前缀末端索引。设计与本报告已收窄到九项选定字段；`closed_loop_final` 和 `turn_class_rows_sparse` 的 `Debug` 文本比较仍不是内部浮点逐位证明。

靶向验证：`CARGO_TARGET_DIR=/Volumes/AgentStorage/cargo-target-1320b cargo test --features backtest_bin --bin replay_compare` exit 0，4/4 通过（不同 NaN payload、正负零、R 分解具体字段、Option 存在性、序列前缀长短）；`cargo build --features backtest_bin --bin replay_compare` exit 0；`rustfmt --check` 与 `git diff --check` exit 0。修订后 `replay_compare ES 600` exit 0，原始输出为 `1528-es-600-rdecomp-revision.txt`，仍为 0 orders / 0 closed trades。新二进制 SHA-256 `7fef84f9a1abf6ead9aaa408a90a025767504e94c193a2a6735a19ecf6ae2faf`。

上文两份 20,000 bar 独立进程收据属于修订前提交 `a6455d4f7a`、二进制 SHA-256 `804ae6676d613b760aa922778e43cca1b4f6f651e97ee01415edc92a30444161`，不能移作修订后的跨进程证据。
