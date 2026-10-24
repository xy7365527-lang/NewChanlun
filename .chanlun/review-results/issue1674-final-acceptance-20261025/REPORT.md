# #1674 最终验收报告：tb03c-book/2 新进程与 SIGKILL 恢复对拍

日期：2026-10-25。票据：#1674 / PR #1684，父义务 #1406 AC5。

## 结论

**代码与资金/恢复合同 PASS，交付材料已补齐；尚未合入 main，未关票。**

代码候选为 `055618db67d07bd418503dbf164078afbf8a2516`。最终非作者独审确认前序同根重跑和非 test 构建泄漏两项 HIGH 均已闭合，未发现新的 CRITICAL/HIGH 代码问题。独审原文见同目录 `INDEPENDENT-REVIEW.md`。

本目录只新增验收与评审文档，不改变代码、测试、冻结夹具或资金合同。extras 的三项 funding 失败仍属 main/#1621 集成门，不能宣称 CI 全绿。

## 实际代码提交面

基线为 `origin/main @ 3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f`。代码候选包含：

- `625fab529e6bc3fa9fdd5968a8a4d21d67eb2d6f`：/2 持久边界的新进程与 SIGKILL 恢复对拍。
- `d40d156d6691d75eda9109ef7386abe899bdd644`：plan 逐步核 E Pending 新增 slot 的 AwaitingPolicy。
- `31c85de5b34891c1ab66af5a77dfa84017b9debf`：cargo fmt。
- `842207240b724f33027e0736adc7692ee001fb8a`：对齐 #1621 可审计观察时钟夹具。
- `6848feea22d4c1934a838753de6d1ef3634b8862`：恢复矩阵支持同根重跑。
- `055618db67d07bd418503dbf164078afbf8a2516`：`tb03c_v2_kill_tests` 仅在 test 构建注册。

本次纯文档提交只入仓本报告与独审原文；代码评审面保持 `055618db67`，不以文档尾提交扩大代码变更面。

## AC 与运行证据

| 验收项 | 结果 | 证据 |
|---|---|---|
| 目标步 × before/after × 两次新进程恢复 | PASS | 13 个目标步，每轮 52 次真实 SIGKILL；连续两轮共 104 次。覆盖三 profile 开局 Capture/Apply、两类 E Pending Capture 与 5m L2 Apply。 |
| recover、重送、续跑与黄金值 | PASS | 提交前/后分别检查 StaleWriter/原收据；续跑终态与不中断基线及 `FUNDING-ORACLE.json` 三 profile 黄金值逐值一致。 |
| 无故障双跑确定 | PASS | 两次新进程完整重放的终态 canonical bytes/hash 相同；meta=`tb03c-book/2`，E/B audit 通过。 |
| pause 位移变异负控 | PASS | `625fab529e` 记录：before_commit 挪到 commit 后使对应 3/3 红；after_commit 挪到 commit 前使对应 3/3 红。后续差异未改 pause 生产语义。 |
| AwaitingPolicy 与冻结输入 | PASS | 未签 `funding_revision`/`q_rebase` 保持 typed AwaitingPolicy；未代填资金默认值，未改生产文件语义或既有冻结夹具。 |
| 同根连续重跑 | PASS | 同一 `TB03C_KILL_ROOT=/tmp/nc1674-reentrant-shared` 连续两轮均 29 passed / 0 failed / 1 ignored；不再复用旧库导致 IdentityConflict/DirectoryNotEmpty。 |
| 非 test 构建 | PASS | `055618db67` 仅补 `#[cfg(test)]`；当前 rust-check、四个 s_session shard 与 coverage 均 SUCCESS，旧 E0432 消失。 |
| 非作者独审 | PASS（代码/合同） | 同目录 `INDEPENDENT-REVIEW.md`。 |

双跑原始日志：`/tmp/nc-remote/20261024-174856-6848feea22.log`。该日志含 104 条 `V2-KILL` 记录和两轮 29/0/1 结果。最终 `6848feea22..055618db67` 仅增加 `#[cfg(test)]`，未改恢复矩阵；当前 head 的 CI 再次覆盖同一 29 个命名测试。

## Linux 实际用时

运行环境为 windows-5090 WSL / nc-remote，候选 `6848feea22`：

- 第一轮 `tb03c_v2_kill`：**18.79 秒**，29 passed / 0 failed / 1 ignored。
- 第二轮同命令、同一外部根：**16.77 秒**，29 passed / 0 failed / 1 ignored。
- 两轮完整命令含首次编译：**EXIT=0，84 秒**。
- 另行执行 `tb03c_` 全集：**330.34 秒**（整命令 376 秒），其中三项失败为下述 #1621 基线问题，不计为 #1674 新回归。

原始日志：`/tmp/nc-remote/20261024-174856-6848feea22.log` 与 `/tmp/nc-remote/20261024-175129-6848feea22.log`。

## 当前 CI

代码候选的 CI run：[40086376719](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40086376719)。

- SUCCESS：rust-check、rust-test-default、s_session shard 0/1/2/3、coverage、backtest-bin、s-observer、fixture-drift、sandcastle-ts、test。
- DevSkim：[40086376637](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40086376637)，SUCCESS。
- `rust-test-s-session-extras`：FAILURE，**250 passed / 3 failed / 10 ignored**，原日志测试体用时 730.43 秒。失败仅为：
  - `each_signed_record_window_gates_alone`
  - `opening_batch_negative_controls_are_typed_and_deterministic`
  - `v2_domain_binds_revision_waits_in_e_and_replays`

这三项与 main run `40005878482` 的既有 funding 回归一致，归 #1621 集成线；不属于 #1674 新增失败，也不因此写成 CI 全绿。

## 剩余交付门

- main 合入仍走用户批准闸；合入顺序与 #1621 的主线 funding 修复一并安排集成验收。
- 关票时仍须核对 resolution 引用的 main commit、关票声明真实性、工位清理和评审面==关票面。
- 本报告与独审已入仓，解决此前“PR 旧提交清单、Linux 用时缺失、仓内无最终验收报告”的材料缺口；不提前宣告 #1674 已关闭或 #1406 已完成。
