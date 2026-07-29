# #602 PanLive provider L3 交付：只读视图扩到 tower[2]（BTC 100k，2026-07-28）

> 角色：executor（实装，前台单线程、禁子代理）
> 基线：`kimi-nest-mainline-20260717` @ `d40607d805`（#573 T1 migrate 后）＋本工位他 session 未提交面
> 票据：#602（map #597 子票）；架构裁定 #598（路线 i：provider 只读视图，先 L2 后 L3）；
> 直接先例 #601（L2 实装）+ #613（#609 F1/F2 收口）+ #527/#559/#578/#592（L1 链）；根因票 #523；
> 口径票 #617（批量确认）/ #618（时机判据边界）
> 口径：本报告不改教义、不关票、不改 map/roster；provider 语义未遇教义层歧义，无上浮。
>
> **基线漂移登记（照实）**：动手时 HEAD = `886a439c78`，工位 `nest_lifecycle.rs` 上有他 session
> 未提交的 #573 T1 重构（183+/54−）。为免与之互相踩，本票实装与全部测量在**隔离副本**
> （`git archive` 出净树到 `/tmp/wt602-work`）上完成。交卷前 HEAD 已推进到 `d40607d805`
> （#573 T1 expand + migrate 均已提交），补丁移植到新基线后**全套验收重跑**：§3–§5 的读数、
> 字节对拍与测试门**全部采自新基线**。附带实证：新旧基线的 p123 三面产物逐字节相同
> （`cmp` 20k/100k dump = 0），即 #573 migrate 对本票关心的面确为 bit-exact。

## 1. 结论

**L3 的「完成前活窗」在生产真实产出了**：`l3:window` 从 **0 → 72** 条命中行，BTC 100k 的
**8 只 L3 完成身份中 1 只（12.5%）在完成前 163 bar 即已被观察**（`observed_at=59334 <
completion_as_of=59497`），L3 首次出现非闪现寿命（此前恒为 0）。

**8/8 归因闭合，零「不知道」**。更重要的是逐只终判把「为什么只有 1 只」拆成了可查的三类：
**3 只在完成信号的同一 bar 才首次被活窗路径看见**（严格 `<` 天然排除 —— #618 时机判据边界的
逐字复现）、**3 只活窗路径比完成路径更晚看见**（滞后 190 / 574 / 860 bar）、**1 只全程不存在
「行进中的 L2 窗口」这个对象**。

**票面前提的实测范围限定（090 照实）**：票面预判「批内更早窗口天生不可观测」在塔层**确实
成立**（探针实测 Σ(m−1) = **91** 个窗口实例被「只取末窗」丢掉，代数必然），但它**不解释这 8 只
里的任何一只**（0/8）。机制真，解释力为零——这是探针的主要产出，不是事后辩解。

**L1/L2 逐位零回归**：`l1:window=610` / `l2:window=342` / `l0:no_active_frontier=57` 等**全部
tally 逐值不变**，L2 归因分桶（17 A + 21 B）逐项不变，L1/L2 逐级终局与寿命分位逐值不变，
`entries=301` / `claimed=2` 不变。为此把 #601/#613 的单一重算键**按局部依赖拆成两把**
（见 §2.3 F-新），并非风格偏好——并键版实测把 `l1:window` 顶到 615，会真改 L1/L2 产出。

递归塔的活动语义按 #598 裁定继续以**只读访问器**暴露：新增 `TowerCache::level_scan_units`；
`LeveledMove`、`recursive_tower.rs`、既有塔消费方**零改动**。

## 2. 修正要点（文件:行号 = 本票 commit 后）

### 2.1 `rust/src/theta_v0/classifier/mod.rs`（+35 / −0，纯新增）

| 位置 | 改动 | 作用 |
|---|---|---|
| `TowerCache::level_scan_units(level)` | 新增只读访问器 | 返回「产出 `tower[level]` 的那一级窗口扫描的**输入 units**」：`level==1` ⟹ 委托 `l0_units()`（同源同值，非第二份）；`level>=2` ⟹ `levels[level-2].projected_units`（stage 09 的投影缓存）；`level==0`/越界 ⟹ `None` |

**为什么必须是这一份**：provider 侧要与塔**逐字同输入**重跑扫描。从 `tower[level-1]` 反投影
另建一份几何等值的 units 是第二查法——`center_from_window` 忽略方向 ⟹ 分歧不会在窗口上暴露，
只会在方向语义上静默漂移。函数文档同时登记同长不变式（`level_scan_units(k).len() ==
tower[k-1].len()`）与「不得跨下一次增量调用持有」。

### 2.2 `rust/src/theta_v0/classifier/nest_lifecycle.rs`

