# #623 S3 修复轮收口报告（claude sonnet）

> 日期：2026-07-29；性质：修复轮 stdout 收口报告原文（影子评审 HIGH-1/M1/M2/M3 钉正）
> 原始日志：/tmp/issue623-fixround-20260729.log

## 收口输出

**1. commit SHA + stat**
`4767db1a3c` — `fix(theta): #623 S3 修复轮——短差档三锁语义钉正（影子评审 HIGH-1/M1/M2/M3）`
3 files changed, 198 insertions(+), 11 deletions(-)：`book.rs` +11、`portal.rs` +61/-11、`tests/short_retrace.rs` +137（新增）

**2. 指纹对照**（本轮实测 vs 修复前基线）

| 项 | 命令 | 结果 |
|---|---|---|
| 编译 | `cargo build --lib --tests` | 零 error，`retrace_ledger`/`portal.rs` 零新增警告 |
| retrace_ledger 单测 | `cargo test --lib retrace_ledger` | 97 passed / 0 failed（含本轮新增 5 条） |
| 全量 lib | `cargo test --lib` | 2180 passed / 0 failed / 137 ignored |
| doctest | `cargo test --doc retrace_ledger` | 1 passed（compile_fail 仍生效） |

**3. HIGH-1/M1/M2/M3 逐项落位**

- **HIGH-1**（consume 毒化）——`portal.rs:255-266`（原 243-248）：改为先查 `records` 存在性再写 `consumed`，未登记身份返回 `None` 且不留副作用。测试：`tests/short_retrace.rs::consume_on_unregistered_identity_does_not_poison_future_sync`。
- **MEDIUM-1**（去重退化为条数）——`portal.rs`：`sync` 由 `or_insert` 改 `insert`（账本为真相，无条件订正）；`balances_with` 改为身份+内容双重比对（bijection）。测试：`balances_with_rejects_bogus_identity_absent_from_ledger`（P4）、`sync_corrects_identity_whose_recorded_content_deviates_from_ledger`（P5）。
- **MEDIUM-2**（判败侧镜像不变量缺失）——`book.rs:assert_entry_invariants` 新增 `Invalidated{RetestReentered}` 镜像断言，与 Confirmed 侧对称。测试：`assert_invariants_catches_tampered_failure_without_retest_position`（P6，`#[should_panic(expected = "判败必带回抽位置")]`，命中 `assert_invariants()` 而非读面 `short_retrace_records()`）。
- **MEDIUM-3**（通知/consume 两路脱节）——`portal.rs::disposal_notices` 改为过滤 `consumed`，与 `consume` 共享同一判重；`RetraceLedger::failure_disposal_notices`（book 层无门户实例）保留为无门控原始源，doc 显式声明两路分工。测试：`disposal_notices_stop_after_the_record_is_consumed`（P7）。

**4. 偏离/存疑**：无。禁区（adapter/book 已验收面除 assert_entry_invariants 一处新增断言外/log/audit/ledger_kernel/#576 拆分面）未触碰；既有测试零改动（只追加）；LOW-1~4 按票面不动，另议。
