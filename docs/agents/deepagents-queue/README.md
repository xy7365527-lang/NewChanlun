# Boat Queued 操作员状态与恢复速查

拿到票号或 run id 后，先查队列状态，再查原 run 的本地记录。未知工蜂继续占槽，恢复沿用原 run 和绑定。

## 准入前提

全局 `/Volumes/AgentStorage/deepagents-exec/cutover.json` 必须有效，旧入口必须保持同代次 `draining`。引擎给合资格实装票挂 `deepagents:queue`。队列允许 `ready-for-agent`、`wayfinder:impl`，拒绝 `sandcastle`、`agent:*`、`wayfinder:grilling`、人闸标签、有负责人、有 blocker 或不满足 SPEC 归属要求的票。旧日志有未知事件或未收尾 claim 时也拒绝准入。

所属 SPEC 的签名绑定正文必须声明 `deepagents-scope`。实装票自行添加写集不授予权限。D8 签批绑定 `ticket`、`run_id`、`claim_generation`、`spec_authorization` 和 SPEC 正文原始字节。另一个 run 的签批不能复用。缺写集、缺签批、签批拒绝或过期时等待人工处理。正文变化需要重新签批。

新 Boat run 必须有受管配置，缺配置不回退本机。准入原文见 [接收引擎放行队列](../../../.sandcastle/deepagents-pilot/EXECUTOR.md#接收引擎放行队列)、[受管配置](../../../.sandcastle/deepagents-pilot/BOAT.md#受管配置) 与 [将引擎放行交给新执行器](../../../.sandcastle/docs/migration-drain-1742.md#将引擎放行交给新执行器)。依据为 `.sandcastle/deepagents-pilot/EXECUTOR.md:129`、`:131`、`:139`、`:143`、`:145`，`.sandcastle/deepagents-pilot/BOAT.md:3`，`.sandcastle/docs/migration-drain-1742.md:132`。

## 状态表

行键取队列状态页的五个状态。发布阶段、持有记录和工蜂结果保留各自名字，不另作队列状态。

| 状态名 | 含义 | 操作员看什么（只读命令或文件） | 操作员能做什么或恢复动作 | 依据（仓内相对路径:行号） |
| --- | --- | --- | --- | --- |
| `queued` | 队列等待处理。 | 部署检出的 `.sandcastle/logs/deepagents-status.md`、状态根的 `queue-status.json`。 | 核对准入前提。已有工蜂运行或结果不明时，先处理原 run，不重派。 | `.sandcastle/deepagents-pilot/EXECUTOR.md:156`、`:162` |
| `active` | run 正在推进。控制进程的退出不证明工蜂结束。 | 队列状态页与 `executor status --run-id t<票号>-1`。 | 工蜂仍为 `Running` 时等待。结果为 `EffectUnknown` 或状态无法读取时继续占槽，由原控制入口恢复和收集。 | `.sandcastle/deepagents-pilot/EXECUTOR.md:68`、`:156`、`:162` |
| `awaiting_human` | 等待人工处理，包括缺 D8 签批、缺 `deepagents-scope` 或人工中断。 | 状态页原因与 `runs/<run>/approvals/spec_authorization.request`。 | 批准人核对并发布绑定当前 run 和正文的签批。缺写集先补 SPEC 正文，再签批。人工中断仅在明确决定继续时显式恢复。 | `.sandcastle/deepagents-pilot/EXECUTOR.md:139`、`:145`、`:162` |
| `awaiting_merge` | 冻结 head 已发布，draft PR 的 head 与它一致，状态和交付标记齐备。 | 原 run 的 `delivery.state.pr.url` 与 PR 的 head。 | 队列交付终点为 `awaiting_merge(h)`。由人在 GitHub 合入 main，执行器不自动合入。 | `.sandcastle/deepagents-pilot/EXECUTOR.md:3`、`:84`、`:147` |
| `halted` | 已停止推进。持有复核或远端 PR 合同失败会停止动作。 | 原 run 的持有记录、队列状态页与本地工蜂记录。 | 交主控裁决，只有主控能释放。停止不证明排空；已持有工蜂仍按取消和排空条件处理。 | `.sandcastle/deepagents-pilot/EXECUTOR.md:87`、`:156`、`.sandcastle/deepagents-pilot/BOAT.md:82` |

原 run 的发布 `phase` 还包括 `stale` 和 `merged`。`stale` 表示远端分支或 PR 的 head 不再等于冻结 head，后代也算；`delivery.stale.observed` 记录观察到的 SHA。执行器不覆盖或收养，由主控决定重新评审还是释放。`merged` 表示人已合入，执行器不再写入。见 [读懂结果](../../../.sandcastle/deepagents-pilot/EXECUTOR.md#读懂结果)，依据为 `.sandcastle/deepagents-pilot/EXECUTOR.md:82`、`:85`、`:86`。

## 只读诊断

`status` 只读本地，不访问 GitHub 或 Boat，不收集工蜂。以下两个入口查询同一类本地事实。`executor` 是部署环境中已有的函数，定义见 [运行](../../../.sandcastle/deepagents-pilot/EXECUTOR.md#运行)。示例中的票号或 `ORIGINAL_RUN` 须换成原 run，不能为了规避未知结果改用新 run。

```sh
executor status --run-id t<票号>-1
python -m pilot1742.executor status --run-id ORIGINAL_RUN
```

状态根缺省为 `/Volumes/AgentStorage/deepagents-exec`。`units/<run>.json` 保存单元，`runs/<run>/` 保存 run；其中 `run.json` 保存冻结记录，`claim.json` 保存代次，`boat/<key>.json` 保存工蜂绑定。签批请求路径由队列状态页列出，签批命令也以该页为准。依据为 `.sandcastle/deepagents-pilot/EXECUTOR.md:39`、`:65`、`:68`、`:113`、`:145`，`.sandcastle/deepagents-pilot/BOAT.md:41`、`:56`、`:61`、`:71`。

核对旧入口的冻结代次时，只读查询为：

```sh
python3 .sandcastle/drain-control.py status
```

此查询不替代旧工蜂、远端 job 和生命周期的排空核验。依据为 `.sandcastle/docs/migration-drain-1742.md:19`、`:33`、`:82`、`:88`。

## 恢复同一 run

控制进程退出但没有人工中断时，原入口可重跑同一 run，工蜂按原键收养，外部写入先读回。退出码 75 表示已有控制进程。普通 `executor run` 和队列的 `executor run --recheck-approval` 都不清除人工中断。依据为 `.sandcastle/deepagents-pilot/EXECUTOR.md:58`、`:61`、`:162`。

已有人工中断，且主控明确决定撤销中断并继续时：

1. 查原 run 的本地状态和绑定。
2. 显式恢复原 run。此命令清除人工中断，并重新核对权威签批链。

   ```sh
   python -m pilot1742.executor run --run-id ORIGINAL_RUN --private-key /absolute/app.pem --resume
   ```

工蜂 VM 已停止，且缺回执或缺 `key_erased=true` 证据，需要显式恢复同一 VM 时：

1. 由主控明确决定恢复原 VM。该动作不属于只读查询，也不自动续费或另派工蜂。
2. 使用原绑定的 VM 身份恢复。

   ```sh
   boat resume bx_WORKER_VM --ttl 1800
   ```

3. 使用上面的显式恢复命令推进原 run。原监督进程仍在时等待；已结束且无回执时保留 `EffectUnknown`。

回执已缓存且删除证据已落盘时，原控制入口沿原绑定确认停止即可，无须恢复 VM。依据为 `.sandcastle/deepagents-pilot/EXECUTOR.md:63`、`:162`，`.sandcastle/deepagents-pilot/BOAT.md:48`、`:64`、`:67`、`:68`、`:95`。完整原文见 [启动、观察和恢复](../../../.sandcastle/deepagents-pilot/BOAT.md#启动观察和恢复)。

## 取消原工蜂与核对排空

控制进程已经退出并释放 run 锁，且决定取消原绑定时：

1. 从原绑定核对 run、角色、目标 head、状态目录和 generation。
2. 调用取消入口。下例对应作者，参数必须匹配原绑定。

   ```sh
   python -m pilot1742.executor cancel-worker --run-id ORIGINAL_RUN \
     --role author --head FULL_TARGET_SHA --private-key /absolute/app.pem
   ```

3. 检查取消后观察到的事实。`Running` 继续等待。`EffectUnknown` 或无法读取继续占槽。准备阶段清理未完成时，再次取消仍沿原绑定，不补派。
4. 核对下面的排空条件。满足条件才可释放队列槽；原票还须由主控释放，之后新 run 取得绑定新 run 与代次的 SPEC 签批，才能换代。

取消在票已关闭、准入撤销或持有为 `halted`、`released` 后仍可执行。取消不导入业务产物，也不把原 run 判为成功。依据为 `.sandcastle/deepagents-pilot/BOAT.md:75`、`:78`、`:79`、`:82`、`:84`、`:99`，`.sandcastle/deepagents-pilot/EXECUTOR.md:71`、`:158`。

排空条件如下：

- 尚未提交的 `creating`、`prepared` 绑定，须已取消、核实 key 删除并确认 VM 停止，排空观察才返回 `NotStarted`。准备中的未知资源继续占槽。
- 已提交绑定，须有 `key_erased=true`，VM 确认为 `stopped` 或 `archived`，并记录 `stop_id` 与 `stop_confirmed=true`。缓存回执的 run、角色、head、generation 和 package 必须全部匹配，退出码必须为明确整数。
- 工蜂回执须为 `terminal`，业务状态为 `failed` 或 `cancelled`，`record.process_exit.code` 必须是与退出码相同的整数，两者允许为 0。
- 监督器取消或非零退出的回执不含 `record`，沿用 `state=unknown`。明确非零退出码与停止证据一起才能证明排空。只有 `unknown` 不足以排空。
- 缺 key 删除证据、停止确认、回执或明确退出码，以及身份不符时继续占槽。成功回执由原控制入口收集，不走失败排空路径。

退出码 0、2 或 Boat stop accepted 均不等于工蜂结束。必须读回 `stopped` 或 `archived`；`stopping`、`archiving` 仍占槽。排空检查只读本地冻结记录，不访问 Boat，也不补收集。依据为 `.sandcastle/deepagents-pilot/EXECUTOR.md:156`，`.sandcastle/deepagents-pilot/BOAT.md:46`、`:56`、`:84`、`:88`、`:90`、`:91`、`:93`、`:95`、`:99`。完整原文见 [取消后释放队列槽或换代](../../../.sandcastle/deepagents-pilot/BOAT.md#取消后释放队列槽或换代)。

## 范围外与生产处置门

本页不改运行时、服务、launchd、技能、模型规则、授权判据或 #1323 业务，不依赖 LFS 实体、市场数据或 Rust 全量运行。

生产切流的 freeze、修改仓库变量须先取得 D6 批准。旧认领释放或重新排队须核对属于 D6 批准范围。bootout 由部署 owner 在安全边界且子进程完成、清单对账后执行。恢复旧入口须先确认新执行器没有活跃副作用，已持有票已完成或正式释放。drain 的 resume 使用当前冻结代次，会立即使既有切流记录失效，不得同时启动两套消费者。这些动作不作为操作员自行执行的常规恢复命令。依据为 `.sandcastle/docs/migration-drain-1742.md:3`、`:35`、`:37`、`:43`、`:49`、`:67`、`:140`。

生产处置按 [冻结旧入口并让在飞任务收尾](../../../.sandcastle/docs/migration-drain-1742.md) 执行。真实队列交付、异家族精确 head 独审、CI 和 App 发布由受管交付链验收；本页的本地核对不能代替真实交付到 `awaiting_merge(h)`。
