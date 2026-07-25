# #269 实装证据：翻向守卫事件化（SPEC #268 T1）——事件谓词 + 判据改造 + 红环转绿

- 日期：2026-07-25
- 性质：TDD 实装证据落盘（票面验收逐项对照；红→绿生成史保留）。
- 实装基线 HEAD：`ff7b52026e`（#262 结案）。构建口径：`cargo test --release --lib`（红环）/ `cargo test --lib`（全量基线）。
- 裁定链：#153（数据浮出）→ #264（归因=接线缺陷，`onebar-prune-attribution-20260725.md`）→ #261（终裁=事件化 A）→ #268（SPEC）→ 本票。

## 1. 事件口径（实装第一件事：先枚举再写码）

塔事件流中「该载体的翻向结构事件」枚举（写入 `persistent.rs::direction_flip_event_active` 文档注释）：

| 候选事件类型 | 算不算翻向 | 依据 |
|---|---|---|
| **载体元素方向翻转**（同 ElementId 树元素被 frontier 重组改判反向 ⟺ 树段 upsert 方向冲突，`FlipGuardProbe::tree_blocked` 计数的那类） | **算（唯一一类）** | I2（anc.pdf §7）：同一持久元素方向不变 ⟹ registry 首见方向（I2 机器锁永固）≠ 当前树元素方向 当且仅当该冲突发生/持续；leg-2-98 勘察 (3,22) PREG Short→Long 单行形态 |
| candidate 段 upsert 冲突（`cand_blocked`：候选点元素 eps=σ vs 载体 ε） | **不算** | σ/ε 对立是 BSP 构造下出生即存在的两轴分层（买点恒附下降段末端 ⟹ σ=−ε 恒真，#264 §2.1 构造性证明），含 χ 域外候选 flip-flop——状态分层非事件 |
| 父元素 destroy+反向重建 | **不存在** | 前缀因果塔单调增长，无 destroy 概念（opsem_dump.rs:167 诚实缺席） |
| 载体退出树（Stale） | **不算** | 归 Stale 四态既有分派（persistent overlay §10 域），非翻向事件 |

**谓词**（`persistent.rs` 新增只读方法，零新字段）：`direction_flip_event_active(pid, tree_eps) ⟺ registry.elements[pid].dir ≠ tree_eps`（无条目 ⟹ false，诚实缺席不伪造杀）。**用事件不用状态轴**——谓词不读腿方向 σ（#264 未能判定②：ε 轴 id 重指/Stale 坑，26 笔幸存笔三态未分）；registry 首见方向由 #233 I2 机器锁永固，树段冲突是其唯一分歧源 ⟹ 分歧存在 ⟺ 翻向事件在场（含翻向持续期；翻回首见方向则分歧消失、事件不再激活——诚实口径）。

## 2. 判据改造（最小改动面）

- `coverage.rs::held_leg_tree_index_indexed`：守卫判据由 #233「`tree[idx].eps != leg.dir`（σ/ε 状态对立）」改为「`registry.direction_flip_event_active(&leg.id, tree[idx].eps)`（翻向事件在场）」；`held_leg_tree_index` 同签名贯通（registry 参数）。frontier/confirmed 同判（#227 焊缝规则保留）。
- 下游零改：Flipped 分派 → `flipped_seeds` → 并入关闭种子 → `step_active_set_with_subtree_close`（只作用 A_t 段）→ silent_drops 入轨链原样（探针 `held_flip_terminated` 语义随事件口径）。
- **不动**（票面约束）：入场侧、塔结构侧（tower_events 与基线**逐字节一致**，diff 实证）、P1–P8 通道语义、opsem dump 既有字段、环5 cert close 通道、#233 守卫意图（真翻向=终结不倒退）。
- 语义恢复：无事件的出生对立腿 Exact 存活，`element_as_leg` 随载体结构方向重登记（#233 前基线语义；事件在场时方向盲复活仍废除）。

## 3. TDD red→green（票面：守卫谓词单元测试两态）

两枚新测试（`coverage.rs` tests 模块，既有 #233 测试族之后）——**先红在案**（实装前跑出），**后绿**：

