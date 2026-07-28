# 影子评审 #559：#527 PanLive provider L1（两轴，新上下文，禁自评）

> 评审对象：commit `7b4547b623`（`feat(theta): #527 PanLive provider L1——完成前活窗可见`）
> diff 范围：`ff816d2ad3..7b4547b623`（2 个生产文件 + 1 份报告；rust 净增 152+737 / 删 30+289）
> 工位：`/tmp/kimi-nest-mainline` @ 分支 `kimi-nest-mainline-20260717`，HEAD = `7b4547b623`（评审开始时已核）
> 尺子：`gh issue view 527` 票面 Acceptance + 永禁清单；#421 E 裁定（comment-5106372198）+ 收口包裁定（comment-5106781616）
> 角色：影子评审（只读；本 session 未改仓内任何文件，只写本报告）

---

## 0. 结论

**PASS WITH CONDITIONS**

票面三项 Acceptance 的**实测读数全部独立复现**：156 只完成身份在完成前即被观察（提前 16/104.5/411 bar）、
八对字节护栏 cmp=0 且 SHA 与 #421 自查档逐字相同、测试门自跑唯一红 #491。
永禁清单七条逐条核过，**无一违反**。核心结论「L1 完成前活窗已在生产真实可见」成立。

条件全部落在**口径 / 归因 / 命名**层，不改变上述核心结论：

| # | 条件 | 级别 | 轴 |
|---|---|---|---|
| C1 | 修正 §4 的 46 只 L1 缺口归因表——它用错了时间窗；按 C 段真实活跃期重算得 29/6 + **11 只无原因码落地**，撤回「248/248 有归因，无『不知道』」 | MED | Spec |
| C2 | 撤回或加限定 `nest_lifecycle.rs:1352-1357` 的 c_start 恒等断言——实测 5 例反例，且与本报告自身 §7 遗留 2 互相矛盾 | MED | Spec |
| C3 | 修正「完成钟三分」口径——`observed_completion_at` 在生产接线下 248/248 恒等于 `as_of`，#421 遗留 3 未真正收口 | MED | Spec |
| C4 | 同步 `v3-lifecycle-rebuild-impl-20260724.md:6-7,100,111`——#430 收口条件 1 刚建立的口径因本票删符号而失真 | LOW-MED | Spec |
| C5 | 修正 `recompute_lifecycle_window_stems` 的 `misses`/`outcomes` 参数名倒置 | MED | Standards |

**不判 FAIL 的理由**：票面把 IdentityVanished 与 ForceOvertake 明写为「读数照实登记，不预设复现 p409 的 113/91.1%」；
交付在 §7 遗留里已把 L2/L3 与身份稳定性照实登记为缺口，未冒充能力。上述 5 条无一触及永禁清单，
也无一推翻「完成前活窗可见」这一交付本体。

---

## 1. 逐项核验（Spec 轴）

### 1.1 IdentityVanished 0→37（票面要求「保持 0」，交付自报未达成）

**(a) 37 条是否全部带审计终局记录、非静默丢弃 —— 成立。**

`/tmp/wt527-post-100k.dump` 的 REV 行分布（独立解析，非引用报告）：

```
19814 Supersedes | 285 Observed | 212 StructureCompleted | 194 FirstProvable
  148 Invalidated（76 NeverConstituted / 37 IdentityVanished / 35 ForceOvertake）| 136 Confirmed
```

37 条 IdentityVanished 每条都有显式 `REV ... revision=Invalidated { reason: IdentityVanished } evidence=None` 行，
带 `as_of` 与完整 key（level/side/seg_a/seg_c_full/b_center_start）。终局路径由
`nest_lifecycle.rs:955-970`（第 8 步身份消失扫描）产生，`entry` 保留不删（谱系保留）。
**取消原因可查到 `IdentityVanished` 这一层**；再往下的机制归因（parser 重划 vs provider 换窗）
不在账本字段内，需从 dump 侧信息重建——这是本次评审做的，也是下面 (c) 的落点。

**(b) 交付归因「parser 末段重划取消活窗」在数据上逐值复现 —— 成立。**

我按 (level, side, seg_a, b_center_start) 锚在 vanish bar 上重建同 bar 新身份，得到：

