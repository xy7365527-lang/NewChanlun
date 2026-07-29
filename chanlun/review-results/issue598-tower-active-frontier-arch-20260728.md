# #598 L2/L3 活动下级单元载体选型研究（BTC 100k，2026-07-28）

> 角色：researcher（只读调查，禁子代理，前台单线程）
> 基线：`kimi-nest-mainline-20260717`（工位 `/tmp/kimi-nest-mainline`，只读，未改动任何生产文件）
> 口径：本报告不裁定路线，只给证据、对照与推荐；不改教义、不改代码、不关票。

## 0. 执行摘要

46 只 L2/L3（L2=38/L3=8）身份仍首见即完成（闪现率 100%），且滞后比 L1 更严重
（L2 `c_start→completion` 均值 823 bar / L3 均值 2182 bar，L1 中位仅 238.5 bar）。根因与
#523/#527 判词一致：递归塔 `LeveledMove` 无 Active/Completed 表达，`tower[level-1]` 被
PanLive stem 与完成事件 provider **同源同判**读取，导致「首见即完成」的近似恒等式在每一级
递归上都成立。

**关键新发现（本票定界，#523/#527 未展开）**：L1 之所以能修，是因为 parser 天然具备**两层
分离**——`ParseLayer.segments`（confirmed）与 `l0.tail.PendingSegment`（pending，未确认走势
的当下状态）是两个独立字段，`active_segment_frontier` 只需读取**既已存在**的 `tail`。但递归
塔 `compose_level_resume`（`recursive_tower.rs:822-`）**没有对应的两层输出**——它的窗口扫描
（`detect_centers_windowed_resume`）只产出**已扫描完（seed+extension 判定完毕）的窗口**，
「行进中、尚未成窗」的状态只以两处**私有**内部量隐式存在：

1. `WindowScanCursor`（`recursive_tower.rs:394`）与 `WinMeta`（`:421`）——每级增量扫描的
   续扫断点与读域上界，存于 `TowerCache.levels: Vec<LevelCache>`（`mod.rs:796-865`，
   `LevelCache` 无 `pub`，字段 `scan_cursor`/`win_meta` 均私有）；
2. 「frontier-pop-and-rescan」协议（`mod.rs:1876-1905`）——每 bar 无条件 pop 掉当前级别
   **最后一个**已产出窗口再重扫，防御性地把它当作可能仍在延展的候选；但 pop 前该窗口已经是
   一个完整 `LeveledMove`（已过 seed+extension 判定），与「确认」窗口在类型上**不可区分**。

即：递归塔的「活动」语义不是缺失，而是**已经存在但从未对外暴露**——是私有实现细节，不是
公开读契约。这直接决定了两条路线的真实分工，见 §3。

## 1. 读码定界

### 1.1 `LeveledMove` 与递归塔构造

- `LeveledMove`（`recursive_tower.rs:110-123`）：`rmove`（结构，descend.rs 镜像 Lean `Move`
  μF）+ `start_index`/`end_index`（坐标）+ `sub_moves`（次级别侧车，`Rc` 共享）+
  `id: ElementId`（跨 bar 稳定身份）。**无 Active/Completed 字段**——#523 遗留 1 原话属实。
- `compose_level`/`compose_level_resume`（`:297`/`:822`）把 lower-level `LeveledMove` 序列
  规约为上一级：`detect_centers_windowed_resume`（`:723-805`）逐窗扫描「seed（3 段判据）+
  extension（后续单元触及冻结核心即吸收）+ non-extension 停止」，**只有扫描到停止条件（显式
  non-extension 单元，或数据耗尽）才 push 一个完整 `(Center, window)` 进 `out`**——不存在
  "半成品中枢" 的中间可读状态。
- 增量协议（`mod.rs:1864-1905`）：`WindowScanCursor.resume_from`（最后一个成立窗口的起点）
  与 `.consumed`（扫描退出断点）。`had_emitted_window = resume_from < consumed` 为真时，
  **无条件 pop** 该级别 `centers`/`upper_moves`/`win_meta` 的**最后一个整窗**再从
  `resume_from` 重扫——这是保证增量与全量 bit-exact 的正确性协议（PDF §九增量等价定理），
  不是"活动对象"的语义设计。