| 测试 | 两态 | RED（#233 状态轴守卫下实测） | GREEN |
|---|---|---|---|
| `birth_opposition_without_flip_event_does_not_terminate` | 出生对立（树 Short/腿 Long）**无事件** ⟹ 不杀 | probe=1、腿被剪（误杀复现）✓红 | probe=0、Exact 存活、重登记 dir=Short |
| `carrier_flip_event_terminates_even_without_direction_opposition` | 真翻向事件（首见 Long/树翻 Short）**无状态对立**（腿 σ=Short==树） ⟹ 杀 | probe=0、Exact 放行（事件盲区复现）✓红 | probe=1、腿终结 |

两态以「事件有无」而非「σ/ε 对立有无」判别——第二态即 #264 幸存笔 (b)「id 重指同向元素」形态，状态轴读不出、事件轴必杀。

**#233 既有 7 枚翻向测试零改全绿**（其 pre_flip registry 构型本身即「首见方向 ≠ 当前树方向」的事件痕迹，事件口径下同判）——已结算条款不倒退。

## 4. 红环（主缝，行为缝）

`/tmp/bug264_red_loop.sh`（wf8 单窗重放 + 断言 L1/L3 1-bar prune 占比 >50% 即红；产物 `/tmp/bug264_dump/`）：

| 读数 | 修复前（基线产物备份 `/tmp/bug264_dump_baseline_prefix/`） | 修复后 |
|---|---|---|
| L1 1-bar prune 占比 | 76/102 = **74.5% RED** | 51/123 = **41.5% ok** |
| L3 1-bar prune 占比 | 34/36 = **94.4% RED** | 10/37 = **27.0% ok** |
| L1/L3 持仓中位 | 1.0 / 1.0 bar | **38 / 132 bar** |
| **LOOP 判定** | **RED（EXIT=1）** | **GREEN（EXIT=0）** |
| 总笔数 / prune / 1-bar prune | 856 / 830 (97.0%) / 790 | 697 / 394 (56.5%) / 203 |
| tower_events | 基线 | **逐字节一致**（diff 实证） |

残余 203 笔 1-bar prune 全部走结构通道（事件谓词激活的翻向杀，或 Stale/子树连清/AncOK 既有路径——出生对立无事件杀在构造上已不可达）；其出场 bar 塔中枢事件命中率 0%（203/203）与旧误杀面同签名，但口径不同：塔事件流只记中枢事件（new_center/extend/level_upgrade），**方向翻向/ id 重指不产中枢事件**（#264 §5 覆盖性诚实边界同构）——该签名对此类杀无判别力，判别证据是谓词构造本身。

## 5. 40 笔保护集逐笔复核（票面回归缝 1）

旧产物 40 笔 hold>1 prune（`/tmp/bug264_dump_baseline_prefix` ≡ `/tmp/v4_C5_rerun_20260725` 同键集合，已互证）按 `(level,ordinal,entry_bar)` 对新产物逐笔对（脚本 `/tmp/bug264_review_40.py`）：

- **仍杀 37 笔**：同键在新产物仍 `via_structural_prune=true`。其中 24 笔 hold 不变或近变（如 L3#21 hold 8127 逐位不变、L2#122 hold 1394 逐位不变），13 笔新轨迹下 t+1 即杀（载体 id 首见方向分歧在出生时已激活——「出生在已翻向载体上」，与 (2,98) gen-10 同构，事件口径合法杀）。
- **放过 0 笔**。
- **判不出 3 笔**（如实列出，不静默放过也不静默杀）：`L0#101 entry=11761`（旧 hold=537 CloseRoot）、`L1#51 entry=25893`（旧 hold=172 CloseRoot）、`L0#664 entry=75926`（旧 hold=464 CloseShortDiff）——新轨迹下同 (voice,entry_bar) 入场未复现（轨迹分叉：载体被前序存活腿占用/候选湮灭，入场侧零改前提下的合法分叉），事件口径下无从判定，交编排者。

## 6. 错标面对账（票面：对照 617 照实报数）

