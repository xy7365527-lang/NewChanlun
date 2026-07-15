# 严格 C2 全量产量重测（task #83）

- 日期：2026-07-15
- 分支：`task-83-yield-remeasure`
- 基线：`main@001313ea42919a0bb868ab76c1f3c7a9d8369c1d`
- 性质：只读生产 seam 测量与报告；不改塔、不改信号、不执行裁决

## 0. 结论先行

**【已验证】2026-07-10 所见“严格 C2 / 完整区间套链条 0 产量”已经解除。**

在 `btc_1m_full.json` 全量终点 `as_of=4,613,598`（最后数据时点 `2026-05-31 23:59:00`）上，L1–L5 合计得到：

- `2,289` 个 CompletedMove；D5 adapter 实际追加 `2,289` 个 CompletedFreezeEvent；
- `1,092` 个严格 C2 pair（两个不同、正向相邻且均 Completed 的 Move）；
- `157` 条贯穿 L1–L5 的完整结构嵌套链；最大连续深度为 `5`；
- CompletedMove 方向分布为 Trend(上) `159`、Trend(下) `515`、Consolidation(None) `1,615`；
- Pending `1,410`，其中 `MissingDivergencePair=21`；Unassigned `215`。

这组结果证明 D1 投影、D2 自动配对、D3 方向、CompletedFreeze 与严格资格门在全量数据上已经形成非零闭环。它是产量测量，不是对 `firstRetrace`、区间套交易含义或任何遗留语义作新裁决。

## 1. 输入与测量口径

### 1.1 数据与唯一生产 seam

- 输入：`analysis/data_cache/btc_1m_full.json`，`4,613,599` 根 BTC 1m bar，原始数据 SHA-256：`16ea13d55f2ae7edcfc503f604a14961fd1a378227afd37c63f23386e894707b`。
- 数据范围：`2017-08-17 04:00:00` 至 `2026-05-31 23:59:00`；终点 `as_of=4,613,598`；parser 合并后 `3,037,868` 根。
- 流程：`parse_layer → classify_with_tower → project_extended_windows → decompose → assemble_level_view → CompletedFreezeAdapter.observe`。探针调用与版本 pin：`rust/src/bin/p83_yield_remeasure.rs:135-188,328-383`。
- 完整 version tuple：
  - `direction_provider_version=central-ggdd-v1`
  - `divergence_pair_provider_version=move-block-ac-v1`
  - `projection_provider_version=extended-to-exact-three-v1`

生产 tuple 定义及 fail-closed 校验：`rust/src/theta_v0/classifier/level_view.rs:28-101`；生产唯一 seam：同文件 `:471-586`。

### 1.2 D1 全量扫描与 Unassigned

生产 D1 对非法 exact-three seed fail closed。为了全量消费而不跨失败点伪造对象，探针对每个塔窗口调用同一个 `project_extended_windows` 判定；非法窗口计入 Unassigned，并成为硬断点；仅在两侧最大连续合法 run 内再次调用完整 D1 投影和 `assemble_level_view`。实现：`rust/src/bin/p83_yield_remeasure.rs:276-325`；D1 实现：`rust/src/theta_v0/classifier/level_view.rs:254-321`。

本报告的 Unassigned 定义为：

```text
D1 projection failure
+ 已投影 seed 中未被任何 AssembledMove.center_indices 覆盖者
```

本轮 `215 = 215 + 0`；没有把非法 seed 静默丢弃，也没有跨非法 seed 拼接 Move。

### 1.3 严格 C2 pair

严格 pair 定义为同一连续 run 的 `view.moves.windows(2)` 中，两个不同、正向相邻且均为 Completed 的 Move。统计实现：`rust/src/bin/p83_yield_remeasure.rs:387-413`。这与既有 D7 原语“不同、正向相邻、已完成”一致：`rust/src/theta_v0/classifier/first_retrace_replay.rs:31-55`。

### 1.4 完整区间套结构链

本任务采用可复算的结构口径，不增补交易语义：

1. 每个严格 pair 的定位区间为 `[leave.start_index, retest.end_index]`；
2. 相邻级别 L(k+1)→Lk 的边成立，当且仅当下级 pair 区间闭包含于上级 pair 区间；
3. “完整链”要求 L1–Ln 每级恰取一个严格 pair，且每条相邻级别边均成立；本轮 `n=5`；
4. 闭包含直接复用生产 `nest::is_sub`：`inner.start>=outer.start && inner.end<=outer.end`，锚 `rust/src/theta_v0/classifier/nest.rs:61-66`；计数动态规划实现于 `rust/src/bin/p83_yield_remeasure.rs:455-516`。

该口径只回答“严格 C2 pair 是否已经具备跨级闭包含结构”，不把这 `157` 条链重述为 `Chi::is_confirmed`、BSP 终端确认或交易信号；见遗留待裁。

## 2. 全量统计

### 2.1 分级别主表

