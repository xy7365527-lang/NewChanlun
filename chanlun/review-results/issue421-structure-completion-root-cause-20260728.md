# #523：#421 结构完成根因调查（BTC 100k，2026-07-28）

> 角色：researcher（只读调查）
> 基线：`kimi-nest-mainline-20260717` @ `391936a486a3b7404b6992a39cdb9f6594626570`
> 结论口径：本报告不改生产代码，不改 `Cargo.toml`，不改 #454/#497，不代替编排者裁定。

## 执行摘要

**根因三选一：bug。** 更精确地说，是 **PanLive provider 接缝/能力缺口**，不是 BTC
行情让结构在一根 bar 内完成，也不是状态机本身 off-by-one：

1. 当前所谓 `PanLive` 并没有读取“行进中的 C 段”。它从递归塔的 lower legs 转成
   `segments`，再调用与完成 `NestCandidateEvent` **同一套**盘整结构定位条件。
2. lower legs 在 L1 来自 parser 的 confirmed `segments`；pending 段另存于
   `pending_start`/`tail`，没有进入这条调用链。L2/L3 的 `LeveledMove` 也没有
   Active/Completed 字段。因此，PanLive 身份最早只能在完成候选也已可构造时出现。
3. `feed_replay_bar` 又把所有 `kind == Consolidation` 的 `NestCandidateEvent` 无条件包装成
   `structure_completed=true`。同一 bar 先喂同源 live、再喂 completion，状态机只能诚实地产生
   `Observed → StructureCompleted → terminal` 的零寿命闪现。

BTC 100k 的 248 只身份全量对拍证明：`p123.observed_at = p409.observed_at =
completion_as_of`，248/248，差值 min/max=`0/0`。但这些身份的完成 C 段端点差
`c_end-c_start` 为 **16 / 135 / 3028**（min/median/max），而完成信号比该 C 段端点还晚
**19 / 57.5 / 5190** bar。故“结构只活一根 bar”的数据解释被直接排除：**零寿命发生在
观测接缝，不发生在价格结构本身。**

p409 的 `structure_completed=false` 不是在生产完成前观察同一身份；它从生产完成首见 bar
才开始观察，然后反事实地继续延展。它得到的 113 个 ForceOvertake 是“若真实完成后仍不结算”
的 holding counterfactual，不能再充当生产 PanLive 生命周期的正面 oracle。

对 #421 的建议是 **新选项 E（D + A 的拆票版）**：

- 当前接线只可重定向并诚实命名为“**当前 provider 首次可见即完成的闪现主导结算账本**”，
  按 D 交付已实现的零丢弃/当场结算能力，同时按 A 撤掉“p409 反超主导必须在生产复现”的 oracle；
- 另开上游 bug/spec 票，补“完成前真实 PanLive provider”，先做 L1-only 最小 prototype；
- p409 保留为独立的“完成后若继续持有”反事实分析，不再与生产账本混称；
- 若 #421 的原 destination 仍要求真正的活假设生命史，则在 provider 修复前应保持 blocked，
  不能用延迟/忽略完成信号（选项 B）造寿命。

## 调查方法

### 1. 基线与权威输入

调查前核验 HEAD 为 `391936a486`，并逐项阅读：

- #421 comment-5103436732（100k 实测与 A/B/C/D）、comment-5106036959（编排者选 C）；
- #523、#409、#402 题一选项 c；
- commit `391936a486` 全文；
- `chanlun/review-results/issue421-acceptance-selfcheck-20260727.md` §9；
- R43 申请、裁定和生命周期状态机实装报告；
- ADR-0003（从 commit `e45e5020a9` 读取；本 worktree 当前没有 `docs/adr/`）。

### 2. 代码路径追踪

从 `NestLifecycleBook::advance` 反向追到 `feed_replay_bar`、p123 的逐 bar 循环、完成事件
provider 和 PanLive provider；再向上追 parser confirmed segment、pending tail 和递归塔
`LeveledMove` 的数据能力。代码发现优先使用项目知识图谱，行锚再以当前 HEAD 源文件核对。