- **关键判别位**：`WinMeta.read_end_src`（`:421` 起字段）在窗口因**数据耗尽**（`j==len`，
  未遇到显式 non-extension 停止单元）而产出时取 `usize::MAX`；因**显式停止单元**产出时取该
  单元的源坐标。前者是真正意义上"仍可能因后续数据而改变"的开放窗口，后者是"扫描判定已经
  终结，只是防御性协议仍会重扫核对"的窗口。当前无任何消费方读取这个区分——`tower[level]`
  对外一律是扁平 `Vec<LeveledMove>`。

### 1.2 L1 已有 `ActiveSegmentFrontier` 参照（#527 交付）

- `ActiveSegmentFrontier`（`nest_lifecycle.rs:1378-1389`）+ `active_segment_frontier(l0)`
  （`:1412-1456`）：唯一构造点，**只读** `l0.tail` 的 `PendingTail::PendingSegment` +
  `l0.strokes[pending_start..]`，禁止从 confirmed segments 回放重建（票面永禁清单原话）。
- `provide_l1_active_pan_live_windows`（`:1496-1574`）：confirmed A/B 锚（`centers`/
  `confirmed_segments`）+ active C（`frontier.as_segment()`）产窗；未产窗给
  `L1LiveOutcome` 六态可审计原因码（`:1581-1594`）。
- 生产接线（`p123_fast_replay.rs:1610-1657` `recompute_lifecycle_window_stems`）**硬编码**
  `for level in 1..2.min(tower.len())`——只喂 L1，函数文档自述「L2/L3 的 lower unit 是
  `LeveledMove`，塔上没有 Active/Completed 表达……本函数不为 L2/L3 产活窗」（`:1603-1607`）。
  与之对照，完成事件路径 `lifecycle_bar_phases` 对**全部**级别循环
  （`p123_fast_replay.rs:1737` `for level in 1..world.tower.len()`）——这正是缺口的精确
  代码定界：完成相已level-agnostic，活窗相被人为收窄到 L1。

### 1.3 `tower[level-1]` 作为 lower legs 的消费点（完成事件与旧 PanLive 共用）

- `lower_legs_from(&[LeveledMove])`（`level_view.rs:442-458`）：把 `tower[level-1]`
  **逐项**（含扫描协议意义上"可能仍是 frontier"的最后一项）投影为 `LowerLeg`/`Segment`，
  **无任何完成状态过滤**。完成事件 provider（`level_view.rs:827-903`，票面点名）与旧
  `provide_pan_live_windows`（`nest_lifecycle.rs:1309-1362`，现降级为参照/p409 反事实口径）
  都从这条同一函数取 C 候选——这就是 #523 根因「同源同判」的确切代码位置。
- `lower_legs_from` 与其消费方本身**是 level-agnostic 的纯函数**（无 `if level == 1` 分支）；
  真正 L1-only 的只有 `recompute_lifecycle_window_stems` 里的活窗产出循环（§1.2）。这意味着
  `NestLifecycleBook::advance`、`bridge_identity`/`same_anchor`、`VanishCause` 两类归因、
  完成钟两分不变量（`assert! signal.completed_at <= signal.as_of`，`:1224-1228`）**全部已经
  level-agnostic**——本票的缺口严格局限在"活窗产出"这一层，不需要动账本状态机。

### 1.4 完成证明的跨级机制（已通用，无需改动）

- `find_move_by_end_index(&[LeveledMove], target)`（`recursive_tower.rs:130-137`，O(log n)
  二分）按 `end_index` 定位 `tower[level-1]` 上的 lower unit，取其 `ElementId` 与物理完成 bar
  （`p123_fast_replay.rs:1872` 附近 `lifecycle_bar_phases` 调用点，票面 §2.1 表列
  「`tower[level-1]` 上按 `end_index` 查证存在，取其 `ElementId`」）——**对任意 level 通用**，
  L2/L3 完成钟落账不需要新代码。

### 1.5 证书真值路径边界（#449 裁定，票面点名不得触碰）