| 现场 | 交付报告 §4 | 本评审独立复现 |
|---|---:|---:|
| 同 bar 同锚出现新 c_start | 10（8 更早 / 2 更晚） | **10（8 更早 / 2 更晚）** |
| 同 bar 有其他锚的新身份建仓 | 10 | **10** |
| 同 bar 无新建 | 17 | **17** |
| vanished 前寿命 min/median/max | 4 / 39 / 423 | **4 / 39 / 423** |

10 只「同 bar 同锚换新起点」逐只列出（`as_of / old_c → new_c`）：
`2047/1869→1839`、`4378/3484→4184`、`6230/6146→5967`、`23184/22895→23083`、`60687/60536→60489`、
`65102/65026→65017`、`66720/66467→66433`、`69471/69415→69347`、`85813/85777→85727`、`94693/94590→94447`。

**(c) 但归因措辞不精确（并入 C2）。** 这 10 只里 **4 只**（`2047`/`65102`/`66720`/`94693`）的「新身份」
不是新活窗，而是**完成相直接建的闪现身份**——即活窗 c_start 与同锚完成事件 c_start 不等，
桥不判同 ⟹ 旧活窗被判消失 + 完成事件另起闪现。这不是「parser 重划取消活窗」，
而是「完成事件可见性滞后（中位 57.5 bar）超过了 C 段存活期，完成到达时活窗已换到下一个 C」。
见 §1.6 与 C2。

**(d) 裁量建议：见 §4。**

### 1.2 156+46+46=248 归因完整性

**156 earlier-live —— 成立（逐条 join 复现）。**
248 条 `COMPLETION_SIGNAL` 与 285 个 REV 身份按 `(level, side, seg_a, c_start, b_center_start)` join：

```
completions=248  earlier Live=156  same-bar(flash)=92  no-REV-match=0
lead bars min/median/max = 16 / 104.5 / 411
```

`earlier-live levels: {1: 156}`（全部 L1）✓；`flash levels: {1: 46, 2: 38, 3: 8}`。

**真闪现（true-flash）实测 0 —— 成立。**
`as_of − completed_at` min/median/max = **19 / 57.5 / 5190**，零值计数 = **0**。
248/248 的完成事件首次可见 bar 都晚于 lower unit 物理完成 bar ⟹ 没有任何一只「同 bar 出生并完成」。

**46 只 L2/L3 确属票面 Scope 外 —— 成立。**
`recompute_lifecycle_window_stems` 的 `for level in 1..2.min(tower.len())`
（`p123_fast_replay.rs:1687`）只对 level=1 产窗；L2=38 / L3=8 全部闪现，与票面 Scope
「L2/L3（递归塔 active lower-frontier 暴露面）不在本票」一致。

**46 只 L1 缺口原因码 —— 不成立（C1，MED）。**

报告 §4 给的 39 `structure_not_locatable` / 6 `center_not_consolidation` / 1 `no_active_frontier`
可以用「区间 = `[c_start, 完成 as_of]` 内 `L1_LIVE_MISS` 行的主导 reason」这一口径**精确复现 39/6**
（我复现到 39/6 + 1 只无覆盖）。但这个时间窗是**错的**：它把完成事件滞后期（中位 57.5 bar，最大 5190）
里的 MISS 行算成了「这只没有更早 Live」的原因——那段时间 C 段早已物理完成，本来就不该有它的活窗。

改用 C 段的真实活跃期 `[c_start, completed_at]` 重算：

| 归因 | 报告 §4 | 按 C 段真实活跃期 |
|---|---:|---:|
| `structure_not_locatable` | 39 | **29** |
| `center_not_consolidation` | 6 | **6** |
| `no_active_frontier` | 1 | **0** |
| **区间内无任何原因码行** | 0 | **11** |

11 只无原因码落地的身份中，**7 只**在其 C 活跃期内 provider 正在重算且**正在产窗**——只是产的是
**别的**身份的窗（`recompute_lifecycle_window_stems` 每个 run 最多返回 1 个窗，
`outcome.window()` 是 `Option`）。逐只（完成 `as_of` / C 区间 / 区间内 recompute 数 / 其中产窗数）：

```
15984 (15738,15821) 3/3   17447 (17170,17255) 4/4   25953 (25582,25890) 1/1
32893 (32653,32702) 1/1   43244 (42998,43175) 1/1   70395 (69803,69888) 1/1
1693  (1334,1352)   0/0   33862 (33670,33799) 0/0   64500 (64374,64459) 0/0
86879 (86812,86828) 0/0   94693 (94447,94510) 0/0
```

