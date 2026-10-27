# #1604 Sandcastle 评审工蜂结论（摘录）

来源：宿主 `.sandcastle/logs/sandcastle-issue-1604-reviewer.log`（review base `02b5b571eb`，2026-10-21T09:53:03Z 起，10:01:47Z 完成）。

---

我评审了 `sandcastle/issue-1604` 相对 `02b5b571eb` 的改动。发现一处越出 issue 范围的改动，已经修正，并追加了 1 个提交 `64af6270e8`，输出不变。

## 修正

**`review: #1604——撤回越界的旧线 rust/src/segment.rs 相切接线（归 TB-10）`**
- **问题**：issue 的「不在本票」节写明，旧线 `rust/src/segment.rs:410/:413` 已裁定、归 TB-10。分支却把旧线的三笔重叠和缺口判断都改成了读记录，还为此给三个 bin（`p940`/`p944`/`p946`）各加了一个 `mod theta_v0` 转接模块。
- **改法**：
  - 这 4 个文件恢复成基准版本。
  - 删掉因此没人再调用的 `Tangency::separates`（`config.rs`）、`active_segment_construction_cached`（`policy_records.rs`）及其测试断言。
  - Python 侧的 `separates` 仍被 `a_segment_v1.py` 使用，保留。
- **为什么输出不变**：旧线改前写死的 `<=` 和 `>`，与签署值 `closed` 逐位相同。
- **副作用**：记录以后改签 `open`，旧线不会跟着变。这已在 `IMPLEMENTATION.md` §7.3 写明，留给 TB-10 处理。
- **文档**：`IMPLEMENTATION.md` §2、§4、§7.3 按撤回后的实际情况改写。`REVIEW.md` 正文不动，只在末尾追加一段：说明原审查对象 `b388916287` 之后修订成了 `703e3d1c7f`，并标出原文中关于旧线的两条已经失效。

## 看过但没改的地方
- **核心逻辑**：`probe_tail_excluded` 的结构论证（尾窗 ≥ 3 时不会置截断标记，`n-2` 每次重探）、增量与全量标记采用同一判定、`skipped_secondkind` 与主循环重复设置时结果一致，这几处都成立。
- **Python v0 口径变了**：`a_segment_v0.py` 从严格 `<` 改为 `closed`（即 `<=`），相切输入上 v0 的输出会变。这在 issue 范围内：issue 要求 Python P1 同读该字段，而 `xianduan.md` 把 P1 写作 `a_segment_v0.py`；实施报告 §7.1 已如实登记，所以没动。
- **常量版本号**：`SECOND_SEQ_SCAN_BUDGET_POLICY` 仍是 `/1`，代码注释和报告都已登记，留给后续票。
- **独审**：分支自带的独审是同模型子代理做的，`REVIEW.md` 自己也注明了。合入门如果要求跨模型或人工审查，还需另外补做。

## 验证（远端 nc-remote，本机没跑 cargo）
- `cargo check --locked --all-targets`：通过（EXIT=0）。三个 bin 的告警来自经 `#[path]` 编入的旧线模块里的死代码，回退后这些文件与基准一致。
- `cargo test --lib -- policy_records tb1604 config:: parser::segment::tests`：55 个通过。
- `cargo test --release --lib -- tb02 tb1493 tb1581 segment`：160 个通过，27 个忽略。
- `cargo test --test theta_v0_lean_parity`：1 个通过。
- Python `tests/test_segment_policy_1604.py`：19 个通过。
- clippy 没验证：远端没装 cargo-clippy，与实施报告的登记一致。

## 已看文件
- **核心**：`parser/segment.rs`、`parser/feature_seq.rs`、`parser/mod.rs`、`parser/second_kind.rs`、`parser/profile.rs`、`config.rs`、`policy_records.rs`。
- **记录**：`policy_records/second-seq-scan-budget.json`、`policy_records/segment-second.json`。
- **测试**：`tb1604_tests.rs`、`tb02c_tests.rs`、`tb02d_tests.rs`、`tb02w_tests.rs`、`tests/theta_v0_lean_parity.rs`、`tests/test_segment_policy_1604.py`。
- **其他**：`rust/src/segment.rs`、三个 `p94x` bin、`s_session_v2/tb02c.rs`、`a_segment_v0.py`、`a_segment_v1.py`、`segment_policy.py`、`IMPLEMENTATION.md`、`REVIEW.md`。

没有 push，也没有 merge。

<promise>COMPLETE</promise>
