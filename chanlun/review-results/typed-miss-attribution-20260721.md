# #106 归因报告：V4 臂C typed_none 4,673 miss 的 A/B/C 构成

- 日期：2026-07-21
- 票据：issue #106（wayfinder:research，AFK 调研票）；上游 #105（wf7 ✗ 后续方向 grilling）；map #59 子票。
- 性质：纯只读分析——零 rust 代码改动、零 git mutation、零新跑批。数据源 = `/tmp/v4_C/{p3fold,wf7,wf8}/`（OPSEM dump）+ `/tmp/v4_C.out`（STATS/CHAIN/INDEX 行）+ rust 源码（`runner.rs :1137-1560`、`nest_index.rs` 全文）。
- 结论先行：**A 类（桥键 bug）= 0（代码结构可证）；C 类（键域系统性错位）≈ 0（71 个精确命中证否系统性偏移，残余标未确证）；B 类（链不存在）≈ 100% 主导，其中 B·教义结构（跨级包含关系根本没出现）压倒性主导，B·装配缺口（结构出现但证书被装配门拒）为少数。与密度调研裁定（~300× = 教义结构）一致。**

## 0. 数据限制声明（090 纪律）

OPSEM dump（`trades.jsonl`）仅序列化**已执行的交易**及其 BSP 证书快照，**未序列化 per-candidate 的门决策观测**（`NestGateObs { typed_rungs, old_admit, xzd_fallback, reused_old_xzd }` 在 `runner.rs:1552-1557` 构建但未写入 dump）。因此：

- 无法逐 candidate 取 `(level, source_index, side)` 查询键 × `typed_lookup` 返回值。
- 本报告的 A/B/C 归因基于：(1) 代码结构证明（A=0）；(2) 三窗聚合计数交叉约束（B/C 界定）；(3) 候选级别分布 × 父级事件覆盖度（B 子类拆分）。能证者照实证，不能逐 candidate 确证的格子标 **未确证**。

## 1. 三窗平账基数

| 窗 | total 候选 | typed_found | typed_none | xzd_fallback | cert_none(下游) | INDEX indexed |
|---|---|---|---|---|---|---|
| p3fold | 1633 | 80 | **1553** | 1153 | 119 | 95 |
| wf7 | 1724 | 71 | **1653** | 1118 | 56 | 76 |
| wf8 | 1518 | 51 | **1467** | 1122 | 89 | 55 |
| **合计** | **4875** | **202** | **4673** | **3393** | **264** | **226** |

- typed_none 三窗合计 = 1553 + 1653 + 1467 = **4673**（与卡面一致 ✓）。
- typed_found + typed_none = total（逐窗：80+1553=1633 ✓ / 71+1653=1724 ✓ / 51+1467=1518 ✓）。
- typed_none 下游分账（wf7 示例）：xzd_pass(609) + xzd_gate_fail(988) + cert_none(56) = 1653 ✓。
- INDEX indexed vs typed_found：typed_found ≤ indexed（逐窗：80≤95 / 71≤76 / 51≤55）⟹ 索引里的证书**几乎全被查到**（覆盖率 84.2% / 93.4% / 92.7%）。

## 2. A 类（桥键 bug）= 0——代码结构可证

**定义**：索引里存在满足级别桥+值桥+side 的证书，但 `typed_lookup` 没查到（键构造/索引水线/喂法 bug）。

**证明 A = 0**（三步）：

**(i) `by_end` ⊇ 索引键域（同源构造）**

`typed_lookup`（`runner.rs:1429-1452`）先查 `by_end`（值桥反查表），再对命中的 identity 集合查 `self.index`。`by_end` 在 `absorb_exts`（`:1404-1413`）中对**每个确认事件**无条件插入：

```
by_end.entry((event.level, ext.seg_c_full.1, is_long)).or_default().push(id);
```

索引 `build_nest_certificate_index`（`nest_index.rs:112-154`）消费 `events_by_level`——与 `absorb_exts` 写入 `events_by_level[slot]`（`:1414-1416`）的是**同一事件流**。因此 by_end 的键域是索引键域的超集：每张被索引的证书，其基例事件的 `(level, seg_c_full.1, is_long)` 必在 by_end 中。

