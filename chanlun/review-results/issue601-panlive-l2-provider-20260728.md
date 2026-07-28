# #601 PanLive provider L2 交付：只读视图暴露塔活动语义（BTC 100k，2026-07-28）

> 角色：executor（实装，前台单线程、禁子代理）
> 基线：`kimi-nest-mainline-20260717` @ `4f941af2b9`（+ 本工位他 session 未提交面）
> 票据：#601（map #597 子票）；架构裁定 #598（路线 i：provider 只读视图，先 L2 后 L3）；
> L1 先例 #527/#559/#592；根因票 #523
> commit：`961a4260ee52a2101258f2fd010681935916859e`
> 口径：本报告不改教义、不关票、不改 map/roster；provider 语义未遇教义层歧义，无上浮。
>
> **采集时点（照实）**：§3–§5 的全部读数与字节对拍采于本票 commit `961a4260ee` 时点
> （其父 = `4f941af2b9`）。此后并发 session 在同一分支上又提交 5 个正交 commit
> （`153613ee5a` #600 / `4ac73c5f82` #546+#581 / `85dba5c882` #532 / `4a57113c0a` #533 /
> `9bc344788d`+`872d193a83` 报告群）。当前 HEAD（`872d193a83`）下复跑 `cargo test --lib`
> 仍为 **2040 passed / 1 failed（唯一红 #491）**，与本票时点逐位一致。

## 0. commit 分离说明（并发工位）

本票动手时，p123 的工作树含**另一 session 尚未提交**的 #532 长函数拆分（`CausalSeries` /
`write_*_lines` 等）。为不吞并他人产出，commit 用 **hunk 级分离**：从
`git diff HEAD -- p123` 的 19 个 hunk 中只 `git apply --cached` 本票的 11 个，并把 #532 专有的
`LifecycleRevision` import 从 index 版剔除；再用 `git write-tree` 导出 index 树独立编译验证
（p123/nest_lifecycle/classifier 三文件零警告，`cargo test --lib` = 2035 passed / 唯一红 #491）
后才 commit。事后验证分离正确：#532 由其本人的 commit `85dba5c882` 独立入库，两边内容无重叠、
无丢失。

## 1. 结论

**L2 的「完成前活窗」已在生产真实可见。** BTC 100k 前缀：38 只 L2 完成身份中
**19 只（50.00%）在完成前即已被观察**（`observed_at < completion_as_of`），提前量
min/median/max = **17 / 146 / 510** bar；L2 闪现率由 **38/38 = 100%** 降到
**19/39 = 48.72%**；L2 首次出现 **2 条** ForceOvertake 链与 **20 条**非闪现寿命
（min/median/max = **16 / 116 / 510** bar，此前恒为 0）。

剩余 19 只全部落到可审计原因码（9 `structure_not_locatable` / 5 定位到别的 C /
3 `tower_level_absent` / 2 `center_not_consolidation`）——**38/38 有归因，零「不知道」**。

**L1 与 L3 逐位零回归**：L1 的 156/202（77.23%）覆盖率、终局分布、寿命分位与 #527 完全一致；
L3 仍 0/8（本票 Scope 外，#602 承接）。

递归塔的活动语义按 #598 裁定以**只读访问器**暴露：`TowerCache::level_scan_cursor` /
`TowerCache::l0_units`——`LeveledMove`、`recursive_tower.rs`、既有塔消费方**零改动**。

## 2. 修正要点（文件:行号，均为 commit `961a4260ee` 后的行号）

### 2.1 `rust/src/theta_v0/classifier/mod.rs`（+27 / −0，纯新增）

| 位置 | 改动 | 作用 |
|---|---|---|
| :1026 `TowerCache::level_scan_cursor(level)` | 新增只读访问器，返回产出 `tower[level]` 的那一级 `WindowScanCursor` | provider 侧「虚拟追加行进中下级单元后重扫」的合法起点（`resume_from`）。下标口径与 `tower_confirmed_len` 一致；`level==0` 与越界级返回 `None`（缺级不猜值） |
| :1038 `TowerCache::l0_units()` | 新增只读切片访问器（`l0_units_cache`） | L1 层窗口扫描的 confirmed 输入 units |

两者均**只读**：不暴露可变引用、不改私有字段可变性、不改塔存储形状。函数文档显式登记
「`WindowScanCursor` 由内部实现细节升格为外部只读契约」的边界（#598 §3.1 负项的应对）。

### 2.2 `rust/src/theta_v0/classifier/nest_lifecycle.rs`