其中报告归为 `no_active_frontier` 的那 1 只（`as_of=94693`）与实测**直接相反**——
完成时刻 frontier 正在产窗：
`L1_LIVE_RECOMPUTE as_of=94693 frontier=(94590,94672,Down) stems_before=1 stems_after=1`，
且区间 `[94447,94693]` 内 `L1_LIVE_MISS` 计数为 **0**。

⟹ 报告 §4「BTC 100k 全量 join（248/248 有归因，无「不知道」）」**不成立**；
票面验收 1 第二分支要求的「明确、可审计的原因」在这 11 只上未达成。
（不判 FAIL：它们不是 true-flash 冒充，报告也没把它们并入 true-flash 分子；
问题是原因码**归错了**，不是缺口被隐藏。）

### 1.3 `ReplayPrefixFeed` / `feed_replay_prefix` 删除

**(a) 无测试/兼容调用方被一并移除 —— 成立。**
`git show ff816d2ad3..7b4547b623 -- rust/` 逐处核：全部 15 处 `feed_replay_prefix(...)` 调用
改写为测试内新增的 `feed_prefix_phases(...)` 夹具 adapter（`nest_lifecycle.rs:2968-3002`），
**无一个 `#[test]` 被删除**。T1–T19 / F1–F10 全部保留；测试门自跑总数 2024（见 §1.7）与报告一致。
全仓 grep 残留：`ReplayPrefixFeed`/`feed_replay_prefix` 在 `.rs` 中已零命中，仅剩 `.md` 报告类引用。

**(b) #430 条件 1 写入的文档口径因删除而失真 —— 成立，需同步注记（C4）。**

`chanlun/review-results/v3-lifecycle-rebuild-impl-20260724.md` 是 #430 收口条件 1 的口径载体
（2026-07-28 同日落笔），三处仍指向已删符号：

- `:6-7`（订正注记）：「`ReplayPrefixFeed`/`feed_replay_prefix` **仅为 legacy/测试兼容**」——符号已不存在；
- `:100`（第 2 条）：「2026-07-28 起 `p123_fast_replay` 已在生产重估 trigger 内调用 `feed_replay_prefix`」；
- `:111`（§7 第 1 条）：「……调用 `feed_replay_prefix`」。

`nest_lifecycle.rs` 模块头与 `provide_pan_live_windows` 文档已由本票改写（分水岭登记，`:1327-1333`），
T8 契约段也改成了「测试内 `feed_prefix_phases` adapter 仅为夹具展开」（`:2256-2257`）——
但那句尾巴「只在该兼容路径，跳过非 trigger prefix 才会晚记」在测试夹具语境下已悬空
（夹具按给定 as_of 直喂，不存在「跳过 prefix」）。LOW，并入 C4。

### 1.4 ForceOvertake 链 35 条

**批量核验（35/35 全过）**，逐条检查：首条为 `Observed` ∧ 含 `FirstProvable`（不早于 Observed）
∧ 恰 1 条 `ForceOvertake` ∧ `ForceEvidence` 含 `area_a`/`area_c`/`dif_peak_a` ∧ `as_of` 单调
∧ 末条为终局。**异常 0**（脚本首轮报的 35 条「c_start 变动」为我自身脚本比错字段的误报，
改正后清零——比较的是随 bar 延展的右端，而非左端）。
Force 寿命 min/median/max = **1 / 53 / 262** ✓ 与报告一致。

抽查链（`seg_a=(1394,1540) c_start=2178 b_center_start=1654`，与报告 §4.2 实例同一条）逐行核：
`Observed@2230` → `FirstProvable@2230` → `Supersedes@2231..2250`（21 条，**左端 2178 一根未动**，
右端逐 bar 追 as_of）→ `Invalidated{ForceOvertake}@2250`，`area_a=3398081382.117`、
`area_c=4087644792.474` 与报告逐位相同。

**p409 113/91.1% 未复现的归因 —— 成立。**
独立核 `/tmp/issue430-tri-p409-100k.jsonl`（248 行）：`ForceOvertake=113`、`IdentityVanished=20`、
截止仍 Provisional=115；Force 寿命 min/median/max = **9 / 378 / 20966**。
报告 §5 引用的「反事实延展寿命中位 378 vs 生产非闪现 79.5」逐值成立，机制论证
（p409 固定 `structure_completed=false` 后在完成之后继续延展，寿命远超生产活窗真实存活期）
在数据上站得住。

