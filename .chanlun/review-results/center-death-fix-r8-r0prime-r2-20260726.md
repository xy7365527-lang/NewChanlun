# #331 死亡指向错位修法包（R8 + R0′ + R2）——实装 + wf8 实测

**票面**：#331（parent #330 / #274 / #278，阻塞 #292）
**日期**：2026-07-26
**根因与裁定来源**：`.chanlun/review-results/center-death-identity-rootcause-20260726.md`（#330）+ #330 用户裁定一/二
**基线**：main @ `f94eaebd3e`（#329 误杀校验落地）

---

## 0. 一句话结论

**R8 结构性成立，R0′/R2 落地，但 miskill 未收敛：888 → 888（零收敛）。** 逐例分类坐实残余机制不是级别错配、不是段游标记账、也不是锁死放大，而是**两条中枢链的推进时钟不同**——事件机的链只能靠三类点/一类点（BSP 死亡事件）推进，分类器的链靠结构推进（窗口扫描游标 + 延伸吸收）。888 条里 **884 条（99.5%）** 是「机器落在塔链上游、载体指向下游」，同层序号差 Δidx ∈ [2, 549]。这正是 #330 §5 划给 **R3** 的面，R8/R0′ 按定义够不着。

票面验收项「miskill 从 888 大幅收敛」**未达成**，如实报告，不做措辞美化。

---

## 1. 三件套改动摘要（符号锚）

### R8（机器向塔对齐）— `opsem_dump.rs::OpsemDump::feed_center_lifecycle`

| 项 | 改前 | 改后 |
|---|---|---|
| ℓ≥1 投影源 | `project_to_units_resume(&tower[lvl-1][..w], &levels[lvl-1].moves, ..)` | `project_to_units_resume(&tower[lvl][..w], &levels[lvl-1].moves, ..)` |
| ℓ≥1 确认前缀 | `confirmed_lens[lvl-1]` | `confirmed_lens[lvl]` |
| ℓ≥1 冻结界 | `min(parent_len-1)`，`parent_len = tower[lvl-1].len()` | `min(tower[lvl].len()-1)` |
| 配对守卫 | 无 | `debug_assert_eq!(levels[lvl-1].centers.len(), tower[lvl].len())` |

**水线冻结重推（模块头已写全）**：单元 i 方向 = `center_own_dir_at(levels[ℓ-1].moves, i)`，i 是中枢下标（`decompose.rs:180-188`）；中枢序列 = `levels[ℓ-1].centers`，与 `tower[ℓ]` 1:1 ⟹ m = `tower[ℓ].len()`；`decompose.rs:26` 关系 R_j 冻结 ⟺ j < m-2，单元 i 用 R(i-1) ⟹ i < m-1 ⟹ `w ≤ tower[ℓ].len()-1`。

**顺带修好 blocks 配对**：#330 §2.3 旁证指出旧式 `blocks=levels[lvl-1].moves` 与 `tower[lvl-1]` 本就不配对（该 blocks 应配 `tower[lvl]`，`mod.rs:2158-2171` 是唯一正确配对）。换层后一次修好两处，`center_own_dir_at` 索引错位随之消失（#330 未能判定项 2 部分闭合——ℓ≥1 侧）。

**1:1 配对不变量实测**：临时把 `debug_assert_eq!` 改硬 `assert_eq!` 跑完整 wf8（264,960 bar，全 5 级），**零 panic**，产物与常规跑逐字节一致。改回 `debug_assert_eq!` 后提交。

### R0′（L0 段游标）— `center_lifecycle.rs::CenterEventMachine::push_point` 三类破坏分支

- 新增 `self.segs.drain(..born_seg_ordinal)`：已死中枢出生窗口及其之前的段随该中枢消费丢弃。
- 模块头「段计数口径」按票面更正：「三类破坏**不清**段序列」→「三类破坏后**已消费段不再参与新中枢计数**」，并把旧口径显式标作废。
- `CenterLifecycleEvent::Broken` 变体文档同步。

