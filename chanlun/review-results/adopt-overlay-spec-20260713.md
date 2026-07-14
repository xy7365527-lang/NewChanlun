# ADOPT_ORPHAN 单调覆盖层生产 spec（task #67，`adopt_v1`）

- 日期：2026-07-13
- 状态：**spec、隔离 Rust 原型、property test 与双 release 重放已完成；35/33 口径冲突已按 33 口径裁决（最终裁决，见 `chanlun/escalate/adopt-overlay-residual-ruling-20260713.md`），生产晋级门 [已裁决-最终]**
- 路线：`greedy_v1` 基座 + `adopt_v1` 单调覆盖层 + `Unassigned` 账本
- 隔离原型：`/private/tmp/task67-adopt-overlay/rust/src/bin/task67_adopt_overlay.rs`
- 固定重放：`/private/tmp/task67-adopt-overlay/task67-adopt-overlay.log`
- 写入边界：仓库仅新增本文件；生产 Rust、Lean、定义、既有裁决与既有报告均未修改

## 0. 结论与验收状态

### 0.1 已完成

1. `adopt_v1` 定义为 `greedy_v1` 之上的**左偏单调并集**：基座已有 host 的 lower 原样保留；只对基座无 host 的 lower 选择合法候选。核心不变量由分支结构直接保证，而不是事后统计：

   ```text
   H1(k) = H0(k),                         if H0(k) != None
         = choose_legal(Candidates(k)),   if H0(k) == None and Candidates(k) != empty
         = None,                          otherwise
   ```

2. 隔离模型的四个测试全部通过：基座 host 单调不变、`host xor ledger` 完备互斥、#58 形式的固定 as-of 前缀稳定、候选平局裁决与输入物理顺序无关。测试输出为 `4 passed / 0 failed`，见 `/private/tmp/task67-adopt-overlay/task67-property-tests.log:2-4`。
3. BTC 1m 固定截点复现 `4,613,599` bars、末端 `as_of=4,613,598`、L0 `40,003` 段，见 task67 log `:1`；#65 的数据口径见 `chanlun/review-results/seed-failure-geometry-20260713.md:3-8`。
4. 基线 52 个无 host 中有 19 个取得候选 host，其中 7 个候选 host 为 Completed；逐级为 `17/5、1/1、1/1`，见 task67 log `:2-18,69,73,92,96,99,103`，与 #65 的旧 DP 交叉数一致：`chanlun/review-results/seed-failure-geometry-20260713.md:68-90`。
5. #65 的 35 个旧重组残余，其 lower ID、source range、`A_NO_COMMON_OVERLAP`、三类 geometry、B/R 边界位逐项嵌入 fixture 并 `assert_eq!`；机器输出见 task67 log `:19-45,74-82,97`，原报告逐项表见 `chanlun/review-results/seed-failure-geometry-20260713.md:147-194`。
6. k1..3 的 9 个固定 `1/4、1/2、3/4` 截点全部通过 `full-input@t == actual-prefix@t` 的覆盖后视图全字段断言，见 task67 log `:70-72,93-95,100-102`；#58 原硬门形式见 `chanlun/review-results/assembler-spec-20260712.md:54-73,201-209`。
7. 两次 release 输出均 104 行，`cmp` 相等、stderr 均 0 行；行数证据见 `/private/tmp/task67-adopt-overlay/task67-replay.line-counts:1-5`，SHA-256 见本文末尾及 `/private/tmp/task67-adopt-overlay/task67-replay.sha256:1-2`。

### 0.2 固定验收冲突：不能把旧 DP 的 35 冒充单调覆盖层残余

#65 同时记录：基线无 host `52`、旧重组取得 host `19`、旧重组残余 `35`。逐对象复核发现旧重组还释放了两个基座已有 host：`L0#33574`、`L0#37199`。task67 在 `:40-45` 逐一拦截这两个回退，汇总为 `legacy_regressions=2`：task67 log `:69,103-104`。

因此两个口径分别是：

```text
旧 DP 替换视图：52 - 19 + 2 = 35
adopt_v1 单调并集：52 - 19     = 33
```

这不是浮点、去重或标签差异，而是集合恒等式。若同时坚持：

```text
M1: 基座已有 host 全部不变
M2: 基座无 host 的 52 个对象中有 19 个获得 host
M3: host 与账本互斥且覆盖这 52 个对象
```

