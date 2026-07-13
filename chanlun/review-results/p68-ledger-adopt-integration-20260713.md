# P68 账本 × `adopt_v1` 隔离集成原型（task #68）

- 日期：2026-07-13
- 状态：**schema 定稿、隔离原型、5 条 property tests、两次账本全量重放均完成**
- 基线：主仓库当前 HEAD `bed65e9960766e209936e97f476307c399ae22a9`
- 隔离区：`/private/tmp/task68-ledger-work`，本地分支 `task68-ledger-adopt-20260713`
- 生产边界：`rust/src`、`formal`、`.chanlun/definitions` **零改动**；没有接入任何分类、信号、回测或交易消费者
- 隔离说明：沙箱禁止主仓库 `.git/refs` 写锁，真实 `git worktree add` 失败；因此用同一 HEAD 的独立本地 clone 作为等价隔离区。task67 目录存在但不是 Git 仓库，无可续作分支；其源码与日志仅作只读 fixture 权威来源。

## 0. 结论与验收

本原型冻结以下结果：

```text
adopt_v1：52 -> 33
新 Assigned：+19
其中 host Completed：7
legacy_recompose_residual：35（仅兼容诊断）
legacy_regressions：2（L0#33574、L0#37199；adopt_v1 已拦截）
```

这与临时裁决一致：单调覆盖保持基座 host 不变，所以 `52 - 19 = 33`；旧 DP 的 `35` 来自额外释放 2 个基座 host，禁止写成 `adopt_v1_residual`。裁决锚：`chanlun/escalate/adopt-overlay-residual-ruling-20260713.md:7-22`；task67 规格锚：`chanlun/review-results/adopt-overlay-spec-20260713.md:35-67,341-354`。

| 验收项 | 结果 | 状态 |
|---|---:|---|
| 基线无 host | 52 | PASS |
| `adopt_v1` 新增 Assigned | 19 | PASS |
| 新增中 Completed host | 7 | PASS |
| 覆盖后 Unassigned | 33 | PASS |
| Tombstoned | 0 | PASS；固定集五门未齐 |
| 守恒 | `52 = 19 + 33 + 0` | PASS |
| 旧重组兼容诊断 | `35 / regressions=2` | PASS；未混入 adopt 命名空间 |
| property tests | `5 passed / 0 failed` | PASS |
| 双全量重放终态 SHA-256 | 两次 `8f061df4e5fa24925bbc285f7708753530c1db5fa3f4edab8c6d4e4b916e667d` | PASS，bit-exact |

机器锚：

- `prototypes/task68-ledger-adopt/artifacts/replay-1.log:1-8`
- `prototypes/task68-ledger-adopt/artifacts/property-tests.log:16-23`
- `prototypes/task68-ledger-adopt/artifacts/replay.sha256:1-2`
- `prototypes/task68-ledger-adopt/artifacts/replay.line-counts:1-5`

## 1. 权威边界与冻结口径

1. Q5 采用追加式 C 账本，任一 lower 在同一 per-level/as-of/rule view 下恰为 `Assigned / Unassigned / Tombstoned` 之一；不能沉默丢弃，也不能无证据强吸。#63 草案锚：`chanlun/review-results/unassigned-ledger-production-spec-20260713.md:56-74`。
2. `ADOPT_ORPHAN` 不是第四个 disposition；它是 `Unassigned -> Assigned` 的 assignment cause，必须与 host/Move 侧事件用同一 correlation 原子对账。#63 锚：同文件 `:228-277`。
3. `adopt_v1` 是 `greedy_v1` 上的 add-only overlay：已有 host 原样保留，只对 `H0(k)=None` 的 key 选择合法候选。#67 锚：`chanlun/review-results/adopt-overlay-spec-20260713.md:13-31,127-161`。
4. `NO_CONTAINING_SEED_WINDOW` 只是否定 seed 资格，不足以墓碑；33 个残余继续保持 Unassigned。#65 锚：`chanlun/review-results/seed-failure-geometry-20260713.md:196-212`。
5. 35→33 已按临时裁决冻结；本任务不重开该口径。

## 2. #63 账本 schema 定稿

### 2.1 身份层次

逻辑身份与归约身份分两层，禁止把 range 或物理行号当第二身份源：

