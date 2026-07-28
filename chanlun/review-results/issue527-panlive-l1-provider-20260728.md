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

---

## 9. #559 修复节（2026-07-28）

> 依据：影子评审 `chanlun/review-results/shadow-527-review-20260728.md`（PASS WITH CONDITIONS，
> 条件 C1–C5 + IdentityVanished 裁量建议）与编排者裁定 `gh issue view 527` comment-5108273992。
> 本节只登记修复轮，不改写 §1–§8 原文（不静默改写历史）。
> 产物前缀 `/tmp/wt527fix-{pre,post}-*`；pre = HEAD `558104a0ff` 的 `git archive` 净出，
> post = 本工位（唯一差异 = 本修复轮三个文件）。

### 9.1 裁定项：IdentityVanished 原因码拆两类落账本字段

**旧不变量 `IdentityVanished = 0` 已撤销**（裁定 1）。新不变量：*凡消失的身份必须带可审计
原因码，且成因拆两类落账本字段*。

| 位置 | 改动 |
|---|---|
| `nest_lifecycle.rs:225` | 新增 `pub enum VanishCause { HypothesisRefuted, ObservationSeam { successor_c_start } }` |
| `nest_lifecycle.rs:248` | `InvalidatedReason::IdentityVanished` → `IdentityVanished { cause: VanishCause }`（原因码载荷，入 revision 留档） |
| `nest_lifecycle.rs:355` | `NestLifecycleEntry` **新增账本字段** `vanish_cause: Option<VanishCause>` |
| `nest_lifecycle.rs:405` | 唯一写入点 `invalidate()` 由 `reason` 现场投影 `vanish_cause`（两字段结构上不可能不一致） |
| `nest_lifecycle.rs:181` | 新增 `same_anchor()`（身份键去 `seg_c_full` 的五元相等；`bridge_identity ⟹ same_anchor`） |
| `nest_lifecycle.rs:1048-1058` | `advance` 第 8 步 = **唯一分类点**：本 prefix 观察集合 `seen` 中若仍有同锚身份（必然 c 左端不同——同左端已被桥吸收成 `Supersedes`）⟹ `ObservationSeam{successor_c_start}`；否则 `HypothesisRefuted` |
| `nest_lifecycle.rs:1103-1112` | 新不变量全态钉死：`vanish_cause.is_some() ⟺ 原因码是 IdentityVanished` |
| `nest_lifecycle.rs:1168-1190` | 消失臂内追加：账本字段与原因码载荷逐位一致；接缝成因的 `successor_c_start ≠ 消失身份 c 左端` |
| `nest_lifecycle.rs:680-684` | `LifecycleSettlementStats.identity_vanished_count` → `identity_vanished_refuted_count` / `identity_vanished_seam_count`（**分列，禁合并**） |
| `nest_lifecycle.rs:66-77` | 模块头登记新不变量 + 撤销旧口径的理由（挂 #523 遗留问题 2） |
| `p123_fast_replay.rs:720 / :1495` | stderr `P421_LIFETIME_SUMMARY` 与 dump `LIFETIME` 行两类分列 |
| `p409_pan_live_probe.rs:219,471-472,514` | 反事实探针同步分列（禁混记口径一致） |

判据为什么落在「同锚」上：`bridge_identity` 已吸收「只有右端不同」的延展，故凡进入消失扫描
的身份，其同锚新键必然是**换了 C 左端**——那正是「provider 换轨」的可观测形态；反之同锚在本
prefix 完全不产窗，才是「该假设的结构前提被后续演化推翻」。判据只读本 prefix 的 `seen`，
不做历史重建（评审版分类靠 dump 侧事后重建，不满足账本级可审计）。

**新增测试**：`t6b_identity_vanish_observation_seam`（(b) 类：同锚换 C，断言成因 + 接手 c 左端
入账本字段、两类计数不互相污染）；`t6_identity_vanish_on_structure_switch` 改判为 (a) 类
（`seg_a` 变 ⟹ 锚已变）并加两类计数断言。

### 9.2 C1（MED）归因窗修正 + 11 只无原因码落地修掉

