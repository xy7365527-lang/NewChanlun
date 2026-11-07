# Boat Queued 操作员状态与恢复速查

拿到票号或 run id 后，先查队列状态，再查原 run 的本地记录。未知工蜂继续占槽，恢复沿用原 run 和绑定。

## 准入前提

全局 `/Volumes/AgentStorage/deepagents-exec/cutover.json` 必须有效，旧入口必须保持同代次 `draining`。引擎给合资格实装票挂 `deepagents:queue`。队列允许 `ready-for-agent`、`wayfinder:impl`，拒绝 `sandcastle`、`agent:*`、`wayfinder:grilling`、`ready-for-human`/`needs-info`/`needs-triage`/`wontfix`、有负责人、有 blocker 或不满足 SPEC 归属要求的票。旧日志有未知事件或未收尾 claim 时也拒绝准入。

所属 SPEC 当前正文必须声明 `deepagents-scope`。依据 #1742 最新用户授权，普通 SPEC 去除人工批准，Queued 每个动作边界消费可信编排账号的非作者独审裁定，裁定须绑定当前正文并带证据位置。无效 PASS、后续 BLOCK、正文变化都拒绝动作，不使用旧人工签名兜底。协议和发布 helper 见 [SPEC 授权](../../../.sandcastle/deepagents-pilot/EXECUTOR.md#为新队列-run-提供-spec-授权)。普通 Queued 不需候选人工签批，显式开启候选批准时仍须签确切 head。

