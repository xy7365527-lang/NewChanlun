# 影子评审 #629：#602 L3 PanLive provider（两轴，新上下文禁自评）

> 角色：影子评审（只读；前台单线程、禁子代理）
> 对象：`fd585859fa`（#602 L3 PanLive provider），基线 `d40607d805`（#573 T1 migrate 后）
> 交付报告：`chanlun/review-results/issue602-panlive-l3-provider-20260728.md`（445 行）
> 工位：`/tmp/kimi-nest-mainline`（**并发活跃**——本评审全部测量在 `git archive` 出的隔离净树
> `/tmp/rev629-head`（`fd585859fa`）/`/tmp/rev629-base`（`d40607d805`）/`/tmp/rev629-573base`
> （`886a439c78`）上完成，独立 target dir，未改仓内任何文件）
> 口径：不改教义、不关票、不改 map/roster；本报告是唯一落盘产物。

## 0. 结论

**PASS WITH CONDITIONS**

- **Spec 轴**：票面 4 条 Acceptance 全部达成。全部**可独立复现**的读数**逐值命中、零偏差**
  ——L1/L2 逐位零回归、L3 8/8 归因闭合、§4.1 逐只终判（含 +163 / Δ=0×3 / −190·−574·−860 /
  1 只全程无行）、四面字节护栏、#533 门、测试门 ±4、#573 migrate bit-exact。本评审**未发现
  任何正确性缺陷**。
- **不可复现项 1 处**（探针读数，见 P2）：探针代码按纪律未入 commit 且未附 patch ⟹ §3
  全部读数（970 次重扫、`nwin` 分布、Σ(m−1)=91、批内非末窗 0/8）目前只有作者单方证词。
  报告已自陈局限，但「批量丢弃解释力 0/8」是本票的**核心裁断之一**，其证据面不可第三方复算。
- **条件**（均不需改生产代码）：C1 #454 增量登记（**重复项**，前两轮已确立纪律）；
  C2 #617 「专门原因码」的显式结账；C3 探针可复算物落盘；C4 三处口径订正（S2/S4/S5）。

## 1. 独立复现矩阵（本评审自跑，非读报告）

净树 = `git archive <commit>` + `find -name '*.rs' -exec touch +`（避 cargo 重放缓存假绿，
报告 §7 已点名同一坑）。

| # | 核验项 | 报告声明 | 本评审实测 | 判定 |
|---|---|---|---|---|
| 1 | `cargo test --lib` post（净树 `fd585859fa`） | 2079 / 1 / 137 | **2079 passed / 1 failed / 137 ignored** | ✅ |
| 2 | 同上 pre（净树 `d40607d805`） | 2075 / 1 / 137，差额 +4 | **2075 / 1 / 137**，差 **+4** | ✅ |
| 3 | 唯一红 | `extract_signals_bit_exact_digest_guard`（#491） | 逐字相同，pre/post 失败集合恒等 | ✅ |
| 4 | `cargo test --bin p123_fast_replay` | 5 passed | **5 passed / 0 failed**（含新增 2 条） | ✅ |
| 5 | `#533` 门（`--ignored`） | 2 passed（重锚后） | **2 passed** | ✅ |
| 6 | p123 stdout 20k/100k | cmp=0；`bd9ac1d655f9d615` / `d8b69c180c23c5e3` | pre/post **cmp=0**，SHA 逐字相同 | ✅ |
| 7 | P116 dump 20k/100k | cmp=0；`fcc8016a9a01a109` / `8a7327feb3b9ba29` | pre/post **cmp=0**，SHA 逐字相同 | ✅ |
| 8 | m8 wf8 两面（抽核） | `b265c52ed27967d6` / `1d8dff0d29e8925f` | trades **`b265c52ed27967d6`**、tower_events **`1d8dff0d29e8925f`** | ✅ |
| 9 | lifecycle dump post SHA | 20k `d7a7143031a6ef63…` / 100k `5ae3da9b5718e0f2…` | 逐字相同，且与 commit 内 golden fixture 新值一致 | ✅ |
| 10 | golden 重锚面 | 只 dump 面；stdout 行未动 | `issue533_p123_{20000,100000}.sha256` **仅 dump 行变**，stdout 行原值；P116 golden 未入 diff | ✅ |
| 11 | `#533` 登记表 | 已追加一行 | `issue533_p123_byte_guardrail.rs` +1 行（含 commit / 动了哪些 golden / 原因 / 报告锚） | ✅ |
| 12 | 探针代码入 commit | 交卷前已删 | `git show \| grep -i probe` 仅命中报告正文；`grep -r P602_PROBE rust/src/` 空 | ✅ |
| 13 | #573 migrate bit-exact | 三面 cmp=0 | `886a439c78` vs `d40607d805` **六面全 cmp=0**（stdout/P116/**dump** × 20k/100k），唯一差异是 `prefix_s` 计时 | ✅（本评审多核了 dump 面） |
| 14 | 三文件零新增警告 | 只剩 `mod.rs:1850` unused_mut（基线既有） | touch 三文件强制重编：唯一命中即 `classifier/mod.rs:1850`，且该行不在本票 diff 内 | ✅ |