### 1.5 永禁清单（逐条 grep/读码）

| 永禁项 | 判定 | 证据锚 |
|---|---|---|
| completion 延迟到下一 bar/trigger | **未违反** | `p123:1391` 同 bar 构造 `phases`、`:1394` 同 bar `feed_lifecycle_bar`；`completion_signals` pre=post=248 |
| 同 bar completion 丢弃 | **未违反** | pre/post `completion_signals=248`、`completion_events=125453` 不变；零丢弃零新增 |
| p409 `structure_completed=false` 接生产 | **未违反** | `p123_fast_replay.rs` 无 p409 代码引用（仅 4 处注释提「与 p409 同构」）；`structure_completed` 全部使用点在 `nest_lifecycle.rs` 与其测试内 |
| `observed_at` 回填到 c_start / seg_c.end | **未违反** | P3 测试断言 `observed_at=95 ≠ c_start=70`（`:3198-3199`）；实测 156 只 earlier-live 的 observed_at 均为首次 advance bar |
| `as_of` ±1 凑数 | **未违反** | diff 内无 as_of 加减；三钟恒序 assert（`:1097-1102`）248/248 无违序 |
| 拿 `MoveBlock.status` 当 segment completion | **未违反** | `MoveBlock`/`MoveStatus` 在生产段零引用（仅 `nest_lifecycle.rs` 测试 mod 内 3 处） |
| `kind==Consolidation` 自动等价 Completed | **形式满足，实质见 S-5** | 已改为显式 typed `PanCompletionEvent`，携 `completed_lower_id`/`completed_at`；但 tower 查证恒真（§1.8） |
| 完成后继续延展 live | **未违反** | `extension_suppressed=12469`；F2/F3 测试族保留 |

### 1.6 字节护栏

**八对全部独立核实 cmp=0，八个 SHA 与 #421 自查档 §7.3 逐字相同**（票据只要求抽一条核；
我核了全部八条）：

| 面 | 窗 | cmp | SHA-256（前 16） | #421 自查档锚 |
|---|---|---:|---|---|
| p123 stdout | 20k | **0** | — | — |
| P116 dump | 20k | **0** | — | — |
| p123 stdout | 100k | **0** | `d8b69c180c23c5e3` | `issue421-acceptance-selfcheck-20260727.md:301` ✓ |
| P116 dump | 100k | **0** | `8a7327feb3b9ba29` | `:302` ✓ |
| m8 trades / tower_events | p3fold | **0/0** | `2da686833581d535` / `83f45a36ab422a92` | `:303` ✓ |
| m8 trades / tower_events | wf7 | **0/0** | `3371f1e62153e8aa` / `aa96b3e836be5a43` | `:304` ✓ |
| m8 trades / tower_events | wf8 | **0/0** | `006c31f54cd72d8e` / `1d8dff0d29e8925f` | `:305` ✓ |

**P-H3 不动 —— 成立。** pre/post stderr 逐字相同：
`provider_requests=2099 provider_reevals=87 provider_reuses=2012` ⟹ 2012/2099 = **95.855169%**。

其余 stderr 面 pre/post 全等（`P123_SPARSE_SUMMARY` 除 `prefix_s` 计时外逐字相同、
三条 `P123_SPARSE_LEVEL` 逐字相同），`completion_events=125453` 不变，
仅 `live_windows`（11026776→32356）与 `extension_suppressed`（11026528→12469）按预期变化。

### 1.7 测试门

**自跑复现（`CARGO_TARGET_DIR=/tmp/kimi-nest-target-check cargo test --lib`）：**

```
test result: FAILED. 2024 passed; 1 failed; 136 ignored; 0 measured
failures: theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard
  panicked at src/theta_v0/classifier/signal.rs:3418  （= #491）
```

与交付报告 §3 的 2024/1/136 **逐值相同**，唯一红确为 #491。

**污染注明（照实登记）：** 工作区含并行未提交面（11 个 `M` + 2 个未跟踪 `.rs`）。其中：
- in-diff 新增 `#[test]` **3 个**（`backtest/` 面）；
- 未跟踪 `rust/src/theta_v0/backtest/gamma_dump.rs` 含 **4 个** `#[test]`，且已由未提交的
  `backtest/mod.rs`（`+mod gamma_dump;`）纳入编译。

