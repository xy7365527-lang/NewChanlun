# fail-closed 缺口分型审计（task #84）

- 日期：2026-07-15
- 分支：`task-84-failclosed-audit`
- 基线：`main@c8f9bdaec35cc77aabb252ae8b81036b98a409e2`
- 性质：只读测量，不裁决；未修改生产塔、信号、裁定、谱系或既有 review-result

## 0. 结论先行

守恒先通过：在 `btc_1m_full.json` 的 `4,613,599` 根全量终点（`as_of=4,613,598`）上，探针逐级重现 #83 的 D1 `165/43/7/0/0`（合计 `215`，TooShort=`0`）与 MissingDivergencePair `0/17/2/1/1`（合计 `21`）。硬守恒及停止条件见 `rust/src/bin/p84_failclosed_audit.rs:29-30,175-199,712-735`；#83 基准数字见 `chanlun/review-results/c2-yield-remeasure-20260715.md:70-95`。

测量结论如下。

1. **D1 的 215 个 InvalidSeed 全部是 A3（首三段核心空，`ZD>ZG`）**；A1=`0`、A2=`0`。其中 `179` 个窗口没有任何其他合法连续三段，按本任务的可复算证据归为“当前窗口内合法 fail-closed”；另 `36` 个窗口存在至少一个其他合法连续三段，归为“潜在投影覆盖缺口”。后者只说明“存在性”，不裁决生产是否应改锚，也不提出改锚方案。生产顺序锚为 `rust/src/theta_v0/classifier/level_view.rs:244-293`、`rust/src/theta_v0/classifier/center.rs:152-201`；探针镜像及滑窗测量见 `rust/src/bin/p84_failclosed_audit.rs:369-471`。
2. **21 个 MissingDivergencePair 全部命中 B2（`locate_departure_move_a=None`）**；B1/B3/B4 均为 `0`。逐条证据均显示 A 窗口“最后回中枢边界之后的同向锚数”为 `0`：其中 `14` 条在整个 A 窗口就没有同向锚，另 `7` 条虽在边界前出现过同向锚，但最后回中枢后归零。因此按指定三分法，`21` 条均为“真无 A 离开 episode”，史末截断=`0`，其他/潜在 provider 覆盖缺口=`0`。生产 provider 顺序见 `rust/src/theta_v0/classifier/level_view.rs:355-407`，A episode 定界见 `rust/src/theta_v0/classifier/divergence.rs:654-755`，探针逐门镜像见 `rust/src/bin/p84_failclosed_audit.rs:609-697`。
3. **L1=0、L2=17 是实测分母/分子差异，不作机制外推。** L1 的 `1,380/1,380` 个 Trend block 全部走到 provider Success，故 Missing=`0`；L2 的 `368` 个 Trend block 中 Success=`351`、B2=`17`，故 Missing=`17`（L2 内 `17/368=4.62%`，占全部 Missing 的 `17/21=80.95%`）。逐级分母与逐门结果由探针 `P84_LEVEL` 输出，计数实现见 `rust/src/bin/p84_failclosed_audit.rs:218-244,530-589`。这里不把级别集中解释成未测的市场或语义机制。

## 1. 输入、生产顺序与逐条导出

- 输入：`analysis/data_cache/btc_1m_full.json`；原始 `4,613,599` 根，parser 合并后 `3,037,868` 根；日期 `2017-08-17 04:00:00` 至 `2026-05-31 23:59:00`；`as_of=4,613,598`。与 #83 输入一致：`chanlun/review-results/c2-yield-remeasure-20260715.md:24-28`。
- 唯一新增探针：`rust/src/bin/p84_failclosed_audit.rs`。它复用 #83 的合法连续 run 骨架，只读调用 `parse_layer → classify_with_tower → project_extended_windows → decompose → assemble_level_view`：`rust/src/bin/p84_failclosed_audit.rs:146-199,302-367,487-589`。
- D1 的 `P84_D1` 共 `215` 行；每行含级别、`ElementId(level:ordinal)`、`sub_moves` 数、A 原子、A1 子走势序号、`ZD/ZG`、前三子走势的 `start:end:direction:lo:hi`，以及其他合法滑窗的起点和数量。字段输出锚：`rust/src/bin/p84_failclosed_audit.rs:246-272`。
- Part B 的 `P84_B` 共 `21` 行；每行含级别、run、block start/end center、方向、Move 坐标、前/末中枢 end、A 窗口段数/同向锚数、最后回中枢边界、边界后同向锚数、C 终段/C 起点和证据分类。字段输出锚：`rust/src/bin/p84_failclosed_audit.rs:274-299`。