### 1.1 L1/L2 逐位零回归（pre/post 双跑，stderr 逐值对拍）

`P527_L1_LIVE_OUTCOMES`（100k）——pre 与 post **逐值相同**，post 仅**追加** L3 八项：

```
l0:no_active_frontier=57  l1:center_not_consolidation=388  l1:structure_not_locatable=818
l1:tower_level_absent=400 l1:window=610
l2:center_not_consolidation=329 l2:lower_frontier_not_absorbed=558 l2:structure_not_locatable=467
l2:tower_level_absent=520 l2:window=342
（post 追加）l3:window=72 l3:structure_not_locatable=213 l3:center_not_consolidation=78
l3:lower_frontier_not_absorbed=462 l3:no_lower_frontier=483 l3:no_window_formed=144
l3:tower_level_absent=834 l3:resume_anchor_out_of_range=1
```

| 面 | pre（实测） | post（实测） | 与报告 |
|---|---|---|---|
| `entries` / `force_overtake_claimed` | 301 / 2 | **301 / 2** | ✅ |
| `first_provable`/`confirmed`/`force_overtake`/`never_constituted` | 207/135/40/74 | **逐值相同** | ✅ |
| `identity_vanished_refuted / _seam` | 28 / 23 | **逐值相同** | ✅ |
| `flash_terminal` / `nonflash_count` | 74 / 226 | **73 / 227** | ✅ 唯一差异 |
| `P559_SEG_LEDGER` | complete=1840 short=0 no_tower=433 | **逐值相同** | ✅（证 `seg_ledger_*` 只挂 lower 段的处理正确） |
| `completion_signals`/`channel_switches`/`retrograde_rejected` | 248/248/0 | **248/248/0** | ✅ 永禁清单 |
| 逐级终局/寿命（本评审自写复算脚本，身份键 = `(level, side, seg_a, λ_C, b)`） | L1 flash45/nonflash191/1‑80‑423；L2 flash21/nonflash34/14‑139‑358；L3 flash **8**/nonflash 0 | L1、L2 **逐值相同**；L3 flash **7**/nonflash **1**/寿命 **163** | ✅ |
| 20k 前缀 | — | L1/L2 逐值不变，唯一新增 `l3:tower_level_absent=781`，L3 完成身份 **0** | ✅ |

### 1.2 L3 8/8 归因与逐只终判（脚本复算）

`sed 's/level=2/level=3/g' issue613-attrib-buckets.py` 跑本评审自产 dump：

```
L3 完成身份（去重）= 8
  A. 存在 earlier Live 1 | B.lower_frontier_not_absorbed 3 | B.no_lower_frontier 2
  B.tower_level_absent 1 | B.window_other_c 1 | 合计 8 | 归因闭合：PASS（零「不知道」）
```