故 2024（post）与报告所称 2019（基线）**两个读数都建立在同一污染快照上**，
「差额 +5 = 本票新增 P1–P5」只在该快照未变的前提下成立。我复现出同样的 2024 ⟹ 前提在评审时刻成立。
但这条不是纯净基线对拍，应在结论中标明。

### 1.8 附：tower 查证的实质性（S-5，LOW）

`lifecycle_bar_phases`（`p123:1727-1731`）用 `find_move_by_end_index(tower[level-1], event.interval_b.1)`
「查证」lower unit，查不到即 `Err` 停线。但 pan 事件的 `interval_b = core.structure.seg_c`
（`level_view.rs:1349`），其右端**必然**是某个 lower leg 的 `end_index`——C 段本来就是从
lower legs 里取的。⟹ 该查证在生产上**恒真**，Err 分支不可达（100k 跑通即旁证）。

修复的实质价值在于**携带了 `completed_lower_id`/`completed_at` 两个新事实**（这确实是新信息，
且使完成钟 provenance 可测），不在于查证提供了新的完成筛选力。
commit message 与报告 §2.2 的「查不到即停线，消费方不再按 kind==Consolidation 猜 provenance」
措辞暗示了不存在的门控强度——**结构上，每条 Consolidation 事件仍被无条件转成 Completed**。

不判违反永禁 7：在当前 provider 架构下「所有 Consolidation 事件都是完成事件」是**事实**而非伪装
（C 段必须是 confirmed leg），#523 批评的是「缺 provenance 证明」而非「等价关系错」，本票补上了 provenance。
但措辞应收窄为「provenance 显式化」，不宜写成「查证/停线」。

---

## 2. Standards 轴

### T-1（MED）参数名与实参完全对调 — Fowler「Mysterious Name」

`p123_fast_replay.rs:1683-1687` 形参：

```rust
fn recompute_lifecycle_window_stems(
    tower, frontier, as_of,
    misses:   &mut BTreeMap<&'static str, usize>,   // ← 实收 l1_live_outcomes（含 window=608）
    outcomes: &mut Vec<(&'static str, Option<usize>)>,  // ← 实收 miss_rows
)
```

调用点 `:1102-1108` 传的是 `&mut lifecycle_stats.l1_live_outcomes, &mut miss_rows`。
函数体里 `*misses.entry(outcome.reason_tag())` 写的是**全部** outcome 分布（含成功的 `window`），
`outcomes.push(...)` 写的是**未命中**行。两个名字互换即正确。
不是行为 bug（stderr `P527_L1_LIVE_OUTCOMES ... window=608` 输出正确），但读码者会被直接误导。

### T-2（LOW-MED）诊断累加器 Data Clump + Primitive Obsession

两个 `&mut` 累加器成对穿过 5 参数签名（Data Clumps）；miss 行用裸元组
`(&'static str, Option<usize>)` 承载「原因码 + 中枢提示」（Primitive Obsession）。
应封装为一个具名 `L1LiveDiagnostics { tally, miss_rows }`——同时天然解决 T-1。

### T-3（LOW）删除标准在同一 diff 内不一致

本票以「无生产调用方」为由删除 `ReplayPrefixFeed`/`feed_replay_prefix`（no-patch-mentality
「无用代码直接删除」）。但 `provide_replay_live_windows`（`nest_lifecycle.rs:1494`）
同样**生产零调用方**——唯一调用方是测试夹具 `feed_prefix_phases`（`:2981`）——却被保留为 `pub`。
可辩护为「测试基础设施」，但应在函数头登记该定位，否则同一 diff 内两套标准。
（`provide_pan_live_windows` 有 p409 生产调用方 `p409_pan_live_probe.rs:161`，保留合法且已登记分水岭。）

### T-4（LOW）`for level in 1..2.min(tower.len())` 语义已退化

`p123:1687`。该循环最多执行一次（level=1）。保留循环外壳掩盖了「本函数只服务 L1」这一
已在文档中明写的事实。改为显式 `if tower.len() >= 2 { let level = 1usize; ... }` 更诚实。

### T-5（LOW）nest_lifecycle.rs 文件长度债务增量