## 2. Part A：215 个 D1 InvalidSeed

### 2.1 失败原子 × 级别

判定严格按生产顺序：先取前三个 `as_unit`（任一失败为 A1）；L1 再查 `dir_alternates`（失败为 A2）；最后查全三段 `ZD≤ZG`（失败为 A3）。生产对应 `rust/src/theta_v0/classifier/level_view.rs:244-280` 与 `rust/src/theta_v0/classifier/center.rs:152-201`；探针镜像见 `rust/src/bin/p84_failclosed_audit.rs:369-398,453-471`。

| 原子 | L1 | L2 | L3 | L4 | L5 | 合计 |
|---|---:|---:|---:|---:|---:|---:|
| A1：`as_unit=None` | 0 | 0 | 0 | 0 | 0 | 0 |
| A2：L1 `dir_alternates=false` | 0 | 0 | 0 | 0 | 0 | 0 |
| A3：核心空 `ZD>ZG` | 165 | 43 | 7 | 0 | 0 | 215 |
| **合计** | **165** | **43** | **7** | **0** | **0** | **215** |

`sub_moves` 分布为：恰 3 段 `174`、4 段 `26`、5 段 `15`；没有大于 5 段的失败窗口。逐条 `ZD/ZG` 与前三段坐标均在 `215` 行 `P84_D1` 中，且每条都满足 `ZD>ZG`；探针若遇到无法落入 A1/A2/A3 的 InvalidSeed 会立即报错停止：`rust/src/bin/p84_failclosed_audit.rs:369-398`。

### 2.2 疑点③：其他合法连续三段

对全部 `sub_moves>3` 且首三段失败的 `41` 个窗口，探针用同级生产函数依次测试全部连续三段（并断言起点 0 仍失败），再统计其余合法起点：L1 调 `center_from_segments`，L≥2 调 `center_from_window`；实现见 `rust/src/bin/p84_failclosed_audit.rs:400-450`。

| 首三段失败原子 | L1 | L2 | L3 | L4 | L5 | 合计 |
|---|---:|---:|---:|---:|---:|---:|
| A1 且存在其他合法三段 | 0 | 0 | 0 | 0 | 0 | 0 |
| A2 且存在其他合法三段 | 0 | 0 | 0 | 0 | 0 | 0 |
| A3 且存在其他合法三段 | 29 | 5 | 2 | 0 | 0 | 36 |
| **合计窗口数** | **29** | **5** | **2** | **0** | **0** | **36** |

这 `36` 个窗口合计存在 `46` 个其他合法三段（部分 5 段窗口有两个合法起点）；其余 `5` 个 `sub_moves>3` 窗口没有其他合法三段。故按本任务的测量分类：`179=215-36` 为“无其他合法三段的合法 fail-closed”，`36` 为“存在性意义上的潜在覆盖缺口”。此处不把“存在其他合法三段”升级为“生产首三段锚错误”，也不提出锚定变更。

## 3. Part B：21 个 MissingDivergencePair

### 3.1 原子 × 级别与证据分类

| 原子 / 分类 | L1 | L2 | L3 | L4 | L5 | 合计 |
|---|---:|---:|---:|---:|---:|---:|
| B1 guard / 其他 | 0 | 0 | 0 | 0 | 0 | 0 |
| B2 无 A episode / 真无 A 离开 episode | 0 | 17 | 2 | 1 | 1 | 21 |
| B3 无 C 终段 / 史末截断 | 0 | 0 | 0 | 0 | 0 | 0 |
| B4 无 C 起点 / 其他（潜在覆盖缺口） | 0 | 0 | 0 | 0 | 0 | 0 |
| **合计** | **0** | **17** | **2** | **1** | **1** | **21** |

生产对每个 Trend block 按 B1→B2→B3→B4 顺序早退：`rust/src/theta_v0/classifier/level_view.rs:365-393`。探针对所有 `1,886` 个 Trend block 镜像同序逐门，并交叉验证“生产 Missing iff 探针非 Success”；若两者不一致立即停止：`rust/src/bin/p84_failclosed_audit.rs:530-589,609-697`。

