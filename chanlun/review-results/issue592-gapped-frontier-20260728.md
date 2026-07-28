# #592 研究：有洞 frontier 产窗死活——教义裁决输入

> 角色：研究执行层（只读；本 session 未改仓内任何既有文件，只写本报告，探针脚本落 `/tmp`，
> 全程前台单线程，未派子代理/Task）。
> 基线：`kimi-nest-mainline-20260717` @ `b86a1335fc`（HEAD，与 #578 交付的 `bf37658101` 同链，
> 未再改代码——`provide_l1_active_pan_live_windows` 判据仍是 `<`，生产行为未变）。
> 数据：`analysis/data_cache/btc_1m_full.json`（BTC 1m，符号链接指向
> `/Users/silencehan/Projects/NewChanlun/analysis/data_cache/btc_1m_full.json`）。
> 构建：`CARGO_TARGET_DIR=/tmp/kimi-nest-target-592 cargo build --release --bin p123_fast_replay`。
> 探针：不改产线代码——复用已有诊断出口 `P421_LIFECYCLE_DUMP`（`nest_lifecycle.rs`/
> `p123_fast_replay.rs` 均未改动一行），逐 bar dump 落
> `/tmp/issue592-lifecycle-{20k,100k}.txt`（20k 25693 行、100k 125176 行），再用
> `/tmp/issue592_parse_dump.py` + `/tmp/issue592_lifecycle_join.py`（本票新写，纯只读解析脚本，
> 落 `/tmp`）离线统计。复现命令：
> ```bash
> DATA=analysis/data_cache/btc_1m_full.json
> P116_MAX_BARS=20000  P116_CKPT=0 P421_LIFECYCLE_DUMP=/tmp/issue592-lifecycle-20k.txt  \
>   $CARGO_TARGET_DIR/release/p123_fast_replay "$DATA"
> P116_MAX_BARS=100000 P116_CKPT=0 P421_LIFECYCLE_DUMP=/tmp/issue592-lifecycle-100k.txt \
>   $CARGO_TARGET_DIR/release/p123_fast_replay "$DATA"
> ```

## 0. 结论摘要

1. **分布**：20k 窗口内「有洞」（`active.start_index > confirmed.last().end_index`）占全部可判定
   recompute 事件的 **62.0%**（215/347）；100k 窗口 **60.1%**（1103/1836）。洞长（bar 数）连续分布，
   无自然断点，中位数 10~11、均值 14~15、最大 120。窗口命中（`window`）里有洞来源占
   20k **42.6%**（43/101）、100k **53.6%**（326/608）——与 #578 报告「收紧后 101→58 / 210 次」
   **逐位复现**（本票独立重算得到完全相同的 43/58/210 三个数字，见 §2）。
2. **语义分类**：证伪「缺段」（`seg_ledger_short=0`，20k/100k 双档全 0，即 L0 段账本与塔层单元数逐
   bar 相等，无遗漏）；坐实「合法结构形态」——`nest_lifecycle.rs` 既有批注（票 #559 遗留、本票之前
   从未被 L2 数据验证）指向笔构造 `gap_ok`/`i+=2` 跳过机制（`stroke.rs:62-105`，
   `new_stroke_min_gap` 默认 3），本票首次给出该批注的真实数据支撑。
3. **与 #591 关联**：具体实证——#591 案例研究的核心识别身份（`seg_a=(31746,31796)`、
   `b_center_start=31805`、C 段左端 32653）在本票数据中**同时是一例有洞 frontier**
   （`as_of=32691`：`frontier.start=32653`，`confirmed_last_end=32644`，洞长 9）。两票共享同一
   架构根因（L1 活窗路径靠稀疏重算读取跨层坐标——parser `tail`/pending 状态 vs 塔层 `tower[0]`/
   `confirmed` 状态两套投影不保证逐 bar 同步），但聚合统计上「有洞」窗口的 `ObservationSeam`
   消失占比（100k：5.4% vs contig 5.9%）与无洞窗口**无显著差异**——不支持「有洞⟹观测接缝」的强因
   果链，是同源两种不同表现，非同一失效模式。