当前 **4014 行**（coding-style 上限 800 的 5.0 倍），本 diff 净增 **448 行**（+12.6%，
生产段约 280 行 / 测试段约 170 行）。S-H2 豁免的 destination 是 #454，本票增量**未加剧到需要新债票**
（新增两个公开函数分别 ~45 / ~58 行，均在函数长度线附近而非远超；新增 `unwrap`/`expect`/`panic!` = **0**）。
建议在 #454 备注本次 +448 的增量来源，避免债务规模在 destination 上失真。

### 通过项（Standards）

- **fail-loud 错误处理合规**：`lifecycle_bar_phases` 的两处 `ok_or_else` 给出含 level/seg_c_end/as_of
  的可诊断消息并经 `?` 传播到 `main` 的 `Result`——符合仓内「不静默吞错」标准。
  （其恒真性见 §1.8，是「防御性但不可达」，不是「吞错」。）
- **`ActiveSegmentFrontier` 数据源纪律与 parser 口径一致**：极值扫描（逐笔取 start/end 两端、
  方向上取极值）与 `parser/tail.rs:72-87` 的 `current_extreme` 计算**逐步同构**；
  初值差异（`first.start_price` vs `first.start_price.max/min(first.end_price)`）被循环重扫 first 抵消，
  结果相同。`extreme_at` 平局保最早坐标，与 parser 同 discipline。
  定位失败三条路径（无 PendingSegment / 首笔坐标不匹配 / 极值退化在起点）全部诚实返回 `None`，不造窗。
- **新 API 面文档质量高**：`ActiveSegmentFrontier` / `active_segment_frontier` /
  `provide_l1_active_pan_live_windows` / `L1LiveOutcome` / `PanCompletionEvent` / `PanProviderPhase`
  均带数据源纪律、边界条件、级别有效域登记，注释密度与仓内风格一致。
  唯一例外是 `provide_l1_active_pan_live_windows` 文档中的 c_start 恒等断言（见 C2）。
- **负控测试到位**：P2 含两条负控（`FrontierNotAfterConfirmed` / `FrontierAheadOfClock`），
  P4 给出 true-flash 的三钟同值判据，P5 三值刻意互不相等——比单纯 happy-path 断言强。
- **测试覆盖缺口（LOW）**：P1 用手填的 `current_extreme` 做「与 parser 同口径」断言（夹具自洽），
  没有用真实 `ParseLayer` 输出交叉验证。若 parser 的极值口径将来改动，本实装会静默偏离
  而 P1 照绿。建议补一条以真实 parser 输出为输入的一致性测试。

---

## 3. 新发现汇总（分级 + 锚）

| ID | 级别 | 轴 | 发现 | 锚 |
|---|---|---|---|---|
| S-1 | MED | Spec | 46 只 L1 缺口归因表用错时间窗；按 C 段真实活跃期重算 **11 只无原因码落地**（含 7 只期间 provider 在为别的身份产窗），报告归为 `no_active_frontier` 的那 1 只实测正在产窗 | 报告 §4 表；`/tmp/wt527-post-100k.dump` `L1_LIVE_MISS`/`L1_LIVE_RECOMPUTE` 行；`p123:1687` |
| S-2 | MED | Spec | 代码文档的 c_start 恒等断言在 100k 有 **5 例 L1 反例**（`as_of=2047/65102/66720/94693/96059`），且与交付自身 §7 遗留 2 互相矛盾 | `nest_lifecycle.rs:1352-1357` vs 报告 §7 遗留 2；dump join |
| S-3 | MED | Spec | 「完成钟三分」在生产上只有二分——`observed_completion_at` 248/248 恒等于 `as_of`；#421 遗留 3 未真正收口 | `p123:1730`（`observed_completion_at: as_of` 硬写）；`CompletionSignal` doc `:72-76`；dump 实测 |
| S-4 | LOW-MED | Spec | #430 收口条件 1 的口径载体因本票删符号而失真，未同步注记 | `v3-lifecycle-rebuild-impl-20260724.md:6-7, 100, 111` |
| S-5 | LOW | Spec | tower 查证恒真，Err 分支生产不可达；措辞暗示了不存在的门控强度 | `level_view.rs:1349`；`p123:1727-1731` |
| T-1 | MED | Standards | `misses`/`outcomes` 参数名与实参完全对调 | `p123:1683-1687` vs `:1102-1108` |
| T-2 | LOW-MED | Standards | 诊断累加器 Data Clump + 裸元组 Primitive Obsession | 同上 |
| T-3 | LOW | Standards | `provide_replay_live_windows` 生产零调用方却保留，与同 diff 删除标准不一致 | `nest_lifecycle.rs:1494` / `:2981` |
| T-4 | LOW | Standards | `for level in 1..2.min(tower.len())` 循环外壳掩盖单级别事实 | `p123:1687` |
| T-5 | LOW | Standards | 文件 4014 行（上限 5.0×），本 diff 净增 448；建议在 #454 备注增量 | `nest_lifecycle.rs` |
| T-6 | LOW | Standards | P1 的「与 parser 同口径」断言是夹具自洽，缺真实 `ParseLayer` 交叉验证 | `nest_lifecycle.rs:3067-3112` |