L2 同脚本原版：**38 = 17 A + 9 + 4 + 3 + 3 + 2**，与报告逐项相同。

逐只终判（本评审自写脚本，锚 = 完整六元含 `seg_a`）——**与报告 §4.1 表逐格相同**，
含 `lower_id`：

| 完成 as_of | λ_C | seg_a | B | lower_id | 首见 | Δ |
|---:|---:|---|---:|---|---:|---:|
| 23577 | 16724 | (2295,3223) | 8073 | (2,8) | 23577 | **0** |
| 24994 | 23954 | (12513,15180) | 16724 | (2,11) | 24994 | **0** |
| 32154 | 30441 | (23954,28179) | 16724 | (2,14) | 32154 | **0** |
| 33470 | 32117 | (28182,30430) | 16724 | (2,15) | 33660 | −190 |
| 39348 | 38525 | (28182,30430) | 32117 | (2,18) | 40208 | −860 |
| 59497 | 56425 | (46318,52957) | 38525 | (2,24) | **59334** | **+163** |
| 68867 | 67403 | (56425,59517) | 59520 | (2,28) | 69441 | −574 |
| 76420 | 75281 | (67403,71033) | 59520 | (2,31) | — | — |

末只窗内 level=3 码分布实测 `no_lower_frontier 7 / no_window_formed 3 /
lower_frontier_not_absorbed 11` —— 与报告逐值相同。
唯一 earlier Live 的 `observed_at=59334 ≠ λ_C=56425` ⟹ **零回填**实证成立（#523 永禁清单）。

### 1.3 机制核（读码，不只读报告）

| 项 | 核实 |
|---|---|
| `scan_active_window` 抽取质量 | 34 行、三道守卫齐备；`active_l1_window_frontier` 退化为 42 行薄包装。**逐位等价的实证** = 上表 L1/L2 全 tally 与逐级寿命分位不动（比读码更硬） |
| build 换级别 | `classifier/mod.rs:307/319` 证塔侧 L1 层用 `center_from_segments`、上级层用 `center_from_window`；provider 的 `center_from_window` 选择与塔逐字同源；`r2` 用同一夹具把两条 build 的分歧钉成断言 |
| 方向来源（L3 特有点） | `project_to_units`（ownership 方向，Q7）与 `lower_legs_from`（`first_leaf_direction`）确为两套；`r1` 夹具令二者逐位相反并断言取前者 ✅ |
| F-新2 输入侧截断 | `scan_units = &units[..l1_view.confirmed_len.min(units.len())]`，水线与 confirmed 段侧同一证书（`tower_confirmed_len`）；生产实测 L3 `lower_frontier_not_after_units` **0 次**（stderr 无该 tag），`resume_anchor_out_of_range=1`（报告已登记「不把锚往前拽」） |
| F-新1 判据键拆两把 | 两把键的字段与「谁消费谁」一一对上；`lower_due ⟹ upper_due` 单向正确；`carry` 的全部输入确在 lower 键内（`l0_units`/`frontier`/`l1_cursor`；`tower[1]` 内容由 `forest_epoch` 覆盖）⟹ upper-only 重算复用 carry 不产生跨 bar 陈旧态。**并键版 `l1:window` 610→615 未复现**（需改代码，只读评审不做），但该断言的**反向证据充分**：拆键后 L1/L2 实测逐位不动 |
| 同长守卫基准订正 | `units.len() != tower[level-2].len()`（输入塔层）正确；`lower_legs_from`/`project_to_units` 均为 1:1 map，故该式即同长不变式；实测 `scan_units_out_of_sync` 恒 0 |
| `#449` / p409 / 塔存储 | diff 未触 `TypedNestCertificate`/`turn_class`/`BspBits`/`Classification` 写路径（仅 import 上下文行）；`recursive_tower.rs`、p409 零改动；`classifier/mod.rs` 删除行 = 0 |
| 新码可达性 | `scan_units_absent` / `lower_legs_unprojectable` 生产实测 0（前者结构上仅在缺级时可达、后者补的是原静默 `continue` 通道，补码本身正确）；`lower_leg_dirs_out_of_sync` 见 **S2** |