归因窗从 `[c_start, 完成 as_of]`（含完成事件滞后期，中位 57.5 bar）改为 **C 段真实活跃期
`[c_start, completed_at]`**。为让每只身份都能落码，补记 **命中行**：原实装只落未命中行
（`L1_LIVE_MISS`），于是「该 run 定位成功、但定位到的是别的 C」在 dump 上无痕。

| 位置 | 改动 |
|---|---|
| `p123_fast_replay.rs:532` | 新增具名诊断行 `L1LiveDiagRow { reason, b_center_start, c_start }`（替代裸元组，兼解 #559 T-2 Primitive Obsession） |
| `p123_fast_replay.rs:1596-1597` | `recompute_lifecycle_window_stems` 逐 run 落一行（命中/未命中都记） |
| `p123_fast_replay.rs:1108` | dump 分 `L1_LIVE_HIT` / `L1_LIVE_MISS` 两种行，均带 `b_center_start` 与 `c_start` |

**BTC 100k 全量归因表（248/248 有码，无「不知道」）**

| 归因 | 报告 §4 旧窗 | #559 新窗（C 段真实活跃期） |
|---|---:|---:|
| A. 存在 earlier Live | 156 | **156**（提前量 16 / 104.5 / 411 bar，不变） |
| B. L1 无 Live — `structure_not_locatable`（同锚） | 39（未分同锚） | **8** |
| B. L1 无 Live — `structure_not_locatable`（活跃期内只在别的锚上未命中） | — | **21** |
| B. L1 无 Live — `center_not_consolidation`（同上） | 6 | **6** |
| B. L1 无 Live — `no_active_frontier` | 1 | **0**（该归因被推翻，见下） |
| B. L1 无 Live — `located_other_c`（**同锚**上定位到别的 C） | — | **2** |
| B. L1 无 Live — `located_other_center`（活跃期内只在别的锚上产窗） | — | **4** |
| B. L1 无 Live — `no_recompute_in_span`（活跃期内结构分量未变 ⟹ 未重算 ⟹ 从未被扫描） | — | **5** |
| 区间内无任何原因码行 | 0（旧窗口径下） | **0**（影子评审按新窗算得 11，现全部落码） |
| C. L2/L3 无 active lower-frontier | 46（38/8） | **46**（38/8，票面 Scope 外，不变） |

小计：156 + (8+21+6+0+2+4+5 = 46) + 46 = **248** ✓。影子评审用新窗独立算出的
`structure_not_locatable=29`（= 8+21）、`center_not_consolidation=6`、`no_active_frontier=0`
逐值复现；其点名的 11 只现分解为 `located_other_c` 2 + `located_other_center` 4 +
`no_recompute_in_span` 5。

**`as_of=94693` 的反向归因已修掉**：原报告归为 `no_active_frontier`（「C 活跃期 parser 无
PendingSegment」），而实测该身份的 C 活跃期是 `[94447, 94510]`，区间内 **recompute 数 = 0**
——即 provider 结构分量在该活跃期内未变、根本没重算，不是「无 frontier」。评审指出的
「完成时刻 frontier 正在产窗」是**完成 as_of=94693 时刻**的事实，与该身份 C 活跃期无关；
两者都指向同一结论：旧归因用错了时间窗。`no_recompute_in_span` 全部 5 只逐条：

```
as_of=1693  span=(1334,1352)  recomputes_in_span=0
as_of=33862 span=(33670,33799) recomputes_in_span=0
as_of=64500 span=(64374,64459) recomputes_in_span=0
as_of=86879 span=(86812,86828) recomputes_in_span=0
as_of=94693 span=(94447,94510) recomputes_in_span=0
```

与影子评审 §1.2 列出的 5 只「区间内 recompute 0/0」逐值相同。

### 9.3 C2（MED）恒等断言限定 + 5 例反例钉因

**钉因结论：前者——「同锚多候选 C」，不是 tower 段序列 gap。** 两路证据：

1. **段账本完整性直测（新增 `P559_SEG_LEDGER`）**：每次活窗重算比对
   `tower[0]` 单元数与 parser 段账本 `l0.segments` 长度。BTC 100k
   **`complete=1836 short=0 no_tower=433`**（20k 同为 `short=0`）——凡塔已构造，
   两者恒等长，**从未缺段**。故「tower[0] 落后于 parser confirmed 段」被否证。
   （`no_tower` = bootstrap 期塔尚未构造，此时 L1 活窗循环本就不执行。）