方向三列只统计 CompletedMove；`Event` 是 D5 adapter 实际 observe 后的 append-only 事件数。

| 级别 | 塔窗口 | D1 seeds | Move | Completed | Event | 严格 pair | Trend 上 | Trend 下 | Consolidation None | Pending | Unassigned |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| L1 | 9,266 | 9,101 | 2,730 | 1,701 | 1,701 | 827 | 123 | 383 | 1,195 | 1,029 | 165 |
| L2 | 2,088 | 2,045 | 716 | 445 | 445 | 212 | 23 | 111 | 311 | 271 | 43 |
| L3 | 446 | 439 | 193 | 109 | 109 | 38 | 9 | 15 | 85 | 84 | 7 |
| L4 | 84 | 84 | 49 | 26 | 26 | 10 | 2 | 4 | 20 | 23 | 0 |
| L5 | 13 | 13 | 11 | 8 | 8 | 5 | 2 | 2 | 4 | 3 | 0 |
| **合计** | **11,897** | **11,682** | **3,699** | **2,289** | **2,289** | **1,092** | **159** | **515** | **1,615** | **1,410** | **215** |

守恒检查：

- `3,699 Move = 2,289 Completed + 1,410 Pending`；
- `2,289 Completed = 159 Trend(Up) + 515 Trend(Down) + 1,615 Consolidation(None)`；
- `2,289 Completed = 2,289 CompletedFreezeEvent`；
- `11,897 塔窗口 = 11,682 D1 seed + 215 D1 InvalidSeed`。

### 2.2 Pending 分类

| 级别 | AwaitingSubsequentMove | MissingDivergencePair | MissingMacdCoordinates | TerminalLegNotDivergent | 合计 |
|---|---:|---:|---:|---:|---:|
| L1 | 155 | 0 | 0 | 874 | 1,029 |
| L2 | 37 | 17 | 0 | 217 | 271 |
| L3 | 6 | 2 | 0 | 76 | 84 |
| L4 | 0 | 1 | 0 | 22 | 23 |
| L5 | 0 | 1 | 0 | 2 | 3 |
| **合计** | **198** | **21** | **0** | **1,191** | **1,410** |

D2 共生成 `1,865` 个 A/C divergence pair。生产 Pending 分支的逐门实现锚为 `rust/src/theta_v0/classifier/level_view.rs:503-569`。

### 2.3 完整跨级链

| 口径 | 结果 |
|---|---:|
| L1–L5 全深度完整链 | 157 |
| 最大连续深度 | 5 |
| 达到最大深度的链数 | 157 |

各级严格 pair 均非零（`827 / 212 / 38 / 10 / 5`），故不是由单一低级别大分母制造的“伪非零”；全深度结构链实际穿过五级。

## 3. 与 #71 的 0 基线对照

| 项 | #71（2026-07-14） | #83（2026-07-15） | 判断 |
|---|---:|---:|---|
| 测量域 | 例2两个指定 probe pair | BTC 1m 全量、L1–L5 | 分母不同，不做比例横比 |
| 严格 C2 pair | 2/2 均未进入，产量 0 | 1,092 | **全局 0 已解除** |
| CompletedMove | 两目标 pair 未形成两个合格 CompletedMove | 2,289 | **Completed 域已产出** |
| D2 自动配对 | 尚缺 | 1,865 pairs；仅 21 个 Pending(MissingDivergencePair) | **D2 已工作** |
| D3 | 四 provider 未结裁 | 唯一 `central-ggdd-v1`，三值方向守恒 | **D3 已工作** |
| D5 持久化 adapter | 未接 | 2,289 append-only events | **D5 已工作** |
| 完整 L1–L5 嵌套链 | 0 问题未解除 | 157 | **结构闭环非零** |

#71 的原结论是“两组 pair 都没有进入严格 C2 合格域”，并明确属于 `NotEligible/Pending/Unassigned`，不是 firstRetrace 语义否定：`chanlun/review-results/c2-production-replay-20260714.md:13-17`。因此 #83 只回答全局产量；不反向改写 #71 两个指定对象的历史结论。

## 4. 分环定位

因为本轮产量不再为 0，以下是剩余损耗分布，而不是新裁决：

| 环节 | 证据 | 判断 |
|---|---|---|
| D1 投影 | `11,897` 窗口中 `11,682` 成 seed，`215` InvalidSeed，TooShort=0 | 已产出；仍有 1.81% fail-closed 缺口 |
| D2 配对 | `1,865` pairs；MissingDivergencePair=21 | 不是主卡点 |
| D3 方向 | Completed 三值守恒 `159+515+1,615=2,289`，无不变量错误 | 不是卡点 |
| CompletedFreeze | Completed/Event 一一对应 `2,289=2,289` | 不是卡点 |
| 资格/终端门 | TerminalLegNotDivergent=1,191；AwaitingSubsequentMove=198 | 当前主要 Pending 来源 |
| 严格 pair 门 | 1,092，且每级均非零 | 0 产量已解除 |

