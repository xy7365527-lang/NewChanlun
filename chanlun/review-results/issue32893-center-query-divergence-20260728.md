# #591 研究：32893 完成检测 vs 活窗路径中枢查询不一致——精确溯因

> 角色：研究执行层（只读；本 session 未改仓内任何文件，只写本报告，全程前台单线程，未派子代理）
> 基线：`kimi-nest-mainline-20260717` @ `04c360a410`（与 #580 报告同一基线，代码未变）
> 数据源：复用既有产物 `/tmp/wt527fix-post-100k.dump`（未重放，未占用 `CARGO_TARGET_DIR=/tmp/kimi-nest-target-591`，
> 未跑 `cargo build`/`cargo test`——本票是对既有 dump 的进一步精读，不需要新数据）。
> 探针：复用既有 `/tmp/wt580_detail.py`（只读）+ 本 session 内联 Python 单次脚本（未落盘，仅 stdout）。
> 票据：#591（map #59 子票，#580 §1.4/§5/§4-3 遗留的「未判定」诊断票）。

## 0. 结论摘要

**判定：时序差（无害），非查询切片实质差异。** `level_view.rs`（完成检测路径）与
`nest_lifecycle.rs::provide_l1_active_pan_live_windows`（活窗路径）对 `nearest_confirmed_center_idx`
的**输入构造算法逐字相同**（同一 `project_extended_windows_carried_only` run 切分 + 同一
`projection.seeds.iter().map(|seed| seed.center)` 取值），差异只在**两路径查询该共享纯函数的时刻与
`seg_start` 取值不同**：活窗路径按「当前全局唯一 pending frontier」稀疏采样（仅在 `(forest_epoch,
frontier)` 变化时重算），完成路径在结构**已完全确认**后对**已固定**的 `segment.start_index` 做一次性
事后查询。32893 现场的 `L1_LIVE_RECOMPUTE` 逐行时间序列显示：中枢 `b=32117` 的确认与目标 C 段
（`32653→32702`）被 parser 判定完成、frontier 前移到下一段（`32714`）**落在同一次 recompute 间隔内**——
活窗路径在 `active.start_index==32653` 的全部快照里，`b=32117` 尚未进入 `centers`；等它进入时，frontier
已经前移，稀疏采样窗口已经错过。这是**中枢确认滞后 + 单一全局 frontier + 事件驱动稀疏重算**三者叠加
的必然结果，不是两条路径对「centers 该取哪些」有不同定义。

## 1. 两路径中枢查询口径对照

### 1.1 共享的查询原语（唯一来源，两路径都调用同一个函数）

`rust/src/theta_v0/classifier/signal.rs:213`：
```rust
pub(crate) fn nearest_confirmed_center_idx(centers: &[Center], seg_start: usize) -> Option<usize> {
    let hi = centers.partition_point(|c| c.end_index <= seg_start);
    if hi == 0 { None } else { Some(hi - 1) }
}
```
纯函数：给定一个**按 end_index 升序排列**的 `centers` 切片和一个 `seg_start`，二分找「end_index ≤
seg_start 的最后一个」。两条路径分歧只能来自 (a) 传入的 `centers` 切片不同，或 (b) 传入的 `seg_start`
不同，或 (c) 调用时机（对应哪个 `tower` 快照）不同。逐条核对如下。

### 1.2 `centers` 切片构造算法——**逐字相同**

| 路径 | 调用点 | `centers` 构造代码 |
|---|---|---|
| 完成检测（生产真值，慢/快路径共用） | `level_view.rs` `provide_nest_candidate_events_ext_resident` → 内部 pan 分支（约 `level_view.rs:1512`） | `let centers: Vec<_> = projection.seeds.iter().map(\|seed\| seed.center).collect();` |
| 完成检测在 p123 回放里的接线（`evaluate_run`） | `rust/src/bin/p123_fast_replay.rs:1954` | `let centers: Vec<_> = projection.seeds.iter().map(\|seed\| seed.center).collect();`（同一行，`projection` 来自同一 `project_extended_windows_carried_only(&windows[start..end])`） |
| 活窗路径在 p123 回放里的接线（`recompute_lifecycle_window_stems`） | `rust/src/bin/p123_fast_replay.rs:1627-1628` | `let centers: Vec<_> = projection.seeds.iter().map(\|seed\| seed.center).collect();`（**同一行**，`projection` 同样来自 `project_extended_windows_carried_only(&windows[start..end])`） |