| 位置 | 改动 | 作用 |
|---|---|---|
| 模块头 090 登记 | 「只有 L1 有 active lower-frontier」→ 逐级现状（L1/L2/L3 均有，L4+ 无） | #601 落地后该句已陈旧；声明与能力不符即声明膨胀 |
| `scan_active_window`（私有内核）+ `ActiveWindowScan` | 从 `active_l1_window_frontier` **抽出**：三道守卫（衔接/锚在界内/禁外推）+ 虚拟追加 + 重扫取末窗 | L1 层与 L2 层共用同一算法（禁第二查法）；`active_l1_window_frontier` 改为其薄包装，**逐位等价**（L1/L2 读数不变即其实证） |
| `ActiveWindowOutcome` | 文档改级别中立 + 新增 `LowerLegDirsOutOfSync` | 码表跨级共用（同一失败换名 = 声明膨胀），计数按 `(level, tag)` 分列 |
| `active_l2_window_frontier` | **新增本体**：`tower[1]` 投影 units + `active_l1_window_frontier` 产物作虚拟单元 → 重跑 `center_from_window` 取末窗 | L3 活窗的 C 来源 |

**三处级别相关差异全部显式化**（不是「换个参数」）：

1. **build 换几何路径**：`center_from_window`（全三段核心非空，**无方向交替**——上级单元是中枢
   外缘区间，没有 §6.1 意义的方向维度）。给 L2 层套 `center_from_segments` 会在几何判据层用完整
   判据，产出塔上不可能存在的窗口。测试 `r2_l2_layer_uses_geometric_build_unlike_l1_layer` 用
   **同一份输入**把两条 build 的分歧钉成断言（几何成窗 / 完整判据 `NoWindowFormed`）。
2. **输入换一层**：`level_scan_units(2)`。
3. **方向另有来源**（L3 特有正确性点）：`l1_units[i].direction` 是 `project_to_units` 的
   **ownership 方向**（Q7 口径：中枢落 Trend(d) 块 ⟹ d，否则 endpoint 降级），**不是**
   `lower_legs_from` 的 `first_leaf_direction`；而 L3 的 confirmed 侧腿取首叶口径 ⟹ 活动腿必须
   同口径，否则 A/C 结构定位是拿两套方向语义对比。故首叶方向表由调用方按 `l1_leg_dirs` 供给
   （= `lower_legs_from(tower[1])` 的 direction 列）。**L1 层不需要这一项**，是因为那一级两套方向
   恰好同值（`segment_to_unit` 直传段方向 = 首叶方向），不是省略。

**虚拟单元 `direction` 字段照实登记**：填首叶方向，与 `l1_units` 同位置元素的 ownership 方向
可以不同值。这不构成分歧，因为本级 build 与延伸判据**都不读 direction**——此处不做「方向无
关紧要」的静默假设，而把「本级扫描不消费方向」写成显式登记。

**「只取末窗」的代数后果**在 `scan_active_window` 文档内登记（#617 同构）：一次重扫可产 m 个
窗口，`windowed.last()` 只取最后一个 ⟹ 同批更早的 m−1 个在**任何粒度上都不曾作为「当下的
行进中窗口」存在过**；守卫 3（末窗须含虚拟单元）又使它们连被拒的机会都没有。**不加队列回补**
——那会凭空发明塔从未处于过的状态（票面「不假设塔侧队列有效」）。

### 2.3 `rust/src/bin/p123_fast_replay.rs`

| 位置 | 改动 |
|---|---|
| `TowerScanView` | 新增：塔侧只读三元（`level_scan_units` / `level_scan_cursor` / `tower_confirmed_len`），按「产出 `tower[level]` 的那一级」取值 |
| `LifecycleLowerKey` / `LifecycleUpperKey` | 重算判据键**拆两把**（见下 F-新） |
| `LowerFrontierCarry` | L2 段派生、L3 复用的中间量（行进中 L1 窗口单元 + 首叶方向列）；L2 段每轮**先清空再回填**，任何失败路径都不给 L3 留旧值 |
| `recompute_lifecycle_window_stems` | 加 `levels: Range` 参数（分级重算）；上界 `1..3` → 按段 `1..3` / `3..4`；`miss()` 收口 tally+dump 行两处同源 |
| level 3 分支 | 调 `active_l2_window_frontier`；confirmed 段侧截 `tower_confirmed_len(2)`；**扫描输入侧**截 `tower_confirmed_len(1)`（F-新2） |
| 同长守卫 | 基准由 `tower[level-1]` 订正为 `tower[level-2]`（扫描的**输入**塔层，不是产出）；码名 `l0_units_out_of_sync` → `scan_units_out_of_sync`（级别中立；该码恒 0 ⟹ dump 无痕） |
| `lower_legs_unprojectable` | 新码：原实装对 `lower_legs_from` 失败**静默 `continue`**，在归因表上不留任何行（「零不知道」的一条静默通道）——补码闭合（实测恒 0） |
| dump | `PAN_LIVE_RECOMPUTE` 补 `l2_resume_from=` 字段；新增 `level=3` 诊断行 |