**有效域（已写进模块头，不藏）**：本条只改**计数口径**，**不改窗口选择**——滑窗恒测「尾 3 段」，尾窗由段序列末端定位，丢弃头部不改变任何被测窗口。所以 `born_seg_ordinal` 的分母变了，出生时点/身份不变。

### R2（逃逸阀）— `center_lifecycle.rs` 计数 + `opsem_dump.rs` 触发

- 字段 `miskill_streak` → `alive_miskills`，访问器 `consecutive_mis_kills()` → `alive_mis_kills()`。
- **口径修正**：原实装是「**连续**拒杀，任何 `Ok` 返回路径（含无关点的 `Ok(None)`）归零」；报告 §5 R2 原文是「**累积**拒杀 N 次」。原口径下阀门被二类点等无关点持续打断，wf8 实测 888 次拒杀只放行 **4 次**（≈ 形同虚设）。改为「同一在场中枢实例上累计拒杀」，归零点 = 在场实例换人的四处（出生 / 破坏放行 / 一类同死 / `resync()`）。
- 触发：`alive_mis_kills() >= MISKILL_ESCAPE_N (=16)` ⟹ 该级 `resync()` + `cl_fed_units[lvl].clear()` + `kind:"resync" reason:"miskill_escape_valve"` 诊断行 + `cl_resync_total` 计数。

---

## 2. wf8 实测读数

命令：`M8_WIN_FILTER=wf8 VOICE_EXEC=1 OPSEM_DUMP_DIR=<dir> cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture`
窗口：BTC anchored i=8，test 段 2023-08-17..2024-02-16，264,960 bar。

### 2.1 总表

| 态 | born | broken | reset | **miskill** | resync |
|---|---|---|---|---|---|
| #291 原态（无校验，`h1_base_dump`） | 763 | 733 | 59 | 0 | 7 |
| #329 锁死态（`h1_after_dump`） | 13 | 1 | 0 | **888** | 7 |
| #331 中间态（R8+R0′，R2 旧「连续」口径） | 14 | 1 | 0 | **888** | 11 |
| **#331 交付态（R8+R0′+R2 累积口径）** | **65** | **1** | **0** | **888** | **62** |

### 2.2 分级（交付态）

| lvl | born | broken | miskill | resync | 不同 alive 身份数 | alive si 取值 |
|---|---|---|---|---|---|---|
| 0 | 51 | 1 | 792 | 49 | **2** | 5, 624 |
| 1 | 6 | 0 | 51 | 5 | **1** | 1254 |
| 2 | 4 | 0 | 18 | 3 | **1** | 1254 |
| 3 | 4 | 0 | 27 | 3 | **1** | 1254 |
| 4 | 0 | 0 | 0 | 2 | — | — |

**锁死解除？部分。** 逃逸阀确实在放行（L0 49 次 resync、born 13→65），但 **51 次 born 只有 2 个不同身份**——每次 resync 后 `cl_fed_units` 清空、从前缀头回放，机器确定性地重生同一个中枢身份再次锁住。这正是 #330 §5 R2 的警告「resync 后重喂从前缀头回放 ⟹ 确定性复演同一条锁死路径」，**在 R8+R0′ 已就位的前提下依然成立**（原报告只说「不配 R8/R0′ 则复锁」，实测表明配了也复锁）。

`miskill` 逐级计数（792/51/18/27）与 #329 锁死态**逐级完全相同**——因为该数 = 该级在「机器有在场中枢」期间收到的一类/三类点总数，机器几乎全程有在场中枢，故与身份对错无关地饱和。

---

## 3. 残余失配逐例分类（888 条全覆盖）

分类脚本 `/tmp/331_resid.py`，数据 `/tmp/fix331_dump2/`（`center_lifecycle.jsonl` + `tower_events.jsonl` 的 `new_center` 身份表）。

### 3.1 分类 A — alive / target 各自在塔链上的归属层（**R8 的直接验收**）

