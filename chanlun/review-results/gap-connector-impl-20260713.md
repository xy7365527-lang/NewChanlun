# gap-connector 实装报告（task #72，`adopt_v2`）

- 日期：2026-07-13
- 状态：**隔离 Rust 原型、9 条 property tests、33 对象逐项重放、双 release bit-exact 均通过**
- 原型：`prototypes/task72-gap-connector/`
- 基座：task #68 账本 reducer + task #67 `adopt_v1` 固定 `52→19→33` fixture
- 写入边界：新增独立 crate 与本报告；生产 `rust/src`、`formal`、`.chanlun/definitions` 零改动

## 0. 结论

`adopt_v2` 在 `adopt_v1` 的 33 个 Unassigned 固定域上通过全部硬门：

```text
R2' disabled: 33 = 19 adopted_v2 + 14 residual_v2
R2' enabled:  33 = 30 adopted_v2 +  3 residual_v2
```

- 跳空 19：全部由 R1 放行 seed 阻断，再由 R2 按分解链坐标挂到其后相邻既有 host；新增 19 条 `ADOPT_ORPHAN`。
- 单调陡峭 11：只有独立 R2' 开关开启时评估；全部由同一个 R2 connector 通道采纳；新增 11 条 `ADOPT_ORPHAN`。
- 其他 3：`L0#4869`、`L0#20299`、`L0#35944`，保持 `Unassigned`，reason=`GEOMETRY_OTHER_OUT_OF_SCOPE`。
- R3、R4 已实现并由 property test 覆盖；固定 33 对象没有需要走 R3 的中枢重叠证据，也没有序列起点对象，因此固定重放计数均为 0，未强行制造命中。
- 新增事件共 30 条，全部是 `ASSIGNED(cause=ADOPT_ORPHAN)`；release=0、degrade=0、tombstone=0。

机器总锚：`prototypes/task72-gap-connector/artifacts/replay-1.txt:1-10,44-45`。

## 1. 权威边界与实现映射

实现保持 `gap-connector-spec-20260713.md` 的规则层次：

1. **R1 跳空豁免**：`GAP_ADJACENT` 不再被 `NO_CONTAINING_SEED_WINDOW` 阻断；R1 只移除 seed/三折否定，不凭空制造 host。原文锚：chan99/0046:85,171。
2. **R2 连接段采纳**：要求左右两个同目标级别、按坐标相邻的已组装 host；归属策略固定为分解链坐标序中的后继 host。该策略只引用既有 host，不创建新 host。原文锚：blog/028:308,746。
3. **R3 震荡吸收**：R2 不成立时，只有 `segment_high>=ZD && segment_low<=ZG` 且 center host 同级才采纳；R2 优先级高于 R3。原文锚：blog/036:34。
4. **R4 序列起点**：只有 `sequence_start=true`、无左 host、有合法右侧首 host 才成立。原文锚：blog/036:128-131。
5. **R2' 小转大**：独立布尔开关；先验证“连续同向、无重叠”，再复用 R2→R3 通道，不进入 R4，也不计入跳空桶。原文锚：chan99/0027:21、思维导图:243-244、blog/028:754；跳空极限情形锚：chan99/0046:95。

核心实现锚：

- 规则求值与 R2→R3→R4 / R2'→R2→R3 优先级：`prototypes/task72-gap-connector/src/lib.rs:1228`
- v2 append-only reducer、单调中止和守恒断言：`prototypes/task72-gap-connector/src/lib.rs:1317`
- 33 对象 typed fixture：`prototypes/task72-gap-connector/src/lib.rs:1493`
- `L0#37199` 保留判例：`prototypes/task72-gap-connector/src/lib.rs:1944`

若检测到事件前缀改变、非 `ADOPT_ORPHAN` 扩展、已有 Assigned host 改配或 release，reducer 返回 `UnsatisfiableWithMonotonicity`；CLI 向 stderr 输出精确字符串 `UNSATISFIABLE_WITH_MONOTONICITY` 并以 exit 2 中止。没有降级 fallback。

## 2. 逐桶统计

| 桶 | 对象 | R2' 关：adopted/residual | R2' 开：adopted/residual | 最终通道 | 结论 |
|---|---:|---:|---:|---|---|
| `GAP_ADJACENT` | 19 | 19 / 0 | 19 / 0 | R2=19，R3=0，R4=0 | PASS |
| `MONOTONE_STEEP` | 11 | 0 / 11 | 11 / 0 | R2=11，R3=0 | PASS，独立开关 |
| `GEOMETRY_OTHER` | 3 | 0 / 3 | 0 / 3 | 无 | 保持 Unassigned |
| **合计** | **33** | **19 / 14** | **30 / 3** | 新增 30 条 ADOPT | PASS |

机器锚：

- R2' 关/开两套守恒：`prototypes/task72-gap-connector/artifacts/replay-1.txt:2-3`
- 三桶统计：同文件 `:4-6`
- 事件安全与最终守恒：同文件 `:8-9`

最终账本为 49 Assigned / 3 Unassigned / 0 Tombstoned；其中 49 是 `adopt_v1` 已有 19 加 `adopt_v2` 新增 30，不能把它误写为 v2 固定域分母。

## 3. 33 对象逐项输出与残余归因

33 个对象按 `(level, ordinal)` 稳定排序逐项输出：

- `prototypes/task72-gap-connector/artifacts/replay-1.txt:11-43`
- adopted 行包含 `bucket / event=ADOPT_ORPHAN / rule / channel / R1 exemption / host range / judge_at`；
- residual 行包含 `bucket / reason / disposition=UNASSIGNED`。

残余只剩 3 个“其他”桶对象：

