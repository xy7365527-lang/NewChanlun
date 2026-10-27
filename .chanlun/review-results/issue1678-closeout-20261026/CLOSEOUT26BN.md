# #1678 收尾核查 26BN

#1678 的旧 main CI 失败已由 PR #1720 修复。两个干净工位已可恢复移除。MACD 工位与旧本地分支保留，票仍 OPEN，待主控验收和最终处置。

## 已交付面

- PR #1682 已合入 main `3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f`，原 head `025934a1e6fd07b6ed6c554bb8246d4d20875be2`。
- PR #1699 证据已合入 main `99df2ed762201236b89f0e81c0276cb3aa847f0a`，原 head `f6118cc6668efb50fe656423e3231870d7a4d073`。
- PR #1720 已合入 main `f8072dc9cbe34b34b468a9aa05772693d0acf187`，最终 head `b1b2016b1dc02be8c2106ab6701b4e816e79ec70`。
- 现场回读 [CI 40258706119](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40258706119) 与 [DevSkim 40258705934](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40258705934)，均 completed/success，headSha 均为上述 f8072dc9。
- 真实 bootout、自愈及下一轮 healthy 原件已在 `.chanlun/review-results/issue1678-selfheal-20261024/`，由 main `99df2ed7` 归档。本轮未重跑探针或 Rust。

## 本轮移除

普通 `git worktree remove` 对两处均退出 0，未使用 force。删除前 tracked、untracked、ignored 状态均为空。侧栏唯一 active 会话和唯一 NewChanlun pinned 会话均在仓根。候选目录未被直接 pin，lsof 未见句柄。Sandcastle #1678 最后事件为 sandbox closed，宿主已无本票工位。

| 原路径 | HEAD | 恢复 ref |
| --- | --- | --- |
| `/Volumes/AgentStorage/issue1678-selfheal-evidence-20261024/worktree` | `f6118cc6668efb50fe656423e3231870d7a4d073` | `refs/archive/issue1678-26bn/evidence` |
| `/Volumes/AgentStorage/pr1720-consume1723-20261026` | `b1b2016b1dc02be8c2106ab6701b4e816e79ec70` | `refs/archive/issue1678-26bn/integrated` |

两份增量 bundle 已通过 `git bundle verify`，list-heads 对齐上述 HEAD。evidence bundle 要求前置提交 `13ed4d45c8ff85175819ca987690328ab2a247a4`；integrated bundle 要求前置提交 `9b66f8a7acc85402f15fc9918628a86439d7ddb3` 与 `99df2ed762201236b89f0e81c0276cb3aa847f0a`。它们不是完整仓库备份。

恢复命令在原仓库执行。

```sh
git worktree add --detach /Volumes/AgentStorage/issue1678-selfheal-evidence-20261024/worktree refs/archive/issue1678-26bn/evidence
git worktree add --detach /Volumes/AgentStorage/pr1720-consume1723-20261026 refs/archive/issue1678-26bn/integrated
```

## 仍保留的对象与下一动作

`/Volumes/AgentStorage/issue1678-macd-ci-20261026` 仍在 `6aff48a6c03dac21bd112c85023cc3f7e0c4cfea`。tracked 和 untracked 为空，ignored 三文件为 `.serena/.gitignore`、`.serena/project.local.yml`、`.serena/project.yml`。三者已保留权限复制到本轮证据目录并逐项 cmp 成功，原件未删，工位未移除。主控仍需决定这三项本地配置的退役处置。

根仓本地分支 `codex/issue-1678-selfheal-evidence` 和 `codex/macd-scaling-lock-20261026` 保留。Sandcastle 宿主本地 `sandcastle/issue-1678` 保留在 `025934a1e6fd07b6ed6c554bb8246d4d20875be2`，未绑定工位。三个远端分支经 `git ls-remote --heads` 均已不存在。本轮未对分支使用强制删除或修改共享宿主。

下一负责人先裁定三项本地配置的退役处置，再清理 MACD 工位。旧本地分支须按各自仓库的 squash 交付关系处置，不能因为不是 main 的祖先而误判仍欠实施。最终关票仍需工位及分支处置、最终报告入 main、resolution 和图索引记账。主控持有 main 合入与关票验收，本轮没有关闭 #1678 或 #1547。

## 证据与实际操作

原件在 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1678-closeout26bn-evidence/`。其中包括 PR 三份回读、精确 main CI 两份回读、删除前后 worktree 清单、三处 status、恢复 ref 清单、两个 bundle 及验证输出、侧栏候选使用摘要、lsof 路径过滤及 Serena 原件备份。原件含本地配置，不纳入 Git。

`df -h` 的外盘可用空间在本轮清理前约 374 GiB，清理后约 377 GiB。`/` 可用均约 128 GiB。df 数值取整且其他任务并发，差值不作为精确物理释放量。

实际命令为 gh issue/pr/run view、git show/log/status/worktree/merge-base/update-ref/bundle/for-each-ref/ls-remote、lsof、df、cp、cmp 与侧栏 list_threads。所有时间来自现场 date。未执行 Rust、自愈探针、服务重启、force、reset、stash、clean 或远端分支删除。

本单元按 Worktree cleanup 完成定向快照、使用核查、可恢复清理和复查。throughput checkpoint 是先释放两个完全干净容器，同时保全有 ignored 配置的对象。Prove It Works 将结果绑定实际工位、句柄及精确 main CI 终态，不采用旧账本失败状态。
