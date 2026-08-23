# RFC 草稿：authenticated remote RLM children（`prime-agent.remote-child/v1`）

> 提交路由（评审前提示）：Prime 的 `CONTRIBUTING.md` 规定 public proposal 先走 GitHub Discussions，Issues 是 maintainer 已接纳的工作队列；Feature request 应进 Feature requests 分类。查重时已存在同主题、同作者、仍打开的 [Discussion #1571](https://github.com/PrimeIntellect-ai/prime-agent/discussions/1571)。因此本稿定位为 **#1571 的 2026-08-23 修订候选**，用于更新证据基线并补齐评审要求；评审通过后应更新/回复 #1571，**不得另开重复 Discussion**。
>
> 证据基线：公开克隆 `PrimeIntellect-ai/prime-agent` `main @ e319a66d7351c75abe7f040d02d9a8d6e25028e9`（commit 时间 2026-08-21）。本文全部 file:line 均 pin 到该 commit；未使用本机安装版本，也不依赖任何私有仓库、私有 issue/blob/SHA、本机路径或凭据。

## 1. Use case 与完成定义

目标场景：一个 **已 detached、仍在 resident daemon worker 中运行的旧 parent session P**，和一个 **GitHub-hosted Actions job 中实际执行编码的 worker W**。W 必须成为“那个 P”的真 RLM child，而不是成为当前聊天、另一个 root，或一个只做状态映射的伪 child。

必须满足：

1. **只有 P 授权 W**：入场请求绑定 P 的持久 session 身份，不绑定“当前总控”或 ticket 字符串。
2. **W 的 `rlm_child_id`、`session_id`、`parent_session_id`、depth 由 daemon 在 admission 中分配**；W 不能自选或覆盖。
3. **durable admission barrier**：在收到持久化的 `ADMITTED` 之前，W 不得初始化模型、执行工具、写 repo/checkout。
4. P 的 `list_subagents()` 显示 `runtime="remote"` 及真实 remote state；P 可 `observe`、收发 message、`steer`/`follow_up`、`stop`、`delete`；同 ticket 合法 retry 可 `resume` 同一 child/session。
5. 错 ticket/repo/workflow/run/parent/family 的 worker 不能 attach 或发消息；parent 关闭、Action cancel、heartbeat 过期、daemon restart 都有显式状态，绝不静默重放模型/工具工作。
6. 不伪造 transcript/usage：remote 的真实 transcript、kernel checkpoint、usage 由 W 持有并上报，parent 侧只放合法 header 与 `remote_child_*` 生命周期/审计记录。

## 2. 当前 local-socket 信任边界：为什么不能 raw relay

- 当前 daemon 只在 Unix 上监听 per-user 目录内的 Unix domain socket（目录 `0700`、socket `0600`），Windows 为固定 named pipe：`daemon-socket.ts#L9-L43`、`#L216-L239`。public peer 一连上就被标记 `authenticated: true`，没有 challenge/token 握手：`daemon-supervisor.ts#L1053-L1065`。envelope 的 `clientId` 会被直接接受为连接身份：`daemon-supervisor.ts#L1349-L1382`。
- public local JSONL 的命令面是 daemon 级 authority：`list/create/attach/kill/prompt/steer/follow_up/send_message/abort/execute_bash/cancel_rlm_child/delete_rlm_subagent/shutdown` 等：`daemon-protocol.ts#L370-L658`。`send_message` 的 `agentOrigin` 也只在本机直连 socket-client 边界被信任：`daemon-supervisor.ts#L2021-L2068`。
- 源码自己把 public protocol 称为 **local daemon JSONL，不是最终 remote gateway protocol**：`daemon-protocol.ts#L44-L50`。private worker 虽有 token + supervisor generation/pid/socket fence，但文档明确它是同 OS user 内的 process coordination，不是 sandbox boundary：`daemon-worker-protocol.ts#L55-L64`、`daemon-mode.ts#L3452-L3493`、`docs/daemon.md#L105-L120`。
- 现有 command journal 与 reconnect resend 是“本地 client/supervisor 恢复”机制：重复 command 返回已记录结果、缺 result 报 uncertain 不重放、重连重发 stable envelope：`command-recovery-journal.ts#L48-L51`、`#L73-L88`、`daemon-client.ts#L282-L291`、`#L438-L459`。它们不含 remote worker lease/heartbeat/broker cursor/Action cancel。

结论：把 public daemon v7 原样 relay 给 Actions，等于把本机 daemon 的广域 authority 交给远端 job；必须新建 child-scoped 的独立 application protocol，不暴露 v7。

## 3. Extension / skill / custom provider 为什么不足

- 当前 child runtime 合同是 **具体本地对象**：`RlmSubagentRuntime` 只有一个 `session: AgentSession`，`SubagentRuntimeHost.createRlmSubagentRuntime()` 必须返回它：`rlm-runtime.ts#L214-L216`、`#L242-L253`。父侧 detached task 拿到 `childRuntime.session` 后直接 `subscribe` 并 `promptAndWait()`：`agent-session.ts#L10355-L10367`、`#L10451-L10457`。没有 external-runtime SPI。
- kernel host request 只查当前 session 的 `hostHandlers[data.type]`，未知 type 直接失败：`kernel/index.ts#L1491-L1494`。`AgentSession` 固定注册 `rlm.run/list/find_models/delete_subagent`：`agent-session.ts#L9059-L9066`。
- Extension API 能注册 tool/command/message renderer/provider：`extensions/types.ts#L992-L1041`、`#L1065`、`#L1114-L1166`，但没有注册 host handler、向已运行 daemon parent 绑定 subagent runtime host、写 RLM family ledger、或授予 family sender authority 的入口。
- Custom provider 只替换/代理当前本地 session 的模型 endpoint/stream：`docs/custom-provider.md#L1-L8`。模型循环、kernel、工具、session、child identity 仍在本地；Actions 里的实际写码进程不会因此成为 child。
- 单独启动 `prime-agent` 只发 `{type:"create", config, sessionPath, ...}`，无 parent-admission 字段：`main.ts#L1001-L1009`。无显式 parent 时 runtime metadata 默认 `kind:"top-level"`，header depth 走 root 逻辑：`agent-session-runtime.ts#L39-L51`、`#L80-L83`、`session-manager.ts#L1207-L1211`。worker 启动环境还显式删除继承的 `RLM_DEPTH`，官方 process test 锁定其 depth 为 0：`daemon-supervisor.ts#L2426-L2439`、`daemon-supervisor-process.test.ts#L310-L329`。结果只会是另一个 root/独立 session。

## 4. Durable admission barrier（本题核心差异）

当前 `rlm.run` 的所谓 admission 不是 remote 开工许可：

- `_startRlmChildRun()` 分配 `sub-*` 目录、写入父进程内存 `_activeRlmChildRuns`（`queued`）后，马上把 handle 返回给 Python：`agent-session.ts#L10240-L10269`、`#L10588-L10593`。
- 真正的 runtime 创建与 task run 在 **不 await 的 detached task** 中执行：`agent-session.ts#L10355-L10361`。官方测试明确锁住“handle 已返回、startup 之后才失败并进入 registry/error”的行为：`agent-session-recursion.test.ts#L1284-L1299`。
- daemon host 内部虽然把 ledger append 作为 admission 完成条件，失败会关闭 child：`daemon-mode.ts#L2690-L2713`，但该 barrier 发生在 detached task 内、handle 返回之后，对远端调用方不可见。

因此 `prime-agent.remote-child/v1` 的 `ADMITTED` 必须是 **持久化 barrier**，顺序如下：

1. 已授权 parent P 创建 **single-use invitation**（短 TTL、绑定 P + workload claims）；创建 invitation 不分配 child。
2. W 提交 `ADMIT`：`invitation_id` + 一次性 `nonce` + 幂等 `admission_request_id` + 可验证 workload claims（如 GitHub OIDC repo/workflow/ref/run_id/attempt）。admission 防重放不依赖尚不存在的 attempt/epoch。
3. daemon 校验 P 仍是 live/resumable parent、claims 匹配、invitation 未消费，然后**由 daemon 分配** `rlm_child_id/session_id`。
4. 一个 crash-recoverable、幂等的持久化事务完成：invitation 消费、session header、remote record、RLM ledger edge 全部 fsync。`RlmSpawnLedger` 保持 family topology 正本：`rlm-ledger.ts#L24-L39`、`#L56-L65`、`#L699-L721`；per-child display 文件仍只做 display/hydration：`rlm-subagent-display.ts#L5-L15`、`#L54-L70`。
5. 只有恢复逻辑能重现已提交 child 后，才分配 `attempt=1`、`connection_epoch=1`，mint child lease 并返回 `ADMITTED`。
6. remote launcher 把模型 init、工具、repo 写全部 gate 在 `ADMITTED` 之后。

相同 `admission_request_id` + 相同 claims 重试返回同一次已提交 admission；invitation 二次使用或 claims 变化 fail closed。任何写入失败都不得返回 `ADMITTED`，也不得留下幽灵 child。

## 5. `RlmChildRuntime` local/remote 抽象

不要把 remote worker 伪装成可本地 `promptAndWait()` 的 `AgentSession`。最小 seam：

```ts
interface RlmChildRuntime {
  readonly identity: RlmChildIdentity; // childId, sessionId, parentSessionId, depth, runtime: "local"|"remote"
  start(task: RlmChildTask): Promise<"admitted" | "started">;
  subscribe(listener: RlmChildEventListener): () => void;
  steer(cmd: RlmChildCommand): Promise<RlmChildCommandReceipt>;
  followUp(cmd: RlmChildCommand): Promise<RlmChildCommandReceipt>;
  stop(cmd: RlmChildStopCommand): Promise<RlmChildCommandReceipt>;
  resume(leaseProof: RlmChildResumeProof): Promise<RlmChildResumeReceipt>;
  close(reason: string): Promise<void>;
}

LocalRlmChildRuntime  // 包装现有 AgentSession，行为不变
RemoteRlmChildRuntime // parent daemon 内的控制代理，背后是 remote record + transport endpoint
```

父侧 `list/delete/cancel/observe/message` 只依赖抽象。`RemoteRlmChildRuntime` 是 parent 侧代理；真正的模型循环、kernel、工具与 checkout 在 Actions 的 W 中，W 的 `agent_message` host handler 接到 remote message controller，sender 仍由 daemon 从 lease/ledger 派生。usage/transcript 订阅等本地专有语义移入 `LocalRlmChildRuntime`，不伪造到 remote。

## 6. Remote record / state / ledger

remote record（daemon 持有，原子 temp+fsync+rename，与 display 文件同风格）至少包含：

```text
protocolVersion, childId, sessionId, parentSessionId, parentSessionPath,
parentActiveSessionId(last known), invitationJtiHash, workloadClaimsHash,
repo/workflow/ref/run_id/run_attempt, attempt, connectionEpoch, state,
leaseHash, workerEventAck, parentCommandAck,
checkpoint{seq, contentHash, verifiedAt}, lastHeartbeatAt, disconnectDeadline,
terminalReason, auditTimestamps[]
```

`RlmSpawnLedger` 仍是 parent/child edge、depth、name 的唯一 topology 正本；remote record 只补充 runtime/connection/attempt/lease 状态，不得重写 family 判据。`list_subagents()` 现有 wire registry 只有 `rlm_child_id/active_session_id/session_id/session_name/session_dir/status`：`rlm-runtime.ts#L24-L31`、`agent-session.ts#L9470-L9550`，需增加 optional `runtime`、`remote_state`、`attempt`、`last_heartbeat_at`，并保留现有 `running/completed/error` 兼容映射。

状态机（`INVITED` 只表示 invitation 已创建，尚未分配 child/写入 remote record）：

```text
INVITED --admit+fsync--> ADMITTED --worker start ack--> RUNNING
RUNNING --transport lost--> DISCONNECTED --valid attach/resume--> RUNNING
RUNNING --parent stop/Action cancel trap--> CANCELLING --worker ACK--> CANCELLED
RUNNING --complete--> COMPLETED
ADMITTED/RUNNING/DISCONNECTED --heartbeat expiry/fatal--> EXPIRED|ERROR|UNCERTAIN
terminal --explicit same-ticket retry--> ADMITTED(attempt+1, 同一 childId/sessionId)
```

## 7. Family 与操作语义

family 判据沿用现有 nuclear-family 谓词（parent / 同 parent sibling / direct child，cousin 与更远拒绝）：`agent-messages.ts#L217-L250`、`#L310-L328`；daemon observe/send 在 active/passive/hydration 路径均执行：`daemon-mode.ts#L5745-L5780`、`#L5790-L5852`、`#L3162-L3225`；官方测试锁定 cousin reject：`daemon-mode.test.ts#L2749-L2834`。但 **remote v1 默认 parent-only**：W 只能发给其精确 parent P；即使本地谓词允许 sibling，也因 Actions job 沙箱信任度低于同机 child 而默认拒绝。扩权由 maintainer 另行决策。v1 remote child 是 leaf，W 内 `rlm()` 显式失败，不产生无账孙代。

| 操作 | 发起方 | 语义 |
|---|---|---|
| `list` | parent | 只列 direct children：现有 filter 以 `parentActiveSessionId === current` + `rlmChildId` 合并：`agent-session.ts#L9474-L9550`。remote row 必须带 runtime/state/attempt/heartbeat。 |
| `observe` | parent | 对 remote child 只返回 bounded summary、checkpoint 新鲜度与合法 lifecycle preview；不读取任意路径，不伪装本地 transcript。现有 observe 走 family target + bounded preview：`daemon-mode.ts#L3209-L3238`。 |
| `message` | W→P | lease 不携带 `from`；daemon 由 lease 派生 sender，target 由 ledger 解析，payload 里的身份字段被忽略。沿用 16,384 chars / 每 target 20 pending / 3 次每秒等上限：`agent-messages.ts#L13-L16`。 |
| `steer` / `follow_up` | P→W | durable `{commandId,seq}`；W ack；重连重发未 ack 命令，重复 commandId 幂等。 |
| `stop` | P | 先封锁新 model/tool admission，W 终止自有进程、尽力 checkpoint 后 ACK；**只有 termination ACK 才置 `cancelled`**；deadline 无 ACK 则 revoke lease 并置 `uncertain`/`lease_expired` + audit，不声称外部副作用已回滚。 |
| `resume` / `attach` | W | 凭 child lease + 最后 ack cursor；匹配 parent/child/audience；新 `connectionEpoch` 立即 fence 旧连接；同 ticket retry 复用 child/session、attempt+1；过期后按规则 terminal 或需 parent 重新授权。 |
| `heartbeat` | W | 单调 event seq + activity/工具/stdout 摘要；lease 续期有上限，不得无限延长 invitation。 |
| `checkpoint` | W | 上传真实 transcript/kernel state 的 content hash + attempt + last event seq；daemon 校验后才推进 resumable cursor；无法确认的窗口标 `uncertain`，不自动重放。 |
| `complete` | W | 只接受当前 attempt/epoch；final ack + result/artifact refs 后原子 terminal；迟到旧 attempt/epoch 拒绝。 |
| `delete` | P | direct-parent only：先 revoke capability/fence transport，再写 ledger tombstone，保留 audit 与 identity；运行中先走 stop 语义；幂等。现有 direct-only 由 selector 解析与官方测试锁定：`agent-session.ts#L9561-L9570`、`#L9614-L9677`、`agent-session-recursion.test.ts#L3967`。 |

Python 侧 `list_subagents()/delete_subagent()` 仍走当前 session host bridge：`prime-agent-runtime/src/rlm/__init__.py#L217-L237`；remote 只是在 parent session 中新增 row/runtime，不新增 Python 身份参数。

## 8. Capability 与防护

1. **Invitation**：parent-minted、≥256-bit opaque random、daemon 只存 hash、分钟级 TTL、single-use、`aud=remote-child-admit`，绑定 parent session + repo/workflow/ref/environment/ticket + GitHub `run_id/run_attempt`。token 不进 argv/log/artifact/report。
2. **Workload proof**：优先 GitHub OIDC 短时 claims（repo/workflow/ref/run id/attempt）经 daemon 信任的 verifier 校验。verifier 在接口后，v1 只要求一个具体实现；broker assertion 不是 authority。
3. **Child lease**：`ADMITTED` 后 mint，绑定 child/session/parent/attempt/connection_epoch + worker proof-of-possession key；操作集仅 `attach/heartbeat/checkpoint/message(parent)/complete/cancel`。**不含** `list/create/attach arbitrary/prompt/bash/kill/shutdown`。
4. **Parent capability**：parent 侧控制由 daemon 当前 parent session authority 派生，只能控制自己的 direct remote child；ticket 映射不是授权。
5. **Replay fence**：invitation JTI single-use；admission 用 `nonce + admission_request_id` 幂等；post-admission 每 mutation 带 `{attempt, connectionEpoch, sequence, idempotencyKey}`；daemon 持久化 high-water marks；新 epoch fence 旧 epoch、新 attempt fence 旧 lease；重复 id 返回记录结果，旧帧拒绝或小窗口缓冲。
6. **Family isolation**：sender/target 全部由 capability + ledger 派生，忽略 remote payload 自报 `from/rlmChildId/sessionId/parentSessionId`；默认 W→P only。
7. **Audit / quotas**：按 invitation/child/parent/repo/connection 记 admission、denial、epoch、控制命令、terminal、quota violation；限制 frame 大小、message/heartbeat/event rate、buffer bytes、retries、并发 children。
8. **Cancellation / restart**：Action cancel trap 正常上报；SIGKILL 由 heartbeat expiry 收割并给 parent terminal notice。daemon restart 从 remote record 恢复 lease hash/cursors，未确认 side effect 不自动 replay。
9. **凭据分离**：remote-child capability 不是模型 provider credential。v1 **不假定**存在“GitHub OIDC → 短时 Prime/Codex/Claude 执行凭据”的 issuer；该发行器是独立的未解决前提，两个 token 不得合并。

**v1 默认值与安全下限（全部可配置；配置不得低于安全下限，测试锁定下限）**：

- Invitation TTL 默认 5 min，允许 1–15 min；只存 hash、单次消费。
- Child lease 默认 60 min；续期单次 ≤10 min、累计 ≤24 h；heartbeat 间隔默认 15 s、下限 5 s，grace = 3×interval（超时 revoke）。
- PoP：ADMIT 时 W 提交 Ed25519 公钥，daemon 把 fingerprint 写入 lease/record；每个 attach/heartbeat/complete/cancel 帧带签名；换 key 走重新 ADMIT。
- 帧上限默认 256 KiB、绝对上限 1 MiB；message 沿用现有 16,384 chars/每 target 20 pending/3 次每秒；heartbeat 事件率与帧大小双重限流；buffer/retries/并发 children 均设显式上限。
- audit 保留 ≥90 天（可加长），记录不含 invitation/lease 明文。

## 9. 分层：application protocol 与 broker

`prime-agent.remote-child/v1` 是 **application protocol**：admission、family message、lifecycle、capability、quota/audit 语义。它由 admission 显式协商版本，承载在可插拔 transport 上。

- 双方出站 broker（parent 与 W 都向 relay 发 TLS 连接）只是 **推荐的后续 transport**，不是协议依赖。Core MVP 全程跑 in-memory transport；RFC 不假定已有托管服务、网络入口、provider credential issuer 或本机开放端口。
- broker 最小信任：只按 channel/capability 转发，有配额/大小/审计，不持有本机 daemon token 或 provider 长期凭据；需要隐藏内容再加应用层会话加密并写入威胁模型。
- 禁止 raw daemon v7 relay；local public socket 仍保持 local-only trust。

## 10. 无网络 mock transport 原型矩阵

使用 daemon-backed parent + `InMemoryRemoteChildTransport`（bounded async queue、确定性故障注入、无端口/凭据）+ `MockRemoteWorker`，其 `startModel()` 断言已收到持久化 `ADMITTED`。

| 类别 | case | 必须断言 |
|---|---|---|
| Admission | 成功 / ledger、remote record、session header 任一写失败 / 响应丢失后同请求重试 | 只有全 durable 才返回 `ADMITTED`；失败零开工、零幽灵 child；重试返回同一 IDs，不二次分配。 |
| Invitation/claims | 过期、撤销、二次使用、错 audience/repo/workflow/ref/run/ticket/parent | fail closed，不分配 child、不泄露 parent 细节，audit 原因。 |
| Identity | payload 伪造 from/childId/sessionId/parentId | 忽略 payload 身份，由 lease+ledger 派生；审计拒绝。 |
| Family | parent/sibling/cousin/other root/grandchild selector | remote W 默认 parent-only；其余拒绝；W 内 `rlm()` 显式 unsupported。 |
| Visibility | parent list/observe/agents view | `runtime="remote"`、诚实 `remote_state`，不伪造本地 transcript/tool rows。 |
| Provenance | 伪造 usage/checkpoint、旧 attempt 的 completion/usage | usage/checkpoint 校验失败 → mark unverified 或 reject；旧 attempt 的 completion/usage 一律拒绝；parent 侧不写 provider-native usage。 |
| Round trip | W→P message；P→W steer/follow-up | 派生 sender、现有 size/rate/pending 上限、ack/重连可恢复、无重复投递。 |
| Disconnect | heartbeat 丢失、grace 内 attach、grace 后 attach | `disconnected` 可见；grace 内同 IDs + epoch+1；过期拒绝旧 lease。 |
| Replay/order | duplicate/out-of-order seq、old epoch、old attempt | dedupe 或拒绝；steer/stop/complete 不重复执行。 |
| Retry split-brain | 同 ticket 两个 worker 并发 retry | 只有一个 epoch 获胜；same child/session + attempt+1；旧 worker fenced。 |
| Parent control | graceful stop、无 ACK 超时、delete running child | 仅 ACK 置 `cancelled`；超时 revoke/`uncertain`+audit；delete 先撤权且幂等。 |
| Action cancel | cancel trap 正常、runner SIGKILL | 正常终态；异常由 TTL 收割且 parent 收 notice。 |
| Daemon restart | admit 后、command 未 ack、complete 前重启 | invitation 消费不回滚；cursors 恢复；不确定 side effect 不自动 replay。 |
| Broker fault | 重复/延迟/丢帧、断线、broker 冒充 command | epoch/seq/capability 挡住；本机 raw daemon 不可达。 |
| Quota | oversize frame、heartbeat/message flood | bounded memory；现有限额回归不退化。 |
| Local regression | 普通 local child spawn/list/message/observe/delete | 现有 tests 全过；local runtime 行为不变。 |

可直接扩展的现有测试锚点：post-handle startup failure `agent-session-recursion.test.ts#L1284-L1299`、direct-only delete `#L3967`、nuclear-family reject `daemon-mode.test.ts#L2749-L2834`、process-level daemon worker `daemon-supervisor-process.test.ts#L310-L329`。

## 11. Version / 向后兼容策略

- `remote-child/v1` 独立版本，admission 时协商；**不暴露、复用或隧穿 local daemon protocol v7**（当前 `DAEMON_PROTOCOL_VERSION=7`、`DAEMON_SCHEMA_REVISION=22`：`daemon-protocol.ts#L53-L70`）。
- host remote child 所需的 local 改动是 **additive**：new server/client capability、registry/UI optional fields、remote record；未升级 client、纯本地 daemon、无此特性的 worker 行为不变。
- 兼容新增可走 capability-gate 或 schema revision；不兼容 wire 变更才 bump protocol（现有规则：`daemon-protocol.ts#L44-L70`、`docs/daemon.md#L91`）。本设计不改 `create/send_message` 的现有本地认证语义，因此不要求 v8。
- Python `RLMSubagent` dataclass 当前字段固定为 6 项：`prime-agent-runtime/src/rlm/__init__.py#L43-L50`；新增 optional `runtime/remote_state/attempt/last_heartbeat_at` 属于 CLI/SDK/UI 真实改动，不是只改 server。

## 12. PR 切分（每段独立可 review/验收）

1. **Core MVP**：`RlmChildRuntime` seam + local adapter、remote record/state machine、registry schema、ledger 关联、invitation/workload-proof/lease 抽象与验证、no-network mock 矩阵。无网络、无 broker、无端口。
2. **Security / transport**：一个具体 workload-identity verifier（GitHub OIDC claims）和 SPI 后第一个真实 transport（双方出站 broker client）；fencing/idempotency/quotas/audit 的 wire 化。**自带验收对端**：in-repo 测试 relay/回环对端覆盖握手/重连/fence（无外部依赖）；若需真实网络路径，最小自托管 broker server 与 client 同段交付、以回环地址验收。PR4 的托管服务仍为可选，不构成 PR2 依赖。
3. **CLI / SDK / UI**：parent invite/revoke/status/retry；remote worker entrypoint/SDK；TUI agents view 显示 `connected/disconnected/attempt/last_heartbeat`，提供 steer/stop/delete。
4. **Broker（可选）**：托管 relay 或用户自托管实现；若 Prime 团队不提供，协议仍可运行。broker secret 不进 repo。

## 13. 查重结论与不重复提交

查询时间 2026-08-23，范围：Prime 公开 Issues（312）、Discussions（144）与 `main @ e319a66` 源码词面/符号。

- **Issue #739（ACP client-backed external child agents）**：本地 ACP command 作为 child backend；未定义跨主机 worker、detached old parent admission、durable start permit、family authority、remote lifecycle record。已随 discussion-first 政策关闭（state reason `not_planned`），不能承接本题。
- **Issue #1182（v0.8 five-stack integration tracker）**：正文是 Core/MCP/Release/Prompts/ACP-Evaluation 的集成 checklist，其中 “remote reviewed refs” 指已推送到远端的 Git refs，不是 remote agent runtime。把本安全协议挂进去会污染 release tracker，且该票明确把上游 prerequisites 分开；不承接。
- 相关 Discussions：#1488 是 extension 用本地 `DaemonClient` 创建 detached top-level session，不产生 RLM family edge；#1470 是本地 lineage 在 session replacement 时丢失；#1421 是本地浏览器 dashboard；#1529/#1506/#1503 等分别涉及 reply doctrine、per-subagent reasoning effort、worker heap，均不含 remote admission。
- **Discussion #1571** 已由同一作者以同标题提交并保持打开，是本提案的现有承载。本稿是它的修订候选（主要变化：证据基线从旧 commit 更新到 `main @ e319a66`，并继续满足 durable admission、分层、mock 矩阵、version、PR 切分），因此**不重复新开 Discussion**。
- 源码负证据：在 `e319a66` 的 `packages/coding-agent/src`、`packages/coding-agent/docs`、`packages/coding-agent/test`、`prime-agent-runtime/src` 中，没有 `RemoteRlmChildRuntime`、`remote-child`、child lease、outbound broker transport 或 remote ADMITTED record 的实现；最近的 schema 演进（revision 22）仍只涉及 ACP MCP replacement 与本地 daemon 能力：`daemon-protocol.ts#L56-L70`。

## 14. 开放决策

1. `RlmChildRuntime` seam 是否采用上述 factoring，还是另设 host-owned runtime adapter？
2. crash-recoverable admission transaction 的存储 owner 是谁（同时保持 `RlmSpawnLedger` topology 权威）？
3. Prime core 直接理解哪些 workload identity claims，哪些留在 verifier 接口后？
4. `observe` 对 remote child 展示哪些 bounded 字段，才不会变成误导性 mirror transcript？
5. v1 是否保持 remote child 为 parent-only leaf；sibling reach 与远程孙代何时开放？
6. 若 upstream 接受，是否把 #1571 转成 maintainer Issue，并邀请 Core MVP 实现？