```text
UnassignedKey = (LedgerScope, target_level, lower_id)
RuleScopedKey = (UnassignedKey, rule_version)
```

- `LedgerScope = (instrument, timeframe, partition, dataset_id)`，只隔离数据宇宙。
- `lower_id = ElementId(level, ordinal)`，且 `lower_id.level + 1 == target_level`。
- 逻辑 key 不含 range；`lower_range` 必须与同版本 lower material 完全相等。
- reducer、幂等与终态 hash 使用 `RuleScopedKey`。这样同版本墓碑不可复活；规则升级只能建立显式跨版本迁移链，旧墓碑仍可原样重放。

隔离类型锚：`prototypes/task68-ledger-adopt/src/lib.rs:8-50`。

### 2.2 事件行字段

以下为 `p68-ledger-v1` 的生产合同；`required` 表示持久化行必填，`conditional` 表示由事件种类强制：

| 分组 | 字段 | 必填性 | 定稿语义 |
|---|---|---|---|
| envelope | `schema_version` | required | 固定 `p68-ledger-v1`；改变 canonical 编码必须升版 |
| envelope | `event_id` | required | `SHA-256(canonical_full_event_payload)`；内容身份 |
| envelope | `idempotency_key` | required | 见 §2.6；交付语义身份，与 payload hash 分离 |
| identity | `scope` | required | instrument/timeframe/partition/dataset_id |
| identity | `target_level` | required | 目标 host 级别 |
| identity | `lower_id` | required | 稳定 `ElementId`；不可使用 `Vec` 下标 |
| identity | `rule_version` | required | reducer/query 显式 pin；禁止 latest |
| chain | `sequence` | required | 同一 `RuleScopedKey` 从 0 严格连续 |
| chain | `generation` | required | 仅 `REOPENED` 增 1；其他转移不变 |
| chain | `previous_event_id` | conditional | `sequence=0` 时空，否则必须指向前一行 |
| material | `lower_range` | required | 校验字段，不是身份；必须匹配 lower material |
| transition | `event_type` | required | `OPENED / ASSIGNED / REOPENED / TOMBSTONED` |
| transition | `reason_code` | required | 封闭枚举；禁止 free text/Unknown |
| transition | `assignment_cause` | conditional | `ADOPT_ORPHAN` 或 `LEGAL_HOST_ESTABLISHED`；仅 ASSIGNED |
| transition | `host_ref` | conditional | ASSIGNED 必填；其余禁止 |
| transition | `supersede_ref` | conditional | host 改配/释放时必填；见 §2.5 |
| time | `judge_at` | required | 本事件全部证据最早共同可知时点；查询只见 `judge_at<=as_of` |
| atomicity | `correlation_id` | conditional | ASSIGNED、REOPENED/改配必须有；同批 Move/ledger 全有或全不可见 |
| atomicity | `batch_ordinal` | conditional | correlation 批内稳定顺序；不得依赖文件行序 |
| provenance | `producer` | required | `BASE_SCANNER / ADOPT_V1_OVERLAY / MOVE_REDUCER / COORDINATE_FINALIZER / RULE_MIGRATION` |
| provenance | `source_stream` | required | 上游事件流稳定命名 |
| provenance | `source_event_id` | required | 上游稳定事件 ID |
| provenance | `source_offset` | required | 审计/恢复位点；不单独作为逻辑身份 |
| evidence | `evidence_kind` | required | open/assignment/supersede/tombstone/migration 对应封闭 variant |
| evidence | `evidence_refs` | required | 所有被引用事实及其 available_at/judge_at |
| evidence | `evidence_hash` | required | canonical evidence payload SHA-256 |

原型实现了 RuleScopedKey、source envelope、event chain、correlation、host、墓碑证据和结构化幂等前像：`prototypes/task68-ledger-adopt/src/lib.rs:52-227`。

### 2.3 disposition 与持久事件

| 最后可见事件 | 当前 disposition | host | generation |
|---|---|---|---:|
| `OPENED` | `Unassigned` | 禁止 | 首次为 0 |
| `ASSIGNED` | `Assigned` | 必填 | 不变 |
| `REOPENED` | `Unassigned` | 禁止 | 前代 +1 |
| `TOMBSTONED` | `Tombstoned` | 禁止 | 不变；同 rule_version 终态 |