## 2. 新发现（分级 + 锚）

### S1 — MEDIUM（**重复项**，跨票纪律断链）：#454 文件长债务再加剧且报告零提及、#454 无本票增量登记

**事实**：

| 项 | #609 F9 立案时 | 本票后（实测） |
|---|---:|---:|
| `nest_lifecycle.rs` 总行 | 4683 | **5844** |
| 同上生产段（`#[cfg(test)]` 之前） | 2042 | **2473**（上限 800） |
| `recompute_lifecycle_window_stems` | 162 行 | **247 行**（上限 50） |
| `run_targeted_prefix_pass` | — | **606 行** |
| `p123_fast_replay.rs` 总行 | — | 2920 → **3193** |

**为什么是重复项**：`gh issue view 454 --comments` 显示已有两条增量登记评论——
①「#601 使 `nest_lifecycle.rs` 再增 ~460 行……debt destination ≠ 增量免记。来源
`shadow-601-review-20260728.md` F9」；②「#603 档1 落地后达 5005 行……来源
`shadow-603-review-20260728.md` L11/L12」。即**「每次加剧须在 #454 单独在案」已是既定纪律**。
本票 `+433` 净行（生产段 +431）、`recompute_lifecycle_window_stems` 再 +85 行，
**交付报告全文零处提及 #454 / 行数 / 文件规模**（`grep '454\|800 行\|文件规模'` 空），
#454 亦无本票评论。

**锚**：`.claude/rules/common/coding-style.md`（200–400 典型 / 800 上限；函数 <50 行；嵌套 ≤4）；
`gh issue view 454 --comments`；`wc -l` 实测。

**修法（不改生产代码）**：在 #454 追一条本票增量评论（行数 + 函数长 + 来源本报告），
与前两条同格式。

### P1 — LOW‑MEDIUM（Spec：上游裁定的动作项未显式结账）：#617 点名的「专门原因码」

#617 Resolution 原文：「接受『机制上不可观测』；**专门原因码进 #602 验收**（先探针实测该类
规模）」。本票**做了**前置（探针实测 Σ(m−1)=91、批内非末窗 0/8），但**未落**任何「批量丢弃」
专门原因码，报告也**未显式说明不落及其理由**——只在 §2.2 论证「守卫 3 使批内更早窗口连被拒
的机会都没有」、§6 遗留 4 登记「下游影响未评估」。

**判断（本评审倾向支持不落码，但要求显式）**：批量丢弃发生在**窗口实例**层，而原因码体系
挂在**身份归因**层；0/8 身份落该类 ⟹ 新设码在归因表上恒 0 且无挂载点，硬设反而是 090 意义上
的声明膨胀。故正确处置是**照实结账**：在报告里写明「#617 要求的专门原因码经实测判定在身份
层无挂载点，故不设；该类规模改以 Σ(m−1) 登记」，并回填 #617/#602 评论。现状是这一句缺席，
读者只能从 §2.2/§6 拼出来。

**锚**：`gh issue view 617 --comments`；报告 §2.2 / §3.4 / §6.4。

### P2 — LOW‑MEDIUM（Spec：核心裁断的证据不可第三方复算）：探针面

§3 全部读数（重扫 970 次、`nwin` 分布、末窗吸收 638/363、**Σ(m−1)=480→91**、批内非末窗
**0/8**、`λ_C == tower[2][ord].start_index` 的 join 键成立性）来自**已删除的临时探针**。
本评审能独立复现的只有其**间接后果**（`lower_frontier_not_after_units` L3 生产 0 次、
`l3:window=72`、8 只 `lower_id` 与 λ_C 的对应），**探针本身的读数无法复算**。