4. **分档影响量化**：见 §4 表；确认率（Confirmed 终局占比）在有洞与无洞候选间**量级相近**
   （100k：34.8% vs 38.8%；20k：42% vs 50%），有洞候选**不是**纯噪声；`NeverConstituted`（从未可证）
   占比有洞组显著更高（100k：32.1% vs 18.8%，约 1.7 倍）；`ForceOvertake`（反超判负）占比有洞组更低
   （100k：13.4% vs 23.5%）。按洞长分档：确认率随洞长弱单调下降（100k：1-5 长 44% → 31+ 长 25%），
   但各桶样本量小（12~58）、无清晰阈值，任何切档数值都需要额外教义依据而非纯统计拟合。

## 1. 复现方法与数据完整性核对

`P421_LIFECYCLE_DUMP` 是既有诊断出口（票 #421/#527/#559 历次交付），本票**未新增或修改**
`nest_lifecycle.rs`/`p123_fast_replay.rs` 任何一行——纯粹打开已有环境变量、跑已有二进制、离线解析
已有 dump 格式。三行关键 dump 记录：

- `L1_LIVE_RECOMPUTE as_of=.. frontier=(start,extreme_at,dir)|none confirmed_last_end=.. tower0_units=.. l0_segments=.. stems_before=.. stems_after=..`
  （`p123_fast_replay.rs:1112-1135`，每次 `forest_epoch`/`frontier` 变化写一行）。
- `L1_LIVE_HIT`/`L1_LIVE_MISS as_of=.. reason=.. b_center_start=.. c_start=..`
  （`p123_fast_replay.rs:1106-1122`，同一触发内 0~N 行，对应 `recompute_lifecycle_window_stems`
  内每个终结 run 的一次 `provide_l1_active_pan_live_windows` 调用）。
- `REV as_of=.. level=.. side=.. kind=.. seg_a=.. seg_c_full=.. b_center_start=.. revision=.. evidence=..`
  （`feed_lifecycle_bar` → `p123_fast_replay.rs:1869-1884`，`NestLifecycleBook` 每次状态修订一行）。

**关键实现细节**（首次尝试时踩了一次坑，订正后数字才与 #578 报告吻合）：dump 文件里 `L1_LIVE_HIT`/
`L1_LIVE_MISS` 行写在**对应的** `L1_LIVE_RECOMPUTE` 行**之前**（源码顺序：先遍历 `diag_rows` 写
HIT/MISS，再写 RECOMPUTE 汇总行），两者共享同一个 `as_of`。若误以为 RECOMPUTE 先于 HIT/MISS 按文件
顺序取值，会把 HIT/MISS 错误关联到*前一个* recompute 事件的 gap，导致「收紧后 window 101→48」（错），
按 `as_of` 精确匹配后是「101→58」（对，见下）。

**基线零行为变化核对**：本票两档 replay 的 `P527_L1_LIVE_OUTCOMES` stderr 汇总与 §2/§3 的逐行统计
（HIT+MISS 行按 gap 类别求和）精确相等：

| 窗口 | stderr 汇总 | 本票逐行求和（gapped+contig+none） |
|---|---|---|
| 20k | `center_not_consolidation=67 no_active_frontier=57 structure_not_locatable=155 window=101`（合计 380） | gapped(210)+contig(113)+none(57)=380 |
| 100k | `center_not_consolidation=387 no_active_frontier=57 structure_not_locatable=817 window=608`（合计 1869） | gapped(1098)+contig(714)+none(57)=1869 |

两两相等——本票的 gap 分类（按 `frontier.start_index − confirmed_last_end` 符号）与生产判据
（`active.start_index < last.end_index` 唯一拒绝分支）读的是同一份数据、同一套判据，无解析误差。

## 2. 量化分布（票面任务 1）

### 2.1 有洞占比（recompute 事件粒度，只统计 `confirmed_last_end` 已定义即塔已建立的事件）

| 窗口 | 有 gap 定义的 recompute 事件 | 有洞（gap>0） | 共端点（gap==0） | 倒灌（gap<0） |
|---|---|---|---|---|
| 20k | 347 | 215（**61.96%**） | 132（38.04%） | 0（0.00%） |
| 100k | 1836 | 1103（**60.08%**） | 733（39.92%） | 0（0.00%） |

倒灌（`<` 分支，现行唯一拒绝判据）在两档真实数据里**均为 0 次**——与生产 `P527_L1_LIVE_OUTCOMES`
从不出现 `frontier_not_after_confirmed` 一致（该 tag 当前只有 `<` 一个触发路径且从未触发）。

