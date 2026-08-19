# RFC draft: authenticated remote RLM children (`prime-agent.remote-child/v1`)

> Draft status: prepared for maintainer review; not yet submitted to Prime Agent. Prime's contribution policy routes public proposals to Discussions, with Issues as the maintainer work queue ([`CONTRIBUTING.md:3-23`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/CONTRIBUTING.md#L3-L23)). Unless a maintainer requests an Issue, post this as a Feature request Discussion after review.
>
> Source baseline: `PrimeIntellect-ai/prime-agent@f8f0036cc2da1a640aad990ae8dcb7c4820ce32e` (`main` head on 2026-08-20). Every source link below is pinned to that commit; this draft is self-contained and depends on no other repository.

## Problem

Prime Agent runs a child `AgentSession` under a resident parent, keeps the family edge, and routes family messages — but only for a child process on the same machine. It cannot admit an agent running on another host as that child.

The target case: a detached, still-running parent session P on a user's workstation, and a coding worker W in a GitHub-hosted Actions job. W must become a real child of that exact P before it starts a model or touches the checkout, and P must then list, observe, message, steer, stop, and delete W under the same family rules as a local RLM child.

Required semantics:

- P — not whichever chat is currently open — authorizes W.
- W receives immutable `rlm_child_id`, `session_id`, `parent_session_id`, and depth from admission; it cannot choose or override them.
- W does not call a model, run a tool, or modify the checkout before it receives a durable `ADMITTED`.
- P's `list_subagents()` shows W as `runtime="remote"` with an honest state; W's `agent_message` can reach P; P can steer, stop, and delete W.
- A worker from another ticket, repo, workflow, run, parent, or family cannot attach or send as W.
- Parent closure, heartbeat expiry, cancellation, and daemon restart have explicit states; none silently replay model or tool work.

## Current state (the gap)