则账本基数唯一为 `52-19=33`。要求其为 35 与 M1–M3 不可满足。原型把该冲突做成运行时守恒断言并输出 `UNSATISFIABLE_WITH_MONOTONICITY`：task67 log `:103-104`。

**[已裁决-最终 2026-07-13]** 用户给定的“覆盖后残余 35；+19 获 host；regressions=0”不能同时签字。已按下列口径冻结（裁决文件：`chanlun/escalate/adopt-overlay-residual-ruling-20260713.md`）：

- 生产验收：`52→33；+19；其中 7 Completed；regressions=0`；
- 兼容诊断：继续复现旧重组 `residual=35 / regressions=2`，但字段必须命名 `legacy_recompose_*`，不得命名 `adopt_v1_*`。

如果坚持 `adopt_v1 residual=35`，则在 52 的固定对象域最多只能记 17 个真正 `Assigned`；另外两个必须降为“候选但未采用”，且需要新的合法拒绝规则，不能任意删两个。

## 1. 权威边界与版本元组

### 1.1 不重开 C2，不改 greedy

- 塔的职责仍是不可变 `WindowUnit` 材料；完整走势与归属是消费层 as-of 视图。#58 明确不改塔、不读未来、从完整 lower ledger 补回游标漏段：`chanlun/review-results/assembler-spec-20260712.md:11-21,88-121`。
- #64 的安全优先级是先生产化完整 lower chain、中枢延伸与 C 账本 seam，不把全历史最优分窗回写旧 as-of：`chanlun/review-results/orphan-reduction-paths-20260713.md:33-38,40-57`。
- #66 将 C2 seam 与扫描策略拆为两个决策，并建议 consumer-first、策略独立版本化与 CompletedFreeze：`chanlun/review-results/c2-architecture-reassessment-20260713.md:13-20,72-85,326-347,361-369`。

### 1.2 消费者必须 pin 完整版本元组

```text
ViewVersion = {
  assembly_seam: c2_v1,
  base_partition_policy: greedy_v1,
  adoption_policy: adopt_v1,
  move_rule_version,
  ledger_rule_version,
  direction_provider_version,
  divergence_pair_provider_version
}
```

规则：

1. `greedy_v1` 与 `adopt_v1` 是独立版本号；升级其中一个不自动升级另一个。
2. 查询方不得请求“latest”；必须显式 pin，沿用 #63 的显式 `rule_version` 规则：`chanlun/review-results/unassigned-ledger-production-spec-20260713.md:366-374`。
3. 输出 hash、事件与缓存键必须含完整 `ViewVersion`；不同元组不可共享 `CoverageIndex`。
4. `direction_provider` 与 A/C provider 未冻结时，Trend completion 仍保持 Pending；#64 已明确 host assignment 与 Completed 是两轴：`chanlun/review-results/orphan-reduction-paths-20260713.md:138-162`。

## 2. 输入与输出

### 2.1 输入：`greedy_v1` 的 as-of 纯函数视图

`adopt_v1` 只接受同一查询生成的 `GreedyAsOfView`，禁止自行读取全历史、全局缓存或 latest 版本：

```rust
struct GreedyAsOfView {
    query: LevelViewQuery,                 // scope/level/W/as_of/version
    visible_lower: Vec<LowerSnapshot>,     // stable order, settled_at <= as_of
    base_moves: Vec<BaseMoveSnapshot>,     // greedy_v1
    base_assignment: Map<LowerKey, HostRef>,
    base_unassigned: Map<LowerKey, CauseLabel>,
    legal_candidate_facts: Vec<CandidateFact>,
}
```

`legal_candidate_facts` 必须由构造 `GreedyAsOfView` 的同一纯 seam、同一 `VisibleLower` 与同一 `as_of` 生成；它是视图输出的一部分，不是 overlay 旁路读取的未来材料。#63 要求 Move、Unassigned 与 Coverage 在唯一 module seam 内归约，禁止调用方自行拼接：`chanlun/review-results/unassigned-ledger-production-spec-20260713.md:300-374`。

当前 #58 的公开 `MoveView` 尚不含 `settled_at`、账本与候选事实，因此生产落地时应在 #63 的内部 `assemble_level_view` seam 扩展材料快照；不得把候选生成暴露为消费者可绕过 XOR 校验的第二 API。#63 对单 seam 与 `settled_at` 的要求见同文件 `:21-25,36-72`。