### 2.2 洞长分布

| 窗口 | n（gap>0） | min | median | mean | max |
|---|---|---|---|---|---|
| 20k | 215 | 2 | 10 | 14.03 | 120 |
| 100k | 1103 | 2 | 11 | 15.07 | 120 |

直方图（100k，top 10）：`4→104, 8→71, 6→57, 7→56, 5→54, 3→53, 10→52, 9→51, 11→47, 19→35`——单峰、
向右长尾，**无双峰或明显断点**（≤5 与 ＞30 之间无空隙），不支持存在一个自然的「合法小洞 / 异常大洞」
分界线。

### 2.3 窗口命中的有洞占比 + 收紧影响（逐位复现 #578）

| 窗口 | window（现状，全接受） | 其中 gapped | 其中 contig | 若收紧（`<`→`!=`，拒绝 gapped）后 window | frontier_not_after_confirmed（收紧后） |
|---|---|---|---|---|---|
| 20k | 101 | 43 | 58 | **58** | **210** |
| 100k | 608 | 326 | 282 | **282** | **1103** |

20k 的 `43/58/210` 与 #578 报告原文「101 降到 58」「0 跳到 210」逐位相等——本票在不改一行代码的前提下
用离线 dump 解析独立复现了该发现，确认其非一次性实验误差。100k 此前未见公开数字，本票补齐。

> 与 #578 报告 §4「约占该函数总 outcome 的 45%（210/463）」的口径有出入：本票 210 次相对「全部
> `provide_l1_active_pan_live_windows` 调用结果」（380，含 no_active_frontier 之外的三类结果）占
> 55.3%，相对「排除 bootstrap 的 323」占 65.0%，均对不上 463 这个分母。未能反推出 463 的构造方式
> （#578 报告未列出该分母的计算过程），本票不采用「45%」这一数字，改用可完全复现的 210/380（55.3%）
> 与 210/323（65.0%）两个口径并列标注。

### 2.4 有洞窗 vs 无洞窗的终局统计画像（票面「寿命/反超/消失两类原因码分列」）

**方法与已知局限**：诊断行 `L1LiveDiagRow`（`nest_lifecycle.rs:531-539`）不携带 `level`/`side`/
`seg_a`，只有 `(reason, b_center_start, c_start)`；`REV` 行的完整身份键是
`(level,side,kind,seg_a,seg_c_full,b_center_start)`。本票用粗键 `(b_center_start, seg_c_full.0)`
把 HIT 行（窗口出生时刻，携带出生时的 gap 分类）与该身份的完整修订链关联。碰撞核查：20k 53 个身份键
中 1 处粗键碰撞（对应 2 个不同完整身份共享同一 `(b,c_start)`），100k 285 个身份键中 3 处——碰撞率
＜2%，量级不影响下述统计画像的方向性结论，但不是逐位精确的身份追踪（边界条件：若后续需要逐一身份
的精确审计，需换用完整键，本票的粗键关联足够支撑统计分布，不足以支撑单身份级别的裁决）。53/285 个
身份中分别只有 37/197 个能匹配到至少一行 HIT（其余身份是「完成时才首次可证」的 flash 型身份，从未
经过 L1 活窗阶段，因此无出生时 gap 分类可归——这部分不计入本节统计，不影响有洞/无洞两组的对照）。

**终局分布**（按出生时 gap 类别分组，100k 样本量更大更可信；20k 附列供交叉核对）：

| 窗口 | 分组 | n | Confirmed | ForceOvertake | NeverConstituted | Vanished(HypothesisRefuted) | Vanished(ObservationSeam) |
|---|---|---|---|---|---|---|---|
| 100k | gapped | 112 | 39（34.8%） | 15（13.4%） | 36（32.1%） | 15（13.4%） | 6（5.4%） |
| 100k | contig | 85 | 33（38.8%） | 20（23.5%） | 16（18.8%） | 10（11.8%） | 5（5.9%）+1 Supersedes（未终结，计尾数） |
| 20k | gapped | 19 | 8（42%） | 3（16%） | 3（16%） | 2（11%） | 3（16%） |
| 20k | contig | 18 | 9（50%） | 4（22%） | 3（17%） | 1（6%） | 1（6%） |