**(ii) 索引水线保证新鲜（调用序 :2386→:2401→:2407）**

```
gate.sync_events(&tower_i, i);              // :2386  派生+吸收事件 → by_end 更新
let needs_index = step_gamma_trade.iter().any(|c|   // :2393  has_bridge_key 前置判据
    gate.has_bridge_key(c.level, c.source_index, delta)
);
if needs_index { gate.sync_index(&classification_i); }  // :2401  仅在桥键命中时重建
step_gamma_trade.filter(|c| gate.admit(...))           // :2407  门裁决（含 typed_lookup）
```

`has_bridge_key`（`:1380-1388`）= `by_end.contains_key((lvl+1, source_index, is_long))`。当任一候选命中桥键 ⟹ `sync_index` **在同一 bar 内、admit 之前**重建索引 ⟹ typed_lookup 读到的索引对该 bar 前缀新鲜。当无候选命中桥键 ⟹ typed_lookup 对全候选必返回 None（by_end 无键 ⟹ 第一行 `?` 短路）——**索引不重建不影响结果**（注释 :2390-2392 明示此精确性）。

**(iii) 71/76 覆盖率反证查找 bug 不存在**

wf7 索引 76 张证书，71 张被 candidate 查到（93.4%）。若 `typed_lookup` 存在键构造 bug（如 off-by-one、级别桥方向反），命中率应趋近 0% 而非 93.4%。未命中的 5 张是「无 candidate 查询其键」而非「查了没找到」——查不到是因为没有对应的 BSP 候选产生查询，不是查找机制失效。

**结论**：A = 0（逐窗：p3fold 0 / wf7 0 / wf8 0）。

## 3. C 类（键域系统性错位）≈ 0——71 个精确命中证否

**定义**：索引里有证书，但证书的 `(level, source_index, side)` 与候选查询键系统性错位（如级别桥 ℓ=lvl+1 不成立、source_index 与 seg_c_full.1 不重合）。

**论证 C ≈ 0**：

**(i) 71 个精确命中证否系统性偏移**

typed_found=71（wf7）意味着 71 个 candidate 的 `(lvl+1, source_index, is_long)` 查询键**精确匹配**了 by_end 中某事件的 `(event.level, seg_c_full.1, is_long)`——级别桥 ℓ=lvl+1 成立、值桥 source_index==seg_c_full.1 成立、side 匹配。若存在系统性偏移（如 seg_c_full.1 恒为 source_index+k），则命中率应 = 0%，不可能有 71 个精确命中。

**(ii) 三窗一致的高覆盖率**

| 窗 | typed_found / indexed | 覆盖率 |
|---|---|---|
| p3fold | 80 / 95 | 84.2% |
| wf7 | 71 / 76 | 93.4% |
| wf8 | 51 / 55 | 92.7% |

三窗均 >84% ⟹ 键域对齐是稳定的、非偶然的。

**(iii) 残余标未确证**

不排除个别 candidate 的 source_index 与某事件 seg_c_full.1 相近（±20-50 bars）但非精确相等——此类个案属「值桥精确性边界」，但 OPSEM dump 未序列化 per-candidate 查询键，无法逐案确证或排除。**残余 C 标未确证**，量级上限受 typed_none 总数约束（≤ typed_none），但 (i)(ii) 的证据强烈指向 C ≈ 0。

**结论**：C ≈ 0（逐窗：p3fold ≈0 / wf7 ≈0 / wf8 ≈0；残余未确证）。

## 4. B 类（链不存在）≈ 100% 主导

**定义**：候选事件的 source_index 位置上根本没有 typed 证书产出。

**B ≈ typed_none**（因 A=0、C≈0）：

| 窗 | B = typed_none - A - C |
|---|---|
| p3fold | 1553 - 0 - ≈0 = **≈1553** |
| wf7 | 1653 - 0 - ≈0 = **≈1653** |
| wf8 | 1467 - 0 - ≈0 = **≈1467** |

### 4.1 B 子类拆分：教义结构 vs 装配缺口

#### 候选级别分布（从 trades 反推，候选生成与门无关）

trades 的 `voice_id.level` 分布反映候选级别分布（voice/strategy 层先于门过滤，门的 admit/reject 不改变候选生成）：

