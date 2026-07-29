# #690 D3 递降钟口径 v2 三窗重测（#687 三裁落地）

- ticket：#690（源票 #687 三裁裁定的口径 v2 修订，接续 #671 v1 三窗实查）；工位 `/private/tmp/wt-690`，分支 `research/d3-caliber-v2`，起点 main 尖端 `b8535ee10a`。
- 执行器：claude sonnet，全程前台单线程，未派发任何子代理/后台任务。
- 结论摘要：口径 v2（父子对加方向一致约束）落地后，三窗违规率**几乎不变**——20k 85.71%（v1 83.33%）、100k 86.21%（v1 87.50%）、300k 75.63%（v1 74.17%），三窗合计 78.06%（v1 77.34%）。#687 猜想③「方向不一致污染违规率」**不成立**：方向过滤只是把 side_mismatch 对整批移出分母/分子，同方向内在违规率本身高低几乎不受影响。归属分布显示同方向下 child 被多个 parent 包含的情况在本数据集中**从未出现**（n2/n3+ 恒为 0）。按预登记判定树，三窗与合计读数均 **≥50%**，落 **高位档：D3 核销候选，待编排者签**。

## 一、基线 / 终态测试计数

| 阶段 | passed | failed | ignored |
|---|---|---|---|
| 基线（改动前） | 2550 | 0 | 138 |
| 终态（口径 v2 落地后） | 2550 | 0 | 138 |

两次计数逐位相同（未见既存失败测试）。注：本次基线计数（2550）与 #671 报告登记的基线（2544）不同，是因为 #671 之后 main 又并入了 #683/#684 等票的改动，非本票引入的漂移——本票 diff 仅动 `p126_d3_descending_clock.rs` 一个文件。

## 二、口径 v2 定义（#687 三裁，相对 v1 的差异）

1. **父子对 = `candidate_is_sub` ∧ 方向一致**（`child.key.side == parent.key.side`）。v1 中方向错配的对仍会被记入 `judged_pairs`/`violations`（用 child 侧的 side 作分桶键，父子方向可以不同）；v2 在检测到方向错配时直接 `continue`——只累加 `side_mismatch_pairs` 计数，不进入任何违规分母/分子。
2. **多重归属不强行唯一**：同方向下一个 child 可被多个 parent 包含，不去重不挑"唯一父"（v1/v2 在这一点上行为一致，本身就是笛卡尔积扫描）；v2 新增归属分布桶，把每个 child 被多少个同方向 parent 包含（n=0/1/2/3+）显式打出来。
3. **违规定义不变**：`parent.first_provable_at > child.first_provable_at`，两端均 `Some` 才入分母；`unjudged_pairs` 计至少一端为 `None` 的对。
4. **新增违规个案全量 dump**（`ISSUE690_D3_VIOLATION` 行）：每条违规打出 child/parent 的 `kind`/`interval`/`c_start`/`seg_a`（级别、side 已在同一行的 window/level/side 字段里）与两端 `first_provable_at`。

**一致性自检**：v1 与 v2 的「同方向 + 错配」总对数应相等（`candidate_is_sub` 判据本身未变，只是分流方式变了）。逐窗核对：
- 20k：v1 `judged+unjudged=12+0=12`；v2 同方向 `7+0=7`；`side_mismatch=5`；`7+5=12` ✓
- 100k：v1 `40+0=40`；v2 `29+0=29`；`side_mismatch=11`；`29+11=40` ✓
- 300k：v1 `151+7=158`；v2 `119+3=122`；`side_mismatch=36`；`122+36=158` ✓

三窗全部对上，确认 v2 实现未改变 `candidate_is_sub` 判据本身，只是重新划分了归属。

## 三、三窗对照表——v1 读数 vs v2 读数并列

| 窗口 | 口径 | judged_pairs | violations | violation_rate | unjudged_pairs | side_mismatch_pairs |
|---|---|---|---|---|---|---|
| 20,000 | v1（#671） | 12 | 10 | 0.8333 | 0 | 5 |
| 20,000 | v2（#690） | 7 | 6 | 0.8571 | 0 | 5 |
| 100,000 | v1（#671） | 40 | 35 | 0.8750 | 0 | 11 |
| 100,000 | v2（#690） | 29 | 25 | 0.8621 | 0 | 11 |
| 300,000 | v1（#671） | 151 | 112 | 0.7417 | 7 | 36 |
| 300,000 | v2（#690） | 119 | 90 | 0.7563 | 3 | 36 |
| **三窗合计** | v1 | **203** | **157** | **0.7734** | **7** | **52** |
| **三窗合计** | v2 | **155** | **121** | **0.7806** | **3** | **52** |

v2 的分母/分子都比 v1 小（因为移出了方向错配对），但**违规率本身几乎不动**（各窗差值 ≤2.4 个百分点，合计仅 +0.72pp）——方向一致这个约束把"污染源"整批筛掉了，但剩下的同方向对内部违规比例和被筛掉之前的整体比例几乎一样高，说明方向错配**不是**造成高违规率的主因，D3 递降现象在同方向对里同样稳固存在。

### 三窗 v2 分级别桶明细

| 窗口 | child→parent | side | judged_pairs | violations | violation_rate | unjudged_pairs |
|---|---|---|---|---|---|---|
| 20,000 | L0→L1 | Long | 1 | 1 | 1.0000 | 0 |
| 20,000 | L0→L1 | Short | 5 | 4 | 0.8000 | 0 |
| 20,000 | L1→L2 | Short | 1 | 1 | 1.0000 | 0 |
| 100,000 | L0→L1 | Long | 6 | 5 | 0.8333 | 0 |
| 100,000 | L0→L1 | Short | 17 | 15 | 0.8824 | 0 |
| 100,000 | L1→L2 | Long | 1 | 1 | 1.0000 | 0 |
| 100,000 | L1→L2 | Short | 5 | 4 | 0.8000 | 0 |
| 300,000 | L0→L1 | Long | 29 | 27 | 0.9310 | 1 |
| 300,000 | L0→L1 | Short | 67 | 45 | 0.6716 | 1 |
| 300,000 | L1→L2 | Long | 4 | 4 | 1.0000 | 0 |
| 300,000 | L1→L2 | Short | 18 | 13 | 0.7222 | 1 |
| 300,000 | L2→L3 | Long | 1 | 1 | 1.0000 | 0 |

