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

`freeze` 返回 `mode=draining` 及 generation，重复执行保持同一代次。冻结和准入用同一个系统 `fcntl.flock`。开放准入持共享锁，不互相串行。freeze 和 resume 持排他锁。freeze 成功返回时，先前的“选择到首次副作用”操作已完成，后来的新操作会拒绝。它不杀已认领工蜂。

- main 的选票与 claim 在锁内；REVIEW_ONLY 的沙盒创建同样须准入。
- claim-loop 只在锁内拉起新 main，并在已拉起的子进程结束后停止下一轮。冻结期间不回滚到旧队列。
- decision-claimer 每票首次 claim 在锁内，已领取的票继续完成；下轮退出。
- swarm planner 可以自然结束，但尚未 claim 的计划在冻结后不能认领；已 claim 批次完成作者、独审和归并，不进入下一轮。
- engine 的每个实际图动作及放行都在锁内；freeze 会等待已经开始的动作结束。大起草动作可能延长等待，不用超时杀进程冒充排空。
- health-monitor 保留只读检查，但不能 heal、kickstart、bootstrap 或回滚认领。其变更动作也使用同一准入锁，避免观察到 open 后冻结才生效的竞态。

`harvest.sh` 不承担新增领票。当前策略允许它在排空期间完成已经过审旧分支的 PR 交付，不改它的既有判据。切流前必须等当前 harvest 单轮结束，再停它的定时入口，记录全部已交付 PR。它仍不是新执行器精确 head 发布器。

取得 freeze 确认后核对所有本机控制进程、工蜂、远端 job、workers.jsonl 尾部、decision summary 和分支归属。没有完整清单，不算排空。

单独列出 workers.jsonl 中有 `phase:claim` 却没有 implementer started 的票，并对照 GitHub 当前认领、标签与本地 checkpoint。认领后、启动前失败的票在冻结下不会自动回队列，不能把“没有工蜂进程”计为完成。逐票记录旧 owner、最后一个已证实步骤、分支和未执行副作用。由旧 owner 在批准范围内选择从原 checkpoint 收尾，或保留冻结认领并登记给新执行器的交接方案。需要释放认领或重新排队时，先核该动作属于 D6 批准范围，再按交接方案执行。全部遗留票有去向及接收记录后，才能确认排空。

冻结后循环正常退出，但 launchd 的 `KeepAlive=true` 可能继续节流重拉，形成启动后立即退出的日志噪音。门禁仍拒绝新选择，这些启动记录不算新增工蜂。待各循环达到安全边界、子进程已完成并且 inventory 已对账，再由部署 owner bootout 对应 plist，不能在子进程仍在飞时卸载父服务。核对 health-monitor 的 KeepAlive 和定时任务，保持 heal 禁用直到恢复步骤最后。

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

测试仅用临时根，覆盖 freeze/claim 竞争、已准入收尾、冻结后无新增副作用、半写状态、代次恢复、并发开放准入及真实 SIGKILL 后内核释放锁。decision-claimer 相邻测试用独立临时根，覆盖 T 类断点、P 类 pDone 在冻结下保留，以及解冻后不重复评论或提交。真实部署、旧消费者排空和远端变量启用仍未执行。

## 将引擎放行交给新执行器

`bash .sandcastle/cutover-runbook.sh` 只打印命令和核对点，不冻结、不写 GitHub、不卸载服务。参数依次为部署检出、venv 和私钥绝对路径。D6 获批后由部署 owner 逐步执行，不把打印成功算成部署。

在 freeze 成功、仓库变量已设为 1 后，执行器的 `cutover snapshot` 记录该代次、存活旧进程的 pid 加启动标识，以及 workers 日志的规范化绝对路径和字节位置。verify 拒绝用另一份日志解释该字节位置。快照文件用 `O_EXCL` 创建，不覆盖已有快照。等待这些进程退出和旧工作收尾，再在安全轮次边界停 claimer、harvest、旧 engine 及 decision-claimer。不要直接卸载仍带在飞子进程的父服务。

```sh
executor cutover snapshot --workers-log "$PWD/.sandcastle/logs/workers.jsonl"
executor cutover verify --workers-log "$PWD/.sandcastle/logs/workers.jsonl" --private-key "$PEM"
executor cutover record --workers-log "$PWD/.sandcastle/logs/workers.jsonl" --private-key "$PEM"
```

`verify` 只读核对冻结代次、快照进程退出、全量旧 claim 终态、快照后无新增 claim、App 读取的全部 `agent-*` job 已完成，以及 claimer 和 harvest 的 launchd 服务已卸载。未知事件、无法读全分页、日志截断、仍未收尾的旧 claim 都拒绝。当前实现要求日志已有终态，交接文字本身不能替代终态证据。

冻结变量单独通过 `gh variable get SANDCASTLE_LEGACY_FREEZE` 回读。现有 App 没有 variables 权限，因此此项运维证据记为 `user-token-readonly`。它不充当人类批准，也不在 App 失败时接替写操作。读不到或值不是 1 都拒绝。

`record` 重做上述验证后，以 0600 和 `O_EXCL` 创建全局 `/Volumes/AgentStorage/deepagents-exec/cutover.json`。内容相同的重复写入成功，内容不同的记录不覆盖。引擎和执行器都读取这份记录，不增加环境变量切流开关。引擎在同一把 drain 共享锁内改挂 `deepagents:queue`，新拆的实装票只先挂 `ready-for-agent`，放行步骤再挂队列标签。

启动新 engine 后，先运行 `--once --dry-run` 检查新放行标签。`executor serve --once` 是实际执行入口，不是 dry-run。它会为缺签批的票生成请求，只有有效 SPEC 签批才进行票面写入。写集与签批登记方法见 [新执行器队列操作](../deepagents-pilot/EXECUTOR.md#接收引擎放行队列)。

安装 `.sandcastle/launchd/com.newchanlun.deepagents-executor.plist` 前，替换全部部署路径占位符。模板只有私钥文件路径，没有密钥内容。由部署 owner 加载服务并核对真实队列交付到精确 `awaiting_merge(h)`，代码和离线夹具不代替这一步。

`drain-control resume --generation <当前代次>` 使既有切流记录立即失效。共享动作锁保证 resume 等当前动作完成再开放旧入口，新执行器的下一动作会停下。先让新执行器到安全边界，再完成或正式释放已持有的票，最后恢复旧服务和 health-monitor 自愈。不要同时启动两套消费者。

切流记录与快照在回退后保留供审计。再次切流前，由部署 owner 将失效的 `cutover.json` 和旧快照移入代次归档目录，再创建新代次记录。程序不自动删除或覆盖旧证据。