### 2.2 输出

```rust
struct AdoptAsOfView {
    query: LevelViewQuery,
    base_policy: GreedyV1,
    overlay_policy: AdoptV1,
    assignments: Map<LowerKey, AssignmentRef>,
    ledger: Map<LowerKey, ResidualLedgerEntry>,
    adopted: Map<LowerKey, AdoptionDecision>,
}

enum AssignmentRef {
    Base(HostRef),
    Adopted(AdoptedHostRef),
}
```

`AdoptedHostRef` 是版本化的 attribution 引用，不覆盖 `BaseMoveSnapshot`。候选 host 的结构 chain 是 membership proof；它不会把 chain 内其他 lower 自动改配到该 host。最终 assignment 的唯一真值是 `assignments[lower_key]`。

### 2.3 核心不变量

令 `K_t` 为 `VisibleLower` 的 key 集，`H0/H1` 分别为基座/覆盖后 host 偏函数，`L1` 为残余账本：

```text
I1 Monotonicity:
  forall k in K_t. H0(k) != None -> H1(k) = H0(k)

I2 Add-only:
  dom(H0) subseteq dom(H1)

I3 Totality:
  dom(H1) union dom(L1) = K_t

I4 Disjointness:
  dom(H1) intersection dom(L1) = empty

I5 Determinism:
  eval(input, adopt_v1) = eval(input, adopt_v1)

I6 No hidden completion promotion:
  adopted completion = candidate host completion proof at the same as_of;
  absence of proof remains Pending, never false Completed
```

I1 通过“先判断 base host，命中即原样返回且不读取候选”的构造保证。I3/I4 与 #63 的 `Assigned xor Unassigned xor Tombstoned` 完全分类同型：`chanlun/review-results/unassigned-ledger-production-spec-20260713.md:56-74,481-498`。

## 3. 合法吸收机制

候选只有同时满足以下公共门才进入 `Candidates(k)`：

```text
C0 base_assignment[k] == None
C1 candidate.level == query.target_level
C2 all evidence.available_at <= query.as_of
C3 candidate membership proof contains exactly k as the adopted key
C4 candidate does not require changing any H0(x) for x != k
C5 candidate does not mutate/reopen a greedy_v1 Completed Move
C6 candidate carries stable host id, version, range, proof hash and judge_at
```

### 3.1 `CENTER_EXTENSION`

合法条件沿用 #58/#64：lower 与固定中枢核心 `[ZD,ZG]` 相交，`high(s)>=ZD && low(s)<=ZG`；只能扫描到下一 seed 或下一走势组前，首个不相交 lower 终止。不得用外围 `GG/DD` 接近或价格距离代替核心重叠。锚：`chanlun/review-results/orphan-reduction-paths-20260713.md:87-128`。

### 3.2 `TREND_ASSEMBLY`

候选 host 必须已有同方向 maximal run 与相邻同向中枢结构。唯一 host 一旦建立即可 Assigned；没有唯一 A/C + MACD completion proof 时仍 Pending。不得自动选择“最强背驰”或任意首尾 pair。锚：`chanlun/review-results/orphan-reduction-paths-20260713.md:130-162`。

### 3.3 `CROSS_WINDOW_RECOMPOSE`

候选事实可枚举合法连续三元组与非重叠选择，但在 `adopt_v1` 中只作为**给基座无 host lower 建立 host 的见证**：

1. 不替换 `greedy_v1` 已选窗口；
2. 不释放任何基座 assignment；
3. 不把候选 partition 写回塔或旧 Move；
4. 固定 t 只使用 t 时可见候选；
5. 若候选证明依赖释放任一基座 lower，则由 C4 拒绝。

#64 的旧 DP 目标、回退与建议的 `regressions=0` 优先级见 `chanlun/review-results/orphan-reduction-paths-20260713.md:164-225,335-346`。本层的关键差异正是把“替换整条 partition”改成“只采纳 H0=None 的 key”。

### 3.4 `TERMINAL_CONTINUATION`

尾段新增 lower 后若首次建立唯一 host，可在新 as-of 追加 assignment；当前末端、暂时无新 bar 或数据文件结束不能触发 tombstone。#64 的四个尾段当前已在 Pending host，通常不会给 task67 固定集带来增量：`chanlun/review-results/orphan-reduction-paths-20260713.md:247-264`。

### 3.5 明确排除