**读法**：
- **确认率量级相近**（100k 34.8% vs 38.8%，20k 42% vs 50%）——有洞候选不是「纯噪声」，相当一部分
  最终仍走到 Confirmed。
- **`NeverConstituted`（从未可证即被终局否证）在有洞组显著更高**（100k 32.1% vs 18.8%，约 1.7 倍；
  20k 因样本量小（各 3 例）方向一致但未达到同等倍率）。
- **`ForceOvertake`（反超判负）在有洞组更低**（100k 13.4% vs 23.5%，约 0.6 倍；20k 方向一致）。
- **两类 Vanished 消失占比总量相近**（100k：18.8% vs 17.6%；20k：26% vs 11%，20k 样本太小不稳）；
  `ObservationSeam`（观测接缝，#559 定义的第二类消失原因）单独占比在两组间**无显著差异**
  （100k：5.4% vs 5.9%）——与 §3/§5 的判断一致：「有洞」不等于「观测接缝」，是两种不同现象。

**寿命（as_of 跨度）**：20k gapped median=57/mean=116.3，contig median=101/mean=116.4；100k gapped
median=85/mean=106.3，contig median=68/mean=93.5——**两档窗口方向不一致**（20k 有洞更短，100k 有洞
更长），判定为噪声/窗口截断效应主导（回放窗口越靠近结尾，寿命被人为截断的概率越高，20k/100k 的截断
点不同），**不构成可用于裁决的稳定信号**——诚实报告不给方向性结论。

**按洞长分桶的确认率**（100k，探索分档阈值是否存在自然断点）：

| 洞长桶 | n | Confirmed | 确认率 |
|---|---|---|---|
| 1–5 | 27 | 12 | 44.4% |
| 6–15 | 58 | 20 | 34.5% |
| 16–30 | 33 | 12 | 36.4% |
| 31+ | 12 | 3 | 25.0% |

弱单调下降但非严格单调（6-15 与 16-30 几乎持平），各桶样本量 12~58，置信区间宽——**不支持从纯统计
拟合中读出一个明确的分档阈值**。

## 3. 语义分类（票面任务 2）

**候选假设三选一**：parser 滞后 / 缺段 / 合法结构形态。

1. **缺段——证伪**。`seg_ledger_complete`/`seg_ledger_short`（`nest_lifecycle.rs` 消费方
   `p123_fast_replay.rs:1093-1104`，比较 `l0.segments.len()` 与 `tower[0].len()`）在两档真实数据里
   **均 `short=0`**（20k：`complete=347 short=0 no_tower=433`；100k：`complete=1836 short=0
   no_tower=433`）——L0 段账本与塔层单元数逐 bar 完全相等，parser 没有静默丢段。这条假设被
   L2（真实数据）实测排除。
2. **合法结构形态——坐实**。`active_segment_frontier`（`nest_lifecycle.rs:1404-1448`）的
   `start_index` 来自 `l0.tail` 的 `PendingTail::PendingSegment`，其口径「与 `parser::tail::
   pending_segment` 同源」；而 `confirmed_segments.last().end_index` 来自 `tower[0]`（已完成分类流程
   的段序列）。这是**同一份笔序列的两种不同确认深度投影**：`tower[0]` 反映「已固化」的段，
   `l0.tail` 的 pending 候选起点反映 parser 对「下一段从哪开始」的最新估计。笔构造算法
   `build_strokes`（`stroke.rs:76-107`）的贪心配对在候选间隔不满足 `new_stroke_min_gap`
   （默认 3，`config.rs:63`）时执行 `i += 2`——**跳过该候选对，不强行成笔**（`stroke.rs:101-104`
   注释「间隔不足：越过 b 与其后同类候选，回到与 a 异类的下一候选」）。这正是 `nest_lifecycle.rs`
   既有批注（`ReplayStats.seg_ledger_*` 字段文档，`p123_fast_replay.rs:513-516`）已经指出的成因：
   「『行进中段起点 − 末段终点 > 0』不是缺段判据（笔构造 `gap_ok` 不足时 `i += 2` 跳过分型 ⟹ 相邻笔
   在源坐标上本就可不相接）」。**该批注此前是纯 L0（代数/定义层）推理，从未有真实数据验证支撑**——
   本票用两档真实数据的 `seg_ledger_short=0` 首次给它补上 L2 验证（`formalization-validity-domain`
   规则要求的有效域实测）。
