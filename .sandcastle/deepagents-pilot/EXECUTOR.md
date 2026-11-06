# 用新执行器交付一张首证专用票

新执行器把专用票或切流后的引擎放行票从持有跑到 `awaiting_merge`。Deep Agents 主控按 poteto-mode 派 Sandcastle 工蜂，作者与异家族独审都经现役 `codex-child`。独审对某个 head 判 PASS 后，执行器用 GitHub App 身份把这个冻结 SHA 发布到 `codex/deepagents/t<票号>`，开目标为 main 的 draft PR，写 `deepagents/review` 状态和票面交付标记。合入 main 由人在 GitHub 上完成。

`register` 保留首证专用票入口。`serve` 接收切流后的引擎放行队列，见文末。接管旧 owner 手里的票仍一律拒绝。

## 准备专用票与任务文件

专用票须从创建起就不在任何旧消费者的可选集里。票正文单独写一行：

```text
deepagents:dedicated-pilot #1742
```

不要给它加任何标签，不要挂父票。下列任一项成立时登记被拒：timeline 里出现过 `sandcastle`、`ready-for-agent`、`wayfinder:impl`、`wayfinder:grilling` 或任何 `agent:` 标签（摘掉了也算）；票有父票或有过挂接父票事件；`workers.jsonl` 里有这张票；任何一项证据读不全，例如 timeline 某页失败或 `workers.jsonl` 不可读。

任务文件与试点格式相同，写集必须与业务在飞工作不重叠：

```markdown
---
title: 补充新执行器操作员故障恢复指南
allowed_prefixes:
  - docs/agents/deepagents-executor/
require_design: false
---
需求原文……
```

## 运行

在仓库根准备一次环境。私钥只给绝对路径，程序只读这个文件签名。

```sh
cd /Users/silencehan/.codex/worktrees/deepagents-pilot-1742/NewChanlun
VENV=/Volumes/AgentStorage/issue1742-deepagents-pilot-20261027/venv
PEM=/Users/silencehan/Downloads/newchanlun-deepagents-1742.2026-10-26.private-key.pem
export PILOT_BOAT_CONFIG=/绝对路径/boat-config.json
export PILOT_BOAT_API_KEY_FILE=/仓外绝对路径/boat-controller.key
executor() { PYTHONPATH=.sandcastle/approval-signing:.sandcastle/deepagents-pilot "$VENV/bin/python" -m pilot1742.executor "$@"; }
git fetch origin <已推送的分支>
BASE=$(git rev-parse origin/<已推送的分支>)
```

基线是任一已在 GitHub 上的提交的 40 位 SHA，不限于 main。登记器核本地与远端都有这个提交对象，否则拒绝。基线不在 main 上时，目标为 main 的 draft PR 的 diff 会包含 main 到基线之间的提交；本 run 的作者、独审和 `deepagents/review` 状态只覆盖基线到冻结 head 的部分。

**登记。** 只读 GitHub，写本机 WorkUnit。同一参数重复登记返回同一单元，票面变化后再登记同一 run 被拒。

```sh
executor register --ticket <票号> --run-id t<票号>-1 --base "$BASE" \
  --task /绝对路径/task.md --playbook feature \
  --scope-ref "用户批准 SPEC r3 专用票真实验证（call_93f52210616b4154be624f4e2d6bfba7）" \
  --private-key "$PEM"
```

**运行或恢复。** 同一命令既首跑也恢复。它先取 run 锁，再在票级锁内持有票（加 `deepagents` 标签、发持有标记、post-check），然后驱动 Deep Agents 图。主控走完 playbook 后调用 `publish_candidate`。

```sh
executor run --run-id t<票号>-1 --private-key "$PEM"
```

退出码 75 表示这个 run 已有控制进程，或状态根所在卷剩余空间低于派工门槛（16 GiB，`run.DISPATCH_FREE_BYTES`）。空间门在领取持有和新建 checkout 之前检查，拦下时不领取、不建 checkout、不写票面，腾出空间后重跑同一命令。控制进程被杀后直接重跑同一命令，工蜂按原键收养，已做的外部写入先读回再决定。受控故障注入用 `--crash-at`，取值为 `after_label_before_receipt`、`after_claim_before_receipt`、`after_push_before_receipt`、`after_pr_before_receipt`、`after_status_before_receipt`、`after_deliver_before_receipt`，进程在该点被 SIGKILL。

