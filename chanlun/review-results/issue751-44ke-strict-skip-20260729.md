# #751 44 课式严格 skip 诊断标注（#686 裁定②落地）

工位 `/private/tmp/wt-751`（分支 `ticket-751`，main 尖端，含 ADR-0009 与既有 `p127_skip_impact.rs`）。
纪律：只写不判、生产文件零改动（`cand_event`/`chain_cert`/`recursive_tower` 全程未碰，#668 在飞
不受影响）；新增探针 bin `rust/src/bin/p129_44ke_strict_skip.rs`，零改动 `p127_skip_impact.rs`、
`ChainEdge` 结构、`Closed` 判定、N7 消费门。

## 零、执行环境

- 数据源：`/private/tmp/analysis/data_cache/btc_1m_full.json`（软链，指向主仓
  `analysis/data_cache/btc_1m_full.json`）。
- 三窗：20,000 / 100,000 / 300,000 根（票面点名），`ParseLayerIncr` 严格只喂窗口内前 W 根。
- 链簿推进节拍：`CHAIN_ADVANCE_EVERY=1000`（周期推进，末根必推），与 `p127_skip_impact` / #711 同
  口径（单次终态推进会让断边观测数学不可达，#711 已有完整推导，本票直接复用不重推）。
- 基线 `cargo test --lib`：**2587 passed / 0 failed / 138 ignored**。
- 终态 `cargo test --lib`：**2587 passed / 0 failed / 138 ignored**（逐字节相同，零回归）。

## 一、票面①：数据形态可读性核查（结论：可读，无需停手）

票面要求先核「塔内既有簿（CenterBook/中枢生命周期出口 + BSP 三类点）能否按『被跳级 × 父区间邻域』
取到中枢完成态与三类点」。核查路径：`rust/src/theta_v0/classifier/{center_book(不存在此名)→
recursive_tower.rs, retrace_ledger, bsp.rs}` 与 `chain_cert/mod.rs`。

### 1.1 首个否定信号——`chain_cert` 自身明确留 fog

`chain_cert/mod.rs:64-67`（模块头部文档，非本票新写）逐字写明：

> 44 课「小转大」的替代必要条件（次级别中枢第三类买卖点）证据路径由 #636 裁定③明确留 fog——skip
> 边因此不携带任何替代证据，它只是"这里跳了一级 + 跳过处的事实计数"。

即：`ChainEdge`/`SkippedLevel` 本身**不携带**中枢完成态或三类点证据；`CandidateEvent.third_class_proof:
Option<usize>` 字段全仓恒 `None`（未接通，`recursive_tower.rs` 单测名
`cp_end_remains_none_without_third_class_proof` 印证）。故**不能从 chain_cert 侧直接读到**这份证据
——这条路径确认不可读。

### 1.2 第二条路径——`Classification.levels[].cp_ownership` 可读

同一次因果重放（`classify_with_tower_events_incremental`，与 chain_cert 完全同源，非另起判据）还
返回 `Classification`（p127 探针此前把它当 `_` 丢弃）。`Classification.levels: Vec<LevelState>`
按级别 ℓ 索引，每级 `LevelState.cp_ownership: Rc<Vec<CpScanOwnership>>` 与 `centers` 1:1：

- `CpScanOwnership.lifecycle: CpLifecycleStatus`（`Pending`/`Closed`）——**完成的次级别中枢**的
  直接生产字段（`recursive_tower.rs:453`），`Closed` = 该中枢 `B_p/c_p` 生命周期终态。
- `CpScanOwnership.third_class_in_c: Option<ThirdClassInCp>`——终态 `c_p` 内部第三类离开/回试结构
  （`recursive_tower.rs:1065-1076`），`Closed` 时才可能 `Some`；携 `side: Side`（`Long`=三买，
  `Short`=三卖，与 `BspPoint.bits.buy3/sell3` 同源，构造点 `recursive_tower.rs:1831-1835` 逐字确认）。
- `CpScanOwnership.b_center: Center`——该中枢核心/外缘/`source_index` 首尾。

