# #527 PanLive provider L1 交付：完成前活窗可见（BTC 100k，2026-07-28）

> 角色：executor（实装）
> 基线：`kimi-nest-mainline-20260717` @ `ff816d2ad3`
> 票据：#527（map #59 子票）；根因票 #523；上游裁定 #421 comment-5103071677 / comment-5106372198
> 口径：本报告不改教义、不关票、不改 map；provider 语义未遇教义层歧义，无上浮。

## 1. 结论

**L1 的「完成前活窗」已在生产真实可见。** BTC 100k 前缀首次出现跨 bar 的真实生命史：
248 只完成身份中 **156 只（全部 L1）在完成前即已被观察**（`observed_at < completion_as_of`，
提前量 min/median/max = **16 / 104.5 / 411** bar）；`Observed → FirstProvable → 逐 bar 延展 →
Invalidated{ForceOvertake}` 的完整链 **35 条**首次在生产出现（此前恒为 0）。
闪现率由 **248/248 = 100%** 降到 **92/284 = 32.39%**。

同时坐实 #523 的判词：**true-flash（同 bar 出生并完成）实测为 0**——248/248 的完成事件
首次可见 bar 都晚于 lower unit 物理完成 bar（滞后 min/median/max = 19 / 57.5 / 5190 bar）。

## 2. 修正要点（文件:行号，均为当前 HEAD 后新码）

### 2.1 `rust/src/theta_v0/classifier/nest_lifecycle.rs`