**F-新1：重算判据键必须拆两把（局部依赖，`lead-parallel-dispatch` 275 号）**。
L3 多两个输入（`level_scan_cursor(2)` / `tower_confirmed_len(2)`），L1/L2 **不消费**它们。
把六项并成一把键 = 用不相干的输入去失效可复用的结果。这不是洁癖：并键版**实测**把
`l1:window` 从 610 顶到 615、`l1:structure_not_locatable` 818→827——因为重算变频 ⟹ 含 `as_of`
的守卫（`FrontierAheadOfClock` 是 `active.end_index > as_of`）在更早的 bar 被重判 ⟹ L1/L2 的
**产出真的变**。拆键后 L1/L2 逐位不动。`seg_ledger_*` 观测面同理只挂 lower 段计数（它问的是
「产 L1/L2 活窗那一刻段账本是否完整」，与 L3 的两项输入无关）。

**F-新2：L3 的扫描输入必须按 `tower_confirmed_len(1)` 整窗截断**——**探针第一轮钉出的缺陷**。
L2 与 L3 在这一点上结构不同（不是可省的对称补丁）：

- L2 的扫描输入 `l0_units` 是 parser **已 emit** 段的投影，行进中段在 `tail` 里、不在输入中
  ⟹ 输入与虚拟单元天然不重叠；
- L3 的扫描输入是 `tower[1]` 的投影，**含塔的开放末窗单元**，而虚拟单元（行进中 L1 窗口）正是
  那批开放单元「计入行进中 L0 段」后的形态 ⟹ 二者同起点，不截断即倒灌：探针轮次 1 实测
  `lower_frontier_not_after_units` **728 次**；补截断后该码 **0 次**、`l3:window` 37 → 72。

截断口径与 confirmed 段侧用**同一条**水线证书（#93 单一来源，禁第二查法）。截断后 `resume_from`
可能落在被截区间内 ⟹ 落 `resume_anchor_out_of_range`（实测 1 次），**不把锚往前拽**——那是猜塔
没走过的扫描路径。方向安全同 #613：水线是保守下界，over-shrink 恒 sound。

**tower[2] 侧开放末窗截断（#601 §7 遗留 1 点名项）已做**：confirmed 段侧按
`tower_confirmed_len(level - 1)` 取，L2/L3 同一口径同一来源，不为 L3 另立判据。

## 3. 探针（开工第一步；方法与读数先于实装结论）

### 3.1 方法（照实登记，含其局限）

「行进中的 L2 窗口」这个对象**在系统里任何地方都不存在**——塔只存判定完毕的整窗，parser 只到
L0。故它无法用「读现有产物」的方式探测：探针只能是**该派生本身的插桩版**。做法：在隔离副本上
给 L2 层重扫加临时 stderr 探针（`P602_PROBE=1`），逐次打印

- 该次重扫产出的**全部**成立窗口的源坐标（不只末窗）、
- 末窗是否吸收虚拟单元；

末 bar 另打印 `tower[2]` 全部元素坐标，供与 `COMPLETION_SIGNAL lower_id=(2,ord)` 对齐
（实测 8 只的 λ_C 与其 `tower[2]` 元素起点逐只相等，join 键成立）。**探针代码交卷前删除，不进
commit**；生产版不保留该开关。

局限（不粉饰）：探针与实装同源，故它测不出「实装本身选错了口径」——第一轮正是因为读数异常
（`lower_frontier_not_after_units` 728）才反向钉出输入侧未截断这个缺陷；若该缺陷不产生异常码，
探针不会自己发现它。

### 3.2 读数

| 项 | 轮次 1（照 L2 配方直译，**未**截断输入） | 轮次 2（补输入侧整窗水线截断） |
|---|---|---|
| 重扫次数 | 970 | 970 |
| `nwin` 分布 | {1:695, 2:197, 3:24, 4:7, 5:22, 6:24, 7:1} | {0:145, 1:754, 2:4, 3:14, 4:13, 5:35, 6:5} |
| 末窗吸收虚拟单元 | 638 / 970 | 363 / 970 |
| **Σ(m−1)（「只取末窗」丢掉的窗口实例）** | 480 | **91** |
| `lower_frontier_not_after_units` | **728**（⟹ 直译版结构上错了） | **0** |
| `l3:window` | 37 | 72 |

### 3.3 逐只预分级（8 只，join 键 = `tower[2][ord].start_index`）

判据：该身份的 C 腿窗口起点在**完成前**是否曾作为「末窗且吸收虚拟单元」出现。

| 完成 as_of | lower_id | λ_C | 末窗且吸收 | 末窗未吸收 | 批内非末窗 | 预分级 |
|---:|---:|---:|---:|---:|---:|---|
| 23577 | (2,8) | 16724 | 0 | 0 | 0 | 不可观测（待细分） |
| 24994 | (2,11) | 23954 | 0 | 0 | 0 | 不可观测（待细分） |
| 32154 | (2,14) | 30441 | 0 | 0 | 0 | 不可观测（待细分） |
| 33470 | (2,15) | 32117 | 0 | 0 | 0 | 不可观测（待细分） |
| 39348 | (2,18) | 38525 | 0 | 0 | 0 | 不可观测（待细分） |
| 59497 | (2,24) | 56425 | **2** | 0 | 0 | **可观测** |
| 68867 | (2,28) | 67403 | 0 | 0 | 0 | 不可观测（待细分） |
| 76420 | (2,31) | 75281 | 0 | 0 | 0 | 不可观测（待细分） |

