# 冻结旧入口并让在飞任务收尾

此页属于 #1742 迁移准备。代码默认不冻结。不要在生产切换 D6 批准前运行下面的 freeze 或修改仓库变量。

## 切换前部署

所有本机旧入口共用 `/Volumes/AgentStorage/newchanlun-legacy-drain`。路径不按 worktree 或环境变量分叉。CLI 的 `--root` 只用于隔离测试，部署命令不传它。

部署必须覆盖实际宿主与所有手工入口的这份代码，并在旧 owner 可核的轮次边界换进程。已加载的旧无限循环不会因磁盘替换而自动执行新门。不能直接卸载常驻父服务，否则可能杀掉在飞子进程。先记录现役版本、plist、进程父子关系和每票 checkpoint；让旧一轮自然完成后再由 owner 安排加载具备门禁的新版本。

远端 `agent-*` 工作流还需要先将本次变量门发布到默认分支。发布这些代码不等于启用冻结。D6 期间设置仓库变量 `SANDCASTLE_LEGACY_FREEZE=1` 后，五个工作流不接受新事件。变量不能撤销已排队、已评估或在运行的 job，必须列出并等它们自然完成；不得凭本地状态宣称远端已排空。

## 冻结与确认

从部署检出运行：

```sh
python3 .sandcastle/drain-control.py freeze
python3 .sandcastle/drain-control.py status
```

`freeze` 返回 `mode=draining` 及 generation，重复执行保持同一代次。冻结和准入用同一个系统 `fcntl.flock`。freeze 成功返回时，先前的“选择到首次副作用”操作已完成，后来的新操作会拒绝。它不杀已认领工蜂。

- main 的选票与 claim 在锁内；REVIEW_ONLY 的沙盒创建同样须准入。
- claim-loop 只在锁内拉起新 main，并在已拉起的子进程结束后停止下一轮。冻结期间不回滚到旧队列。
- decision-claimer 每票首次 claim 在锁内，已领取的票继续完成；下轮退出。
- swarm planner 可以自然结束，但尚未 claim 的计划在冻结后不能认领；已 claim 批次完成作者、独审和归并，不进入下一轮。
- engine 的每个实际图动作及放行都在锁内；freeze 会等待已经开始的动作结束。大起草动作可能延长等待，不用超时杀进程冒充排空。
- health-monitor 保留只读检查，但不能 heal、kickstart、bootstrap 或回滚认领。其变更动作也使用同一准入锁，避免观察到 open 后冻结才生效的竞态。

`harvest.sh` 不承担新增领票。当前策略允许它在排空期间完成已经过审旧分支的 PR 交付，不改它的既有判据。切流前必须等当前 harvest 单轮结束，再停它的定时入口，记录全部已交付 PR。它仍不是新执行器精确 head 发布器。

取得 freeze 确认后核对所有本机控制进程、工蜂、远端 job、workers.jsonl 尾部、decision summary 和分支归属。没有完整清单，不算排空。health-monitor 的 launchd KeepAlive/定时任务也须对账；冻结门只保证重启后的新选择被拒，不声称进程从此不再被 launchd 拉起。

未知版本、坏 JSON、读取失败或错误 generation 都失败关闭。控制进程崩溃后 helper 的 stdin 关闭，内核释放 flock，不靠 PID 年龄猜锁是否陈旧。持久状态保持 draining，不自动恢复 open。

## 恢复旧入口

先确认新执行器没有活跃副作用。新执行器已持有的票必须完成或正式释放，不得直接重贴旧标签。恢复旧代码和 plist后，使用冻结时记录的同代次解冻：

```sh
python3 .sandcastle/drain-control.py resume --generation 1
```

把示例 `1` 换成当前冻结代次。旧代次不能解开后来的新冻结。恢复远端变量前核已排队 job，最后才恢复 health-monitor 自愈，避免它提前拉起旧执行链。

## 离线验证

```sh
node --import tsx --test .sandcastle/drain-control.test.mts
```

测试仅用临时根，覆盖 freeze/claim 竞争、已准入收尾、冻结后无新增副作用、半写状态、代次恢复及真实 SIGKILL 后内核释放锁。真实部署、旧消费者排空和远端变量启用仍未执行。