2. **5 例反例逐条现场**（`/tmp/wt527fix-post-100k.dump`）：

| as_of | 完成事件 c_start | 活窗 c_start | 同锚？ | 账本成因 |
|---:|---:|---:|---|---|
| 2047 | 1839 | 1869 | 是（`seg_a=(1334,1352)`, `b=1352`） | `ObservationSeam{successor_c_start: 1839}` |
| 65102 | 65017 | 65026 | 是（`seg_a=(64847,64914)`, `b=64224`） | `ObservationSeam{successor_c_start: 65017}` |
| 66720 | 66433 | 66467 | 是（`seg_a=(66205,66348)`, `b=65627`） | `ObservationSeam{successor_c_start: 66433}` |
| 94693 | 94447 | 94590 | 是（`seg_a=(93934,93956)`, `b=93994`） | `ObservationSeam{successor_c_start: 94447}` |
| 96059 | 95826 | 95655 | 是（`seg_a=(95060,95137)`, `b=95283`） | 该活窗早在 `as_of=95822` 即 `HypothesisRefuted`；96059 的完成属**同锚的另一个 C**，非同 bar 接缝 |

五例全部是**同一锚上先后存在多个候选 C 段**：完成事件首见滞后（中位 57.5、最大 5190 bar）
超过 C 段活窗存活期时，到账的完成事件指向的 C 与当下活窗的 C 不是同一段，桥自然不判同。

**修复动作**：`nest_lifecycle.rs:1460-1477` 的 c_start 恒等断言由无限定表述收窄为
「**同一 C 段**上恒等 / 跨 C 段不恒等」，并显式写出反例来源与两类成因的账本落点。

**衔接守卫未按正确性缺陷修正的理由（照实登记，`nest_lifecycle.rs:1491-1503`）**：
本轮曾实装严格衔接守卫（`active.start_index != last.end_index ⟹ 拒绝`）并回退。回退依据：
(a) 生产上洞结构上不可能——`tower[0] = l0.segments` 全量纯函数（`classifier/mod.rs:1598-1607`），
parser 发射段用 `end_stroke=k-1`、下一段 `seg_start=k`（`parser/segment.rs:415-425`），
`P559_SEG_LEDGER short=0` 实测印证；(b) 仓内 pan 合成夹具 `pan_real_fixture` 用「段间 +1
不共端点」坐标约定（(50,59)/(60,69)/…），与生产共端点约定不同，`==` 守卫在该夹具上恒拒，
会把一条生产不可达的守卫做成只能靠改夹具才可测的死分支。**「行进中段起点 − 末段终点 > 0」
不是缺段判据**——笔构造在 `gap_ok` 不足时 `i += 2` 跳过分型（`parser/stroke.rs:101-104`），
相邻笔在源坐标上本就可不相接（20k 实测该差值分布 0/2/3/…/43 皆有）。夹具坐标约定与生产
不一致本身登记为遗留（见 §9.7）。

### 9.4 C3（MED）完成钟三分 → 两分

**选择：简化为两钟**（`completed_at` 物理完成 / `as_of` 账本收到），删除
`observed_completion_at`。理由：现行接线下 provider 相构造（`lifecycle_bar_phases`）与账本
喂入（`feed_lifecycle_bar`）在同一 bar、同一调用链内完成，该钟被硬写为 `as_of`，
BTC 100k **248/248 恒等** ⟹ 不是独立可测时点，只是 `as_of` 的别名。保留一个恒等于另一字段
的钟 = 声明代码不具备的分辨力（090 声明 = 能力）。完成可见性滞后读数口径不变
（= `as_of − completed_at`，19 / 57.5 / 5190）。若将来 provider 相构造与账本喂入解耦，
第三钟才成为真实时点，届时按真实产出重新引入——**不预留空字段**。