`ORPHAN_ASSIGNED`/`ADOPT_ORPHAN` 是动作/原因，不是持久 disposition。wire row 写 `event_type=ASSIGNED, assignment_cause=ADOPT_ORPHAN, reason_code=LEGAL_HOST_ESTABLISHED`。`ORPHAN_TOMBSTONED` 同理写 `event_type=TOMBSTONED, reason_code=FINALIZED_NO_LEGAL_HOST`。

### 2.4 状态机

表外转移全部 fail closed：

| from | to | generation | reason/cause | 原子侧事件 |
|---|---|---:|---|---|
| `ABSENT` | `OPENED` | `0` | 五个 OPEN reason 之一 | 无 |
| `OPENED` | `ASSIGNED` | 不变 | `LEGAL_HOST_ESTABLISHED / ADOPT_ORPHAN` | `MOVE_CREATED` 或 `MOVE_SUPERSEDE(ADOPTED)` |
| `REOPENED` | `ASSIGNED` | 不变 | 同上 | 同上 |
| `ASSIGNED(old)` | `ASSIGNED(new)` | 不变 | `HOST_SUPERSEDED_ADOPTED` | `MOVE_SUPERSEDE(ADOPTED)` |
| `ASSIGNED` | `REOPENED` | `old+1` | `HOST_SUPERSEDED_RELEASED` | `MOVE_SUPERSEDE(RELEASED)` |
| `OPENED/REOPENED` | `TOMBSTONED` | 不变 | `FINALIZED_NO_LEGAL_HOST` | 墓碑 T1–T5 全通过 |
| 旧版本 `TOMBSTONED` | 新版本 `REOPENED` | `old+1` | `RULE_VERSION_REEVALUATION` | 显式 migration event |

硬禁止：同 rule version 的 `TOMBSTONED -> ASSIGNED/REOPENED`、`ASSIGNED -> TOMBSTONED` 直跳、`ABSENT -> ASSIGNED` 的隐式历史行、无 correlation 的 assignment/release、同一 host version 的分叉 supersede。

原型 reducer/守卫锚：`prototypes/task68-ledger-adopt/src/lib.rs:341-588`。

### 2.5 supersede 链与墓碑防线

`supersede_ref` 定稿字段：

```text
old_host: HostRef
new_host: Option<HostRef>
move_supersede_event_id: EventId
disposition: ADOPTED | RELEASED
supersede_judge_at: SourceIndex
chain_parent_event_id: EventId
```

约束：

1. `old_host` 必须等于 reducer 当前 host；不能从历史分叉。
2. `ADOPTED` 必须给 `new_host`，同级、membership proof 含该 lower；`RELEASED` 禁止给 host。
3. `(move_id,move_version)` 构成有向无环链；自环、回边、同 old 的第二条出边均拒绝。
4. Move 侧与 ledger 侧使用同一 correlation；半批次不可见。
5. `adopt_v1` 不允许释放基座 host；因此固定 fixture 不产生 RELEASED。

墓碑仍采用 #63 五项严格合取：lower settled、后继封闭、显式坐标最终化、穷举无合法或 Pending host、`judge_at` 精确等于四证据时点最大值。任一项缺失均拒绝。原型锚：`prototypes/task68-ledger-adopt/src/lib.rs:136-163,493-518,561-579`。

### 2.6 事件源与幂等键

事件源职责定稿：

| producer | 可发事件 | 禁止行为 |
|---|---|---|
| `BASE_SCANNER` | `OPENED` | 不可直接 Assigned/Tombstoned |
| `ADOPT_V1_OVERLAY` | `ASSIGNED(cause=ADOPT_ORPHAN)` | 不可改写/释放 base host |
| `MOVE_REDUCER` | adopted supersede、released reopen | 不可绕过 correlation |
| `COORDINATE_FINALIZER` | `TOMBSTONED` | 不可用数据末端替代 finalization |
| `RULE_MIGRATION` | 新版本 `REOPENED` | 不可删除旧版本墓碑 |

幂等键的 canonical 前像冻结为：

```text
schema_version
|| RuleScopedKey
|| producer
|| source_stream
|| source_event_id
|| event_type
```

