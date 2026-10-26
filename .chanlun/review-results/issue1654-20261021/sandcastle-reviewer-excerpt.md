# #1654 Sandcastle 评审工蜂结论（摘录）

来源：宿主 `.sandcastle/logs/sandcastle-issue-1654-reviewer.log`（review base `63ecebf8f8`，2026-10-21T11:17:41Z 起，11:22:36Z 完成）。

---

我评审了 `sandcastle/issue-1654` 相对 `63ecebf8f8` 的改动，追加了一个注释修正提交，没有发现必须改代码的问题。远端检查和测试都已通过。

**已看文件（12 个，全部过到）**
- `rust/src/recursive_t/` 下：`types.rs`、`stream.rs`、`rec_stream.rs`、`ffi.rs`、`mod.rs`、`divergence.rs`
- `rust/src/trading/` 下：`config.rs`、`level_sub_lou.rs`、`nested_fugue.rs`、`positional.rs`、`positional_fusion.rs`、`unified_osc.rs`

**核查结论**
- **只改了注释和断言消息**：12 个文件里没有任何判据或执行逻辑变化，`FIRST_BSP_LADDER`（`trading/types.rs:30` = 2）也没动。
- **验收 1 已满足**：`rg -n "笔 ?= ?a0|a₀ = 笔序列|a₀（笔序列）" rust/src` 没有命中。
- **引用的依据逐条对过，都成立**：
  - `zhongshu.md` §2.1.1 第 2 条（笔不在 `a₀` 值域内）；
  - `roadmap.yaml:970-972` 的 A/B 否证记录；
  - `ADR 0005` 和 `AGENTS.md` 里 #799 裁定六的对照臂口径；
  - #951 B3 关于冻结 `backtest_t_fugue.py` 的处置。
- **交易阶梯注释改得对**：`nested_fugue.rs` 注释里「`FIRST_BSP_LADDER - 1` 就是 bi 层」与 `MAX_LADDER` 的层号约定（0 = bar，1 = bi，2 = segment）一致。
- **提交纪律合规**：commit message 带票号和编号声明，改动范围就是 issue 里的 C2 和 C3 两组，没有写入密钥。

**修正（提交 `0183e92004`，`review: #1654——parse_a0 注释订正…`）**

问题出在「笔这一档不进任何准入路径」的证据上。原注释说 `parse_a0` 只服务冻结的 `backtest_t_fugue.py`，但实际上 `TFugueStream` 和 `RecTStream` 两个 Python 接口类都用它解析 `a0` 参数。其中 `RecTStream` 走的是 `new_production` 这个生产入口，所以原说法不准确，也拿不出证据。

我查了现有的 Python 调用者（`backtest_t_fugue.py:167`、`analysis/capture_ratio_matrix.py:85`、`analysis/t3_exit_trigger_diag.py:36`），三处都不传 `a0`，也就是全部走默认的线段（`Segment`）。`git log -S` 也显示 `backtest_t_fugue.py` 从来没传过 `a0`。我把这些事实写进了 `ffi.rs` 里 `parse_a0` 的注释，代码语义没有改。

**一个留给你判断的点（没改）**

`types.rs:76` 说笔这一档用来复现冻结链 `backtest_t_fugue.py` 的 24 个 `t_fugue_*.json` 缓存。但这个 Python 文件从来没传过 `a0`，所以这些缓存当初是不是通过笔这一档产出的，我在仓库里证实不了。这个说法沿用的是 issue 原文和 #951 B3 的口径，所以我没动。

**远端验证（nc-remote，在提交 `0183e92004` 上跑）**
- `cargo check --locked`：通过（EXIT=0，有既有 warning）。
- `cargo test --locked --lib recursive_t`：31 个通过，0 个失败。
- `cargo test --locked --lib trading`：414 个通过，0 个失败，7 个忽略。
- 测试计数是按名称过滤得到的实数；带 ignore 标记的全量回测 `rec_btc` 没有跑。

<promise>COMPLETE</promise>
