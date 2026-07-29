# #671（N5 裁定两件小活）落地报告——① 冻结钟正名 + ② D3 递降钟三窗实查

- ticket：#671（源票 #669 三裁编排者 2026-07-29 裁定的两件落地小活）；工位 `/private/tmp/wt-671`，分支 `ticket-671`，起点 main 尖端 `089f659f01`。
- 执行器：claude sonnet，全程前台单线程，未派发任何子代理/后台任务。
- 结论摘要：① 机械改名已落地，diff 全改名无逻辑动，测试计数不变。② 三窗实查结果为**非零**——BTC 20k/100k/300k 三窗上「父级 `first_provable_at` > 子级 `first_provable_at`」的违规率介于 66.7%~100%，**不是零/近零**，按裁定原文「非零→留计数+数据回炉再裁」处置，不自行成门；且发现一项须一并回炉的解释歧义（见§四），照实陈述，不代编排者定档。

## 一、基线 / 终态测试计数

| 阶段 | passed | failed | ignored |
|---|---|---|---|
| 基线（改动前，起点 `089f659f01`） | 2544 | 0 | 138 |
| 终态（① + ② 落地后） | 2544 | 0 | 138 |

两次计数逐位相同；未见既存失败测试（无需照实登记失败测试名）。

## 二、① 改名：`FrozenCompletedMove.judge_at` → `frozen_at`

文件：`rust/src/theta_v0/classifier/level_view_store.rs`（票面「仅本文件」预期核验通过——见下）。

**调用面核查**：`grep -rn "FrozenCompletedMove" rust/ --include='*.rs'` 命中全部落在本文件内；跨仓 `judge_at` 的其余命中（`p108/p92/p116/p119/p123/p124_merge`、`nest.rs`、`nest_lifecycle.rs`、`turn_class.rs`、`level_view/confirm.rs`、`level_view/pan_provider.rs`、`runner.rs`）均是**另一批同名字段**的结构体（`NestCandidateEvent`/`TypedNestCertificate`/`CandidateEvent` 家族等），与 `FrozenCompletedMove` 无关，不在本票改动面。

**diff 摘要**（7 行改，5 行删，逐处即改名，无逻辑分支变动）：
- `FrozenCompletedMove` 结构体字段声明 `judge_at` → `frozen_at`，附正名注释「冻结钟，非判定钟：记录本条 Completed 快照被写入冻结态时的 as_of，不是该走势完成的判定时刻」；
- `FrozenCompletedMove::from_move` 构造处同步改名；
- `CompletedFreezeAdapter::observe` 内 `frozen.judge_at` → `frozen.frozen_at`（读取既存冻结快照的字段访问）；
- `WireSnapshot::from_snapshot` / `into_snapshot`（磁盘序列化边界）**保留 `judge_at` 字段名不变**——JSONL 是正式持久化边界（模块头注释原文），且 `legacy_v0_migrates_on_reopen` 测试对磁盘 JSON 字面量 `"judge_at": 99` 有显式依赖（读的是历史落盘格式，不是内存结构体），改动这个名字会破坏向后兼容与该测试；只在 `from_snapshot`/`into_snapshot` 两处内存↔磁盘转换函数体内，把访问 `FrozenCompletedMove` 一侧的字段名同步改为 `frozen_at`。

commit：`749b7b54b6`。

## 三、② D3 递降钟探针：设计与口径

新增只读常驻 census bin：`rust/src/bin/p126_d3_descending_clock.rs`（p124/p125 已占用，顺延至 p126；`ticket-668`/main 均未占用该编号，核对无冲突）。

**量测对象**：`cand_event::CandidateEvent.first_provable_at`（#551 首证钟，塔内 N1 候选事件，`cand_event.rs:134`）。

**父子级别关系口径**：复用生产既有的跨级候选包含谓词 `cand_sub::candidate_is_sub`（`child.event_level < parent.event_level` ∧ `C⊆C` 区间包含，`cand_sub.rs:38`）——这是本仓库唯一的、非本票新造的「跨级候选父子关系」判据，与 `cand_sub::scan_adjacent_containment`（#552 既有诊断电池 `issue550_event_battery` 沿用的同一扫描口径）同源，逐相邻级 `(child_level, child_level+1)` 做 child×parent 笛卡尔积。

**违规定义**：一对 `candidate_is_sub` 成立的 (child, parent)，若两者 `first_provable_at` 均已钉死（`Some`）且 `parent.first_provable_at > child.first_provable_at`，计一次违规。仅当两端都可判时才计入分母 `judged_pairs`；至少一端尚未可证（`None`）的对单独计入 `unjudged_pairs`，不计入违规率分母（避免「未判」与「判过不违规」混同）。

**分桶**：窗口 × 级别对（`child_level→parent_level`）× 方向（`child.key.side`，`Long`/`Short`）。另单列 `side_mismatch_pairs`（child 与 parent 方向不同的对数，非违规判据的一部分，纯计数供解释用）。

**数据/窗口**：BTC 全量 1 分钟数据 `/private/tmp/analysis/data_cache/btc_1m_full.json`（软链至主仓 `analysis/data_cache/btc_1m_full.json`，只读），窗口 20,000/100,000/300,000 bar（#641 电池窗口口径同款）。每窗口独立走 `ParseLayerIncr` 逐 bar append 到窗口末尾，取终态 `classify_with_tower_events` 的 fresh-full `CandidateStreams`（每 key 最新 revision）。

