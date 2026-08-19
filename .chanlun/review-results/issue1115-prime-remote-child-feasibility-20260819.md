# #1115 调研报告：Prime remote child 的 core 可行性与最小协议

- 日期：2026-08-19
- 票据：[#1115](https://github.com/xy7365527-lang/NewChanlun/issues/1115)；硬验收语义来自 [#1114 的 B 裁定](https://github.com/xy7365527-lang/NewChanlun/issues/1114#issuecomment-5348493971)
- NewChanlun 基线：`main @ af10903417cb6ad84b066733636a784489a49ab2`
- 研究分支：`research/issue-1115-prime-remote-child`
- Prime 上游快照：`PrimeIntellect-ai/prime-agent @ f8f0036cc2da1a640aad990ae8dcb7c4820ce32e`（2026-08-19 查询到的 `main` HEAD）
- 本机版本：`prime-agent 0.7.3`；上游 release commit/tag：`61131b2d195ba7a67a4ce8ac60bb10cecae07b67`（`v0.7.3`）
- 边界：**只读源码/文档/测试研究**。未启动、停止、更新或覆盖本机 Prime；未连接本机 daemon socket；未开端口、tunnel 或生成凭据；未向 Prime 上游提交 issue/PR。
- 术语：以下明确区分 **【源码事实】**、**【据此判断】**、**【协议提案】**。提案不是在声称上游已有该能力。

## 0. 裁决摘要

1. **【据此判断】#1114 B 不能由现有 extension、Python skill 或 custom provider 做成，必须改 Prime core/daemon/RLM。** 现有 extension API 能注册工具、命令、消息、provider，却没有注册 kernel host handler、向现存父 `AgentSession` 注入 child runtime、写 daemon RLM ledger 或建立 family edge 的 API；custom provider 只替换模型 API/streaming，并不把 GitHub Actions 中实际写码的进程变成 child。当前 `SubagentRuntimeHost` 虽由 SDK 导出，但其返回值被硬约束为一个本地 `AgentSession`，随后 `runRlmChild` 直接订阅该 session、调用 `promptAndWait()`、读取 usage 并调用 `abort()`，不是 external-runtime SPI。【[ExtensionAPI](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/extensions/types.ts#L957-L1148)；[provider 能力边界](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/docs/custom-provider.md#L1-L8)；[`SubagentRuntimeHost` 类型](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/rlm-runtime.ts#L214-L254)；[child 对本地 `AgentSession` 的直接调用](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session.ts#L9545-L9600)】
2. **【源码事实】当前所谓 `rlm()` “admission handle”只表示父进程先把一个 queued run 放进内存 registry 并分配了 `sub-*` 目录；daemon child runtime 创建和 ledger 持久化在随后 detached task 中才发生。** 因此 handle 可以先返回，后续 startup/ledger 失败再把 child 标成 `error`。这与 #1114 B 的“远端工蜂未完成不可伪造且已持久化的 admission 不得启动模型工作”不是同一个强度；remote protocol 必须新增**持久化 admission barrier**，不能把当前 handle 当成远端开工许可。【[queued run、detached startup、先返回 handle](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session.ts#L9492-L9518) [续](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session.ts#L9586-L9599) [返回值](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session.ts#L9803-L9808)；[daemon ledger append 是后续强制屏障](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-mode.ts#L2677-L2701)；[官方测试：handle 已返回后 startup 失败进入 registry/error](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/test/agent-session-recursion.test.ts#L1158-L1185)】
3. **【源码事实】daemon 当前没有 TCP 或 WebSocket listener。** Unix 上是 per-user 目录里的 Unix domain socket（目录 `0700`、socket `0600`），Windows 是固定 named pipe；公开协议是 JSONL。RPC 是另一个进程的 stdin/stdout JSONL，不是 daemon 的网络端口。私有 supervisor↔worker 通道也在本地 socket 上，使用二进制 frame、每 worker token 与 supervisor generation fence。【[默认 socket 与权限](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-socket.ts#L9-L43) [目录归属/权限](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-socket.ts#L216-L239)；[supervisor `listen(path)`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-supervisor.ts#L646-L679)；[RPC 官方文档](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/docs/rpc.md#L1-L36)；[private worker frame/token](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-worker-protocol.ts#L22-L112)】
4. **【据此判断】绝不能把当前 public daemon JSONL 原样 relay 给 Actions。** public socket 一连上就被标为 `authenticated: true`；client ID 是 client 自报并被接纳；命令面含 create/attach/kill/prompt/steer/send_message/abort/bash/delete child 等。agent-origin sender 也只在“直连本机 socket”信任边界上被信任。其安全根是“能打开 owner-only 本机 socket”，不是远端凭据或 per-parent capability。【[连接即本地受信](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-supervisor.ts#L1019-L1057)；[clientId 直接接纳](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-supervisor.ts#L1206-L1237) [续](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-supervisor.ts#L1270-L1275)；[公开命令面](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-protocol.ts#L349-L503)；[agentOrigin 的本地信任注释与 family 校验](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-supervisor.ts#L1807-L1855)】
5. **【协议提案】最小可接受形态是新建 `prime-agent.remote-child/v1` capability protocol，而不是把 daemon protocol v7 暴露出去。** 父 daemon 与 GitHub worker 都只向一个 relay/broker 发起出站 TLS 连接；父先 mint 一次性、短 TTL、绑定 parent/ticket/workflow identity 的 invitation；daemon 原子消费 invitation、生成 `rlm_child_id/session_id/attempt` 并把 edge/remote record fsync 后才返回 `ADMITTED`；worker 此后只持有 child-scoped lease，不能 list/attach/kill 任意 session。parent→worker 支持 steer/follow-up/stop，worker→parent 支持 message/heartbeat/complete；所有 mutation 带单调序号和幂等 id，重连用 ack cursor + connection epoch fence。
6. **【协议提案】最小原型不应先接真实模型。** 先用 in-memory mock transport + mock remote worker 证明：目标旧 parent 的 `list_subagents()` 出现 `runtime=remote`；worker→parent `agent_message` 与 parent→worker steer 往返；断线恢复同一 child/session；invitation replay、错 parent、冒名、并发 retry、Action cancel、daemon restart 全部 fail closed。该原型通过前，#1110 不应解冻“外部 Claude/Prime 进程冒充 child”。

## 1. 证据基线：0.7.3 与最新 main

### 1.1 本机版本确实对应 `v0.7.3`

【本机实测】`/Users/silencehan/.local/lib/node_modules/prime-agent/package.json` 报 `0.7.3`。本机打包内的三份官方文档与 Python RLM shim 逐字节等于 `v0.7.3 @ 61131b2d...`：

| 文件 | SHA-256（本机 = tag） |
|---|---|
| `dist/prime-agent-runtime/src/rlm/__init__.py` | `9f9d76ff50fd403652597e63d6de2c35826bdde63ef0137e00048b008a046331` |
| `docs/daemon.md` | `90d1d1d328ae9d1e6f44bd12a3068b4737540c3f4830831b6954bfa339d6c1f2` |
| `docs/rlm-runtime.md` | `b1c81e49a1e44bcfe474754067b08980d05676daef7a58119f59ad0f7d36c46d` |
| `docs/rlm.md` | `7e88409a3f3ce0ef6b8f470ab7bc63906e4301930b63f52f5c127997819b1fce` |

上游 tag 的 package version 也为 `0.7.3`。【[`v0.7.3` package.json](https://github.com/PrimeIntellect-ai/prime-agent/blob/61131b2d195ba7a67a4ce8ac60bb10cecae07b67/packages/coding-agent/package.json#L1-L5)】

### 1.2 与最新 main 的相关差异

`v0.7.3..f8f0036c` 没有 remote/external child、remote registration、broker、WebSocket 或 TCP daemon transport。与本题有关的实质变化只有：

- `e85a67ac` 修复 daemon supervisor 从 child 环境启动时错误继承 `RLM_DEPTH`：最新 main 启 worker 前显式删除 `RLM_DEPTH`；官方 process test 证明即使 supervisor 环境为 1，新建 top-level 仍为 depth 0。【[main 修复点](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-supervisor.ts#L2185-L2205)；[回归测试](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/test/daemon-supervisor-process.test.ts#L310-L337)】
- `824a9ee3` 给 `rlm()` 增加 child `thinking` 选项；不改变 family/admission/transport。【[main Python API](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/prime-agent-runtime/src/rlm/__init__.py#L143-L153)】
- `8189b12d` 规范化 daemon socket path；不增加网络 listener 或远端认证。【[socket path 仍是 path/pipe](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-socket.ts#L38-L43)】

【重要订正】`docs/daemon.md` 仍写 “Public Daemon Protocol v4”，但 0.7.3 tag 与最新 main 源码常量都是 **protocol 7 / schema revision 16**；本报告以源码为准，不能引用旧标题断言 wire version。【[源码常量](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-protocol.ts#L43-L64)；[滞后的文档标题](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/docs/daemon.md#L76-L93)】

## 2. 当前 `rlm()` child：调用、身份与持久化

### 2.1 精确调用链

1. Python `await rlm(prompt)` → `run()` → `host_request("rlm.run", ...)`。shim 打开 Jupyter comm target `host.request`，等待 control channel 回复；spawn handle 字段只有 `rlm_child_id/name/session_dir/model`。【[Python shim](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/prime-agent-runtime/src/rlm/__init__.py#L24-L32) [host request](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/prime-agent-runtime/src/rlm/__init__.py#L84-L153)】
2. `KernelManager` 收到 comm，按当前 session 注册的 handler dispatch；未知 type 被拒。`AgentSession` 固定注册 `rlm.run/list_subagents/delete_subagent`，不是 extension 动态注册项。【[comm dispatch](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/kernel/index.ts#L1275-L1356)；[handler wiring](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session.ts#L8540-L8553)】
3. `_startRlmChildRun()` 校验 depth/name/model/thinking，`mkdir session-artifacts/<parent>/<sub-xxxxxxxx>`，以目录 basename 作为 `rlm_child_id`，先写入父内存 `_activeRlmChildRuns`（`queued`）并发 `rlm_child_update`。【[目录/ID](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session.ts#L8720-L8754)；[run admission](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session.ts#L9450-L9543)】
4. 函数随后启动一个不 await 的 detached task，再马上把 handle 回给 Python。task 内才调用 `_createRlmSubagentRuntime()`；daemon 模式走 `SubagentRuntimeHost`，inline 模式就地 new `SessionManager/Agent/AgentSession`。【[host/inline 分派](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session.ts#L8786-L8825)；[detached task](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session.ts#L9586-L9600)】
5. daemon host 创建 child `SessionManager`，header 写 `parentSession` 与 `rlmDepth=parent+1`；runtime metadata 写 live/persisted identity；`addRuntime` 成功后在 child publish 点写 display metadata 和 RLM spawn ledger。ledger append 必须 fsync 成功，否则关闭 child，令本次 startup 失败。【[daemon child creation/metadata](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-mode.ts#L2561-L2671)；[durable barrier](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-mode.ts#L2677-L2701)】
6. 本地 child 执行的是普通 `AgentSession.promptAndWait()`；child 显式 `agent_message` 回父。完成后 parent 保留 child session；error/cancel/delete 更新 registry/tombstone并清理 runtime。【[spawn custom message + prompt](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session.ts#L9664-L9705)；[retention/cancel/delete](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session.ts#L9209-L9295)】

### 2.2 三类 ID/edge 不能互相替代

| 字段 | 所有者/用途 | 是否稳定持久化 |
|---|---|---|
| `rlmChildId` / Python `rlm_child_id` | RLM edge/run ID，当前由 `sub-*` 目录 basename 生成 | ledger + display；parent list 的第一 selector |
| `activeSessionId` | daemon 当前 resident runtime 的路由 ID | supervisor/worker 生命周期内稳定；恢复时可恢复，但不是 transcript ID |
| `sessionId` | transcript header 的 session UUID | child JSONL 的持久身份 |
| `parentActiveSessionId` | live daemon runtime edge | runtime metadata；用于 resident child 与 parent 绑定 |
| `parentSessionId` / `parentSessionFile` | persisted family edge | runtime metadata、header/ledger；恢复与 family auth 使用 |
| `rlmParentNodeId` | RLM UI/context tree node | runtime/display metadata；不是授权 token |

字段定义见 [`AgentSessionRuntimeMetadata`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session-runtime.ts#L37-L51)、[`SessionHeader`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/session-manager.ts#L75-L90) 与 daemon child metadata 写点（[同上](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-mode.ts#L2623-L2634)）。官方测试还锁定 child header 必须包含父路径与派生 depth。【[test](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/test/agent-session-recursion.test.ts#L412-L421)】

### 2.3 持久化模型

- **拓扑正本**是 per-sessions-dir append-only `RlmSpawnLedger`：`spawn {childId,parent,child,depth,name}`、rename、delete；小写入 `O_APPEND`，append 后 `fsync`。它按 parent/child canonical session path 还原 family，不从 transcript body 猜关系。【[ledger contract/record shape](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/rlm-ledger.ts#L24-L95)；[append+fsync](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/rlm-ledger.ts#L699-L724)】
- **每 child display/hydration metadata**在 `<child session dir>/rlm-subagent.json`，含 childId/name/session file/max depth/prompt/spawn code/model/status；temp+fsync+rename 原子写。它明确不是 topology store。【[display file contract](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/rlm-subagent-display.ts#L5-L33) [write](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/rlm-subagent-display.ts#L54-L70)】
- **parent 内存 registry**是 `_activeRlmChildRuns` + `_rlmChildSessions`；`list_subagents()` 再与 daemon agent catalog 中 `parentActiveSessionId=current` 且 `rlmChildId` 相符的 child 合并。只列**直接 child**；root 不能用 delete API 删除 grandchild，官方 test 明确锁住这一点。【[list merge/filter](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session.ts#L8941-L9020)；[direct-only delete](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session.ts#L9023-L9041)；[test](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/test/agent-session-recursion.test.ts#L2886-L2904)】

## 3. `list/delete/agent_message/observe`：授权与 transport

### 3.1 Kernel API 是 session-scoped capability

`rlm.list_subagents()` 与 `delete_subagent()` 都经当前 session 的 Jupyter host bridge 调当前 `AgentSession` 方法；Python 不能传一个 parent session id 切换 authority。delete selector 可匹配本 parent 的 childId/activeSessionId/sessionId/unique name，但候选集始终来自该 parent 的 direct-child list。【[Python API](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/prime-agent-runtime/src/rlm/__init__.py#L181-L232)；[selector 与 parent 限定](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session.ts#L9023-L9041)】

`agent_message` Python skill 同样只是 host-bridge shim；sender 不由 Python payload提供。role/name 先由当前 session 的 roster 解析，host 才交给 daemon controller。广播也只遍历 family roster。【[Python skill](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/skills/agent-message/src/agent_message/__init__.py#L1-L67)；[role roster resolution](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-messages.ts#L525-L605)】

### 3.2 Family policy

family 是 persisted parent edge 上的 nuclear family：parent、同一 parent 的 siblings、direct children；grandchild/cousin不直接可达。关系用 `sessionId/sessionPath + rlmDepth` 计算，parent active id 只是 live routing 辅助。agent send 与 observe 在 hydration 前后都执行 family assertion；官方 daemon test 验证 child 可见 root/sibling/grandchild，但 cousin 的 observe/send 全拒绝。【[纯 family 判据](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-messages.ts#L216-L249) [授权](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-messages.ts#L302-L327)；[daemon observe/send enforcement](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-mode.ts#L5574-L5632)；[official test](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/test/daemon-mode.test.ts#L2653-L2738)】

注意一个现状：所有 depth-0 roots 被视为同一 sibling scope；这适合本机 session-to-session messaging，但**不能等价成远端 token 可以访问所有 roots**。remote child lease 必须绑定唯一 parent/child，不得继承 public socket 的广域本机信任。【[root sibling 行为测试](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/test/agent-session-bus.test.ts#L264-L316)】

### 3.3 消息实际走哪条路

- 同一 worker 内：`AgentSession` host handler → daemon controller → target `AgentSession.acceptAgentMessagePrompt/queueAgentMessagePrompt`；busy/streaming 时 queue，idle 时立即 delivery；有 16,384 chars、每 target 20 pending、token bucket 3/秒等上限。【[限额](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-messages.ts#L12-L24)；[accept/queue](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-mode.ts#L5798-L5863)】
- 跨 worker：source worker 通过本机 public supervisor socket 发 `send_message {fromActiveSessionId, agentOrigin:true}`；supervisor 重新算 source/target family，然后用已认证 private worker connection 发 `worker_deliver_message`。sender endpoint 在 supervisor 侧构造，不接受 child 自报身份。【[source worker path](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-mode.ts#L5750-L5795)；[supervisor enforcement/routing](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-supervisor.ts#L1807-L1878)】
- `agent_observe` 不读任意路径；daemon 先 resolve/hydrate family target，再只返回 bounded summary/recent previews（1–50 条、每条 80–2000 chars）。【[Python shim](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/skills/agent-observe/src/agent_observe/__init__.py#L1-L51)；[daemon implementation](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-mode.ts#L3149-L3225)】

## 4. daemon 的本地信任、恢复边界与“单独进程为何不是 child”

### 4.1 public socket 不是远端安全 API

源码的 public protocol 注释自己称其为 **local daemon JSONL**，并说它“不是最终 remote gateway protocol”；JSON-serializable 只意味着未来 gateway 可以 wrap/proxy，不意味着已经有远端认证。【[protocol 注释](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-protocol.ts#L43-L50)】

public supervisor connection没有 challenge/token 握手；OS socket ACL 是主认证。相比之下 private worker socket 初始为 unauthenticated，必须提交随机 worker token + supervisor generation/pid/socket identity，失败即断开；descriptor/token 文件权限为 owner-only。这套 private auth 是可借鉴的模式，但它只防旧 supervisor/错误本机进程，不是跨 Internet sandbox boundary。【[private auth fields](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-worker-protocol.ts#L41-L112)；[worker handshake/fence](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-mode.ts#L3434-L3484)；[官方 daemon 文档的 trust 限定](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/docs/daemon.md#L122-L137)】

### 4.2 当前恢复能复用什么、不能证明什么

可复用的 building blocks：

- public client envelopes 带 stable `clientId+commandId`；mutating command 在 dispatch 前进入 append-only journal，结果已知则 dedupe 返回，只有 receipt 没有 result 则报 uncertain、不自动重放。【[command journal](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/command-recovery-journal.ts#L37-L105)】
- client 支持 reconnect、保留 pending envelope、重新握手后 resend；attach/events 使用 `{generation,sequence}` cursor 与 snapshot resync。【[DaemonClient reconnect/request recovery](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-client.ts#L225-L318) [resend](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-client.ts#L431-L460)；[cursor types](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-protocol.ts#L68-L74)】
- supervisor restart能 adopt live local workers；worker crash recovery把不确定的 model/tool/bash/child work标 interrupted，**不会自动 replay**。这正说明 remote worker 也必须有显式 ack/attempt/fencing，不能把“daemon 会恢复”当 exactly-once 保证。【[daemon docs](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/docs/daemon.md#L31-L44)；[worker crash marker](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-catalog-process.ts#L200-L214)】

这些都是**本地 client/worker 恢复**；上游没有 remote worker lease、remote heartbeat、broker cursor 或 Action cancellation handler。

### 4.3 单独运行 `prime-agent` 的身份

普通 CLI 的 daemon `create` 只发送 config/session path/lifecycle/env，不发送 parent identity 或 child admission；`AgentSessionRuntime` 默认 metadata 是 `{kind:"top-level"}`，无 parent 时 `SessionManager` header depth 为 0。最新 main 还主动从新 worker 环境删除继承的 `RLM_DEPTH`，确保它是 root。【[CLI create payload](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/main.ts#L975-L1013)；[runtime default](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session-runtime.ts#L73-L85)；[header root depth](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/session-manager.ts#L695-L705)】

public `create` type虽有内部 `runtimeMetadata?` 字段，但伪填它仍不完成 RLM admission：不会经过父 `_activeRlmChildRuns`、不会由父 daemon host写 spawn ledger，且最新 worker depth仍被归零。最多制造不一致/表面 row，不会得到可 steer/resume/delete 的真 child，不能当 extension seam。【[wire create shape](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-protocol.ts#L349-L370)；[真正 daemon child 写 edge 的唯一 create path](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-mode.ts#L2561-L2701)】

0.7.3 尚缺 `delete workerEnvironment.RLM_DEPTH` 修复：若恰在 child 环境里另起 CLI，可能继承一个错误 depth；但它仍没有 parent IDs、parent run registry、ledger admission 或 sender authority，因此是**错误标深的独立 session**，不是受支持 child。

## 5. 为什么 extension / skill / provider 均不足

| 机制 | 当前能做 | 缺失的承重能力 | 结论 |
|---|---|---|---|
| Markdown/Python skill | 在当前 kernel 中调用 `host_request` 或网络/API | host 只 dispatch 当前 session 已注册 handler；skill不能注册 daemon runtime/ledger/family authority | 不足 |
| Extension | tool/command/event/message/provider、当前 session custom entry | 无 child admission API、无 `SubagentRuntimeHost` binding、无 daemon transport/auth扩展点 | 不足 |
| Custom provider | 把当前本地 `AgentSession` 的 LLM stream 发到代理/私有 endpoint | tool loop、session、child identity仍在本地；Actions 写码进程不是 child | 不足 |
| SDK embedder | 可 new 自己的 `AgentSession`，也能实现当前 `SubagentRuntimeHost` 类型 | host必须返回本地 `AgentSession`；不能挂到已运行旧 daemon parent，且无 remote control protocol | 原型参考，不满足旧 parent |
| raw `DaemonClient` | 本机 list/create/attach/prompt/kill 等 | 权限过大、无 remote auth；`create`不写 parent RLM admission | 禁止对外 |

关键证据是 kernel handler表由 `AgentSession` 构造并固定注册 RLM/a2a/observe（[源码](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session.ts#L8540-L8641)），extension surface没有 host-handler/subagent-host方法（[源码](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/extensions/types.ts#L957-L1148)）。

## 6. 【协议提案】最小 `prime-agent.remote-child/v1`

### 6.1 core 抽象的最小切口

不要让 remote worker伪装成一个可本地 `promptAndWait()` 的 `AgentSession`。把当前 child orchestration 中“identity/lifecycle/control”从具体 session拆成一个 core runtime 接口：

```text
RlmChildRuntime
  identity(): childId, sessionId, parentSessionId, runtime=local|remote, attempt
  start(task) -> admitted/started
  subscribe(activity|message|terminal)
  steer(commandId, text)
  followUp(commandId, text)
  stop(commandId, reason)
  close/delete()

LocalRlmChildRuntime  -> 包装现有 AgentSession（行为不变）
RemoteRlmChildRuntime -> 包装持久化 remote record + transport endpoint
```

父 `list/delete/cancel` 只依赖这个抽象。这里的 `RemoteRlmChildRuntime` 是**父 daemon 内的控制代理对象**，不是 bridge 工蜂；GitHub Actions 中实际写码的 Prime 进程须在收到 `ADMITTED` 后，用 admission capsule 给定的 `childId/sessionId/parent edge/depth` 创建真正的 remote `AgentSession`，其模型循环和工具执行都发生在 Actions。该 remote session 的 `agent_message` host handler 必须接到 `RemoteAgentMessageController`，经 broker 发 `message` frame；sender仍由父 daemon按 lease派生，所以模型调用的仍是现有 `agent_message` API而不是旁路 webhook。

usage/accounting 对 remote 采用 worker 上报且 daemon 验证的独立字段，不伪造 provider assistant usage。父侧 mirror JSONL 可以只有合法 header + `remote_child_*` custom lifecycle records；stdout/tool/heartbeat 放独立 bounded event journal/artifact，**不得编造普通 assistant/toolResult transcript**。真正的 remote transcript/kernel checkpoint由 worker append，并以 content hash + attempt/seq 做 authenticated checkpoint；同票 retry只从 daemon已确认的 checkpoint恢复。若最后 checkpoint与外部副作用之间存在窗口，状态必须标 `uncertain`，不得自动重放。

### 6.2 状态机与持久化

```text
INVITED --atomic admit+fsync--> ADMITTED --worker ack start--> RUNNING
RUNNING --transport lost--> DISCONNECTED --valid attach--> RUNNING
RUNNING --parent stop/action cancel--> CANCELLING --> CANCELLED
RUNNING --complete--> COMPLETED
ADMITTED/RUNNING/DISCONNECTED --expiry/fatal--> ERROR|EXPIRED
terminal --explicit same-ticket retry--> ADMITTED(attempt+1, same childId/sessionId)
```

remote record至少持久化：

```text
protocolVersion, childId, sessionId, parentSessionId, parentSessionPath,
parentActiveSessionId(last known), ticketBinding, repo/workflow/run claims,
attempt, state, capabilityHash, issuedAt/expiresAt,
connectionEpoch, workerEventAck, parentCommandAck,
checkpointRef/checkpointHash/checkpointSeq,
lastHeartbeatAt, disconnectDeadline, terminalReason, audit timestamps
```

拓扑仍写现有 RLM ledger，但应增加/关联 `runtime=remote` 的正本 metadata。`list_subagents()` schema增加可选 `runtime`, `remote_state`, `attempt`, `last_heartbeat_at`；现有 `status=running|completed|error` 保留兼容映射。当前 registry wire没有这些字段，故不能只靠 UI改字。【[现有 registry shape](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/rlm-runtime.ts#L15-L40)】

### 6.3 操作语义

| 操作 | 发起方 | 必须满足/结果 |
|---|---|---|
| `invite` | 已授权 parent/local integration | 绑定 exact parent session + ticket/repo/workflow claims；生成一次性 invitation ref；不创建 child |
| `admit` | remote worker | TLS 下提交 invitation + GitHub OIDC/workflow claims；原子 consume、防 replay；daemon生成 IDs，fsync session header/remote record/ledger 后返回 `ADMITTED`；**此响应前 worker不得调模型/改仓** |
| `attach` / `resume` | worker | child lease + attempt + last ack；匹配 parent/child/audience；connection epoch+1，旧连接立即 fenced；同票 retry复用 child/session，attempt+1 |
| `heartbeat` | worker | 单调 event seq、activity/工具/stdout摘要；续 lease只到有上限的 deadline，不得无限延长 invitation |
| `checkpoint` | worker | 上传/登记真实 transcript + kernel state的 content hash、attempt、last event seq；daemon只在完整校验后推进 resumable cursor |
| `message` | worker | token不携带 `from`；daemon由 lease派生 child sender，只允许 parent（最小权限）；remote `agent_message` skill走此操作；沿用 message size/rate/pending上限 |
| `steer` / `follow_up` | parent | durable command id + seq；worker ack；重连后未 ack command重送，重复 command幂等 |
| `stop` | parent | 软取消→deadline；worker ack并终止 model/tool/process；超时后 revoke lease并标 terminal，不宣称外部副作用回滚 |
| `cancel` | worker/Action trap | 正常 cancellation 上报；SIGKILL/Action abrupt cancel 由 heartbeat expiry收割；parent收到明确 terminal notice |
| `complete` | worker | 只允许当前 attempt/epoch；final ack + result/artifact refs；daemon原子 terminal；晚到旧 attempt被拒 |
| `delete` | parent | 先 revoke capability/停止 transport，再写 ledger tombstone；保留审计与 transcript identity，行为对齐现有 delete |

### 6.4 capability 与防护

最小安全设计：

1. **Invitation**：256-bit opaque random，daemon/broker只存 hash；TTL建议分钟级；单次原子 consume；绑定 `aud=remote-child-admit`、parent session、repo、workflow path、ref/environment、ticket、GitHub `run_id/run_attempt`。token不得出现在 CLI argv、日志、artifact、report。
2. **Action 身份**：优先 GitHub OIDC短时 identity去 broker换 invitation proof；workflow只要 `id-token: write`，不拿长期 Prime/Codex/Claude credential。provider临时凭据是另一个独立问题，不能复用 child capability。
3. **Child lease**：admit 后换发另一枚短 TTL、child-scoped lease，只含 `attach/heartbeat/message(parent)/complete/cancel`；绝不含 daemon `list/create/attach arbitrary/prompt/bash/shutdown`。
4. **Parent capability**：parent 侧 control由 daemon当前 parent session authority派生，只能控制自己的 direct remote child；ticket映射不是 authority。
5. **防 replay/双活**：invitation JTI single-use；每 mutation `{attempt,connectionEpoch,seq,commandId}`；daemon持久化 high-water marks；新 attach fence 旧 epoch；相同 commandId返回同结果，旧 attempt一律拒绝。
6. **family isolation**：sender/target从 capability和ledger派生，不接受 remote payload自报 `parentSessionId/from/rlmChildId`；默认只允许 worker→parent，siblings/children需父显式增 scope。
7. **broker 最小信任**：broker只按 channel/capability转发、有配额/大小限制/审计；不持有本机 daemon token或 provider长期凭据。若 broker终止 TLS并可见内容，必须在威胁模型明写；要隐藏内容再加应用层会话加密，不把它假装成现状。
8. **失联与取消**：heartbeat TTL + grace；parent stop和Action cancel均可终止；daemon restart后从 remote record恢复 lease hash/cursors，broker重连后同 child attach；无法确认的 side effect标 `uncertain`，不自动 replay。

【未解决的外部前提】remote-child capability只解决 Prime family/control身份，**不是模型 provider credential**。当前 Prime源码没有“GitHub OIDC → 短时 Prime/Codex/Claude执行凭据”的发行器；若 GitHub-hosted实际工蜂还需要长期 provider token，#1114 B 的凭据约束仍不通过。上游/执行服务必须另行提供短时、job-bound模型授权，或使用已具隔离身份的 runner；两种 token不得合并，也不得在本报告中假定已经存在。

## 7. Transport 候选：当前源码能支撑到哪一步

| 候选 | 当前已有事实 | 要补的 core | 结论 |
|---|---|---|---|
| 直接公开 daemon socket/TCP | 只有 UDS/named pipe；public peer即本机受信，命令面很宽 | TCP/TLS/auth/per-parent ACL/remote runtime全缺 | **禁止** |
| tunnel/SSH 转发 UDS | JSONL可字节转发 | 仍把本机广域 authority交给远端；本任务也禁止 tunnel | **不是方案** |
| 安全 local gateway + direct relay | protocol对象可 JSON serialize，源码明确为未来 gateway留 proxy可能；command id/cursor可复用 | gateway必须在 core拿 parent/remote-child authority，过滤成新 v1；不能只是extension转发 raw v7 | 可做，但远端到本机仍需可达入口；不是本轮首选 |
| 双方出站 broker（推荐原型） | 当前 client reconnect、idempotent envelope、generation cursor、private worker token/fence提供机制参考 | 新 broker transport、invitation/lease、remote runtime/persistence、OIDC exchange | **最符合“不开放本机端口”**；需 core+服务 |
| RPC stdin/stdout | 官方已有 LF-JSONL headless process control | 它拥有自己的 standalone AgentSession；无旧 parent/family admission | 仅可作为 remote worker内部 adapter，不是 family transport |
| Custom provider HTTP/WS | provider可把 LLM流送远端 | 工具/session/child仍本地 | 不满足“实际 Actions 写码进程是 child” |
| GitHub issue/check/artifact relay | NewChanlun/GitHub可持久化状态、日志、artifact | Prime源码没有该 transport；轮询延迟/权限/取消/双向 steer均需自建，且不可承载 daemon authority | 只做集成状态/取证，不做 child control plane |
| self-hosted runner 同机 UDS | 同一 OS user技术上可开 owner socket | raw authority过大；独立 CLI仍是 root；runner隔离不足 | 仅可在隔离测试 socket做实验，不可生产 |

【据此判断】最小安全路线不是给 GitHub-hosted job一个可入站访问本机的地址，而是**父 daemon 出站注册 invitation，Action 出站携 OIDC换短时 capability，broker配对**。这完全不要求本机开监听端口；但它确实需要一个上游认可的 remote gateway/broker组件，当前仓没有。

## 8. 最小 mock 原型与测试矩阵

### 8.1 原型通过标准

在 Prime repo 的 test harness 中增加 `InMemoryRemoteChildTransport`（双向 async queue，无真实端口/凭据）与 `MockRemoteWorker`：

1. 建一个 daemon-backed旧 parent，调用 core `invite(parent,ticket)`；
2. mock worker `admit`，测试在收到 `ADMITTED` 前若尝试 `startModel()`会被断言失败；
3. admission fsync完成后，parent 的 `await rlm.list_subagents()`含相同 `rlm_child_id/session_id`、`runtime="remote"`、`remote_state="connected"`；
4. worker `message("result")` 由 daemon派生 sender，进入 parent；parent steer 到 worker，worker ack；
5. 断开后 list显示 `disconnected`，合法 attach恢复同一 IDs；complete后显示 completed；delete写 tombstone并撤 capability。

### 8.2 必测矩阵

| 类别 | case | 必须断言 |
|---|---|---|
| Admission | 正常、ledger/display/remote record任一写失败 | 只有全 durable才返回 `ADMITTED`；失败零开工、零幽灵 child |
| Invitation | 过期、二次使用、篡改 claims、错 repo/workflow/ticket | fail closed；不分配/不暴露 parent IDs |
| Parent binding | A 的 token带 B parent、parent已关闭/替换 | 拒绝；绝不回流“当前总控” |
| Identity spoof | payload伪造 from/childId/sessionId/parentId | 忽略payload身份，由 capability派生；审计记录拒绝 |
| Visibility | parent list、observe、agents view | remote runtime明确；不伪装 local transcript/tool rows |
| Round trip | worker→parent message；parent steer/follow-up | role正确、size/rate/pending限制生效、ack可恢复 |
| Isolation | sibling/cousin/other root selector | 默认 scope拒绝；wrong family不可 observe/message/control |
| Disconnect | heartbeat丢失、grace内attach、grace后attach | 同 IDs恢复；过期后 terminal/新 attempt规则明确 |
| Replay/order | duplicate/out-of-order seq、old epoch、old attempt | dedupe或拒绝；不重复执行 steer/stop/complete |
| Retry | 同票 retry并发两个 worker | 只有一个 epoch获胜；same child/session + attempt+1；旧worker fenced |
| Parent control | soft stop、无 ack超时、delete running child | worker收到 stop；超时 revoke；delete幂等且先撤权 |
| Action cancel | trap正常 cancel、runner被 SIGKILL | 正常终态；异常靠 TTL收割且 parent获 notice |
| Daemon restart | admit后/command未ack/complete前重启 | invitation consume不回滚；cursors恢复；不确定 side effect不自动 replay |
| Broker故障 | 重复/延迟/丢帧、断开、broker冒充 command | seq/epoch/capability挡住；本机 raw daemon不可达 |
| Quota | oversized frame、heartbeat flood、message flood | bounded memory；现有限额回归不退化 |
| Local regression | 普通 local child spawn/list/message/delete | 现有 tests全过，local runtime行为不变 |

现有可直接扩展的官方测试面：

- `agent-session-recursion.test.ts` 已覆盖 handle后 startup failure、cancel notice、direct-only delete；remote abstraction必须不破坏这些锁。【[startup/cancel test](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/test/agent-session-recursion.test.ts#L1158-L1219)】
- `agent-session-bus.test.ts` / `daemon-mode.test.ts` 已覆盖 family policy与 cousin rejection；remote identity必须复用同一语义，而不是另写宽松判据。【[pure policy test](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/test/agent-session-bus.test.ts#L264-L316)；[daemon test](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/test/daemon-mode.test.ts#L2653-L2738)】
- `daemon-supervisor-process.test.ts` 已是进程级 daemon/worker/family验证面；remote端到端另增 mock broker process，但第一版仍可全内存、不开端口。【[现有 process test 示例](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/test/daemon-supervisor-process.test.ts#L310-L337)】

## 9. 上游贡献面与版本策略

### 9.1 建议先开 upstream design/RFC issue

不是“加一个 extension hook”的小 feature。issue 应明确：

- use case：detached old parent + GitHub-hosted actual worker；
- current trust boundary：public local socket不可暴露；
- durable admission barrier 与 child/runtime抽象；
- threat model（token theft/replay/wrong family/broker compromise/runner cancellation/daemon restart）；
- transport分层：remote-child application protocol 与 broker implementation分离；
- 不伪造 transcript/usage；
- mock matrix和向后兼容。

### 9.2 PR 切分

1. **Core state PR**：`RlmChildRuntime` local adapter、remote record/state machine、registry schema、ledger关联、mock transport/tests；不连外网。
2. **Security/transport PR**：invitation/lease、OIDC claims interface、outbound broker transport、fencing/idempotency/quotas/audit。
3. **CLI/SDK/UI PR**：parent invite/revoke/status/retry；remote worker entrypoint/SDK；TUI agents view显示 remote connected/disconnected/attempt/heartbeat，提供 steer/stop。
4. **Broker/deployment PR 或独立官方组件**：若 Prime 团队接受托管 relay，再落服务；否则协议允许用户自托管。不得把 broker secret塞进 repo。

### 9.3 protocol version

- 当前 local public wire 是 protocol 7/schema 16；源码规则/文档说明 compatible addition可 capability-gate/升 schema，不兼容 wire change才 bump protocol。【[capability/schema fields](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-protocol.ts#L52-L103)；[官方版本规则](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/docs/daemon.md#L88-L93)】
- 推荐新增独立 `prime-agent.remote-child/v1`，不要复用/暴露 local v7。local daemon若只加 optional registry/UI fields和新 capability，可保持 protocol 7、升 schema revision；如果改变现有 `create/send_message` 的认证/语义，则必须 bump public protocol，但更安全的做法是**不改写它们**。
- Python runtime `RLMSubagent` dataclass、AgentConnection snapshot、daemon session summary、agents-view row都要增 optional remote字段；这是 CLI/SDK/UI真实改动，不是只改 server。

## 10. NewChanlun 集成边界

Prime core原型通过前，NewChanlun只应保留设计/映射，不提交“伪 child”实现。上游能力落地后，本仓最小职责是：

1. ticket创建时把**发起它的旧 parent session**绑定为 immutable mapping；跨票新 child，同票 retry复用 child/session并增加 attempt；
2. workflow仅申请最小 `id-token: write` 与 repo内容/PR所需权限，向 broker换短时 child capability；不持有 daemon token或长期 Prime/Codex/Claude credential；
3. 保存 `ticket ↔ parent session ↔ rlm_child_id ↔ session_id ↔ run_id/attempt` 的非秘密审计映射；
4. Sandcastle启动实际模型前等待 `ADMITTED`；Action cancel trap调用 cancel，finally上传正常 artifact；
5. smoke只验证 list可见、双向消息、取消/恢复/family隔离；Prime协议/安全逻辑不复制进 NewChanlun。

## 11. 最终判断

**Go（研究结论）**：Prime现有 core有足够好的本地构件——持久 family ledger、daemon worker分层、private token/generation fence、command journal、reconnect cursor、parent-scoped list/delete、family-authenticated message/observe——所以 remote child不是从零设计。

**No-Go（当前实现）**：0.7.3 与最新 main都没有 external runtime/admission或远端安全 transport；extension/skill/provider不能把 Actions实际工蜂变成旧 parent的真 child；raw socket relay会把本机 daemon广域 authority交给远端。

**下一道闸**：先在 Prime 上游达成 remote-child core/RFC，再做无网络 mock；只有 mock矩阵过关，才选择/实现 outbound broker并解冻 #1110。当前最小、符合 #1114 B 且不开放本机端口的候选是**双方出站 broker + parent-minted one-time invitation + child-scoped lease**，不是 GitHub comment relay、self-hosted独立 CLI或 extension shadow registry。
