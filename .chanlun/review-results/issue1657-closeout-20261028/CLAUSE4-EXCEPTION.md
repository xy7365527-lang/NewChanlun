# #1657 关票门第 4 子句例外登记：PR #1710 以 merge commit 合入

`docs/agents/delivery-discipline.md` 关票门第 4 子句要求：多 commit 交付逐个声明可编译，且合入 `main` 时用 squash 合成一个可编译提交。PR #1710 于 2026-10-28T11:07Z 以 merge commit `df0ec357ab`（父 `0b3097aae8` + `8f02689951`）合入 `main`，未 squash。`main` 不改写历史，squash 一项无法事后补做，本文件登记该例外。逐 commit 可编译一项不靠例外，下表逐个给出证据。

## 逐 commit 可编译性

范围：`df0ec357ab^1..df0ec357ab^2` 中属于本票的 6 个提交（合并带入的 `main` 侧提交按其自身在 `main` 上的历史计，不在本票列举范围）。

| commit | 内容 | 可编译证据 |
|---|---|---|
| `c7301cf768` | Rust 签发/撤销 + 合同 + 测试 | nc-remote（WSL）`cargo test --features s_session --bin s_structure_session admission_prepare_`，日志首行 `sha=c7301cf768c99d22d559691046a0a2537b783eb9` → `test result: ok. 14 passed; 0 failed`，`EXIT=0` |
| `5a06675c88` | launcher 合同门、workflow、ADR、测试 | nc-remote（WSL）`--list` + `cargo test … admission_prepare_` + `cargo build --release …`，日志首行 `sha=5a06675c88d711cf6460f79f63ad596cb449cd4d` → `14 passed`、`Finished release`、拒签输出 `{"error":"AdmissionContractAwaitingMeasurement","ok":false}`，`EXIT=0` |
| `507c3fed75` | 纯文档（ADR 改名 0047→0049、ADR 0039 一行） | 纯文档豁免；另有 CI [Admission run 40200729673](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40200729673) → success |
| `1e171dc1f7` | merge `main` f7f57ce4 | CI [Admission run 40447105683](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40447105683) → success |
| `3409e9e245` | 空提交（触发 CI，无树变化） | 树同 `1e171dc1f7`；CI [Admission 40448480371](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40448480371)、[CI 40448480477](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40448480477) → success |
| `8f02689951` | merge `main` 58039c60（PR 合入头） | CI [Admission run 40667559702](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40667559702) → success；CI run 40667559868 全部 job success |

两条 nc-remote 日志原件在 `/Volumes/AgentStorage/issue1657-admission-cli-20261025/rust-first-tests.log`（:1、:2577、:2579）与 `rust-final-release.log`（:1、:5150、:8444-8446），体积 78 KB / 255 KB，未入仓；上表摘录了判定所需的首行身份与结论行。

## 豁免声明

豁免声明：PR #1710 已以 merge commit `df0ec357ab` 并入 `main`，改为 squash 需要改写 `main` 历史；证据：`git show -s --format='%H %P' df0ec357ab` → `df0ec357ab804a04b7a2e9ae77e50391c7c7317a 0b3097aae8… 8f02689951…`（两个父提交）；因此豁免第 4 子句中「合入 `main` 时用 squash」一项。逐 commit 可编译一项不在豁免范围，由上表证据覆盖：本票 6 个提交均可编译或属纯文档/空提交。

同类先例：#1394 PR #1573（`.chanlun/review-results/issue1394-ac123-closeout-20261019/PERCOMMIT-CHECK.md`），该例另有 12 个不可编译中间提交；本例没有。

## 后续

#1657 收尾 PR（分支 `codex/1657-closeout-20261028`）按单一提交交付，合入时用 squash。