`windows[start..end]` 的 run 切分算法本身也逐字相同：完成路径侧的 `build_run_ranges`
（`p123_fast_replay.rs:1908-1932`）与活窗路径侧 `recompute_lifecycle_window_stems` 内联的 run 扫描
（`p123_fast_replay.rs:1617-1665`）都是「单窗 `project_extended_windows_carried_only` 判 valid →
连续 valid 区间即一个 run」，且都作用于**同一个 `tower[level]`**（`Rc<Vec<LeveledMove>>`，回放主循环
每 bar 只产一份，两条路径读同一份引用，见 `p123_fast_replay.rs:1073` `tower` 的产生处）。

`nest_lifecycle.rs:1456-1458` 的模块头声明「结构判据一律复用同一套单一来源（禁第二查法）……与完成事件
provider（level_view.rs pan 分支）同序同判」——**本节核实这句声明在 `centers` 构造算法层面成立**，
#580 §1.4 标注的「未判定」在这一层已可判定：**不是算法分岔**。

### 1.3 `seg_start` 语义——**不同变量，但设计上应指向同一物理量**

| 路径 | `seg_start` 取值 | 来源 |
|---|---|---|
| 完成检测 | `segment.start_index`，`segment` 取自 `segments: Vec<_> = legs.iter().map(leg_as_segment).collect()`（`level_view.rs:1420`），`legs: &[LowerLeg]` = **已确认**的 lower legs | 只在该 leg 已完成、被 parser 承认为 confirmed leg 后才存在 |
| 活窗 | `active.start_index`，`active = frontier.as_segment()`，`frontier: &ActiveSegmentFrontier` = `active_segment_frontier(&l0)` 返回的**当前唯一** pending（行进中）段 | 每 bar 都存在（只要 parser 有 pending 段），随该段延展 `extreme_at` 变化，**同一个物理段**从 pending 到 confirmed 全程 `start_index` 不变（`nest_lifecycle.rs:1463-1467` 已文档化此不变式并注明其限定范围） |

对**同一个物理 C 段**，两个 `seg_start` 理论上同值（活窗路径追的是它 pending 时的 `start_index`，完成
路径追的是它 confirmed 后同一 `start_index`）——这一点在 32893 现场也确认成立：活窗路径 32691/32720 两次
命中的 `c_start=32653` 与完成事件 `seg_c_full=(32653,32702)` 左端逐位相同。**差异不在 `seg_start` 的值，
在查询这个值的“窗口”能持续多久**——见 §2。

### 1.4 调用时机——**真正的分岔点**

- **活窗路径**：`p123_fast_replay.rs:1080-1082` 门控——只在 `forest_epoch` 或 `frontier`（含
  `extreme_at`，逐 bar 变）变化时重算，但这不等于逐 bar 重算：`ActiveSegmentFrontier` 本身**只表示当前
  唯一 pending 段**——一旦该段被 parser 判定完成（供给下一段），`frontier.start_index` 立刻跳到下一段
  起点，**活窗路径对旧段的可观测窗口随之关闭**，不存在「回头再查一次旧段」的机制。
- **完成检测路径**：只在结构**已经**完成（`segment` 来自 confirmed `legs`）之后，对一个**已固定不变**
  的 `segment.start_index` 做查询，且该查询在 p123 回放里由 `refresh_lifecycle_cache` 的 per-run
  `dirty` 判据驱动（`self_gen`/`lower_gen`/`watermark_crossed`，`p123_fast_replay.rs:1707-1722`）——
  只要 `tower[level]` 或其 lower 塔层有任何变化就会重估，粒度明显细于活窗路径的
  `(forest_epoch, frontier)` 双门控。**这个粒度差本身也不是「查询切片不同」，是两条路径对「什么时候
  值得重新问一次」的节流策略不同**（活窗路径是预览性质、完成路径是终局判定，节流策略不对齐本属预期）。

## 2. 32893 现场逐条（`L1_LIVE_RECOMPUTE` 精确时间序列，本票新增证据）

