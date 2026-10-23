# #1585 Sandcastle 评审工蜂结论（摘录）

来源：宿主 `.sandcastle/logs/sandcastle-issue-1585-reviewer.log`（review base `d07fc20438`，2026-10-21T10:15:13Z 起，10:18:54Z 完成）。

---

评审完成。我追加了 1 个修正提交，没有改功能语义；远端编译和 tb1585 测试都已通过。

**修正（`f5a4d46370`，`review: #1585——…`）**
- `capture_oos.rs` 里的 `readable_confirmed_len` 原来返回 `Option<usize>`。拒读分支随后又写了 `layer.segments_parse_invalid_from.unwrap_or_default()`，把一个已经确定是 `Some` 的值再取一遍，读起来像默认取 0。现在改成返回 `Result<usize, usize>`，`Err(失败笔号)` 直接带出笔号，这个 `unwrap_or_default()` 就不需要了。负控断言相应改成 `Err(0)`。

**验证（nc-remote，提交 `f5a4d46370`）**
- `cargo check --locked`：EXIT=0，日志在 `/tmp/nc-remote/20261021-181638-f5a4d46370.log`。
- `cargo test --locked --lib tb1585`：4 个测试全部通过，日志在 `/tmp/nc-remote/20261021-181735-f5a4d46370.log`。
  - 这一步是必须跑的：本分支的改动大多在 `cfg(test)` 下，`cargo check` 编译不到这些代码。
  - 日志里 `wverify_run.rs` 的两条 warning 在 main 上就有（3125 行那条在 main 上是 3120 行），不是本分支新增的。

**已看文件（9 个，全部看过）**
- backtest：`capture_oos.rs`、`segment_gn.rs`、`wverify_run.rs`
- classifier/diag：`cand_delta.rs`、`cp_recall_audit.rs`、`mod.rs`、`tb1585_tests.rs`
- parser：`mod.rs`、`tb1581_tests.rs`

**结论要点**
- **issue 列出的 5 处直读路径都处理了标记。**
  - `cand_delta`、`cp_recall_audit`、`cascade_dual`：整层拒读，口径和 #1581 的 classifier 入口相同。
  - `segment_gn`、`capture_oos` 的终局账本：只读失败段之前的部分（`segments_valid_prefix`），并在报告里写明段解析无效。
  - `capture_oos` 的逐 bar 增量部分：遇到标记的 bar 不按段确认开平仓；持仓期间碰到标记的交易标为受污染，不进任何统计，同时报出标记出现的 bar 区间。
  - 这些都只在内存里产出报告，不写生产账。
- **每处都有一条负控和一条正控。** `cand_delta_tower` 返回空 Vec 后，下游 `opsem_dump` 和 `cand_delta_entry_tower` 都用迭代或 `.get` 访问，不会越界。
- **commit message 带了票号和编号声明，改动没有超出 issue 范围，也没有写入密钥。**
- **一处行为说明（没改）：** `capture_oos` 在标记消失后的第一个 bar，会把标记期间积压的段确认一次性处理掉。这沿用了原有的高水位跳跃逻辑，没有新增交易路径，所以按语义不变的原则保留。如果要把这些积压的确认也当成受污染，那是新的取舍，需要另行决定。

我没有 merge、push，也没有动 main。

<promise>COMPLETE</promise>