- #53 `CandDeltaEntryEvent` 不是 host/membership 事实，不能直接触发 ADOPT：`chanlun/review-results/orphan-reduction-paths-20260713.md:226-245`。
- `NO_CONTAINING_SEED_WINDOW` 只否定 seed 资格，不否定 host；1,333 中基线已有 1,281 个 host，见 `chanlun/review-results/seed-failure-geometry-20260713.md:10-16,196-212`。
- 任意邻近 host、最短价格距离、未来全历史最优、Completion 优先都不是合法性门。

## 4. 确定性顺序与平局裁决

合法候选先按以下稳定键升序；首项为唯一选择：

```text
(
  mechanism_rank,        // EXTENSION < TREND < RECOMPOSE < TERMINAL
  host_span,             // end-start，较紧者优先
  host.start,
  host.end,
  stable_host_ordinal
)
```

规则：

1. 先过滤 C0–C6，再排序；tie-break 不能把非法候选变合法。
2. Completed/Pending 不参与排序，防止为了可消费计数偏置结构归属。
3. `stable_host_ordinal` 必须属于已 pin 的候选策略版本；不得用 `Vec` 内存地址、HashMap 迭代顺序或文件行序。
4. 完全相同稳定键却 payload/hash 不同是 `ConflictingCandidateIdentity`，fail closed。
5. 原型的 `deterministic_tie_break_is_input_order_independent` 正反输入顺序结果一致，见 property test 结果 `/private/tmp/task67-adopt-overlay/task67-property-tests.log:2-4`。

## 5. 与 Unassigned 账本衔接

### 5.1 覆盖后 reducer

对每个 visible key：

1. `H0(k)!=None`：输出 `AssignmentRef::Base`；不得生成 ADOPT 事件。
2. `H0(k)==None && H1(k)!=None`：追加 `ASSIGNED / LEGAL_HOST_ESTABLISHED`；correlation 内携 `ADOPT_ORPHAN` attribution/host event。
3. `H1(k)==None`：输出 `ResidualLedgerEntry`；状态保持 `OPENED/REOPENED`，不得因 seed failure 自动 tombstone。
4. 任一 key 同时有 host 与 ledger、两 host、无 host 且无 ledger均为错误。

#63 的状态机、原子 correlation 与 assignment proof 要求见 `chanlun/review-results/unassigned-ledger-production-spec-20260713.md:228-277`。

### 5.2 残余成因标签

账本主 reason 沿用 #63 优先级：

```text
TERMINAL_INSUFFICIENT
NO_CONTAINING_SEED_WINDOW
LEADING_RESIDUAL
CURSOR_GAP_RECOMPOSABLE
NO_HOST_AT_AS_OF
```

机检判据见 `chanlun/review-results/unassigned-ledger-production-spec-20260713.md:279-298`。对 #65 的 exact-three 对象还必须附：

```text
first_failure = A_NO_COMMON_OVERLAP | B_INVALID_DIRECTION |
                C_CONTAINMENT_DEGENERATE | D_OTHER
geometry = GAP_ADJACENT | MONOTONE_STEEP | GEOMETRY_OTHER
risk = {
  S_PERMANENT_SEED_FOR_THIS_LOWER_AND_RULE_VERSION,
  H_BOUNDARY_OPEN,
  TOMBSTONE_FORBIDDEN
}
```

#65 固定集的 1,333 个对象全部为 `A_NO_COMMON_OVERLAP`，geometry 分布与 35 个风险清单见 `chanlun/review-results/seed-failure-geometry-20260713.md:44-66,92-117,147-194`。

### 5.3 本次 35/33 对齐

- 旧重组 35：与 #65 逐项完全相等；task67 log `:19-45,74-82,97`。
- 其中 `L0#33574`、`L0#37199` 在 greedy 基座有 host，只是旧重组把它们释放；#65 的 B/R 均为 `0/1`，见 `chanlun/review-results/seed-failure-geometry-20260713.md:179,182`，task67 拦截见 log `:40-45`。
- `adopt_v1` 账本：其余 33 个，机器输出见 task67 log `:46-68,83-91,98`。33 全部仍携 #65 的 `NO_CONTAINING_SEED_WINDOW + A + geometry + H-boundary-open + 禁止墓碑` 标签。

## 6. CompletedFreeze

同一 `ViewVersion` 下：