**结论：可读**。「完成的次级别中枢 + 第三类买卖点」在生产既有簿里是**同一个对象**
（`CpScanOwnership{lifecycle:Closed, third_class_in_c:Some(_)}`），零新增判据、零拼缝——本票只是
把 `classify_with_tower_events_incremental` 已经算出但此前被丢弃的 `Classification` 返回值接上。

## 二、口径落地

### 2.1 完成的次级别中枢 + 第三类买卖点

`Classification.levels[level].cp_ownership` 过滤 `lifecycle == Closed && third_class_in_c.is_some()`。

### 2.2 父区间

`ChainEdge::parent`（`CandidateKey`）经候选事件流（`latest_by_key`，与 `p127` 同一构造）取
`CandidateEvent::interval`（C 段闭区间，`source_index` 坐标）——与 `chain_cert::build_edge` 计算
`SkippedLevel::inside_parent` 时用的**同一个**父区间字段，未引入第二个"父区间"定义。

### 2.3 邻域定义（本票操作化选择，两档并报）

1. **containment**（首选尝试，镜像 `inside_parent` 的同一条包含谓词 `interval_is_sub`，应用到
   `b_center.(start_index,end_index)`）：**首跑发现恒 0**（诊断见下）——次级别中枢核心窗口的跨度
   系统性大于父级单条 C 段跨度（中枢可持续延伸，父 C 段只是父级单条走势腿），"候选 C 段 vs 候选
   C 段"的包含谓词搬到"中枢核心窗口 vs 候选 C 段"不传递，全包含在几何上近乎不可能满足。
   诊断复核（`P129_DIAG2`，100k 窗口节选）：`side_match=2`（该级别、该方向已有 2 个 Closed+三类点
   候选）但 `strict=false` 逐条成立，确认不是"无候选"而是"候选存在但几何全包含判不过"。
2. **overlap**（订正后主读数）：两区间不相离（`!intervals_are_disjoint`，`cand_event.rs` 单一来源，
   与 `interval_is_sub` 同一相离/相切/退化判据族的另一支，非本票新造）——次级别中枢核心窗口与父
   C 段有交集即算落入邻域。

两档均落盘（`P129_STRICT_BY_PAIR`/`P129_STRICT_TOTAL` 的 `contain_*`/`overlap_*` 字段），本报告的
"严格 skip 占比"主读数取 **overlap** 档；containment 档的恒零结果作为口径选择的证据一并登记，不
隐藏。

### 2.4 方向匹配

票面字面：顶背驰/卖点侧链（`edge.parent.side == Side::Short`）→ 三类卖（`tc.side == Short`）；
底背驰/买点侧链（`Side::Long`）→ 三类买（`Side::Long`）。直接比较 `tc.side == edge.parent.side`
（同一 `Side` 编码约定，`types.rs:316-318` 头部注释确认，无需换算）。

### 2.5 "缺"不预先剔除

`SkippedLevel.alive_at_level == 0`（该级别无存活*候选*）不代表该级别无*中枢*——候选（`cand_event`）
与中枢识别（`recursive_tower`）是两条独立产线。本票对每个 `SkippedLevel` 一律评估，不因
`alive_at_level==0` 跳过；同时与 `chain_cert` 已有的缺/断-在外/断-在内三分（`P736_CUT1` 系列）做
交叉输出（`P129_STRICT_CROSS736`）。

## 三、读数（overlap 口径为主，containment 并列）

### 3.1 按窗口汇总（`P129_STRICT_TOTAL`）

| 窗口 | 跳级实例总数 | overlap 严格 | overlap 占比 | containment 严格 | containment 占比 |
|---|---:|---:|---:|---:|---:|
| 20k | 9 | 0 | 0.0000 | 0 | 0.0000 |
| 100k | 41 | 0 | 0.0000 | 0 | 0.0000 |
| 300k | 147 | 3 | 0.0204 | 0 | 0.0000 |