改动点：`CompletionSignal`（`nest_lifecycle.rs:612-637`）、`PanCompletionEvent`、
`register_completion_signal`、三钟恒序不变量 → 两钟恒序、`p123_fast_replay.rs:1778`
函数头与 `COMPLETION_SIGNAL` dump 列。测试
`p5_completion_clock_three_points_are_distinguishable` →
`p5_completion_clock_two_points_are_distinguishable`（`:4132`），并在测试文档里登记
原断言是夹具自洽而非生产分辨力；P4 的 true-flash 判据同步由三值同刻收窄为两值同刻。

### 9.5 C4（LOW-MED）#430 口径载体与自查档同步订正

- `chanlun/review-results/v3-lifecycle-rebuild-impl-20260724.md`：头部追加「再订正
  （2026-07-28 晚）」，声明 `ReplayPrefixFeed`/`feed_replay_prefix` 已由 #527 整体删除；
  §6 第 2 条与 §7 第 1 条就地加订正夹注。
- `chanlun/review-results/issue421-acceptance-selfcheck-20260727.md`：新增 §10.3
  「#527/#559 后的口径失效登记」——`IdentityVanished=0` 撤销、legacy adapter 口径失效、
  完成钟三分收窄为两分。
- `nest_lifecycle.rs` T8 契约尾句（原「只在该兼容路径，跳过非 trigger prefix 才会晚记」）
  按实际能力收窄：夹具 `feed_prefix_phases` 按给定 as_of 直喂，**不存在**该路径。

### 9.6 C5（MED）参数名符实

`recompute_lifecycle_window_stems`（`p123_fast_replay.rs:1596-1597`）：
`misses: &mut BTreeMap<..>` → `outcome_tally`（实收全部 outcome 分布，含 `window`）；
`outcomes: &mut Vec<..>` → `diag_rows`（实收逐 run 诊断行）。函数体与调用点同步。

### 9.7 验收读数

**测试门**（`cargo test --lib`，`CARGO_TARGET_DIR=/tmp/wt527fix-target`）：

```
test result: FAILED. 2025 passed; 1 failed; 136 ignored
唯一红：theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard（#491）
```

基线 2024 → 2025，差额 +1 = 本轮新增 `t6b_identity_vanish_observation_seam`。
本票两个 bin（`p123_fast_replay` / `p409_pan_live_probe`）`cargo build` 干净；
`cargo build --bins` 收尾复核 **0 error**。
**照实登记**：测试门跑动时 `cargo build --bins` 一度红，唯一失因是他 session 当时未跟踪的
`rust/src/bin/issue570_ledger_probe.rs`（`use newchan_rust::theta_v0::backtest` 不存在），
与本轮改动无关、未触碰；该文件随后由该 session 自行移除，收尾复核已绿。

**字节护栏（十对，全部 cmp=0；pre = HEAD `558104a0ff` 净出）**：

| 面 | 窗 | cmp | SHA-256（前 16） | 跨票核对 |
|---|---|---:|---|---|
| p123 stdout | 20k | **0** | `bd9ac1d655f9d615` | — |
| P116 dump | 20k | **0** | `fcc8016a9a01a109` | — |
| p123 stdout | 100k | **0** | `d8b69c180c23c5e3` | = #421 自查档 §7.3 ✓ |
| P116 dump | 100k | **0** | `8a7327feb3b9ba29` | = #421 自查档 §7.3 ✓ |
| m8 trades / tower_events | p3fold | **0/0** | `2da686833581d535` / `83f45a36ab422a92` | = §4 验收 3 ✓ |
| m8 trades / tower_events | wf7 | **0/0** | `3371f1e62153e8aa` / `aa96b3e836be5a43` | = §4 验收 3 ✓ |
| m8 trades / tower_events | wf8 | **0/0** | `006c31f54cd72d8e` / `1d8dff0d29e8925f` | = §4 验收 3 ✓ |

P-H3 复用率不动：pre/post 均 `provider_requests=2099 provider_reevals=87 provider_reuses=2012`
⟹ **95.855169%**。

**lifecycle dump 前后对照（本票本体面，允许变）**：