**只写不判**：bin 本身不做任何门、不拦截、不改生产路径，只打印分桶计数行（`ISSUE671_D3_BUCKET` / `ISSUE671_D3_WINDOW`）。

命令：
```
cargo build --release --bin p126_d3_descending_clock
./target/release/p126_d3_descending_clock /private/tmp/analysis/data_cache/btc_1m_full.json 20000,100000,300000
```
运行耗时：三窗合计约 2m50s（多为 JSON 全量文件重复解析的 IO 开销，非分类计算本身）。

## 四、三窗读数（照实登记）

| 窗口 | child→parent | side | judged_pairs | violations | violation_rate | unjudged_pairs |
|---|---|---|---|---|---|---|
| 20,000 | L0→L1 | Long | 3 | 2 | 0.6667 | 0 |
| 20,000 | L0→L1 | Short | 6 | 5 | 0.8333 | 0 |
| 20,000 | L1→L2 | Short | 3 | 3 | 1.0000 | 0 |
| **20,000 合计** | | | **12** | **10** | **0.8333** | **0**（side_mismatch=5） |
| 100,000 | L0→L1 | Long | 13 | 11 | 0.8462 | 0 |
| 100,000 | L0→L1 | Short | 19 | 17 | 0.8947 | 0 |
| 100,000 | L1→L2 | Long | 1 | 1 | 1.0000 | 0 |
| 100,000 | L1→L2 | Short | 7 | 6 | 0.8571 | 0 |
| **100,000 合计** | | | **40** | **35** | **0.8750** | **0**（side_mismatch=11） |
| 300,000 | L0→L1 | Long | 51 | 41 | 0.8039 | 1 |
| 300,000 | L0→L1 | Short | 72 | 50 | 0.6944 | 5 |
| 300,000 | L1→L2 | Long | 4 | 4 | 1.0000 | 0 |
| 300,000 | L1→L2 | Short | 23 | 16 | 0.6957 | 1 |
| 300,000 | L2→L3 | Long | 1 | 1 | 1.0000 | 0 |
| **300,000 合计** | | | **151** | **112** | **0.7417** | **7**（side_mismatch=36） |

## 五、结论读数——非零，照实陈述不代定档

裁定原文二值判据：「零/近零 → 成门；非零 → 留计数 + 数据回炉再裁」。三窗实测**全部非零**，违规率介于 **66.7%~100%**，随窗口增大总体读数稳定在 74%~88% 区间——按裁定原文，走**「留计数 + 数据回炉再裁」**支路，本票不自行升门。

**一项须一并回炉的解释歧义**（照实陈述，供编排者裁档参考，非本执行器结论）：

1. **可能是真违规**：候选事件的首证钟本应遵循「父级结构包含子级结构 ⟹ 父级证据不应晚于子级」的因果直觉，若实测系统性打破这条直觉，是塔内候选判定的真实时序缺陷。
2. **也可能是级别粒度的结构性必然，非缺陷**：父级 C 段区间在时间上包含子级 C 段区间（`candidate_is_sub` 的几何前提），父级的结构谓词（`comparable`/`extreme`）需要更大的 C 段跨度才能满足，天然要求更多 bar 的价格行为——子级（更细粒度）结构宽候选提前证成、父级（更粗粒度）晚于子级证成，可能是「细节先于全局」的几何必然结果，而非因果倒置。若确系如此，"D3 递降"这个违规概念对**当前这种跨级包含关系**本身可能就不成立/不该作为违规口径。
3. **`side_mismatch_pairs` 占比不低**（20k 窗口 5/12=42%，100k 窗口 11/40=28%，300k 窗口 36/151=24%）：相当比例的「父子对」child 与 parent 方向不同——笛卡尔积口径下，一个 child 候选可能被配对到**多个**几何上包含它、但方向不同的 parent 候选（反之亦然），这些对未必是同一走势脉络的真实"父子"传承，可能稀释/污染违规率的可解释性。是否应把方向一致作为「父子」定义的必要合取项，是回炉裁定的一个具体候选方向。

以上两点（§2 结构性必然假说、§3 方向不一致污染）均只是**候选解释**，本票未做进一步验证或取舍——按纪律，成因判定与键口径收紧均属编排者裁量，非本执行器自主决定范围。

## 六、护栏自检

- 仅动 `level_view_store.rs`（① 单文件）+ 新增 `p126_d3_descending_clock.rs`（② 新 bin，非既有文件改动）；未触碰 `cand_event.rs`/`chain_cert/`/`recursive_tower.rs`，与 #668 在飞工位无耦合。
- `cand_sub.rs` 零改动——探针只**调用**其既有公开函数 `candidate_is_sub`，不修改该模块本身（该模块文档明确自述「零消费：调用点 = 本模块单测 + 诊断 bin」，新增一个诊断 bin 调用点与既有纪律一致）。
- 未 push；未对主仓 `/Users/silencehan/Projects/NewChanlun` 做任何写操作（数据文件只读）。

## 七、交账清单

| 项目 | 结果 |
|---|---|
| ① 改名 | 完成，commit `749b7b54b6`，diff 纯改名 + 注释，`cargo test --lib` 前后计数相同 |
| ② 探针 bin | 新增 `rust/src/bin/p126_d3_descending_clock.rs`，只写不判 |
| BTC 三窗违规率 | 20k=83.33%，100k=87.50%，300k=74.17%（全部非零） |
| 裁档 | 按裁定原文走「非零→留计数+数据回炉再裁」，本票不自行成门 |
| 停手事项 | 无——两件小活均按票面完成；§四所列解释歧义是**回炉候选项**，非停手 |