| 位置 | 改动 | 作用 |
|---|---|---|
| :1508 `provide_active_pan_live_windows` | 由 `provide_l1_active_pan_live_windows` 泛化：`frontier: &ActiveSegmentFrontier` → `active: Segment` | 级别中立（判据只作用在 `centers`/`kinds`/`confirmed_segments`/`active` 四元组，`level` 只随窗口带出）；L1/L2 共用同一实装，禁第二查法 |
| :1592 `PanLiveOutcome` | 由 `L1LiveOutcome` 更名 | 名实一致（它现在也承载 L2；tag 名与实际级别不符是声明膨胀，090） |
| :1654 `ActiveWindowFrontier` | 新增：行进中的 L1 窗口单元载体（方向/起止/外缘） | L2 活窗唯一合法的 C 腿；`as_segment()` 与 `level_view::leg_as_segment` 逐字同口径 |
| :1691 `ActiveWindowOutcome` | 新增：派生结果 + **L2 特有的未产出原因码**（5 类） | 「这只 L2 身份为何没有更早 Live」的可审计落点 |
| :1749 `active_l1_window_frontier` | 新增本体：`tower[0]` confirmed units + L0 行进中段**虚拟追加** → 从 `resume_from` 重跑**与塔逐字相同**的窗口扫描（`detect_centers_windowed_resume` + `center_from_segments`），取末窗 | 派生「当下若把行进中 L0 段计入、L1 层会形成但塔上尚不存在的候选窗口」 |

**禁外推判据**（#523 红线的 L2 对应条款，:1785）：末窗必须含虚拟追加单元
（`win.1 + 1 == units.len()`，等价于 `WinMeta.read_end_src == usize::MAX` 但不依赖该私有侧车）。
否则该末窗由纯 confirmed 单元构成、与 `tower[1]` 已有窗口同源 ⟹ 判
`LowerFrontierNotAbsorbed`，空产出。拿 `tower[1]` 的任何已产出窗口（含协议性每 bar 重扫的
末窗）冒充活动 C，就是与完成事件同源同判，逐字重演 #523。

**级别范围诚实登记**：函数固定 L1 层 build（`center_from_segments`），不加 `is_l0` 之类的
预留参数——L3 需要的是 `center_from_window` 几何判据 + 另一层输入，属另一次递归复合，由 #602 另立。

### 2.3 `rust/src/bin/p123_fast_replay.rs`

| 位置 | 改动 |
|---|---|
| :518 / :541 | `l1_live_outcomes` 的 key 由 `&str` 改 `(level, &str)`；`L1LiveDiagRow` 新增 `level` 字段——L1/L2 共用同一诊断面，**禁合并计数**（合记则两级读数互相污染） |
| :725-731 | stderr `P527_L1_LIVE_OUTCOMES` 按级别分列（`l{level}:{tag}=n`） |
| :1069 / :1105 / :1108 / :1172 | 活窗重算触发由二元组扩为三元组：`forest_epoch ∨ frontier ∨ level_scan_cursor(1)`。**L1 扫描断点必须独立进判据**——它可在 `forest_epoch` 不变时前进（不成立支 `i += 1` 推进 `consumed`/`resume_from` 而无窗口产出 ⟹ 塔字节未变 ⟹ epoch 不 bump），否则 L2 派生会读到陈旧锚 |
| :1139 / :1160 | dump 行 `L1_LIVE_{HIT,MISS,RECOMPUTE}` → `PAN_LIVE_{HIT,MISS,RECOMPUTE}` + 新增 `level=` / `l1_resume_from=` 字段（tag 名写死 "L1_" 却承载 L2 = 声明膨胀） |
| :1642-1669 | `recompute_lifecycle_window_stems` 循环上界 `2.min(tower.len())` → 写死 `1..3`，**级别是否已在塔上涌现改由显式原因码 `tower_level_absent` 回答**，不静默跳过 |
| :1706 | level=2 分支：调 `active_l1_window_frontier` 派生 C 腿；派生失败逐码落 tally + diag 行 |
| :1734 | **confirmed 与 active 不得重叠**：行进中 L1 单元与 `tower[1]` 末窗同起点时，从 confirmed 序列 pop 掉该末窗 |

第 :1734 条的必要性有实测：塔的末窗即使仍开放（未被 non-extension 单元终结、每 bar pop 重扫），
在类型上也与确认窗口不可区分（#598 §1.1）；行进中 L1 单元正是它「计入行进中 L0 段」后的形态，
二者同起点。不截断则 `active.start_index < last.end_index` 恒真 ⟹ L2 恒判
`frontier_not_after_confirmed`（**实测 844 次**，截断后降到 93、`window` 从 97 升到 314）。
判据取**同起点**而非「末项一律截断」：虚拟单元 seed 出新窗口时活动腿起点严格晚于末窗起点，
此时末窗确已被终结，必须保留在 confirmed 侧。L1 侧的对应事实是 parser 的 `l0.segments`
天然不含 pending 段——本条只是把塔补齐到同一语义。

## 3. 红绿记录