---

## 4. IdentityVanished 裁量建议

**问题**：37 条 IdentityVanished 是与 #421 `IdentityVanished=0` 不变量的**语义冲突**，
还是活窗语义下的**合法新终局形态**？

**建议：合法新终局形态，但需编排者显式裁定吸收，且必须撤销 `IdentityVanished=0` 作为后续不变量。**

理由三条：

1. **`IdentityVanished=0` 从来不是独立不变量，是根因 bug 的推论。**
   #523 已钉死：修复前 `first_visible(PanLive) = completion_as_of` 是代码拓扑上的近似恒等式。
   活窗与完成同刻出生 ⟹ 身份从不跨 bar 存活 ⟹ 无从消失。所以 `=0` 是「首见即完成」这个 bug
   的副产品，不是设计保证。把它当不变量继承到 #527，等于要求修复后仍保留 bug 的可观测特征。

2. **教义层直接蕴含活窗身份可被取消。**
   R43 裁定「pan 行进中对象合法存在」（`chanlun/escalate/r43-lifecycle-ruling-20260721.md:31-39`），
   `parser/tail.rs` 模块头裁定「未完成走势只能唯一分类为当下状态，不能强行分类为最终结果」。
   两者合取 ⟹ 基于当下状态建立的活假设身份，必然可以被后续结构演化取消。
   禁止 IdentityVanished 就等于要求 provider 预判最终结果——那正是 `parser/tail.rs` 禁的东西。

3. **37 条全部是可审计终局，不是静默丢弃。**
   每条带显式 `Invalidated{reason: IdentityVanished}` revision、`as_of`、完整 key、
   可计算寿命（4/39/423 bar），`entry` 保留不删（谱系保留）。
   `assert_invariants` 逐条钉死「IdentityVanished 恒无力度证据」（`:1067-1076`），
   与 ForceOvertake / NeverConstituted 三者严格可区分。

**但吸收前必须附一个条件**（来自 S-2）：这 37 条至少混了两种性质不同的事件——

- **结构否证型**：活假设的结构前提被后续演化推翻（真正的「假设死了」）；
- **provider 窗口切换型**：假设本身没死，是 provider 换到了下一个 C
  （完成事件滞后中位 57.5 bar > C 段存活期，完成到达时活窗早已推进），
  其中至少 5 例表现为「同锚活窗 c_start ≠ 完成事件 c_start ⟹ 桥不判同 ⟹ 旧窗判消失 + 完成记闪现」。

后者不是假设失效，是观测接缝伪影——与 #523 钉的根因同类（只是从「首见即完成」变成「窗口错配」）。
把两类混记成同一个 `IdentityVanished` 计数，会让下一轮「活假设寿命/反超率」统计吃进伪影。

**建议裁定条款：**

- (a) **撤销** `IdentityVanished=0` 作为 #527 及后续票的验收不变量；改为
  「每条 IdentityVanished 必须带可审计终局记录 **且** 落到一个原因分类」。
- (b) 新开子项：把 `InvalidatedReason::IdentityVanished` 拆成两个可区分的原因码
  （结构否证 / provider 窗口切换），落到**账本字段**而非报告分析层——
  当前的分类只能靠 dump 侧信息事后重建（本评审即如此），不满足「可审计」的账本级要求。
- (c) 该子项挂在 #523 遗留问题 2（稳定身份）之下——它正是遗留 2 说的
  「如何以 ElementId/结构锚证明 live 与 completed 是同一身份，并退役仅忽略右端的工程桥」
  在真实数据上的第一批反例。

