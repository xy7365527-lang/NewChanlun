# P47：完整 `c_p` 证书闭合生命周期修复（任务 #47，2026-07-12）

- 分支：`p0-replay-dparent`
- 基线 HEAD：`99ce554b8a6bfbe2e5ce601585192616b1fc3d8c`
- 直接权威：`p46-third-closure-zero-20260712.md` §3.2/§3.3、700 号生成态谱系。
- 数据：BTC 1m 4,613,599 bar，`ThetaConfig::default()`。
- 边界：本轮未修改 `judge_third_cert`、未放宽第三类闭合条件、未给 `anchor=None`
  回填方向、未以前视方式回填任何历史确认快照。

## 0. 结论

任务 #47 已按 P46 的对象生命周期方案落地：完整 `c_p` 不再等待另一个
`CandDeltaEvent` 才获得闭合机会。`CpScanOwnership` 现在是按 `B_p/c_p` 身份持续推进的
`Pending -> Closed` 对象；每个新同级相邻递归单元对到达时，生产路径复用原
`nearest_confirmed_center_idx + judge_third_cert + anchor provenance` 判据推进对象。

两个时间点和两个消费视图已经物理分开：

1. `divergence_confirm_src`：背驰事件的算法确认时点（`confirm_src` 保留为兼容别名）；
2. `cp_certificate_confirm_src`：第三类/完整机器结构首次可证时点；
3. `cp_certificate_at_divergence(event)`：只读确认时冻结快照；
4. `cp_terminal_certificate(event, objects)`：沿稳定归属边显式读取终态对象。

全量 P46 L1-L4 31 个唯一对象的确认时快照仍为 **0/31**，终态对象闭合为
**11/31**，剩余 **20/31** 仍为 Pending。P46 §2.2 的 11 个晚到第三类全部进入对象态，
没有把 20 个终点仍无合法 third 的对象误闭合。

★ Insight ─────────────────────────────────────

- 事件是时间切片，对象是生命周期载体。把终态写回事件会伪造确认时可知性；把终态绑在新事件上
  又会漏掉 B 样本。稳定边把两类错误同时排除。
- `c_p` 右端现在取首次合法 retest 的结束点；随后 Cand 事件只能引用同一对象，不能贡献旧
  `c_p` 的右端。
- 本轮 Closed 只表示“现有第三类机器证书生命周期闭合”，不膨胀为第37课完整趋势 `c` 已获
  定义性证明；700 号剩余定义张力继续开放。

─────────────────────────────────────────────────

## 1. 实现

### 1.1 `B_p/c_p` 持续对象态

`CpScanOwnership` 新增：

- `lifecycle: Pending | Closed`；
- `cp_certificate_confirm_src`；
- 终态 `c_structure`；
- 终态 `third_class_in_c`。

`advance_cp_lifecycles` 对新到达或发生变异的同级相邻单元对执行：

```text
new (leave, retest)
  -> nearest_confirmed_center_idx(leave.start)
  -> 唯一 B_p/c_p Pending 对象
  -> 原 judge_third_cert(B_p, leave, legal_anchor, retest)
  -> Closed at retest.end / cp_certificate_confirm_src
```

每个 pair 只路由到唯一最近已确认中枢；增量推进为 `O(new_units log centers)`，Closed 对象
不重复判定。批式 `classify_with_tower` 与增量 `classify_with_tower_incremental` 都调用同一推进函数。
frontier 窗口 pop/recompose 若重建出相同 `B_p + departure` 身份，会继承已扫描对象态并只从
`dirty_from` 推进；若旧 Closed 证书终端落入 dirty 后缀，则禁止继承、从 departure 重判。

### 1.2 双时点与冻结快照

`CandDeltaEvent` 同时保存：

- `divergence_confirm_src`（兼容别名 `confirm_src`）；
- 确认时 `cp_certificate_confirm_src: Option<usize>`；
- 确认时 `c_interval_full / c_structure / third_class_in_c`。

对象后来闭合时，writer 的可变入参只有 `&mut [CpScanOwnership]`，不持有也不修改历史
`CandDeltaEvent`。因此 B 的确认事件可以永久保持 `None`，同时通过稳定边读取终态 3306426。