**「批内非末窗」全列为 0** ⟹ **0/8** 属于「机制上不可观测（批量丢弃）」类。

### 3.4 对票面前提的实测裁断（090）

票面预判：「L2 `active_l1_window_frontier` 只取最后一个成立窗口，批内更早窗口天生不可观测，
代数必然非数据集特例」。

- **机制成立**：塔层同构确认（`scan_active_window` 文档已登记），本窗实测 Σ(m−1) = 91 个窗口
  实例被丢，m≥2 的重扫占 71/970 = 7.3%。
- **但它不是本窗 L3 的成因**：0/8 身份落在该类。7 只不可观测另有其因（§4.1 细分）。

即：把「批量丢弃」当作 L3 覆盖率低的解释，是**未经验证的归因**。本票不采信，按实测细分。

## 4. 验收逐条

### 验收 1（票面 1 / #617 口径）：每只 L3 Completed 要么 earlier Live，要么可审计原因码

归因窗 = C 段真实活跃期 `[λ_C, completed_at]`（#559 C1 口径）；身份锚 = 完整六元
（含 `seg_a`，#618 订正后的口径）。脚本 = `issue613-attrib-buckets.py` 的 level=3 版
（逐字同规则，仅换级别过滤）。

| 归因 | 只数 |
|---|---:|
| A. 存在 earlier Live | **1** |
| B. `lower_frontier_not_absorbed` | 3 |
| B. `no_lower_frontier` | 2 |
| B. `tower_level_absent` | 1 |
| B. `window`（定位到别的 C） | 1 |
| **合计** | **8**（**零「不知道」**） |

#### 4.1 逐只终判（比分桶更细：首见 bar vs 完成 bar）

| 完成 as_of | λ_C | seg_a | B | 首个**同身份**活窗行 | Δ = 完成 − 首见 | 分级 |
|---:|---:|---|---:|---:|---:|---|
| 23577 | 16724 | (2295,3223) | 8073 | 23577 | **0** | **时机判据边界**（同 bar 首见，严格 `<` 排除） |
| 24994 | 23954 | (12513,15180) | 16724 | 24994 | **0** | 时机判据边界 |
| 32154 | 30441 | (23954,28179) | 16724 | 32154 | **0** | 时机判据边界 |
| 33470 | 32117 | (28182,30430) | 16724 | 33660 | −190 | 活窗路径**滞后于**完成路径 |
| 39348 | 38525 | (28182,30430) | 32117 | 40208 | −860 | 同上 |
| 59497 | 56425 | (46318,52957) | 38525 | **59334** | **+163** | **可观测（earlier Live）** |
| 68867 | 67403 | (56425,59517) | 59520 | 69441 | −574 | 活窗路径滞后于完成路径 |
| 76420 | 75281 | (67403,71033) | 59520 | — | — | 全程无同身份活窗行（窗内码：`no_lower_frontier` 7 / `no_window_formed` 3 / `lower_frontier_not_absorbed` 11 ⟹ 「行进中的 L2 窗口」当时不存在） |

三类归并：**可观测 1 / 时机判据边界 3 / 活窗路径滞后 3 / 该对象不存在 1**。

**统一刻画（承 #591/#592/#618，不新立机制）**：后两栏（Δ=0 与 Δ<0）是**同一架构特征的两个
时间切面**——活窗路径按 `(forest_epoch, frontier, cursor, watermark)` 稀疏重算 + 塔的确认水线是
**保守下界**，完成路径则在结构完全确认后一次性事后查询。Δ=0 是该滞后恰好压到 0 的边界；
Δ=−190/−574/−860 是同一滞后放大。本票**不**把它们分别命名成两种机制。

**新增原因码（L3 语境下照实命名，未复用旧码冒充）**：`lower_leg_dirs_out_of_sync`（首叶方向表
与 units 不等长）、`scan_units_absent`（该级扫描输入不存在）、`lower_legs_unprojectable`
（补此前的静默通道）。三者本窗实测均为 **0**——登记为「能力已具备、本窗未触发」，不冒充已验证。

### 验收 2（票面 4）：L1/L2 逐位零回归

**活窗定位结果分布（100k，stderr `P527_L1_LIVE_OUTCOMES`）**

```
pre  l0:no_active_frontier=57 l1:center_not_consolidation=388 l1:structure_not_locatable=818
     l1:tower_level_absent=400 l1:window=610 l2:center_not_consolidation=329
     l2:lower_frontier_not_absorbed=558 l2:structure_not_locatable=467 l2:tower_level_absent=520
     l2:window=342
post 上列**逐值相同**，另加：
     l3:window=72 l3:structure_not_locatable=213 l3:center_not_consolidation=78
     l3:lower_frontier_not_absorbed=462 l3:no_lower_frontier=483 l3:no_window_formed=144
     l3:tower_level_absent=834 l3:resume_anchor_out_of_range=1
```