这不是违规——「探针不进 commit」是本票自定且合理的纪律，报告也诚实登记了探针局限
（「探针与实装同源，测不出实装本身选错口径」）。但 §3.4 的裁断「把批量丢弃当 L3 覆盖率低的
解释是未经验证的归因，本票不采信」是**票面前提的翻转**，其证据面目前是单方证词。

**修法**：把探针 patch（`git diff` 文本）或其 stderr 原始产物落到
`chanlun/review-results/` 附件，标注「诊断用，不进生产」。与「不进 commit」不冲突。

### S2 — LOW（Standards / 090 口径）：`LowerLegDirsOutOfSync` 在唯一消费方结构不可达

守卫在 `active_l2_window_frontier` 内为 `l1_leg_dirs.len() != l1_units.len()`，但唯一调用点
（`p123_fast_replay.rs`）传的是
`&carry.l1_leg_dirs[..carry.l1_leg_dirs.len().min(scan_units.len())]`：

- `leg_dirs` 更长 ⟹ 被 `min` **静默截齐**，守卫不触发；
- `leg_dirs` 更短 ⟹ 需 `lower_legs_from(tower[1]).len() < project_to_units(tower[1]).len()`，
  而两者均为对 `tower[1]` 的 1:1 `map`，且 `scan_units_out_of_sync` 已先拦同长破坏 ⟹ 不可能。

故该码在生产路径上**恒不可达**，而非「能力已具备、本窗未触发」（报告 §4.1 把它与
`scan_units_absent` / `lower_legs_unprojectable` 三码并列作同一句登记；后两者的「未触发」是
数据性的，这一条是结构性的）。API 层保留该守卫本身正确（对未来其它调用方有效），要订正的是
**登记口径**。

**锚**：`p123_fast_replay.rs`（L3 分支 `active_l2_window_frontier(...)` 调用点）；
`level_view.rs:442 lower_legs_from`；`recursive_tower.rs:898 project_to_units`；报告 §4.1 末段。

### S3 — LOW（Standards / 注释陈旧）：`unreachable!` 的理由句与实装不符

`recompute_lifecycle_window_stems` 的
`_ => unreachable!("循环上界写死 `for level in 1..4`")` —— 本票已把上界改成**参数** `levels`，
不再「写死」。两处调用（`1..3` / `3..4`）并集恰为 `1..4` 使其当前成立，但这是调用点约定而非
代码事实；同理 `1 => frontier.expect("levels.start==1 ⟹ frontier 为 None 时已在上面早退")`
也是靠调用点约定维持的 panic 通道。建议改成对 `levels` 自身的显式断言或落码。

**锚**：`p123_fast_replay.rs` `recompute_lifecycle_window_stems` 的 `match level` 尾臂与 L1 臂。

### S4 — LOW（报告数值口径）：「L1 身份 239」与 `entries=301` 不自洽

报告 §验收2 表记「L1 逐级终局/寿命 身份 239 / flash 45 / nonflash 191 / 1‑80‑423」。
本评审复算（键 = `(level, side, seg_a, λ_C, b)`）得 **L1 身份 238**，其余四项逐值相同；
且 **238 + 55（L2）+ 8（L3）= 301 = `entries`**，报告口径 239+55+8=302 与 `entries` 差 1。
pre/post 同口径 ⟹ **不影响零回归结论**，属登记数值需订正。

### S5 — LOW（报告登记完备性）：两个变化了的 summary 读数未登记

`P421_LIFECYCLE_SUMMARY` 中 `live_windows` **50702 → 55950**、`extension_suppressed`
**26040 → 31121**（本评审 pre/post 实测）。二者是本票本体面（L3 活窗行新增）的直接产物、
不构成回归，但报告 §验收2/§验收3 均未登记。「照实登记」纪律下应出现。

## 3. 逐条对照（票 #629 重点核验项八条）