目标完成事件（`COMPLETION_SIGNAL as_of=32893`）：
```
level=1 side=Long kind=Consolidation seg_a=(31998,32089) seg_c_full=(32653,32702) b_center_start=32117
lower_id=(0,246) completed_at=32702
```
其 REV 全历史（`b=32117`，`seg_a=(31998,32089)`）：`Observed`/`FirstProvable`/`StructureCompleted`/
`Confirmed` **全部同刻写在 `as_of=32893`**——即完成检测路径本身也是「事后一次性」看到这只身份，不存在
更早的 REV 记录。

`L1_LIVE_RECOMPUTE`（活窗路径的每次重算，`frontier=(start,extreme_at,direction)`，`confirmed_last_end`
= 当时 `tower[0]` 最后一个已确认 L0 段终点）在 `[32600,32950]` 内的完整序列：

| as_of | frontier (start,extreme_at,dir) | confirmed_last_end | L1_LIVE 行 |
|---:|---|---:|---|
| 32691 | (32653,32690,Down) | 32644 | HIT b=**31805** c_start=32653 |
| 32720 | (32653,32714,Down) | 32644 | HIT b=**31805** c_start=32653 |
| **32737** | **(32714,32736,Up)** | **32702** | MISS reason=structure_not_locatable b=31805 |
| 32748 | (32714,32740,Up) | 32702 | MISS b=31805 |
| 32759 | (32714,32758,Up) | 32702 | MISS b=31805 |
| 32825 | (32714,32820,Up) | 32702 | HIT b=**31805** c_start=32714 |
| 32893 | (32855,32892,Up) | 32855 | HIT b=**32475** c_start=32855 |

**关键行是 32720→32737 的跳变**：`confirmed_last_end` 从 32644 跳到 **32702**——这正是目标完成事件
`seg_c_full` 的右端，即**目标 C 段（32653→32702）在这一步被 parser 正式确认完成**。同一步里
`frontier.start_index` 从 32653 跳到 32714——活窗路径对「`active.start_index==32653`」这一查询目标的
观测窗口**在这一步之后永久关闭**（下一次 recompute 的 `active` 已经是下一个物理段）。

`b=32117` 在整个 `[32691,32893]` 窗口内的 `L1_LIVE` 命中记录：**一次也没有**。对比：
- `seg_start=32653` 时（32691/32720）：答案是 31805，说明此刻 32117 尚未进入 `centers`（若已进入，
  按 `partition_point` 语义 32117 起点晚于 31805，只要其 `end_index≤32653` 就会覆盖 31805 成为新答案）。
- `seg_start=32714` 时（32737/32748/32759/32825，跨度长达 88 bar）：答案**仍是 31805**，说明 32117
  直到 as_of=32825 仍未进入 `centers`（若已进入且 `end_index≤32714`，理应覆盖 31805）。
- `seg_start=32855` 时（32893）：答案变成 **32475**（比 32117 起点更晚的中枢），32117 被**跳过**——
  说明 32117 与 32475 大概率在 `[32825,32893]` 这一稀疏采样间隔内**先后确认**，活窗路径下一次落地
  重算时（32893）看到的已经是「32475 更新」的终态，32117 曾经短暂是「最新」这件事**没有任何一次
  recompute 恰好落在那个时刻窗口内**。

同一 `b=32117` 中枢在 `as_of=32893` 完成事件里同时被**另一只 Short 候选**（`seg_a=(32091,32117)`，
`seg_c_full=(32475,32644)`）引用为 B——该候选 `seg_start=32475` 比目标候选的 32653 更早，说明 32117 的
`end_index` ≤ 32475（否则连这个更早的 seg_start 都够不上），进一步印证 32117 的**确认发生得相当早**
（其中枢本身的 end_index 早于 32475），但它被写入 `centers`（即被 parser/decompose 判定为「已确认」）
这个动作，直到活窗路径两次采样窗口（32720→32737 与 32759→32825）之间都还没发生——**中枢的几何完成
（end_index）与它在 `centers` 里被算法承认「已确认」（进入 `projection.seeds`）之间存在滞后**，这与
#527/#580 报告已记录的「完成可见性滞后」（中位 57.5、最大 5190 bar）是同一族现象在中枢确认这一层的
体现，不是新机制。

## 3. 判定：时序差（无害），证据锚

**判据**：若是「查询切片实质差异」，需要证明两条路径在**同一 as_of、同一 seg_start**下，用**不同**的
`centers` 集合算出不同答案——即算法输入本身不同源。若是「时序差」，两条路径用的是**同一算法、同一数据
源**，只是**从未在同一时刻对齐过查询**（一条稀疏采样一个移动目标，一条只在终局对固定目标查一次）。