| 行类 | pre | post | 说明 |
|---|---:|---:|---|
| `FEED` | 100000 | 100000 | **逐行逐字相同** |
| `REV` | 20789 | 20789 | 把 `IdentityVanished{cause:…}` 归一回 `IdentityVanished` 后**逐行逐字相同** |
| `COMPLETION_SIGNAL` | 248 | 248 | 去掉已删的 `observed_completion_at` 列后**逐行逐字相同** |
| `L1_LIVE_MISS` | 1261 | 1261 | 去掉新增 `c_start` 列后**逐行逐字相同** |
| `L1_LIVE_HIT` | — | 608 | 本轮新增（命中行，= `P527_L1_LIVE_OUTCOMES window=608`） |
| `L1_LIVE_RECOMPUTE` | 2269 | 2269 | 新增 `confirmed_last_end` / `tower0_units` / `l0_segments` 三列 |
| `LIFETIME` | 1 | 1 | `identity_vanished=37` → `identity_vanished_refuted=25 identity_vanished_seam=12` |

即：本轮对账本行为**零改动**，改的是原因码类型、账本字段、诊断面与文档口径。

**BTC 100k 两类分列新读数**：

| 指标 | pre（#527） | post（本轮） |
|---|---:|---:|
| entries / first_provable / provisional | 285 / 194 / 1 | **285 / 194 / 1**（不变） |
| Confirmed / NeverConstituted | 136 / 76 | **136 / 76**（不变） |
| ForceOvertake（链数） | 35 | **35**（不变） |
| ForceOvertake 寿命 min/median/max | 1 / 53 / 262 | **1 / 53 / 262**（不变） |
| IdentityVanished 合计 | 37 | **37**（不变） |
| ├ (a) `HypothesisRefuted` 假设被推翻 | — | **25**（寿命 4 / 44 / 127） |
| └ (b) `ObservationSeam` 观测接缝伪影 | — | **12**（寿命 4 / 31 / 423） |
| 闪现（零寿命）终局 / 非闪现 | 92 / 192 | **92 / 192**（不变） |
| 非闪现寿命 min/median/max | 1 / 79.5 / 423 | **1 / 79.5 / 423**（不变） |
| 闪现率 | 32.39% | **32.39%**（不变） |
| `retrograde_rejected` / `completion_force_unavailable` | 0 / 0 | **0 / 0** |

20k 同向：`entries=53`、`identity_vanished_refuted=3` / `identity_vanished_seam=4`（合计 7 = pre 7）、
闪现 17 / 非闪现 36（4 / 89.5 / 423）、ForceOvertake 7（5 / 68 / 129）。

**(b) 类逐条**（`as_of / 旧 c 左端 → 接手 c 左端`）：
`2047/1869→1839`、`3520/3423→3223`、`4378/3484→4184`、`6230/6146→5967`、`23184/22895→23083`、
`52685/52545→52281`、`60687/60536→60489`、`65102/65026→65017`、`66720/66467→66433`、
`69471/69415→69347`、`85813/85777→85727`、`94693/94590→94447`。
其中 10 只影子评审已用 dump 侧重建独立列出（`§1.1(b)` 的 10 条逐值相同）；账本级判据
另多识别 2 只（`3520`、`52685`）——评审的重建只找「同 bar 同锚**新建仓**」，账本判据看
本 prefix **全部**观察键（含完成相键），故覆盖更全。

**下游口径提示**：`ObservationSeam` 的 12 只**不是**假设失效，其寿命（4/31/423）不得进入
「活假设寿命 / 反超率」统计（裁定条款 2）。剔除后可用于寿命统计的消失身份 = 25 只
（4/44/127）。

### 9.8 遗留（本轮不做，照实登记）

1. **合成夹具坐标约定与生产不一致**：`pan_real_fixture` 的段用「段间 +1 不共端点」
   （(50,59)/(60,69)/…），生产段共享端点。后果：任何以段衔接为前提的守卫/断言都无法用该
   夹具验证（本轮衔接守卫回退的直接原因，§9.3）。属测试基础设施债，不在 #527/#559 Scope。
2. **`no_recompute_in_span` 5 只是否可修**：活窗重算触发口径为
   「`forest_epoch` 变 ∨ active frontier 值变」；这 5 只的 C 活跃期内两者都没变，故从未被
   扫描。是否应把「C 段候选集变化」也并入触发条件，属 provider 触发口径设计问题，本票不替裁
   （与 §7 遗留 4「L1 缺口可修性」同源）。