## 四、归属分布（#687 裁定②，n=0/1/2/3+ 各多少 child）

| 窗口 | children_total | n0（0 个同方向 parent 包含） | n1（1 个） | n2（2 个） | n3+（≥3 个） |
|---|---|---|---|---|---|
| 20,000 | 41 | 34 | 7 | 0 | 0 |
| 100,000 | 234 | 205 | 29 | 0 | 0 |
| 300,000 | 740 | 618 | 122 | 0 | 0 |

`children_total` = 全体 `by_level` 中的 child 事件数（含最高级别、无上一级可对比的 child，此类恒记 n0）；`n1` 之和恰等于对应窗口 v2 `judged_pairs+unjudged_pairs`（20k: 7，100k: 29，300k: 122——因为一个 child 被恰好 1 个同方向 parent 包含时，笛卡尔积扫描下这个 child 只产生 1 对）。

**读数**：三窗 n2/n3+ 恒为 0——本数据集中，同方向约束下从未出现一个 child 被两个或以上 parent 同时包含的情况。#687 裁定②预留的"多重归属"口子在 BTC 全量数据上目前是空集，不代表口子本身不必要（生产判据不强行唯一是对的，只是这批数据没触发）。

## 五、违规个案 dump（#687 裁定③，全量）

三窗共 121 条违规（20k=6，100k=25，300k=90），逐条打出 child/parent 键标识与两端首证钟。完整原始输出（含 `ISSUE690_D3_VIOLATION`/`ISSUE690_D3_BUCKET`/`ISSUE690_D3_WINDOW`/`ISSUE690_D3_ATTRIBUTION` 全部 139 行）附于本节末尾代码块，逐位保留探针原始打印，未做任何筛选或改写。

**粗读观察**（非逐例初判——判定树落在高位档，按票面规则不强制个案钉因；此处只记录一个跨窗口反复出现的结构模式，供编排者参考）：dump 中相当比例的违规对，child 的区间起点与 parent 的区间起点**相同**（如 20k 第 1 条 `child_interval=(5622,5657)` vs `parent_interval=(5622,5905)`，两者 `c_start` 均为 5622）——即父子共享同一个 C 段起点、父级只是延伸得更晚结束。这类对里"父级晚证成"至少部分是**父级区间本身更晚封口**的直接几何结果（父级要等更多 bar 才能把自己的结构谓词证完），而不必然是独立的时序倒置；但 dump 中也有 child 与 parent 起点不同、纯粹靠区间包含关系配对的违规（如 20k 第 2 条 `child_interval=(10780,10949)` vs `parent_interval=(10750,11233)`，起点不同），这类不能用"共享起点"来解释。两类violation 混在同一违规率里，是否需要按"起点是否共享"进一步拆分子桶，本票不做，留给编排者按需下达。

<details>
<summary>三窗完整原始输出（139 行，点击展开）</summary>