`gh issue view 449` 裁定（编排者 2026-07-27/28 封口）：core 判据不得用 **nest 证书**
（`TypedNestCertificate`）或 `turn_class` **回写、过滤、改判**同一历史锚点的
`Classification`/`BspBits`——方向是**证书→回灌判据**。本票讨论的方向是**递归塔（判据侧）
→ PanLive provider（`nest_lifecycle.rs`，同属 `classifier/` 目录、且已是既定的只读消费方，
与 #527 已实装的 L1 读法同构）**，是纯读、不产证书、不回写 `Classification`/塔本身——不落入
#449 禁令覆盖的方向。两条候选路线均不涉及新增证书回灌，此边界对本票选型**中性**（两路线都
合规），故不构成路线取舍的判据，仅作为设计护栏在 §4 最小切片建议中重申。

## 2. 46 只逐级钉现状（复用 #523 TSV 口径，本票新算 L2/L3 切片统计）

数据源：`chanlun/review-results/issue421-structure-completion-identities-20260728.tsv`
（248 行，L1=202/L2=38/L3=8，全部 `derived_type` 前缀 `PanConsolidation/L{n}`，`kind` 恒
`Consolidation`）。

| 级别 | 只数 | `prod_observed_at == completion_as_of` | `p409_state=Invalidated` | `p409_state=Provisional` |
|---|---:|---:|---:|---:|
| L2（`lower=L1-WindowUnit`） | 38 | 38/38（100%） | 20 | 18 |
| L3（`lower=L2-WindowUnit`） | 8 | 8/8（100%） | 6 | 2 |

三组跨度统计（TSV 列 `cstart_to_completion` / `structural_seg_c_span` /
`completion_visibility_lag`，单位 bar）：

| 级别 | `c_start→completion` 均值 | `结构 C 段跨度` 均值 | `完成可见性滞后` 均值 | 最大滞后 |
|---|---:|---:|---:|---:|
| L1（对照，#523 全量） | — | — | 中位 57.5 | 5190 |
| L2 | **822.9** | 439.3 | **383.7** | 4702 |
| L3 | **2182.1** | 1480.5 | **701.6** | 5190（同一只跨级引用的最大值） |

判读：L2/L3 不仅闪现率与 L1 缺口前（#527 修前）逐位相同（100%），其**结构真实存活期比 L1
更长**（L2 均值 823 bar ≈ L1 中位的 3.5 倍，L3 均值 2182 bar ≈ L1 中位的 9 倍）——即修复
L2/L3 活窗的**潜在信息增量比 L1 更大**：L1 修复后闪现率从 100% 降到 32.39%（#527 §1），
L2/L3 因结构跨度更长，可及早观察的窗口理论上更宽，是否能同比例转化取决于 §3 两条路线各自
的「structure_not_locatable」等未产窗损耗。

样例（L2/L3 各 1 条，逐字段见 TSV 原文）：

```
L2  seg_a=(1654,2287)  c_start=5123  b_center_start=2295
    completion_as_of=8179  c_end=5622  跨度=3056  滞后=2557（存活最长的一类）

L3  seg_a=(2295,3223)  c_start=16724  b_center_start=8073
    completion_as_of=23577  c_end=18387  跨度=6853（全表最大结构跨度）  滞后=5190
```

## 3. 两条架构路线对照

### 3.1 路线 (i)：provider 侧只读 frontier view

**做法**：不改 `LeveledMove`/`recursive_tower.rs` 的塔存储与公开返回形状；给
`TowerCache`/`LevelCache` 新增**只读投影访问器**（`TowerCache` 已有同类先例：
`generation()`/`forest_epoch()`，`mod.rs:984-991`），把当前私有的 `scan_cursor`/`win_meta`
以只读形式暴露给 caller（`p123_fast_replay.rs`）；provider 侧新增
`active_leveled_move_frontier`（L1→L2 一级递归的直接类比 `active_segment_frontier`）——用
「L0 confirmed units + L0 active frontier（已有）」重跑一次 `detect_centers_windowed_resume`
式的 seed/extension 判定，得到"当下若把行进中 L0 段计入，L1 层会形成的候选窗口"，作为 L2 的
C 候选喂给 `provide_l1_active_pan_live_windows` 的通用化版本。L3 同理再套一层（需要 L2 的
"假设性活动窗口"作为输入）。