| 项 | 结果 |
|---|---|
| `cargo test --lib`（debug） | **2040 passed / 1 failed / 137 ignored** |
| `cargo test --release --lib` | **2040 passed / 1 failed / 137 ignored** |
| 唯一红 | `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`（#491） |
| 零新增失败证明 | 同工位、同他 session 未提交面、仅把本票两个 lib 文件还原到 `4f941af2b9` 实测 **2036 passed / 1 failed / 137 ignored**，失败集合逐项相同；差额 **+4 = 本票新增 4 条测试** |
| commit 自洽性验证 | 从 `git write-tree` 导出的 index 树（= `4f941af2b9` + 仅本票三文件）独立编译：p123/nest_lifecycle/classifier 三文件**零警告**；该树 `cargo test --lib` = 2035 passed / 1 failed（唯一红 #491） |
| 新增测试 | `q1_active_l1_window_frontier_absorbs_pending_l0_segment`、`q2_active_l1_window_frontier_rejects_tower_only_window`、`q3_active_l1_window_frontier_reason_codes`、`q4_l2_live_window_bridges_completion_without_backfill`（4 条全绿） |
| 既有 lifecycle 测试族 | 36 条（T/F/P 全族）改签名后逐条重跑全绿 |

新增测试覆盖票面点名的四面：**L2 活窗产出**（q1 派生 + q4 产窗）、**同身份衔接**（q4 桥迁移，
`book.len()==1`）、**零回填**（q4 `observed_at=95 ≠ c_start=69`，完成不改写观察钟）、
**原因码路径**（q2 禁外推臂 + q3 四码逐条可达）。

**一次 flaky 观测（照实登记）**：首轮全量 `cargo test --lib` 曾出现
`theta_v0::backtest::open_ledger::tests::parent_units_follows_latest_instance_vertical` 失败一次；
其后同一二进制复跑 2 轮全量 + 3 轮定向均绿。该测试属他 session 未提交面（#571 typed ledger），
`open_ledger.rs` 不引用 `nest_lifecycle`，本票三文件亦不被其引用——记为并发顺序相关的既有
flaky，不并入本票红绿。

## 4. 五项验收逐条

### 验收 1：每个 L2 Completed 要么 earlier Live 要么可审计原因码（BTC 100k 全量 join）

归因窗 = C 段真实活跃期 `[c_start, completed_at]`（#559 C1 口径）；桥身份 = 除 `seg_c` 右端外
全等的六元锚。

| 归因 | 只数 | 说明 |
|---|---:|---|
| A. 存在 earlier Live（`observed_at < completion_as_of`） | **19** | 提前量 17 / 146 / 510 bar |
| B. `structure_not_locatable` | 9 | C 活跃期窄锚与 A′ 都无法定位，或 Extreme 预滤未过（该假设当时确实尚未成立） |
| B. `window`（定位到**别的** C） | 5 | 该 run 定位成功但落在同锚的另一个 λ_C 上（`window_other_c` 逐只可查 dump） |
| B. `tower_level_absent` | 3 | C 活跃期整段落在「塔还没长出 L2」的时期（completed_at = 3477 / 5622 / 5905，首个 L2 完成事件可见 bar = 8179） |
| B. `center_not_consolidation` | 2 | C 活跃期最近已确认中枢当时不属 Consolidation 块 |
| **合计** | **38** | **零「不知道」** |

`tower_level_absent` 是本票为闭合归因新加的原因码：完成事件的物理完成 bar 可远早于其首次可见
bar（L2 滞后最大 4702），故一只 L2 身份的 C 活跃期可能整段落在塔尚未涌现该级别的时期。原实装
的 `for level in 1..3.min(tower.len())` 会**静默跳过**，使这 3 只在归因表里没有任何行——与
#527 §9.2「补记命中行」同一纪律，改为写死上界 + 显式落码。

L2 特有缺口新设原因码共 6 个（`ActiveWindowOutcome::reason_tag` 5 个 + provider 侧
`tower_level_absent`），全部按实际失败位置命名，未复用 L1 码表冒充：

`no_lower_frontier`（parser 无 pending 段）／`lower_frontier_not_after_units`（倒灌）／
`resume_anchor_out_of_range`（塔与 units 不同步，实测 1 次）／`no_window_formed`（虚拟追加后
seed 不过）／`lower_frontier_not_absorbed`（**禁外推臂**，实测 557 次）／`tower_level_absent`。

### 验收 2：BTC 100k 复测读数照实（不预设复现任何数值）

**前后对照（pre = `4f941af2b9` + 他 session 未提交面，post = 本票 commit；唯一差异 = 本票三文件）**

| L2 指标（BTC 100k） | pre | post |
|---|---:|---:|
| 完成身份数 | 38 | **38**（零丢弃、零新增） |
| 存在 earlier Live | **0（0.00%）** | **19（50.00%）** |
| 提前量 min/median/max | — | **17 / 146 / 510** bar |
| 账本 entries | 38 | **39** |
| 闪现（零寿命）终局 | **38** | **19** |
| 非闪现终局 | **0** | **20** |
| 非闪现寿命 min/median/max | — | **16 / 116 / 510** bar |
| 闪现率 | **100.00%** | **48.72%** |
| ForceOvertake（反超） | **0** | **2** |
| NeverConstituted | 11 | 10 |
| Confirmed | 27 | 26 |
| `IdentityVanished{HypothesisRefuted}` | **0** | **1** |
| `IdentityVanished{ObservationSeam}` | **0** | **0** |
| Provisional | 0 | 0 |

