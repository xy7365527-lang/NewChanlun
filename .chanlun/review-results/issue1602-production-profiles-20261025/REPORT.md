# #1602 生产 profile 与准入交付报告（2026-10-25）

## 候选与结论

- 代码候选：`07e3443c5118bbdc3da2c3978f12576a1e5fece1`。
- 非作者尾部与整票复审：代码与合同 **PASS**，无新增 CRITICAL/HIGH，原文见 `REVIEW.md`。
- 当前状态：实现完成，尚未合入或关票；PR CI 的 extras 与 main/#1621 的 funding 基线仍是独立门。

## 三项前序阻断的闭合

1. 正式 S 在 Driver 开门、pending 恢复和 Begin 前构造并核验生产 profile；P0 未签 tick/成本时 fail-closed。既有测试只经显式 fixture 分派使用诊断配置。GenerationRuntime 使用同一配置，`Generated.policy_records` 只作运行元数据，不改生成态内容字节。
2. `theta_accept` 将配置/政策失败与数据失败/空窗分开；另一品种成功也不能使配置失败的整批通过。
3. 生产 profile 头损坏直接传播 `InvalidPolicy`，不再吞成 `RecordMissing`。

## 当前 head 验证

| 范围 | 结果 | 证据 |
|---|---:|---|
| 完整 S（排除 WSL 既有 macOS-only 项） | 517 passed / 0 failed / 12 ignored | `/tmp/nc-remote/20261025-043454-07e3443c51.log` |
| 前次失败集 | v2 23/23、S3 CLI 3/3、恢复 1/1 | `/tmp/nc-remote/20261025-042930-07e3443c51.log` |
| 生成与版本戳 | 5/5 | `/tmp/nc-remote/20261025-035704-d9a04f21fc.log` |
| theta_accept | 2/2 | `/tmp/nc-remote/20261025-043247-07e3443c51.log` |
| config / run_records | 10/10、8/8 | 同上 |
| s_session / backtest_bin 非 test 全目标编译 | PASS | `/tmp/nc-remote/20261025-042930-07e3443c51.log` |
| 新增生产负控 | 各 1/1 | `/tmp/nc-remote/20261025-033153-841a94391d.log` |

本机未运行 cargo/rustc/cargo test；均由 windows-5090 WSL 的 nc-remote 独立工位执行。

## 平台与基线

- 手工 macOS run [40130575778](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40130575778)：`v3::financial_session` **26 passed / 0 failed / 3 ignored**，详见 `MACOS.md`。复审后取得的该证据消除了报告中的 macOS NOT_VERIFIED。
- 同一 macOS run 的 economic 库测试为 222 passed / 3 failed / 9 ignored；失败项为 `each_signed_record_window_gates_alone`、`opening_batch_negative_controls_are_typed_and_deterministic`、`v2_domain_binds_revision_waits_in_e_and_replays`，与 main/#1621 原日志同名同断言，属既有基线，不是本候选新增 profile 回归。
- PR CI [40130421406](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40130421406) 其余已完结检查均通过；extras 仍须取得终态并核同一基线。不得写成全套 CI 全绿。

## 剩余门

extras 终态与主线 #1621 集成安排、纯文档尾提交后的 CI、main 合入批准及关票门。纯文档尾提交不改代码与评审面。
