# Unassigned 账本生产化 spec（task #63）

- 日期：2026-07-13
- 状态：**[草案]** 生产接口与验收规范；本任务不实现代码
- 写入边界：本文件是 task #63 唯一仓库写入；生产 Rust、定义、Lean、回放程序与既有报告均不修改
- 裁决状态：Q5 已裁方案 C；本文细化承载方式，不重开 A/B/C 方案裁决

## 0. 裁决边界、事实基线与术语

### 0.1 已冻结事实

1. **[事实]** Q5 已裁方案 C：使用显式追加式 `Unassigned` 账本，禁止沉默孤儿与无证据强制吸收；逻辑键按 `(target_level, lower_id)` per-level 建账，assignment 与 host supersede 原子关联，区间套只消费 Assigned+Completed 并显式返回 coverage gap。锚：`chanlun/escalate/c-ruling-decision-20260713.md:15-19`。
2. **[事实]** b 路线双轨分层已经授权：塔保持不可变 `WindowUnit` 构造层，完成走势由消费层 as-of 纯函数组装器产生；前缀稳定、`entry_bar >= judge_at + 1` 与追加式 supersede 是三道强制无 repaint 防线。锚：`chanlun/escalate/c-ruling-decision-20260713.md:11-15`、`chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:232-236`。
3. **[事实]** #61 的 schema 是最小草案而非生产类型；它已经给出 `OPENED/ASSIGNED/TOMBSTONED/REOPENED`、稳定 orphan key、`judge_at`、host、前序事件、correlation、evidence 与 rule version 的字段骨架。锚：`chanlun/review-results/q5-orphan-attribution-research-20260713.md:96-119`。
4. **[事实]** #62 将完全分类提炼为完备、互斥、判据纯度、级别性与当下性四项；对 Q5 的工程结论是任一 lower element 在任一 as-of 恰落入 `Assigned / Unassigned / Tombstoned` 一类。锚：`chanlun/review-results/q5-complete-classification-principle-20260713.md:41-58`。
5. **[事实]** `ElementId { level, ordinal }` 已定义为全量/增量跨 bar 稳定身份，而非 per-bar `Vec` 索引；L0 ordinal 是线段序号，上层 ordinal 是该级窗口产出序。锚：`rust/src/theta_v0/classifier/recursive_tower.rs:52-72`。
6. **[事实]** #58 组装器契约是只读前缀纯函数，当前报告明确生产调用方为 0；它按坐标从完整 lower ledger 恢复窗口游标跳过段，并要求固定 `as_of=t` 时未来超集不能改变 `MoveView` 全字段。锚：`chanlun/review-results/assembler-spec-20260712.md:7-21,54-73,88-106`。
7. **[事实]** 当前 main 的 `classifier` 模块表没有注册 #58 报告所述 `move_view` 模块；因此 #58 应作为待落地原型/规范输入，而不能被描述成已接入生产。锚：`rust/src/theta_v0/classifier/mod.rs:50-64`、`chanlun/review-results/assembler-spec-20260712.md:7-9`。
8. **[事实]** #60 证明 `213/93/38/0` 属于旧窗口对象宇宙，不能原搬到 CompletedMove/C2；最近对象映射把 213 分为 `22 CompletedDistinct / 146 至少一侧 Pending / 45 至少一侧无归属`，把其中 93 个 late-success 分为 `7/69/17`。锚：`chanlun/review-results/c-ruling-evidence-replay-20260712.md:67-88`。

### 0.2 本 spec 的设计结论

- **[推断]** `Unassigned` 不能只是一个诊断数组；它必须与 `MoveView` 在同一个 as-of 纯归约 seam 内生成，否则 MoveView 可通过 prefix test，而孤儿状态仍可被未来信息重写。依据：`chanlun/review-results/q5-orphan-attribution-research-20260713.md:85-91`。
- **[草案]** 对外只暴露一个深 module interface：`assemble_level_view(...) -> Result<LevelAsOfView, AssembleError>`。`MoveView` 组装、事件归约、三分覆盖校验在 module 内部完成；测试也只穿过该 interface。
- **[草案]** `UnassignedEvent` 是追加事实行；`UnassignedView`、`CoverageIndex` 和 `LevelAsOfView` 都是可丢弃、可重算的派生视图，禁止把派生视图反写为历史事实。
- **[草案]** 本文中的 `Assigned+Completed` 一律表示合取：lower key 已归属于某 host，且该 host 在同一 `as_of` 下是 `Completed`。它不是两个并列集合。

### 0.3 名词

- **[草案] `LedgerScope`**：instrument、base timeframe、data partition/replay run 的外层命名空间。它隔离不同数据宇宙，但不改变裁决指定的逻辑键。
- **[草案] `UnassignedKey`**：scope 内的 `(target_level, lower_id)`；其中 `lower_id: ElementId`。
- **[草案] `generation`**：同一 key 被释放或因规则迁移重开后的生命周期代数，从 0 开始，只能在 `REOPENED` 时加 1。
- **[草案] `sequence`**：同一 key 的全生命周期追加序号，从 0 严格递增；用于同一 `judge_at` 内确定顺序。
- **[草案] `rule_version`**：归属/组装规则版本。墓碑只对其事件所写版本成立；跨版本重审必须显式 `REOPENED`。

## 1. 完全分类不变量与可见域

### 1.1 per-level 对象域

**[草案]** 在查询 `Q=(scope,target_level,W=[w0,w1],as_of=t,rule_version=r)` 下，lower 可见域为：

```text
VisibleLower(Q) = stable_sort_by(lower_id) {
  x in LowerLedger[target_level-1] |
  x.id.level + 1 = target_level
  and w0 <= x.start_index <= x.end_index <= min(w1,t)
  and x.settled_at <= t
  and x.rule_version = r
}
```