**两类消失分列**：L2 只出现 `HypothesisRefuted` 1 只，`ObservationSeam` **0 只**。这与 L1 的
形态不同（L1 为 25 / 12）——照实登记，不外推成因：L2 的活动 C 腿每 bar 由塔的 `resume_from`
重扫派生，其 λ_C 的换轨频率与 parser 末段重划不同源；本票不猜该差异的机制，留待 #602 与桥接
语义研究票（#597 Not-yet-specified）一并审读。

**L1 / L3 零回归（pre 与 post 逐位相同）**

| 指标 | L1 pre | L1 post | L3 pre | L3 post |
|---|---:|---:|---:|---:|
| earlier Live | 156/202（77.23%） | **156/202（77.23%）** | 0/8 | **0/8** |
| entries | 239 | **239** | 8 | **8** |
| flash / nonflash | 46 / 192 | **46 / 192** | 8 / 0 | **8 / 0** |
| 寿命 min/med/max | 1 / 79.5 / 423 | **1 / 79.5 / 423** | — | — |
| force / never / refuted / seam | 35 / 64 / 25 / 12 | **35 / 64 / 25 / 12** | 0 / 1 / 0 / 0 | **0 / 1 / 0 / 0** |

全局：`completion_signals` 248→**248**、`channel_switches` 248→**248**、
`retrograde_rejected` 0→**0**、`completion_force_unavailable` 0→**0**、`as_of` 全局单调保持。
`extension_suppressed` 12469→27140（L2 新增活窗后完成时抑制延展的增量，设计内）。

**20k 前缀同向复现**：L2 完成 8 只、earlier Live 0→**2**（25.00%，提前量 100/189.5/279）、
L2 闪现 8→6、非闪现 0→3（16/100/159）、ForceOvertake 0→1；L1 侧 29/38（76.32%）不动。

**活窗定位结果分布（100k，stderr `P527_L1_LIVE_OUTCOMES`，按级别分列）**

```
l0:no_active_frontier=57
l1:window=608  l1:structure_not_locatable=817  l1:center_not_consolidation=387  l1:tower_level_absent=400
l2:window=314  l2:structure_not_locatable=423  l2:center_not_consolidation=304
l2:lower_frontier_not_absorbed=557  l2:frontier_not_after_confirmed=93
l2:resume_anchor_out_of_range=1  l2:tower_level_absent=520
```

`l2:lower_frontier_not_absorbed=557` 是**禁外推臂的实际工作量**：这 557 次里，塔的末窗都在
那一刻可被拿来当「活动窗口」用，本票逐次拒绝。

### 验收 3：稳定身份机制 + 完成钟延伸在案

**稳定身份（#523 遗留 2 在 L2 的证明方向）**：L2 的桥判身份继续复用已 level-agnostic 的
`bridge_identity` / `same_anchor` / `VanishCause` 机制（#598 §1.3 已核实其 level 无关性），
**未新增身份规则**。L2 侧的「行进中→确认后同身份」由两条支撑：

1. **λ_C 在同一 C 段上恒定**：活窗左端 = `structure.seg_c.0`，由 `departure_move_c_start` 在
   `[B.end_index, C.start_index]` 上定界，只依赖 C 段的**左端与方向**；行进中 L1 单元的
   `start_index` = 其窗口首单元起点，该窗口被塔 emit 为 confirmed 单元后首单元起点不变
   ⟹ 同一 λ_C ⟹ 桥判同身份。q4 单测把这条钉成断言（`book.len()==1`，桥迁移非新建）。
2. **生产实证**：100k 的 19 只 earlier-Live L2 身份全部经桥迁移完成（`channel_switches`
   pre==post==248，无一只走「旧活窗消失 + 完成事件另起闪现」）。

**限定（照实，不写成无限定断言）**：与 L1 同构地，跨 C 段的恒等**不成立**——完成事件首次可见
bar 晚于物理完成 bar（L2 滞后 19/64/4702），滞后超过活窗存活期时桥不判同身份。L2 实测该现象
落在 `ObservationSeam=0`（本窗未触发），但这是**本样本读数**，不是「L2 不会发生接缝」的证明。