### 3. 回放与逐身份 join

100k 主证据复用 commit `391936a486` 同码产物：

- `/tmp/wt421e-final-100k-lifecycle.dump`
  SHA-256 `3df5fbc732514929fcde7c6db6298b40539ca338e8af799ea0de58ac729db71c`；
- `/tmp/wt421e-p409-btc100k-entries.jsonl`
  SHA-256 `28290ceff9aaa502d7d804be975e9f39fee9637984dda3a58b0d330033a5ff5c`；
- `/tmp/wt421e-first-vs-completion.tsv`
  SHA-256 `e7410cab2cf3bc1f267f803278e4ec7fbf8da5f9e65639082eb53580d583b1b8`。

另在当前 HEAD 独立重跑 20k 缩短反馈环：

```bash
cd rust
P116_MAX_BARS=20000 P116_CKPT=0 \
  P421_LIFECYCLE_DUMP=/tmp/issue523-p123-20k.dump \
  ./target/release/p123_fast_replay ../analysis/data_cache/btc_1m_full.json

P409_MAX_BARS=20000 P409_PROGRESS=0 \
  P409_DUMP=/tmp/issue523-p409-20k.jsonl \
  ./target/release/p409_pan_live_probe ../analysis/data_cache/btc_1m_full.json
```

20k 独立复现为：p409=`46`、completion=`46`、joined=`46`、missing=`0`、
nonzero delta=`0`，即 46/46 仍同 bar；p123 为 46/46 闪现、非闪现 0。

100k join key 取与工程桥一致的稳定分量：

```text
(level, side, seg_a, c_start, b_center_start)
```

即故意不把会随活窗延展的 C 右端放进身份键；这与
`bridge_identity` 的实际比较字段一致。完整 248 行结果见同目录：
`issue421-structure-completion-identities-20260728.tsv`。

## 关键证据

### 1. 248 身份的数值证据

| 证据 | 100k 结果 | 判读 |
|---|---:|---|
| 身份类型 | L1=202，L2=38，L3=8；全部 `Consolidation` | 没有 bi/trend 身份；是盘整背驰候选 |
| `p123.observed_at = p409.observed_at = completion_as_of` | 248/248（100%），差值 min/max=0/0 | live 首见与完成首见是同一接缝 |
| C 段端点差 `completed_seg_c_end-c_start` | min/median/max=16/135/3028，零值=0 | 行情结构并非“一根 bar 完成” |
| `completion_as_of-c_start` | 52/238.5/6853 | 从 C 起点到生产完成信号有真实跨度 |
| `completion_as_of-completed_seg_c_end` | 19/57.5/5190，248/248 > 0 | completion 甚至晚于记录的完整 C 端点；不是过早一根 |
| p409 在 production completion 后仍观察 | 247/248 > 0；min/median/max=0/3791.5/97952 | p409 主要观察的是完成后的反事实延展 |
| p409 终局 | Force=113，IdentityVanished=20，prefix 截止仍 Provisional=115 | 113 Force 来自禁用完成后的 holding 路径 |

类型字段需按实际代码能力解释：

- L1 202 只：`PanConsolidation/L1(lower=parser-Segment)`，lower substrate 是 parser
  confirmed 线段；
- L2 38 只：lower substrate 是 L1 `LeveledMove/WindowUnit`；
- L3 8 只：lower substrate 是 L2 `LeveledMove/WindowUnit`；
- `Center` 只作为 `b_center_start` 身份锚，不是这 248 条记录的事件类型；笔也不直接进入此
  provider 输入。

### 2. 代码链与行锚