新 Boat run 必须有受管配置，缺配置不回退本机。准入原文见 [接收引擎放行队列](../../../.sandcastle/deepagents-pilot/EXECUTOR.md#接收引擎放行队列)、[受管配置](../../../.sandcastle/deepagents-pilot/BOAT.md#受管配置) 与 [将引擎放行交给新执行器](../../../.sandcastle/docs/migration-drain-1742.md#将引擎放行交给新执行器)。

## 状态表

行键取队列状态页的六个状态。原 run 的发布 `phase`、持有记录和工蜂结果保留各自名字，在状态页上折叠进这六个状态：发布 `merged` 显示为 `awaiting_merge`，发布 `stale` 显示为 `halted`。原发布状态查 `queue-status.json` 的 `delivery_phase` 或 `executor status` 的 `phase`。

| 状态名 | 含义 | 操作员看什么（只读命令或文件） | 操作员能做什么或恢复动作 | 依据（运行时符号与操作章节） |
| --- | --- | --- | --- | --- |
| `queued` | 队列等待处理。 | 部署检出的 `.sandcastle/logs/deepagents-status.md`、状态根的 `queue-status.json`。 | 核对准入前提。已有工蜂运行或结果不明时，先处理原 run，不重派。 | `QueueIntake.once`、[运行队列](../../../.sandcastle/deepagents-pilot/EXECUTOR.md#运行队列) |
| `active` | 原 run 仍占队列槽：run 正在推进；子进程以 75 退出或被信号终止（退出码为负），下一轮恢复同一 run；或持有已是 `halted`、`released` 或单元已损坏，但工蜂尚未排空或观察无法完成。控制进程退出码 0 或 2 不证明工蜂结束。 | 队列状态页 `reason`、`executor status --run-id t<票号>-1` 与原 run 的持有记录。 | 工蜂仍为 `Running` 时等待。持有仍有效时，`EffectUnknown` 或状态无法读取由原控制入口恢复和收集。持有已是 `halted`、`released` 时，`executor run` 会被拒绝，改走“取消原工蜂与核对排空”。 | `QueueIntake._worker_activity`、`TicketExecutor.claim` |
| `awaiting_review` | 当前 SPEC 正文还没有可信编排账号发布的有效非作者 PASS 裁定：缺裁定、裁定无效或正文已变（原因 `awaiting_review`），或最新裁定为 BLOCK、已被撤回（原因 `review_block`）。不需要人批准，持有保持 `active`，队列每轮重核，新裁定出现即在同一 run 继续。`spec_authorization.status.json` 的 `status` 同为 `awaiting_review`。工蜂未排空时，此状态仍占槽。 | 状态页原因、`runs/<run>/approvals/spec_authorization.status.json`。 | 编排账号发布绑定当前正文的非作者裁定；BLOCK 时先按独审意见改 SPEC 再重审。 | `QueueIntake.once`、`TicketExecutor.spec_approval` |
| `awaiting_human` | 等待人工处理，常见原因包括首次登记缺 `deepagents-scope`（`needs_info`），或 `runs/<run>/interrupt.request` 有停点。停点来源是操作员中断、`execution_unknown` 或 `brain_stopped_early`。此状态也会显示两类不需要签批的原因：`delivery read failed; retry next round` 是交付读回暂时失败，下一轮自动重试；`source checkout lacks main commit` 要求部署或 fetch main。工蜂未排空时，此状态仍占槽。 | 状态页原因、`runs/<run>/approvals/` 下的状态文件、`runs/<run>/interrupt.request` 的 `by` 与 `reason`。 | 缺写集先补 SPEC，再独审。停点按下文“恢复同一 run”处理。封包 SPEC 与当前授权不符造成的 `execution_unknown` 不走 `--resume`，走“取消原工蜂与核对排空”。 | `QueueIntake.once`、`TicketExecutor.spec_approval` |
| `awaiting_merge` | 冻结 head 已发布，draft PR 的 head 与它一致，状态和交付标记齐备。已合入（发布 `merged`）也显示为此状态。 | 原 run 的 `delivery.state.pr.url` 与 PR 的 head；是否已合入以 `executor status` 的 `phase` 为准。 | 队列交付终点为 `awaiting_merge(h)`。精确 head 独审使用 `pilot1742.external_review review-pr`。队列不自动触发独审，也无 hillclimb 合入消费者；主控按当前授权完成合入。 | `TicketExecutor.reconcile_delivery`、[独审入口](../../../.sandcastle/deepagents-pilot/EXTERNAL-REVIEW.md) |
| `halted` | 已停止推进。来源包括重启时当前 SPEC scope 不再覆盖冻结写集、持有复核失败、远端 PR 不合交付合同、发布 `stale`，以及子进程结束后既无停点也无待签批、退出码不是 75 也不为负、发布 `phase` 不是 `awaiting_merge` 或 `merged`（含退出码 0）。 | 原 run 的持有记录、`executor status` 的 `phase`、队列状态页 `reason` 与本地工蜂记录。 | 交主控裁决，只有主控能释放。停止不证明排空；工蜂未排空时状态页显示为 `active`，按 `active` 行处置。 | `TicketExecutor.claim`、`QueueIntake.once` |

`stale` 表示远端分支或 PR 的 head 不再等于冻结 head，后代也算；`delivery.stale.observed` 记录观察到的 SHA。执行器不覆盖或收养，由主控决定重新评审还是释放。`merged` 表示已合入，执行器不再写入。见 [读懂结果](../../../.sandcastle/deepagents-pilot/EXECUTOR.md#读懂结果)。

## 只读诊断

`status` 只读本地，不访问 GitHub 或 Boat，不收集工蜂。本页的执行器命令都用 `executor` shell 函数。先在仓库根的当前 shell 中定义它。`VENV` 与 [运行](../../../.sandcastle/deepagents-pilot/EXECUTOR.md#运行) 相同；`PYTHONPATH` 另加 `.sandcastle/approval-signing`，因为执行器导入的 `approval_signing` 不在该 venv 中：

```sh
VENV=/Volumes/AgentStorage/issue1742-deepagents-pilot-20261027/venv
executor() { PYTHONPATH=.sandcastle/approval-signing:.sandcastle/deepagents-pilot "$VENV/bin/python" -m pilot1742.executor "$@"; }
```

`PYTHONPATH` 是相对路径，所以从仓库根调用。不带这个函数直接写 `python -m pilot1742.executor` 会找不到 `pilot1742`；只设 `.sandcastle/deepagents-pilot` 会找不到 `approval_signing`。示例中的 `ORIGINAL_RUN` 须换成原 run id（如 `t<票号>-1`），不能为了规避未知结果改用新 run。

```sh
executor status --run-id ORIGINAL_RUN
```

状态根缺省为 `/Volumes/AgentStorage/deepagents-exec`。`units/<run>.json` 保存单元，`runs/<run>/` 保存 run；其中 `run.json` 保存冻结记录，`claim.json` 保存代次，`boat/<key>.json` 保存工蜂绑定。普通 Queued 查看 `approvals/spec_authorization.status.json` 的独审裁定原因；仅显式候选签批使用请求文件与签批命令。

核对旧入口的冻结代次时，查询为：

```sh
python3 .sandcastle/drain-control.py status
```

此查询不改冻结状态，但会短暂持有准入排他锁；有引擎动作正持共享锁时，它会等该动作结束。它也不替代旧工蜂、远端 job 和生命周期的排空核验。

## 恢复同一 run

控制进程退出但没有停点时，原入口可重跑同一 run，工蜂按原键收养，外部写入先读回。退出码 75 表示已有控制进程或低盘。低盘阻止新 run 和新增派工。已有 run 可启动恢复、取回远端终态并回收，后续已交付票仍逐票对账。普通 `executor run` 和队列的 `executor run --recheck-approval` 都不清除停点。

`interrupt.request` 已有停点，且主控明确决定撤销停点并继续时：

1. 查原 run 的本地状态、绑定和停点的 `by`、`reason`。原因是封包 SPEC 与当前授权不符时停在这里，改走下节取消路径；`--resume` 重跑同一工具会再次停下。
2. 显式恢复原 run。此命令清除停点，并重新核对当前正文与授权来源。图已结束时，只有 `brain_stopped_early` 的条件仍成立才继续；否则 `--resume` 只读。

   ```sh
   executor run --run-id ORIGINAL_RUN --private-key /absolute/app.pem --resume
   ```


已提交（`submitted`）工蜂的 VM 已停止，且缺回执或缺 `key_erased=true` 证据，需要显式恢复同一 VM 时：

1. 由主控明确决定恢复原 VM。该动作不属于只读查询，也不自动续费或另派工蜂。
2. 使用原绑定的 VM 身份恢复。

   ```sh
   boat resume bx_WORKER_VM --ttl 1800
   ```

3. 按原 run 的持有状态选择入口：持有仍有效时，使用上面的 `executor run --resume` 推进原 run；持有已是 `halted` 或 `released` 时，按下节重跑原 `cancel-worker`，不走 `run --resume`。原监督进程仍在时等待；已结束且无回执时保留 `EffectUnknown`。

回执已缓存且删除证据已落盘时，原控制入口沿原绑定确认停止即可，无须恢复 VM。已取消的 `creating`、`prepared` 绑定不走 `run --resume`；VM 已停止但缺 key 删除证据时，恢复同一 VM 后再次调用原 `cancel-worker`。完整原文见 [启动、观察和恢复](../../../.sandcastle/deepagents-pilot/BOAT.md#启动观察和恢复)。

## 取消原工蜂与核对排空

控制进程已经退出并释放 run 锁，且决定取消原绑定时：

1. 从原绑定核对 run、角色、目标 head、状态目录和 generation。
2. 调用取消入口。下例对应作者，参数必须匹配原绑定。

   ```sh
   executor cancel-worker --run-id ORIGINAL_RUN \
     --role author --head FULL_TARGET_SHA --private-key /absolute/app.pem
   ```

3. 检查取消后观察到的事实。`Running` 继续等待。`EffectUnknown` 或无法读取继续占槽。准备阶段清理未完成时，再次取消仍沿原绑定，不补派。
4. 核对下面的排空条件。满足条件才可释放队列槽；原票还须由主控 `release`，之后新 run 取得当前正文的独审裁定，才能换代。

取消在票已关闭、准入撤销或持有为 `halted`、`released` 后仍可执行。取消不导入业务产物，也不把原 run 判为成功。

SPEC 正文变化并重新裁定后，封包旧 SPEC 的工蜂按同一入口处置。取消入口在回执封包 SPEC 不是当前授权目标时记 `cancel_requested`，旧 SPEC 的 `completed` 回执作废但原样保留，不收集、不记成功，此后可按排空条件排空，再 `release` 后换代。发布时独审封包 SPEC 不符返回 `review_spec_mismatch`，同 head 不能重派独审，同样走 `release` 后换代。

排空条件如下：

- 尚未提交的 `creating`、`prepared` 绑定，须已取消、核实 key 删除并确认 VM 停止，排空观察才返回 `NotStarted`。准备中的未知资源继续占槽。
- 已提交绑定先须同时具备共同证据：`key_erased=true`，VM 确认为 `stopped` 或 `archived`，记录 `stop_id` 与 `stop_confirmed=true`，缓存回执的 run、角色、head、generation 和 package 全部匹配，退出码为明确整数。再按回执来源满足以下三种之一：
  - 工蜂回执为 `terminal`，业务状态为 `failed` 或 `cancelled`，`record.process_exit.code` 是与退出码相同的整数，两者允许为 0。
  - 工蜂回执为 `terminal`，业务状态为 `completed`，且绑定已记 `cancel_requested`（取消入口判定封包 SPEC 不是当前授权目标），`record.process_exit.code` 同样须与退出码相同。
  - 监督器取消或非零退出的回执不含 `record`，沿用 `state=unknown`，退出码为明确非零整数。只有 `unknown`、没有明确退出码不足以排空。
- 缺 key 删除证据、停止确认、回执或明确退出码，以及身份不符时继续占槽。与当前授权同 SPEC 的成功回执由原控制入口收集，不走排空路径。

退出码 0、2 或 Boat stop accepted 均不等于工蜂结束。必须读回 `stopped` 或 `archived`；`stopping`、`archiving` 仍占槽。排空检查只读本地冻结记录，不访问 Boat，也不补收集。完整原文见 [取消后释放队列槽或换代](../../../.sandcastle/deepagents-pilot/BOAT.md#取消后释放队列槽或换代)。

## 范围外与生产处置门

本页不改运行时、服务、launchd、技能、模型规则、授权判据或 #1323 业务，不依赖 LFS 实体、市场数据或 Rust 全量运行。

生产切流的 freeze、修改仓库变量须先取得 D6 批准。旧认领释放或重新排队须核对属于 D6 批准范围。bootout 由部署 owner 在安全边界且子进程完成、清单对账后执行。恢复旧入口须先确认新执行器没有活跃副作用，已持有票已完成或正式释放。drain 的 resume 使用当前冻结代次，会立即使既有切流记录失效，不得同时启动两套消费者。这些动作不作为操作员自行执行的常规恢复命令。

生产处置按 [冻结旧入口并让在飞任务收尾](../../../.sandcastle/docs/migration-drain-1742.md) 执行。真实队列交付、异家族精确 head 独审、CI 和 App 发布由受管交付链验收；本页的本地核对不能代替真实交付到 `awaiting_merge(h)`。