3. **parser 滞后——与 §2.4/§5 的『观测接缝』相关但非本票空间坐标缺口的主因**。#559/#523 已文档化的
   「完成事件首次可见 bar 晚于 lower unit 物理完成 bar」是**时间轴**上的滞后（同一身份，完成信号晚
   到账），与本票的「洞」是**空间坐标**上的不相接，两者概念不同。§2.4 数据显示
   `ObservationSeam`（该滞后现象在消失原因码里的落点）在有洞组和无洞组占比几乎相等——不支持「有洞」
   本身是 parser 滞后的直接同义词。

**边界条件（未做的验证）**：本票未逐 as_of 重建具体的笔/段坐标序列做单实例级证据链（例如取一个
洞长=2 的最小实例，手工核对 `l0.strokes`/`l0.segments` 在该 bar 的完整快照，确认洞两端具体是哪几根
K 线被 `gap_ok` 判定候选跳过）——这需要额外搭建独立探针二进制（新建 Cargo 工程通过 path 依赖引用
`newchan_rust` lib target，或临时修改产线代码打印内部状态），超出本票读专预算，且与「不改仓内既有
文件」的票面约束有张力。若教义裁决需要逐实例级证据（而非本票给出的统计分布 + 架构级代码引用），
建议后续票单开一个可写探针的 ticket。

## 4. 与 #591 的关联（票面任务 3）

#591（`chanlun/review-results/issue32893-center-query-divergence-20260728.md`，本票撰写时已在同一
map 下并发交付）判定：`level_view.rs`（完成检测路径）与 `nest_lifecycle.rs::
provide_l1_active_pan_live_windows`（活窗路径）对 `nearest_confirmed_center_idx` 的输入构造算法
**逐字相同**，32893 现场的分歧是**时序差（无害）**——「中枢确认滞后 + 单一全局 frontier + 事件驱动
稀疏重算」三者叠加导致活窗路径在某个稀疏采样窗口里错过了刚确认的中枢。

**本票发现的具体交叉点**：#591 案例研究锁定的核心身份——`seg_a=(31746,31796)`、
`b_center_start=31805`、C 段左端 32653——在本票 100k dump 中同时命中一次**有洞** recompute：

```
L1_LIVE_RECOMPUTE as_of=32691 frontier=(32653,32690,Down) confirmed_last_end=32644 ...
```

即 `active.start_index(32653) − confirmed_last_end(32644) = 9`——洞长 9，落在 §2.2 分布的主体区间
（median 10~11）内，非极端异常值。**同一身份、同一时间窗口，两票各自独立发现的两个现象共现**。

**关联判定（不替 #591/#592 各自裁决，只判定家族关系）**：
- **架构同源**：两个现象都是 L1 活窗路径「按 `(forest_epoch, frontier)` 变化稀疏重算」这一采样策略
  的产物——#591 的滞后是采样时机错过刚确认的中枢，本票的洞是 pending frontier 起点（parser tail 层）
  与 confirmed 段终点（tower 层）两套投影不保证逐 bar 同步。两者都源于「L1 活窗路径混用 parser 原始
  尾状态与分类器已确认状态两个不完全同步的坐标系」这一个更上位的架构特征。
- **非同一失效模式**：§2.4 的聚合统计（`ObservationSeam` 占比在有洞/无洞组间无显著差异）不支持
  「有洞 ⟹ 触发 #591 式的中枢查询滞后」这样的强因果链——32893 是两种现象恰好在同一身份上共现的一个
  **具体实例**，不能从单一实例外推到「有洞 frontier 普遍伴随中枢查询滞后」的统计规律。
- **对裁决的意义**：若后续裁决决定收紧有洞守卫（拒绝产窗），32893 这一具体案例会同时被两条路径的
  问题触及（既是 #591 式的滞后候选，又是被 #592 拒绝的有洞候选）——即两票的修复范围有交集但不是
  包含关系，教义裁决应分别评估，不能假定修一个自动带走另一个。

## 5. 分档影响量化与推荐（票面任务 4，不替裁）

### 5.1 三种处理方式的代价对照