- 相同 key + 相同 canonical payload：`DuplicateNoop`，不追加、不改终态。
- 相同 key + 不同 payload hash：`IdempotencyConflict`，整批 fail closed。
- `source_offset` 用于恢复和审计，但上游重分片可能改变 offset，故不取代 `source_event_id`。
- `event_id` 仍是完整 payload SHA-256；幂等 key 与 event_id 不合并，才能检测“同交付身份、不同内容”。

原型锚：`prototypes/task68-ledger-adopt/src/lib.rs:196-227,371-387`。

### 2.7 canonical 终态与 hash

终态只由以下稳定字段编码并按 `RuleScopedKey` 排序：schema version、scope、target/lower/rule、range、disposition、generation、host ID/version/range/completion、last reason、last sequence、last idempotency preimage、judge_at、supersede edges。禁止编码 HashMap 次序、内存地址、日志时间或进程随机值。

原型 canonical encoder 与 SHA-256：`prototypes/task68-ledger-adopt/src/lib.rs:265-338` 及文件末 `sha256_hex`。

## 3. 隔离集成原型

### 3.1 文件

| 文件 | 作用 |
|---|---|
| `prototypes/task68-ledger-adopt/src/lib.rs` | schema 子集、纯 reducer、overlay、fixture、canonical hash |
| `prototypes/task68-ledger-adopt/src/main.rs` | 52 对象验收、两次 fresh replay、重复投递 replay、稳定输出 |
| `prototypes/task68-ledger-adopt/tests/properties.rs` | 5 条 property tests |
| `prototypes/task68-ledger-adopt/artifacts/*` | 测试日志、双重放、stderr、SHA、行数 |

独立 crate 无外部依赖，不注册到生产 `rust` crate。

### 3.2 组装流程

1. 读取同一 `as_of=4,613,598` 的 frozen task67 52-key base-unassigned fixture。
2. 对每个 visible key 追加 `OPENED(NO_CONTAINING_SEED_WINDOW)`。
3. `adopt_v1` 只对 base host 为空的 key 读候选，并按 `(mechanism, span, start, end, stable_host_id)` 排序。
4. 19 个 key 追加 `ASSIGNED(cause=ADOPT_ORPHAN)`；每条携稳定 correlation 与 host completion snapshot。
5. 33 个 key 保持 `OPENED/Unassigned`；不因 seed failure 墓碑。
6. reducer 校验 event chain、per-level、host level、correlation、supersede DAG、墓碑门和幂等冲突后，生成 canonical terminal state。

overlay 组装锚：`prototypes/task68-ledger-adopt/src/lib.rs:630-730`；冻结 fixture 锚：同文件 `:732` 起。

task68 的“全量重放”指完整 71 行账本事件源重放（52 OPENED + 19 ASSIGNED），不是重新跑 4,613,599 bars 的塔/组装器。原始 bars→19/33 的量化事实沿用 task67 已验证 fixture；本任务验证的是账本 × overlay 的事件化、归约、守恒和 bit-exact。

## 4. 验收数字

| target level | 基线 Unassigned | `adopt_v1` Assigned | 其中 Completed | 终态 Unassigned | Tombstoned |
|---:|---:|---:|---:|---:|---:|
| 1 | 40 | 17 | 5 | 23 | 0 |
| 2 | 10 | 1 | 1 | 9 | 0 |
| 3 | 2 | 1 | 1 | 1 | 0 |
| **合计** | **52** | **19** | **7** | **33** | **0** |

机制互斥归因：`CENTER_EXTENSION=1`、`TREND_ASSEMBLY=2`、`CROSS_WINDOW_RECOMPOSE=16`、`TERMINAL_CONTINUATION=0`。机器锚：`prototypes/task68-ledger-adopt/artifacts/replay-1.log:1-7`。

与旧口径的关系：

```text
adopt_v1:          52 - 19     = 33
legacy_recompose:  52 - 19 + 2 = 35
```

两个 `+2` 是旧重组释放的已有 host；本原型的 add-only 分支不为 base-assigned key 生成 ledger/ADOPT 行。

## 5. 两次账本全量重放

两次 release 进程均从同一 71-event 输入新建 reducer：