### 3.2 与悬置档关系（`P129_STRICT_SUSPENDED`，ADR-0009 严档=Closed∧零跳，悬置档=Closed∧含跳）

| 窗口 | 悬置档内跳级实例 | 悬置档内 overlap 严格 | 占比 |
|---|---:|---:|---:|
| 20k | 9 | 0 | 0.0000 |
| 100k | 27 | 0 | 0.0000 |
| 300k | 125 | 3 | 0.0240 |

### 3.3 按级别对 × 方向分桶（300k 窗口，唯一出现非零读数的窗口，`P129_STRICT_BY_PAIR`/`_BY_SIDE`）

| 级别对 | overlap 严格 | overlap 噪声 | 合计 | overlap 占比 |
|---|---:|---:|---:|---:|
| L2→L1 | 3 | 95 | 98 | 0.0306 |
| L3→L1 | 0 | 19 | 19 | 0.0000 |
| L3→L2 | 0 | 30 | 30 | 0.0000 |

| 方向 | overlap 严格 | overlap 噪声 | 合计 | overlap 占比 |
|---|---:|---:|---:|---:|
| Long（买点侧/底背驰） | 3 | 47 | 50 | 0.0600 |
| Short（卖点侧/顶背驰） | 0 | 97 | 97 | 0.0000 |

3 例严格 skip 全部出自 300k 窗口的 L2→L1、Long 侧。20k/100k 窗口全部为 0——本数据集/窗口规模下
「44 课式严格 skip」是**极稀有**读数，非普遍现象。

### 3.4 与 #736 缺/断三分交叉（`P129_STRICT_CROSS736`，300k 窗口）

| 三分档 | overlap 严格 | overlap 噪声 |
|---|---:|---:|
| 缺（alive_at_level=0） | 0 | 0（本数据集三窗全程无此档，与 #736 报告一致） |
| 断-在父外（inside_parent=0） | 2 | 51 |
| 断-在父内接不上（inside_parent>0） | 1 | 93 |

3 例严格 skip 中 2 例落在"断-在父外"、1 例落在"断-在父内接不上"——两档均可能是 44 课式严格 skip，
样本太小（合计 3 例）不支持进一步判系统性偏好。

## 四、可复跑命令

```bash
cd rust
# 主读数（默认）
cargo run --release --bin p129_44ke_strict_skip -- /private/tmp/analysis/data_cache/btc_1m_full.json
# 诊断①：各级别 cp_ownership/closed/closed_with_third 原始基数
P129_DIAG=1 cargo run --release --bin p129_44ke_strict_skip -- /private/tmp/analysis/data_cache/btc_1m_full.json
# 诊断②：逐跳级实例的 side_match/containment 明细（用于核验 containment 恒零非"无候选"）
P129_DIAG2=1 cargo run --release --bin p129_44ke_strict_skip -- /private/tmp/analysis/data_cache/btc_1m_full.json
# 回归复核
cargo test --lib
```

## 五、停手项

无。数据形态核查通过（§一），全票面按裁定②执行完毕，无需停手拼缝。

唯一需要显式登记的口径敏感性：邻域定义从 containment 改为 overlap 是**本票的操作化选择**，不是
新裁定——若未来对"邻域"有更严格或更精确的裁定（例如要求中枢核心窗口与父区间的交集达到某个最小
比例，而非任意相交即算），当前 overlap 口径下的读数（尤其 300k 窗口 2.04%/2.40%）需要重新核算，
但探针的两档口径均已落盘，重算无需改探针结构。

## 六、诊断标注不进门（重申）

本票产出的「44 课式严格 skip」标注是**读数层**输出（`P129_*` 系列 println，无返回值消费者），不
写回 `ChainEdge`/`SkippedLevel`、不进 `Closed` 判定、不改 N7 消费门（ADR-0009 严档谓词仍只是
`Closed ∧ 零跳`）。与 #736 的两刀读数（inside/outside 分离、skip 边位置）同一纪律：诊断标注可叠加
观察维度，但入门只有零跳一维。