| 环节 | 当前 HEAD 行锚 | 决定性事实 |
|---|---|---|
| p123 逐 bar 与 `forest_epoch` | `rust/src/bin/p123_fast_replay.rs:942-954` | 每 bar 都推进；epoch 变化才重算 stem |
| p123 双通道 feed | `p123_fast_replay.rs:1198-1239` | 同 bar 取 completion events、扩当前 live window 后一起 feed |
| PanLive 上游 | `p123_fast_replay.rs:1352-1387` | 从 `tower[level-1]` 的 lower legs 转 `segments`，再投影/分解/产窗 |
| PanLive 条件 | `rust/src/theta_v0/classifier/nest_lifecycle.rs:1142-1192` | 遍历已在 lower set 的 segment，要求 Consolidation + 同一 locate/extreme 条件 |
| completion Event 条件 | `rust/src/theta_v0/classifier/level_view.rs:827-903` | 同样遍历 segments、同样 Consolidation + locate/extreme，然后产 `NestCandidateEvent` |
| 实际“完成判定” | `nest_lifecycle.rs:1344-1350` | **仅以 `event.kind == Consolidation` 就包装 `event(..., true)`** |
| 同身份桥 | `nest_lifecycle.rs:136-153,1366-1371` | 忽略 C 右端；同源 Event/live 立即匹配 |
| 状态机结算 | `nest_lifecycle.rs:789-928` | 新建 `Observed` 后看到 `structure_completed` 即当 bar 结算；行为符合输入契约 |
| confirmed 与 pending 分离 | `rust/src/theta_v0/parser/segment.rs:461-475`；`parser/mod.rs:274-294` | `segments_rc` 是 confirmed 前缀；未完成段在 `pending_start`/`tail`，本调用链未消费 |
| 高级别对象能力 | `rust/src/theta_v0/classifier/recursive_tower.rs:109-123` | `LeveledMove` 无 Active/Completed 字段，不能表达真正的活动 lower leg |
| p409 反事实开关 | `rust/src/bin/p409_pan_live_probe.rs:253-254,276-348` | 同一 stem producer，但固定 `structure_completed=false` 后继续逐 bar 延展 |

`forest_epoch` 不是根因。p409 的 `P409_VERIFY=1` 路径会在 epoch 不变时也强制重算并逐值
比较（`p409_pan_live_probe.rs:281-309`）；既有验证 `verify_mismatches=0`。本轮 20k 直接 join
也仍为 46/46 同刻。换句话说，逐 bar 循环没有漏掉一个已经存在但未喂入的 live 对象；上游根本
没有在完成候选之前产出该对象。

### 3. 三选一归类

| 候选 | 判定 | 证据 |
|---|---|---|
| 数据特性 | 排除 | 每只 C 的端点差都 > 0，min=16；完成信号距 C 起点 min=52，且距 C 端点还至少 19 bar |
| 结构定义天然过急 | 排除 | R43 裁定明确授权“pan 行进中对象”合法存在（`chanlun/escalate/r43-lifecycle-ruling-20260721.md:31-39`）；ADR-0003 要的是 PanLive→Event 数据源切换，且明确两通道窗口不同（`git show e45e5020a9:docs/adr/0003-live-hypothesis-structure-completion.md:1-3,14-16`） |
| bug | **成立** | 参照 PanLive producer 与 Event producer 使用同一已完成 lower-leg 集、同一定位谓词；随后 generic Consolidation event 被无类型来源证明地当作完成 |

这里的 bug 不是 `NestLifecycleBook::advance` 提前 return，也不是把 `as_of` 加减一能修的
off-by-one。状态机收到 `structure_completed=true` 后当场结算是正确行为。真正错误在
**provider 边界的声明与能力不一致**：

- 2026-07-20 实装报告已经诚实登记“交付状态机核心 + 消费契约，不是生产接线；PanLive
  production 需要 provider 暴露面，由调用方喂入”
  （`chanlun/review-results/v3-lifecycle-statemachine-impl-20260720.md:60-66`）；