**[事实]** #58 已冻结坐标可见条件 `w0 <= start <= end <= min(w1,t)`，并要求 tower/lower/divergence 都不得越过 `t`。锚：`chanlun/review-results/assembler-spec-20260712.md:54-73`。

**[推断]** 为使 `ElementId` 的“稳定对象”与事件的“最早可知”分离，生产材料必须显式携带 `settled_at`；仅用 `end_index <= t` 无法证明 parser 或上级对象已经结算。

### 1.2 三分完备性

**[草案]** 对每个 `k in VisibleLower(Q)`，同一查询中必须恰有一个 disposition：

```text
Assigned_Q(k) xor Unassigned_Q(k) xor Tombstoned_Q(k)

|VisibleLower(Q)|
  = |Assigned_Q| + |Unassigned_Q| + |Tombstoned_Q|
```

其中：

- **[草案]** `Assigned_Q(k)`：同一 as-of 的 `MoveView` 中恰有一个可见 host chain 包含 `k`；若账本当前行为 `ASSIGNED`，其 `HostRef` 必须与该 host/version 完全一致。
- **[草案]** `Unassigned_Q(k)`：该 key 的可见事件链归约结果为 `OPENED` 或 `REOPENED`，且不存在可见 host membership。
- **[草案]** `Tombstoned_Q(k)`：可见事件链归约结果为 `TOMBSTONED`，且不存在可见 host membership。
- **[草案]** “无事件且无 host”、多 host、状态与 host 冲突都返回 `AssembleError`；不得降级成普通 coverage gap，否则会重新制造沉默孤儿。

**[事实]** 完全分类的 per-level、无含糊与当下性依据见 `chanlun/review-results/q5-complete-classification-principle-20260713.md:11-27,41-58`。

## 2. `UnassignedEvent` schema 定稿

### 2.1 Rust 类型草案

以下类型全部是 **[草案]**；名称用于冻结字段语义，不表示本任务已经写入 Rust。

```rust
pub type SourceIndex = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RuleVersion(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MoveId(pub u128);            // opaque；最终编码由 Move 事件 spec 冻结

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CorrelationId(pub u128);     // 同一原子追加批次

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UnassignedKey {
    pub target_level: u32,
    pub lower_id: ElementId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UnassignedEventId {
    pub key: UnassignedKey,
    pub sequence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    pub start: SourceIndex,
    pub end: SourceIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostCompletionAtEvent {
    Pending,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostRef {
    pub move_id: MoveId,
    pub move_version: u32,
    pub level: u32,
    pub range: SourceRange,
    pub completion_at_event: HostCompletionAtEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnassignedEventType {
    Opened,       // wire/storage name: OPENED
    Assigned,     // wire/storage name: ASSIGNED
    Tombstoned,   // wire/storage name: TOMBSTONED
    Reopened,     // wire/storage name: REOPENED
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasonCode {
    CursorGapRecomposable,
    NoContainingSeedWindow,
    LeadingResidual,
    TerminalInsufficient,
    NoHostAtAsOf,
    LegalHostEstablished,
    HostSupersededAdopted,
    HostSupersededReleased,
    FinalizedNoLegalHost,
    RuleVersionReevaluation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceRef {
    CursorGap(CursorGapEvidence),
    NoHost(NoHostEvidence),
    Assignment(AssignmentEvidence),
    Supersede(SupersedeEvidence),
    Tombstone(TombstoneEvidence),
    RuleMigration(RuleMigrationEvidence),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnassignedEvent {
    pub event_id: UnassignedEventId,
    pub key: UnassignedKey,
    pub generation: u32,
    pub lower_range: SourceRange,
    pub event_type: UnassignedEventType,
    pub reason: ReasonCode,
    pub judge_at: SourceIndex,
    pub host: Option<HostRef>,
    pub previous_event_id: Option<UnassignedEventId>,
    pub correlation_id: Option<CorrelationId>,
    pub evidence: EvidenceRef,
    pub rule_version: RuleVersion,
}
```

**[草案]** `EvidenceRef` 各 payload 至少包含以下可机检字段：

```rust
pub struct CursorGapEvidence {
    pub lower_settled_at: SourceIndex,
    pub scan_evaluated_at: SourceIndex,
    pub lower_ordinal: u64,
    pub selected_left_window: Option<ElementId>,
    pub selected_right_window: Option<ElementId>,
    pub containing_seed_candidates: Vec<SeedCandidateVerdict>,
}

pub struct NoHostEvidence {
    pub lower_settled_at: SourceIndex,
    pub assembly_evaluated_at: SourceIndex,
    pub coordinate_window: SourceRange,
    pub candidate_host_ids: Vec<MoveId>,
    pub rejection_codes: Vec<HostRejectionCode>,
}

pub struct AssignmentEvidence {
    pub lower_settled_at: SourceIndex,
    pub host_event_id: MoveEventId,
    pub host_judge_at: SourceIndex,
    pub membership_proof: HostMembershipProof,
}

pub struct SupersedeEvidence {
    pub move_supersede_event_id: MoveEventId,
    pub old_host: HostRef,
    pub new_host: Option<HostRef>,
    pub disposition: SupersedeDisposition, // ADOPTED | RELEASED
    pub supersede_judge_at: SourceIndex,
}

pub struct TombstoneEvidence {
    pub lower_settlement: LowerSettlementEvidence,
    pub successor_closure: SuccessorClosureEvidence,
    pub coordinate_finalization: CoordinateFinalizationEvidence,
    pub exhaustive_host_check: ExhaustiveHostCheckEvidence,
}

pub struct RuleMigrationEvidence {
    pub from_rule_version: RuleVersion,
    pub to_rule_version: RuleVersion,
    pub migration_event_id: RuleMigrationEventId,
    pub migration_judge_at: SourceIndex,
}
```