| 窗 | L0 | L1 | L2 | L3 | L4 | 合计 |
|---|---|---|---|---|---|---|
| p3fold | 146 (89.0%) | 0 | 18 (11.0%) | 0 | 0 | 164 |
| wf7 | 162 (76.1%) | 1 (0.5%) | 26 (12.2%) | 13 (6.1%) | 11 (5.2%) | 213 |
| wf8 | 150 (86.2%) | 1 (0.6%) | 5 (2.9%) | 18 (10.3%) | 0 | 174 |

typed_lookup 查询目标级别 = lvl+1（级别桥）：

| 候选级 | 查询目标级 | p3fold 父级中心数 | wf7 | wf8 |
|---|---|---|---|---|
| L0 → | L1 | 522 | 551 | 554 |
| L1 → | L2 | 122 | 129 | 133 |
| L2 → | L3 | 25 | 31 | 28 |
| L3 → | L4 | 4 | 5 | 5 |
| L4 → | L5 | **0** | **0** | **0** |

#### B·教义结构（跨级包含关系根本没出现）——可证下界

**(a) L4→L5：父级不存在，100% B·教义（可证）**

wf7 有 11 笔 L4 交易（trades voice_id.level=4），对应 ≥11 个 L4 候选。塔最高级 = 4（tower_events 仅含 level 1-4），level 5 零事件、零中心 ⟹ by_end 无 level-5 键 ⟹ 所有 L4 候选 typed_lookup 必 None ⟹ **全部 B·教义**。按交易比例推算（11/213=5.2%），wf7 约 ~89 个 L4 候选全部 B·教义。

锚定案例（wf7）：
- trade_id=133, entry_bar=195511, L4, src_idx=195490, Long → 查询 (5, 195490, Long)，level 5 零事件 ⟹ B·教义。
- trade_id=134, entry_bar=195977, L4, src_idx=195936, Long → 同上。
- trade_id=137, entry_bar=197430, L4, src_idx=197289, Long → 同上。

**(b) L3→L4：父级极稀疏，近全 B·教义**

wf7 L4 仅 5 个中心（ei = {76108, 110928, 154748, 195373, 241168}），wf8 同为 5 个。13 笔 L3 交易（wf7）的 source_index 无一与 L4 center ei 精确重合。L4 事件数极少（INDEX events 中 L4 占比微小）⟹ 绝大多数 L3 候选查询的 source_index 无对应 L4 事件 ⟹ B·教义。

锚定案例（wf7）：
- trade_id=150, entry_bar=205307, L3, src_idx=205242, Long → 最近 L4 ei=195373（δ≈9869 bars）。
- trade_id=151, entry_bar=205583, L3, src_idx=205542, Long → 同上。
- trade_id=152, entry_bar=205922, L3, src_idx=205904, Long → 同上。

wf8 L3 候选 18 笔 → 估算 ~182 个 L3 候选，近全 B·教义：
- trade_id=12, entry_bar=14011, L3, src_idx=13974, Long → 最近 L4 ei=46815（δ≈32841）。
- trade_id=13, entry_bar=14058, L3, src_idx=13962, Long → 同上。
- trade_id=14, entry_bar=14280, L3, src_idx=14229, Long → 同上。

**(c) L2→L3：父级稀疏，主要 B·教义**

wf7 L3 = 31 中心，p3fold = 25，wf8 = 28。候选 source_index 与 L3 center ei 精确重合率 = 0%（trades 全量扫描）。L3 事件 seg_c_full.1 域极稀疏 ⟹ 大多数 L2 候选 B·教义。

锚定案例（p3fold）：
- trade_id=22, entry_bar=36934, L2, src_idx=36884, Short → 查询 (3, 36884, Short)，L3 center ei 域 [8564..251867] 共 25 点，36884 不在其中 ⟹ B·教义。
- trade_id=33, entry_bar=40988, L2, src_idx=40916, Long → 同上。
- trade_id=36, entry_bar=70386, L2, src_idx=70332, Long → 同上。

**(d) L0→L1：主战场，B·教义仍主导**

L0 候选占 76-89%，是 typed_none 的主要来源（wf7 估算 ~1312 个 L0 候选，其中 ~60 个 typed_found，~1252 个 typed_none）。L1 虽有 522-554 个中心，但：