| 位置 | 改动 | 作用 |
|---|---|---|
| :1246-1289 `ActiveSegmentFrontier` | 新增 L1 行进中 C 段载体（方向/起点/起点价/极值/**极值结构点**） | 活窗 C 的唯一合法数据源；右端取极值所在笔端点，**禁 as_of 冒充结构点** |
| :1283-1330 `active_segment_frontier(&ParseLayer)` | 从 parser `tail` 的 `PendingSegment` + `strokes[pending_start..]` 读出 frontier | 与 `parser/tail.rs` 的 `current_extreme` 同口径；额外解析极值坐标（tail 只存值不存坐标） |
| :1332-1414 `provide_l1_active_pan_live_windows` | **confirmed A/B 锚 + active C frontier** 产窗 | #523 根因修复本体：C 只能是行进中段，禁 confirmed segments 回放重建 |
| :1416-1455 `L1LiveOutcome` | 定位结果 + **未产窗原因码** | 「某身份为何没有更早 Live」的可审计落点（诊断只写不判） |
| :1518-1548 `PanCompletionEvent` / `PanProviderPhase` | 完成事件成为显式 typed 输出（携 `completed_lower_id`/`completed_at`/`observed_completion_at`） | 消费方不再按 `kind == Consolidation` 猜 provenance |
| :1550-1559 `ReplayBarFeed` | `live_windows`+`completion_events` → `phases: &[PanProviderPhase]` | 两相由 provider 显式给出；出口内仍 live 相先、completion 相后 |
| :552-570 `CompletionSignal` | 加 `completed_lower_id` / `completed_at` / `observed_completion_at` | **完成钟三分**（物理完成 / 事件首见 / 账本收到）分列留档 |
| :1100-1109 `assert_invariants` | 新增 `completed_at ≤ observed_completion_at ≤ as_of` | 三钟恒序不变量（违序 = provider 回填/前视，停线） |
| 删除 `ReplayPrefixFeed` / `feed_replay_prefix` | legacy trigger adapter 无生产调用方 | no-patch：删除而非保留兼容垫片；测试改用本地 `feed_prefix_phases` 夹具 adapter |

`provide_pan_live_windows`（旧「从已完成 legs 重建」的产窗机）**保留但降级**：仅作结构定位参照与
p409 反事实探针口径，函数头登记它不再是生产「完成前活窗」的来源（:1178-1191）。

### 2.2 `rust/src/bin/p123_fast_replay.rs`

| 位置 | 改动 |
|---|---|
| :1044-1085 逐 bar 循环 | 活窗结构分量重算触发由「`forest_epoch` 变」扩为「`forest_epoch` 变 ∨ **active frontier 值变**」；落 `L1_LIVE_RECOMPUTE` / `L1_LIVE_MISS` 诊断行 |
| :1352 | 新增 `lifecycle_bar_phases(...)`：把完成事件包成 typed 完成相 |
| :1516-1590 `recompute_lifecycle_window_stems` | 只对 **L1** 产活窗、且只从 frontier 产；无 frontier ⟹ 空（不回落 confirmed 重建） |
| :1705-1745 `lifecycle_bar_phases` | 用 `find_move_by_end_index(tower[level-1], event.interval_b.1)` **查证** lower unit，取 `ElementId` 与物理完成 bar；**查不到即 `Err` 停线** |
| :1786-1800 `COMPLETION_SIGNAL` dump 行 | 增列 `lower_id` / `completed_at` / `observed_completion_at` |
| :679-688 stderr | 新增 `P527_L1_LIVE_OUTCOMES`（定位结果分布） |

## 3. 红绿记录

| 项 | 结果 |
|---|---|
| `cargo test --lib`（debug） | 2024 passed / **1 failed** / 136 ignored |
| `cargo test --release --lib` | 2024 passed / **1 failed** / 136 ignored |
| 唯一红 | `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`（#491） |
| 零新增失败证明 | 同一基线 worktree（`ff816d2ad3` + 他 session 未提交面，不含本票改动）实测 **2019 passed / 1 failed / 136 ignored**，失败集合逐项相同；差额 +5 = 本票新增测试 |
| 新增测试 | `p1_active_frontier_reads_parser_pending_tail_only`、`p2_active_frontier_live_window_matches_completed_identity`、`p3_live_before_completion_settles_same_identity_without_backfill`、`p4_completion_without_earlier_live_stays_flash`、`p5_completion_clock_three_points_are_distinguishable`（5 条全绿） |
| 既有 lifecycle 测试族 | T1–T19 / F1–F10 共 30 条全绿（改签名后逐条重跑） |

新增测试覆盖票面第 4 项要求的五面：active tail 产 Live（P1/P2）、Live→Completed 同身份（P3）、
零回填（P3：`observed_at=95 ≠ c_start=70`，完成不改写观察钟）、true-flash 路径（P4，附三钟同值判据）、
完成钟三时点区分（P5，三值 99/105/110 互不相等）。P2 另含两条负控：frontier 落在末 confirmed 段
内部 ⟹ `FrontierNotAfterConfirmed`；结构点未到当前 bar ⟹ `FrontierAheadOfClock`。

## 4. 四项验收逐条

### 验收 1：每个 Completed identity 要么有 earlier Live，要么有可审计的 true-flash 原因

BTC 100k 全量 join（248/248 有归因，无「不知道」）：

| 归因 | 只数 | 说明 |
|---|---:|---|
| A. 存在 earlier Live（`observed_at < completion_as_of`） | **156** | 全部 L1；提前量 16 / 104.5 / 411 bar |
| B. L1 无 Live — `structure_not_locatable` | 39 | C 活跃期窄锚与 A′ 都无法定位，或 Extreme 预滤未过（该假设当时**确实尚未成立**） |
| B. L1 无 Live — `center_not_consolidation` | 6 | C 活跃期最近已确认中枢当时不属 Consolidation 块 |
| B. L1 无 Live — `no_active_frontier` | 1 | C 活跃期 parser 无 `PendingSegment` |
| C. L2/L3 无 active lower-frontier | 46（L2=38 / L3=8） | 票面 Scope 外：`LeveledMove` 塔上无 Active/Completed 表达（#523 遗留 1） |

**true-flash 实测 = 0**：248/248 的 `as_of − completed_at ≥ 19`（min/median/max = 19 / 57.5 / 5190），
即没有任何一只是「同 bar 出生并完成」。B/C 两类是 provider 能力边界，**不冒充** true-flash——
本报告按票面要求把它们单列为已知缺口，不并入 true-flash 分子。

L1 覆盖：202 只 L1 身份中 156 只（**77.2%**）拿到完成前活窗。

### 验收 2：真实生命史首次在生产出现（读数照实，不预设 p409 的 113/91.1%）

| 指标（BTC 100k） | pre（`ff816d2ad3`） | post（本票） |
|---|---:|---:|
| entries | 248 | **285** |
| completion_signals | 248 | 248（**零丢弃、零新增**） |
| 闪现（零寿命）终局 | 248 | **92** |
| 非闪现终局 | 0 | **192** |
| 非闪现寿命 min/median/max | — | **1 / 79.5 / 423** bar |
| 闪现率（零寿命 / 全部终局） | **100.00%** | **32.39%** |
| ForceOvertake | 0 | **35** |
| ForceOvertake 寿命 min/median/max | — | **1 / 53 / 262** bar |
| 完整 Observed→FirstProvable→ForceOvertake 链 | 0 | **35** |
| Confirmed / NeverConstituted | 151 / 97 | 136 / 76 |
| IdentityVanished | 0 | **37** |
| Provisional（前缀截止仍存活） | 0 | 1 |
| retrograde_rejected | 0 | **0** |
| `as_of` 全局单调 | 是 | **是** |
| completion_force_unavailable | 0 | 0 |

生产 ForceOvertake 链实例（可查账，`/tmp/wt527-post-100k.dump`）：

```
level=1 side=Long seg_a=(1394,1540) c_start=2178 b_center_start=1654
  as_of=2230  Observed
  as_of=2230  FirstProvable
  as_of=2231..2250  Supersedes（活窗右端逐 bar 延展，左端不动）
  as_of=2250  Invalidated{ForceOvertake}
              evidence=ForceEvidence{ area_a=3.398e9, area_c=4.088e9, dif_peak_a=4.382e8, ... }
```

20k 前缀同向复现：entries 46→53、闪现 46→17、非闪现 0→36（4 / 89.5 / 423）、ForceOvertake 0→7
（5 / 68 / 129）、IdentityVanished 0→7、闪现率 100%→32.08%。

**IdentityVanished=37 ≠ 0（票面读数偏离，照实登记）**。它不是回归，而是活窗被引入后必然出现的
诚实终局——活假设在完成前被结构变更取消。逐条归因（100k）：

| 现场 | 只数 |
|---|---:|
| 同 bar 同 (level, side, seg_a, b_center) 出现**新 c_start**（8 更早 / 2 更晚）⟹ parser 段重划 / λ_C 改变 | 10 |
| 同 bar 有其他锚的新身份建仓 | 10 |
| 同 bar 无新建（frontier 被 confirm/重划后新 frontier 无法定位） | 17 |

vanished 前寿命 min/median/max = 4 / 39 / 423 bar。根源是 parser 「未完成走势只给当下状态、
不预判最终结果」（`parser/tail.rs` 裁定）：pending 段的当下起点未必等于它最终被划成的段起点，
故活窗身份可被重划取消。这正是 #523 遗留问题 2（稳定身份）的实测边界，见 §6。

### 验收 3：字节护栏

pre = 独立 worktree `/tmp/wt527-baseline`（`ff816d2ad3` + 本工位他 session 未提交面同步复制，
**唯一差异 = 本票两个文件**），post = 本工位。

| 面 | 窗 | cmp | SHA-256（前 16） |
|---|---|---:|---|
| p123 stdout | 20k | **0** | — |
| P116 dump | 20k | **0** | — |
| p123 stdout | 100k | **0** | `d8b69c180c23c5e3`（与 #421 自查档 §7.3 逐字相同） |
| P116 dump | 100k | **0** | `8a7327feb3b9ba29`（同上） |
| m8 trades / tower_events | p3fold | **0 / 0** | `2da686833581d535` / `83f45a36ab422a92` |
| m8 trades / tower_events | wf7 | **0 / 0** | `3371f1e62153e8aa` / `aa96b3e836be5a43` |
| m8 trades / tower_events | wf8 | **0 / 0** | `006c31f54cd72d8e` / `1d8dff0d29e8925f` |

六个 m8 SHA 与 #421 自查档 §7.3 记录逐字相同（跨票一致）。

P-H3：pre/post 均为 `provider_requests=2099 provider_reevals=87 provider_reuses=2012`，
复用率 **95.855169%** 不动。

结构佐证：`nest_lifecycle` 的消费方只有 `p123_fast_replay` 与 `p409_pan_live_probe` 两个 bin
（全仓 grep 实证），backtest/m8 路径不引用它。

lifecycle dump 是本票本体，不作 cmp=0，前后对照见验收 2。post 产物 SHA-256：
20k `9b365a9f4dc7c6c8…`、100k `c113e47a4ed4871c…`。

### 验收 4：测试门

见 §3。全库 debug/release 唯一红 #491，零新增失败；新增 5 条单测覆盖票面点名的五面。

## 5. 与 #523 报告 202/248 覆盖预期的偏差归因

| 项 | #523 预期 | 实测 | 归因 |
|---|---|---|---|
| L1 身份总数 | 202 | **202** | 完全吻合（L2=38 / L3=8 亦逐值吻合） |
| L1 拿到完成前 Live | 「L1 prototype 可覆盖 202/248 的身份」 | **156/202** | 预期是**身份可达性**（这些身份属 L1 域），不是「每只都必有活窗」。46 只缺口全部落到可审计原因码：39 `structure_not_locatable`、6 `center_not_consolidation`、1 `no_active_frontier`。前两者是**结构事实**——C 活跃期该盘整背驰的前提（Extreme 破 A 极值 / 中枢归属 Consolidation 块）尚未成立，此时不产活窗是诚实的，造窗才是伪修。 |
| 完成信号总数 | 248 | **248** | 零丢弃、零新增；completion 时刻一根 bar 未延迟（三钟恒序不变量在 release 亦执行） |
| p409 的 113 / 91.1% | 明确不作 oracle | 生产 35 条 ForceOvertake | 照票面「不预设复现」登记。差距的机制原因：p409 固定 `structure_completed=false` 后在**完成之后**继续延展（post-completion counterfactual），其 Force 寿命中位 378 bar 远长于生产活窗的真实存活期（本票非闪现寿命中位 79.5 bar）；生产在真实完成点结算，故反超只能发生在完成前的窗内。 |

## 6. 结果包六要素

1. **结论**：见 §1、§4。L1 完成前活窗已生产化；完成事件成为显式 typed 输出并携完成钟三分。
2. **定义依据**：
   - 行进中 C 的合法载体 = `Origin.ChanlunElements.OpenTail.pendingSegment`（`parser/tail.rs`
     模块头逐字：「未完成走势只能唯一分类为当下状态，不能强行分类为最终结果」）；本票只读
     其方向/起点/当前极值三项当下状态，不预判终结。
   - 盘整背驰 A/C 结构判据全部复用单一来源：`locate_pan_div_structure`（窄锚，049:36-38 最标准锚优先）
     → `locate_pan_div_structure_front_anchor`（A′ 回退，061:28 中枢两头比较）→
     `pan_div_structure_extreme`（037:20 破极值预滤）；λ_C 经 `departure_move_c_start`
     （Q5 + codex ac4 #2 回中枢定界）——与完成事件 provider（`level_view.rs` pan 分支）同序同判，禁第二查法。
   - 活假设结算语义不动：061:26（被反超要求曾构成）/ 061:28（从未构成）/ 024:24（完成时复核）
     与 R43 裁定（`chanlun/escalate/r43-lifecycle-ruling-20260721.md:31-39`，pan 行进中对象合法存在）。
   - 完成证明：lower unit 在 `tower[level-1]` 上按 `end_index` 查证存在，取其 `ElementId`
     （`recursive_tower.rs` 定义：跨 bar 稳定的确定性身份）与物理完成 bar。
3. **边界条件（结论在何时翻转）**：
   - 若 parser 的 `tail` 不再输出 `PendingSegment`（或 `strokes` 与 `tail.start_index` 坐标不再一致），
     `active_segment_frontier` 恒 `None` ⟹ L1 活窗归零，退回首见即完成；
   - 若 λ_C 的定界改为依赖行进中段右端（现只依赖左端与方向），活窗与完成事件的 `seg_c.0`
     不再恒等 ⟹ 桥判不同身份 ⟹ 覆盖率坍塌为 0 而测试仍绿（P2 的桥断言即为此设的锁）；
   - 若 `bridge_identity` 改为比较 `seg_c` 右端，全部活窗身份会退化成「身份消失 + 新建仓」；
   - 若给 L2/L3 用「已完成 tower unit」外推行进中窗，会重演 #523 的同源恒等式（本票明确拒绝）。
4. **下游推论**：
   - #421 的原 destination（真实活假设生命史）在 **L1 域**已有输入来源；其收窄后的「闪现主导账本」
     描述对 L1 不再成立（L1 闪现率 32.39%），复审引用需按本票读数更新；
   - p409 的 91.1% 保持为 post-completion counterfactual，本票不改其定位；
   - 消费侧仍 Closed-only（`consumable_closed` 只放 Confirmed），N^δ 装配/证书真值路径未接入本 book——
     本票不改这条边界；
   - 新出现的 35 条 ForceOvertake 与 192 条非闪现寿命，使「活假设寿命/反超率」首次成为可统计的生产量，
     但它是 **L2 级证据（单标的单窗真实数据）**，跨品种外推需另跑（#523 遗留 5）。
5. **谱系引用**：#523 根因（PanLive provider 接缝缺口）；#421 逃生门裁定（comment-5103071677）
   与选项 E 裁定（comment-5106372198）；R43（pan 行进中对象合法）；ADR-0003（结构完成 = 通道切换）；
   090（声明=能力：本票把「活窗」从声明变成能力，并把做不到的 L2/L3 明写为缺口）；
   `formalization-validity-domain`（认识论等级：产窗判据 L0 结构、覆盖率读数 L2 真实数据单窗）。
6. **影响声明**：改动 2 个生产文件（`nest_lifecycle.rs`、`p123_fast_replay.rs`）。
   受影响面 = lifecycle sidecar（账本、dump、stderr 审计）；**未改** p123 stdout / P116 dump /
   m8 trades / tower_events / 装配 / 证书 / `Cargo.toml` / #454 / #497 / 他 session 未提交面。
   `ReplayPrefixFeed`/`feed_replay_prefix` 删除（无生产调用方）。

## 7. 遗留（不在本票 Scope，照实登记）

1. **L2/L3 的 active lower-frontier**（#523 遗留 1）：46 只身份（L2=38 / L3=8）仍首见即完成。
   需先定义递归塔的行进中单元载体（只读 frontier view 或独立 active sidecar），不得从已完成
   tower unit 外推。
2. **身份稳定性的真实边界**（#523 遗留 2）：`IdentityVanished=37`，其中 10 只可直接观察到
   同 bar 同锚新 `c_start`。工程桥（除右端外全等）能吸收活窗延展，但吸收不了 parser 末段重划导致的
   左端迁移。是否为「pending 段重划」定义一条受控的身份继承规则，属教义/规格问题，**本票不替裁**。
3. **完成 Event 首次可见 bar 的滞后**：248/248 的 `observed_completion_at − completed_at ≥ 19`
   （median 57.5、max 5190）。这是 provider 结构可见性（中枢/投影/run 缓存）的滞后，不是账本问题；
   本票已把三钟分列使其可测，但**未**缩短它。
4. **L1 46 只缺口的可修性**：39 只落在 `structure_not_locatable`。是否有一部分源于「Extreme 预滤
   要求 C 已破 A 极值」这一门槛（即活假设按定义此时尚不成立），还是有可提前的定位路径，需要
   逐案审读后再定，本票不猜。
5. **跨品种外推**（#523 遗留 5）：248 / 156 / 35 / 32.39% 全部是 BTC 100k 单窗读数，不得写成规格常量。

## 8. 复现命令

```bash
cd rust
cargo build --release --bin p123_fast_replay
P116_MAX_BARS=100000 P116_CKPT=0 \
  P116_DUMP=/tmp/wt527-post-100k.p116 \
  P421_LIFECYCLE_DUMP=/tmp/wt527-post-100k.dump \
  ./target/release/p123_fast_replay ../analysis/data_cache/btc_1m_full.json \
  > /tmp/wt527-post-100k.stdout 2> /tmp/wt527-post-100k.stderr

# m8 三窗（护栏面）
for w in p3fold wf7 wf8; do
  M8_WIN_FILTER=$w OPSEM_DUMP_DIR=/tmp/wt527-post-m8-$w \
    cargo test --release --lib m8_e2e_all_systems_oos -- --ignored --nocapture
done
```

产物：`/tmp/wt527-{pre,post}-{20k,100k}.{stdout,p116,dump}`、`/tmp/wt527-{pre,post}-m8-{p3fold,wf7,wf8}/`。