| lower | range | residual reason | disposition |
|---|---|---|---|
| `L0#4869` | `[542428,542539]` | `GEOMETRY_OTHER_OUT_OF_SCOPE` | Unassigned |
| `L0#20299` | `[2286677,2286887]` | `GEOMETRY_OTHER_OUT_OF_SCOPE` | Unassigned |
| `L0#35944` | `[4132879,4133078]` | `GEOMETRY_OTHER_OUT_OF_SCOPE` | Unassigned |

机器逐项锚分别为 replay `:13,20,32`。三者没有被伪 Completed、没有墓碑、没有用距离或任意邻近 host 强吸。

## 4. `L0#37199` 显式判例

`L0#37199` 不属于 33 个 `adopt_v1` residual；它在 greedy/adopt_v1 基座已经 Assigned，旧 DP 才错误释放它。因此 v2 的正确动作是 **NO_EVENT + 保留原 host**，而不是再发一条 ADOPT。

固定证据：

```text
lower:        L0#37199 [4277801,4277866]
geometry:     GAP_ADJACENT
base host:    [4276595,4280151], PENDING_UNDETERMINED
inner chain:  [4277222,4277751] | gap | [4277866,4279768]
legal basis:  R1 + R2
```

合法性解释：R1 明确使该 gap 成为无内部结构的合法最低级别连接段，故 `NO_CONTAINING_SEED_WINDOW` 不能否定它；R2 的内部链坐标证明 gap 位于同级分解链相邻成分之间，且整个内部链落在既有 base host 范围内。故该 host 本来就合法，单调门必须保留；旧 DP 的 release 没有合法拒绝规则。

机器锚：`prototypes/task72-gap-connector/artifacts/replay-1.txt:10`。

## 5. 统一机理

固定重放结论：**同构成立**。

- 跳空桶实际路径集合：`{R2}`；
- 单调陡峭桶在 R2' 开启后的实际路径集合：`{R2}`；
- 两类对象由同一 evaluator 进入允许的 `R2/R3` connector 通道，固定集路径集合完全相等；
- property test 另用 R3-only 合法证据验证两桶都能走 R3，排除“只是两个独立硬编码 R2 列表”的伪同构。

因此本固定域支持 spec 假设：跳空与单调陡峭都是小转大/级别跃迁事件；跳空是级别无限低的极限情形。两者的 seed 失败不是终态无归属证据，而是同级结构缺席时需要转入级别以下 connector 通道的预期表现。

机器锚：replay `:7`；property test `prototypes/task72-gap-connector/artifacts/property-tests.txt:18-19`。

## 6. Property tests

共 9 条，`9 passed / 0 failed`：

1. `prop_monotonic_preserves_every_adopt_v1_assignment`
2. `prop_no_release_or_degrade_event_can_be_emitted`
3. `prop_event_stream_is_strictly_append_only`
4. `prop_judge_at_lte_t_is_the_only_visible_candidate_domain`
5. `prop_33_is_conserved_and_host_ledger_are_disjoint`
6. `prop_r2_prime_switch_is_independent_and_reported_separately`
7. `prop_gap_and_monotone_paths_are_isomorphic_over_r2_r3_channels`
8. `prop_r1_r4_and_r2_over_r3_precedence_are_fail_closed`
9. `prop_full_replay_is_idempotent_and_sha256_bit_exact`

测试源码：`prototypes/task72-gap-connector/tests/properties.rs:48-304`；机器结果：`prototypes/task72-gap-connector/artifacts/property-tests.txt:17-28`。

## 7. 双 release 重放

两次 `cargo run --release --quiet` 均从同一固定 71-event v1 源与 33 个 connector facts fresh 构造：

| 项 | replay 1 | replay 2 |
|---|---:|---:|
| stdout 行数 | 45 | 45 |
| stderr 行数 | 0 | 0 |
| 完整 stdout SHA-256 | `17beb42d939f9e5b6f1a7ea07fc2c23285bc66bf5c48294da210b8e370d89eb6` | 相同 |
| terminal-state SHA-256 | `d59fa6c850863a66c49fd1a2fe93da4c4051960bc5640bd08c5ebdeac4aa0ac6` | 相同 |
| `cmp` | exit 0 | bit-exact |

机器锚：

- stdout SHA：`prototypes/task72-gap-connector/artifacts/replay.sha256:1-2`
- 行数/stderr：`prototypes/task72-gap-connector/artifacts/replay.line-counts:1-5`
- terminal hash 与 duplicate replay：`prototypes/task72-gap-connector/artifacts/replay-1.txt:44`

## 8. 复现命令与边界说明

```bash
cd prototypes/task72-gap-connector
cargo fmt --check
cargo test
cargo run --release --quiet > artifacts/replay-1.txt 2> artifacts/replay-1.err
cargo run --release --quiet > artifacts/replay-2.txt 2> artifacts/replay-2.err
cmp artifacts/replay-1.txt artifacts/replay-2.txt
shasum -a 256 artifacts/replay-1.txt artifacts/replay-2.txt
```

本次“重放”严格指：复用 task67 已由 4,613,599 bars 冻结的 52→19→33 量化结果，以及 task65 的 33 条真实 geometry/range/相邻 host 坐标，重放 task68 账本事件与新增 v2 connector facts；本任务没有再次运行整条 bars→tower→MoveView 生产流水线，也没有把原型接入生产消费者。fixture host ID 由 `(target level, host start)` 稳定派生，host range 来自 task65 机器行；生产晋级仍须由唯一 `assemble_level_view` seam 提供正式 host ID/version/membership proof。

未发现任何需要 release 或降级的对象，因此没有触发 escalate 草稿；若未来正式 host proof 与本 fixture 冲突且只能靠释放解决，必须输出 `UNSATISFIABLE_WITH_MONOTONICITY` 并中止，不能放宽本报告门禁。