**The runtime contract is local.** `RlmSubagentRuntime` holds a concrete `AgentSession`, and `SubagentRuntimeHost.createRlmSubagentRuntime()` must return one ([`rlm-runtime.ts:214-254`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/rlm-runtime.ts#L214-L254)). The extension API adds tools, commands, events, and providers, but cannot register a kernel host handler, bind a subagent host into a running daemon parent, write the RLM family ledger, or grant family sender authority ([`extensions/types.ts:957-1148`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/extensions/types.ts#L957-L1148)).

**A standalone start is not a child.** The daemon `create` request has no parent-admission fields; a runtime without explicit parent metadata is top-level ([`main.ts:975-1013`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/main.ts#L975-L1013), [`agent-session-runtime.ts:73-85`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session-runtime.ts#L73-L85)). Starting `prime-agent` separately yields another root session, even with a ticket mapping beside it.

**The current handle is not a start barrier.** `_startRlmChildRun()` allocates the `sub-*` directory and an in-memory run and returns the handle before ledger persistence ([`agent-session.ts:9492-9518`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-session.ts#L9492-L9518)); a caller can already hold a handle when startup later fails ([`agent-session-recursion.test.ts:1158-1185`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/test/agent-session-recursion.test.ts#L1158-L1185)). That is acceptable for local children, but a remote launcher needs `ADMITTED` to be a durable start permit and nothing earlier.

**The daemon socket is a local trust boundary.** It listens on an owner-only Unix socket / named pipe; a connection is marked authenticated because filesystem access *is* the authentication, and the command set includes create/attach/kill/prompt/steer/bash/delete ([`daemon-socket.ts:9-43`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-socket.ts#L9-L43), [`daemon-supervisor.ts:1019-1057`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-supervisor.ts#L1019-L1057)). The source itself calls this the local daemon protocol, not the final remote gateway ([`daemon-protocol.ts:43-50`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/daemon-protocol.ts#L43-L50)). Tunnelling it would hand that broad authority to an Actions job, so a remote path must expose only child-scoped operations and must not forward raw daemon frames.

## Prior art

The closest existing item is [Issue #739, "ACP client-backed external child agents"](https://github.com/PrimeIntellect-ai/prime-agent/issues/739): it launches a configured ACP command as a child backend. It does **not** specify a worker on another host, admission into a detached parent, a durable start permit, family authorization, or a remote lifecycle record, and it was closed when public intake moved to Discussions. A local ACP backend could later implement the child-runtime seam below, but it does not own remote admission.

Other near items keep execution local: [Discussion #1488](https://github.com/PrimeIntellect-ai/prime-agent/discussions/1488) (plugin-initiated detached *top-level* sessions via local `create`/`prompt`), [Discussion #1470](https://github.com/PrimeIntellect-ai/prime-agent/discussions/1470) (lineage survival across local session replacement), [Discussion #1421](https://github.com/PrimeIntellect-ai/prime-agent/discussions/1421) (loopback browser dashboard for local sessions). [Issue #1182](https://github.com/PrimeIntellect-ai/prime-agent/issues/1182) (v0.8 integration tracker) uses "remote" to mean remotely-available Git refs, not remote agent processes; this proposal should not be attached there. No existing issue, discussion, or source symbol defines remote admission into an exact old parent.

## MVP semantics

### One seam: local vs remote child runtime

Split child control from the concrete local `AgentSession`:

```ts
interface RlmChildRuntime {
  // identity: childId, sessionId, parentSessionId, depth, runtime ("local" | "remote")
  readonly identity: RlmChildIdentity;
  start(task: RlmChildTask): Promise<void>;
  subscribe(listener: RlmChildEventListener): () => void;
  steer(cmd: RlmChildCommand): Promise<RlmChildCommandReceipt>;
  followUp(cmd: RlmChildCommand): Promise<RlmChildCommandReceipt>;
  stop(cmd: RlmChildStopCommand): Promise<RlmChildCommandReceipt>;
  close(reason: string): Promise<void>;
}
```

`LocalRlmChildRuntime` wraps today's `AgentSession` with no behavior change. `RemoteRlmChildRuntime` is a parent-daemon control object backed by durable remote state; the real session, kernel, model loop, tool calls, and checkout live in the remote job. Parent orchestration depends only on `RlmChildRuntime`; direct transcript subscription and provider-usage reads move behind the local adapter. Attempt and epoch are **not** part of identity — they do not exist until a child does (see admission).

### Admission is the only start permit

Admission must be crash-recoverable and idempotent, and the ADMIT request carries no attempt or epoch:

1. Code authorized as parent P creates a single-use invitation with a short TTL and exact workload constraints. Creating an invitation allocates no child.
2. W presents `invitation_id`, a single-use `nonce`, a client-generated `admission_request_id`, supported protocol versions, and verifiable workload claims. Admission anti-replay rests on single-use invitation consumption + nonce + idempotent request id — not on attempt/epoch, which do not yet exist.
3. The daemon checks that P is still the same live/resumable parent and that the claims match, then allocates `child_id` and `session_id` itself.
4. One recoverable, fsynced transaction records invitation consumption, the immutable IDs, the family edge, the session header, the remote record, `attempt=1`, and `connection_epoch=1`.
5. Only once recovery can reproduce the committed child does the daemon mint the child lease and return `ADMITTED`. Attempt and epoch are assigned here, after admission, never before.
6. The remote launcher gates model init, tools, and repo writes on `ADMITTED`.

A retry with the same `admission_request_id` and identical claims returns the same committed admission; a second use of the invitation, or changed claims, fails closed. `RlmSpawnLedger` stays the append-only family-topology authority ([`rlm-ledger.ts:24-95`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/modes/daemon/rlm-ledger.ts#L24-L95)); the remote record only supplements that edge with runtime, connection, attempt, and lease state.

### Family: parent-only by default

Prime's family predicate permits parent, same-parent siblings, and direct children, rejecting cousins and more distant nodes ([`agent-messages.ts:216-249`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/src/core/agent-messages.ts#L216-L249)). A remote worker in v1 is **parent-only**: W may message its exact parent P and nothing else. Sibling and every other family target is rejected — even a sibling the local predicate would allow — because W's job sandbox is less trusted than a co-resident local child. The daemon derives the sender from the authenticated lease and the target from its ledger; it ignores any `from`, `child_id`, `parent_session_id`, or relationship claimed in a worker payload. A ticket mapping grants no family access.

v1 remote children are leaves: `rlm()` inside W fails with an explicit unsupported error rather than creating an untracked descendant. Broadening W's reach to siblings or wider family, and remote grandchildren via delegated admission, are maintainer future decisions (see Open decisions).

### Lifecycle: stop, parent closure, and replacement

- `STOP` is a durable parent command. W first blocks new model/tool admission, aborts current work, terminates child processes it owns, writes a final checkpoint if it can, and acknowledges cancellation. **Only a received termination ACK marks the attempt `cancelled`.**
- If no ACK arrives before the deadline, the daemon revokes the lease and marks the attempt `uncertain` or `lease_expired` per the last acknowledged activity, and audits it. Revocation never claims an external side effect was rolled back and never fabricates a `cancelled`.
- When parent P is closed or replaced (session replacement), the daemon revokes the invitation and any live lease and issues `STOP` under the same rule: `cancelled` only with a termination ACK, otherwise `uncertain`/`lease_expired` + audit.
- Heartbeat expiry collects an abrupt runner loss into the same uncertain/expired path.
- `delete_subagent()` stays direct-parent only ([`agent-session-recursion.test.ts:2886-2904`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/test/agent-session-recursion.test.ts#L2886-L2904)): revoke authority, fence the epoch, request stop when reachable, write the topology tombstone, and retain the audit record needed to reject late frames. Deletion is idempotent.

### Provenance, not fabrication

W owns the real transcript, kernel checkpoint, and usage. The parent may store a small session header and typed `remote_child_*` lifecycle records; it must **not** write fabricated assistant, tool-call, tool-result, stdout, token, or cost entries to make W look local. `observe` labels remote data and checkpoint freshness. Reported usage carries explicit provenance (`source=remote_worker_report`, attempt, checkpoint hash, verification status) and never overwrites provider-native usage.

## Threat invariants

In scope: invitation theft/replay/reuse; a worker for the wrong repo/workflow/run/ticket/parent; payload identity spoofing and confused-deputy family requests; lease theft; duplicate workers, delayed old frames, reconnect races, retry split brain; daemon crash mid-admission; cancellation, runner loss, stale processes; oversized frames and message/heartbeat floods; false transcript/checkpoint/usage claims. Out of scope: compromise of the local OS user or the remote job sandbox itself — a compromised worker can do whatever its sandbox permits; the design only bounds which Prime family and controls it can reach.

Three capabilities, all minimal and all required for durable admission:

- **Invitation** — created/revoked/inspected only by code already authorized as parent P; ≥256 bits entropy, short TTL, single allocation, audience `remote-child-admit`, constrained to P plus workload identity; stored as a hash, not plaintext.
- **Workload proof** — short-lived workload identity (e.g. GitHub OIDC claims for repo id, workflow ref, run id/attempt) validated by a verifier the daemon trusts. A broker assertion alone is not authority; the verifier sits behind an interface so v1 can ship one concrete verifier.
- **Child lease** — minted at `ADMITTED`, bound to child/session/parent/attempt/connection-epoch and a worker proof-of-possession key. Its operation set is limited to attach, heartbeat, checkpoint, parent-family message, complete, and cancel. It cannot list, create, prompt, bash, kill, or shut down arbitrary daemon sessions.

Fencing invariants (post-admission): every mutation carries `{attempt, connection_epoch, sequence, idempotency_key}`; a later epoch fences the former, a later attempt fences every former lease, duplicate ids return the recorded result, out-of-order frames are rejected or buffered in a small fixed window. Quotas apply per invitation/child/parent/repo/connection (frame size, event/message/heartbeat rate, buffered bytes, retained duration, retries, concurrent children). Audit records admissions, denials, capability changes, epochs, control commands, terminal transitions, and quota violations without recording secrets. The child capability is unrelated to model-provider credentials: v1 defines no OIDC-to-provider issuer, and a deployment must solve provider authorization separately with job-bound credentials.

## No-network mock (Core MVP acceptance slice)

The invitation, workload-proof, and lease abstractions above, plus their verification, are one acceptance slice — they do **not** wait on any broker, network, or transport PR. The prototype uses a daemon-backed parent, an `InMemoryRemoteChildTransport` (bounded async queues, deterministic fault injection, no port, no credential), and a `MockRemoteWorker` whose `startModel()` asserts a committed `ADMITTED` was received.

| Area | Case | Required assertion |
|---|---|---|
| Admission | success | Ledger edge, remote record, session identity, invitation consumption durable before `ADMITTED`; parent lists one remote child. |
| Admission | authoritative write fails / response lost after commit | No start permit and no ghost child on failure; same request + claims return the same IDs, never a second allocation. |
| Invitation / claims | expired, revoked, wrong audience, reuse; wrong repo/workflow/ref/run/ticket/parent | Fail closed before child allocation, reveal no parent details, audit the reason. |
| Identity | forged sender/child/session/parent fields | Ignore payload identity; derive from lease + ledger. |
| Family | parent, sibling, cousin, other root, grandchild selectors | Parent-only for remote W; reject every other relationship; leaf `rlm()` fails explicitly. |
| Messaging | worker→parent and parent→worker | Derived sender, existing limits, durable receipt, no duplicate delivery. |
| Control | steer / follow-up | Distinct delivery, ack recovery, deadline behavior, idempotent duplicate ids. |
| Disconnect | heartbeat loss, grace-period attach, attach after expiry | Show `disconnected`; reattach same IDs at a new epoch; reject an expired lease. |
| Split brain | two attaches / two retries race | Exactly one epoch/attempt wins; the loser is fenced. |
| Stop / close | graceful ack, timeout, parent closure/replacement | Only an ACK yields `cancelled`; timeout/closure yields `uncertain`/`lease_expired` + audit, never a fabricated stop or rollback claim. |
| Delete | running, disconnected, terminal, duplicate | Revoke first, direct-parent only, tombstone once, reject late frames. |
| Restart | before commit, after commit, command/completion without ack | Recover one state, preserve cursors, resend only idempotent commands, never auto-replay model/tool work. |
| Provenance | forged usage/checkpoint, stale-attempt completion | Mark unverified or reject; never overwrite native transcript/usage. |
| Regression | ordinary local spawn/list/message/observe/delete | Existing local behavior and tests unchanged. |

Existing tests already anchor these boundaries — post-handle startup failure and cancellation ([`agent-session-recursion.test.ts:1158-1219`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/test/agent-session-recursion.test.ts#L1158-L1219)), family/cousin rejection ([`daemon-mode.test.ts:2653-2738`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/test/daemon-mode.test.ts#L2653-L2738)), and daemon process behavior ([`daemon-supervisor-process.test.ts:310-337`](https://github.com/PrimeIntellect-ai/prime-agent/blob/f8f0036cc2da1a640aad990ae8dcb7c4820ce32e/packages/coding-agent/test/daemon-supervisor-process.test.ts#L310-L337)); the remote suite should extend them, not build a second, weaker family policy.

## Layering, versioning, and delivery

**Layered over a pluggable transport.** `prime-agent.remote-child/v1` is an application protocol — admission, family messaging, lifecycle, capability semantics — carried over a transport chosen at deployment. Any outbound broker on either side (a parent or a worker reaching a hosted relay) is only a *recommended* future transport, never a dependency: the Core MVP runs entirely over the in-memory transport and assumes no broker, network, port, credential, or hosted service exists.

**Versioning is independent and additive-only.** `remote-child/v1` is versioned on its own and negotiated at admission; it does **not** expose, reuse, or tunnel the local daemon protocol (currently v7). The local additions needed to host a remote child are additive and capability-gated, so an unpatched client, a local-only daemon, and a worker without the feature all keep their current behavior.

**Delivery** — each stage independently reviewable and acceptance-gated:

1. **Core MVP** — the `RlmChildRuntime` seam, durable admission (invitation / workload-proof / lease), family/lifecycle/provenance semantics, and the no-network mock matrix above. Acceptable on its own, with no transport.
2. **Security / transport** — one concrete workload-identity verifier and a real transport behind the SPI.
3. **CLI / SDK / UI** — surface remote children through `list_subagents`, `observe`, messaging, and steer/stop/delete.
4. **Broker (optional)** — a hosted relay for a parent and worker that cannot reach each other directly; recommended only, out of core scope.

## Open decisions

1. Is a `RlmChildRuntime` seam (local `AgentSession` behind an adapter) the right factoring?
2. Which store should own the crash-recoverable admission transaction while `RlmSpawnLedger` stays the family-topology authority?
3. Which workload-identity claims should Prime core understand directly, and which stay behind the verifier interface?
4. Which remote event and checkpoint fields are enough for `observe` without a misleading mirror transcript?
5. Should v1 remote children stay strictly parent-only leaves, or is sibling reach / nested remote admission in scope now?
6. Is any outbound transport (broker or otherwise) in scope for Prime Agent, or should core stop at a transport SPI plus this no-network conformance suite — and should a compromised-broker-resistant profile be required before any hosted transport?

The wire operations, envelope layout, broker deployment shape, and version-negotiation mechanics are intentionally deferred to the accepted design. This RFC fixes the problem, the current-state gap, the MVP semantics and threat invariants, the layering/versioning/delivery stance above, and the no-network acceptance slice — not a full wire protocol.