**[草案]** `MoveEventId`、`HostMembershipProof` 等是未来 Move supersede 账本的 opaque 类型；本 spec 冻结它们必须可被 reducer 验证，不冻结其序列化布局。

### 2.2 行级不变量

每一条事件行必须通过以下 **[草案]** 校验：

1. `event_id.key == key`。
2. `key.lower_id.level + 1 == key.target_level`；禁止跳级与跨级复用。
3. `lower_range.start <= lower_range.end`，且与同 `rule_version` 的 lower material 对该 `lower_id` 的范围完全一致；range 不是第二身份源。
4. `event_id.sequence == 0` 时 `previous_event_id=None`；否则 `previous_event_id.sequence + 1 == event_id.sequence`，且前序 key 相同。
5. `judge_at >= lower_settled_at`，并且 `judge_at >= evidence` 内每个事实的 `available_at`；墓碑另有等号硬门，见 §7。
6. `ASSIGNED` 必须 `host=Some`、`correlation_id=Some`；`OPENED/TOMBSTONED` 必须 `host=None`。
7. `REOPENED` 必须 `host=None`、`correlation_id=Some`，并指向被释放的 Move supersede 或规则迁移事件。
8. host 的 `level == key.target_level`，且 host chain 的同版本 membership proof 必须包含 `lower_id`。
9. `correlation_id` 一旦出现，Move 账本与 Unassigned 账本在该 correlation 下必须形成完整原子批次；缺任一侧时整批不可见。
10. 没有 `Unknown`/自由文本 reason；新增 reason 必须提升 `rule_version`、补 reducer exhaustive match 和迁移测试。

**[事实]** assignment 改变既有 host chain 时必须同 correlation 追加 Move supersede；旧版本不可覆盖。锚：`chanlun/review-results/q5-orphan-attribution-research-20260713.md:121-133`。

## 3. 状态机

### 3.1 可见状态与 disposition 映射

**[草案]** 事件链最后一个可见事件就是账本当前状态；`REOPENED` 是稳定可查询状态，不是必须立刻跟一个 `OPENED` 的瞬时过渡。

**[草案]** `ORPHAN_ASSIGNED` 与 `ORPHAN_TOMBSTONED` 是转移名称；持久化行分别写成 `event_type=ASSIGNED, reason=LEGAL_HOST_ESTABLISHED` 与 `event_type=TOMBSTONED, reason=FINALIZED_NO_LEGAL_HOST`。这样状态枚举保持四值封闭，转移动作仍可在审计日志中明确命名。

| 当前事件状态 | 完全分类 disposition | host | generation 语义 |
|---|---|---|---|
| `OPENED` | `Unassigned` | 禁止 | 首次进入账本，必须为 0 |
| `ASSIGNED` | `Assigned` | 必填 | 同代已转正或已改配 host |
| `TOMBSTONED` | `Tombstoned` | 禁止 | 同 rule version 的终态 |
| `REOPENED` | `Unassigned` | 禁止 | 新一代已开启，等待新归属/墓碑 |

### 3.2 合法转移表

以下转移全集是 **[草案]**；表外转移一律 `InvalidTransition`。

| from | to | generation | 必需 reason | 原子关联 |
|---|---|---:|---|---|
| `ABSENT` | `OPENED` | `0` | 五种 OPEN reason 之一 | 无 |
| `OPENED` | `ASSIGNED` | 不变 | `LEGAL_HOST_ESTABLISHED` | `MOVE_CREATED` 或 `MOVE_SUPERSEDE` |
| `REOPENED` | `ASSIGNED` | 不变 | `LEGAL_HOST_ESTABLISHED` | `MOVE_CREATED` 或 `MOVE_SUPERSEDE` |
| `OPENED` | `TOMBSTONED` | 不变 | `FINALIZED_NO_LEGAL_HOST` | 无；须过 §7 合取门 |
| `REOPENED` | `TOMBSTONED` | 不变 | `FINALIZED_NO_LEGAL_HOST` | 无；须过 §7 合取门 |
| `ASSIGNED(old)` | `ASSIGNED(new)` | 不变 | `HOST_SUPERSEDED_ADOPTED` | 同一 `MOVE_SUPERSEDE+ADOPTED` |
| `ASSIGNED` | `REOPENED` | `old+1` | `HOST_SUPERSEDED_RELEASED` | 同一 `MOVE_SUPERSEDE+RELEASED` |
| `TOMBSTONED` | `REOPENED` | `old+1` | `RULE_VERSION_REEVALUATION` | 规则迁移事件；新 rule version |

**[草案]** 若 lower 在第一次可见时已经直接进入唯一 host，可由 MoveView membership 派生 `Assigned`，不强制制造 `ABSENT->ASSIGNED` 的 Unassigned 事件；但若它后来进入本账本，事件链必须从 `OPENED` 开始。派生 Assigned 与显式 Assigned 使用同一 coverage 校验。

**[草案]** 禁止 `TOMBSTONED->ASSIGNED` 直跳；必须先在新 `rule_version` 下 `REOPENED(generation+1)`，再以新证据 `ASSIGNED`。禁止 `ASSIGNED->TOMBSTONED` 直跳；必须由 Move supersede 先释放为 `REOPENED`，再过墓碑合取门。

### 3.3 ReasonCode 全集与判定条件

**[事实]** #61 给出的最小 reason 集包含五个初始未归属原因、host 释放与最终无 host；它同时要求 assignment 与 host supersede 建立事件关联。锚：`chanlun/review-results/q5-orphan-attribution-research-20260713.md:119-132`。

以下十个 code 是本 spec 的封闭 **[草案]** 全集。所有 OPEN 原因共同要求 `lower_id in VisibleLower(Q)` 且同一 `MoveView(Q)` 中不存在 host；然后再按表中优先级判定，保证互斥。每个谓词都在同一 `target_level/as_of/rule_version` 下计算。

