# 恢复中断或停止的执行器 run

本指南适用于 v2 `python -m pilot1742.executor`。依据为基线 `bcc83ab1cef86b98b67fbf88b06090b72e100b72` 的 [EXECUTOR.md](../../../.sandcastle/deepagents-pilot/EXECUTOR.md)、执行器源码和本基线实测 CLI 帮助。操作员负责取证，并在条件成立时执行原 run 命令。主控负责对账、释放、登记、停止控制进程和处置远端分支。字段、读回合同、身份不变量和完整帮助见[恢复参考](recovery-reference.md)。

## 先守住三条恢复规则

1. 写结果不明时，只靠读回解决。经过多久都不能作为再次发送未知写请求的依据。
2. 满足下文条件时，恢复只指在同一 `run_id`、同一状态根上原样重跑 `executor run`。重新登记、换 run 和 `release` 都不是恢复。
3. 每个写动作都必须先满足读回条件。条件不成立或证据读不全时，停手并交接主控。

## 准备命令环境

先从交接记录确认原控制仓库和状态根。下列仓库路径与 `VENV` 取自 [EXECUTOR.md 的运行环境](../../../.sandcastle/deepagents-pilot/EXECUTOR.md#L29)。如果原控制仓库已迁移，先由主控确认原执行器位置。不要把作者 checkout 当作控制仓库。

确认路径后，逐行设置环境。将占位参数替换为交接记录中的真实值。私钥只提供已批准的绝对路径，不读取或打印文件内容。

```sh
cd /Users/silencehan/.codex/worktrees/deepagents-pilot-1742/NewChanlun
VENV=/Volumes/AgentStorage/issue1742-deepagents-pilot-20261027/venv
EXECUTOR_STATE_ROOT='<original-state-root-absolute-path>'
EXECUTOR_RUN_ID='<run-id>'
EXECUTOR_TICKET='<ticket-number>'
EXECUTOR_RUN_DIR="$EXECUTOR_STATE_ROOT/runs/$EXECUTOR_RUN_ID"
EXECUTOR_CLAIMS_ROOT=/Volumes/AgentStorage/deepagents-exec/claims
EXECUTOR_REPO=xy7365527-lang/NewChanlun
executor() {
	PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=.sandcastle/deepagents-pilot "$VENV/bin/python" -B -m pilot1742.executor --state-root "$EXECUTOR_STATE_ROOT" "$@"
}
```

`--state-root` 是顶层参数，必须放在子命令之前。函数已固定这个顺序。不要改为一个空的新状态根。票级持有路径固定在 `EXECUTOR_CLAIMS_ROOT`，不随状态根变化。

## 确认原 run 与状态根

先读登记记录。

```sh
jq '{work_unit, task}' "$EXECUTOR_STATE_ROOT/units/$EXECUTOR_RUN_ID.json"
```

若 `run.json` 已存在，读取冻结身份。

```sh
jq '{v, run_id, ticket, base, work_unit, task, author_checkout, review_checkout, author_model, reviewer_model}' "$EXECUTOR_RUN_DIR/run.json"
```

若票级记录已存在，读取持有身份和停止原因。

```sh
jq '{ticket, run_id, run_dir, generation, state, reason, updated_at}' "$EXECUTOR_CLAIMS_ROOT/$EXECUTOR_TICKET.json"
```

若 run 内代次记录已存在，读取绑定。

```sh
jq . "$EXECUTOR_RUN_DIR/claim.json"
```

按参考中的[五者不变量](recovery-reference.md#五者不变量)核对票号、`run_id`、解析后的绝对 `run_dir`、`generation` 和 WorkUnit。代次从票级记录读取，不从 run 名称推算。已有 `run.json` 的恢复要求 `v=2`，且两份 WorkUnit 一致。

若 `run.json` 和作者 checkout 都存在，再读取聚合状态。

```sh
executor status --run-id "$EXECUTOR_RUN_ID"
echo $?
```

`status` 只读本机，不访问 GitHub。它不返回图检查点，也不将 `claim.state=halted` 合并进 `status.phase`。同时核对 `.claim`、`.status.workers`、`.status.conflict` 和 `.status.delivery`。无冲突时 `conflict` 为 `null`。

若在持有阶段中断，`run.json`、`claim.json` 或 checkout 可能尚未创建。此时 `status` 可能报 `FileNotFoundError`。不要补写文件。仅当判别表 R1 成立时才交还同一入口续做。

若 `trail.jsonl` 已存在，读取图和停止事件。

```sh
jq -c 'select(.event == "graph_start" or .event == "graph_resume_after_crash" or .event == "graph_already_finished" or .event == "halted" or .event == "publish_stale" or .event == "published" or .event == "brain_stopped_early")' "$EXECUTOR_RUN_DIR/trail.jsonl"
```

## 读退出码和 stderr

先保存上一次命令的完整输出、stderr 和退出码。若退出码没有保留，记录为未知，不重新执行写命令来补取。

| 退出码 | 含义 | 接下来核对的证据 |
| --- | --- | --- |
| `0` | 命令正常返回，不证明交付完成。图中断或发布工具返回失败也可能正常返回。 | 聚合状态、票级持有、工具结果和图状态。 |
| `2` | 被捕获的执行器异常或参数错误。被捕获的异常在 stderr 中为 `类型名: 消息`。参数错误由 argparse 输出 usage 和 error。 | 按类型名使用下面的停止卡。若错误被工具捕获，类型名也可能出现在工具 JSON 中。 |
| `75` | `run` 或 `reconcile` 未取得本 run 的控制锁。 | owner PID 的当前命令和已有控制进程的输出。不要启动第二个控制进程。 |
| 其他或未知 | 本指南没有为该退出方式定义恢复结论。被杀也不能靠退出码判断写入是否生效。 | 原输出、进程事实和各层记录。交回判别表。 |

退出码依据见[参考中的 CLI 行为](recovery-reference.md#cli-行为与图终态)。

## 按证据选择下一步

先检查停止行。只要任一停止行成立，就不能使用允许继续的行。R7 是后续章节唯一的通用恢复准入条件。

| 观察到的情况 | 操作前必须确认的证据 | 允许动作与预期结果 | 禁止动作 |
| --- | --- | --- | --- |
| R0 身份或状态根不符 | 票级 `run_id`、绝对 `run_dir`、代次绑定或冻结 WorkUnit 任一不符。必要记录损坏或无法解释其缺失。 | 停手，保存不一致的两端记录，交主控。 | 改 JSON、复制状态根、重新登记、换 run 绕过。 |
| R1 持有引导阶段被中断 | 原 `units` 记录有效。尚无 `run.json`，没有后续工蜂、提交或发布事实。票级记录不存在，或属于同一 run 和绝对目录且为 `intent`、`mirrored` 或 `active`。无其他停止条件。 | 按 R7 的命令在原状态根续做持有。预期由原效果记录认回镜像，再绑定代次和冻结 run。 | 手工补持有标记、代次或 run 文件。 |
| R2 控制锁被占用 | `run` 或 `reconcile` 返回 `75`。文件存在本身不算锁被占用。 | 只读观察已有控制进程。需要停止时交主控。 | 删锁文件或并行启动控制进程。 |
| R3 工蜂结果不明或未成功 | 任一工蜂为 `effect_unknown`，或终态为 `failed`、`cancelled`，或作者完成却无产出。 | 停手，保存原派工键、`state.json`、`result.json`、`intents` 和进程证据，交主控。`effect_unknown` 会导致 `Halted`。 | 换键重派、手写结果、把目录删掉后重来。 |
| R4 本地提交或 checkout 冲突 | `conflict` 非 `null`，或提交意图与实际 first parent、tree、trailer 不符。 | 停手，保存意图、HEAD 和分支证据，交主控。 | 手工提交、reset、改分支或补 trailer。 |
| R5 发布 stale | `delivery/stale.json`、工具结果或 `publish_stale` 记录表明远端 ref 或 PR head 与冻结 head 不同，后代也算。 | 停止推进，按 stale 章节交接。 | 强推、收养另一个 head、改用旧 PASS。 |
| R6 持有 halted、released 或已被隔离 | 票级 `state` 为 `halted`、`released`，或收到 `Fenced`、准入拒绝、票面或 PR 合同拒绝。 | 停手，保留 `reason`。同 run 的 halted 持有再次运行会先得到 `ClaimRefused`。 | 再次运行试图激活、手改票面或 PR 来解锁。 |
| R7 可交回同一入口 | 无任何停止条件，且尚未满足 R10。原身份和状态根已核对。已有 run 的持有为匹配的 `active`，或满足 R1。原有检查点、派工和效果记录保留。没有图已结束证据，也没有尚待主控处理的图中断。 | 只执行下文原 run 命令。由执行器读取原检查点，按原键接回工蜂、认回提交或读回效果。不承诺图一定继续到发布。 | 手工调用业务工具、开工蜂、重复任何业务写入。 |
| R8 `EffectPending` | 从现有效果记录读出 `what`，按参考合同完整读回。未命中不代表不存在。 | 命中且 R7 成立时，由原入口认回。未确认结果时只读取证。确证未生效时交主控决定对账。 | 用时长或一次空读回授权非幂等写入。 |
| R9 图已结束但尚未交付，或停在中断 | 已有 `graph_already_finished` 输出或 trail 事件，或者 `has_checkpoint=true`、`interrupts=[]`、`next=[]` 同时成立，且交付尚未满足 R10。若 `interrupts` 非空，单独归为图中断。 | 已结束时停止并交主控，CLI 没有继续业务工具的入口。图中断时保存 interrupt 并交主控，v2 不执行批准重检。 | 仅凭 `next=[]` 判断终态。声称对账后图会自动再发布。 |
| R10 已完成交付 | 无上述停止条件，`delivery/state.json` 为 `awaiting_merge` 或 `merged`，且远端读回符合参考合同。 | `awaiting_merge` 交人决定是否合入。已 merged 时记录远端事实。 | 再发 PR、status、评论或擅自合入。 |

满足 R7 时，执行以下命令一次。若原命令有经批准的 `--workers-log`，保留原参数和路径。

```sh
executor run --run-id "$EXECUTOR_RUN_ID" --private-key '<approved-app-private-key-absolute-path>'
echo $?
```

若原图状态尚无可用输出，保留 `graph.sqlite` 并让原入口读取。若这次只记录 `graph_already_finished` 且交付未齐，转 R9，不能循环重跑来触发业务工具。若交付已齐，按 R10 交接。

遇到以下类型名时，按对应停止卡取证。唯一允许的操作均为只读取证并交接主控，除非主控已完成本指南规定的决策条件。

- 收到 `LockTimeout` 时，保存票号、动作、时间、stderr 和票级记录。它表示票级动作锁超出代码中的等待上限，不授权再次业务写入。
- 收到 `IdentityError` 或其子类时，保存错误类型、失败端点和已有效果的 `what`。不要导出认证材料。`RequestNotSent` 和 `WriteRejected` 只证明那一次发送未生效，旧 pending 仍须单独读回。
- 收到 `Unpublishable` 时，保存冻结 head、base 和 SHA 重建错误。不要改用普通 Git 推送。
- 收到 `ClaimRefused`、`AdmissionRefused` 或 `Fenced` 时，保存票级记录、run 内绑定和错误原因。`AdmissionRefused` 继承 `ClaimRefused`，被 CLI 捕获时退出 `2`。不要重新登记绕过。
- 收到 `Halted`，尤其是“已冻结 run 的 WorkUnit 与登记不同”时，保存两份 WorkUnit、代次绑定和相关 trail。不要修补冻结文件。工蜂或提交导致的 `Halted` 还须保存相应事实。
- 收到 `ToolRefusal`、`ValueError` 或 `FileNotFoundError` 时，保存原参数、报错路径和已有记录。只有已核实的 R1 引导阶段缺文件可按 R1 处理，其余交主控。

## 接回中断留下的工蜂或提交

满足 R7 且工蜂键已存在时，先读该键的派工意图。键从 `status.workers` 或原 `intents` 文件名取得，不能自造。

```sh
EXECUTOR_WORKER_KEY='<role-and-head-key-read-from-status-or-intents>'
jq . "$EXECUTOR_RUN_DIR/intents/$EXECUTOR_WORKER_KEY.json"
```

若对应文件存在，分别读取工蜂事实。

```sh
jq . "$EXECUTOR_RUN_DIR/workers/$EXECUTOR_WORKER_KEY/state.json"
jq '{run_id, ticket, mode, model, expected_head, status, checkout_after, result, failure_reason}' "$EXECUTOR_RUN_DIR/workers/$EXECUTOR_WORKER_KEY/result.json"
```

`result.json.run_id` 是工蜂 UUID，不是执行器的 `run_id`。监督进程身份取唯一 `--output-dir`，不能只信旧 PID。判定细节见[工蜂和提交字段](recovery-reference.md#工蜂与提交)。

若工蜂是 `running` 或已有有效的 `terminal` 回执，且 R7 仍成立，使用原 run 命令。执行器接回原键，不另派同键工蜂。若目录已认领、没有可用结果且监督进程已死，转 R3。即使工蜂进程组还活着，也不能把它当成可自动接回的 `running`。

若提交意图已存在而提交回执缺失，先读取该意图。父 head 从现有文件名取得。

```sh
EXECUTOR_PARENT_HEAD='<parent-head-40hex-read-from-commit-intent>'
jq . "$EXECUTOR_RUN_DIR/commits/author-$EXECUTOR_PARENT_HEAD.intent.json"
EXECUTOR_AUTHOR_CHECKOUT="$(jq -r .author_checkout "$EXECUTOR_RUN_DIR/run.json")"
git -C "$EXECUTOR_AUTHOR_CHECKOUT" --no-optional-locks status --short
git -C "$EXECUTOR_AUTHOR_CHECKOUT" branch --show-current
git -C "$EXECUTOR_AUTHOR_CHECKOUT" rev-list --parents -n 1 HEAD
git -C "$EXECUTOR_AUTHOR_CHECKOUT" rev-parse 'HEAD^{tree}'
git -C "$EXECUTOR_AUTHOR_CHECKOUT" log -1 '--format=%(trailers:key=Deepagents-Run,valueonly)'
```

若实际 HEAD 的 first parent、tree 和 `Deepagents-Run` trailer 三项均与意图吻合，且 R7 成立，交回原入口补提交回执。若 HEAD 仍为意图 parent，且写集、分支、索引和作者回执满足[写前条件](#写动作的前置条件)，原入口才可继续原提交。若三项不符，转 R4。不要为了补回执再做一个提交。

## 认回结果不明的 GitHub 写入

遇到 `EffectPending` 时，先按 R8 从错误路径或现有文件确认效果位置。效果名来自真实文件，参数不带 `.json`。只执行符合该记录位置的读取命令。

```sh
EXECUTOR_EFFECT='<effect-name-from-existing-record>'
```

若效果位于 run 的 `delivery` 目录，读取该记录。

```sh
jq . "$EXECUTOR_RUN_DIR/delivery/$EXECUTOR_EFFECT.json"
```

若效果位于持有阶段，先从票级记录确认代次，再读取固定路径。

```sh
EXECUTOR_GENERATION='<generation-read-from-claim>'
jq . "$EXECUTOR_CLAIMS_ROOT/$EXECUTOR_TICKET.effects/$EXECUTOR_RUN_ID-g$EXECUTOR_GENERATION/$EXECUTOR_EFFECT.json"
```

按 `what` 中的目标，使用[GitHub 读回命令和合同](recovery-reference.md#github-读回合同)核对对象。`gh` 只用于 GET，执行器的 GitHub 写入只用 App 身份。

若完整读回命中合同，且 R7 成立，交回原入口认回，不做 `reconcile`。若图已结束，转 R9，保存已存在的对象交主控。

若缺页、权限不足、网络失败或单次读回未见对象，保持结果未知。不要据此声明对象不存在。只有主控核实未生效并满足[对账条件](#主控决策不是恢复)后，才可考虑 `reconcile`。

同名标签和非强推建 ref 在执行器中按幂等效果处理。PR、评论和 status 的 pending 只认读回，不自动再发。操作员不能手动补任何一种写入。

## 在 stale 或 halted 时停止推进

若 R5 成立，读取发布状态和漂移证据。

```sh
jq . "$EXECUTOR_RUN_DIR/delivery/stale.json"
```

若 `delivery/state.json` 已存在，也读取它。

```sh
jq . "$EXECUTOR_RUN_DIR/delivery/state.json"
```

保存冻结 head、`observed`、分支、PR 号和完整远端读回。由主控决定重新评审还是释放。不要收养不同 head，不能继承旧 head 的 PASS。

若 R6 成立，保存票级 `state` 和 `reason`。票被关闭、镜像标签被摘、票或 PR 出现禁用标签、持有 post-check 失败、PR 合同不符都可能使持有 halted。不要重开 PR、改目标、改 draft、补标签或删除 marker 来恢复。持有 halted 后，同 run 的原命令会被 `ClaimRefused` 拒绝。

## 控制进程被杀之后

先使用原状态根读取各层记录。不要因为 `owner.lock` 仍存在而判定进程存活。flock 随进程死亡释放，锁文件可以留下旧 PID。

若锁文件存在，读取 PID。

```sh
cat "$EXECUTOR_RUN_DIR/owner.lock"
EXECUTOR_OWNER_PID='<pid-read-from-owner-lock>'
ps -ww -p "$EXECUTOR_OWNER_PID" -o pid=,ppid=,pgid=,stat=,command=
```

先核对完整命令、`run_id` 和实际状态根。若命令省略状态根，只有原状态根确为 CLI 默认值时才匹配。PID 已复用、进程身份不清或控制进程仍在时，交主控，不发送信号。

若控制进程已死，重新核对 R0 至 R10。工蜂监督进程可能继续运行，GitHub 请求也可能已经生效。满足 R7 时才执行原 run 命令。若返回 `75`，转 R2。若工蜂为 `effect_unknown`，转 R3。若发布为 pending，转 R8。若图已结束，转 R9。

## 写动作的前置条件

先满足对应行的全部条件，再由唯一执行途径决定是否写入。R7 不替代这些检查。操作员不直接调用表中的业务工具。

| 写动作 | 操作前的读回条件 | 唯一执行途径 |
| --- | --- | --- |
| 派工 | 当前票级持有为匹配的 `active`。实时票面允许动作。现有原键必须先被分类。`running` 或 `terminal` 接回原键。只有 `not_started` 或合法新键才可派工，且满足设计、裁判、作者和独审顺序门。任何 `effect_unknown` 都停止。 | 原图调用 `run_worker`，由宿主锁内派工。 |
| 本地提交 | 作者完成，分支和 HEAD 符合回执。已有意图先比对 first parent、tree、trailer。未提交时改动必须在写集内，索引无 `index.lock`，已有意图的 tree 未变。 | 原图调用 `commit_candidate`，认回或完成原提交。 |
| 推送和建 ref | 本 run 回执链冻结 head 有精确异家族 PASS，且本地 tip 和提交区间无漂移。远端 ref 缺失才建，等于冻结 head 才认回，任何其他 SHA 都 stale。对象重建的 SHA 必须逐个相等。 | 原图调用 `publish_candidate`，App git 数据 API 非强推建 ref。 |
| PR | 建立前远端 ref 等于冻结 head。完整读取该分支 `state=all` 的 PR。已有 PR 必须符合合同和精确 head，才能认回。非幂等 pending 未解决时不新建。 | `publish_candidate` 的 PR 效果。 |
| 评论 | 完整读回本票评论，按对应 `what.marker` 和 App Bot 身份认回。持有、撤回 marker 使用 base，交付 marker 使用冻结 head。交付前还须复核票面、PR 和 ref。非幂等 pending 未解决时不再发。 | 持有流程或 `publish_candidate` 的对应评论效果。 |
| status | 完整读取冻结 SHA 的 statuses。只认同 SHA 上 `deepagents/review`、`success` 和指定 App 登录的记录。发布前实时复核票面、PR 和 ref。非幂等 pending 未解决时不再发。 | `publish_candidate` 的 status 效果。 |

字段核对细节见[五者不变量](recovery-reference.md#五者不变量)及[GitHub 读回合同](recovery-reference.md#github-读回合同)。

## 主控决策，不是恢复

操作员把证据交回主控。以下命令只供主控在完成相应决定后执行。

### 核实未生效后对账

只有在主控已确认控制进程停止、现有记录唯一且仍为 `pending`、已按 `what` 完整核实 GitHub 效果未生效，并保存依据后，才可执行对账。若对象已存在、结果仍未知或读回不完整，不执行。

```sh
executor reconcile --run-id "$EXECUTOR_RUN_ID" --effect '<effect-name-from-existing-record>' --note '<时间、查了哪里、完整 marker 或 SHA、确认未生效的依据>'
echo $?
```

`reconcile` 只加 run 锁和票级锁并修改本地记录，不查询 GitHub。它不让已结束的图重新调用发布工具。若返回 `75`，转 R2。对账成功后仍须重新满足 R7，下一次实际执行该效果才会先读回，再决定是否发送。

先将 pending 原文、对账说明和完整读回依据保存到交接记录。当前实现把 pending 放进 `reconciled_not_applied.history`，但随后认回或发送成功写 `done` 时不保留 `history` 和 `note`。不要把效果文件当成永久对账档案。

### 停止控制进程

只有在主控决定停止，且刚用上文 `ps` 核实 PID、完整命令、run 和状态根仍吻合时，才对单个控制 PID 发 TERM。

```sh
kill -TERM "$EXECUTOR_OWNER_PID"
ps -ww -p "$EXECUTOR_OWNER_PID" -o pid=,ppid=,pgid=,stat=,command=
```

若进程未退出，停手交主控继续判断。不要升级信号或停工蜂进程组。控制进程停止也不能证明已发送的 GitHub 写入没有发生。对账和再次运行仍由各自命令取得控制锁，不能用 PID 推定取锁成功。

### 释放、新代次与远端分支处置

只有在主控决定退回，且票级记录仍属于该票和该 run 时，才释放。

```sh
executor release --ticket "$EXECUTOR_TICKET" --run-id "$EXECUTOR_RUN_ID" --reason '<主控退回原因及证据引用>'
echo $?
```

释放取票级动作锁，会等待在飞的那一步写入离开锁。控制进程已死不阻止释放匹配的持有记录。释放只写本机 `released`，不终止工蜂、不撤销 GitHub 对象，也不恢复原 run。

新代次登记由主控另行授权。上一代次的控制锁和工蜂必须满足静默检查。代次由票级记录递增，不能通过改 run 名称自行分配。本节不提供登记命令。

票级发布分支跨代次固定。新代次不因此获得覆盖旧 head 的权利。远端分支、旧 PR 和已有评论的处置由主控决定，不属于本指南的恢复动作。

## 禁止事项

- 不重新登记、换 run、换状态根或 `release` 来冒充恢复。
- 不手工派工、提交、推送、开 PR、评论或写 status。
- 不删锁、删效果记录、改冻结 WorkUnit、改回执或补写 PASS。
- 不因时间经过或一次空读回再次发送结果不明的写请求。
- 不运行 `--crash-at`。它会在指定点 SIGKILL 本进程，是受控故障注入，不是操作员恢复工具，也不能证明本次恢复已验收。
- 不用 v1 `python -m pilot1742` 推进 v2 run。
- 不读取 `.env`、认证文件、私钥内容或钥匙串，不把凭据写入交接记录。

## 核对恢复结果并交接

命令返回后，重新读取本机状态和已有记录。根据操作涉及的对象，按参考合同执行只读 GitHub GET。若本机与远端不一致，回到判别表，不追加写入。

交接记录保留以下内容。

- 票号、原状态根、执行器 `run_id`、绝对 run 目录和票级代次。
- 冻结 base、回执链 head、作者实际 HEAD，以及该 head 的独审实记模型与首行结论。
- 退出码、stderr、工具失败结果和图终态或中断证据。
- 工蜂原键、提交意图及认回的三项比对结果。
- pending 的 `what`、完整 marker 或 SHA、GitHub 读回范围、时间、对象 URL 及是否读全。
- 对账前记录、主控决定和对账说明。成功记录可能已覆盖这些字段。
- 当前停止条件和下一步责任人。`awaiting_merge` 交人决定是否合入，不能写成已合入。

## 本指南不声称已验收的事项

本指南不声称真人签批、Studio 控制、Docker、生产切换或 main 合入已经验收。v2 CLI 用 `recheck_approval=False` 驱动，不消费 v1 批准账本。相关测试文件存在，但本文未在本次任务中运行这些测试，也未执行真实恢复或发布。
