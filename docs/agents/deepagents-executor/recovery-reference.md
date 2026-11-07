# 执行器故障恢复参考

本参考对应基线 `bcc83ab1cef86b98b67fbf88b06090b72e100b72`，为 #1746 的操作员文档。操作流程见[恢复中断或停止的执行器 run](recover-a-run.md)。本页中的源码行号均按该基线核对，不代表后续版本的位置。

[EXECUTOR.md](../../../.sandcastle/deepagents-pilot/EXECUTOR.md) 的“运行”“结果不明时怎么办”“状态放在哪”“身份与发布方式”说明原入口与机制。本文补充的限制以源码为准，尤其是成功记录覆盖对账历史和图终态不能继续的问题。

## 状态位置与输出层次

下表中的 `<state-root>` 是原 `--state-root`，缺省为 `/Volumes/AgentStorage/deepagents-exec`。持有目录固定为 `/Volumes/AgentStorage/deepagents-exec/claims`，不随该参数变化。

| 层 | 位置 | 核对字段与含义 | 本基线依据 |
| --- | --- | --- | --- |
| 登记 | `<state-root>/units/<run-id>.json` | `work_unit` 与 `task`。WorkUnit 包含票号、run、base、写集、playbook、设计要求和准入依据。 | [executor.py:515](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L515)、[executor.py:531](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L531) |
| 冻结 run | `<state-root>/runs/<run-id>/run.json` | `v=2`、`run_id`、`ticket`、`base`、`work_unit`、checkout 和冻结模型。已有 run 的 WorkUnit 必须与登记一致。 | [run.py:170](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L170)、[executor.py:538](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L538) |
| 票级持有 | `/Volumes/AgentStorage/deepagents-exec/claims/<ticket-number>.json` | `run_id`、绝对 `run_dir`、`generation`、`state`、`reason`。状态有 `intent`、`mirrored`、`active`、`halted`、`released`。 | [claims.py:16](../../../.sandcastle/deepagents-pilot/pilot1742/claims.py#L16)、[claims.py:43](../../../.sandcastle/deepagents-pilot/pilot1742/claims.py#L43) |
| run 内代次 | `<state-root>/runs/<run-id>/claim.json` | `ticket`、`run_id`、`generation`。同 run 恢复时不得与票级记录不同。 | [run.py:426](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L426) |
| 工蜂 | run 下 `intents/<role>-<head>.json` 与 `workers/<role>-<head>/{state,result}.json` | 派工键、实记模型、终态、期望 head、进程身份。工蜂 UUID 与执行器 run 不同。 | [run.py:273](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L273)、[run.py:604](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L604) |
| 本地提交 | run 下 `commits/author-<parent>.intent.json` 与 `.receipt.json` | intent 的 `author`、`parent`、`tree`、`paths`、`trailer`、`generation`。receipt 包含 `intent`、`commit`、`recovered`。 | [run.py:532](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L532)、[run.py:732](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L732)、[run.py:785](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L785) |
| 发布效果 | run 下 `delivery/{push,pr,status,deliver}-<head>.json` | `state`、`what`、`result`、`recovered`。pending 有 `sent_at`、`pid`，对账记录有 `note`、`history`。字段随状态改变。 | [delivery.py:80](../../../.sandcastle/deepagents-pilot/pilot1742/delivery.py#L80)、[delivery.py:111](../../../.sandcastle/deepagents-pilot/pilot1742/delivery.py#L111) |
| 发布结果 | run 下 `delivery/state.json` 与 `delivery/stale.json` | state 有 `phase`、`head`、`branch`、`pr`。stale 有 `head`、`observed`、`at`。 | [executor.py:443](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L443) |
| 持有效果 | `/Volumes/AgentStorage/deepagents-exec/claims/<ticket-number>.effects/<run-id>-g<generation-read-from-claim>/{label,claim,withdraw}.json` | 同一 Journal 格式。代次取票级记录，不能按 run 后缀猜测。 | [executor.py:211](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L211)、[executor.py:461](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L461) |
| 图 | run 下 `graph.sqlite`、`trail.jsonl`、`agent-trace.jsonl` 和原 run 输出 | 图快照有 `has_checkpoint`、`interrupts`、`next`。CLI 输出有 `graph`，`executor status` 没有。 | [cli.py:28](../../../.sandcastle/deepagents-pilot/pilot1742/cli.py#L28)、[cli.py:74](../../../.sandcastle/deepagents-pilot/pilot1742/cli.py#L74) |
| 控制锁 | run 下 `owner.lock` | 内容是最近取锁者 PID。文件存在不等于锁被占用。`run` 和 `reconcile` 都通过 flock 排他取锁。 | [run.py:348](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L348)、[executor.py:480](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L480) |
| 票级动作锁 | `/Volumes/AgentStorage/deepagents-exec/claims/<ticket-number>.lock` | 每一步派工、提交和 GitHub 写入在锁内复核持有与实时票面。锁等待上限为 300 秒，超限抛 `LockTimeout`。 | [claims.py:17](../../../.sandcastle/deepagents-pilot/pilot1742/claims.py#L17)、[claims.py:110](../../../.sandcastle/deepagents-pilot/pilot1742/claims.py#L110)、[executor.py:283](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L283) |

`executor status` 顶层输出为 `claim` 和 `status`。`executor run` 的图驱动输出为 `graph` 和 `status`。工具失败还可能出现在 `agent-trace.jsonl` 的工具结果中。

`status.phase` 从工蜂与提交事实推导，仅在该阶段为 `gate` 时采用发布阶段。票级 `claim.state=halted` 不被合并进去。无冲突时 `status.conflict=null`，不是 `false`。依据为 [run.py:814](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L814) 和 [domain.py:384](../../../.sandcastle/deepagents-pilot/pilot1742/domain.py#L384)。这些层不能压成一个阶段枚举。

### 工蜂与提交

工蜂结果由 [run.py:273](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L273) 校验票号、终态、`expected_head` 和模式。`result.json.run_id` 进入 `child_run_id`，是工蜂 UUID。独审的实记模型取回执，不取请求模型。

[domain.py:119](../../../.sandcastle/deepagents-pilot/pilot1742/domain.py#L119) 的分类顺序如下。

1. 有可解析的结果时为 `terminal`。
2. 监督进程仍活着时为 `running`。
3. 没有输出目录时为 `not_started`。
4. 输出目录存在、没有结果且监督进程不在时为 `effect_unknown`，无论工蜂进程组是否还活着。

回执解析失败也产生 `effect_unknown`。监督进程靠命令行中唯一的 `--output-dir` 识别，僵尸进程不算存活。工蜂进程组身份核对 `state.json.process_pid`、进程组首进程和 `codex` 命令，见 [run.py:334](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L334)、[run.py:504](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L504)。

现有键由执行器接回。`effect_unknown` 在驱动工蜂时记录 `halted` 并抛 `Halted`，新键也不能绕过未成功或未知的旧键。依据为 [run.py:580](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L580)、[run.py:644](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L644)、[domain.py:339](../../../.sandcastle/deepagents-pilot/pilot1742/domain.py#L339)。

提交认回由 [domain.py:165](../../../.sandcastle/deepagents-pilot/pilot1742/domain.py#L165) 判定。实际 HEAD 等于 intent.parent 表示尚未提交。HEAD 已变时，只有 first parent 等于 intent.parent、tree 等于 intent.tree、trailer 值集合含 intent.trailer，才认回原提交。其余为冲突。

v2 的 trailer key 是 `Deepagents-Run`。值形如 `<run-id>/author-<parent> gen=<generation-read-from-claim>`。格式来自 [domain.py:140](../../../.sandcastle/deepagents-pilot/pilot1742/domain.py#L140)。认回成功只补 receipt，不再提交，见 [run.py:735](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L735)。

## 五者不变量

这里的五者为执行器 `run_id`、票级持有代次、冻结 head、精确异家族 PASS 和 App 发布。它们的关系由以下七条约束描述。

1. 持有身份由票号、`run_id`、绝对 `run_dir` 和 `generation` 组成。业务动作要求持有为 `active` 且身份吻合。票号定位固定的持有文件，`holds` 核对其余三项。代次由票级记录分配，再绑定进 run 的 `claim.json`，不由 run 后缀推算。证据为两份 claim、冻结 WorkUnit，以及 [claims.py:56](../../../.sandcastle/deepagents-pilot/pilot1742/claims.py#L56)、[executor.py:208](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L208)、[run.py:426](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L426)。
2. 冻结 head 取本 run 从 base 出发的提交回执链末端。远端最新 head、PR head 和工蜂 session 均不能代替它。提交已做而回执未落时，先通过三项比对补回执，之后才形成链上的 head。发布还要求作者实际 tip 与链一致，`base..head` 不含链外提交。证据为 `commits/*.receipt.json`、提交意图、`status.recorded_head` 和实际 Git HEAD，见 [domain.py:294](../../../.sandcastle/deepagents-pilot/pilot1742/domain.py#L294)、[executor.py:295](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L295)。
3. PASS 只属于同一个 head。review 必须为 `completed`，summary 首行整行等于 `VERDICT: PASS`，独审实记模型与生成该提交的作者实记模型属于不同家族。PASS 不跨 head 继承。证据为该 head 对应的作者和独审回执、提交 intent.author，以及 `status.verdicts[head]`，见 [domain.py:190](../../../.sandcastle/deepagents-pilot/pilot1742/domain.py#L190)、[run.py:545](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L545)。
4. App 只将通过上述门的冻结 SHA 精确发布到 `codex/deepagents/t<ticket-number>`。执行器经 git 数据 API 重建对象，逐个核对 SHA，非强推建 ref，再建 draft PR、写 `deepagents/review` 和交付评论。发布过程中远端 ref 或 PR head 出现任何其他 SHA，含后代，都算 stale。PR 已合入时的复核不再要求分支 ref 仍存在。证据为冻结回执链、`delivery` 文件和远端 GET，见 [executor.py:385](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L385)、[executor.py:371](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L371)、[delivery.py:252](../../../.sandcastle/deepagents-pilot/pilot1742/delivery.py#L252)。
5. 作者分支为 `codex/deepagents-run/<run-id>-author`，票级发布分支为 `codex/deepagents/t<ticket-number>`。发布分支跨代次固定，新代次不获得覆盖旧 head 的权利。证据为冻结 checkout、实际作者分支、WorkUnit.branch 和远端 ref，见 [run.py:237](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L237)、[workunit.py:138](../../../.sandcastle/deepagents-pilot/pilot1742/workunit.py#L138)、[executor.py:313](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L313)。
6. marker 的覆盖范围按对象不同。claim、withdraw、PR 和 deliver 正文在生成时带 ticket、run、gen、head。claim 和 withdraw 的 head 是 unit.base，PR 和 deliver 的 head 是冻结 head。评论认回检查完整 marker 子串及 App Bot 身份。PR 认回只检查本票标记前缀和 head 片段，没有逐项核验 run、gen。ref 没有 marker，只按分支和精确 SHA 认回。status 没有 run、gen、head marker，只在指定 SHA 的列表中按 context、success 和 App 登录认回。证据为 `what`、原正文与远端对象，见 [delivery.py:122](../../../.sandcastle/deepagents-pilot/pilot1742/delivery.py#L122)、[delivery.py:159](../../../.sandcastle/deepagents-pilot/pilot1742/delivery.py#L159)、[executor.py:322](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L322)、[executor.py:378](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L378) 及下一节的读回合同。
7. App 发布的对象不代表人类批准。`awaiting_merge` 只表示等待人决定是否合入 main。人类合入事实来自 PR 的远端状态，不来自 PASS 或发布标记。证据为 `delivery/state.json` 与 PR GET，见 [executor.py:365](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L365)、[executor.py:447](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L447)。

## GitHub 读回合同

以下是只读命令样例，沿用 how-to 中的环境变量。列表查询全部分页，PR 列表包含 `state=all`。这些命令不创建或修改 GitHub 对象。`gh` 沿用现有认证，仅用于读；业务写入只经执行器的 App 身份。

读取范围缺页、权限不足、网络错误或单次不可见时，结果仍未知。一次空列表或一次 404 都不足以授权重新发送非幂等 pending。执行器的对象查找合同和主控“确认未生效”的判断是两件事。

### 票面、标签与持有标记

票面和完整评论的 GET 样例如下。

```sh
gh api --method GET "repos/$EXECUTOR_REPO/issues/$EXECUTOR_TICKET"
gh api --method GET --paginate "repos/$EXECUTOR_REPO/issues/$EXECUTOR_TICKET/comments" -f per_page=100
gh api --method GET --paginate "repos/$EXECUTOR_REPO/issues/$EXECUTOR_TICKET/timeline" -f per_page=100
gh api --method GET "repos/$EXECUTOR_REPO/issues/$EXECUTOR_TICKET/parent"
```

`label` 的 probe 只检查票面存在同名 `deepagents` 标签。claim 和 withdraw 评论认回要求 `user.login` 等于 App 登录、`user.type=Bot`，且正文包含 `what.marker` 的完整字符串。marker 格式如下。

```text
<!-- deepagents:claim ticket=<ticket-number> run=<run-id> gen=<generation-read-from-claim> head=<base-40hex-read-from-work-unit> -->
<!-- deepagents:withdraw ticket=<ticket-number> run=<run-id> gen=<generation-read-from-claim> head=<base-40hex-read-from-work-unit> -->
<!-- deepagents:deliver ticket=<ticket-number> run=<run-id> gen=<generation-read-from-claim> head=<frozen-head-40hex> -->
```

marker 生成规则见 [delivery.py:122](../../../.sandcastle/deepagents-pilot/pilot1742/delivery.py#L122)。claim 和 withdraw 使用 base 的位置为 [executor.py:216](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L216)、[executor.py:256](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L256)。评论查找见 [delivery.py:159](../../../.sandcastle/deepagents-pilot/pilot1742/delivery.py#L159)，交付标记见 [executor.py:432](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L432)。

准入还会读完整 timeline、父票和旧 `workers.jsonl`，读不全直接拒绝，不按空集处理。实时动作要求票 open、镜像标签仍在、票与相关 PR 无禁用标签。禁用项为 `sandcastle`、`ready-for-agent`、`wayfinder:impl`、`wayfinder:grilling` 和所有 `agent:` 前缀。证据见 [executor.py:78](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L78)、[executor.py:267](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L267)、[workunit.py:11](../../../.sandcastle/deepagents-pilot/pilot1742/workunit.py#L11)。

### 精确分支与 PR

冻结 SHA 和 PR 号从现有证据取得。

```sh
EXECUTOR_FROZEN_HEAD='<frozen-head-40hex>'
EXECUTOR_PR_NUMBER='<pr-number-read-from-evidence>'
gh api --method GET "repos/$EXECUTOR_REPO/git/ref/heads/codex/deepagents/t$EXECUTOR_TICKET"
gh api --method GET --paginate "repos/$EXECUTOR_REPO/pulls" -f state=all -f "head=xy7365527-lang:codex/deepagents/t$EXECUTOR_TICKET" -f per_page=100
gh api --method GET "repos/$EXECUTOR_REPO/pulls/$EXECUTOR_PR_NUMBER"
gh api --method GET "repos/$EXECUTOR_REPO/issues/$EXECUTOR_PR_NUMBER"
```

ref 的合同为准确的 `refs/heads/codex/deepagents/t<ticket-number>`、`object.type=commit` 和 `object.sha` 等于冻结 SHA。其他 SHA 直接 stale。ref 缺失可被 probe 视为未建分支，`_require_ref` 要求已存在时缺失则为 `Stale("absent")`，见 [delivery.py:178](../../../.sandcastle/deepagents-pilot/pilot1742/delivery.py#L178)、[executor.py:308](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L308)。

PR 选择先筛选该 head 分支，多个 open PR 被拒绝。有一个 open 时选它，没有 open 时选最大 PR 号。不会只看默认最多 30 条的列表，见 [executor.py:354](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L354) 和 [delivery.py:193](../../../.sandcastle/deepagents-pilot/pilot1742/delivery.py#L193)。

PR 合同包含以下字段。

- `head.ref` 为本票发布分支，`head.repo.full_name` 为本仓。
- `base.ref=main`。
- `user.login` 为指定 App 登录，`user.type=Bot`。
- 正文包含 `<!-- deepagents:pr ticket=<ticket-number> ` 前缀。
- open PR 必须 `draft=true`。closed 且未 merged 被拒绝。
- `head.sha` 精确等于冻结 SHA，否则 stale。
- 正文含 ` head=<frozen-head-40hex> -->` 片段，否则合同被拒绝。

代码不逐项核验 PR marker 中的 run 和 gen，也不验证上述两个片段一定属于同一个 marker。依据为 [executor.py:322](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L322)、[executor.py:338](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L338)。完整 marker 可以作为交接证据，不能写成执行器已核验了这些额外字段。

每个后续发布写入前都会重新读 PR，open 时还核对远端 ref。已有成功回执不能替代该复核。若 PR 已 merged，发布路径跳过后续 status 和 deliver 写入，记录 merged，见 [executor.py:371](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L371)、[executor.py:422](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L422)。

### status 与交付评论

指定 SHA 上的完整 statuses GET 样例如下。交付评论复用前面的完整评论查询。

```sh
gh api --method GET --paginate "repos/$EXECUTOR_REPO/commits/$EXECUTOR_FROZEN_HEAD/statuses" -f per_page=100
```

status probe 查询目标 SHA，要求 `context=deepagents/review`、`state=success`、`creator.login` 为指定 App。它不检查 run、gen、描述正文或 marker，也不额外检查 creator.type。成功回执含 `id`、context 和 SHA，见 [executor.py:378](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L378)、[delivery.py:203](../../../.sandcastle/deepagents-pilot/pilot1742/delivery.py#L203)。

交付评论按完整 deliver marker 和 App Bot 身份认回。评论和 status 是不同效果，不能用一个的成功证明另一个成功。实现位置为 [executor.py:423](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L423)、[executor.py:432](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L432)。

本基线 App slug 固定为 `newchanlun-deepagents-1742`。`bot_login` 由 slug 加 `[bot]` 得到，因此预期登录名为 `newchanlun-deepagents-1742[bot]`。此为源码确定值，不是本次远端身份验收。依据为 [executor.py:564](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L564)、[github_app.py:128](../../../.sandcastle/deepagents-pilot/pilot1742/github_app.py#L128)。

## pending、对账与释放

`Journal.converge` 的记录规则如下，见 [delivery.py:80](../../../.sandcastle/deepagents-pilot/pilot1742/delivery.py#L80)。

- 非 done 记录先 probe。找到对象即写 done。
- 非幂等 pending 未读到对象时抛 `EffectPending`，不再次发送。
- 同名标签、非强推建 ref 为幂等效果。它们在未命中时可由执行器再次发送。
- 发送之前写 pending。只有 send 自身抛 `RequestNotSent` 或 `WriteRejected` 才恢复此前记录。
- `OutcomeUnknown`、发送后读回异常、响应正文不可读或读回不合合同不清除 pending。写响应正文不是效果成立的依据。
- `reconciled_not_applied` 后，下一次实际调用该效果仍先 probe。仍未命中时才发送一次。

`reconcile` 先取 run 锁，再取票级锁。它在 run 的 `delivery` 和该 run 所有代次的持有效果目录里寻找同名文件，要求恰好一个匹配且为 pending。效果参数不带 `.json`。它不按效果种类设白名单，因此 `push-<head>`、`pr-<head>`、`status-<head>`、`deliver-<head>`、`label`、`claim`、`withdraw` 在满足唯一 pending 条件时都可被改为 `reconciled_not_applied`。这只是本地记录能力，不证明原图还有路径执行该效果。尤其 withdraw 只在持有 post-check 失败路径调用，halted 后同 run 会先被持有检查拒绝。依据为 [executor.py:474](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L474)、[executor.py:224](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L224)、[executor.py:193](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L193)。

对账不访问 GitHub，也不核实调用者的说明。它没有在函数中调用 `require_holder`。主控确认身份、未生效事实和授权是操作前提，不能用命令成功替代，见 [executor.py:479](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L479)。

对账时，pending 和 note 暂存在 `reconciled_not_applied` 记录内。之后认回成功或发送成功写 done 时，记录只保留 `state`、`result`、`recovered`、`what`，不保留 history 或 note。此处修正 [EXECUTOR.md:103](../../../.sandcastle/deepagents-pilot/EXECUTOR.md#L103) 对历史保留的概括，实际行为见 [delivery.py:88](../../../.sandcastle/deepagents-pilot/pilot1742/delivery.py#L88)、[delivery.py:108](../../../.sandcastle/deepagents-pilot/pilot1742/delivery.py#L108)、[delivery.py:111](../../../.sandcastle/deepagents-pilot/pilot1742/delivery.py#L111)。交接记录才保留操作员已经读到的对账依据。

`release` 不取 run 锁，只取固定票级动作锁并核对票号对应记录的 run。控制进程已死时，记录仍匹配即可释放。它将 `intent`、`mirrored`、`active`、`halted` 或 `released` 写为 `released`，不调用 GitHub。它不等待全部工蜂完成，也不终止进程。新代次 claim 另有上一 run 的控制锁和工蜂静默检查。依据为 [executor.py:465](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L465)、[claims.py:21](../../../.sandcastle/deepagents-pilot/pilot1742/claims.py#L21)、[executor.py:172](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L172)。

## CLI 行为与图终态

退出码 `75` 为 `EXIT_BUSY`，见 [cli.py:25](../../../.sandcastle/deepagents-pilot/pilot1742/cli.py#L25)。v2 的 `run` 和 `reconcile` 分别在 [executor.py:634](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L634)、[executor.py:604](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L604) 捕获 Busy。

执行器捕获的异常输出 `类型名: 消息` 并返回 `2`。范围包含 ClaimRefused、Fenced、LockTimeout、EffectPending、IdentityError、Unpublishable、Halted、ToolRefusal、ValueError、FileNotFoundError。AdmissionRefused 继承 ClaimRefused。参数解析发生在捕获块之前，参数错误由 argparse 以 `2` 退出。依据为 [executor.py:67](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L67)、[executor.py:70](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L70)、[executor.py:598](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L598)、[executor.py:640](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L640)。

`drive` 正常返回 `0`，包括未重检的图中断，以及图执行后业务尚未交付的情况。v2 传入 `recheck_approval=False`。图的分派为无 checkpoint 时开始、存在 interrupt 时返回、不含 interrupt 且 next 非空时恢复、已有 checkpoint 且两者都空时记 `graph_already_finished`。依据为 [executor.py:639](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L639)、[cli.py:78](../../../.sandcastle/deepagents-pilot/pilot1742/cli.py#L78)。

图终态的证据为输出或 trail 中的 `graph_already_finished`，或者 `has_checkpoint=true`、`interrupts=[]`、`next=[]` 三者同时成立。单看 next 为空不充分。`executor status` 不查询图，不能单独证明图仍可继续。

`publish_candidate` 可以把 `EffectPending` 等异常转成工具 JSON，图随后可能正常结束，见 [agent.py:129](../../../.sandcastle/deepagents-pilot/pilot1742/agent.py#L129)。`reconcile` 只改 Journal，不改图 checkpoint。已结束的图再次运行只记 `graph_already_finished`。CLI 没有“继续调用发布工具”的参数。

stale 没有专用退出码。发布代码捕获 Stale，写 `delivery/stale.json` 并返回 `decision=stale`。若图正常结束，CLI 可返回 `0`。整个 run 最终退出码仍取决于图执行是否正常返回，不能单凭 stale 推定。依据为 [executor.py:443](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py#L443) 和 [cli.py:100](../../../.sandcastle/deepagents-pilot/pilot1742/cli.py#L100)。

## 已知限制与开放问题

以下为本基线可以从代码确认的限制。

1. 已结束图没有 CLI 继续入口。对账后不保证发布工具被再次调用。
2. v2 不重检图中的批准 interrupt，也不消费 v1 批准账本，见 [run.py:798](../../../.sandcastle/deepagents-pilot/pilot1742/run.py#L798)。
3. `status` 不带图状态，不合并持有 halted，也不访问 GitHub。单一 phase 不能判断是否能恢复。
4. `effect_unknown` 不自动修复。它停止原键，新键也被派工门拒绝。
5. 成功 done 记录覆盖对账历史。效果文件不提供永久完整的对账链。
6. reconcile 支持唯一 pending 的各种效果名，但并不恢复业务控制流。withdraw 的本地对账能力不意味着 halted run 能继续撤回。
7. release 在控制进程已死时仍可作用于匹配记录，只做本机释放。它不会清理外部对象。
8. stale 的业务结果和进程退出码不同。源码确定没有单独的 stale 退出码。
9. App Bot 的预期登录名已由代码确定。PR marker 的 run、gen 不被逐项核验，status 和 ref 根本不带这些 marker。

以下事实在本次文档任务中未确认，不能作为恢复准入或验收证据。

- 某个实际 run 的当前 checkpoint、持有状态和远端对象。本次没有访问真实运行记录或 GitHub。
- 已发送但结果未知的请求最终是否生效。停止控制进程、经过时间或一次空读回都不能回答。
- 某个实际 pending 是否满足“完整核实未生效”。该结论由主控根据当次证据作出，reconcile 不代查。
- 真人签批、Studio 控制、Docker、生产切换和 main 合入。相关实现或测试文件的存在不构成这些事项已验收。

本次只采集帮助和核对源码，没有执行 register、run、reconcile、release、`--crash-at` 或 GitHub 写入，也没有运行相关测试。`.sandcastle/deepagents-pilot/tests/` 中的断言不是本次运行通过的声明。

## 本基线 CLI 帮助原文

下列六段为本 checkout 在基线 `bcc83ab1cef86b98b67fbf88b06090b72e100b72` 上重新采集的 stdout。六次帮助命令均以 `0` 退出。每段前的命令可在同一基线仓库根重新采集帮助，只触发参数帮助，不执行子命令业务。

### 顶层

```sh
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=.sandcastle/deepagents-pilot /Volumes/AgentStorage/issue1742-deepagents-pilot-20261027/venv/bin/python -B -m pilot1742.executor --help
```

```text
usage: pilot1742.executor [-h] [--state-root STATE_ROOT]
                          {register,run,status,reconcile,release} ...

新执行器：v2 专用票登记、持有、运行与释放

positional arguments:
  {register,run,status,reconcile,release}
    register            主控登记首证专用票（只读 GitHub，写本机 WorkUnit）
    run                 持有并运行或恢复一个已登记的 v2 run
    status              只读本机状态，不访问 GitHub
    reconcile           主控核实 pending 效果在 GitHub 上没有生效后，允许再发一次
    release             主控释放持有（等在飞副作用结束）

options:
  -h, --help            show this help message and exit
  --state-root STATE_ROOT
```

### register

```sh
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=.sandcastle/deepagents-pilot /Volumes/AgentStorage/issue1742-deepagents-pilot-20261027/venv/bin/python -B -m pilot1742.executor register --help
```

```text
usage: pilot1742.executor register [-h] --ticket TICKET --run-id RUN_ID --base
                                   BASE --task TASK --playbook
                                   {feature,bug-fix,refactoring} --scope-ref
                                   SCOPE_REF [--workers-log WORKERS_LOG]
                                   --private-key PRIVATE_KEY

options:
  -h, --help            show this help message and exit
  --ticket TICKET
  --run-id RUN_ID
  --base BASE
  --task TASK
  --playbook {feature,bug-fix,refactoring}
  --scope-ref SCOPE_REF
                        用户批准本 SPEC 的记录引用，只作引用
  --workers-log WORKERS_LOG
  --private-key PRIVATE_KEY
```

### run

```sh
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=.sandcastle/deepagents-pilot /Volumes/AgentStorage/issue1742-deepagents-pilot-20261027/venv/bin/python -B -m pilot1742.executor run --help
```

```text
usage: pilot1742.executor run [-h] --run-id RUN_ID [--workers-log WORKERS_LOG]
                              --private-key PRIVATE_KEY
                              [--crash-at {after_label_before_receipt,after_claim_before_receipt,after_push_before_receipt,after_pr_before_receipt,after_status_before_receipt,after_deliver_before_receipt}]

options:
  -h, --help            show this help message and exit
  --run-id RUN_ID
  --workers-log WORKERS_LOG
  --private-key PRIVATE_KEY
  --crash-at {after_label_before_receipt,after_claim_before_receipt,after_push_before_receipt,after_pr_before_receipt,after_status_before_receipt,after_deliver_before_receipt}
                        受控故障注入：在该点对本进程 SIGKILL
```

### status

```sh
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=.sandcastle/deepagents-pilot /Volumes/AgentStorage/issue1742-deepagents-pilot-20261027/venv/bin/python -B -m pilot1742.executor status --help
```

```text
usage: pilot1742.executor status [-h] --run-id RUN_ID

options:
  -h, --help       show this help message and exit
  --run-id RUN_ID
```

### reconcile

```sh
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=.sandcastle/deepagents-pilot /Volumes/AgentStorage/issue1742-deepagents-pilot-20261027/venv/bin/python -B -m pilot1742.executor reconcile --help
```

```text
usage: pilot1742.executor reconcile [-h] --run-id RUN_ID --effect EFFECT
                                    --note NOTE

options:
  -h, --help       show this help message and exit
  --run-id RUN_ID
  --effect EFFECT  效果名，如 deliver-<head>、status-<head>、claim
  --note NOTE      核实依据，写进记录
```

### release

```sh
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=.sandcastle/deepagents-pilot /Volumes/AgentStorage/issue1742-deepagents-pilot-20261027/venv/bin/python -B -m pilot1742.executor release --help
```

```text
usage: pilot1742.executor release [-h] --ticket TICKET --run-id RUN_ID
                                  --reason REASON

options:
  -h, --help       show this help message and exit
  --ticket TICKET
  --run-id RUN_ID
  --reason REASON
```