| 优先级 / code | 允许事件 | 可机检判据 | 必需 evidence | 依据锚 |
|---|---|---|---|---|
| 1 `TERMINAL_INSUFFICIENT` | `OPENED` | lower 位于扫描停止后的 0–2 个尾元素中，且从未走过失败 `i+=1` 分支 | lower ordinal、scan stop、尾部 ordinal 集 | `chanlun/review-results/q5-orphan-attribution-research-20260713.md:23-29,33-42` |
| 2 `NO_CONTAINING_SEED_WINDOW` | `OPENED` | 对包含 q 的所有连续三元组起点 `j in [q-2,q]`（裁到合法边界）穷举，全部 seed predicate=false | 每个 candidate 的三 lower IDs、判据版本、失败原子 | `chanlun/review-results/q5-orphan-attribution-research-20260713.md:25-31` |
| 3 `LEADING_RESIDUAL` | `OPENED` | 至少一个 containing seed 合法，且 q 严格位于首个已选成功窗之前 | 合法 candidate、首个 selected window ID | `chanlun/review-results/q5-orphan-attribution-research-20260713.md:23-29` |
| 4 `CURSOR_GAP_RECOMPOSABLE` | `OPENED` | 至少一个 containing seed 合法，且 q 位于两个已选成功窗之间 | 左/右 selected window IDs、合法 candidate 列表 | `chanlun/review-results/q5-orphan-attribution-research-20260713.md:23-29,37-46` |
| 5 `NO_HOST_AT_AS_OF` | `OPENED` | q 不满足前四项，但 `VisibleLower` 中存在、MoveView 中无 host；必须给出完整 candidate host 拒绝表，不得作为 catch-all 吞掉漏审 | coordinate window、候选 host IDs、逐 host rejection | `chanlun/review-results/q5-orphan-attribution-research-20260713.md:119,147-168` |
| `LEGAL_HOST_ESTABLISHED` | `ASSIGNED` | 前态为 `OPENED/REOPENED`，可见 host chain 唯一包含 lower，host event 与本事件同 correlation | host event、membership proof、host completion snapshot | `chanlun/review-results/q5-orphan-attribution-research-20260713.md:121-133` |
| `HOST_SUPERSEDED_ADOPTED` | `ASSIGNED` | 前态 `ASSIGNED(old)`；同 correlation 的 supersede 令新 host/version 仍唯一包含 lower | old/new host、`ADOPTED` move event | `chanlun/review-results/q5-orphan-attribution-research-20260713.md:127-132` |
| `HOST_SUPERSEDED_RELEASED` | `REOPENED` | 前态 `ASSIGNED(old)`；同 correlation 的 supersede 标为 `RELEASED`，且新可见 MoveView 无 host 包含 lower | old host、release event、无 host proof | `chanlun/review-results/q5-orphan-attribution-research-20260713.md:127-132` |
| `FINALIZED_NO_LEGAL_HOST` | `TOMBSTONED` | §7 五项墓碑条件全部为真 | 完整 `TombstoneEvidence` | `chanlun/review-results/q5-orphan-attribution-research-20260713.md:135-145` |
| `RULE_VERSION_REEVALUATION` | `REOPENED` | 前态为旧 rule version 的 `TOMBSTONED`；存在显式迁移事件，`to_version != from_version`，generation+1 | 迁移事件与新旧版本 | `chanlun/review-results/q5-orphan-attribution-research-20260713.md:145` |

**[草案]** `NO_HOST_AT_AS_OF` 是经审计的余类，不是默认分支：若其 `candidate_host_ids/rejection_codes` 不完备，reducer 返回 `IncompleteNoHostEvidence`。

## 4. 组装器集成与纯归约 interface

### 4.1 唯一外部 seam

以下 interface 是 **[草案]**：

```rust
pub struct LevelViewMaterial<'a> {
    pub move_material: MoveViewMaterial<'a>,
    pub lower_meta: &'a [LowerSettlement],
    pub unassigned_events: &'a [UnassignedEvent],
    pub move_events: &'a [MoveEvent],
    pub finalized_coordinates: &'a [CoordinateFinalized],
}

pub struct LevelViewQuery {
    pub scope: LedgerScope,
    pub target_level: u32,
    pub coordinate_window: CoordinateWindow,
    pub as_of: SourceIndex,
    pub rule_version: RuleVersion,
}

pub struct LevelAsOfView {
    pub query: LevelViewQuery,
    pub move_view: MoveView,
    pub unassigned_view: UnassignedView,
    pub coverage: CoverageIndex,
}

pub fn assemble_level_view(
    material: LevelViewMaterial<'_>,
    query: LevelViewQuery,
) -> Result<LevelAsOfView, AssembleError>;
```

**[草案]** `assemble_move_view`、`reduce_unassigned_events` 与 `validate_complete_classification` 是该 module 的内部 implementation；不得让调用方先各自调用再自行拼接，否则不同 consumer 会产生不同 visible domain 或漏掉 XOR 校验。

### 4.2 事件可见域与归约顺序

**[草案]** 事件可见域：

```text
VisibleEvent_Q = stable_sort_by(key, sequence) {
  e in unassigned_events |
  e.key.target_level = Q.target_level
  and e.key.lower_id in VisibleLower(Q)
  and e.rule_version participates in Q's explicit migration chain
  and e.judge_at <= Q.as_of
}
```

**[草案]** 归约算法固定为：

1. 计算 `VisibleLower(Q)`；lower material 本身不得读未来。
2. 只用 `judge_at<=t` 的 Move material/events 组装 `MoveView(Q)`，并按可见 supersede 链解析 host 当前版本。
3. 对每个 key 按 `sequence` 校验 previous chain、状态转移、generation 与 reason/evidence。
4. 将事件链末态归约为 `OPENED/ASSIGNED/TOMBSTONED/REOPENED`。
5. 从 `MoveView` 建立 `lower_id -> HostRef` 唯一 membership index。
6. 对每个 visible lower 执行 §1.2 XOR 校验；任何缺类、重类或 host/version 冲突立即报错。
7. 仅在全部 key 校验通过后构造 `UnassignedView` 与 `CoverageIndex`。