| # | 核验项 | 判定 | 备注 |
|---|---|---|---|
| 1 | 探针预分级：两轮读数 / 「批量丢弃存在但不解释这 8 只」裁断 / 「当解释=未验证归因」是否照实 | **部分可核** | 裁断成立性在逻辑与间接证据上成立（L3 `lower_frontier_not_after_units` 生产 0 次、0/8 与 §4.1 四分法自洽）；读数本身不可复算 → **P2**。「不采信未验证归因」这一句照实、方向正确 |
| 2 | 8/8 归因闭合逐只抽核 | **PASS** | 分桶与逐只终判 100% 复现（§1.2），零偏差 |
| 3 | F-新1（拆键）/ F-新2（输入侧截断）机制与证据 | **PASS** | F-新2 有单测锁 + 生产实测；F-新1 的 615 断言未复现但反向证据充分（§1.3） |
| 4 | L1/L2 逐位零回归独立复现 | **PASS** | pre/post 双跑，全 tally + 逐级终局/寿命逐值；唯一差异 flash 74→73 即那 1 只 L3（§1.1） |
| 5 | 护栏四面 + m8 六面 SHA / #533 重锚纪律 | **PASS** | 四面自跑 cmp=0；m8 wf8 两 SHA 抽核命中；golden 只 dump 面 + 登记表一行在案 |
| 6 | #573 并发移植与 migrate bit-exact 附带实证 | **PASS** | `886a439c78` vs `d40607d805` **六面** cmp=0（比报告声明多核 dump 面）；本票全套验收均在 `d40607d805` 上复现成功 |
| 7 | 测试门（唯一红 #491 / +4 / 探针不入 commit / 污染面先核） | **PASS** | 净树 2079 vs 2075，+4；bin 5 passed；#533 门 2 passed；无探针入 commit。**污染实证**：直接在工位工作区跑得 **2080**，因他 session 的 #573 复审改动（`ledger_kernel/mod.rs` 与 `nest_lifecycle.rs` 各 +assert）正在落盘——报告用隔离副本的做法是必要的，本评审同样只信净树读数 |
| 8 | 永禁清单 + #449 + Standards 全轴 + `#454` 增量 + 探针不入 commit | **CONDITIONS** | 永禁清单/#449 逐条核实通过；Standards 轴见 **S1–S5**，其中 S1 为 MEDIUM 重复项 |

## 4. Standards 轴（Fowler 12 + 仓内标准源）

| 味道 | 判定 |
|---|---|
| Long Function | **触发**：`recompute_lifecycle_window_stems` 247 行（本票 +85）、`run_targeted_prefix_pass` 606 行（上限 50）→ S1 |
| Large Class/File | **触发**：`nest_lifecycle.rs` 生产段 2473 行 / `p123_fast_replay.rs` 2960 行（上限 800）→ S1 |
| Long Parameter List | **触发（既有加剧）**：`recompute_lifecycle_window_stems` 9 参（本票 −3 +2 净 +1），`#[allow(clippy::too_many_arguments)]` 在案。缓解：`TowerScanView` 三元打包是**正确方向**的抽象 |
| Duplicated Code | **改善**：`scan_active_window` 内核抽取消除了 L1/L2 两层重复；`miss()` 收口 tally+dump 两处同源 |
| Divergent Change / Shotgun Surgery | 未触发（改动集中在 provider 侧三文件，塔存储与消费方零适配） |
| Feature Envy / Data Clumps | `TowerScanView` 正是对 Data Clumps 的正解 |
| Primitive Obsession | 轻微：`levels: Range<usize>` + `if level == 2/3` 分支承载级别语义；可接受（上界 4，且每处差异都有显式登记） |
| Comments（作为除臭剂用） | **本仓库特有取向**：注释密度极高且承载谱系锚，与既有风格一致；但 S3 显示注释已开始与实装漂移 |
| Mutable Data | `LowerFrontierCarry` 是就地更新的跨级中间量——落在 `coding-style.md` 2026‑07‑27/28 两条例外注记（回放驱动性能缓存 / 单线程状态机）的语境内，且「先清空再回填」纪律显式；不判违规 |
| 抽取质量（`scan_active_window`） | **好**：34 行、单一职责、三守卫 + 一后果登记齐全，L1/L2 薄包装等价由生产读数背书 |
| 新原因码 `lower_legs_unprojectable` | **好**：补的确实是原静默 `continue` 通道（原代码 `let Ok(lower) = ... else { continue }` 不留行），口径与「零不知道」一致 |
| `level_scan_units` 访问器 | **好**：`level==1` 委托 `l0_units()` 而非重建（同源同值）、越界返 `None` 不猜值、同长不变式与持有边界写进文档 |