- 2026-07-21 R43 又正式授权 pan 行进中对象；
- 后续接线却把“从已完成对象重建并把右端延到 as_of”的参照 helper 当成了真实 PanLive
  provider。它能制造延展窗口，却不能制造**完成前**的观察历史。

还有一个次级类型安全缺口：`NestCandidateEvent` 本体没有“这是首次完成事件”的 provenance，
`feed_replay_bar` 只能用 `kind == Consolidation` 代替完成证明。即使先补显式 completion
wrapper，若仍没有 active C 数据源，248 只仍会闪现；故它是必要边界修复，但不是充分修复。

## 逐身份元数据

机器可读附表：
`chanlun/review-results/issue421-structure-completion-identities-20260728.tsv`
（248 data rows，SHA-256
`81bd478fd00adf0238a7a7e96798eda758c257186dbb6bbe66fd16cb248073d2`）。

每行包含：

- 派生类型、level、side、`seg_a`、`c_start`、`b_center_start`、kind；
- p123 `observed_at`、p409 `observed_at`、production `completion_as_of`；
- 完成 Event 的 `seg_c` 右端及三组跨度；
- p409 最后一次观察 bar、相对 production completion 的额外观察长度、终局/原因/钟；
- 实际完成分支：
  `level_view::provide_nest_candidate_events_ext/consolidation-pan ->
  feed_replay_bar(kind==Consolidation)`。

全部 248 行的完成分支相同。当前事件载荷不记录窄锚
`locate_pan_div_structure` 与 A′ fallback 二者究竟命中哪一臂；这不影响完成根因，因为两臂
都先收敛成同一个 `NestCandidateEvent(kind=Consolidation)`，真正把它标成完成的是后续
`kind==Consolidation` 分支。

## 根因归类与论证

因果链可以压缩为：

```text
parser confirmed segments / completed tower units
                │
                ├─ 同一 locate + extreme ─→ “PanLive” stem
                │                              （最早此刻才出生）
                └─ 同一 locate + extreme ─→ Consolidation Event
                                               │
                          kind==Consolidation ──┴─→ structure_completed=true
                                                       │
                                        同 bar Observed→terminal
```

所以“248 只全部首见即完成”是代码拓扑上的近似恒等式，不是 BTC 100k 的偶然统计：

```text
first_visible(PanLive identity)
    = first_visible(completed candidate substrate)
    = first_visible(Consolidation Event)
    = completion_as_of
```

数据只决定有多少身份、在哪些 bar 触发；在当前接线下，它几乎不能制造
`observed_at < completion_as_of`。唯一可能的例外只能来自两条同源扫描暂时不同步、缓存失配或
桥键不一致，而 248/248 全量 join 与 `verify_mismatches=0` 又把这些偶然例外排除了。

## 对 #421 的建议

### 1. 原定义下能否继续

**当前 provider 能力下不能。** 如果 #421 的 destination 是“从完成前的活假设开始，记录
Provisional→FirstProvable→ForceOvertake/完成结算的真实生命史”，则缺少第一段输入，验收
不能靠 state machine 或 feed cadence 补出来。

禁止以下伪修：

- completion 延迟到下一 bar/下一 trigger；
- 同 bar completion 丢弃；
- 把 p409 `structure_completed=false` 接入生产；
- 把 `observed_at` 回填到 `c_start` 或 `seg_c.end`；
- 仅把 `as_of` 加一/减一；
- 未证明语义即拿 `MoveBlock.status` 当 segment completion。

这些办法要么造未来/回填，要么改结算教义，要么只掩盖输入缺失。

### 2. 建议裁定：新选项 E（D + A，拆分真实能力）

1. **#421 当前切片按 D 收窄 destination**：登记为“当前 provider 首次可见即完成的闪现主导
   结算账本”；交付它真实具备的逐 bar feed、零丢弃、完成即结算、护栏和审计能力，但不得再
   声称已经生产化“完成前活假设”。