### 3.2 21 条逐个明细

共同 `as_of=4,613,598`。`A窗` 写作“段数/原始同向锚数”；`回界`是 A 窗口内最后一个回中枢段的 `end_index`，`—`表示没有回界；`界后锚`为该边界之后仍可充当 A 的同向锚数。所有 21 条 `界后锚=0`，逐字对应 `episode_start_in=None → locate_departure_move_a=None`：`rust/src/theta_v0/classifier/divergence.rs:677-699,718-735`。

| # | 级别/run | block center | 方向 | Move start:end | prev→last center end | A窗 | 回界 | 界后锚 | 原子 | 证据分类 |
|---:|---|---|---|---|---|---:|---:|---:|---|---|
| 1 | L2/r0 | 23:25 | Up | 53012:61430 | 57657→61430 | 5/0 | 59517 | 0 | B2 | 真无 A episode |
| 2 | L2/r1 | 31:32 | Up | 170778:174527 | 172156→174527 | 3/0 | — | 0 | B2 | 真无 A episode |
| 3 | L2/r4 | 63:64 | Up | 594210:597962 | 596253→597962 | 4/0 | 597053 | 0 | B2 | 真无 A episode |
| 4 | L2/r4 | 73:74 | Down | 610241:613220 | 611277→613220 | 4/0 | 611866 | 0 | B2 | 真无 A episode |
| 5 | L2/r9 | 5:6 | Down | 1138413:1143509 | 1139424→1143509 | 7/2 | 1141541 | 0 | B2 | 真无 A episode |
| 6 | L2/r11 | 35:36 | Up | 1291614:1295226 | 1293126→1295226 | 3/0 | — | 0 | B2 | 真无 A episode |
| 7 | L2/r13 | 50:51 | Up | 1772607:1777959 | 1774287→1777959 | 5/0 | 1774723 | 0 | B2 | 真无 A episode |
| 8 | L2/r13 | 65:66 | Up | 1819616:1827477 | 1821072→1827477 | 7/1 | 1823308 | 0 | B2 | 真无 A episode |
| 9 | L2/r13 | 219:220 | Down | 2221006:2228595 | 2222331→2228595 | 9/3 | 2226220 | 0 | B2 | 真无 A episode |
| 10 | L2/r13 | 238:239 | Down | 2269158:2273945 | 2270146→2273945 | 5/0 | 2270688 | 0 | B2 | 真无 A episode |
| 11 | L2/r31 | 7:8 | Up | 3640330:3642948 | 3641154→3642948 | 4/0 | — | 0 | B2 | 真无 A episode |
| 12 | L2/r33 | 36:38 | Up | 3889247:3896607 | 3892672→3896607 | 6/1 | 3893720 | 0 | B2 | 真无 A episode |
| 13 | L2/r33 | 92:93 | Down | 4045988:4050481 | 4047278→4050481 | 5/0 | — | 0 | B2 | 真无 A episode |
| 14 | L2/r33 | 108:109 | Down | 4083156:4086857 | 4084528→4086857 | 4/0 | 4085565 | 0 | B2 | 真无 A episode |
| 15 | L2/r38 | 1:2 | Down | 4271275:4274241 | 4273212→4274241 | 3/0 | — | 0 | B2 | 真无 A episode |
| 16 | L2/r38 | 12:13 | Up | 4298779:4301628 | 4299884→4301628 | 3/0 | — | 0 | B2 | 真无 A episode |
| 17 | L2/r38 | 33:34 | Up | 4342585:4347348 | 4343913→4347348 | 7/2 | 4345336 | 0 | B2 | 真无 A episode |
| 18 | L3/r5 | 1:2 | Up | 1280620:1300087 | 1284275→1300087 | 8/1 | 1293126 | 0 | B2 | 真无 A episode |
| 19 | L3/r5 | 58:59 | Down | 1950196:1965971 | 1955866→1965971 | 3/0 | — | 0 | B2 | 真无 A episode |
| 20 | L4/r0 | 46:48 | Up | 2741411:2947281 | 2880443→2947281 | 6/1 | 2913414 | 0 | B2 | 真无 A episode |
| 21 | L5/r0 | 1:2 | Down | 425334:775278 | 554267→775278 | 5/0 | 642844 | 0 | B2 | 真无 A episode |