| 项 | replay 1 | replay 2 |
|---|---:|---:|
| terminal entries | 52 | 52 |
| canonical bytes | 14,804 | 14,804 |
| terminal-state SHA-256 | `8f061df4e5fa24925bbc285f7708753530c1db5fa3f4edab8c6d4e4b916e667d` | 相同 |
| stdout 行数 | 63 | 63 |
| stderr 行数 | 0 | 0 |
| full stdout SHA-256 | `af312c42c54cf67e25a6844b8a80abee892c22c9b372c40180666aa8aac4a38d` | 相同 |

此外，同一 reducer 接收整批事件第二遍时，71 条全部按幂等语义成为 no-op，终态结构与 SHA 均不变；机器输出 `duplicate_replay=PASS`。锚：`prototypes/task68-ledger-adopt/artifacts/replay-1.log:8`。

## 6. 五条 property tests

| # | test | 覆盖 | 生成范围 | 结果 |
|---:|---|---|---:|---|
| 1 | `prop_tombstone_cannot_be_resurrected` | T1–T5 逐项负测；合法墓碑后 ADOPT 拒绝；墓碑原行重复投递 no-op | 64 keys × 5 缺项 | PASS |
| 2 | `prop_supersede_chain_is_acyclic` | 线性 host version chain；回到任一祖先形成环时拒绝且状态不变 | chain length 2..32 | PASS |
| 3 | `prop_adopt_orphan_is_add_only_and_preserves_base_hosts` | 任意候选不能改变已有 H0；base key 不生成 adopted/ledger 行 | 256 seeds × 64 keys | PASS |
| 4 | `prop_ledger_entries_are_conserved_and_disjoint` | `Visible = BaseAssigned + LedgerAssigned + Unassigned + Tombstoned`；host 与非 Assigned 互斥 | 192 seeds × 96 keys × 3 levels | PASS |
| 5 | `prop_full_replay_is_idempotent_and_sha256_bit_exact` | fresh/double-delivery/inter-key reverse-order 三种完整重放终态及 SHA 一致；SHA-256 `abc` 已知向量校验 | frozen 71 events | PASS |

源码锚：`prototypes/task68-ledger-adopt/tests/properties.rs:85-367`；机器锚：`prototypes/task68-ledger-adopt/artifacts/property-tests.log:16-23`。

## 7. #54 接入点位与改动面（仅说明，不实施）

### 7.1 接入原则

#54 的 `2,630 / 213 / 93 / 38 / 0` 属于 WindowUnit/ownership 对象宇宙，不能直接塞进新账本或改名为 C2 真值。最近 bridge 仅证明：213 映射为 `22 CompletedDistinct / 146 至少一侧 Pending / 45 至少一侧无归属`，93 映射为 `7/69/17`。锚：`chanlun/review-results/c-ruling-evidence-replay-20260712.md:67-88`。

接入必须先把每个 #54 pair 的 leave/retest 两侧在同一 `LevelAsOfView(scope,level,as_of,rule tuple)` 下解析成当前 disposition/host，然后只把两个不同、相邻、Completed host 的 pair 送入 third/firstRetrace judge。Pending/Unassigned/Tombstoned 是 `NotEligible(reason)`，不是 third 判负。

### 7.2 点位表