**改动面**：新增 1 个只读访问器方法（`TowerCache`）+ 1（L2）/2（L2+L3）个 provider 侧递归
派生函数（`nest_lifecycle.rs`，与 `active_segment_frontier`/
`provide_l1_active_pan_live_windows` 同文件同风格）+
`recompute_lifecycle_window_stems` 循环上界从 `2.min(tower.len())` 放宽。**不touch**
`LeveledMove`、`compose_level`/`compose_level_resume`、`tower_snapshots` 的返回类型、
`m8`/`backtest`/`admission` 等所有其他 `tower[level]` 消费方。

**教义兼容**：与 #527 已获生产验收的 L1 实装**同构**——「只读既有内部状态，不预判/不回填，
每 bar 重新派生」。`WinMeta.read_end_src == usize::MAX` 是判别"数据耗尽的开放窗口"与"遇到
显式停止单元但仍被协议性重扫的窗口"的现成信号，避免把已经结构性终结的窗口错当"活动"（对应
"不得从已完成 tower unit 外推"的红线）。

**稳定身份**：延续 L1 的 `bridge_identity`/`same_anchor`/`VanishCause` 机制（已 level-agnostic，
§1.3），无需新设计；但 L2/L3 的「观测接缝」成因会**多一层来源**——不仅 parser 段重划可触发，
L1/L2 塔自身的 cascade 失效（`mod.rs:1752-1846` dirty 前缀回退）也可能改写已产出窗口，
两种源头都归入既有 `VanishCause::ObservationSeam`，账本字段不需扩类型，但诊断/复盘时需要
区分两种触发源（建议落诊断 sidecar，不进真值路径，参照 #527 §7 遗留 4 处理方式）。

**完成钟三分延伸**：`find_move_by_end_index` 已通用（§1.4），两分模型（`completed_at`/
`as_of`）直接适用，无额外设计。

**成本/风险**：
- 正：字节护栏风险接近零——不改 `LeveledMove`/塔存储，`p123 stdout`/`P116 dump`/`m8 trades`/
  `tower_events` 等既有黄金字节对照（#527 §3 验收 3 已证 L1 同类改动可做到全 0 diff）结构上
  不受影响；`WinMeta`/`WindowScanCursor` 已是既有类型，只需放宽可见性，无新公开类型。
- 负：递归复杂度**逐级复合**——L2 的假设窗口依赖 L0 的既有 frontier，L3 的假设窗口依赖"L2
  的假设窗口"（后者本身依赖 L1 假设窗口），派生函数不能简单复制 `active_segment_frontier`，
  需要为每一级单独写"虚拟追加一个未定形单元后重跑 seed/extension 判定"的逻辑，实现量随级别
  增长（不是 L1 代码的简单参数化）。
- 负：`WinMeta`/`WindowScanCursor` 原设计定位是"增量正确性协议的内部细节"，一旦被 provider
  读取，其字段语义（尤其 `read_end_src`/`last_window_emitted`）从"内部实现细节"升格为"外部
  读契约"的一部分——未来若重构增量扫描算法，需要额外保证这个读面的语义稳定（成本可控，但
  是新增的耦合面，需要在实装时显式登记契约边界，防止把内部算法变更误判为无害）。

### 3.2 路线 (ii)：塔上独立 active sidecar

**做法**：给递归塔的构造管线本身（`recursive_tower.rs`/`mod.rs`）新增一个**一等公民**输出
——例如每级一个 `ActiveWindowFrontier`（结构类比 `LeveledMove` 但携带"仍可能因后续数据改变"
语义），由 `classify_with_tower_incremental` 在正常构造流程中**主动计算并持久化**，作为
`tower_snapshots` 的姊妹产物（类似 parser 天然同时维护 `segments`（confirmed）与
`tail`（pending）两个字段）。Provider 直接消费这个现成 sidecar，不做任何派生计算。