**[草案]** `UnassignedView` 包含当前 `OPENED/REOPENED/TOMBSTONED` 项及其最后事件、generation、reason、judge_at、evidence；`ASSIGNED` 项由 `MoveView+CoverageIndex` 查询，不在 `unassigned_view.open_entries` 中重复返回。

**[草案]** 同一 `judge_at` 的多事件不能依赖物理文件顺序；合法顺序只由 per-key `sequence/previous_event_id` 与 correlation batch 决定。

### 4.3 as-of 硬规则

- **[草案]** `e.judge_at > t` 的事件对 `as_of=t` 完全不可见，即使其物理行已经存在。
- **[草案]** 可见事件引用的 host、supersede、finalization、rule migration 事实本身也必须 `judge_at<=t`；引用未来事实是 `FutureEvidence` 错误。
- **[草案]** 查询不得自动选“最新 rule version”；调用方必须显式传 `rule_version`，或显式选择一个迁移链入口。
- **[草案]** 未来 assignment/reopen/tombstone 只追加新事件与新 host version；旧行、旧 host version 与旧 correlation batch 的字节内容不得改变。
- **[草案]** `LevelAsOfView.query.as_of`、`MoveView.as_of`、`UnassignedView.as_of` 必须全相等；不允许消费层拼接不同快照。

**[事实]** 追加新版本而不改写当时可见事实，是已裁 supersede 口径。锚：`chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:232-236`。

## 5. coverage-gap 查询 interface

### 5.1 区间套入口

**[事实]** 已裁口径要求区间套成为组装视图之上的自顶向下、嵌套、as-of 只读查询，而不是直接把窗口单元冒充完成走势。锚：`chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:238-250`。

以下查询 interface 是 **[草案]**：

```rust
pub struct NestedQueryInput<'a> {
    pub level_views: &'a [LevelAsOfView],
    pub parent_range: SourceRange,
    pub as_of: SourceIndex,
    pub rule_version: RuleVersion,
}

pub struct NestedQueryResult {
    pub matches: Vec<NestedDivergenceMatch>,
    pub coverage_gaps: Vec<CoverageGap>,
}

pub struct CoverageGap {
    pub target_level: u32,
    pub lower_id: ElementId,
    pub lower_range: SourceRange,
    pub kind: CoverageGapKind,
    pub last_event_id: Option<UnassignedEventId>,
    pub judge_at: SourceIndex,
}

pub enum CoverageGapKind {
    OpenUnassigned,
    ReopenedUnassigned,
    Tombstoned,
    AssignedToPendingHost { host: HostRef },
}

pub fn nested_divergence_search_as_of(
    input: NestedQueryInput<'_>,
) -> Result<NestedQueryResult, NestedQueryError>;
```

### 5.2 消费与 fail-closed 规则

- **[草案]** `matches` 只消费 `Assigned_Q(k) && host.completion(Q)==Completed` 的结构；Pending host、OPENED、REOPENED、TOMBSTONED 一律不能成为父子范围、背驰段或完成证据。
- **[草案]** 上述四种合法但不可消费的状态必须进入 `coverage_gaps`；返回 match 与 gap 可并存，调用方不得以“有 match”掩盖同范围 gap。
- **[草案]** gap 按 `(target_level, lower_range.start, lower_id)` 稳定排序；同 key 恰返一个当前 gap。
- **[草案]** `UnclassifiedLower`、多 host、事件链断裂、rule/as_of 不一致不是业务 gap，而是 `NestedQueryError::InvalidLevelView`；查询必须 fail closed。
- **[草案]** 所有 `level_views` 必须来自同一 `scope/as_of/rule_version`；否则拒绝查询。

**[推断]** `AssignedToPendingHost` 不是归属缺口，而是完成证据缺口；仍需返回它，才能区分“已知归属但尚未完成”与“无归属/不可归属”。依据：#61 明确 Assigned 不等于 Completed，且区间套只把 `Assigned ∧ host=Completed` 当证据，锚：`chanlun/review-results/q5-orphan-attribution-research-20260713.md:131-133`。

## 6. 前缀稳定性 property test 扩展

### 6.1 总性质

**[事实]** #58 当前硬门只比较 `MoveView` 全字段；Q5 账本要求固定 `t` 时只看 `judge_at<=t` 的 open/assign/tombstone 事件，未来只能生成新版本。锚：`chanlun/review-results/assembler-spec-20260712.md:66-73,201-209`、`chanlun/review-results/q5-orphan-attribution-research-20260713.md:85-89`。

**[草案]** 扩展后的总性质为：

```text
for all valid material D, future extension F, query Q(as_of=t):

assemble_level_view(Visible<=t(D), Q)
  ==
assemble_level_view(D ++ F, Q)

where every event/evidence row in F has judge_at > t
```

等号必须覆盖：

- `MoveView` 全字段；
- `UnassignedView` 全字段，包括 state、generation、reason、last event、host absence 与 evidence；
- `CoverageIndex` 的三类计数、逐 key disposition、host version；
- `coverage_gaps` 的内容与稳定顺序；
- 错误结果：若短前缀合法，纯未来行不得让固定 t 的查询从 `Ok` 变 `Err` 或反之。

### 6.2 必须落地的 properties

以下测试名与断言均为 **[草案]**：

1. `ledger_prefix_stability_ignores_future_events`
   - 生成合法 `OPENED -> ASSIGNED -> REOPENED -> ASSIGNED/TOMBSTONED` 链；随机 t；把 `judge_at>t` 后缀加到输入后，固定 t 的 `LevelAsOfView` bit-exact 相等。