**完成钟 provenance 在 L2 的延伸（#523 遗留 3）**：完成钟三分（物理完成 `completed_at` /
事件首见 `observed_completion_at` / 账本收到 `as_of`）与 `find_move_by_end_index` 查证机制
**本就 level-agnostic**（#598 §1.4），本票未改一行即在 L2 生效：38 条 L2 完成信号全部携带
`completed_lower_id`（level=1 的 `ElementId`）与 `completed_at`，三钟恒序不变量
（`completed_at ≤ observed_completion_at ≤ as_of`）在 release 亦执行、零违序。L2 的
completion_lag（`as_of − completed_at`）= 19 / 64 / 4702 bar，逐只可查 dump。

### 验收 4：边界

| 边界项 | 证据 |
|---|---|
| 塔存储零改动 | `git show 961a4260ee --stat -- rust/src/theta_v0/classifier/recursive_tower.rs` **为空**（`LeveledMove`/`compose_level{,_resume}`/`detect_centers_windowed_resume`/`WindowScanCursor`/`WinMeta` 全部未动） |
| 既有消费方签名零改动 | commit 只触 3 文件；`classifier/mod.rs` 的 diff **删除行数 = 0**（纯新增两个 `pub fn`），`classify_with_tower_incremental{,_resume}` 返回形状不变 ⟹ backtest/m8/`admission.rs`/`TreeCache`/`extract_carrier_forest` 等全部塔消费方零适配 |
| provider 侧签名变更的影响面 | `provide_active_pan_live_windows`/`PanLiveOutcome` 的消费方只有 `p123_fast_replay`（全仓 grep 实证；`p409_pan_live_probe` 消费的是旧 `provide_pan_live_windows`，未动） |
| #449 禁令零触碰 | 本票方向 = 递归塔（判据侧）→ PanLive provider（只读消费方），纯读；不产 `TypedNestCertificate`、不读 `turn_class`、不回写/过滤/改判任何 `Classification`/`BspBits`。#449 覆盖的是「证书→回灌判据」的反方向 |
| #523 永禁清单逐条 | 见下表 |

**#523 永禁清单逐条核（#527 票面原文六条）**

| 禁条 | 核验 |
|---|---|
| completion 延迟到下一 bar / 下一 trigger | 未改完成相任何代码；`completion_signals` 248→248、`channel_switches` 248→248 逐位不变 |
| 同 bar completion 丢弃 | 同上；`retrograde_rejected=0` |
| p409 `structure_completed=false` 接生产 | `p409_pan_live_probe.rs` 零改动，未进 p123 路径 |
| `observed_at` 回填到 c_start 或 seg_c.end；`as_of` ±1 凑数 | 活窗 `observed_at` = 实际观察 bar；q4 单测断言 `observed_at != c_start`；生产 L2 提前量 min=17 bar（既非 0 也非 c_start 回填）。`ActiveWindowFrontier.end_index` 取行进中 L0 段的**极值结构点**，禁 `as_of` 冒充结构点 |
| 未证明语义即拿 `MoveBlock.status` 当 segment completion | 本票不读 `MoveBlock.status`；行进中判据取「末窗是否含虚拟追加单元」（等价 detect 的数据耗尽停止条件），是扫描算法自身的语义，不是借用第三方状态位 |
| generic `NestCandidateEvent(kind=Consolidation)` 自动等价 Completed；完成后继续延展 live | 完成相仍走 `lifecycle_bar_phases` 的 typed 路径（携 `completed_lower_id`/`completed_at`，查不到即停线），本票未改；完成后延展由 book 状态机抑制（`extension_suppressed` 计数照常工作） |

### 验收 5：字节护栏 + 测试门

pre = `/tmp/wt601-baseline`（`4f941af2b9` 净出 + 本工位他 session 未提交面逐文件同步，
p123 由本票 patch 反向还原 ⟹ **唯一差异 = 本票三文件**）；post = 本工位。

| 面 | 窗 | cmp | SHA-256（前 16） |
|---|---|---:|---|
| p123 stdout | 20k | **0** | `bd9ac1d655f9d615` |
| P116 dump | 20k | **0** | `fcc8016a9a01a109` |
| p123 stdout | 100k | **0** | `d8b69c180c23c5e3`（与 #527 §4 验收 3 记录**逐字相同**，跨票一致） |
| P116 dump | 100k | **0** | `8a7327feb3b9ba29`（同上） |
| m8 trades / tower_events | p3fold | **0 / 0** | `8196a2941bd3560f` / `83f45a36ab422a92` |
| m8 trades / tower_events | wf7 | **0 / 0** | `868e415081c78520` / `aa96b3e836be5a43` |
| m8 trades / tower_events | wf8 | **0 / 0** | `b265c52ed27967d6` / `1d8dff0d29e8925f` |

三个 `tower_events` SHA 与 #527 §4 记录逐字相同；三个 `trades` SHA 与 #527 不同——那是他
session 未提交面（#533 golden 重锚 / #571 typed ledger）造成的，pre/post 同基线对拍仍 cmp=0。

**P-H3 复用率**：pre/post 均 `provider_requests=2099 provider_reevals=87 provider_reuses=2012`，
复用率 **95.855169%** 不动。