**改动面**：`recursive_tower.rs` 新增公开类型 + 构造函数；`mod.rs`
`classify_with_tower_incremental`/`classify_with_tower_incremental_resume`（两个现有调用点，
`p123_fast_replay.rs:1008/1081`）的返回签名从 `(Classification, Vec<Rc<Vec<LeveledMove>>>)`
扩为三元组（或等价包装类型），**触达全部消费该返回值的调用点**——不止 PanLive provider，
还包括 backtest/m8 流程、`admission.rs`、`TreeCache`/`extract_carrier_forest`（K_i 森林抽取，
`forest_epoch` 复用判据所在）等（`grep tower[` 命中已知至少 6+ 处非本票相关文件）。即使这些
消费方不需要 sidecar 数据，也需要接受新的返回形状或显式忽略新字段——存在纯churn风险。

**教义兼容**：若 sidecar 的计算逻辑本身仍必须"只读 L0 pending + 递归复合"（否则重演 #523），
则路线 (ii) 并**不能省掉**路线 (i) 描述的那套递归派生算法——只是把算出来的结果**搬家**到
塔构造管线内部，而不是留在 provider 侧。即：两条路线的核心算法工作量**相同**，路线 (ii)
额外多付的是"塔公开类型扩容 + 全部消费方适配"的成本，不是"省了递归派生"的收益。

**稳定身份/完成钟**：无实质差异（两条路线复用同一套 `NestLifecycleBook` 机制）。

**成本/风险**：
- 正：单一物化点——若未来出现第二个消费方（非 PanLive）也需要"活动窗口"语义，路线 (ii) 
  避免了逻辑在多处重复派生。当前**只有 PanLive provider 一个消费方**（全仓 grep 复核，
  `nest_lifecycle` 的生产调用方仅 `p123_fast_replay`/`p409_pan_live_probe` 两个 bin，
  #527 §4 验收 3 已确认），这条收益暂不成立。
- 负：blast radius 大——触达塔存储形状与全部下游消费方，字节护栏（m8 trades/tower_events
  SHA、P-H3 复用率计数器）风险显著高于路线 (i)，即便 sidecar 逻辑本身只读，塔构造管线每 bar
  的额外计算/字段读写仍可能扰动既有性能/缓存复用计数（`provider_reevals`/`provider_reuses`
  等 P-H3 指标，#527 §3 已把这类指标纳入必须 cmp=0 的护栏）。
- 负：与 `recursive_tower.rs` 模块头显式声明的三层分工（结构层/坐标层/力度层，
  `recursive_tower.rs:11-24`「no-声明膨胀：坐标不塞进 RMove」）存在张力——"活动/待定"是该
  文档未预留的第四种关注点，硬塞进现有类型或新增并列类型都需要重新论证分层边界，属架构设计
  工作，不是简单加字段。
- 负：更容易重演 #523——若实装图省事，直接把"塔当前最后一个已产出窗口"当作"活动窗口"暴露
  （而不做 §3.1 描述的那套虚拟追加+重判定），则与旧 `provide_pan_live_windows` 犯的错误
  （从已完成对象外推）在结构上等价，只是外壳换了个"看起来是活动"的类型名——**声明膨胀
  风险更高**，因为塔构造管线的"官方产物"身份会让下游更倾向于不加怀疑地信任它。

### 3.3 对照小结

| 维度 | 路线 (i) 只读 frontier view | 路线 (ii) 独立 active sidecar |
|---|---|---|
| 改动面 | 小（新访问器 + provider 侧函数，单消费方局部） | 大（塔公开类型+签名扩容，触达全部消费方） |
| 教义兼容（不外推） | 与 #527 L1 实装同构，风险可控 | 若实装取巧，重演 #523 的风险更高 |
| 稳定身份/完成钟 | 复用既有 level-agnostic 机制，零新设计 | 相同（无差异） |
| 字节护栏风险 | 低（不动塔存储与既有消费方） | 高（触达 m8/backtest/K_i 森林等既有黄金对照） |
| 核心算法工作量 | 与 (ii) 相同（递归虚拟追加+重判定，逐级复合） | 与 (i) 相同，只是物化位置不同 |
| 唯一收益差异 | 无 | 仅当未来出现第二消费方时才体现（当前不成立） |
| 与既有代码风格文档的张力 | 低（只读访问器是既有模式的延伸） | 中（新增"活动"层，需重新论证三层分工边界） |