```text
forall base move m.
  m.completion == Completed
  -> bytes(m), chain(m), base_membership(m) remain unchanged
```

实施规则：

1. `adopt_v1` 不写 `MOVE_SUPERSEDE(RELEASED)`；它只有 add-only attribution。
2. 基座 `AssignmentRef::Base` 的 `HostRef`、move version 与 completion bytes 原样复制。
3. adopted lower 不触发基座 Completed 重开；候选 completion 只在同 as-of 的完整 proof 已存在且 adoption 不改变该 proof 时标 Completed。
4. 规则升级必须新开显式版本；旧 `greedy_v1/adopt_v1` 结果仍可按旧 tuple 重放。
5. 旧重组会释放 `L0#33574/L0#37199`，因此不能直接作为 `adopt_v1` reducer；task67 log `:40-45,103-104` 是 CompletedFreeze/单调门的具体反例。

CompletedFreeze 的正式建议与“普通 supersede 只可作用 Pending”边界见 `chanlun/review-results/c2-architecture-reassessment-20260713.md:88-124,143-179`。

## 7. as-of 语义与前缀稳定

### 7.1 可见域

```text
Visible_t(X) = stable_sort {
  x in X |
  W.start <= x.start <= x.end <= min(W.end,t)
  and x.settled_at <= t
  and x.version == pinned_version
}
```

所有 candidate evidence 与 assignment decision 还必须 `judge_at<=t`。未来事实可以物理存在，但对 t 完全不可见；这与 #58、#63 的可见域一致：`chanlun/review-results/assembler-spec-20260712.md:54-73`、`chanlun/review-results/unassigned-ledger-production-spec-20260713.md:338-374`。

### 7.2 硬门

```text
forall D, F, t.
  adopt(greedy(D[<=t], t), adopt_v1)
  ==
  adopt(greedy(D ++ F, t), adopt_v1)

where every fact in F is available/judged after t
```

等号覆盖 assignment map、base/adopted HostRef、账本 key/state/reason/geometry/risk、候选选择、host completion snapshot 与错误结果。不得只比较计数。

### 7.3 固定截点

| target k | cut / as_of | 结果 |
|---:|---|---|
| 1 | `10000/1106020`、`20001/2250635`、`30002/3431564` | 3/3 pass |
| 2 | `3016/1089157`、`6032/2228868`、`9048/3416055` | 3/3 pass |
| 3 | `889/1076894`、`1778/2229328`、`2667/3393706` | 3/3 pass |
| 合计 | 9 个 | **9/9 pass** |

机器锚：task67 log `:70-72,93-95,100-103`；#65 原截点见 `chanlun/review-results/seed-failure-geometry-20260713.md:236-250`。

## 8. Property tests

隔离源码在 `task67_adopt_overlay.rs` 内直接测试纯 reducer；不修改生产 crate 文件。

1. `monotonicity_preserves_every_base_host`
   - 生成不同 base/candidate 分布；对每个 `H0(k)!=None` 断言 `H1(k)==H0(k)` 且 `k` 不出现在 adopted map。
2. `coverage_is_total_and_disjoint`
   - 对所有可见 key 断言 `assignment xor ledger`，并断言计数守恒。
3. `prefix_stability_matches_58_hard_gate`
   - 同一 t 比较“含未来后缀的全输入”与“实际前缀输入”，比较 `AdoptOverlayView` 全字段。
4. `deterministic_tie_break_is_input_order_independent`
   - 候选反序后选择不变，并命中 §4 的稳定键。

结果：`4 passed / 0 failed`，见 `/private/tmp/task67-adopt-overlay/task67-property-tests.log:2-4`。

全历史 main 另有：1,333/52/19/7/35/2/33 守恒、35 fixture 全字段对齐、9 个真实截点和双 replay determinism 断言。汇总见 task67 log `:69-104`。

## 9. 固定截点量化结果

| k | exact-three 对象 | 基线 host | 基线无 host | adopt_v1 新增 | 其中 Completed | adopt_v1 账本 | 旧重组残余 | 旧回退 |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 767 | 727 | 40 | 17 | 5 | **23** | 25 | 2 |
| 2 | 353 | 343 | 10 | 1 | 1 | **9** | 9 | 0 |
| 3 | 213 | 211 | 2 | 1 | 1 | **1** | 1 | 0 |
| 合计 | **1,333** | **1,281** | **52** | **19** | **7** | **33** | **35** | **2** |