| 顺序 | 只读现状点位 | 未来接入动作 | 改动面；本任务未改 |
|---:|---|---|---|
| 1 | `rust/src/theta_v0/classifier/mod.rs:50-64` 当前未注册 `move_view/level_view` | 注册唯一 `assemble_level_view` seam；内部组装 Move + ledger + coverage | 新 `classifier/level_view.rs`、`unassigned_ledger.rs`、`move_view.rs`；`mod.rs` 仅注册入口 |
| 2 | #54 分支 `/private/tmp/p54-audit-work/rust/src/theta_v0/classifier/mod.rs:563-601` 的 `cp_recall_upper_bound_audit` | 新增显式 `LevelAsOfView`/version tuple 参数；禁止内部读 latest tower ownership | #54 诊断 API 签名与 fixture adapter |
| 3 | #54 分支 `recursive_tower.rs:507-671` 的 `CpRecallPairAudit` 与逐相邻 WindowUnit 扫描 | pair 先 join `CoverageIndex`；映射为 CompletedMove IDs、host versions、completion judge_at；只有 eligible pair 调 judge | `CpRecallAuditCase` 输出 schema、pair identity、失败/不合格桶 |
| 4 | #54 分支 `cp_capability_smoke.rs:354-430` 的 `P54_ATOM_SUMMARY` 与硬编码 `2630/213` | 保留 `legacy_*` fixture；新增 attribution 分母 `A+U+T` 与 eligible/ineligible 守恒，不复用旧真值 | 独立 replay bin/report；不进 library 判据 |
| 5 | `/private/tmp/p54-audit-work/rust/src/bin/c60_evidence.rs:294-340` 的旧 bridge | 复用逐对象 legacy ID 对照，但输出 pin 完整 ViewVersion、host version、ledger disposition | bridge fixture 与报告字段 |
| 6 | `rust/src/theta_v0/classifier/signal.rs:614-625` 当前相邻 Segment third 入口；#54 分支另有 `judge_third_cert` | 高层 #54 只接受 CompletedMove pair；L0 Segment 路径保持独立，不能把两对象域混用 | signal adapter/新 CompletedMove judge wrapper；先裁 firstRetrace identity |
| 7 | `rust/src/theta_v0/backtest/runner.rs:338-358,592-670` 每 bar 因果塔进入策略 | 若未来迁移消费者，每 bar `as_of=i` 先组 `LevelAsOfView`，订单仍满足 `entry_bar>=judge_at+1` | runner/coverage/strategy 的后续独立迁移任务；#68 不实施 |

### 7.3 #54 新输出最低合同

```text
ReplayHeader {
  dataset_id, bars, as_of, coordinate_window,
  assembly_seam, base_partition_policy, adoption_policy,
  move_rule_version, ledger_rule_version,
  direction_provider_version, divergence_pair_provider_version
}

CoverageCounts {
  visible_lower,
  assigned_completed, assigned_pending,
  unassigned_opened, unassigned_reopened, tombstoned,
  invariant_ok
}

ThirdReplayCounts {
  completed_distinct_eligible,
  same_host_collapsed, pending_pair,
  unassigned_pair, tombstoned_pair, non_adjacent_pair,
  success, late_success
}

LegacyBridge {
  legacy_window_universe=2630,
  legacy_success_213, legacy_late_success_93,
  legacy_trend_context_38, legacy_completed_trend_0
}
```

硬门：`VisibleLower=A+U+T`；`candidate_pairs=eligible+所有 ineligible reason`；无 `unknown/dropped`；legacy 数只能位于 `LegacyBridge`。

## 8. 生产区零改动与复现

### 8.1 零改动

隔离分支执行：

```bash
git diff -- rust/src formal .chanlun/definitions
```

结果无输出。本任务新增内容仅为：

```text
chanlun/review-results/p68-ledger-adopt-integration-20260713.md
prototypes/task68-ledger-adopt/
```

### 8.2 复现命令

```bash
cd /private/tmp/task68-ledger-work/prototypes/task68-ledger-adopt

cargo fmt --check
cargo test

cargo run --release --quiet > artifacts/replay-1.log 2> artifacts/replay-1.err
cargo run --release --quiet > artifacts/replay-2.log 2> artifacts/replay-2.err
cmp artifacts/replay-1.log artifacts/replay-2.log
shasum -a 256 artifacts/replay-1.log artifacts/replay-2.log
```

验收：format 通过；5/5 properties；`cmp` exit 0；两次 terminal SHA 与完整 stdout SHA 分别 bit-exact；两次 stderr 均为空。

## 9. 明确非目标

1. 未把原型注册到生产 crate，未实现数据库/文件 adapter，未迁移 consumer。
2. 未重新运行 4,613,599-bar 塔/MoveView；task68 重放对象是 task67 已冻结的完整 71-event ledger source。
3. 未裁 direction provider、唯一 A/C pair 或 Completed Trend；7 个 Completed 仅沿用 task67 同一 as-of host snapshot。
4. 未把 33 个 seed-failure residual 墓碑；没有 coordinate finalization 与 exhaustive no-Pending evidence。
5. 未实现 #54 接入；§7 仅冻结接入 seam、改动面与计数合同。