2. `future_assignment_creates_new_version_without_repainting_opened`
   - t 时为 OPENED，T 时因新 host 转为 ASSIGNED；断言 query(t) 永远 OPENED，query(T) 指向新 host version，OPENED 原行内容不变。
3. `future_release_reopens_new_generation_only`
   - t 时 ASSIGNED(old)，T 时 `MOVE_SUPERSEDE+RELEASED`；断言 query(t) 仍为 old host，query(T) 为 REOPENED 且 `generation=old+1`。
4. `future_rule_migration_does_not_erase_tombstone`
   - 旧版本 t 时 TOMBSTONED，新版本 T 时 REOPENED；旧 `rule_version/as_of` 查询仍还原墓碑，新版本只见显式迁移链。
5. `same_judge_at_uses_sequence_not_input_order`
   - 打乱物理行顺序，合法 previous chain 归约结果相等；sequence 重复/断链则稳定报错。
6. `move_and_unassigned_views_share_one_as_of`
   - 随机制造 host membership 与 event state；只有 XOR 完整且 as_of 相同才返回 `Ok`。
7. `append_only_rows_are_byte_stable`
   - 在持久化 adapter 的测试 fixture 中记录既有行编码，追加未来 batch 后旧行 bytes/hash 不变，且没有 update/delete 操作。
8. `atomic_correlation_is_all_or_nothing`
   - 缺 Move supersede 或缺对应 Unassigned event 的半批次均不可见并报错；完整批次一次生效。
9. `coverage_partition_is_total_and_disjoint`
   - 对任意生成的合法 view，逐 key XOR 成立且 `N=A+U+T`；删除一行、复制 host 或冲突状态必须失败。
10. `tombstone_gate_rejects_each_missing_conjunct`
    - §7 的五个合取项逐个翻 false，`TOMBSTONED` 均被拒绝；仅全 true 时接受。

**[草案]** #58 已有的 `prefix_stability_at_extension_boundaries`、offline oracle、孤儿恢复与 leading skip 测试必须保留并提升到 `assemble_level_view` interface，而不是删除 MoveView 断言。现有硬门名称与范围锚：`chanlun/review-results/assembler-spec-20260712.md:201-209`。

## 7. per-level 建账与 `ORPHAN_TOMBSTONED` 合取门

### 7.1 per-level 可执行检查

以下检查全部是 **[草案]**，每次 append 与 reduce 都必须执行：

| ID | 可机检判据 | 失败 |
|---|---|---|
| PL-1 | `key.lower_id.level.checked_add(1) == Some(key.target_level)` | `CrossLevelKey` |
| PL-2 | scope 内 `(target_level,lower_id)` 对应同 rule version 的唯一 lower material | `MissingOrDuplicateLower` |
| PL-3 | event `lower_range == lower_material.range` | `RangeIdentityConflict` |
| PL-4 | 若有 host，`host.level == target_level`，host chain membership 包含该 `lower_id` | `CrossLevelHost` / `FalseMembership` |
| PL-5 | 同 key 的 sequence 严格连续，generation 只在 REOPENED 时 `+1` | `BrokenEventChain` |
| PL-6 | 同一 query 不混用未显式迁移的 rule versions | `RuleVersionMix` |
| PL-7 | 每个 `VisibleLower` 通过 `Assigned xor Unassigned xor Tombstoned` | `IncompleteClassification` |
| PL-8 | `sum(level counts) == global counts`，且 key 不能跨 target level 去重 | `CrossLevelCountCollapse` |

**[事实]** per-level 不是存储优化，而是完全分类的级别性要求。锚：`chanlun/review-results/q5-complete-classification-principle-20260713.md:24-27,60-63`。

### 7.2 墓碑五项合取条件

**[事实]** P0 裁决要求墓碑只能通过 #61 §4.3 的严格合取门；该门列明 lower settled、后继封闭、坐标域最终化、当前规则穷举无 host 且无 Pending 跨越、`judge_at` 取四个时点最大值。锚：`chanlun/escalate/c-ruling-decision-20260713.md:15-19`、`chanlun/review-results/q5-orphan-attribution-research-20260713.md:135-145`。

生产 predicate 定稿为以下 **[草案]**：

```text
CanTombstone(key, Q, E) := T1 and T2 and T3 and T4 and T5
```

1. **T1 — lower 已 settled**
   - 可机检：存在 `LowerSettled` 事实，`id/range/rule_version` 与 key/event 相等；在 `lower_settled_at..judge_at` 的可见事件中不存在未被解析的 lower supersede。
   - evidence：`lower_settlement_event_id`、`lower_settled_at`、`material_hash`。
2. **T2 — 后继完成事件已封闭候选归属区间**
   - 可机检：存在 `CompletedMove`/closure event，`successor.start_index > lower.end_index`；其 closure range 与 deterministic candidate search range 的右界相等或越过，并且 `successor_closed_at<=judge_at`。
   - evidence：successor host/version、range、completion event ID、`successor_closed_at`。
3. **T3 — 显式坐标最终化水位越过全部候选边界**
   - 可机检：存在同 scope/rule version 的 `CoordinateFinalized { finalized_through, judge_at }`；`finalized_through >= exhaustive_search_range.end`；普通输入末端、当前最大 bar 或“暂时没有新数据”不能替代该事件。
   - evidence：finalization event ID、`coordinate_finalized_at`、`finalized_through`、search range。
4. **T4 — 穷举无合法 host 且无 PendingMove 跨越 lower**
   - 可机检：确定性 candidate enumerator 对 search range 内所有 host/seed candidates 生成稳定 `candidate_set_hash`；每项有 rejection verdict，`legal_host_ids.is_empty()`；同时不存在 `PendingMove` 满足 `pending.start <= lower.start && lower.end <= pending.end`，也不存在声明可延伸边界跨越 lower 的 Pending host。
   - evidence：candidate IDs、逐项 rejection、candidate set hash、pending scan hash、`exhaustive_check_at`。