2. **按 A 修 oracle**：p409 的 91.1% Force 从生产验收移出，单列为
   `post-completion holding counterfactual`。它仍可回答“若不在真实完成时结算，后来有多少
   反超”，但不是生产真值。
3. **新开 upstream bug/spec 票**：实现真正的 PanLive provider 后，再决定是重开 #421
   原 destination，还是作为下一阶段接线票。若编排者坚持 #421 本票必须包含真实生命史，
   则 #421 保持 blocked 直至该票完成。
4. **不选 B**：真实 completion 的当场结算没有错；修复应让 live 更早、不是让 completion
   更晚。

### 3. 最小 prototype（伪代码，不落地）

先只做 L1，验证 producer seam；不要一次扩到 L2/L3：

```rust
enum PanProviderPhase {
    // 必须来自 parser 当前 pending/tail，而非 confirmed segments 的回放重建。
    Live(PanLiveWindow),
    // 只有 lower unit 真正进入 completed set 时才产生。
    Completed(PanCompletionEvent),
}

struct PanCompletionEvent {
    event: NestCandidateEvent,
    completed_lower_id: ElementId,
    completed_at: usize,
}

fn provide_l1_pan_phases(
    confirmed: &[Segment],
    active_tail: &Tail,
    as_of: usize,
) -> Vec<PanProviderPhase> {
    // 1. 用 confirmed A/B 锚 + active C frontier 产稳定 c_start 的 Live。
    // 2. active C 进入 confirmed 后，停止 Live，并以同一 identity 产 Completed。
    // 3. 不回填、不延迟；同 bar 顺序仍是 live phase 后 completion phase。
}
```

最小验收不预设“必须 113 Force”，而应锁定语义：

```text
对每个 Completed identity：
  要么存在 earlier Live，observed_at < completion_as_of；
  要么有明确、可审计的 true-flash 原因（该对象确实同 bar 出生并完成）。

禁止：
  generic NestCandidateEvent(kind=Consolidation) 自动等价于 Completed；
  完成后继续延展 live；
  为满足寿命统计而推迟 completed_at。
```

L1 prototype 可覆盖本样本 202/248 的身份，足以验证根因。L2/L3 需要先定义递归塔的 active
lower-frontier 暴露面；当前 `LeveledMove` 没有完成状态，硬从 completed tower unit 外推会重演
同一 bug。完成事件也应成为显式 typed output，而不是消费方按 `kind` 猜 provenance。

## 遗留问题

1. **L2/L3 活动 lower unit 的正式载体**：是给 provider 增加只读 frontier view，还是给递归塔
   增加独立 active sidecar；不得污染已完成塔和证书真值路径。
2. **稳定身份**：L1 pending C 进入 confirmed 后，如何以 `ElementId`/结构锚证明 live 与
   completed 是同一身份，并退役仅忽略右端的工程桥。
3. **完成钟 provenance**：显式区分“lower unit 物理完成 bar”“完成 Event 首次可见 bar”
   和“账本收到 bar”；本样本三者当前并不相等。
4. **窄锚 vs A′ fallback 诊断**：现有事件载荷没有该 provenance。若后续需要分支占比，应加
   只写诊断 sidecar；它不影响本次根因判定。
5. **跨品种外推**：当前 bug 是结构性的，但身份数量/级别/反事实反超率仍是数据相关量；修复后
   应在 BTC 与至少一个非 BTC 窗复跑，不能把 248 或 91.1% 写成规格常量。
6. **p409 截止删失**：115 个 `Provisional` 的最后观察 bar 为 99999，表示前缀截止仍存活，
   不是自然终局。

## 只读/改动声明

未修改任何生产代码、`Cargo.toml`、#454/#497、roster、`econ_positive.rs`、shadow 报告或
issue/map 状态；未创建 throwaway branch。仓内新增仅为本调查报告与 248 行机器可读证据表。