| 面 | pre | post |
|---|---|---|
| `P421_LIFETIME_SUMMARY entries` | 301 | **301** |
| `force_overtake_claimed`（档1 真值列） | 2 | **2** |
| `first_provable / confirmed / force_overtake / never_constituted` | 207 / 135 / 40 / 74 | **逐值相同** |
| `identity_vanished_refuted / _seam` | 28 / 23 | **逐值相同** |
| `flash_terminal / nonflash_count` | 74 / 226 | 73 / 227（**唯一差异 = 那 1 只 L3 由闪现转非闪现**） |
| `P559_SEG_LEDGER` | complete=1840 short=0 no_tower=433 | **逐值相同** |
| L2 归因分桶（38 只） | 17 A + 9 + 4 + 3 + 3 + 2 | **逐项相同** |
| L1 逐级终局/寿命 | 身份 239 / flash 45 / nonflash 191 / 1‑80‑423 | **逐值相同** |
| L2 逐级终局/寿命 | 身份 55 / flash 21 / nonflash 34 / 14‑139‑358 | **逐值相同** |

### 验收 3（票面 2/3）：BTC 100k L3 读数照实登记

| L3 指标（BTC 100k） | pre | post |
|---|---:|---:|
| 完成身份数 | 8 | **8**（零丢弃、零新增） |
| 存在 earlier Live | **0（0.00%）** | **1（12.50%）** |
| 提前量 | — | **163 bar**（单只，不构成分布） |
| 活窗命中行 `l3:window` | 0 | **72** |
| 闪现（零寿命）终局 | 8 | **7** |
| 非闪现终局 | 0 | **1** |
| 非闪现寿命 | — | **163**（min=med=max，单样本） |
| 终局分布 | Confirmed 7 / NeverConstituted 1 | **不变** |
| 反超（ForceOvertake） | 0 | **0** |
| `IdentityVanished{HypothesisRefuted / ObservationSeam}` | 0 / 0 | **0 / 0** |

**两类消失分列**：L3 两类均为 **0**——本样本未出现任何 L3 身份消失。**不得**据此断言「L3 不会
发生消失」：L3 只有 8 只、其中 7 只寿命为 0，消失的结构前提（跨 bar 存活）基本不成立。

**#598 的「L3 均滞后 2182 bar ⟹ 预期寿命更宽」在本窗未兑现**（唯一非闪现寿命 163 bar <
L2 的中位 139 但远小于其最大 358）。照实登记，不外推成因；#598 原文限定为「潜在信息增量更大」，
未作承诺。

**20k 前缀**：`l3:tower_level_absent=781`，**L3 完成身份 0 只**（塔在 20k 内未长出 L3 级），
L1/L2 全部读数与 pre 逐值相同。

### 验收 4：边界

| 边界项 | 证据 |
|---|---|
| 塔存储零改动 | 本票未触 `recursive_tower.rs`（`git show --stat` 为空）；`LeveledMove`/`compose_level{,_resume}`/`detect_centers_windowed_resume`/`WindowScanCursor`/`WinMeta` 全未动 |
| 既有消费方签名零改动 | `classifier/mod.rs` diff **删除行数 = 0**（纯新增一个 `pub fn`）；`classify_with_tower_incremental{,_resume}` 返回形状不变 ⟹ backtest/m8/`admission.rs`/`TreeCache` 等塔消费方零适配 |
| provider 侧新 API 的影响面 | `active_l2_window_frontier`/`ActiveWindowOutcome::LowerLegDirsOutOfSync` 的消费方只有 `p123_fast_replay`（全仓 grep）；`active_l1_window_frontier` 签名未变 |
| #449 禁令零触碰 | 纯读方向（递归塔 → PanLive provider）；不产 `TypedNestCertificate`、不读 `turn_class`、不回写/过滤/改判任何 `Classification`/`BspBits` |
| #523 永禁清单 | 未改完成相任何代码（`completion_signals` 248→248、`channel_switches` 248→248、`retrograde_rejected=0`）；p409 零改动；`observed_at` 无回填（唯一 earlier Live 的 `observed_at=59334 ≠ λ_C=56425`）；行进中判据取「末窗是否含虚拟追加单元」= 扫描算法自身的语义，不借第三方状态位；禁外推臂 `l3:lower_frontier_not_absorbed=462` 是它的实际工作量 |
| 不碰他 session 未提交面 | commit 只 stage 本票 7 个文件；`chanlun/agent-roster-*`、`issue571-*`、`treasury-*.json`、`p100_cert_bsp_recon.rs` 原样留在工作区 |

### 验收 5：字节护栏（产物 `/tmp/wt602b-*`）

pre = `d40607d805` 净树（`git archive` 导出到 `/tmp/wt602-base`，**强制 touch 全部源文件后重建**
以避开 cargo mtime 重放缓存的假绿）；post = 工位当前树。