5. **T5 — judge_at 取证据最晚时点的精确最大值**
   - 可机检：

     ```text
     event.judge_at == max(
       lower_settled_at,
       successor_closed_at,
       coordinate_finalized_at,
       exhaustive_check_at
     )
     ```

   - 使用 `>=` 而非 `==` 也拒绝，防止无解释地把永久结论推迟或与其他版本混淆。

### 7.3 墓碑公共转移守卫

除 T1–T5 外，还必须满足以下 **[草案]** 行/状态守卫：当前态只能是 `OPENED/REOPENED`；`generation` 不变；reason 只能是 `FINALIZED_NO_LEGAL_HOST`；event rule version 与 exhaustive checker 相同；event 是追加行且 previous chain 连续。

**[推断]** 墓碑只对同一 `rule_version` 永久；规则升级后的重审必须追加 `RULE_VERSION_REEVALUATION`，不能删除或覆盖旧墓碑。该结论来自 #61 的规则迁移边界，锚：`chanlun/review-results/q5-orphan-attribution-research-20260713.md:145`。

## 8. #54 类计数重放接入口径

### 8.1 两层分母，禁止混名

**[事实]** #54 的旧 2,630 对象与 `213/93/38/0` 来自可变长 WindowUnit/ownership 对象宇宙；#58/C2 尚无生产入口、合法 direction provider 与 A/C hook，因此 C2 的最终分母和四个数当前未定义。锚：`chanlun/review-results/c-ruling-evidence-replay-20260712.md:22-37,67-88,335-341`。

**[草案]** 新重放先建立 attribution 总分母：

```text
D_attr(Q) = |VisibleLower(Q)|
          = Assigned(Q) + Unassigned(Q) + Tombstoned(Q)
```

**[草案]** `Assigned` 再细分：

```text
Assigned = AssignedCompleted + AssignedPending
AssignedCompleted = CompletedDistinctEligible
                  + SameHostCollapsed
                  + NonAdjacentCompleted
                  + OtherCompletedIneligible
```

**[草案]** 只有 `CompletedDistinctEligible`（离开/回试映射到两个不同、相邻、已完成的 CompletedMove）进入新的 third/firstRetrace judge。Pending、Unassigned、Tombstoned、same-host collapsed 与 non-adjacent 都是显式 `NotEligible(reason)`，不得伪装成 third 判负。

**[推断]** 这延续 #60 对 firstRetrace 的最小 C2 前提：离开走势与回试走势须是不同、已完成、相邻的 CompletedMove。锚：`chanlun/review-results/c-ruling-evidence-replay-20260712.md:90-109`。

### 8.2 重放输出 schema

以下是 **[草案]**：

```text
ReplayHeader {
  dataset_id, bars, source_end,
  assembler_rule_version, ledger_rule_version,
  direction_provider_version, divergence_pair_provider_version,
  as_of, coordinate_window
}

CoverageCounts {
  visible_lower,
  assigned_completed,
  assigned_pending,
  unassigned_opened,
  unassigned_reopened,
  tombstoned,
  invariant_ok
}

ThirdReplayCounts {
  completed_distinct_eligible,
  same_host_collapsed,
  pending_pair,
  unassigned_pair,
  tombstoned_pair,
  non_adjacent_pair,
  success,
  late_success,
  trend_context_by_completed_move_definition,
  completed_trend_decomposition
}

LegacyBridge {
  legacy_window_universe,
  legacy_success_213,
  legacy_late_success_93,
  legacy_trend_context_38,
  legacy_completed_trend_0,
  mapped_statuses_only_for_frozen_fixture
}
```

### 8.3 与 #60 的衔接方式

1. **[草案] 冻结兼容 fixture，不冻结新真值。** 使用 #60 同一 BTC 数据、同一 legacy IDs 与同一 experimental ownership/direction provider，先复现 `SUCCESS: 22/146/45=213`、`late SUCCESS: 7/69/17=93` 的映射分区。该门只证明 bridge 没漂移。事实锚：`chanlun/review-results/c-ruling-evidence-replay-20260712.md:73-80`。
2. **[草案] 旧四数只写 `legacy_*` 字段。** 新输出禁止把 `213` 填入 `success`、把 `93` 填入 `late_success`、把 `38` 填入 CompletedMove trend context，或把无 A/C hook 造成的 `0` 宣称为完成趋势真值。
3. **[草案] 新基线生成前置。** direction provider、A/C pair provider、CompletedMove adjacency 与 rule versions 全部冻结后，才在 ledger 总分母上重跑并生成新的 `success/late/trend/completed-trend` 基线。
4. **[草案] 所有未进入 judge 的对象可归约。** 每个 legacy pair 的两侧都必须关联到 coverage disposition；输出明确是 collapsed、Pending、Unassigned、Tombstoned 或其他 ineligible，不允许 `dropped/unknown` 桶。
5. **[草案] 分母守恒硬门。** 每级与全局必须同时满足 `N=A+U+T`；third 子队列还必须满足 `candidate_pairs = eligible + all_ineligible_reasons`。

**[事实]** `38` 的 legacy trend_context 与 C2 Trend 的对象定义不同，`0/213` 在原型中仍为 0 的原因是缺 A/C hook，而不是不存在结构 Trend。锚：`chanlun/review-results/c-ruling-evidence-replay-20260712.md:82-88`。

## 9. b 路线双轨落地切分与验收门

### 9.1 双轨约束