## 4. 推荐（不替裁）与 L2 最小切片建议

### 4.1 推荐

倾向**路线 (i)**。核心理由：两条路线的核心派生算法工作量相同（§3.3 表倒数第二行），
但路线 (ii) 为"单一物化点"这个**当前不存在的收益**（只有一个消费方）付出显著更大的
blast radius 与字节护栏风险，且更容易在实装时无意中重演 #523（外推已完成对象冒充活动）。
路线 (i) 是 #527 已验证生产模式的直接延伸，审查与验收路径可复用同一套字节护栏方法论
（pre/post cmp=0 对照）。此推荐**不构成裁定**——若编排者判断"未来会有第二消费方"这一前提
成立（例如若已有其他票据规划要读取塔活动状态），收益结构会反转，应重新评估。

### 4.2 L2 最小切片建议

**先 L2 后 L3，不同步**——理由与 #527 §3「先只做 L1，验证 producer seam；不要一次扩到
L2/L3」相同，且本票 §3.1 已指出递归复杂度逐级复合：L3 的虚拟活动窗口依赖 L2 的虚拟活动
窗口是否正确，若两级同时实装，一旦验收数字异常，无法定位是 L2 派生错误还是 L3 复合错误。

最小切片建议顺序：

1. `TowerCache` 新增只读访问器（如 `level_scan_state(&self, level: usize) -> (&WindowScanCursor, &[WinMeta])`），
   仅暴露读，不改变私有字段可变性；
2. 新增 L1→L2 一级递归的虚拟活动窗口派生函数（暂拟名
   `active_l1_window_frontier`）：输入 = L0 confirmed units（`tower[0]` 投影）+ L0
   `ActiveSegmentFrontier`（已有）+ L1 层 `scan_cursor`/`win_meta`（步骤 1 新access）；
   输出与 `ActiveSegmentFrontier` 同构的 L1 级"当下若纳入行进中 L0 段会形成的候选窗口"
   （`Option<...>`，无法定位时给可审计原因码，复用 `L1LiveOutcome` 命名风格新增变体或平行
   枚举）；
3. `provide_l1_active_pan_live_windows` 泛化为可接受任意 level 的"active leg as Segment"
   输入（当前签名已接受 `&ActiveSegmentFrontier`，需改为更通用的"能转 Segment 的活动腿"
   接口，或新增 L2 专用镜像函数——优先选择前者以复用既有测试覆盖）；
4. `recompute_lifecycle_window_stems` 循环上界 `2.min(tower.len())` → `3.min(tower.len())`
   （仅放开到 L2，即循环覆盖 level ∈ {1,2}）；
5. 验收对照复用 #527 方法论：BTC 100k 全量重跑，L2 38 只身份逐只归因（"存在 earlier Live"/
   可审计未产窗原因码两支，禁止"不知道"），字节护栏 p123 stdout/P116 dump/m8 三窗 cmp=0，
   `cargo test --lib` 唯一红仍应为 #491（不新增失败）；
6. L2 验收通过、遗留问题（若有）钉因后，再决定是否以同方法论开 L3 切片（此步骤本票不预判
   结果，留给下一票）。

## 5. 结果包六要素

1. **结论**：46 只 L2/L3 身份闪现根因与 L1 修复前同构（`tower[level-1]` 无 Active/Completed
   表达，PanLive stem 与完成事件同源同判）；L2/L3 比 L1 结构存活期更长（均值 823/2182 bar
   vs L1 中位 238.5），修复潜在信息增量更大。递归塔的"活动"语义已以 `WindowScanCursor`/
   `WinMeta` 私有形式存在，只是从未对外暴露——两条路线的分野不是"要不要造出活动语义"，而是
   "在哪里物化对外读契约"。推荐路线 (i)（provider 侧只读 frontier view），先 L2 后 L3。