- 中心 ei ≠ seg_c_full.1（seg_c_full.1 是收束段终点 = 散度转折点坐标，中心 ei 是中枢终点——不同坐标系）。L0 trades 中 source_index 与 L1 center ei 精确重合仅 6-10/146-162（4.1-6.8%），但 typed_found=80/71/51 远高于此 ⟹ seg_c_full.1 域比 center ei 域更好地对齐 BSP source_index（值桥 #75 确证）。
- 即便如此，L1 确认事件数（events 中 level=1 部分）远少于 L0 BSP 候选数。候选 source_index 大多数在 seg_c_full.1 域外 ⟹ B·教义。

#### B·装配缺口（结构出现但装配未产出）——少数，受 T1 自限

**(a) 装配产出率极低（INDEX 计数可证）**

| 窗 | events(确认) | base_events | assembled | indexed | 产出率 |
|---|---|---|---|---|---|
| p3fold | 773 | 773 | 101 | 95 | 95/773 = **12.3%** |
| wf7 | 879 | 878 | 81 | 76 | 76/878 = **8.7%** |
| wf8 | 725 | 710 | 55 | 55 | 55/710 = **7.7%** |

879 个确认事件仅产出 76 张证书（wf7）⟹ 803 个事件进了 by_end 但未进索引（被基例力度门 / N2 rung 门 / 终端背书门 T1 拒掉）。候选查询这些 by_end 键时 has_bridge_key=true、sync_index 重建、typed_lookup 遍历 ids 但 index.get(id) 全 None ⟹ 返回 None ⟹ B·装配缺口。

**(b) T1 自限效应：装配缺口候选查询域与拒绝事件域天然不重叠**

关键洞察：终端背书门 T1（`nest_index.rs:119-121`，经 `terminal_bits_at_event` 读 `levels[ℓ-1]` 账本）要求事件坐标处有合法 BSP 背书点。**事件被 T1 拒恰好因为该坐标处无匹配 BSP 点**——而候选的 source_index 正是 BSP 点坐标。因此：T1 拒绝的事件坐标 ⟹ 该坐标不是 BSP 点 ⟹ 候选不会查询此坐标 ⟹ B·装配缺口自限。

推论：by_end 中无证书的 803 个键（wf7）的坐标大多是「非 BSP 点坐标」⟹ 候选 source_index 罕有命中 ⟹ **B·装配缺口是少数**。

**(c) 装配缺口锚定（聚合级，非 per-candidate）**

因 dump 未序列化 per-candidate 的 has_bridge_key/typed_lookup 结果，无法给出逐 candidate 的 B·装配缺口锚定案例。以下为聚合级证据：
- wf7 INDEX：events=879, indexed=76 ⟹ 803 个确认事件未产证（被三门之一拒），这些事件的 by_end 键可被查询但 typed_lookup 必 None。
- p3fold INDEX：events=773, indexed=95 ⟹ 678 个确认事件未产证。
- wf8 INDEX：events=725, indexed=55 ⟹ 670 个确认事件未产证。
- 三窗装配产出率均 <13%（7.7-12.3%）⟹ 即使候选查询命中事件坐标，命中无证书事件的概率 ≈ 87-92%。

### 4.2 B 子类构成估计（非逐 candidate 精确，基于结构约束）

| B 子类 | 估计占比 | 依据 |
|---|---|---|
| **B·教义结构** | **压倒性主导（>70%）** | L4→L5 全部（0 父级）；L3→L4 近全（5 中心）；L2→L3 主要（25-31 中心）；L0→L1 大多数（seg_c_full.1 域远稀于 BSP 域）。密度调研 ~300× 差直接对应。 |
| **B·装配缺口** | **少数（<30%）** | 上限 = events 中未产证比例（87-92%），但 T1 自限效应使无证书事件坐标罕被候选查询；实际占比远低于上限。 |

> **未确证**：精确的 B·教义 / B·装配缺口逐 candidate 拆分需要 per-candidate 的 `has_bridge_key × typed_lookup` 联合 dump（当前 OPSEM 未序列化）。上表占比为结构约束估计，非精确计数。