方向分布为 Up=`12`、Down=`9`；不是单一方向造成。14 条原始同向锚数为 0；另外 7 条原始同向锚数为正，但均有较后的回中枢边界，且边界后同向锚数为 0。这是 B2 的逐条结构证据，不是对 A episode 语义作新定义。

### 3.3 L1=0、L2=17 的测得解释

| 级别 | Trend block | provider Success | B1 | B2 | B3 | B4 | Missing 率 |
|---|---:|---:|---:|---:|---:|---:|---:|
| L1 | 1,380 | 1,380 | 0 | 0 | 0 | 0 | 0.00% |
| L2 | 368 | 351 | 0 | 17 | 0 | 0 | 4.62% |
| L3 | 102 | 100 | 0 | 2 | 0 | 0 | 1.96% |
| L4 | 29 | 28 | 0 | 1 | 0 | 0 | 3.45% |
| L5 | 7 | 6 | 0 | 1 | 0 | 0 | 14.29% |
| **合计** | **1,886** | **1,865** | **0** | **21** | **0** | **0** | **1.11%** |

- **L1=0**：不是“L1 没有 Trend Move”；实测恰有 `1,380` 个，且 `1,380` 个全部通过 B1–B4 并生成 pair，所以 Missing 分子为 0。生产 pair 与 Missing 的连接锚为 `rust/src/theta_v0/classifier/level_view.rs:503-569`。
- **L2=17**：不是由 B1 越界、史末无 C 或 B4 造成；`368` 个 Trend block 中只有这 `17` 个在 B2 早退，逐条 `界后锚=0`，其余 `351` 个成功。L2 承担 `80.95%` 的 Missing，只是 `17/21` 的测得分布；本任务没有测得足以支持更远的因果叙述。

## 4. 双跑、SHA-256 与回归

正式运行使用同一个 release 二进制，两次命令相同：

```text
cd rust
target/release/p84_failclosed_audit ../analysis/data_cache/btc_1m_full.json
```

| 运行 | stdout | stderr | SHA-256 |
|---|---:|---:|---|
| run 1 | 77,316 bytes / 243 行 | 0 bytes | `7130ad70c9c16d3d4f5335093a4eb8a7cfc9ab586ac343a47b7688f87c35dd12` |
| run 2 | 77,316 bytes / 243 行 | 0 bytes | `7130ad70c9c16d3d4f5335093a4eb8a7cfc9ab586ac343a47b7688f87c35dd12` |

`cmp -s` 返回 0，stdout bit-exact。

执行 `cd rust && cargo test --release`：`1689 passed / 0 failed / 133 ignored`，与基线完全一致；其中 lib 为 `1635 passed / 0 failed / 127 ignored`，其余 target 合计 `54 passed / 0 failed / 6 ignored`。新增 bin 自身为 `0 tests`。

## 5. 测量边界与遗留待裁

1. **“合法/潜在覆盖”是本任务的操作性分桶，不是语义裁决。** D1 的 `179/36` 只以“同一扩展窗口是否存在其他合法连续三段”为界；是否允许首三段以外的锚、如何保持身份/前缀稳定，均未裁。B 的三分法只按当前 provider 的逐门返回值与 episode helper 证据归类。
2. **全量终点快照不是因果 prefix 重放。** 本轮证明在 `as_of=4,613,598` 的塔与 lower legs 上，21 条在 A 门即失败；不反推这些对象历史上何时首次进入或离开 Pending。
3. **D1 的 36 个潜在覆盖缺口留待另行裁定。** 本任务没有修改 `project_extended_windows`、没有尝试滑动锚进入生产、没有比较替代 seed 对下游 Move 身份/产量的影响。
4. **Part B 本数据上没有 B3/B4 样本。** 因而不能从“本轮为 0”推出这些分支在一般数据上不可达；只可报告本全量快照的观测值。
5. **逐条导出的持久载体是确定性探针 stdout。** 215 条 D1 和 21 条 B 明细可由上述 release 命令复算，并由双跑 SHA-256 锁定；报告只展开了用户要求的 Part B 21 条表，Part A 逐条坐标保留在 `P84_D1` 输出，避免把 215 行原始证据重复抄入结论表。