3. **`located_other_c` / `located_other_center` 6 只**：provider 每个 run 每次重算最多产
   1 个窗（`outcome.window()` 是 `Option`），同锚上并存多个候选 C 时只有一个能出生。
   是否应产出全部候选，属 provider 产窗基数设计问题，本票不替裁。
4. §7 原有 5 条遗留全部维持（L2/L3 active lower-frontier、身份稳定性、完成事件可见性滞后、
   L1 46 只缺口可修性、跨品种外推）。其中遗留 2 已由本轮的两类成因账本字段部分收口——
   「是否为 pending 段重划定义受控身份继承规则」仍是教义/规格问题，本票不替裁。

### 9.9 结果包六要素（修复轮）

1. **结论**：#559 五条件 C1–C5 与 IdentityVanished 裁定全部落地。账本行为零改动
   （FEED/REV/COMPLETION_SIGNAL/L1_LIVE_MISS 归一化后逐行逐字相同），字节护栏十对 cmp=0，
   测试门唯一红仍为 #491。C2 钉因结论 = **同锚多候选 C**（段账本完整性 1836/1836，`short=0`）。
2. **定义依据**：编排者裁定 comment-5108273992（撤销 `IdentityVanished=0`、两类原因码落账本
   字段、五条件）；`parser/tail.rs` 模块头「未完成走势只能唯一分类为当下状态」（(a) 类成因的
   教义依据）；R43（pan 行进中对象合法存在）；090（声明 = 能力，C3 删第三钟的依据）；
   `bridge_identity`/`same_anchor` 的严格强弱关系（两类判据互斥且穷尽的结构依据）。
3. **边界条件（结论在何时翻转）**：
   - 若 provider 改为同一 prefix 内对同锚产出多个候选 C 的窗，则「同锚仍有窗 ⟹ 假设未死」
     的判据失效——彼时 (b) 类可能吸收掉本应判 (a) 的身份，需按候选集而非单窗重定判据；
   - 若 `bridge_identity` 改为比较 `seg_c` 右端，全部延展都会变成「消失 + 新建」，两类计数
     被 Supersedes 流失的部分污染，判据须同步重定；
   - 若 `P559_SEG_LEDGER` 在其它标的/窗口上出现 `short > 0`，§9.3 的钉因结论翻转为
     「段序列 gap」，衔接守卫须按 MED-HIGH 正确性缺陷重开；
   - 若 provider 相构造与账本喂入解耦，C3 删掉的第三钟须重新引入（届时它才是真实时点）。
4. **下游推论**：引用 `IdentityVanished=0` 的一切下游判定（#421 自查档 §10.1 等）作废，
   改按新不变量重读；`identity_vanished_count` 字段已不存在，读该字段的下游须改读两个分列
   字段；(b) 类 12 只禁入寿命/反超率统计；本节全部读数仍是 **L2**（BTC 单标的单窗真实数据），
   跨品种外推需另跑（#523 遗留 5）。
5. **谱系引用**：#523（PanLive provider 接缝缺口，根因；本轮 (b) 类挂其遗留问题 2）；
   #421 E 裁定 / 收口包裁定（C4 口径载体）；#527（本票实装）；#559（影子评审）；
   R43；ADR-0003；090（C2/C3 均是声明与能力不一致的实例）；
   `no-patch-mentality`（C3 删字段而非留空字段；C2 回退不可测守卫并照实登记，不留 TODO）；
   `formalization-validity-domain`（§9.3 证据 1 为 L2 实测，衔接不可能性的构造论证为 L0）。
6. **影响声明**：改动 3 个生产文件（`nest_lifecycle.rs`、`p123_fast_replay.rs`、
   `p409_pan_live_probe.rs`）+ 3 份文档（本报告、`v3-lifecycle-rebuild-impl-20260724.md`、
   `issue421-acceptance-selfcheck-20260727.md`）。受影响面 = lifecycle 账本原因码类型与字段、
   lifecycle dump/stderr 诊断面、上述文档口径；**未改** p123 stdout / P116 dump / m8 trades /
   tower_events / 装配 / 证书 / `Cargo.toml` / 他 session 未提交面（含未跟踪的
   `rust/src/bin/issue570_ledger_probe.rs`，未触碰）。未 push、未关票、未改 map、未改 roster。