**lifecycle dump 是本票本体，不作 cmp=0**（同 #527 口径）。post 产物 SHA-256：
20k `00b68a99722f8487…`、100k `4697f7644bf2b859…`（pre 分别 `588d8af17cf12ee6…` /
`85b82fbe49d5222a…`）。

测试门见 §3：debug/release 均 2040 passed / **唯一红 #491**，零新增失败。

产物：`/tmp/wt601-{pre,post}-{20k,100k}.{stdout,p116,dump,stderr}`、
`/tmp/wt601-{pre,post}-m8-{p3fold,wf7,wf8}/`。

## 5. 与 #598 报告 L2 最小切片建议的偏差归因

| #598 §4.2 建议 | 实装 | 归因 |
|---|---|---|
| 步骤 1：`TowerCache` 新增 `level_scan_state(level) -> (&WindowScanCursor, &[WinMeta])` | 实装为**两个**访问器：`level_scan_cursor(level) -> Option<WindowScanCursor>`（值拷贝，`Copy` 类型）+ `l0_units()` | `WinMeta` **不需要暴露**：行进中判据「末窗含虚拟追加单元」与 `read_end_src == usize::MAX` 等价，且由重扫结果自证，读私有侧车反而扩大读契约面（#598 §3.1 负项）。改用 `Option` 返回是为了让缺级走显式原因码而非 panic。另需 `l0_units()`——#598 未列出这一项，但派生的 confirmed 输入必须来自与塔同源的 units（从 `tower[0]` 反投影会引入第二查法） |
| 步骤 3：`provide_l1_active_pan_live_windows` 泛化为「能转 Segment 的活动腿」接口 | 直接把参数改为 `active: Segment`（不引入 trait） | 该函数本就 level-agnostic，唯一 L1-specific 处是入参类型。引入 trait 只为一个方法且只有两个实现者，属过度抽象；改 `Segment` 后既有测试覆盖全部复用，diff 最小 |
| 步骤 4：循环上界 `2.min(tower.len())` → `3.min(tower.len())` | 改为写死 `1..3` + 级别未涌现时落 `tower_level_absent` 码 | `3.min(tower.len())` 会**静默跳过**塔尚未长出 L2 的时期，导致 3 只 L2 身份在归因表里没有任何行（验收 1 的「零不知道」不成立）。这是 #598 静态分析未预见、由 100k 实测暴露的 |
| §3.1 预判：「派生函数不能简单复制 `active_segment_frontier`，需要为每一级单独写虚拟追加+重跑判定的逻辑」 | 属实 | L2 的派生（~50 行）确实无法由 L1 参数化得到 |
| §3.1 未预见项 | **confirmed 与 active 重叠**（§2.3 :1734） | #598 §1.3 判断「缺口严格局限在活窗产出这一层，不需要动账本状态机」——正确；但未预见 `tower[1]` 的开放末窗会同时以 confirmed 与 active 两个身份进入判定。实测 844 次 `frontier_not_after_confirmed` 才暴露。这是塔与 parser 的**结构性差异**（parser 的 confirmed 与 pending 是两个字段；塔的开放末窗与确认窗口同一容器），#598 §1.1 已钉住该差异，只是未推到这一步后果 |
| §2 数据：L2 均滞后 823 bar ⟹ 潜在信息增量比 L1 大 | 实测 L2 覆盖率 **50.00%** < L1 的 77.23%；但 L2 非闪现寿命中位 **116 bar** > L1 的 79.5 bar | 「滞后大」预示的是**单只可观察窗口更宽**（已兑现：寿命中位更长、最大 510 vs 423），不是「覆盖率更高」。覆盖率受限于结构判据（`structure_not_locatable` 9 只 + `center_not_consolidation` 2 只 = 该假设当时确实尚未成立）与塔涌现时点（`tower_level_absent` 3 只）。#598 §2 原文已限定为「是否能同比例转化取决于未产窗损耗」，未作承诺 |

## 6. 结果包六要素

1. **结论**：见 §1、§4。L2 完成前活窗已生产化（19/38 有 earlier Live，闪现率 100%→48.72%）；
   递归塔活动语义经只读访问器暴露，塔存储与既有消费方零改动；L1/L3 逐位零回归。