---

## 5. 结果包六要素

1. **结论**：#527 判 **PASS WITH CONDITIONS**，条件 C1–C5 见 §0。核心交付
   （L1 完成前活窗生产可见、完成事件 typed 化、字节护栏零回归）成立且逐项独立复现。

2. **定义依据**：
   - 票面 Acceptance 三项（`gh issue view 527`）与永禁清单七条为唯一尺子；
   - #421 E 裁定（comment-5106372198）第 1 条把「反超主导神谕」移出生产验收，
     故 35 ≠ 113 不作缺陷；第 4 条永禁清单逐条核（§1.5）；
   - #430 收口包裁定（comment-5106781616）第 1 条「trigger adapter 仅 legacy/测试兼容」
     的文档口径，因本票删符号而失真（C4）；
   - #523 §3 L1 伪代码的最小验收语义「要么存在 earlier Live，要么有明确、可审计的 true-flash 原因；
     禁 generic 事件自动等价 Completed / 完成后延展 live / 为寿命推迟 completed_at」——
     前两禁满足，第三禁满足；「明确可审计原因」在 11 只上未达成（C1）。

3. **边界条件（本结论在何时翻转）**：
   - 若 C1 的 11 只中出现「C 活跃期内 provider 本可定位却未产窗」的实例（即缺口可修而未修），
     判定应从 PASS WITH CONDITIONS 降为 FAIL——本评审只证明了归因错，未证明缺口不可修；
   - 若并行未提交面（11 M + 2 未跟踪 `.rs`，含 7 个 `#[test]`）在报告落笔与本评审之间发生过变化，
     则 §1.7 的「+5 = 本票新增」推论失效，需在纯净基线上重测；
   - 若后续把 `provide_l1_active_pan_live_windows` 改为对 L2/L3 也产窗且沿用已完成 tower unit，
     §1.2 的 Scope 合规判定翻转为违反 #523 根因（报告 §6.3 已自钉该条，本评审确认其成立）；
   - 若 S-2 的 5 例反例被证明源于 tower[0] 落后于 parser confirmed 段（段序列 gap）而非
     「同锚多候选 C」，则 `provide_l1_active_pan_live_windows` 缺少衔接性守卫
     （现只查 `active.start_index >= last.end_index`，允许跳过任意多段）将升级为 MED-HIGH 正确性缺陷。
     本评审的证据（`as_of=94693` 处 frontier.start=94590 > completed_at=94510）指向前者，
     但未排除后者——**照实登记为未判定**。

4. **下游推论**：
   - #421 收窄后的「闪现主导账本」描述对 L1 不再成立（L1 闪现率 46/202 = 22.8%，
     全域 92/284 = 32.39%）——引用 #421 读数的下游文档需按本票更新；
   - 新出现的 35 条 ForceOvertake 与 192 条非闪现寿命是 **L2 级证据**（单标的单窗真实数据），
     跨品种外推需另跑（#523 遗留 5）；本评审不改其等级；
   - `IdentityVanished` 计数在拆分原因码之前，**不得**直接进入任何寿命/反超率统计（§4 条件 b）；
   - `observed_completion_at` 字段在拆开 provider 事件产出与账本喂入之前，不得当作独立时点使用（C3）。

5. **谱系引用**：#523（PanLive provider 接缝缺口，根因）；#421 E 裁定 / 收口包裁定；
   R43（pan 行进中对象合法存在）；ADR-0003（结构完成 = 通道切换）；
   090（严格性 / 声明=能力——C2、C3、S-5 三条均是声明与能力不一致的实例）；
   `formalization-validity-domain`（认识论等级：本评审的读数复现全部为 L2，单标的单窗真实数据；
   §1.8 的恒真性论证为 L0 纯代码推导）；
   `no-patch-mentality`（T-3：同一 diff 内两套删除标准）。

6. **影响声明**：本评审为只读产出，**未修改仓内任何文件**，只新增本报告
   `chanlun/review-results/shadow-527-review-20260728.md`。
   未改 issue/map 状态，未关票，未改 roster。
   评审期间在 `/tmp/kimi-nest-target-check` 跑了一次 `cargo test --lib`（仅写该 target 目录），
   在 `/tmp/rv527/` 落了 5 个一次性解析脚本（仓外）。
   本报告的所有读数均为独立复现，未直接采信交付报告的任何数字。