| 机器 lvl | alive 塔层 | target 塔层 | 条数 | 判读 |
|---|---|---|---|---|
| 0 | 1 | 1 | 756 | ★ 同层 |
| 0 | 1 | 查无 | 20 | target 不在塔链 |
| 0 | 查无 | 1 | 16 | alive 是塔链外身份 |
| 1 | 2 | 2 | 34 | ★ 同层 |
| 1 | 2 | 查无 | 17 | target 不在塔链 |
| 2 | 3 | 3 | 18 | ★ 同层 |
| 3 | 4 | 4 | 21 | ★ 同层 |
| 3 | 4 | 查无 | 6 | target 不在塔链 |

**层错配条数 = 0。** 对照 #330 §3 例 3/例 4：改前 lvl1 是 alive@塔L1 / target@塔L2、lvl2 是 alive@塔L2 / target@塔L3，整整差一级。**R8 结构性成立，96 条 ℓ≥1 层错配全部消除。**（收益是结构正确性，不是计数下降。）

### 3.2 分类 C — 同层内序号差 Δidx = target_idx − alive_idx

- 落在同层可比的 **829 条**：Δidx **全为正**，范围 **[2, 549]**，取值 350 余个、最大频次 21。
- alive 不在塔链：16 条（L0，塔链外身份，即 #330 §3 例 1 的 `si=624`）。
- target 不在塔链：43 条。

**Δidx 全正且散布极广 = 机器系统性落在塔链上游。** 不是常数偏移（排除 off-by-one/平移），不是随机噪声（无负值）。

### 3.3 分类 D — target 的可达性

| 档 | 条数 | 占比 |
|---|---|---|
| target 本机曾生出过（纯时序滞后 / D6 陈旧请求） | 4 | 0.45% |
| target 是塔链上真实中枢、但本机**从未生出** | 841 | 94.7% |
| target 身份不在塔链、核心 (zd,zg) 也查无 | 43 | 4.8% |

### 3.4 分类 E — 起点先后

| 关系 | 条数 |
|---|---|
| target 起点**晚于** alive（机器落后于塔链） | **884** |
| target 起点早于 alive（陈旧请求，D6） | 4 |

### 3.5 归因

三条分类交叉指向同一机制：

> **事件机的中枢链只能靠 BSP 死亡事件推进（一中枢一场 + 在场期不测出生），分类器/塔的中枢链靠结构推进（窗口扫描游标 `i=j*` + 延伸吸收）。** 机器出生于链上第 k 个中枢后，只有当某个三类点**恰好指名第 k 个**才能换代；而载体取的是「当下正在离开的中枢」= 第 k+Δ 个。Δ 一旦 ≥1 就再也回不来 ⟹ 后续每个死亡请求都失配。

R8 修的是「两条链在哪一层比」，R0′ 修的是「死后段怎么记数」，二者都不改变链的推进时钟。**这是 #330 §5 R3（alive 侧改为消费塔的同一中枢链）的域。**

### 3.6 R0′ 在本窗的爆炸半径上界（静态可判，无需另跑）

R0′ 只作用于 `Broken` 分支。wf8 全窗 `broken = 1`、`reset = 0` ⟹ **R0′ 在本窗最多影响 1 个事件**。888 条残余在结构上不在 R0′ 的可达范围内——无论取「最小版（丢弃已消费段）」还是「完整版（破坏后对存活后缀前向重扫）」，上界都是 1。故本票未实现完整版重扫（那是窗口调度的教义改动，须另行裁决），亦无需实测即可判定其对本窗残余无效。

---

## 4. 双轨与锁读数

| 项 | 结果 |
|---|---|
| `wf8 trades.jsonl` vs `h1_after_dump` 基线 | **逐字节一致**（948,340 B） |
| `wf8 tower_events.jsonl` vs 基线 | **逐字节一致**（354,946 B） |
| wf8 报告行 | `execR=+4040483 MaxDD=0.0917 stage=I R=+4417092 LCB(R)=-1921382 → INCONCLUSIVE`（与基线同） |
| `cargo test --lib` | **1909 passed / 0 failed / 138 ignored**（票面基线 1907 + 本票新增 2 条单测） |
| #329 校验两态单测 | `kill_passes_when_point_owner_matches_alive_center` ✅ / `mis_kill_rejected_when_point_owner_differs_from_alive_center` ✅ |
| 四把锁（`--ignored`，BTC 数据） | 4/4 绿：`btc_type2_open_short_channel_witness` / `btc_prune_leg_exit_type_matches_account_identity` / `btc_type2_residual_correction_witness` / `typed_ledger_btc_smoke` |