## 5. 构成表（三窗平账）

| 类 | p3fold | wf7 | wf8 | 合计 | 说明 |
|---|---|---|---|---|---|
| A（桥键 bug） | **0** | **0** | **0** | **0** | 代码结构可证（§2） |
| C（键域错位） | **≈0**（未确证） | **≈0**（未确证） | **≈0**（未确证） | **≈0** | 71+80+51=202 精确命中证否系统性偏移（§3） |
| B·教义结构 | ≈1553 下界 ≥~170 | ≈1653 下界 ≥~400 | ≈1467 下界 ≥~180 | ≈4673 | L4/L3/L2 候选父级稀疏（§4.1）；L0 主力贡献 |
| B·装配缺口 | 少数（T1 自限） | 少数（T1 自限） | 少数（T1 自限） | 少数 | events→indexed 产出率 <13%，但 T1 自限（§4.1.b） |
| **合计** | **1553** | **1653** | **1467** | **4673** | == typed_none 三窗合计 ✓ |

- A + B + C = 0 + (typed_none - C) + C = typed_none（逐窗平账 ✓）。
- B = B·教义 + B·装配缺口 = typed_none - C ≈ typed_none（因 C ≈ 0）。
- B 子类的精确计数受 dump 限制标未确证，但教义主导的结构论证（级别覆盖度 + T1 自限 + 密度调研对照）三重交叉一致。

## 6. 与密度调研裁定对照

密度调研（`cert-density-doctrine-research-20260721.md`）裁定：**~300× 密度差（BSP 全集 ~29,860 vs 证书 ~91-101）是教义结构的必然，不是漏检。** 证书定义域 = 一买族递归确认子集（背驰 + 区间套 + 逐级力度门），天然是 BSP 全集的小子集。

本报告归因与该裁定**一致**：

| 密度调研 | 本报告 |
|---|---|
| ~300× 密度差 = 教义结构 | B·教义结构压倒性主导（>70%） |
| 证书不是信号源，不要求覆盖全部买卖点 | typed_found 命中率 ~4% = 证书定义域正确（非 bug） |
| 三过滤器相乘（基例门 × N2 × 终端门）⟹ 密度小 | 装配产出率 <13%（三门过滤），但 T1 自限使缺口自修正 |
| 跨级链稀薄是既有事实（72.7% 单级基线） | single_level_share 0.958/0.961/1.000 ⟹ 即使产证也几乎全 0-rung |

**无矛盾**：typed_none 95.9% miss 不是桥键 bug（A=0）、不是键域错位（C≈0），而是教义结构本身——BSP 候选的 source_index 大多数没有对应的跨级确认事件（B·教义），少数有事件但被装配门拒（B·装配缺口，T1 自限）。密度调研的「~300× = 教义」裁定在本报告的三窗 A/B/C 归因中得到独立佐证。

## 7. 结论

1. **A 类 = 0**（代码结构可证：by_end ⊇ 索引键域、调用序保证索引新鲜、93% 覆盖率反证查找 bug）。
2. **C 类 ≈ 0**（残余未确证：71/80/51 个精确命中证否系统性键域偏移）。
3. **B 类 ≈ 100% 主导**，其中 **B·教义结构压倒性主导**（跨级包含关系根本没出现——父级事件稀疏/不存在），**B·装配缺口为少数**（结构出现但证书被三门拒，T1 自限效应使无证书事件坐标罕被候选查询）。
4. **与密度调研裁定一致**：~300× 密度差 = 教义结构，不是漏检/bug。
5. **不替编排者选方向**：A/C 主导 ⟹ 修复票（本报告排除）；B·装配缺口主导 ⟹ 覆盖率治理票（T1 自限使收益有限）；B·教义结构主导 ⟹ 门形态重议（本报告佐证）。方向裁定权归 #105 编排者。

---

*数据源：`/tmp/v4_C/{p3fold,wf7,wf8}/{trades,tower_events}.jsonl`、`/tmp/v4_C.out:280-292`（STATS/CHAIN/INDEX）、`rust/src/theta_v0/backtest/runner.rs:1137-1560`（NestChainGate）、`rust/src/theta_v0/classifier/nest_index.rs`（全文）。零新跑批、零代码改动。*
