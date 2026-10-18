# #1495 定向清理执行收据

结论：按主控本次明确授权，三个定向命令全部exit0完成。两个目标工位注册和两个已合本地分支已实际移除；仓外176K可恢复备份10项SHA与清单自身SHA在清理前后完全一致。未推送、未合main、未关票、未全局prune、未升级-D，未触及其他工位。

## 执行前再核

原工位HEAD仍dedf11f047b9c809c167b35e728e4a2ee8bf3494，status仅` M formal/Origin/SegmentFeatureSeq.lean`；binary patch与备份逐字一致（SHA75520fbe1ec95a0a641f067de71366e5fb71ac31945e812e469a3b574a1db01b），文件SHA2f87dd64dc52d7b4c7babb5a3cd3dd41cba28e22346560f718013edc05d8647f；未追踪0，忽略1144件仍全formal/.lake/。两分支HEAD与预检一致，recovery路径仍缺失而目标注册存在。全打开文件lsof无stderr/exit0，两完整路径无占用；ps完整路径/分支参数命中0。备份10项和SHA清单自身均重新核过，未发现漂移。

## 命令与结果

1. `git -C /Users/silencehan/Projects/NewChanlun worktree remove --force /Volumes/AgentStorage/sc-wt/1495`：exit0，stdout/stderr空；实际移除约1.4G旧工位（包含已收编并完整归档的188行WIP与再生产缓存）及`.git/worktrees/1495`。
2. `git -C /Users/silencehan/Projects/NewChanlun worktree remove --force /private/tmp/goal1323-native-20261012/wt-1495-recovery`：exit0，stdout/stderr空；实际移除缺失工位注册`.git/worktrees/wt-1495-recovery`。
3. `git -C /Users/silencehan/Projects/NewChanlun branch -d codex/1495-lean-segment-turn-20261014 codex/1495-lean-recovery-20261015`：exit0，明确输出两分支Deleted。Git提示二者已合其upstream但不在当前根HEAD祖先链；这与预检一致，使用的是普通-d且未强制-D。

## 清理后独核与恢复

worktree列表前后差集恰为上述两个目标，无额外消失工位；两目标目录和git注册元目录均不存在。两本地refs的show-ref --verify --quiet均exit1（不存在）。仓外备份保持原176K，10项内容SHA与执行前完全相同，SHA256SUMS自身仍`4eb084a4645f32b0095b27dbe77e4bb3936a5f319e447e3717d96d4d11450054`。dedf11及859886两完整提交对象cat-file均仍存在；origin/main仍0c397d20de4076dbb6f7caa62738fc04cf72322f。没有把路径原占用大小冒称已测得磁盘free增长。

恢复仍沿预检报告：从dedf11f047b9c809c167b35e728e4a2ee8bf3494建新隔离工位，apply备份uncommitted.patch并核原件SHA；recovery提交859886fdb7dc664f59358fc427db23122122adc0仍可在Git中取回。备份路径 `/Volumes/AgentStorage/issue1495-old-wip-backup-20261018` 必须继续保留，不在后续缓存清理范围。

逐步命令、exit、stdout/stderr、执行时间和前后SHA原件见 `/tmp/1495-cleanup-execution-20261018.json`。主控可据真实处置事实更新roster及关票门第5项；本代理没有写GitHub、没有关票。