### 1.3 稳定边不携终态右端

`CandDeltaCpEdge` 现在只保存：

- `b_center_id`；
- `cp_departure_move_id`；
- `cp_source_start`。

已删除边上的 `cp_terminal_move_id / cp_source_end`。终端元素和右端只属于所选择的快照证书或
终态对象证书，不能由随后某个事件写入稳定边。

确认时若 third 已存在，快照右端也改为 `third.retest_interval.1`，不再使用当前事件
`event_end`。这使 A 的 42841/42998 两条后续事件都只观察到 42759，而不是制造两个不同
`c_p` 右端。

## 2. 两条最小复现

### 2.1 A：`B=L1#73`，有后续事件

| 观察点 | 修复前事件字段 | 修复后确认快照 | 修复后终态对象 |
|---|---|---|---|
| `confirm=42704` | `c_end_full=None` | `cp_confirm=None / end=None` | `cp_confirm=42759 / end=42759` |
| `confirm=42841` | `c_end_full=42841` | `cp_confirm=42759 / end=42759` | `cp_confirm=42759 / end=42759` |
| `confirm=42998` | `c_end_full=42998` | `cp_confirm=42759 / end=42759` | `cp_confirm=42759 / end=42759` |

结构回归 `p46_a_l1_73_closes_at_42759_independent_of_later_cand_events` 分别只喂到
retest=42759，以及继续喂到 42841/42998；两个终态 `CpScanOwnership` 逐字段相等。
真实 100k 重放也输出三条事件终态均为 42759。

### 2.2 B：`B=L2#1508 / dep=L1#6703`，无后续事件

| 观察点 | 修复前 | 修复后 |
|---|---|---|
| 背驰确认 3306324 | 快照 `None`；终态不可达 | 快照仍 `third/end/cp_confirm=None`，稳定边存在 |
| `L1#6704` retest 3306426（+102 bar） | 无新 Cand writer，历史事件永久 `None` | 对象态独立闭合：`cp_confirm=end=3306426` |

结构回归 `p46_b_l2_1508_closes_without_later_event_and_snapshot_stays_none` 同时断言：

- `cp_certificate_at_divergence(snapshot_event) == None`；
- `cp_terminal_certificate(snapshot_event, objects).cp_certificate_confirm_src == 3306426`。

全量真实重放对应行：

```text
P46_CASE level=1 divergence_confirm_src=3306324
snapshot_cp_confirm=None snapshot_end=None
terminal_cp_confirm=Some(3306426) terminal_end=Some(3306426) stable_edge=true
```

## 3. 全量闭合数对比

### 3.1 P46 L1-L4 的 31 个唯一对象

| 级别 | 对象数 | 修复前终态对象闭合 | 修复后确认时快照 | 修复后终态闭合 | 修复后仍 Pending |
|---:|---:|---:|---:|---:|---:|
| L1 | 7 | 0 | 0 | 1 | 6 |
| L2 | 13 | 0 | 0 | 6 | 7 |
| L3 | 9 | 0 | 0 | 3 | 6 |
| L4 | 2 | 0 | 0 | 1 | 1 |
| 合计 | **31** | **0** | **0** | **11** | **20** |

结论：终态对象闭合数 = 确认时数 `0 + 11`；P46 §2.2 的 11 例全部转为对象态闭合，
终点仍无合法 third 的 20 例逐一保持 Pending。

### 3.2 全级别唯一对象

| 口径 | 闭合对象数 |
|---|---:|
| 确认时快照 | 108 |
| 终态对象 | 202 |
| 终态新增 | **+94** |

全量事件 483 条，483 条均有稳定归属边；该表按 `(level, B_p id, departure id)` 去重，不把
同一对象的多个 Cand 事件重复计数。

## 4. 确认时快照未被改写的证明

证明链由类型边界、结构回归和真实重放三层组成：

