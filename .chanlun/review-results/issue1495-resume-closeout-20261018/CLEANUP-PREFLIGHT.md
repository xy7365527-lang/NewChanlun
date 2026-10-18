# #1495 旧工位清理只读预检

结论：当前快照未发现未备份成果、身份漂移或可见活写者，满足交主控最终处置判断的预检；本代理没有删除、reset、clean、移除工位/分支、推送或运行产品。可回收外盘工位约1.4G；176K可恢复备份必须保留。操作前仍须复查瞬时HEAD/status/占用，不能把本只读快照当长期锁。

## 备份与恢复原件独核

备份 `/Volumes/AgentStorage/issue1495-old-wip-backup-20261018` 的SHA256SUMS共10项逐一实际重算全通过；清单自身SHA `4eb084a4645f32b0095b27dbe77e4bb3936a5f319e447e3717d96d4d11450054`。base文件与Git dedf11原blob同SHA `4f727a53f24ee548abd340ab6894be5f976d21ef4bd1d12bde18f5e3973429dc`；original及restore-check原件同SHA `2f87dd64dc52d7b4c7babb5a3cd3dd41cba28e22346560f718013edc05d8647f`；保存patch SHA `75520fbe1ec95a0a641f067de71366e5fb71ac31945e812e469a3b574a1db01b`，对restore-check执行git apply --check --reverse成功。未重写备份或恢复件。

## 两工位/分支实时身份

1. `/Volumes/AgentStorage/sc-wt/1495`存在，关联主仓`.git/worktrees/1495`，未锁；分支`codex/1495-lean-segment-turn-20261014`，HEAD `dedf11f047b9c809c167b35e728e4a2ee8bf3494`。status仍仅` M formal/Origin/SegmentFeatureSeq.lean`；binary diff与保存patch逐字相同、文件SHA相同。未追踪0，忽略1144全位于`formal/.lake/`，与备份排除声明相同。初末两次HEAD/status一致。
2. `/private/tmp/goal1323-native-20261012/wt-1495-recovery`不存在，仅注册残条`.git/worktrees/wt-1495-recovery`；分支`codex/1495-lean-recovery-20261015`，HEAD `859886fdb7dc664f59358fc427db23122122adc0`，未锁。不是另一个尚存可扫描工位，不伪称其本机旧未提交文件已恢复。

两分支`fdc5c6a7fd2fcb6b8d436d419984c1c826b669f6..branch`均0条未合commit，且两个HEAD都已是delivery merge与本地origin/main `0c397d20de4076dbb6f7caa62738fc04cf72322f`祖先。当前根HEAD并不含两分支，故不以当前根线作已交付依据。旧分支upstream为origin/main，recovery upstream为origin/codex/1495-lean-recovery-20261015；下列branch -d可利用已核upstream，若拒绝必须停止，不自动改-D。

## 可见进程与owner

`lsof -nP -Fpcn -d cwd`及全打开文件`lsof -nP -Fpcfn`均exit0、无stderr，两目标路径命中0；`ps`精确匹配两个完整路径和分支名命中0。仅报告当前权限可见的快照，没有借此断言机器上永远无其他写者。
实时GitHub票仍OPEN、assign xy7365527-lang。当前roster第136行resume_1495_formal写明“WIP备份对照完成，转r5合同”，第137行明确主控负责1495工位处置；旧roster的2026-10-14工蜂登记为历史，不能当活写者。未发现新的1495写锁或活工蜂证据。

## 最小具体命令（尚未执行）

按已核身份依次、每步检查exit，任何失败/漂移停手；只作用两个目标，不用全仓prune或rm：

```sh
git -C /Users/silencehan/Projects/NewChanlun worktree remove --force /Volumes/AgentStorage/sc-wt/1495
git -C /Users/silencehan/Projects/NewChanlun worktree remove --force /private/tmp/goal1323-native-20261012/wt-1495-recovery
git -C /Users/silencehan/Projects/NewChanlun branch -d codex/1495-lean-segment-turn-20261014 codex/1495-lean-recovery-20261015
```

第一步会丢掉已归档且已被交付收编的188行旧WIP以及1144个可再生产`.lake`缓存；第二步仅预期移除缺失工位的目标注册残条；第三步移除两个已交付本地branch引用。若Git版本不接受缺失path的定向remove，停报主控，不执行会清其他残条的全局prune。远端分支、main、其他工位、仓外备份均不在写集。

恢复路径：原Git dedf11及859886都被delivery/main引用保留。需要旧WIP时，向新路径`git worktree add -b codex/1495-restored-wip-20261018 <新工位绝对路径> dedf11f047b9c809c167b35e728e4a2ee8bf3494`，再在新工位`git apply --check /Volumes/AgentStorage/issue1495-old-wip-backup-20261018/uncommitted.patch`及`git apply`，核上述原件SHA。如只恢复分支引用，可从所列两个完整HEAD重建分支，不需要原缓存；recovery已提交交付原件在Git main可取。此次只读核验未实际执行这些恢复写操作。

细项JSON：`/tmp/1495-cleanup-preflight-20261018.json`。本报告不代替处置收据，不预写“已清理”或关票；主控执行后仍须按真实结果更新roster/票面。