- §1.2 已证明 `centers` 构造算法逐字相同、数据源（`tower[level]`）相同——排除「切片定义不同」。
- §2 的 `L1_LIVE_RECOMPUTE` 序列证明：活窗路径**从未**在 `centers` 已含 32117 的状态下查询过
  `seg_start∈{32653,32714}`（32653 的最后一次查询在 32720，32117 尚未入池；32714 的最后一次查询在
  32825，32117 仍未入池）——这是**采样时刻错过**，不是「查了但被过滤掉/查了不同池子」。
- 完成路径对 `segment.start_index=32653` 的查询发生在 `as_of=32893`（终局一次性事后查询），此刻
  `centers` 已经同时含 32117 与 32475；`nearest_confirmed_center_idx` 对**固定** `seg_start=32653`
  的答案是 32117（32475 的 start=32475>32653，不满足 `end_index≤32653`，天然出局）——这与活窗路径
  在 `seg_start` 已经前移到 32855 时选中 32475 **完全自洽**，二者是同一个 `centers` 集合在不同
  `seg_start` 下的正确不同答案，不是矛盾。

**结论**：32893 的「两路径未同序同判」现场，根因是**活窗路径的稀疏事件驱动重算 + 单一全局 frontier
只能追踪当前 pending 段**，在中枢确认滞后达到「跨越 frontier 换段」量级时，结构性地无法回看已经换段
的旧目标。这是活窗机制自身文档已承认的设计边界（`nest_lifecycle.rs:1469-1474`「不成立的是跨 C 段的
恒等」一节，此前只描述过「桥判不同身份」的后果，本票补上了**更早一步的因**：桥判不同身份之前，活窗
路径可能连一次命中都没有——`b=32117` 这一支比 `nest_lifecycle.rs` 文档原描述的「跨 C 段观测接缝」更
极端：它连**同一 C 段**内都没等到一次命中）。`nest_lifecycle.rs:1456-1458`「同序同判」的声明在**算法
定义**层面成立（§1.2），在**每一时刻的实际命中结果**层面不成立（本就不该要求成立——两条路径的采样
时刻集合本来就不相交）。**文档措辞建议订正**（见 §5-4），不涉及代码缺陷。

## 4. 影响面量化

本票精确追踪的机制（B 中枢确认滞后 + frontier 前移导致活窗路径永久错过命中窗口）与 #580 报告已归类的
**类型 α**（15984/32893/70395，3 只）是**同一机制**——#580 §1.1「`b=14709` 在活跃期内尚未确认」、
§1.6「活窗路径在该 C 段整个生命周期内只见过 68917，从未见过 69347」描述的是同一因果链的不同实例
（69347/14709 情形是「B 全程未确认」，32117 情形是「B 确认了但确认时机落在两次 frontier 前移之间的
盲区」——机制同源，只是滞后窗口相对 frontier 换段速度的比例不同，都归结到「中枢确认滞后 vs frontier
前移速度的赛跑，活窗路径必输给已经换段的旧目标」这一条通用不变式）。

**结论：不新增身份计数**。#580 §2.1/§2.3 已对 6 只（含本票 3 只 α）做过反事实量化——L1 完成前活窗覆盖
156/202=77.2%，全部 6 只若挽回的代数上界也只是 +3.0pp——本票没有发现在这 6 只之外还有第七类现象需要
BTC 100k 全量重扫。若要精确统计「B 确认滞后超过 frontier 换段窗口」这一具体子机制在全量身份里的
命中数，需要新写一个探针逐身份比对「B 中枢 end_index 相对该身份 C 段两次 frontier 切换时刻的位置」——
本票判断这项工作的**边际信息量低**（结论已经是「设计边界内的已知现象，非缺陷」，量化更多实例不改变
判定，只是让 #580 §2.3 的样本量从 3 变成某个更大但仍是「设计边界内」的数），故不建议做，除非后续要
量化活窗覆盖率提升空间的精确上界（那是 #580 §4-2 的既有推荐范围，非本票 Scope）。

## 5. 建议（不替裁）