1. **writer 边界**：`advance_cp_lifecycles` 只能修改 `CpScanOwnership`，函数签名不接收事件；
2. **消费边界**：确认快照与终态对象是两个不同函数，终态读取必须提供对象集合并沿稳定边查找；
3. **A 真实证据**：42704 快照继续为 `None`，终态为 42759；后续事件未改写 42704；
4. **B 真实证据**：3306324 快照继续为 `None`，终态为 3306426；
5. **负向证据**：20 个终点无 third 的对象仍 Pending，没有因“数据已到终点”或其它事件被回填。

因此不存在“未来 third 注入确认时历史”的写路径。

## 5. 禁区审计

| 禁止项 | 审计结果 |
|---|---|
| 修改 `judge_third_cert` | P47 未修改；生命周期推进只调用该单一真值函数 |
| 放宽闭合条件 | 未增加替代条件；仍要求 ownership、合法 anchor、方向配对、严格离开与严格不重入 |
| 回填历史 `None` | 未回填；事件快照与终态对象物理分离 |
| 给 `anchor=None` 回填方向 | 未做；L≥1 继续原样读取 `Option<Direction>`，`None` 直接传入原判据失败 |
| 用后续事件 `seg.end` 当旧右端 | 已消除；右端唯一取首次合法 retest.end |

L0 的 `anchor_dirs=None -> Some(segment.direction)` 是既有 L0 内在方向口径，不是 686 号禁止的
L≥1 fallback；本轮未扩张其有效域。

## 6. 修改文件清单

| 文件 | P47 增量 |
|---|---|
| `rust/src/theta_v0/classifier/recursive_tower.rs` | 生命周期对象、推进器、双证书视图、稳定边、A/B 回归 |
| `rust/src/theta_v0/classifier/mod.rs` | 批式/增量生产路径推进对象态；LevelState/LevelCache 语义更新 |
| `rust/src/bin/strict_nest_check.rs` | 稳定边消费者改从证书结构读取终端；测试构造器补双时点 |
| `rust/src/theta_v0/classifier/nest.rs` | 测试/兼容事件构造器补双时点字段 |
| `rust/src/theta_v0/backtest/runner.rs` | 测试事件构造器补双时点字段 |
| `rust/src/bin/cp_capability_smoke.rs` | 100k/全量、批式/增量、唯一对象快照/终态统计 |
| `.chanlun/genealogy/pending/700-*.md` | 追加 P47 实现层部分回溯结算；定义张力保持生成态 |
| `chanlun/review-results/p47-cp-lifecycle-fix-20260712.md` | 本报告 |

`rust/src/theta_v0/classifier/signal.rs` 中的 `judge_third_cert` 证书化是 P45/P46 前置未提交基线，
不属于 P47 修改；P47 没有改变其条件分支。

## 7. 验证

| 命令 | 结果 |
|---|---|
| `cargo check --all-targets` | PASS |
| `cargo test p46_ --lib` | 2 passed / 0 failed |
| `cargo test` | **1554 passed / 127 ignored / 0 failed**；bin、Lean parity、分类器、证书链均绿 |
| `CP_SMOKE_MAX_BARS=100000 cargo run --release --bin cp_capability_smoke` | A 三事件终态均 42759；42704 快照仍 None |
| `CP_SMOKE_MODE=batch CP_SMOKE_MAX_BARS=4613599 CP_SMOKE_VERBOSE=0 cargo run --release --bin cp_capability_smoke` | 31 对象：snapshot 0 / terminal 11 / pending 20；B=3306426 |
| `git diff --check` | PASS |

仓库全局 `cargo fmt -- --check` 的基线已有大量与 P47 无关的历史格式差异；本轮新增独立 bin 已单独
`rustfmt`，且 `git diff --check` 通过。本报告不把全仓格式基线冒充为 P47 回归失败。

## 8. 仍开放的定义边界

700 号继续保持生成态，不移入 settled，也不改 block topology。P47 没有裁决：

- 剩余 20/31 是非完整趋势 `c`，还是递归对象映射仍不足；
- 第37课第20行创新高/低、第22行至少两个次级别中枢如何机器映射；
- `CandDeltaEvent` 算法背驰候选与原文完整趋势 `c` 资格的最终命名/有效域。

这些问题不能由“第三类机器证书生命周期已修”自动推出答案。