| 面 | 窗 | cmp | SHA-256（前 16） |
|---|---|---:|---|
| p123 stdout | 20k | **0** | `bd9ac1d655f9d615` |
| P116 dump | 20k | **0** | `fcc8016a9a01a109` |
| p123 stdout | 100k | **0** | `d8b69c180c23c5e3` |
| P116 dump | 100k | **0** | `8a7327feb3b9ba29` |
| m8 trades / tower_events | p3fold | **0 / 0** | `8196a2941bd3560f` / `83f45a36ab422a92` |
| m8 trades / tower_events | wf7 | **0 / 0** | `868e415081c78520` / `aa96b3e836be5a43` |
| m8 trades / tower_events | wf8 | **0 / 0** | `b265c52ed27967d6` / `1d8dff0d29e8925f` |

八个 SHA 与 #601 §4 验收 5 记录**逐字相同**（跨票一致，且蕴含 #573 T1 migrate 对这些面亦零漂移）。

**lifecycle dump 是本票本体，不作 cmp=0**（同 #527/#601 口径）。post SHA-256：
20k `d7a7143031a6ef63…`、100k `5ae3da9b5718e0f2…`。

**#533 golden 重锚（按票面纪律执行）**：本票故意改变 lifecycle dump（新增 `level=3` 诊断行 +
`PAN_LIVE_RECOMPUTE` 补 `l2_resume_from=` 字段），三档 golden 的 **dump 面**重锚
（2000 全文 + 20k/100k SHA 行）；**stdout / P116 三窗零漂移**（同轮 `cmp` 实证，golden 未动）。
`tests/issue533_p123_byte_guardrail.rs` 模块头变更登记表**已追加一行**（commit / 动了哪些 golden /
原因，引用本报告）。重锚后该门 **2 passed**。

### 验收 6：测试门

| 项 | 结果 |
|---|---|
| `cargo test --lib`（`CARGO_TARGET_DIR=/tmp/kimi-nest-target-602`） | **2079 passed / 1 failed / 137 ignored** |
| 唯一红 | `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`（#491） |
| 零新增失败证明 | 同一轮在 `d40607d805` 净树（`/tmp/wt602-base`，独立 target）实测 **2075 passed / 1 failed / 137 ignored**，失败集合逐项相同；差额 **+4 = 本票新增 4 条 lib 测试** |
| `cargo test --bin p123_fast_replay` | **5 passed / 0 failed**（含本票新增 2 条） |
| `cargo test --test issue533_p123_byte_guardrail -- --ignored` | **2 passed**（重锚后） |
| 三文件零新增警告 | `touch` 三文件强制重编后，编译告警只剩 `classifier/mod.rs:1850` 的 `unused_mut`（**基线既有**，`d40607d805` 上同样存在） |

**新增测试（6 条）逐条覆盖票面点名的五面**：

| 测试 | 覆盖 |
|---|---|
| `r1_active_l2_window_frontier_absorbs_active_l1_unit_with_leg_dirs` | **L3 活窗产出**（行进中 L2 单元派生）+ 首叶方向表口径（夹具令方向表与 units 自带方向逐位相反，断言取前者） |
| `r2_l2_layer_uses_geometric_build_unlike_l1_layer` | **build 真的换了**：同一输入，几何路径成窗 / 完整判据 `NoWindowFormed` |
| `r3_active_l2_window_frontier_reason_codes` | **原因码逐条可达**（6 条，含新增 `LowerLegDirsOutOfSync` 与禁外推臂） |
| `r4_l3_live_window_bridges_completion_without_backfill` | **同身份衔接**（`book.len()==1` 桥迁移非新建）+ **零回填**（`observed_at=95 ≠ λ_C=69`，完成不改写观察钟）+ L3 非闪现寿命 |
| `l3_scan_input_truncates_by_tower1_whole_window_watermark` | **F-新2**：不截断恒判倒灌 / 截到水线后吸收成立 |
| `l3_confirmed_side_truncates_by_tower2_whole_window_watermark` | **tower[2] 侧开放末窗截断**（#601 §7 遗留 1）：不截断恒判 `frontier_not_after_confirmed` / 截断后判据按序推进 |

r1/r4 的夹具口径**照实标注**：`pan_real_fixture` 的 centers/segments 是级别无关的结构对象，
r4 用它驱动 `level=3` 的产窗路径，测的是「provider 对任意 level 同判 + L3 身份的桥衔接与观察钟」，
**不是** L3 真实塔数据的端到端复现（后者由 BTC 100k 生产回放验收）。

## 5. 结果包六要素

1. **结论**：见 §1、§4。L3 完成前活窗已生产化（1/8 有 earlier Live，提前 163 bar，
   `l3:window` 0→72）；8/8 归因闭合并逐只终判为四类；票面「批量丢弃」前提经探针实测**范围
   限定为 0/8 解释力**；L1/L2 逐位零回归；塔存储与既有消费方零改动。