2. **定义依据**：
   - 行进中 L1 单元的合法构造 = 「`tower[0]` confirmed units + parser `OpenTail.pendingSegment`
     虚拟追加」经 `detect_centers_windowed_resume` + `center_from_segments` 重跑——**与塔自身
     每 bar 用的是同一函数、同一 build、同一 resume 锚**（`WindowScanCursor::resume_from` 的
     frontier 协议，`recursive_tower.rs` 模块头「唯一合法 resume 协议 = pop 末位 center +
     从 `resume_from` 重扫」）。禁第二查法。
   - 行进中的判别 = 末窗因**数据耗尽**而停（无 non-extension 哨兵），即 `detect` 的
     `j == units.len()` 分支，等价于 `WinMeta.read_end_src == usize::MAX`（#598 §1.1 关键判别位）。
   - 方向/外缘口径与 `level_view::lower_legs_from`（`first_leaf_direction` + `rmove.lo()/hi()`）
     逐字同源；外缘读 `center.dd/gg` 由 `LeveledMove::envelope` 的投影契约保证 bit-equal。
   - 盘整背驰 A/C 结构判据全部复用单一来源（`nearest_confirmed_center_idx` /
     `locate_pan_div_structure` 窄锚 → `locate_pan_div_structure_front_anchor` A′ 回退 →
     `pan_div_structure_extreme` 预滤），与完成事件 provider 同序同判（措辞限定沿用 #591 裁定）。
   - 完成证明：lower unit 在 `tower[level-1]` 上按 `end_index` 查证存在
     （`find_move_by_end_index`），取 `ElementId` 与物理完成 bar——level-agnostic，未改。
   - 活假设结算语义不动：061:26 / 061:28 / 024:24 与 R43 裁定
     （`chanlun/escalate/r43-lifecycle-ruling-20260721.md:31-39`）。
3. **边界条件（结论在何时翻转）**：
   - 若 `WindowScanCursor.resume_from` 的语义在其他票中被重构（例如增量扫描改为不回退末窗），
     `active_l1_window_frontier` 的重扫起点即失去合法性，L2 活窗会静默产出与塔不一致的窗口
     ——`TowerCache::level_scan_cursor` 的文档已把这条读契约边界显式登记，重构时必须同步核对；
   - 若 parser 的 `tail` 不再输出 `PendingSegment`，L1 与 L2 的活动 C 腿**同时**归零（L2 的腿
     也由该段虚拟追加派生）⟹ 两级一起退回首见即完成；
   - 若 §2.3 :1734 的「同起点截断」被改成「末项一律截断」，虚拟单元 seed 出新窗口的场景会误删
     一个真已终结的 confirmed 单元 ⟹ λ_C 定界窗口跨过缺失单元 ⟹ 活窗身份漂移；
   - 若给 L3 直接复用 `active_l1_window_frontier`（build 是 `center_from_segments`），等于在
     几何判据层用了完整判据，会产出塔上不可能存在的窗口——本票已在函数文档明写拒绝；
   - 若把 `tower[1]` 的开放末窗直接当活动 C（省掉虚拟追加+重判定），则与完成事件同源同判，
     重演 #523——q2 单测即为此设的锁。
4. **下游推论**：
   - #597 map 的「46 只 L2/L3 身份钉因」目标 L2 部分达成（38/38 有归因，19 只拿到活窗），
     L3 的 8 只仍全数闪现，由 #602 承接；#602 需要的「L2 层行进中窗口」判据是
     `center_from_window`（几何路径），输入是 L1 层 units（`levels[0].projected_units`）+ 本票的
     `ActiveWindowFrontier` 作虚拟单元——需新增第三个只读访问器（本票**不预留**）；
   - L2 的 `ObservationSeam=0` 与 L1 的 12 形成对照，是否意味着塔侧换轨机制与 parser 段重划
     不同源，属桥接语义放宽研究票的输入，本票只给读数不作机制断言；
   - 新出现的 2 条 L2 ForceOvertake 与 20 条 L2 非闪现寿命使「递归级别上的活假设寿命」首次可统计，
     但它是 **L2 级证据（BTC 单标的单窗真实数据）**，跨品种外推需另跑（#527 §7 遗留 5 同口径）。
5. **谱系引用**：#598（架构裁定：路线 i + 先 L2 后 L3，编排者 2026-07-28）；#523（PanLive
   provider 接缝根因，遗留 1/2/3）；#527/#559/#578/#592（L1 实装 + 修复轮 + 有洞 frontier 复核 +
   `gap_len` 诊断，本票的直接先例与措辞限定来源）；#591（同序同判措辞限定）；#449（证书真值路径
   边界，本票读取方向核验为不落入禁令覆盖范围）；#533（p123 字节护栏门，见 §7 遗留 1）；
   090（声明=能力：本票把「L2 活窗」从缺口声明变成能力，并把做不到的 L3 明写为缺口；dump 行
   tag 由 `L1_*` 改 `PAN_*` 亦出自这条）；`formalization-validity-domain`（认识论等级：派生判据
   L0 结构、覆盖率与寿命读数 L2 真实数据单窗）；`no-patch-mentality`（§2.3 :1734 选择在源头补齐
   语义而非在 provider 内加特例分支）。