## 5. 结果包六要素

1. **结论**：**PASS WITH CONDITIONS**。#602 的 L3 provider 实装在正确性与验收复现上无缺陷
   ——全部可复算读数逐值命中、零回归双跑实证、#573 移植 bit-exact 附带实证比报告声明更强
   （六面）。四条条件均为**登记/结账/证据面**问题，不需改生产代码：
   **C1** 在 #454 追本票增量登记（S1，MEDIUM，重复项）；**C2** 显式结账 #617 的「专门原因码」
   （P1）；**C3** 探针 patch 或原始产物落盘以使 §3 可复算（P2）；**C4** 订正 S2（`LowerLegDirsOutOfSync`
   登记口径：结构不可达 ≠ 未触发）、S4（L1 身份 238 非 239）、S5（补登 `live_windows` /
   `extension_suppressed` 变化）。
2. **定义依据**：Acceptance 逐条对齐 #602 票面 1–4；归因闭合按 #613 分桶脚本的可执行规则
   （身份键含 `seg_a`，#618 订正口径）；活窗合法性按 #598 §1.1「行进中 = 因数据耗尽而停」
   与 #523 红线「不得拿 `tower[level]` 已产出窗口冒充活动腿」；截断口径按 #93 水线证书单一
   来源；级别中立内核的等价性按「L1/L2 生产读数逐位不变」实证而非声明。
3. **边界条件（本结论何时翻转）**：
   - 若 §3 探针读数在第三方复算下与报告不符（尤其「批内非末窗 0/8」），则 §3.4 对票面前提的
     翻转裁断失效，`l3` 覆盖率 1/8 的四分法归因需重做 → 结论降级；
   - 若 `forest_epoch` 被证实**不**覆盖 `tower[1]` 内容或 `l0_units` 的某类变化，则
     `LowerFrontierCarry` 的跨 bar 复用会引入陈旧态，F-新1 的拆键正确性随之失效
     （本评审按现行 `forest_epoch` 语义核为成立）；
   - 若「L1 身份 239/238」的差 1 实为身份迁移（3 条 `CenterUpgraded`）造成的口径分歧而非
     笔误，则 S4 降为无效发现（不影响其余结论）；
   - 若后续把 `levels` 传成含 `0` 或跨 `1..4` 之外的范围，S3 指出的 `unreachable!`/`expect`
     会从「不可达」变成 panic 通道。
4. **下游推论**：#597 map 的「46 只 L2/L3 身份钉因」在本评审独立复算下确实全部到位
   （L2 38/38、L3 8/8、两级零「不知道」）；§4.1 的滞后量（190/574/860）作为新可量化面成立，
   可作后续采样密度/桥接研究票的输入；`level_scan_units` + `scan_active_window` 的形状使 L4
   扩展成为纯参数化工作（报告 §6.5 已给配方），但 L4 前应先清 S1 的文件长债务，否则
   `recompute_lifecycle_window_stems` 会突破 300 行。