**四把锁「零翻动」的诚实标注（沿用 #323 Critical-2 先例）**：本票全部改动位于 `feed_center_lifecycle` 及其调用的 `CenterEventMachine`，二者由 `OPSEM_DUMP_DIR` env 门控（`fill.rs:1071-1073`：`if let Some(dump) = opsem.as_mut()`）。四把锁**不设该 env**，因此对本票改动路径的覆盖为 **0**。**零翻动是零覆盖的结果，不构成安全证据。** 真正覆盖本次改动的是「开着 dump 跑 wf8 后 `trades.jsonl`/`tower_events.jsonl` 逐字节不变」这一条——它证明诊断旁路即便打开也不回馈生产链。

**慢锁不跑标注**：本票未跑重型多窗回归（`m8_e2e_all_systems_oos` 全窗臂、`l3_*` 多窗族、`p1xx` 校准 bin）。理由 = 生产轨迹逐字节不变 + 改动全在 env-gated 旁路内，重型窗口的期望翻动为 0；**留重型窗口**，如需硬证据请在重型窗口批次里补跑。

---

## 5. 未能判定项

1. **票面验收「miskill 大幅收敛」未达成**（888 → 888）。R8/R0′/R2 三件套按裁定实装到位并逐条验证生效，但残余机制归 R3。是否上 R3、以及 R3 要付的代价（放弃独立复算、重定义「一中枢一场」、#291 全部对账基线作废、#329 校验退化为近恒真）须用户裁决。
2. **R0′ 完整版（破坏后前向重扫存活后缀）未实现**——那是窗口调度的教义改动（模块头「滑窗恒测尾 3 段」），#330 裁定二的措辞是计数口径（「已消费段不再参与新中枢计数」），未授权改调度。本窗爆炸半径上界 = 1 事件（§3.6），不影响本次结论。
3. **resync 后从前缀头回放** 是可修的工程缺陷（应从 frontier 附近恢复而非从头），本票未动——它会让每次逃逸都重生同一身份（L0 51 次 born 只 2 个身份）。修它能提高逃逸阀有效性，但不解决 §3.5 的时钟问题。另立票候选。
4. **43 条 target 核心在塔链完全查无**（4.8%）未逐条溯源——可能是 canonical 表 tail_centers 追加、#148 升级重切、或载体快照与最终塔态的时序差。本票未展开。
5. **跨窗普适性**：全部读数只来自 wf8 单窗。R8 的层对齐是结构性的（不依赖窗口），但 Δidx 分布、43 条查无占比等均为单窗观测。
6. **L4 面**：交付态 L4 有 2 次 resync、0 born、0 miskill，未纳入分类（#330 未能判定项 5 仍未闭合）。
7. **`h1_after_dump` 作基线**依据 #330 §1 的 mtime 时序推断（早于 `f94eaebd3e` 4.5 分钟），未做二进制级对应验证；本票沿用该判断。

---

## 6. 纪律

- 只 `git add` 本票自己改的两个源文件 + 本报告；`p107_level_calib.rs` 等既有脏文件未动。
- 未回关 issue、未发 GitHub 评论。
- 分析脚本在 `/tmp`（`/tmp/331_resid.py`）；产物 `/tmp/fix331_dump`（中间态）、`/tmp/fix331_dump2`（交付态）、`/tmp/fix331_assert`（硬断言验证跑）。
- TDD 红检：撤掉 `segs.drain(..born_seg_ordinal)` 后 `broken_by_third_class_point` 与 `broken_after_alive_period_fed_beyond_birth_window_keeps_trailing_segments` 双双变红，确认新断言非恒真；R2 口径改动先改测试变红再改实装。