```
ISSUE690_D3_VIOLATION window_bars=20000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(5622,5657) child_c_start=5622 child_seg_a=(4398,4756) child_first_provable_at=5657 parent_kind=Pan parent_interval=(5622,5905) parent_c_start=5622 parent_seg_a=(2915,3223) parent_first_provable_at=5905
ISSUE690_D3_VIOLATION window_bars=20000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(10780,10949) child_c_start=10780 child_seg_a=(10051,10115) child_first_provable_at=10949 parent_kind=Pan parent_interval=(10750,11233) parent_c_start=10750 parent_seg_a=(3484,5119) parent_first_provable_at=11233
ISSUE690_D3_VIOLATION window_bars=20000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(14389,14567) child_c_start=14389 child_seg_a=(12232,12285) child_first_provable_at=14567 parent_kind=Pan parent_interval=(14193,14705) parent_c_start=14193 parent_seg_a=(8073,9253) parent_first_provable_at=14705
ISSUE690_D3_VIOLATION window_bars=20000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(16724,16863) child_c_start=16724 child_seg_a=(16182,16275) child_first_provable_at=16863 parent_kind=Pan parent_interval=(16724,17018) parent_c_start=16724 parent_seg_a=(14193,14705) parent_first_provable_at=17018
ISSUE690_D3_VIOLATION window_bars=20000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(18390,18447) child_c_start=18390 child_seg_a=(17514,17581) child_first_provable_at=18447 parent_kind=Pan parent_interval=(18390,19156) parent_c_start=18390 parent_seg_a=(16283,16714) parent_first_provable_at=19156
ISSUE690_D3_VIOLATION window_bars=20000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(16724,17018) child_c_start=16724 child_seg_a=(14193,14705) child_first_provable_at=17018 parent_kind=Pan parent_interval=(16724,18387) parent_c_start=16724 parent_seg_a=(3223,5622) parent_first_provable_at=18387
ISSUE690_D3_BUCKET window_bars=20000 child_level=0 parent_level=1 side=Long judged_pairs=1 violations=1 violation_rate=1.000000 unjudged_pairs=0
ISSUE690_D3_BUCKET window_bars=20000 child_level=0 parent_level=1 side=Short judged_pairs=5 violations=4 violation_rate=0.800000 unjudged_pairs=0
ISSUE690_D3_BUCKET window_bars=20000 child_level=1 parent_level=2 side=Short judged_pairs=1 violations=1 violation_rate=1.000000 unjudged_pairs=0
ISSUE690_D3_WINDOW window_bars=20000 levels=3 judged_pairs=7 violations=6 violation_rate=0.857143 unjudged_pairs=0 side_mismatch_pairs=5
ISSUE690_D3_ATTRIBUTION window_bars=20000 children_total=41 n0=34 n1=7 n2=0 n3_plus=0
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(5622,5657) child_c_start=5622 child_seg_a=(4398,4756) child_first_provable_at=5657 parent_kind=Pan parent_interval=(5622,5905) parent_c_start=5622 parent_seg_a=(2915,3223) parent_first_provable_at=5905
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(35737,35832) child_c_start=35737 child_seg_a=(34638,34846) child_first_provable_at=35832 parent_kind=Pan parent_interval=(35687,36254) parent_c_start=35687 parent_seg_a=(34191,34632) parent_first_provable_at=36254
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(38525,38564) child_c_start=38525 child_seg_a=(37448,37609) child_first_provable_at=38564 parent_kind=Pan parent_interval=(38525,38704) parent_c_start=38525 parent_seg_a=(35687,36254) parent_first_provable_at=38704
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(66983,67110) child_c_start=66983 child_seg_a=(66205,66348) child_first_provable_at=67110 parent_kind=Pan parent_interval=(66937,67371) parent_c_start=66937 parent_seg_a=(60489,61430) parent_first_provable_at=67371
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(97461,97484) child_c_start=97461 child_seg_a=(96500,96581) child_first_provable_at=97484 parent_kind=Pan parent_interval=(97408,97647) parent_c_start=97408 parent_seg_a=(96339,96581) parent_first_provable_at=97647
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(10780,10949) child_c_start=10780 child_seg_a=(10051,10115) child_first_provable_at=10949 parent_kind=Pan parent_interval=(10750,11233) parent_c_start=10750 parent_seg_a=(3484,5119) parent_first_provable_at=11233
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(14389,14567) child_c_start=14389 child_seg_a=(12232,12285) child_first_provable_at=14567 parent_kind=Pan parent_interval=(14193,14705) parent_c_start=14193 parent_seg_a=(8073,9253) parent_first_provable_at=14705
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(16724,16863) child_c_start=16724 child_seg_a=(16182,16275) child_first_provable_at=16863 parent_kind=Pan parent_interval=(16724,17018) parent_c_start=16724 parent_seg_a=(14193,14705) parent_first_provable_at=17018
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(18390,18447) child_c_start=18390 child_seg_a=(17514,17581) child_first_provable_at=18447 parent_kind=Pan parent_interval=(18390,19156) parent_c_start=18390 parent_seg_a=(16283,16714) parent_first_provable_at=19156
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(28343,28499) child_c_start=28343 child_seg_a=(27455,27633) child_first_provable_at=28499 parent_kind=Pan parent_interval=(28182,29164) parent_c_start=28182 parent_seg_a=(25303,25759) parent_first_provable_at=29164
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(36765,36851) child_c_start=36765 child_seg_a=(36254,36611) child_first_provable_at=36851 parent_kind=Pan parent_interval=(36413,37195) parent_c_start=36413 parent_seg_a=(32820,33270) parent_first_provable_at=37195
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(53182,53263) child_c_start=53182 child_seg_a=(52475,52545) child_first_provable_at=53263 parent_kind=Pan parent_interval=(53012,53427) parent_c_start=53012 parent_seg_a=(48941,49608) parent_first_provable_at=53427
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(56425,56457) child_c_start=56425 child_seg_a=(55578,55632) child_first_provable_at=56457 parent_kind=Pan parent_interval=(56425,56635) parent_c_start=56425 parent_seg_a=(52545,52957) parent_first_provable_at=56635
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(63289,63356) child_c_start=63289 child_seg_a=(61825,62036) child_first_provable_at=63356 parent_kind=Pan parent_interval=(63188,63877) parent_c_start=63188 parent_seg_a=(58899,59517) parent_first_provable_at=63877
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(65516,65537) child_c_start=65516 child_seg_a=(65046,65096) child_first_provable_at=65537 parent_kind=Pan parent_interval=(65516,65618) parent_c_start=65516 parent_seg_a=(62036,63182) parent_first_provable_at=65618
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(71123,71195) child_c_start=71123 child_seg_a=(69999,70322) child_first_provable_at=71195 parent_kind=Pan parent_interval=(71033,71236) parent_c_start=71033 parent_seg_a=(66433,66922) parent_first_provable_at=71236
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(74348,74384) child_c_start=74348 child_seg_a=(73107,73334) child_first_provable_at=74384 parent_kind=Pan parent_interval=(74053,74488) parent_c_start=74053 parent_seg_a=(70396,71033) parent_first_provable_at=74488
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(86483,86799) child_c_start=86483 child_seg_a=(85876,86082) child_first_provable_at=86799 parent_kind=Pan parent_interval=(86082,87258) parent_c_start=86082 parent_seg_a=(84283,84552) parent_first_provable_at=87258
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(86836,86948) child_c_start=86836 child_seg_a=(86483,86799) child_first_provable_at=86948 parent_kind=Pan parent_interval=(86082,87258) parent_c_start=86082 parent_seg_a=(84283,84552) parent_first_provable_at=87258
ISSUE690_D3_VIOLATION window_bars=100000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(92534,92570) child_c_start=92534 child_seg_a=(91362,91581) child_first_provable_at=92570 parent_kind=Pan parent_interval=(92416,92905) parent_c_start=92416 parent_seg_a=(89953,90164) parent_first_provable_at=92905
ISSUE690_D3_VIOLATION window_bars=100000 child_level=1 parent_level=2 side=Long child_kind=Pan child_interval=(38525,38704) child_c_start=38525 child_seg_a=(35687,36254) child_first_provable_at=38704 parent_kind=Pan parent_interval=(38525,40390) parent_c_start=38525 parent_seg_a=(25782,28179) parent_first_provable_at=40390
ISSUE690_D3_VIOLATION window_bars=100000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(16724,17018) child_c_start=16724 child_seg_a=(14193,14705) child_first_provable_at=17018 parent_kind=Pan parent_interval=(16724,18387) parent_c_start=16724 parent_seg_a=(3223,5622) parent_first_provable_at=18387
ISSUE690_D3_VIOLATION window_bars=100000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(28182,29164) child_c_start=28182 child_seg_a=(25303,25759) child_first_provable_at=29164 parent_kind=Pan parent_interval=(28182,30430) parent_c_start=28182 parent_seg_a=(16724,23954) parent_first_provable_at=30430
ISSUE690_D3_VIOLATION window_bars=100000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(56425,56635) child_c_start=56425 child_seg_a=(52545,52957) child_first_provable_at=56635 parent_kind=Pan parent_interval=(56425,59517) parent_c_start=56425 parent_seg_a=(36413,38525) parent_first_provable_at=59517
ISSUE690_D3_VIOLATION window_bars=100000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(74053,74488) child_c_start=74053 child_seg_a=(70396,71033) child_first_provable_at=74488 parent_kind=Pan parent_interval=(74053,75281) parent_c_start=74053 parent_seg_a=(56425,59517) parent_first_provable_at=75281
ISSUE690_D3_BUCKET window_bars=100000 child_level=0 parent_level=1 side=Long judged_pairs=6 violations=5 violation_rate=0.833333 unjudged_pairs=0
ISSUE690_D3_BUCKET window_bars=100000 child_level=0 parent_level=1 side=Short judged_pairs=17 violations=15 violation_rate=0.882353 unjudged_pairs=0
ISSUE690_D3_BUCKET window_bars=100000 child_level=1 parent_level=2 side=Long judged_pairs=1 violations=1 violation_rate=1.000000 unjudged_pairs=0
ISSUE690_D3_BUCKET window_bars=100000 child_level=1 parent_level=2 side=Short judged_pairs=5 violations=4 violation_rate=0.800000 unjudged_pairs=0
ISSUE690_D3_WINDOW window_bars=100000 levels=3 judged_pairs=29 violations=25 violation_rate=0.862069 unjudged_pairs=0 side_mismatch_pairs=11
ISSUE690_D3_ATTRIBUTION window_bars=100000 children_total=234 n0=205 n1=29 n2=0 n3_plus=0
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Trend child_interval=(295851,295878) child_c_start=295851 child_seg_a=(294678,295735) child_first_provable_at=295878 parent_kind=Pan parent_interval=(295782,296059) parent_c_start=295782 parent_seg_a=(288681,288992) parent_first_provable_at=296059
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(5622,5657) child_c_start=5622 child_seg_a=(4398,4756) child_first_provable_at=5657 parent_kind=Pan parent_interval=(5622,5905) parent_c_start=5622 parent_seg_a=(2915,3223) parent_first_provable_at=5905
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(35737,35832) child_c_start=35737 child_seg_a=(34638,34846) child_first_provable_at=35832 parent_kind=Pan parent_interval=(35687,36254) parent_c_start=35687 parent_seg_a=(34191,34632) parent_first_provable_at=36254
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(38525,38564) child_c_start=38525 child_seg_a=(37448,37609) child_first_provable_at=38564 parent_kind=Pan parent_interval=(38525,38704) parent_c_start=38525 parent_seg_a=(35687,36254) parent_first_provable_at=38704
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(66983,67110) child_c_start=66983 child_seg_a=(66205,66348) child_first_provable_at=67110 parent_kind=Pan parent_interval=(66937,67371) parent_c_start=66937 parent_seg_a=(60489,61430) parent_first_provable_at=67371
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(97461,97484) child_c_start=97461 child_seg_a=(96500,96581) child_first_provable_at=97484 parent_kind=Pan parent_interval=(97408,97647) parent_c_start=97408 parent_seg_a=(96339,96581) parent_first_provable_at=97647
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(115967,115992) child_c_start=115967 child_seg_a=(115039,115127) child_first_provable_at=115992 parent_kind=Pan parent_interval=(115859,117057) parent_c_start=115859 parent_seg_a=(111255,111890) parent_first_provable_at=117057
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(116348,116404) child_c_start=116348 child_seg_a=(115654,115848) child_first_provable_at=116404 parent_kind=Pan parent_interval=(115859,117057) parent_c_start=115859 parent_seg_a=(111255,111890) parent_first_provable_at=117057
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(116741,116862) child_c_start=116741 child_seg_a=(116348,116586) child_first_provable_at=116862 parent_kind=Pan parent_interval=(115859,117057) parent_c_start=115859 parent_seg_a=(111255,111890) parent_first_provable_at=117057
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(117066,117150) child_c_start=117066 child_seg_a=(116079,116219) child_first_provable_at=117150 parent_kind=Pan parent_interval=(117066,117274) parent_c_start=117066 parent_seg_a=(112985,113982) parent_first_provable_at=117274
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(124611,124683) child_c_start=124611 child_seg_a=(123718,123878) child_first_provable_at=124683 parent_kind=Pan parent_interval=(124576,124866) parent_c_start=124576 parent_seg_a=(120891,121867) parent_first_provable_at=124866
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(170778,170938) child_c_start=170778 child_seg_a=(170026,170186) child_first_provable_at=170938 parent_kind=Pan parent_interval=(170778,171269) parent_c_start=170778 parent_seg_a=(168360,169366) parent_first_provable_at=171269
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(185276,185306) child_c_start=185276 child_seg_a=(184185,184324) child_first_provable_at=185306 parent_kind=Pan parent_interval=(184839,185357) parent_c_start=184839 parent_seg_a=(182584,183061) parent_first_provable_at=185357
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(206838,206867) child_c_start=206838 child_seg_a=(205949,206058) child_first_provable_at=206867 parent_kind=Pan parent_interval=(206740,207088) parent_c_start=206740 parent_seg_a=(201348,201950) parent_first_provable_at=207088
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(209379,209464) child_c_start=209379 child_seg_a=(208668,208765) child_first_provable_at=209464 parent_kind=Pan parent_interval=(209352,209509) parent_c_start=209352 parent_seg_a=(206075,206731) parent_first_provable_at=209509
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(212715,212768) child_c_start=212715 child_seg_a=(212407,212613) child_first_provable_at=212768 parent_kind=Pan parent_interval=(212041,212919) parent_c_start=212041 parent_seg_a=(211099,211630) parent_first_provable_at=212919
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(214816,214873) child_c_start=214816 child_seg_a=(214542,214607) child_first_provable_at=214873 parent_kind=Pan parent_interval=(214816,215498) parent_c_start=214816 parent_seg_a=(212041,212919) parent_first_provable_at=215498
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(214909,214972) child_c_start=214909 child_seg_a=(214816,214873) child_first_provable_at=214972 parent_kind=Pan parent_interval=(214816,215498) parent_c_start=214816 parent_seg_a=(212041,212919) parent_first_provable_at=215498
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(215068,215157) child_c_start=215068 child_seg_a=(214672,214733) child_first_provable_at=215157 parent_kind=Pan parent_interval=(214816,215498) parent_c_start=214816 parent_seg_a=(212041,212919) parent_first_provable_at=215498
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(222411,222433) child_c_start=222411 child_seg_a=(221268,221657) child_first_provable_at=222433 parent_kind=Pan parent_interval=(221804,222948) parent_c_start=221804 parent_seg_a=(217698,218165) parent_first_provable_at=222948
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(245463,245633) child_c_start=245463 child_seg_a=(244661,244906) child_first_provable_at=245633 parent_kind=Pan parent_interval=(244913,245966) parent_c_start=244913 parent_seg_a=(243087,243633) parent_first_provable_at=245966
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(246759,246810) child_c_start=246759 child_seg_a=(246063,246128) child_first_provable_at=246810 parent_kind=Pan parent_interval=(246632,246944) parent_c_start=246632 parent_seg_a=(244219,244439) parent_first_provable_at=246944
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(253056,253136) child_c_start=253056 child_seg_a=(252419,252462) child_first_provable_at=253136 parent_kind=Pan parent_interval=(252952,253735) parent_c_start=252952 parent_seg_a=(248268,249030) parent_first_provable_at=253735
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(272700,272783) child_c_start=272700 child_seg_a=(271776,271896) child_first_provable_at=272783 parent_kind=Pan parent_interval=(272700,272891) parent_c_start=272700 parent_seg_a=(268661,269076) parent_first_provable_at=272891
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(272988,273001) child_c_start=272988 child_seg_a=(272467,272544) child_first_provable_at=273001 parent_kind=Pan parent_interval=(272937,273034) parent_c_start=272937 parent_seg_a=(270757,271101) parent_first_provable_at=273034
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(275474,275498) child_c_start=275474 child_seg_a=(274746,274835) child_first_provable_at=275498 parent_kind=Pan parent_interval=(275423,275547) parent_c_start=275423 parent_seg_a=(272700,272891) parent_first_provable_at=275547
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(284239,284304) child_c_start=284239 child_seg_a=(283708,283895) child_first_provable_at=284304 parent_kind=Pan parent_interval=(284239,284545) parent_c_start=284239 parent_seg_a=(281658,281995) parent_first_provable_at=284545
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Long child_kind=Pan child_interval=(298416,298509) child_c_start=298416 child_seg_a=(297895,297972) child_first_provable_at=298509 parent_kind=Pan parent_interval=(298329,298663) parent_c_start=298329 parent_seg_a=(297515,297760) parent_first_provable_at=298663
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(10780,10949) child_c_start=10780 child_seg_a=(10051,10115) child_first_provable_at=10949 parent_kind=Pan parent_interval=(10750,11233) parent_c_start=10750 parent_seg_a=(3484,5119) parent_first_provable_at=11233
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(14389,14567) child_c_start=14389 child_seg_a=(12232,12285) child_first_provable_at=14567 parent_kind=Pan parent_interval=(14193,14705) parent_c_start=14193 parent_seg_a=(8073,9253) parent_first_provable_at=14705
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(16724,16863) child_c_start=16724 child_seg_a=(16182,16275) child_first_provable_at=16863 parent_kind=Pan parent_interval=(16724,17018) parent_c_start=16724 parent_seg_a=(14193,14705) parent_first_provable_at=17018
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(18390,18447) child_c_start=18390 child_seg_a=(17514,17581) child_first_provable_at=18447 parent_kind=Pan parent_interval=(18390,19156) parent_c_start=18390 parent_seg_a=(16283,16714) parent_first_provable_at=19156
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(28343,28499) child_c_start=28343 child_seg_a=(27455,27633) child_first_provable_at=28499 parent_kind=Pan parent_interval=(28182,29164) parent_c_start=28182 parent_seg_a=(25303,25759) parent_first_provable_at=29164
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(36765,36851) child_c_start=36765 child_seg_a=(36254,36611) child_first_provable_at=36851 parent_kind=Pan parent_interval=(36413,37195) parent_c_start=36413 parent_seg_a=(32820,33270) parent_first_provable_at=37195
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(53182,53263) child_c_start=53182 child_seg_a=(52475,52545) child_first_provable_at=53263 parent_kind=Pan parent_interval=(53012,53427) parent_c_start=53012 parent_seg_a=(48941,49608) parent_first_provable_at=53427
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(56425,56457) child_c_start=56425 child_seg_a=(55578,55632) child_first_provable_at=56457 parent_kind=Pan parent_interval=(56425,56635) parent_c_start=56425 parent_seg_a=(52545,52957) parent_first_provable_at=56635
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(63289,63356) child_c_start=63289 child_seg_a=(61825,62036) child_first_provable_at=63356 parent_kind=Pan parent_interval=(63188,63877) parent_c_start=63188 parent_seg_a=(58899,59517) parent_first_provable_at=63877
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(65516,65537) child_c_start=65516 child_seg_a=(65046,65096) child_first_provable_at=65537 parent_kind=Pan parent_interval=(65516,65618) parent_c_start=65516 parent_seg_a=(62036,63182) parent_first_provable_at=65618
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(71123,71195) child_c_start=71123 child_seg_a=(69999,70322) child_first_provable_at=71195 parent_kind=Pan parent_interval=(71033,71236) parent_c_start=71033 parent_seg_a=(66433,66922) parent_first_provable_at=71236
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(74348,74384) child_c_start=74348 child_seg_a=(73107,73334) child_first_provable_at=74384 parent_kind=Pan parent_interval=(74053,74488) parent_c_start=74053 parent_seg_a=(70396,71033) parent_first_provable_at=74488
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(86483,86799) child_c_start=86483 child_seg_a=(85876,86082) child_first_provable_at=86799 parent_kind=Pan parent_interval=(86082,87258) parent_c_start=86082 parent_seg_a=(84283,84552) parent_first_provable_at=87258
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(86836,86948) child_c_start=86836 child_seg_a=(86483,86799) child_first_provable_at=86948 parent_kind=Pan parent_interval=(86082,87258) parent_c_start=86082 parent_seg_a=(84283,84552) parent_first_provable_at=87258
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(92534,92570) child_c_start=92534 child_seg_a=(91362,91581) child_first_provable_at=92570 parent_kind=Pan parent_interval=(92416,92905) parent_c_start=92416 parent_seg_a=(89953,90164) parent_first_provable_at=92905
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(104792,104846) child_c_start=104792 child_seg_a=(103613,103786) child_first_provable_at=104846 parent_kind=Pan parent_interval=(104750,105327) parent_c_start=104750 parent_seg_a=(102886,103208) parent_first_provable_at=105327
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(114982,115039) child_c_start=114982 child_seg_a=(114428,114590) child_first_provable_at=115039 parent_kind=Pan parent_interval=(114982,115351) parent_c_start=114982 parent_seg_a=(110896,111255) parent_first_provable_at=115351
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(128825,129023) child_c_start=128825 child_seg_a=(128130,128348) child_first_provable_at=129023 parent_kind=Pan parent_interval=(128825,129269) parent_c_start=128825 parent_seg_a=(123300,123878) parent_first_provable_at=129269
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(133678,133764) child_c_start=133678 child_seg_a=(133299,133339) child_first_provable_at=133764 parent_kind=Trend parent_interval=(133678,135840) parent_c_start=133678 parent_seg_a=(128825,132020) parent_first_provable_at=135840
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(134297,134367) child_c_start=134297 child_seg_a=(133508,133546) child_first_provable_at=134367 parent_kind=Trend parent_interval=(133678,135840) parent_c_start=133678 parent_seg_a=(128825,132020) parent_first_provable_at=135840
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(135330,135366) child_c_start=135330 child_seg_a=(134297,134367) child_first_provable_at=135366 parent_kind=Trend parent_interval=(133678,135840) parent_c_start=133678 parent_seg_a=(128825,132020) parent_first_provable_at=135840
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(140455,140477) child_c_start=140455 child_seg_a=(139615,139655) child_first_provable_at=140477 parent_kind=Pan parent_interval=(140422,140947) parent_c_start=140422 parent_seg_a=(135190,135840) parent_first_provable_at=140947
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(155050,155153) child_c_start=155050 child_seg_a=(154293,154539) child_first_provable_at=155153 parent_kind=Pan parent_interval=(155050,155267) parent_c_start=155050 parent_seg_a=(152164,152817) parent_first_provable_at=155267
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(165961,166072) child_c_start=165961 child_seg_a=(165041,165226) child_first_provable_at=166072 parent_kind=Pan parent_interval=(165763,166590) parent_c_start=165763 parent_seg_a=(161384,162184) parent_first_provable_at=166590
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(174573,174598) child_c_start=174573 child_seg_a=(173346,173457) child_first_provable_at=174598 parent_kind=Trend parent_interval=(174527,175928) parent_c_start=174527 parent_seg_a=(172192,174527) parent_first_provable_at=174784
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(187728,187848) child_c_start=187728 child_seg_a=(186713,186792) child_first_provable_at=187848 parent_kind=Pan parent_interval=(187631,188242) parent_c_start=187631 parent_seg_a=(184493,184816) parent_first_provable_at=188242
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(187999,188067) child_c_start=187999 child_seg_a=(187728,187848) child_first_provable_at=188067 parent_kind=Pan parent_interval=(187631,188242) parent_c_start=187631 parent_seg_a=(184493,184816) parent_first_provable_at=188242
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(196242,196291) child_c_start=196242 child_seg_a=(195684,195787) child_first_provable_at=196291 parent_kind=Pan parent_interval=(196109,196538) parent_c_start=196109 parent_seg_a=(193222,193510) parent_first_provable_at=196538
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(202926,203052) child_c_start=202926 child_seg_a=(201705,201950) child_first_provable_at=203052 parent_kind=Trend parent_interval=(202739,205339) parent_c_start=202739 parent_seg_a=(199107,202731) parent_first_provable_at=203325
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(210402,210437) child_c_start=210402 child_seg_a=(209542,209597) child_first_provable_at=210437 parent_kind=Pan parent_interval=(210344,210562) parent_c_start=210344 parent_seg_a=(208499,208858) parent_first_provable_at=210562
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(211638,211702) child_c_start=211638 child_seg_a=(210740,210812) child_first_provable_at=211702 parent_kind=Pan parent_interval=(211638,212031) parent_c_start=211638 parent_seg_a=(208499,208858) parent_first_provable_at=212031
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(212985,213013) child_c_start=212985 child_seg_a=(211868,211902) child_first_provable_at=213013 parent_kind=Pan parent_interval=(212940,214078) parent_c_start=212940 parent_seg_a=(210680,211090) parent_first_provable_at=214078
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(213604,213788) child_c_start=213604 child_seg_a=(212768,212919) child_first_provable_at=213788 parent_kind=Pan parent_interval=(212940,214078) parent_c_start=212940 parent_seg_a=(210680,211090) parent_first_provable_at=214078
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(214078,214163) child_c_start=214078 child_seg_a=(213604,213788) child_first_provable_at=214163 parent_kind=Pan parent_interval=(214078,214323) parent_c_start=214078 parent_seg_a=(211638,212031) parent_first_provable_at=214323
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(217387,217448) child_c_start=217387 child_seg_a=(216325,216390) child_first_provable_at=217448 parent_kind=Pan parent_interval=(217387,217698) parent_c_start=217387 parent_seg_a=(214323,214607) parent_first_provable_at=217698
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(224709,224810) child_c_start=224709 child_seg_a=(224200,224290) child_first_provable_at=224810 parent_kind=Pan parent_interval=(224709,225111) parent_c_start=224709 parent_seg_a=(220714,221801) parent_first_provable_at=225111
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(251690,251757) child_c_start=251690 child_seg_a=(250085,250343) child_first_provable_at=251757 parent_kind=Pan parent_interval=(251655,252100) parent_c_start=251655 parent_seg_a=(244455,244906) parent_first_provable_at=252100
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(255182,255205) child_c_start=255182 child_seg_a=(254248,254422) child_first_provable_at=255205 parent_kind=Pan parent_interval=(254836,256220) parent_c_start=254836 parent_seg_a=(250368,251617) parent_first_provable_at=256220
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(255406,255431) child_c_start=255406 child_seg_a=(254616,254675) child_first_provable_at=255431 parent_kind=Pan parent_interval=(254836,256220) parent_c_start=254836 parent_seg_a=(250368,251617) parent_first_provable_at=256220
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(258031,258229) child_c_start=258031 child_seg_a=(257114,257348) child_first_provable_at=258229 parent_kind=Pan parent_interval=(257991,258504) parent_c_start=257991 parent_seg_a=(255205,256220) parent_first_provable_at=258504
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(259570,259715) child_c_start=259570 child_seg_a=(259048,259114) child_first_provable_at=259715 parent_kind=Trend parent_interval=(259538,261080) parent_c_start=259538 parent_seg_a=(257991,259538) parent_first_provable_at=259835
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(271496,271554) child_c_start=271496 child_seg_a=(270978,271101) child_first_provable_at=271554 parent_kind=Pan parent_interval=(271496,271757) parent_c_start=271496 parent_seg_a=(269402,269888) parent_first_provable_at=271757
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(276991,277118) child_c_start=276991 child_seg_a=(276235,276280) child_first_provable_at=277118 parent_kind=Pan parent_interval=(276991,277358) parent_c_start=276991 parent_seg_a=(274844,275400) parent_first_provable_at=277358
ISSUE690_D3_VIOLATION window_bars=300000 child_level=0 parent_level=1 side=Short child_kind=Pan child_interval=(296519,296654) child_c_start=296519 child_seg_a=(296370,296414) child_first_provable_at=296654 parent_kind=Pan parent_interval=(296519,296883) parent_c_start=296519 parent_seg_a=(294678,295735) parent_first_provable_at=296883
ISSUE690_D3_VIOLATION window_bars=300000 child_level=1 parent_level=2 side=Long child_kind=Pan child_interval=(38525,38704) child_c_start=38525 child_seg_a=(35687,36254) child_first_provable_at=38704 parent_kind=Pan parent_interval=(38525,40390) parent_c_start=38525 parent_seg_a=(25782,28179) parent_first_provable_at=40390
ISSUE690_D3_VIOLATION window_bars=300000 child_level=1 parent_level=2 side=Long child_kind=Pan child_interval=(209352,209509) child_c_start=209352 child_seg_a=(206075,206731) child_first_provable_at=209509 parent_kind=Pan parent_interval=(209352,210320) parent_c_start=209352 parent_seg_a=(196662,198793) parent_first_provable_at=210320
ISSUE690_D3_VIOLATION window_bars=300000 child_level=1 parent_level=2 side=Long child_kind=Pan child_interval=(246632,246944) child_c_start=246632 child_seg_a=(244219,244439) child_first_provable_at=246944 parent_kind=Pan parent_interval=(246632,247741) parent_c_start=246632 parent_seg_a=(239118,241571) parent_first_provable_at=247741
ISSUE690_D3_VIOLATION window_bars=300000 child_level=1 parent_level=2 side=Long child_kind=Pan child_interval=(252952,253735) child_c_start=252952 child_seg_a=(248268,249030) child_first_provable_at=253735 parent_kind=Pan parent_interval=(252952,256220) parent_c_start=252952 parent_seg_a=(244913,246574) parent_first_provable_at=256220
ISSUE690_D3_VIOLATION window_bars=300000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(16724,17018) child_c_start=16724 child_seg_a=(14193,14705) child_first_provable_at=17018 parent_kind=Pan parent_interval=(16724,18387) parent_c_start=16724 parent_seg_a=(3223,5622) parent_first_provable_at=18387
ISSUE690_D3_VIOLATION window_bars=300000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(28182,29164) child_c_start=28182 child_seg_a=(25303,25759) child_first_provable_at=29164 parent_kind=Pan parent_interval=(28182,30430) parent_c_start=28182 parent_seg_a=(16724,23954) parent_first_provable_at=30430
ISSUE690_D3_VIOLATION window_bars=300000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(56425,56635) child_c_start=56425 child_seg_a=(52545,52957) child_first_provable_at=56635 parent_kind=Pan parent_interval=(56425,59517) parent_c_start=56425 parent_seg_a=(36413,38525) parent_first_provable_at=59517
ISSUE690_D3_VIOLATION window_bars=300000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(74053,74488) child_c_start=74053 child_seg_a=(70396,71033) child_first_provable_at=74488 parent_kind=Pan parent_interval=(74053,75281) parent_c_start=74053 parent_seg_a=(56425,59517) parent_first_provable_at=75281
ISSUE690_D3_VIOLATION window_bars=300000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(101011,101698) child_c_start=101011 child_seg_a=(95826,96280) child_first_provable_at=101698 parent_kind=Trend parent_interval=(101011,105327) parent_c_start=101011 parent_seg_a=(74053,87502) parent_first_provable_at=105327
ISSUE690_D3_VIOLATION window_bars=300000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(102886,103208) child_c_start=102886 child_seg_a=(101011,101698) child_first_provable_at=103208 parent_kind=Trend parent_interval=(101011,105327) parent_c_start=101011 parent_seg_a=(74053,87502) parent_first_provable_at=105327
ISSUE690_D3_VIOLATION window_bars=300000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(105684,105904) child_c_start=105684 child_seg_a=(100498,101011) child_first_provable_at=105904 parent_kind=Pan parent_interval=(105684,108097) parent_c_start=105684 parent_seg_a=(86082,87502) parent_first_provable_at=108097
ISSUE690_D3_VIOLATION window_bars=300000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(152844,153285) child_c_start=152844 child_seg_a=(150804,151192) child_first_provable_at=153285 parent_kind=Trend parent_interval=(152844,158776) parent_c_start=152844 parent_seg_a=(145672,151192) parent_first_provable_at=156189
ISSUE690_D3_VIOLATION window_bars=300000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(155050,155267) child_c_start=155050 child_seg_a=(152164,152817) child_first_provable_at=155267 parent_kind=Trend parent_interval=(152844,158776) parent_c_start=152844 parent_seg_a=(145672,151192) parent_first_provable_at=156189
ISSUE690_D3_VIOLATION window_bars=300000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(256273,256837) child_c_start=256273 child_seg_a=(252110,252462) child_first_provable_at=256837 parent_kind=Pan parent_interval=(256273,259538) parent_c_start=256273 parent_seg_a=(233342,235748) parent_first_provable_at=259538
ISSUE690_D3_VIOLATION window_bars=300000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(257424,257978) child_c_start=257424 child_seg_a=(256273,256837) child_first_provable_at=257978 parent_kind=Pan parent_interval=(256273,259538) parent_c_start=256273 parent_seg_a=(233342,235748) parent_first_provable_at=259538
ISSUE690_D3_VIOLATION window_bars=300000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(257991,258504) child_c_start=257991 child_seg_a=(255205,256220) child_first_provable_at=258504 parent_kind=Pan parent_interval=(256273,259538) parent_c_start=256273 parent_seg_a=(233342,235748) parent_first_provable_at=259538
ISSUE690_D3_VIOLATION window_bars=300000 child_level=1 parent_level=2 side=Short child_kind=Pan child_interval=(276991,277358) child_c_start=276991 child_seg_a=(274844,275400) child_first_provable_at=277358 parent_kind=Pan parent_interval=(276991,278122) parent_c_start=276991 parent_seg_a=(265912,267805) parent_first_provable_at=278122
ISSUE690_D3_VIOLATION window_bars=300000 child_level=2 parent_level=3 side=Long child_kind=Pan child_interval=(241822,243633) child_c_start=241822 child_seg_a=(215707,217381) child_first_provable_at=243633 parent_kind=Pan parent_interval=(241822,246574) parent_c_start=241822 parent_seg_a=(181542,187631) parent_first_provable_at=246574
ISSUE690_D3_BUCKET window_bars=300000 child_level=0 parent_level=1 side=Long judged_pairs=29 violations=27 violation_rate=0.931034 unjudged_pairs=1
ISSUE690_D3_BUCKET window_bars=300000 child_level=0 parent_level=1 side=Short judged_pairs=67 violations=45 violation_rate=0.671642 unjudged_pairs=1
ISSUE690_D3_BUCKET window_bars=300000 child_level=1 parent_level=2 side=Long judged_pairs=4 violations=4 violation_rate=1.000000 unjudged_pairs=0
ISSUE690_D3_BUCKET window_bars=300000 child_level=1 parent_level=2 side=Short judged_pairs=18 violations=13 violation_rate=0.722222 unjudged_pairs=1
ISSUE690_D3_BUCKET window_bars=300000 child_level=2 parent_level=3 side=Long judged_pairs=1 violations=1 violation_rate=1.000000 unjudged_pairs=0
ISSUE690_D3_WINDOW window_bars=300000 levels=4 judged_pairs=119 violations=90 violation_rate=0.756303 unjudged_pairs=3 side_mismatch_pairs=36
ISSUE690_D3_ATTRIBUTION window_bars=300000 children_total=740 n0=618 n1=122 n2=0 n3_plus=0
```