2. **定义依据**：`LeveledMove` 结构定义（`recursive_tower.rs:110-123`，无 Active/Completed
   字段，坐实 #523 遗留 1 原话）；`detect_centers_windowed_resume`/
   `compose_level_resume`（`:723-928`）的窗口扫描仅产出已判定完毕的窗口；
   `WindowScanCursor`/`WinMeta`（`:394-421`）与 `LevelCache`（`mod.rs:796-865`，私有）
   构成当前唯一的"活动"信号来源；L1 参照 `ActiveSegmentFrontier`/
   `active_segment_frontier`/`provide_l1_active_pan_live_windows`
   （`nest_lifecycle.rs:1378-1574`）；`recompute_lifecycle_window_stems` 的 L1-only 硬编码
   （`p123_fast_replay.rs:1629`）对照完成事件路径的 level-agnostic 循环（`:1737`）；
   `find_move_by_end_index`（`recursive_tower.rs:130-137`）与完成钟两分不变量
   （`nest_lifecycle.rs:1224-1228`）已 level-agnostic；#449 裁定（`gh issue view 449`
   评论区，编排者 2026-07-27/28 封口）划定证书回灌方向的禁令边界，与本票读取方向正交。
3. **边界条件（结论在何时翻转）**：
   - 若日后出现**第二个**需要读取"塔活动窗口"的消费方（非 PanLive provider），路线 (ii)
     的"单一物化点"收益从不成立转为成立，推荐应重新评估；
   - 若路线 (i) 实装时发现"虚拟追加未定形单元重跑 seed/extension 判定"的递归复合逻辑复杂度
     超出可维护阈值（例如 L3 需要的三层递归派生在测试/审计上无法做到与 L1 同等的可审计性），
     则路线 (ii) 的"一次性在塔构造侧付清复杂度"可能反而更可控，需具体实装后再评估，本票的
     "工作量相同"判断是基于当前代码结构的静态分析，非实测；
   - 若 `WinMeta`/`WindowScanCursor` 的内部语义在其他票据中被重构（例如改变
     `read_end_src`/`last_window_emitted` 的含义），路线 (i) 新增的读契约需要同步维护，
     届时应重新核对本票 §3.1 的字段语义描述是否仍然成立。
4. **下游推论**：若路线 (i) + L2 最小切片被采纳，#597 map 的「46 只 L2/L3 身份钉因」目标
   将分两批推进（L2 先行、L3 待 L2 验收后另票）；L2/L3 的 `IdentityVanished` 归因
   （`HypothesisRefuted`/`ObservationSeam`）预期会出现比 L1 更丰富的 `ObservationSeam`
   触发源（parser 段重划 + 塔自身 cascade 失效两类），下游若消费 `VanishCause` 统计需注意
   这一复合来源尚未在账本字段层面区分（本票只指出，不建议现在拆分字段，参照 #527 §7
   "不预留空字段"纪律）；`gap_len`/诊断 sidecar 等既有 L1 诊断面（#559/#592）的命名与结构
   可直接复用于 L2/L3，不需要新设计。
5. **谱系引用**：#523（PanLive provider 接缝根因，遗留 1/2/3 originating）；#527/#559（L1
   实装 + 影子评审修复轮，本票路线 (i) 的直接架构先例）；#449（证书真值路径边界，编排者
   2026-07-27/28 裁定，本票读取方向核验为不落入禁令覆盖范围）；#580/#591/#592（map 笔记
   提及的 α/β 类型与措辞限定，本票未展开，留给桥接语义放宽研究票）；
   `no-patch-mentality`/`formalization-validity-domain`（两条路线对照分析遵循"严格性=概念
   清晰+边界明示"要求；本票 §2 数据统计标注为 L2 认识论等级——BTC 单标的单窗真实数据，不得
   写成规格常量，跨品种外推需另跑，同 #527 §7 遗留 5 口径）。
6. **影响声明**：本报告为纯只读调查，未修改任何生产代码、测试、`Cargo.toml`、issue/map 状态、
   roster；仅新增本报告文件 `chanlun/review-results/issue598-tower-active-frontier-arch-20260728.md`。
   §2 数据统计由 `awk` 直接对现有 TSV 文件（`issue421-structure-completion-identities-20260728.tsv`）
   聚合计算，未生成新探针产物，未跑回放二进制（数据复用 #523 报告已落盘的 100k 回放结果）。