5. **谱系引用**：#598（路线 i 裁定）；#523（PanLive 接缝根因 + 永禁清单）；#601/#613/#609
   （L2 链与截断口径来源）；#617（批量确认 ⟹ 中间状态不存在——本评审对其「专门原因码」
   条款提 P1）；#618（时机判据边界——3 只 Δ=0 逐字复现）；#591/#592（同一架构特征的时间
   切面刻画）；#449（证书真值路径边界，核为不落入）；#533（字节护栏门与 golden 纪律）；
   #454（文件长债务 destination——本评审 S1 为其第三次增量点名）；090（声明 = 能力：S2/S3/S4
   均属此条）；`formalization-validity-domain`（全部读数为 **L2 级：BTC 单标的单窗**，
   1/8、163 bar、72 行、Σ(m−1)=91 不得写成规格常量——报告 §6.6 已自律登记）。
6. **影响声明**：本评审**未改仓内任何文件**，唯一落盘产物 = 本报告
   （`chanlun/review-results/shadow-602-review-20260728.md`）。测量全部在 `/tmp` 隔离净树与
   独立 target dir 完成（`/tmp/rev629-{head,base,573base}`、`/tmp/kimi-nest-target-629*`），
   未触工位工作区、未 stash、未碰他 session 未提交面。本报告不关票、不改 map/roster、
   不改教义；C1–C4 的执行属另一轮动作，需票面授权。

## 6. 复现命令（本评审自用，可第三方重跑）

```bash
# 净树（避 cargo 重放缓存假绿）
for c in fd585859fa d40607d805 886a439c78; do
  d=/tmp/rev629-$c; mkdir -p $d
  git -C /tmp/kimi-nest-mainline archive $c | tar -x -C $d
  (cd $d && find . -name '*.rs' -exec touch {} +)
  mkdir -p $d/analysis/data_cache
  ln -sf /Users/silencehan/Projects/NewChanlun/analysis/data_cache/btc_1m_full.json \
     $d/analysis/data_cache/btc_1m_full.json
done

# 测试门（每树独立 target dir）
(cd /tmp/rev629-fd585859fa/rust && CARGO_TARGET_DIR=/tmp/t629 cargo test --lib)      # 2079/1/137
(cd /tmp/rev629-d40607d805/rust && CARGO_TARGET_DIR=/tmp/t629b cargo test --lib)     # 2075/1/137

# 生产回放（pre/post 各一轮）+ 四面对拍
for w in 20000 100000; do n=$([ $w = 20000 ] && echo 20k || echo 100k)
  P116_MAX_BARS=$w P116_DUMP=/tmp/X-$n.p116 P421_LIFECYCLE_DUMP=/tmp/X-$n.dump \
    <target>/release/p123_fast_replay ../analysis/data_cache/btc_1m_full.json \
    > /tmp/X-$n.stdout 2> /tmp/X-$n.stderr
done
cmp pre-$n.stdout post-$n.stdout && cmp pre-$n.p116 post-$n.p116

# L3 归因 + 逐只终判 + 逐级终局（后两个脚本见 /tmp/rev629_perid.py、/tmp/rev629_settle.py）
sed 's/level=2/level=3/g' chanlun/review-results/issue613-attrib-buckets.py > /tmp/l3.py
python3 /tmp/l3.py /tmp/post-100k.dump -v
python3 chanlun/review-results/issue613-attrib-buckets.py /tmp/post-100k.dump

# 护栏
CARGO_TARGET_DIR=/tmp/t629r cargo test --release --test issue533_p123_byte_guardrail -- --ignored
M8_WIN_FILTER=wf8 OPSEM_DUMP_DIR=/tmp/m8-wf8 CARGO_TARGET_DIR=/tmp/t629r \
  cargo test --release --lib m8_e2e_all_systems_oos -- --ignored
```

> **踩过的坑（登记）**：直接在工位工作区跑 `cargo test --lib` 得 **2080**（≠ 净树 2079）
> ——他 session 的 #573 复审改动正在同一批文件上落盘。并发工位上的任何计数类验收，
> 必须先 `git archive` 出净树再测，否则读数归因不成立。