2. **定义依据**：
   - 行进中 L2 单元的合法构造 = 「`tower[1]` 投影 confirmed units（截至 `tower_confirmed_len(1)`）
     + 行进中 L1 窗口单元虚拟追加」经 `detect_centers_windowed_resume` + `center_from_window`
     重跑——**与塔自身每 bar 用的是同一函数、同一 build、同一 resume 锚**
     （`WindowScanCursor::resume_from` 的 frontier 协议）。禁第二查法。
   - 行进中的判别 = 末窗因**数据耗尽**而停（`j == units.len()` 分支），等价 `WinMeta.read_end_src
     == usize::MAX`（#598 §1.1 关键判别位），但不读该私有侧车。
   - 上级层用几何判据而非完整判据：`center.rs` 的诚实有效域登记（上级单元是中枢外缘区间，
     无 §6.1 方向交替维度；对齐 `Origin.CenterStates.classifyDevelopment`）。
   - 方向/外缘口径与 `level_view::lower_legs_from`（`first_leaf_direction` + `rmove.lo()/hi()`）
     同源；外缘读 `center.dd/gg` 由 `LeveledMove::envelope` 投影契约保证 bit-equal。
   - 盘整背驰 A/C 结构判据全部复用单一来源（`nearest_confirmed_center_idx` /
     `locate_pan_div_structure` 窄锚 → `_front_anchor` A′ 回退 → `pan_div_structure_extreme` 预滤），
     与完成事件 provider 同序同判（措辞限定沿用 #591 裁定）。
   - 确认水线口径 = #93 水线证书单一来源（`tower_confirmed_len`），over-shrink 恒 sound。
3. **边界条件（结论在何时翻转）**：
   - 若 `WindowScanCursor.resume_from` 或 `confirmed_watermark` 的语义在其它票中被重构，
     L2/L3 两级的重扫起点与截断口径**同时**失去合法性——`level_scan_cursor` /
     `level_scan_units` 的文档已把这条读契约边界显式登记，重构须同步核对；
   - 若 parser 的 `tail` 不再输出 `PendingSegment`，L1/L2/L3 的活动 C 腿**同时**归零
     （上两级的腿逐级由该段虚拟追加派生）⟹ 三级一起退回首见即完成；
   - 若把输入侧截断（F-新2）撤掉，L3 恒判倒灌（实测 728 次）——即回到「零 earlier Live」；
     若改成「同起点 pop 1 个」，在 `last_window_emitted > 1` 时仍残留可变子单元（#613 F1 同因）；
   - 若把 L3 的两项输入并回单一重算键，L1/L2 的产出会真的变（实测 `l1:window` 610→615）；
   - 若给 L3 复用 `active_l1_window_frontier`（build 是 `center_from_segments`），同向重叠的上级
     窗口会被整类丢掉（`r2` 即此锁）；
   - 若为「批内更早窗口」加队列回补，等于发明塔从未处于过的状态（#617 裁定已判出范围）。
4. **下游推论**：
   - #597 map 的「46 只 L2/L3 身份钉因」目标**全部到位**：L2 38/38（17 A）+ L3 8/8（1 A），
     两级零「不知道」；覆盖率的**结构上限**由 §4.1 的四分法给出，不再是「provider 能力缺口」
     这一句笼统话；
   - **完成钟 provenance 在 L3 的延伸**（#523 遗留 3，#597 Not-yet-specified）：三钟机制
     level-agnostic，本票未改一行即在 L3 生效（8 条 L3 完成信号全携 `completed_lower_id`
     level=2 与 `completed_at`，`completed_at ≤ as_of` 零违序）——可据此具体化出票；
   - §4.1 的「活窗路径滞后于完成路径 190/574/860 bar」是**新的可量化面**，它把 #591/#592/#618
     的定性刻画变成了带数值的分布，可作后续桥接/采样密度研究票的输入；本票只给读数不作机制断言。
5. **谱系引用**：#598（架构裁定：路线 i + 先 L2 后 L3）；#523（PanLive provider 接缝根因，遗留
   1/2/3）；#601 + #613/#609（L2 实装与 F1/F2 收口，本票的直接先例与截断口径来源）；
   #527/#559/#578/#592（L1 链）；#591（同序同判措辞限定）；#617（批量确认 ⟹ 中间状态不存在；
   本票在塔层同构登记并给出**范围限定实测**）；#618（时机判据边界；本票 3 只 Δ=0 是其逐字复现）；
   #449（证书真值路径边界，核为不落入）；#533（p123 字节护栏门与 golden 变更纪律）；
   090（声明 = 能力：模块头级别有效域订正、`scan_units_out_of_sync` 改名、`lower_legs_unprojectable`
   补码、三条 0 次的新码登记为「已具备未触发」）；`lead-parallel-dispatch` 275（局部依赖 ⟹ 判据键
   拆两把）；`formalization-validity-domain`（认识论：派生判据 L0 结构；覆盖率/寿命/滞后读数
   **L2 真实数据单标的单窗**，禁写成规格常量）；`no-patch-mentality`（F-新2 在源头补齐语义，
   不在 provider 内加特例分支）。