1. **#580 §5「32893 未判定」应结项，判定为「时序差，非缺陷」**——本票 §2/§3 的 `L1_LIVE_RECOMPUTE`
   逐行证据足以支撑该只从「未判定」转为已判定，#580 §2/§3/§4 关于 6 只全部源于「当下状态与回顾性
   完成判定两条时间线不同步」的结论**对 32893 成立**（#580 §5 边界条件第一条的翻转条件未触发）。
2. **`nest_lifecycle.rs:1456-1458` 文档措辞订正**：「同序同判」在算法定义层面准确，但容易被读成
   「任一时刻的查询结果也应一致」，建议补一句限定——「同序同判」指两路径用同一 `nearest_confirmed_
   center_idx`/`locate_pan_div_structure` 算法族，不保证在不同 `seg_start`/不同时刻的查询会得到相同
   答案（这本就不该要求，两个 `seg_start` 本来就可能不同）。纯文档改动，不改行为。
3. **若未来要提升 L1 活窗覆盖率**，本票的精确因果链（B 确认滞后 + frontier 前移窗口关闭）指向与
   #580 §4-2 相同的方向——**身份桥接语义放宽**（同 C 段、B 因新中枢确认而升级时判定为身份延续而非
   两个不相关身份）是唯一能挽回类型 α 的手段，本票不改变这一推荐，只是把 32893 从「未判定」补齐为
   「已确认属于该机制」的第三个实证案例。
4. 本票不建议开新票做 BTC 100k 全量扫描量化（见 §4 理由），除非后续有明确需求要精确统计这一具体
   子机制的样本量。

## 6. 结果包六要素

1. **结论**：32893 的两路径中枢不一致是时序差（活窗路径稀疏采样错过 B 确认与 frontier 前移之间的
   窄窗口），非查询切片定义差异；`centers` 构造算法在两路径间逐字相同。
2. **定义依据**：`signal.rs:213` `nearest_confirmed_center_idx` 纯函数定义；`level_view.rs:1512`/
   `p123_fast_replay.rs:1954`/`p123_fast_replay.rs:1627-1628` 三处 `centers` 构造代码逐字比对；
   `p123_fast_replay.rs` dump 内 `L1_LIVE_RECOMPUTE`/`L1_LIVE_HIT`/`L1_LIVE_MISS`/`COMPLETION_SIGNAL`/
   `REV` 五类诊断行的 32893 现场逐行时间序列（§2）。
3. **边界条件**：若未来发现某只身份的 `centers` 构造在两路径间**输入不同的 `windows[start..end]`
   切片**（例如 run 边界判定因某种未预见的状态差异而不同），则本票「算法逐字相同」的前提失效，需
   重新判定为查询切片实质差异；本票只核实了 32893 这一现场与 §1.2 的静态代码比对，未对 6 只逐一重跑
   本票同等粒度的 `L1_LIVE_RECOMPUTE` 精确追踪（15984/70395 沿用 #580 已有的、粒度较粗的追踪结论）。
4. **下游推论**：#580 §5 32893 未判定项可结项；#580 §2/§3/§4 的既有结论（6 只全部是设计边界内现象，
   非缺陷；narrow 版全候选产窗挽回 0/6；broader 版需身份桥接语义放宽）对 32893 成立，不需要改判。
5. **谱系引用**：#523（PanLive provider 接缝缺口根因）；#527（L1 活窗实装，`nest_lifecycle.rs:1450-
   1474` 模块头「c_start 稳定性」限定表述的原始来源）；#559（条件 C2，「同锚多候选 C」初步归因）；
   #580（本票直接前置票，6 只逐条钉现场 + 32893 未判定项的登记处）；`formalization-validity-domain`
   （本票 §2/§3 为 L2 实测——复用既有 100k 回放 dump 的真实数据，非合成；§4 的「不新增身份计数」
   结论是基于 #580 已有 L2 样本的推论，未做新的全量扫描，标注为**沿用 #580 的 L2 证据**而非本票
   独立产出新 L2 证据）。
6. **影响声明**：本报告为只读研究产出，**未修改仓内任何文件**，只新增本报告本身。未跑
   `cargo build`/`cargo test`，未占用/新建 `CARGO_TARGET_DIR=/tmp/kimi-nest-target-591`（未使用，
   复用既有 dump 已足够回答票面问题）。未改 issue/map 状态，未关票，未改 roster。全程前台单线程，
   未使用 Task/Agent/后台任务。