D3 的语义实现仍严格绑定现有 `classify_relation` GG/DD 判据与 `MoveBlock.dir` 三值：`rust/src/theta_v0/classifier/center.rs:203-215`、`rust/src/theta_v0/classifier/decompose.rs:41-74`；结裁记录锚 `chanlun/escalate/d3-direction-ruling-20260714.md:7-19`。本任务没有修改这些文件。

## 5. 双跑与 SHA-256

正式运行使用同一个 release 二进制，两次命令相同：

```text
cd rust
target/release/p83_yield_remeasure ../analysis/data_cache/btc_1m_full.json
```

结果：

| 运行 | stdout | stderr | SHA-256 |
|---|---:|---:|---|
| run 1 | 3,230 bytes | 0 bytes | `2258a2997963505abf7e0f6d53e4e46ea1279edecbab32d5515ee82589ab1b84` |
| run 2 | 3,230 bytes | 0 bytes | `2258a2997963505abf7e0f6d53e4e46ea1279edecbab32d5515ee82589ab1b84` |

`cmp -s` 返回 0；stdout bit-exact。

## 6. 回归

执行：

```text
cd rust && cargo test --release
```

结果：`1689 passed / 0 failed / 133 ignored`。

- lib：`1635 passed / 0 failed / 127 ignored`；
- 其余 bin / integration / doc-test targets 合计：`54 passed / 0 failed / 6 ignored`；
- 新增 `p83_yield_remeasure` bin 自身为 `0 tests`，本任务未新增看守测试，因此总数相对基线保持不变；
- `git diff --check`：通过。

## 7. 实现与证据锚

- 独立探针及输出字段：`rust/src/bin/p83_yield_remeasure.rs:1-4,126-273`；
- D1 非法 seed 硬断点与合法 run：同文件 `:276-325`；
- 完整 tuple + `assemble_level_view` 消费：同文件 `:328-366`；
- Completed/Pending/方向/严格 pair 计数：同文件 `:387-413`；
- D5 adapter 实际事件计数：同文件 `:415-445`，生产 adapter `rust/src/theta_v0/classifier/level_view_store.rs:239-330`；
- 跨级嵌套计数：`rust/src/bin/p83_yield_remeasure.rs:455-516`，闭包含权威实现 `rust/src/theta_v0/classifier/nest.rs:61-66`；
- D2 pair provider：`rust/src/theta_v0/classifier/level_view.rs:337-407`；
- 完成资格门：同文件 `:418-442,503-569`。

## 8. 遗留待裁 / 测量边界

1. **结构链不等于完整交易证书。** 本轮 `157` 是严格 pair 区间在 L1–L5 的闭包含链；生产 `Chi::is_confirmed` 还要求终端 BSP 确认：`rust/src/theta_v0/classifier/nest.rs:112-134`。是否把严格 C2 pair 链升级为某类 `Chi` / `NestCertificate` 候选，需要另行语义裁决；本任务停在结构统计。
2. **Unassigned 是本探针的显式测量口径。** 当前 `LevelAsOfView` 没有一等 `Unassigned` 输出字段；本轮以 D1 fail-closed 窗口加未被 Move 覆盖的 seed 计数。若要与历史 ledger 的 `Assigned/Unassigned/Tombstoned` 完全同域对账，需要另立 adapter/spec；本任务不替其裁决。
3. **215 个 D1 InvalidSeed 仍需工程审计。** 它们已显式隔离且未阻断其余 run，但尚未分型到具体几何失败原因；不能把它们口头改称 Tombstoned 或合法对象。
4. **本轮是全量终点快照测量。** D5 事件数由最终 as-of 的每个连续 run 首次 observe 得到，证明当前 Completed 对象可持久化且事件数守恒；它不提供每个对象在历史 raw-prefix 中的首次完成时点。若要产量时间序列，必须另做因果 prefix/reducer 重放，不能从 history-end 反推。
5. **不重开既有裁决。** #71 两个指定 probe pair 与 #76 例2的对象身份结论不因全局非零而改变；本报告也不对 `firstRetrace` 作新裁决。

## 9. 复核建议（不执行裁决）

1. 对 `215` 个 D1 InvalidSeed 只读导出级别、窗口 ID、前三子走势坐标与具体 `center_from_*` 失败原子，判断是预期 Unassigned 还是投影覆盖缺口。
2. 对 `21` 个 MissingDivergencePair 分级抽样，核对是合法无 A/C episode 还是 provider 覆盖缺口；保持与 `1,191` 个 TerminalLegNotDivergent 分桶，不得合并成“背驰失败”。
3. 若产品需要“完整区间套交易证书产量”，先裁定严格 C2 pair 区间如何进入 `Chi/NestCertificate` 的 candidate 与 terminal-confirm 域，再另开任务做因果 prefix 重放；不要把本轮 `157` 直接升级为交易结论。