6. **影响声明**：改动 7 个文件——3 个生产文件（`classifier/mod.rs` 纯新增一个只读访问器、
   `classifier/nest_lifecycle.rs`、`bin/p123_fast_replay.rs`）+ #533 护栏测试头登记 + 3 份 golden
   夹具（仅 dump 面）。受影响面 = lifecycle sidecar（账本、dump、stderr 审计）+ #533 dump 门；
   **未改** `recursive_tower.rs` / 塔存储 / 塔消费方签名 / p123 stdout / P116 dump / m8 trades /
   tower_events / 装配 / 证书 / `Cargo.toml` / p409 / 他 session 未提交面。

## 6. 遗留（不在本票 Scope，照实登记）

1. **L3 覆盖率 1/8 的可修性**。§4.1 四分法给了方向但都需另裁：
   (a) 3 只「同 bar 首见」要动的是**严格 `<` 判据本身**（#618 已判为教义边界，非缺口）；
   (b) 3 只「活窗路径滞后 190–860 bar」的两个候选成因——重算判据键的稀疏采样、确认水线的保守
   下界——本票**未做归因实验**，不猜；(c) 1 只「对象不存在」是结构事实。
2. **输入侧截断的保守代价未量化**。`tower_confirmed_len(1)` 是保守下界，over-shrink 只会少产
   活窗。探针轮次 1→2 之间 `l3:window` 37→72 是**净增**（因为倒灌拒绝面消失），但截断本身
   独立损失了多少，本票未做第三组对照（需要一个既不倒灌又不截断的口径，而那个口径不存在）。
3. **`active_l1_window_frontier` 的输入侧不对称**：L2 的扫描输入 `l0_units` **未**按
   `tower_confirmed_len(0)`（= parser `segments_confirmed_len`）截断。这与「L1 confirmed 尾段跨
   bar 可重划（古怪线段）」是同一条线索（#601 §7 遗留、#613 沿用），本票未动（动它即破 L2 零回归）。
4. **批量丢弃 Σ(m−1)=91 的下游影响未评估**。本窗对 L3 的 8 只身份解释力为 0，但不等于对
   「活窗命中行总量」无影响——那需要另一套口径（按窗口实例而非按身份）来问，本票不越界。
5. **L4 及以上无 active frontier**。上界写死 `1..4`；不预留参数、不写半成品（#527 §7 纪律）。
   若日后要做，需要的是「L3 层行进中窗口」——build 仍是 `center_from_window`，输入换
   `level_scan_units(3)`、虚拟单元换本票产物、方向表换 `lower_legs_from(tower[2])`。
6. **跨品种外推**：1/8、163 bar、72 条命中行、Σ(m−1)=91 全部是 **BTC 100k 单窗读数**，
   不得写成规格常量（L2 级证据）。

## 7. 复现命令

```bash
cd rust
CARGO_TARGET_DIR=/tmp/kimi-nest-target-602 cargo build --release --bin p123_fast_replay
for w in 20000 100000; do
  n=$([ $w = 20000 ] && echo 20k || echo 100k)
  P116_MAX_BARS=$w P116_DUMP=/tmp/wt602b-post-$n.p116 P421_LIFECYCLE_DUMP=/tmp/wt602b-post-$n.dump \
    /tmp/kimi-nest-target-602/release/p123_fast_replay ../analysis/data_cache/btc_1m_full.json \
    > /tmp/wt602b-post-$n.stdout 2> /tmp/wt602b-post-$n.stderr
done

# m8 三窗（护栏面）
for w in p3fold wf7 wf8; do
  M8_WIN_FILTER=$w OPSEM_DUMP_DIR=/tmp/wt602b-post-m8-$w \
    CARGO_TARGET_DIR=/tmp/kimi-nest-target-602 \
    cargo test --release --lib m8_e2e_all_systems_oos -- --ignored
done

# #533 门（重锚后应绿）
CARGO_TARGET_DIR=/tmp/kimi-nest-target-602 \
  cargo test --release --test issue533_p123_byte_guardrail -- --ignored

# L3 归因（level=3 版分桶脚本 = issue613-attrib-buckets.py 的 level 过滤替换）
sed 's/level=2/level=3/g' chanlun/review-results/issue613-attrib-buckets.py > /tmp/wt602_attrib_l3.py
python3 /tmp/wt602_attrib_l3.py /tmp/wt602b-post-100k.dump -v
# L2 零回归对拍
python3 chanlun/review-results/issue613-attrib-buckets.py /tmp/wt602b-post-100k.dump
```

> **pre 基线重建注意（踩过的坑）**：`git archive` 落盘的 mtime 是 commit 时间，早于既有构建
> 产物 ⟹ `cargo build` 会直接 `Finished in 0.05s` 不重编（重放缓存假绿）。重建 pre 必须先
> `find . -name '*.rs' | xargs touch` 再 build，否则量到的是上一版二进制。
