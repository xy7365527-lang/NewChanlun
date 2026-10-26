# #1621 / #1622 时钟与 SIGKILL 恢复集成

日期：2026-10-24  
集成工位：`/Volumes/AgentStorage/issue1621-1622-clock-integration-20261024`  
目标 PR：#1688；本候选同时 `Refs #1621 #1622`，不单独宣告任一票关票。

## 集成范围

- 合并 `sandcastle/issue-1622@e17fd4c90e` 与 `codex/1621-main-regression-20261024@ab40e7fe09`，合并提交 `ebe6938e8b`。
- 保留 #1621 的 `B.first_known_ns >= E.commit_ns` 正式写入与恢复审计判据。
- 保留 #1622 的 BindVoice 合法证据、真实 SIGKILL 矩阵、完整终态对拍和变异负控。
- `04619b6500` 只改 `tb03c_kill_tests.rs`：E 被杀并恢复后，三个 B 也分别由新进程 recover，后续写操作使用 epoch 2；时钟规范化同步按该实际恢复轨核商。B 被杀仍只恢复被杀 B，不扩大影响面。
- `afb905521a` 为 `cargo fmt`，无行为改动。

## 红 → 绿证据

未修集成候选 `ebe6938e8b` 在 `tb03c_kill_s00_capture_open_n1::before_commit` 失败：E 重送提交进入第二时钟列，B 仍用第一列，续跑第 1 步返回错误而非 Applied。

- RED：`/tmp/nc-remote/20261024-173523-ebe6938e8b.log`（1 failed，EXIT=101）。
- GREEN：`/tmp/nc-remote/20261024-173849-04619b6500.log`（同用例 1 passed，EXIT=0）。

## 靶向验证

环境：windows-5090 WSL / `nc-remote` / 独立编译 target；本机未运行 Cargo。

`04619b6500` 的 `/tmp/nc-remote/20261024-174238-04619b6500.log`：

| 检查 | 结果 |
|---|---|
| E Capture 提交前/后 SIGKILL | 2 passed |
| E Resolve 提交前/后 SIGKILL | 2 passed |
| BindVoice 提交前/后 SIGKILL | 2 passed |
| 跳过重送与错误 allocation 负控 | 3 passed |
| `tb03c_funding_tests`（含原三项主线回归及 E→B 负控） | 14 passed |
| `tb03c_chain_tests`（含 S→B 负控） | 3 passed |

同一日志末尾的 `cargo fmt --all -- --check` 报一处格式差异；已按 rustfmt 修正为 `afb905521a`，行为代码未变。

- 格式终验：`/tmp/nc-remote/20261024-174843-afb905521a.log`（EXIT=0）。

## 尚需门禁

- 推送后的完整 CI 与 22 步全矩阵结果；本报告不以靶向测试替代完整 CI。
- 对最终完整 diff 的非作者独审。
- #1621 的 X 执行域仍按原 AC 核定承接；本候选没有把 B 内转移称为 X。
- main 合入、#1621/#1622 关票及 #1406 后续验收仍走原人工门与完整关票门。

## 2026-10-25 返修：BindVoice 丢失负控与 C4b 恢复时序

- `888fb99f97` 增加步骤 21 的 `skip_resend` 负控：跳过重送后，全表比较明确报域 1 Voice 差异，公开查询比较失败，直接 `QueryVoice(kill-voice-1)` 返回 `MissingDependency：Voice 尚未绑定`。公开查询对拍新增 QueryVoice 覆盖。
- 同提交远端日志 `/tmp/nc-remote/20261025-013852-888fb99f97.log`：新负控 1/1、BindVoice 正常 SIGKILL 2/2、全部负控 4/4、rustfmt 全过。
- 上一轮 CI 40083596265 的 extras 唯一失败是 C4b 恢复后 E 被重复恢复、领先 B 一个时钟列；不是主线旧 funding 红。`5a0ac0b14b` 只恢复三个 B 追上已恢复的 E，生产时钟门不变。
- `/tmp/nc-remote/20261025-015017-5a0ac0b14b.log`：原失败 C4b 用例及两个相关恢复用例各 1/1，funding 14/14，rustfmt 全过。
- 非作者 HIGH-1 已返修，待新 head 补审与完整 CI。#1621 的 X 域仍按原 AC 另行承接，不把 B 内转移称为 X；两票保持 OPEN。