</details>

## 六、判定树落档

预登记判定树（照判标注，不自行改档）：

- 每窗违规率：20k=85.71%、100k=86.21%、300k=75.63%，**全部 ≥50%**。
- 三窗合计：violations=121，judged_pairs=155，合计违规率=78.06%，**≥50%**。

→ 落 **高位档：D3 核销候选，待编排者签**。不落"近零档"（三窗均远高于 2%，合计远高于 5 对）；也不落"个案钉因档"的中间区间（各窗与合计均在 ≥50% 一侧，未落在 (5%~50%) 区间）。

按票面纪律，本档不要求逐例初判（该义务限定于"中间档"），已按裁定③交出全量 dump（见上节），供编排者做核销/回炉裁决时直接取用个案证据。

## 七、护栏自检

- 本票除 `p126_d3_descending_clock.rs`（探针）与本报告外，**零文件改动**——`git diff --stat` 只命中这两个文件；`cand_sub.rs`/`cand_event.rs` 等生产文件未碰（#668 在飞工位无耦合）。
- 探针只写不判：只打印分桶/dump 行，不做任何门、不拦截、不改生产路径。
- `cargo test --lib` 基线与终态计数逐位相同（2550/0/138）。
- 未 push；未对主仓 `/Users/silencehan/Projects/NewChanlun` 做任何写操作（数据文件只读软链）。

## 八、交账清单

| 项目 | 结果 |
|---|---|
| 探针改动 | `p126_d3_descending_clock.rs` 落口径 v2（方向过滤 + 归属分布 + 违规 dump），142 行插入 / 29 行删除 |
| 三窗 v2 违规率 | 20k=85.71%（v1 83.33%），100k=86.21%（v1 87.50%），300k=75.63%（v1 74.17%），合计 78.06%（v1 77.34%） |
| 归属分布 | 三窗 n2/n3+ 恒为 0——同方向多重归属在本数据集未出现 |
| 判定树落档 | 高位档：D3 核销候选，待编排者签 |
| 测试计数 | 基线与终态均 2550 passed / 0 failed / 138 ignored，逐位相同 |
| 停手事项 | 无——方向过滤未压低违规率，判定树落在高位档，本票按纪律不自行成门，交编排者签 |