| 方案 | 20k window | 100k window | 代价 | 收益 |
|---|---|---|---|---|
| A. 保持现状（全接受，`<`） | 101（不变） | 608（不变） | `NeverConstituted` 噪声占比更高（100k 有洞组 32.1% vs 无洞组 18.8%）；零工程改动 | 不损失任何 Confirmed 候选（有洞组 34.8%~42% 确认率，量级与无洞组相近，不是零） |
| B. 全部拒绝（`<`→`!=`） | 101→**58**（-42.6%） | 608→**282**（-53.6%） | 直接删除已知会最终 Confirmed 的候选——100k 损失 39 个、20k 损失 8 个（§2.4 表）——是真实召回率损失，不是纯粹降噪 | 干净：`frontier_not_after_confirmed` 语义单一化，不再需要区分倒灌/有洞两种成因 |
| C. 按洞长分档 | 视阈值而定 | 视阈值而定 | §2.4 显示确认率随洞长弱单调下降但无自然断点，任何阈值都是外部强加而非数据自证；工程复杂度最高（需要拆分 `FrontierNotAfterConfirmed` 为两个原因码或加一个 `gap_len` 字段，呼应 #578 报告 §4 已提出的「拆两类原因码」建议） | 若阈值选得好，可在损失少量 Confirmed 候选的同时压低 `NeverConstituted` 噪声占比；但「选得好」缺乏数据支撑的客观依据 |

### 5.2 推荐（执行层量化输入，不替裁）

- 不支持方案 B 的无条件收紧：确认率数据（§2.4）显示有洞候选不是可被忽略的噪声，收紧的直接代价是
  实打实地丢失约 3~4 成本可确认的身份中的一部分（有洞组内 Confirmed 占比本身已不算低，34.8%~42%）。
- 若裁决关注下游消费质量而非源头开关，`NeverConstituted` 占比的显著差异（100k 32.1% vs 18.8%）提示
  一个折中方向：**在 `L1LiveOutcome::Window` 载荷上加一个诊断态的 `gap_len: usize` 字段（呼应
  #578 报告 §4 已提出的「`FrontierNotAfterConfirmed` 应拆两类原因码」建议，思路一致，落点从「原因码
  拆分」扩展到「窗口本身携带诊断信息」），下游消费方按需过滤，而不是在 `nest_lifecycle.rs` 源头
  做硬编码 accept/reject**——这条路径不需要教义先给出一个「多大算合法多大算异常」的阈值即可先落地
  可观测性，阈值判断留给下游或留给后续教义裁决，不阻塞本票交付。
- 方案 C（源头分档）在数据侧没有自证的阈值，若裁决坚持要走这条路，阈值本身需要独立的教义依据
  （例如以 `new_stroke_min_gap=3` 的若干倍作理论锚，而非纯统计拟合），本票不代为提出该阈值。
- 与 #591 的交叉实例（§4）提示：无论裁决选哪条路径，都不能假定「解决了有洞就顺带解决了中枢查询
  滞后」或反之——两者需要分别验收。

## 6. 影响声明

- 本票**未修改任何仓内既有文件**（`git status` 核对：改动仅新增本报告 1 个文件 + `/tmp` 下的探针
  脚本与 dump 产物，产线代码 0 改动）。
- 复现所用的诊断出口（`P421_LIFECYCLE_DUMP`、`L1LiveDiagRow`、`REV` 行）全部是票 #421/#527/#559
  的既有交付，本票只是新的**消费方**（离线解析 dump），不改变其产出格式或触发条件。
- `git status` 显示的工作区其余改动（`admission.rs`/`fill.rs`/`mod.rs`/`opsem_dump.rs`/
  `open_ledger.rs` 等）属于本 worktree 内并发的其他工位，与本票无关，本票读取时未触碰。

## 7. 谱系引用

- 本票的核心判据（`seg_ledger_short` 用于排除「缺段」假设、`gap_ok`/`min_gap` 用于坐实「合法结构
  形态」假设）继承自票 #559 已入库的批注（`nest_lifecycle.rs`/`p123_fast_replay.rs` 现有注释），
  本票是对该批注的**首次 L2 验证**，不是新提出的假设——`formalization-validity-domain` 规则要求
  的「有效域 ≠ 定义域」在此适用：#559 的批注此前是 L0 声明，本票用两档真实数据补上 L2 支撑。
- `.chanlun/genealogy` 未见与「有洞 frontier 该不该拒绝产窗」这一教义问题直接对应的既有结算条目
  （本票是该问题的首次量化输入，不是复核）。
