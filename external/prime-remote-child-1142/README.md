# prime-agent.remote-child/v1 Core MVP 本地原型（#1142）

- 票据：[#1142](https://github.com/xy7365527-lang/NewChanlun/issues/1142)
- 上游 base：`PrimeIntellect-ai/prime-agent @ af0b8e00b9f704e834787fd321065ca78281f2aa`（tag `v0.7.4`）
- 本地分支：`prototype/remote-child-core-1142`
- 本地 commit：`2e0ec1f`（完整可复现，见 `prime-agent-remote-child.patch`，`git am` 到 v0.7.4 即可复现）
- 边界：**未 push 上游、未创建 Prime Issue/PR、未修改本机 installed prime-agent、未开监听端口、未读写任何模型 provider secret**。

## 这是什么

在 Prime Agent 干净 clone（v0.7.4）中实现最小 `prime-agent.remote-child/v1` 协议，使一个 mock remote worker 经**原旧父会话** durable admission 成为真实 child。本原型同时覆盖了被评审打回的 Phase 1（独立 core：durable admission / restart / STOP / 命令重投递 / Ed25519 possession proof / journal fail-closed / 配额 / revoke / tombstone / attempt fencing）与 Phase 2-A（capability-gated daemon 纵切：旧父 session 的 `list_subagents` / worker→parent message / family isolation / restart）。steer/stop/delete 的 daemon 命令面（Phase 2-B）不在本切片内。

## 分层与版本

- `prime-agent.remote-child/v1` 是**应用协议**（admission、family messaging、lifecycle、capability 语义），独立版本，**不暴露、不复用、不转发** local daemon protocol v7，也不假设 inbound tunnel 或 broker。
- transport 走 `RemoteChildTransport` SPI；Core MVP 用 `InMemoryRemoteChildTransport`（同步、有界、确定性故障注入：drop / duplicate / disconnect，无端口、无凭据）。生产 transport（如双方出站 broker）是部署决策，不是 core 依赖。
- capability identity（invitation hash / lease token / worker Ed25519 possession proof）与 model provider 凭据**完全分离**；本模块不读、不写、不要求任何 provider secret。

## 状态机

```
invited --(原子 admit + fsync)--> admitted --(worker start_ack)--> running
running --(lease 过期)--> disconnected --(grace 内 attach)--> running
running --(parent stop)--> cancelling --(worker ACK)--> cancelled
running --(worker complete)--> completed
running --(worker fail: provider terminal error)--> failed
cancelling --(stop deadline 超时且 lease 有效)--> uncertain
cancelling --(stop deadline 超时且 lease 已过期)--> lease_expired
disconnected --(grace 超时)--> lease_expired
```

关键不变量：**STOP 无 ACK 只能进入 `uncertain` / `lease_expired`，禁止伪造 `cancelled`**（`collectExpiry()` 单点实现，测试锁定）。

## 持久 schema（journal.jsonl，append-only + 每条 fsync）

per-parent 目录 `<storeDir>/parent-<sha256(parentSessionId)[:24]>/journal.jsonl`，记录形状（JSONL，`v:1`）：

| op | 字段 | 说明 |
|---|---|---|
| `meta` | `sessionsDir` | 首条元记录 |
| `invitation` | `record{invitationHash, state, issuedAt, expiresAt, parentSessionId, parentSessionFile, depth, name?, workload, consumedBy?, consumedAt?}` | **只存 hash，不存明文 invitation** |
| `child` | `record{protocolVersion, childId, sessionId, parentSessionId, parentSessionFile, depth, name, workload, state, attempt, connectionEpoch, workerPublicKeyHash, issuedAt, admittedAt, startedAt?, lastHeartbeatAt?, lastEventSeq, admissionRequestId, admissionInvitationHash, parentCommandHighWater, acknowledgedCommandSeq, disconnectDeadlineAt?, terminalAt?, terminalReason?, checkpoint*}` | 每次状态变更 last-writer-wins 追加 |
| `command` | `childId, command{commandId, commandSeq, kind, text, reason?, issuedAt, deadlineAt?, childIdForRouting?}` | parent→worker 命令，幂等键 = commandSeq |
| `delete` | `childId, reason` | tombstone |

replay 是 fail-closed 的：未知 op、`v≠1`、缺失必填字段、attempt/epoch 非法都会抛错而不是静默丢弃。admission 的原子性 = 先追加 `invitation(state=consumed)` 再追加 `child(state=admitted)`，两者都 fsync 后**回读 replay 验证 child 可复现**，才 mint lease 并返回 `ADMITTED`；回读失败不返回、不签发开工许可。

## 已实现语义（对照 issue 七条 Outcome）

1. **durable admission**：admission 请求在父会话/daemon 侧落盘；ACK（`ADMITTED`）前 worker 不得进入 running（`MockRemoteWorker.startModel()` 在未收到 committed ADMITTED 时抛错）；daemon 重启后 pending/accepted 状态可恢复且幂等（同 `admissionRequestId` + 同 claims + 同 invitation 返回同一 childId/sessionId，绝不二次分配）。
2. **family 默认 parent↔child**：remote worker 是 leaf，只能 message 其唯一 parent；sender 由 lease + ledger 派生，payload 里的身份字段一律忽略；两个不同 parent 的 host 互不可见对方 child（测试锁定）。
3. **worker 以自身身份回旧父**：`onWorkerMessage` 收到 host 派生的 sender（childId/sessionId/parentSessionId），daemon 侧接 `acceptAgentMessagePrompt`；registry 测试证明消息回到**发起邀请的旧父 session**，不汇总到另一个控制会话。
4. **invitation one-time、lease、epoch/seq 防重放**：单次消费（hash 存储）、TTL、audience/workload 绑定；重复 admission 幂等、乱序 sequence 拒绝、旧 epoch/attempt 一律 fence。
5. **STOP 无 ACK 只能 uncertain/lease_expired**：见状态机与测试。
6. **capability identity 与 provider/model secret 分离**：Core 测试只生成 Ed25519 keypair，不触碰任何 Claude/DeepSeek/Codex 凭据。
7. **transport seam 与 local daemon v7/UDS 分层**：不暴露 raw UDS、不假设 inbound tunnel；MVP 走 in-memory transport，broker 不是 core 依赖。

## 兼容与退场说明

- **additive-only**：本地改动全部是能力门控的可选新增——`RlmSubagentRegistryEntry` 增加可选 `runtime/remote_state/attempt/connection_epoch/last_heartbeat_at` 字段；`AgentSession` 增加 `setRemoteChildRegistryProvider()`，未安装 provider 时 `list_subagents()` 行为逐字节不变（测试锁定）。
- **local daemon protocol 7 / schema 16 不变**：本模块不 touch daemon-protocol、daemon-socket、supervisor。
- **退场条件**：本原型是 provisional API，不是上游承诺。`#1126` 外部 maintainer 回应后按回应适配；若上游接受 `RlmChildRuntime` 因式分解，`RemoteChildAdmissionHost` 可整体搬入该 seam，`RlmRemoteChildRegistryProvider` 与 `setRemoteChildRegistryProvider()` 作为过渡桥可删除。该退场路径已在代码注释中标注。

## 对 #1126 RFC 的差异清单

| RFC 点 | 本原型 | 差异/理由 |
|---|---|---|
| `RlmChildRuntime` seam（local `AgentSession` 适配器 + remote runtime） | 采用更小的 `RlmRemoteChildRegistryProvider` + `setRemoteChildRegistryProvider()` seam | 避免对 `SubagentRuntimeHost`/`_startRlmChildRun` 大改的回归风险；remote runtime 仍由 `RemoteChildAdmissionHost` 承载。**这是本原型对 RFC 的主要偏差**，留待 maintainer 裁定 seam 归属。 |
| admission 六步（single-use invitation + nonce + admission_request_id + 原子 fsync + 回读 + lease） | 已实现 | 一致 |
| 三 capability（invitation / workload proof / child lease） | invitation + lease 已实现；workload proof 用**精确 claims 匹配** + worker Ed25519 possession proof，**未接 GitHub OIDC verifier**（RFC Stage 2） | 一致（verifier 在接口后） |
| family parent-only、leaf、`rlm()` 内失败 | parent-only + leaf 语义在 host 层实现；worker 内 `rlm()` 报 unsupported 属 CLI/SDK 层（Stage 3） | 待 Stage 3 |
| STOP/取消/provenance 不伪造 | 已实现（uncertain/lease_expired；remote transcript 不写伪 assistant/tool 行） | 一致 |
| 同票 retry attempt+1（同 child/session） | record 已带 `attempt` 与 `admissionRequestId` 幂等，但**attempt+1 显式入口未做**（Phase 2-B） | 待 Phase 2-B |
| 真实 transport / broker | 仅 in-memory（RFC 明示 Core MVP 无 broker 可验收） | 一致 |
| 配额 | frame 大小、message 长度、pending 父命令上限、单 parent child 上限已实现；rate/heartbeat 洪泛未做 | 部分（留 Stage 2 加固） |

## 复现步骤

```bash
git clone --depth 1 --branch v0.7.4 https://github.com/PrimeIntellect-ai/prime-agent.git pa
cd pa && git am ../external/prime-remote-child-1142/prime-agent-remote-child.patch
npm ci
# 类型 + lint（完整 npm run check）
npm run check
# remote-child 测试（19/19）
cd packages/coding-agent && npx vitest --run test/remote-child-core.test.ts test/remote-child-registry.test.ts
# 回归：local RLM 不回归（recursion 需清继承的 RLM 环境变量，见下）
env -u RLM_DEPTH -u RLM_MAX_DEPTH npx vitest --run test/agent-session-recursion.test.ts test/agent-session-bus.test.ts
```

## 测试证据

- `remote-child-core.test.ts`：**16/16 PASS**（admission barrier、幂等 admission、duplicate invitation、过期/撤销/claims 不匹配、nonce replay/伪造 possession proof、message 派生 sender、steer/follow-up、STOP ACK→cancelled、STOP 无 ACK→uncertain、running→disconnected→lease_expired、daemon restart 恢复 + stale 帧拒绝不重投、family 隔离、possession key 不匹配、tombstone delete + 幂等、checkpoint + provider error→failed、pending 命令上限）。
- `remote-child-registry.test.ts`：**3/3 PASS**（旧父 session `list_subagents` 出现 runtime=remote 且 sibling 父不可见；无 provider 时 list 行为不变；worker→parent message 回到发起邀请的父、不进 sibling 父）。
- `agent-session-recursion.test.ts`：**98/98 PASS**（需 `env -u RLM_DEPTH -u RLM_MAX_DEPTH`，失败源于沙盒继承的 Prime 运行时环境变量污染，非本改动；与 Phase 1 评审记录同因）。
- `agent-session-bus.test.ts`：**14/14 PASS**。
- `npm run check`（Biome + tsgo + installer + browser-smoke）：**PASS**。