事实锚：task67 log `:69,92,99,103`。#65 只报告旧替换视图的 `52/35/19/7`，没有把 2 个回退单列到该表；其逐项 B/R 标签已足以定位这两例，见 `chanlun/review-results/seed-failure-geometry-20260713.md:68-90,147-194`。

新增 19 的机制分布为：中枢延伸 1、趋势组装 2、跨窗重组见证 16；task67 log `:2-18,73,96,103`。这些机制计数互斥是本原型按最高优先级给每个对象选一个标签后的结果，不可与 #64 的重叠机制总量直接相加；#64 已明确机制重叠：`chanlun/review-results/orphan-reduction-paths-20260713.md:19,123-128`。

## 10. 隔离实现与复现命令

### 10.1 边界

- 复用 `/private/tmp/task65-seed-geometry` 的 Rust/harness 副本；源码、target、日志均在 `/private/tmp`。
- 数据 cache 沿用 #58 隔离工作树；生产目录只读。
- `CARGO_TARGET_DIR=/private/tmp/c58-assembler-work/rust/target`。

### 10.2 测试与 release

```bash
cd /private/tmp/task67-adopt-overlay/rust

CARGO_TARGET_DIR=/private/tmp/c58-assembler-work/rust/target \
  cargo test --features backtest_bin --bin task67_adopt_overlay

CARGO_TARGET_DIR=/private/tmp/c58-assembler-work/rust/target \
  cargo build --release --features backtest_bin --bin task67_adopt_overlay
```

### 10.3 双重放

```bash
cd /private/tmp/task67-adopt-overlay

/private/tmp/c58-assembler-work/rust/target/release/task67_adopt_overlay \
  > task67-adopt-overlay.log 2> task67-adopt-overlay.err

/private/tmp/c58-assembler-work/rust/target/release/task67_adopt_overlay \
  > task67-adopt-overlay.verify.log 2> task67-adopt-overlay.verify.err

cmp task67-adopt-overlay.log task67-adopt-overlay.verify.log
shasum -a 256 task67-adopt-overlay.log task67-adopt-overlay.verify.log
wc -l task67-adopt-overlay.log task67-adopt-overlay.verify.log \
  task67-adopt-overlay.err task67-adopt-overlay.verify.err
```

结果：stdout 各 104 行、`cmp` 0、stderr 各 0 行。证据：`/private/tmp/task67-adopt-overlay/task67-replay.line-counts:1-5`。

## 11. 生产晋级门

只有以下全部成立，`adopt_v1` 才能从隔离原型进入生产候选：

1. P0 明确修正固定验收口径：推荐把 overlay residual 从 35 改为 33，并保留 `legacy_recompose_residual=35` 诊断字段。
2. 生产 `GreedyAsOfView` 在 #63 单 seam 内提供 `settled_at`、完整 base disposition 与候选 proof；无旁路 latest/全历史读取。
3. 对全部 `VisibleLower`（不只 1,333 fixture）通过 I1–I6 与 `Assigned xor Unassigned xor Tombstoned`。
4. 每个 adopted host 有可验证 `HostMembershipProof`、稳定 ID/version、judge_at 与 correlation；半批次不可见。
5. 基座 Completed bytes/chain/membership hash 在追加 overlay 后不变；规则迁移另起版本。
6. 全部 commit/selection 边界通过 prefix property，不止本次 9 个样本；#66 候选生产 Gate P1–P5 见 `chanlun/review-results/c2-architecture-reassessment-20260713.md:388-400`。
7. `entry_bar >= judge_at+1`、旧行 append-only、consumer pin version 全过；#63 共同硬门见 `chanlun/review-results/unassigned-ledger-production-spec-20260713.md:646-656`。

**[未完成]** 第 1 项尚未裁定；本报告不得被解读为已把 `adopt_v1` 接入生产消费者。

## 12. 生产区干净性与双重放 SHA-256

执行：

```bash
git status --short -- rust formal .chanlun/definitions
```

结果：无输出；task67 未修改三处生产区域。

双 release 重放 SHA-256 完全一致：

```text
7314884cf39dc991a4d2db368890d5b1ff3a9f959fafc40c552a7a266bad7684  task67-adopt-overlay.log
7314884cf39dc991a4d2db368890d5b1ff3a9f959fafc40c552a7a266bad7684  task67-adopt-overlay.verify.log
```