明确决定撤销人工中断并继续运行时才加 `--resume`。恢复 Boat 在途工蜂时也一样：它会清除人工中断，不是只读对账选项；只查看本地事实用 `executor status`。

**人工停点。** `runs/<run>/interrupt.request` 记录一次需要人决定的停止，`by` 字段写明来源：

| `by` | 何时写入 | 图的位置 |
|---|---|---|
| 操作员（如 `studio`） | Studio 取消或操作员请求中断 | 停在下一次工具调用前的 checkpoint |
| `execution_unknown` | v2 工具在派工中遇到 `ExecutionUnknown`（例如 Boat VM 等待 `hydrated` 超时），或观察、补派、收集得到 `EffectUnknown` 且该绑定尚无已证终态的排空证据 | 停在该工具调用的 checkpoint，`reason` 是原文 |
| `brain_stopped_early` | 图已结束，业务相位不是 `gate`/`halted`/`awaiting_merge`/`merged`/`stale`/`conflict`，且当前 head 有已登记、未取消、未收集、未排空的 Boat 绑定 | 图已结束 |

只有 `executor run --resume` 和 Studio 中断处的 Resume 清除停点。`--recheck-approval`、普通 `executor run`、Studio 的 Continue 和队列都不清除。停点已写入、图还没落到中断 checkpoint 时进程崩溃，图会停在待执行的工具前；这时 CLI 与 Studio 的 Continue 都只重跑该工具，工具在调用前看到停点就重新停下。队列看到停点即报 `awaiting_human`，不再起子进程。只有已证终态且已排空的 Boat 绑定（例如已取消并清理）才转 `halted`。

`--resume` 在 checkpoint 处重跑该工具调用，同角色同 head 的工蜂按原绑定收养，已有 VM 不再 fork。图已结束时，只有上表 `brain_stopped_early` 的条件成立才继续：宿主在同一 thread 追加一条固定恢复消息，主控从原历史接着走。条件不成立（正常结束、绑定已取消、已收集或已排空、候选 head 漂移）时 `--resume` 只读，不重开。签批、持有代次与 SPEC 正文漂移在打开 run 时先拒绝，到不了恢复。

**查看。** 只读本机，不访问 GitHub。

```sh
executor status --run-id t<票号>-1
```