6. **影响声明**：改动 3 个生产文件（`classifier/mod.rs` 纯新增两个只读访问器、
   `classifier/nest_lifecycle.rs`、`bin/p123_fast_replay.rs`）。受影响面 = lifecycle sidecar
   （账本、dump、stderr 审计）；**未改** `recursive_tower.rs` / 塔存储 / 塔消费方签名 /
   p123 stdout / P116 dump / m8 trades / tower_events / 装配 / 证书 / `Cargo.toml` / p409 /
   他 session 未提交面。commit 只 stage 本票三文件的改动（p123 用 hunk 级 patch 精确分离
   #532 长函数拆分的未提交改动，index 树独立编译验证通过，见 §3）。

## 7. 遗留（不在本票 Scope，照实登记）

1. **`issue533_p123_byte_guardrail` 的 lifecycle dump golden 须重锚（待编排者批）**。该门把
   `P421_LIFECYCLE_DUMP` 也纳入 golden（2000-bar 全文 + 20k/100k SHA）。本票故意改变了
   lifecycle dump（新增 L2 活窗行 + `level=` 字段 + tag 由 `L1_*` 改 `PAN_*`），两条测试现红
   （当前 HEAD `872d193a83` 下复测确认）：
   `P533 DRIFT：2000-bar lifecycle dump 与仓内 golden 不再逐字节相等` /
   `max_bars=20000 lifecycle dump SHA-256 与仓内 golden 不等`。**stdout 面未漂移**——2000-bar
   的 stdout 全文断言与 20k 的 stdout SHA 断言都先于 dump 断言通过（本票另有 20k/100k stdout
   cmp=0 独立实证，§4 验收 5）。
   本票动手时该门与其 fixture 尚是他 session 的未跟踪面，按工位纪律未改；其后已由 `4a57113c0a`
   （#533）正式入库。按该文件自述的「golden 只允许因**已审阅、故意的**行为变化而更新，且更新须
   在同一 PR 里说明原因」条款，重锚属**需编排者批准**的动作（同 #571 treasury ArmR golden 重锚
   先例），本票不自行重生成 golden——留作交卷项。该门是 `#[ignore]` 集成测试，不在票面
   `cargo test --lib` 门内，不影响 §3 的红绿判定。
2. **L3（8 只）仍全数闪现**（#602）。需要「L2 层行进中窗口」的派生：build 换
   `center_from_window`、输入换 L1 层 units + 本票 `ActiveWindowFrontier` 作虚拟单元、
   重扫锚取 `level_scan_cursor(2)`。本票不预留参数、不写半成品。
3. **L2 的 19 只未覆盖身份的可修性**。9 只 `structure_not_locatable` + 2 只
   `center_not_consolidation` 是结构事实（假设当时尚未成立）；5 只「定位到别的 C」与 3 只
   `tower_level_absent` 中，前者是否可通过同锚多候选 C 并行产窗改善、后者是否可通过塔的
   bootstrap 期语义补齐，需逐案审读，本票不猜。
4. **`resume_anchor_out_of_range` 实测 1 次**。塔与 `l0_units` 在某个 bar 不同步（`resume_from >
   l0_units.len()`）。当前按「不猜锚、落码、跳过本级」处理（诚实空产出）。该现场的成因（cascade
   回退与 units 重建的时序）未追，出现率 1/2269 次重算，不影响归因闭合。
5. **L2 的 `ObservationSeam=0`**：本样本未出现观测接缝型消失。不得据此断言「L2 不会发生接缝」
   ——L2 completion_lag 最大 4702 bar 远超其活窗寿命中位 116 bar，接缝的结构前提存在。
6. **跨品种外推**：50.00% / 17-146-510 / 48.72% 全部是 BTC 100k 单窗读数，不得写成规格常量。

## 8. 复现命令

```bash
cd rust
CARGO_TARGET_DIR=/tmp/kimi-nest-target-601 cargo build --release --bin p123_fast_replay
for w in 20000 100000; do
  n=$([ $w = 20000 ] && echo 20k || echo 100k)
  P116_MAX_BARS=$w P116_DUMP=/tmp/wt601-post-$n.p116 P421_LIFECYCLE_DUMP=/tmp/wt601-post-$n.dump \
    /tmp/kimi-nest-target-601/release/p123_fast_replay ../analysis/data_cache/btc_1m_full.json \
    > /tmp/wt601-post-$n.stdout 2> /tmp/wt601-post-$n.stderr
done

# m8 三窗（护栏面）
for w in p3fold wf7 wf8; do
  M8_WIN_FILTER=$w OPSEM_DUMP_DIR=/tmp/wt601-post-m8-$w \
    CARGO_TARGET_DIR=/tmp/kimi-nest-target-601 \
    cargo test --release --lib m8_e2e_all_systems_oos -- --ignored --nocapture
done

# 逐身份 join / 归因 / 终局分列（脚本：/tmp/wt601_{join,attrib,settle}.py）
python3 /tmp/wt601_join.py   /tmp/wt601-post-100k.dump
python3 /tmp/wt601_attrib.py /tmp/wt601-post-100k.dump
python3 /tmp/wt601_settle.py /tmp/wt601-post-100k.dump
```
