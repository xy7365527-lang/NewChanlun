# #1585 专用退役工位已完成可恢复处置

三处工位已用非 force 命令移除，四个本地分支已按旧 SHA 比较删除。恢复包已从空裸仓实际导入，保留 squash 没有保留的原工蜂、harvest、集成与 P1 分支历史。处置执行时间为 `2026-10-26T04:08:02.703462+00:00` 至 `2026-10-26T04:08:07.733080+00:00`，来自实际执行日志。

本任务绑定 #1709，仅处置下列对象。没有改 main、远端分支、共享 S、root 图或服务，没有关闭业务票。

## 主控还需归档本报告，再发布关票声明

既有技术评审报告已在落地提交 `7fe402cd79b0afdad87ed2a828d6d75fb8ade07b` 的 `.chanlun/review-results/issue1585-20261021/`。本轮以 git ls-tree 确认 11 份文件实体，包含历史两轮审查、P1 首轮 BLOCK、F1 最终 PASS、返修记录和时序索引。清单保存在恢复目录的 `existing-main-reports.txt`。

本报告是新产生的工位处置实体，目前仅在仓外。按 `docs/agents/delivery-discipline.md` 关票门第 6 子句，新增报告的入 git 门尚未满足。建议主控逐字归档本报告到现有 issue1585 目录，并沿用纯文档流程。bundle、裸仓、命令脚本及工具原始输出继续留在仓外。主控完成该归档后再引用实际入 main 的相对路径关票。本任务没有自建 PR。

PR #1664 已现场回读为 MERGED，main squash 为上述提交。主线 [CI 40259428149](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40259428149) 的 `headSha` 为同一完整 SHA，状态为 `completed/success`。原始回读位于 `1585-closeout-recovery26/owner-and-ci-before.txt`。本轮不重跑 Rust、不扩大原验收。#1279 保持 open。

## 三处工位实际移除

| 工位 | 删除前 HEAD | common repo | 结果 |
|---|---|---|---|
| `/Volumes/AgentStorage/harvest-owner/pr-1585` | `f7ad4bcb227288818839d279c11d8510cfe62c80` | `/Users/silencehan/Projects/NewChanlun-sandcastle-host` | `git worktree remove` 成功，路径及登记均消失 |
| `/Volumes/AgentStorage/issue1585-integration-20261026` | `8ba59b157302ee6009376984fee74a6afe696c1d` | `/Users/silencehan/Projects/NewChanlun` | 同上 |
| `/Volumes/AgentStorage/issue1585-p1-repair-20261026` | `ae66abed0b4b1effed10e4524105b52f2b28991d` | `/Users/silencehan/Projects/NewChanlun` | 同上 |

三个路径均先从各自 `git worktree list --porcelain` 的登记核认。删除前再次核 HEAD、branch、tracked、untracked、ignored、工位及其专属 gitdir 的 lsof。所有状态输出为空，lsof 均无句柄，专属 index.lock 不存在。进程命令行没有三个路径的消费者。

Sandcastle `workers.jsonl` 与 `.sandcastle/logs/STATUS.md` 的 #1585 最后记录为 `sandbox closed`，时间 `2026-10-21T10:18:54.375Z`。当前团队无其他在飞 #1585 owner。侧栏唯一 active 是本主控线程，唯一 pinned Codex 线程为"接续 Wayfinder #1339"，没有指向三个专用工位。侧栏筛选原件在 `owner-sidebar.json`。GitHub assignee 仍为 `xy7365527-lang`，本任务没有清除认领。

## 四个本地分支实际删除

| common repo | 本地 ref | 删除前 SHA | 从 bundle 恢复的可达 commit 数 |
|---|---|---|---|
| host | `refs/heads/harvest/issue-1585` | `f7ad4bcb227288818839d279c11d8510cfe62c80` | 5746 |
| host | `refs/heads/sandcastle/issue-1585` | `f5a4d46370dec801ded70b54b94c3606916eb636` | 5735 |
| 主仓 | `refs/heads/codex/1585-integration-20261026` | `8ba59b157302ee6009376984fee74a6afe696c1d` | 5774 |
| 主仓 | `refs/heads/codex/1585-p1-repair-20261026` | `ae66abed0b4b1effed10e4524105b52f2b28991d` | 5779 |

host 即 `/Users/silencehan/Projects/NewChanlun-sandcastle-host`，主仓即 `/Users/silencehan/Projects/NewChanlun`。每个 ref 删除前均核验没有工位继续使用，并逐字比较旧 SHA。实际使用 `git update-ref -d <ref> <旧SHA>`，随后 `git show-ref --verify --quiet` 返回 1。没有使用 `branch -D`、`--force`、`rm -rf` 或全局 prune。其他 refs 不在本轮写入范围。

## 恢复包已通过实际恢复验证

恢复目录为 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1585-closeout-recovery26/`。

- `host.bundle` 含 host 两个精确分支的完整历史。
- `main.bundle` 含主仓两个精确分支的完整历史。
- `verified-host.git` 与 `verified-main.git` 是本轮从空裸仓导入 bundle 的实际恢复对象。
- `bundle-verification.json` 保存 bundle verify、空仓 fetch、四个 ref 的精确 SHA、与原仓一致的可达 commit 数及根 tree 读取结果。两个 bundle 均无外部前置历史。
- `preflight.json` 保存删除前的实际状态，`disposition.json` 保存每次删除前检查、命令退出码、路径消失及 ref 不存在的证据。
- `RECOVERY.md` 给出恢复用法。`restore-original-worktrees.sh` 可在原目标仍空闲时恢复四分支及三工位，目标存在即停止。脚本通过 `bash -n`。没有执行该恢复脚本重新创建退役工位。
- `prepare_recovery.py` 和 `dispose_recovered.py` 保留本轮可审阅的操作逻辑。它们不是项目实现代码，不入仓。

bundle 先于删除创建并验证。四个分支均以实际 bundle 导入结果证明可恢复，没有把 squash ancestry 当作原分支历史已保留。

## 空间读数与处置边界

删除步骤前后 `df -h /` 均显示 Avail `137Gi`。AgentStorage 从 Avail `376Gi` 变为 `380Gi`，这是文件系统显示精度下的 4 GiB 差额。三个工位删除前的 `du -sk` 分别为 `1441664`、`1455252`、`1455276` KiB。并行工作可能影响 df，不能把文件系统差额全归给本任务。恢复包及已验证裸仓继续占用外盘。

上游 cleanup playbook 原 audit 脚本会全量扫描两个 repo 并 fetch origin ref。当前任务只授权三个工位，因此本轮采用真实 worktree 登记加定点检查，未运行会写入其他 ref 的全量 audit。playbook 的 force 删除、rm -rf、全局 prune 和模拟器清理均没有执行。没有新占用或 WIP 需要保留，也没有对未授权对象作处置。

本轮应用 Prove It Works，选择从空裸仓导入 bundle 后才删原工位。应用 Sequence Work into Verifiable Units，选择每个工位删除后立即核路径和登记，再处置下一处。应用 Build the Lever，保存定点核验、恢复与比较删除脚本，使主控能审阅实际命令。上述原则叶技能均已在本轮全文读取。