**释放。** 主控退回一张票。释放会等正在执行的那一个外部写入做完，旧控制进程之后的派工、提交与发布都会停下，原工蜂仍可经 `cancel-worker` 取消。释放本身不代表远端工蜂已退出。重新持有要登记新的 run id；票级锁内确认旧控制锁、本机工蜂和所有旧 Boat 绑定均已排空后，代次才加一。`Running`、`EffectUnknown` 或不可读的 Boat 绑定通常阻止换代。唯一的失败排空路径要求原绑定已有 `stop_id`、回执身份完全匹配且明确证明失败退出或取消，详见 [取消后释放队列槽或换代](BOAT.md#取消后释放队列槽或换代)。手工入口与队列共享这一判定。检查只读本地冻结记录，不访问 Boat、不补提交，也不收集旧产物。

```sh
executor release --ticket <票号> --run-id t<票号>-1 --reason "主控退回：……"
executor register --ticket <票号> --run-id t<票号>-2 --base "$BASE" ...
```

发布分支按票固定。上一代次若已把某个 head 推到 `codex/deepagents/t<票号>`，新代次发布另一个 head 时会得到 `stale`，执行器不强推，由主控决定如何处置远端分支。

## 读懂结果

`status` 的 `phase` 在发布后取以下值。

- `awaiting_merge`。冻结 head 已在远端分支，draft PR 的 head 等于它，状态与交付标记都在。PR 链接在 `delivery.state.pr.url`。
- `merged`。人已合入该 PR，执行器不再写任何东西。
- `stale`。远端分支或 PR 的 head 不是冻结 head，后代也算。执行器不覆盖、不收养，`delivery.stale.observed` 是观察到的 SHA。由主控决定重新评审还是释放。
- `halted`（持有记录）。锁内复核发现票被关、`deepagents` 标签被摘、票或 PR 被加了禁用标签，或持有 post-check 失败。远端 PR 不合交付合同时也转 halted：已关闭未合入、目标不是 main、open 时不是 draft、不是本 App 所开、正文缺本票 PR 标记。执行器不改动这个 PR，不重开、不改目标、不改回 draft。只有主控能释放。

派工、本地提交与每一次 GitHub 写都在票级锁内重新读票面。发布的每一步写之前还会重新读 PR 与远端分支，已记账的步骤也不跳过这一步，所以恢复时发现 head 变了会先 stale，不会先写状态或交付评论。

## 结果不明时怎么办

工具返回 `EffectPending` 表示一次外部写已经发出、结果不明，读回也没看到它生效。执行器在效果记录里留下 `pending`，并写明要找的对象（`what` 字段，例如评论标记、分支与 SHA、状态上下文）。

- 同名标签与非强推建分支是幂等的，下一次运行会直接再发。
- 评论、PR 与状态不是幂等的。等多久都不会自动再发：请求可能延迟生效，读回也可能暂时看不到。每次运行只读回，读到就认领。
- 只有两种失败会撤销本次 pending：连接根本没建立（`RequestNotSent`），或 GitHub 对这次写请求明确返回 4xx（`WriteRejected`）。下一次运行正常发送。
- 其他失败都保留 pending，包括写请求得到 2xx 但正文不可读、发出后读回请求失败、读回内容不合合同。效果是否成立只由读回判定，执行器不使用写请求的响应正文。

读回一直看不到时由主控对账。先停掉该 run 的控制进程，在 GitHub 上按 `what` 核实对象确实不存在，再写下对账。对账全程持有该 run 的控制锁，控制进程在时命令以 75 退出，对账期间也无法启动控制进程：

```sh
executor reconcile --run-id t<票号>-1 --effect deliver-<head> --note "已在 #<票号> 评论区核实无交付标记"
executor run --run-id t<票号>-1 --private-key "$PEM"
```

对账后下一次运行仍先读回，还是看不到才发一次，原 pending 与对账说明留在记录的 `history` 里。效果记录在 run 目录 `delivery/`（`push-<h>`、`pr-<h>`、`status-<h>`、`deliver-<h>`），持有阶段的在 `claims/<票号>.effects/<run>-g<代次>/`（`label`、`claim`、`withdraw`）。核实对象确实已存在时不要对账，重跑即可，执行器会读回认领。

## 状态放在哪

票级持有记录与动作锁固定在 `/Volumes/AgentStorage/deepagents-exec/claims/<票号>.json` 与 `<票号>.lock`。它们不随 `--state-root` 变化，所以换一个状态根也拿不到同一张票。持有记录绑定 run 目录的绝对路径，另一个状态根下的同名 run 也被拒。

`--state-root`（缺省 `/Volumes/AgentStorage/deepagents-exec`）下有 `units/<run>.json` 与 `runs/<run>/`。run 目录里的 `run.json` 带 `"v": 2` 与冻结的 WorkUnit，`claim.json` 记本 run 的代次，`delivery/` 是发布的待认回记录，`trail.jsonl` 与 `agent-trace.jsonl` 是审计轨迹。

## 身份与发布方式

执行器只用 App installation token，权限为 metadata 读，contents、issues、pull_requests、statuses 写，限定到本仓库。token 只在控制进程内存，工蜂的环境是白名单，不含任何 GitHub 凭据。队列、准入和发布不调用 gh、git 凭据、SSH 或 keychain。`cutover verify/record` 的运维屏障单独调用只读 `gh variable get` 核对冻结变量，记录来源为 `user-token-readonly`，不生成批准、不执行 GitHub 写，也不在 App 失败时改用用户身份。

推送不走 `git push`。执行器经 GitHub git 数据 API 逐个重建 `base..h` 的 blob、tree 与 commit，每个返回的 SHA 必须等于本地 SHA，全部对上才非强推地创建分支 ref。v2 的本地提交用 UTC 时间且不签名，以便原样重建。重建对不上时不创建任何 ref，报 `Unpublishable`。

执行器不调用 `harvest.sh`，不写 `workers.jsonl`，分支命名空间也不在 harvest 的扫描范围内。

## 与 v1 的关系

`python -m pilot1742` 的 v1 CLI、`real3`、`real4` 与 `pilot_acceptance` 账本不变。v2 run 只能经本入口推进；用 v1 CLI 打开 v2 run 做任何副作用都会被拒。

## 接收引擎放行队列

#1742 D6 的队列入口只在全局 `cutover.json` 有效且旧入口保持同代次 `draining` 时工作。切流准备与回退见 [冻结旧入口并让在飞任务收尾](../docs/migration-drain-1742.md)。切流不改变专用票准入，也不打开 `Takeover`。

引擎只给合资格实装票挂 `deepagents:queue`。队列允许 `ready-for-agent` 和 `wayfinder:impl`，拒绝 `sandcastle`、`agent:*`、`wayfinder:grilling`、所有人闸标签、有负责人、有 blocker 或 SPEC 不在票的祖先链上，也不与票同属实际 map的票。旧日志的未知事件和未收尾 claim 也会阻止准入。

所属 SPEC 正文必须声明写集。多个目录用空格分隔：

```text
deepagents-scope: rust/src/example/ rust/tests/example/
```

队列只从受 SPEC 签名绑定的正文读取写集。切流记录同时绑定日志绝对路径与字节位置，所有队列动作都读取这份日志，包括 Studio 打开的新队列 run。实装票自己新增的 scope 行不授予权限。缺少声明时，状态页显示 `awaiting_human` 和 `deepagents-scope`，不会扩大到整个仓库。补正文会改变 SPEC 摘要，因此需要对新正文签批。登记时冻结 main 的当前 SHA、写集和任务，缺少该本地提交对象时要求先部署或 fetch，不在后台擅自更新源检出。

### 为新队列 run 提供 SPEC 授权

D8 的请求绑定 `ticket`、`run_id`、`claim_generation`、`spec_authorization` 与 SPEC 正文原始字节。已有同绑定、同正文且未过期的已发布签批直接复用。历史评论关键词批准和另一个 run 的签批不能复制到新 run。它们的历史批准记录不改写。

签批判定为 approved 后，宿主在同一次动作锁内把那份正文原始字节写入 `runs/<run>/spec/<SPEC 号>.md`，旁注 `spec/<SPEC 号>.json` 记签批目标与代次。两个文件之间崩溃留下的错配，在下一次 approved 判定时按读到的原字节重写收敛，不需要人工删目录。读出时重算摘要，与签批目标不符即拒绝。每个 Boat 工蜂的 request 附封包时的这份正文。主控的系统提示在 `build_agent` 时取快照，同一控制进程内重签不会刷新提示；下一次打开 run 才用新正文。放行不依赖提示，只看独审封包（见下文）。它是数据，不是批准：放行仍只看每个动作边界的签批与正文复核。工蜂没有 GitHub 凭据，不读票面。

宿主先完成旧 prompt 的路径映射，再原样附上这份正文，正文字节不改写。Boat 工蜂的封包在 `request.json` 的 `spec` 中记下所附正文的摘要。派工前（包括 `prepared` 续派和监督未认领时的补派）以及收集回执前，都在动作锁内核对它是否仍等于当前签批目标。不符时落 `execution_unknown` 停点，不重封包、不派工、不收集。派工前不符时，原包、绑定与 VM 都保留。收集前不符时，回执已经取回，按既有回收顺序删除 key 并停止 VM，原回执保留、checkout 不写。处置走既有入口：`cancel-worker` 把该绑定标为已取消，旧 SPEC 的完成回执作废但原样保留，不记成功，此后满足 [排空条件](BOAT.md#取消后释放队列槽或换代)，`release` 后换代。本版本之前封包、`request.json` 没有 `spec` 的 `prepared`、`submitted` 或已收集绑定都按不符处理，走同一条取消、排空、换代路径；t1863-1 当时尚未封包，按原身份恢复。

发布同样以此为判据：`publish_candidate` 在推送、PR、状态、交付四个动作锁内，等签批与正文重核之后，核对放行用的同 head 独审封包 `spec` 是否等于当前签批目标。不符时返回 `decision=refused`、`reasons=["review_spec_mismatch"]`，停在下一次写之前；已完成的写、原包、回执与候选都保留，日志重放时同样先核对。同 head 的独审键已收集，不能重派，要在新 SPEC 下取得独审须走 `release` 后换代。

首次 `serve --once` 会登记 `t<票号>-1`，在任何票面写入前生成 `runs/<run>/approvals/spec_authorization.request`。状态页列出请求路径及签批命令。由批准人在受管签批工具中核对、签名并发布该请求，再运行下一轮。签名未发布、拒绝或已过期时保持等待。每个动作都重读权威决定链和 SPEC 正文。正文变化时暂停并生成新请求，模型恢复载荷不算批准。

这条要求只作用于新的 `Queued` 准入。v1 账本、已持有 `DedicatedPilot` run 与原来的专用票交付条件不变。队列默认不要求 `candidate_head` 签批。它只交付到 `awaiting_merge(h)`，不合 main。

### 运行队列

```sh
executor serve --once --workers-log "$PWD/.sandcastle/logs/workers.jsonl" --private-key "$PEM"
executor serve --loop --poll-seconds 60 --workers-log "$PWD/.sandcastle/logs/workers.jsonl" --private-key "$PEM"
```

`serve` 沿用旧 claim-loop 单票在飞的上限。票级 ClaimStore 和全局 serve 锁防止两个控制进程重复持有。返回 75 或被 SIGKILL 的子进程在下一轮恢复同一 run 和代次；返回 0 或 2 也不代表工蜂结束。队列在派下一票前只读原 run 的工蜂事实，也枚举没有 intent 的 Boat 绑定。`Running`、`EffectUnknown` 或无法读取的工蜂状态继续占槽，包括已中断、halted、released 或已摘队列标签的票；已满足 [失败排空条件](BOAT.md#取消后释放队列槽或换代) 的绑定可释放槽位。队列的检查不访问 Boat，不收集或重派工蜂；原控制入口负责恢复和收集。完成的 run 不重新派工，发布沿用逐效果 Journal 的读回去重。派工前状态根所在卷剩余空间低于派工门槛时，该票保持 `queued` 并写明可用与所需字节，本轮不再派工，下一轮重试；已交付 run 的读回对账、`cancel-worker`、`reconcile` 与 `release` 不经过这道门。

旧 claim 已 `released` 且工蜂已排空时，仍在队列内的票登记或收养下一个 run；旧 run 的终态不阻止新 run 进入票级锁内的排空核验和换代。新 run 仍须取得绑定新 run 与代次的 SPEC 签批，之后才增加持有代次。未排空的旧 run 继续占槽，不登记下一代。

队列登记先完整写入并同步临时文件，再排他发布 `units/<run>.json`。旧版本遗留的损坏单元保留原件，在状态页按票显示路径和错误，不重建或覆盖该票。没有在途工蜂的坏单元不阻挡其他票；该票仍有运行中或未知工蜂时继续占槽。

状态分为 `queued`、`active`、`awaiting_human`、`awaiting_merge` 和 `halted`，写入部署检出的 `.sandcastle/logs/deepagents-status.md` 与状态根的 `queue-status.json`。队列子进程使用 `executor run --recheck-approval` 重读权威签批链。即使人工中断发生在队列检查之后，它也只恢复批准核验，不清除中断请求，本轮状态仍为 `awaiting_human`。CLI 中只有显式 `executor run --resume` 清除人工中断请求，再按权威签批链恢复同一 run。普通 `executor run` 不清除人工中断。Studio 的人工 Continue 入口保持不变。没有有效切流记录时 `serve` 退出码为 0，说明停用原因，不读取 App、不登记单元、不改票面。

launchd 模板在 `.sandcastle/launchd/com.newchanlun.deepagents-executor.plist`。部署 owner 把 `__REPO__`、`__VENV__`、`__PRIVATE_KEY_PATH__`、`__BOAT_CONFIG__` 和 `__BOAT_API_KEY_FILE__` 全部替换为已核对的绝对路径后安装。后两个占位符分别设置 `PILOT_BOAT_CONFIG` 与 `PILOT_BOAT_API_KEY_FILE`。配置和控制端 key 文件的要求见 [Boat 受管配置](BOAT.md#受管配置)。

使用模板的 `PATH` 执行 `command -v boat`，确认服务能找到已登录的 CLI。模板默认包含 `/Users/silencehan/.local/bin`。App 私钥路径在参数中，Boat key 文件路径在环境中，两种密钥的内容都不进入 plist。
