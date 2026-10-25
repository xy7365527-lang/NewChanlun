# #1678 health-monitor 真实自愈验收（2026-10-24）

## 候选与前置

- 代码：PR #1682 已 squash 合入 main，关票候选为 main `3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f`；原 PR head `025934a1e6fd07b6ed6c554bb8246d4d20875be2`。
- 执行前宿主 `NewChanlun-sandcastle-host` 同为该 main 提交，`git status --porcelain` 为空；wayfinder 可拾取 0、running 0，无 Sandcastle implementer/reviewer 进程。
- PR CI [39671690664](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39671690664) 为 success，DevSkim [39671690616](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39671690616) 为 success。
- 非作者 Sandcastle reviewer 审过实现并修正 `heal` 名称遮蔽；主控另核尾部三行重命名 diff，并由本次真实入口覆盖该调用。

## 实际操作

执行记录文件时间：`2026-10-24T18:24:00+0800`。从 claim-loop 之外的 Codex 主控进程执行；没有在飞 Sandcastle 工蜂。恢复入口在执行前确认为仓内 `.sandcastle/launchd/com.newchanlun.sandcastle-claimer.plist`。

1. `launchctl bootout gui/<uid>/com.newchanlun.sandcastle-claimer`：退出 0；claim-loop 进程从 3 个变 0（见 `bootout.log`）。
2. 在宿主运行 `npx tsx .sandcastle/health-monitor.mts --heal --report <heal.json>`：退出 0；先报 `RED claim_loop: claim-loop 进程不在`，随后记录 `未加载，已 bootstrap …，进程已起（复判：healthy）`（见 `heal.log`、`heal.json`）。
3. 再运行 `npx tsx .sandcastle/health-monitor.mts --report <after.json>`：退出 0；`claim_loop` 为 healthy，最近一轮未 fail-closed，整体 healthy（见 `after.log`、`after.json`）。
4. 结束时复核 claim-loop 已恢复为 3 个进程；未执行仓库清理、issue 关闭或 main 合入操作。

## 验收对照

- 单测与 tail 修正：PR CI 的 `sandcastle-ts` 已绿；reviewer 记录为 14/14。
- red → bootstrap 自愈 → 下一轮 ok：本目录三份原始 log 与两份脱敏 JSON 为实体证据，满足票面最后缺失的实证项。JSON 仅删除 `checks[*].url` 的回环端点，check 名称、状态、HTTP 状态、模型数、动作与恢复结果均保持原值；脱敏不改变验收语义。
- 无密钥写入：归档只含进程计数、服务动作与 health checks；不含 launchd 环境转储或回环 IP 字面量。
-  main CI 照实记录：[40005878482](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40005878482) 总体为 failure，失败在 `rust-test-s-session-extras` 的三项 funding 回归，由 #1621 承接；同 run 的 [sandcastle-ts](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40005878482/job/120593517048) 为 success。本票不把这轮主线红改写成全绿。

## 边界

本报告只补 #1678 的自愈实证，不关闭 #1547，也不证明 #1323 的其它 Destination。证据 PR 合入、工位处置与关票面复核仍按关票门执行。