「CloseRoot 且 `trigger_bsp_class_at_exit` 为空」计数：旧产物 **617** → 新产物 **271**（−56.1%）。残余 271 笔全部是结构 prune（silent_drops 轨设计上 trigger 恒 null，fill.rs:1576 注释）——但每一刀 now 有事件/结构路径可查（无事件不剪），错标面随误杀面收口而收敛；prune 余量 394 = CloseRoot 271 + CloseShortDiff 123。

## 7. 全量基线 + pnl 对照（照实报，回升是预期不是判据）

- `cargo test --lib`：**1919 passed / 0 failed**（基线 1917 + 2 枚新测试；136 ignored 重型不含）。三把 bit-exact 锁含于全量绿，未触发重算（锁内轨迹非本守卫可达面）。
- wf8 三产物对照（费前未杠杆口径，RATE_UNCALIBRATED 标签不动）：

| | 基线（#233前） | 旧 rerun（#233） | 新（#269） |
|---|---|---|---|
| 总笔数 / prune | 518 / 187 (36.1%) | 856 / 830 (97.0%) | 697 / 394 (56.5%) |
| L0 中位 hold / pnl | 209.5 / +479.17 | 1.0 / +495.26 | 134.0 / +7584.50 |
| L1 中位 hold / pnl | 157.5 / −3810.33 | 1.0 / +62.68 | 38.0 / −2644.00 |
| L2 中位 hold / pnl | 340.0 / −292.99 | 1.0 / −38.18 | 142.5 / +9177.01 |
| L3 中位 hold / pnl | 255.0 / **+10985.85**（23 笔） | 1.0 / +3026.13（36 笔，−72.5%） | 132.0 / **+11053.32**（37 笔，≈基线 100.6%） |
| 全窗 pnl | +7361.70 | +3545.89 | +25170.83 |

L3 长持引擎恢复实证（票面「pnl 应大部分回升」：+3026 → +11053，回到基线水位）；持仓中位未完全回到基线（L1 38 vs 157.5、L3 132 vs 255）——差距=残余事件合法杀 + 轨迹分叉，照实标注不粉饰。L1 pnl 为负（基线同号为负：−3810→−2644），照实报。

## 8. 未能判定/诚实边界

1. **3 笔保护集判不出**（§5 清单在案）——入场未复现，交编排者。
2. **事件激活窗口口径**：谓词对「腿出生前载体已翻向（首见方向分歧出生即激活）」同判杀（(2,98) gen-10 形态——出生在已终结结构上，#233 已结算行为保留）；「腿存活期翻向」本义杀自不待言。两类的逐笔分桶需 per-bar registry 快照，现产物不含，未分桶（对红环断言无影响——两者皆非「出生对立无事件」）。
3. **残余 203 笔 1-bar prune 的逐笔通道分桶**（翻向事件杀 vs Stale/连清/AncOK）未做——红环断言以占比口径验收已过；逐笔需探针落盘，另案。
4. **理论边**：`merge` step 3' held/op_parent `or_insert` 以 `leg.dir` 首见登记（该 pid 此前从未树段 upsert 时才可达）——若该 pid 后于树出现且方向不同会产伪分歧；生产可达性未实证排除（op_parent 于子腿出生 bar 必在树 ⟹ 实际不可达的论证在案），wf8 数字无异常签名。
5. **L2 级 pnl +9177（16 笔）与 L0 +7584 为轨迹分叉后新形态**，与基线不可逐笔对；重型套件（M7/M8 多品种）未跑（用户裁定），BTC train 三锁以 `cargo test --lib` 全绿覆盖。
6. 旧 26 笔非 prune 幸存笔去向（附带观察）：仍非 prune 16、转 prune 6（id 重指同向=事件合法杀，即测试②形态）、入场未复现 4。

## 9. 变更清单（commit 只含这三件）

- `rust/src/theta_v0/strategy/persistent.rs`：`direction_flip_event_active` 谓词（+文档枚举，零新字段零行为改）。
- `rust/src/theta_v0/strategy/coverage.rs`：守卫判据事件化（`held_leg_tree_index`/`held_leg_tree_index_indexed` 贯通 registry）+ 注释同步 + 两枚新测试。
- 本报告。分析脚本 `/tmp/bug264_review_40.py`（未落仓）。

*report 完。红→绿生成史、判不出项、理论边均如实保留。*