- **[草案] W 轨（WindowUnit 构造层）**：`recursive_tower.rs` 保持行为与对象身份不变，只作为稳定窗口/ElementId 原料源；task #63 后续实现不得修改其扫描、compose、ID 或 Lean 镜像。
- **[草案] V 轨（CompletedMove + Attribution 消费层）**：导入/生产化 #58 纯函数组装器，追加 Unassigned 事件归约、Move supersede 对账、coverage 与 query。
- **[草案]** 两轨只在只读材料 seam 相接：V 轨消费 W 轨窗口与稳定 IDs，W 轨不回读 MoveView/UnassignedView。

**[事实]** 该分层与裁决授权一致，且裁决明确本阶段不修改塔与 Lean。锚：`chanlun/escalate/c-ruling-decision-20260713.md:11-15,21-24`。

### 9.2 实现顺序

| 顺序 | 未来任务（均为 [草案]） | 建议文件 seam | 验收门 |
|---:|---|---|---|
| B0 | 冻结类型、错误枚举与 fixture；把本 spec 转成测试清单 | `classifier/level_view.rs` 对外 interface；内部 ledger implementation | Rust 类型可表达全部合法态；表外态不可构造或稳定报错；`recursive_tower.rs` 零 diff |
| B1 | 落地 `UnassignedEvent` reducer，不接生产 consumer | `classifier/unassigned_ledger.rs`（内部 module） | 状态模型 property、previous/generation/reason exhaustive、T1–T5 逐项负测、`N=A+U+T` |
| B2 | 导入并收敛 #58 原型，新增 `assemble_level_view` 单一 seam | `classifier/level_view.rs`；`classifier/mod.rs` 仅注册公开入口 | #58 四类硬门保留；Move+Unassigned 总 prefix property 通过；当前分类/信号/交易调用方仍为 0 |
| B3 | 落地 Move event/supersede 与 correlation 原子批次 | Move ledger module + in-memory test adapter + production append adapter | 半批次不可见；旧行 byte/hash 不变；ADOPTED/RELEASED host 对账 bit-exact |
| B4 | coverage-gap 与区间套只读迁移 | `classifier/nest.rs` 的 as-of 查询 adapter | 只消费 Assigned+Completed；四类 gap 显式；缺类/重类 fail closed；所有级同 snapshot |
| B5 | #54/#60 bridge 与新重放 | 独立 replay bin/report，不进入 library 判据 | 冻结 fixture 复现 22/146/45、7/69/17；旧四数只在 legacy 字段；新分母守恒且无 dropped |
| B6 | 分批迁移 higher center、背驰、买卖点与区间套 consumer | consumer adapters；保留 WindowUnit 诊断路径 | 每个入口有 old/new shadow diff；Pending/Unassigned 不当 false；`entry_bar >= judge_at+1` 硬门；全历史重放签字后才切主 |

### 9.3 每阶段共同硬门

以下均为 **[草案]**：

1. `cargo test` 全量通过，新增 property 有确定 seed 可复现；`git diff --check` 通过。
2. `rust/src/theta_v0/classifier/recursive_tower.rs` 与 Lean/定义文件无 diff；若确需改动，必须另立裁决任务，不能夹带。
3. 固定 `as_of=t` 时，追加任何 `judge_at>t` 的 Move/Unassigned/finalization/migration 事实都不得改变旧视图。
4. 没有 update/delete 旧事件行的代码路径；correlation batch 要么完整可见，要么完全不可见。
5. 任一 visible lower 都能落入 A/U/T 且只落一类；任何 unknown/dropped/silent fallback 使阶段失败。
6. 区间套、背驰、买卖点与交易入口在迁移前保持未调用新 module；迁移后不得绕过 `LevelAsOfView` 自行拼 snapshot。
7. 进入交易路径前必须同时满足 `entry_bar >= judge_at+1`；该门已有裁决依据，锚：`chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:232-236`。

## 10. 明确非目标与尚待冻结项

- **[事实]** 合法 CompletedMove direction provider、唯一 A/C pair 自动生成、higher center 从 CompletedMove 重建目前仍未完成，相关 C2 计数不能提前宣称为 0。锚：`chanlun/review-results/c-ruling-evidence-replay-20260712.md:335-341`。
- **[草案]** 本 spec 不选择 ledger 的磁盘格式、数据库或压缩方式；无论 adapter 如何选择，纯 reducer、append-only、atomic correlation 与 as-of 语义不得改变。
- **[草案]** 本 spec 不改变 exact-three seed predicate，也不把 `NO_CONTAINING_SEED_WINDOW` 等同于“永远不能进入 CompletedMove chain”。#58 已允许按坐标把非 seed lower 纳入 host chain，事实锚：`chanlun/review-results/assembler-spec-20260712.md:88-121`。
- **[草案]** 本 spec 不授权 Pending/Unassigned/Tombstoned 进入交易证据；它们的价值是保持完全分类、审计与未来追加修正，不是伪造结构完成。

## 11. task #63 完成判据

本 spec 只有在以下条目同时满足时才可作为后续实现依据：

1. **[草案]** Rust 行类型、状态机、十个 ReasonCode、行级不变量均形成封闭 exhaustive contract。
2. **[草案]** `MoveView + UnassignedView + CoverageIndex` 由同一可见前缀、同一 as-of、同一 rule version 纯归约。
3. **[草案]** 区间套只消费 `Assigned ∧ Completed`，四类合法 gap 显式返回，分类不完整则 fail closed。
4. **[草案]** prefix property 覆盖 Unassigned 状态、generation、host version、tombstone、correlation 与旧行不可改写。
5. **[草案]** #54 重放分母满足 `Assigned+Unassigned+Tombstoned`，并以 #60 映射作 bridge fixture 而非复制 `213/93/38/0` 为新真值。
6. **[草案]** per-level key 与墓碑五项合取门都有逐条可机检判据和逐项负测。
7. **[草案]** b 路线按 B0→B6 顺序落地；W 轨不可变，V 轨在消费层生产化，未通过前置验收不得提前迁移交易 consumer。
