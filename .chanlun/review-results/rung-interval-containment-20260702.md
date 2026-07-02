# 区间套问题①：跨级 rung 定位端点相等 vs 区间包含（阶段0 诊断裁决）

**任务**：#77（区间套问题①修复）
**认识论等级**：L2（真实 BTC 350K bar 逐信号两口径对照，可产否定性结果）
**结论**：**NO-SHIP**（阶段0 诊断否证问题① 归因，不改生产代码）
**源权威**：`docs/formal-chain/区间套.pdf`（编排者 2026-07-01 最后下载=链最新端点）

---

## 一、阶段0 诊断裁决（决定性 L2 结果）

区间套.pdf §一 4 裁定：跨级 rung 定位应从端点相等 `end_index==source` 改为区间包含 `J_child⊆J_parent`，因为端点相等是区间包含的**真子条件**（`s=ρ(m)⟹s∈I(m)`，反之不成立），会产生系统性 false negative，"depth≥2 恒 0 正是这种 false negative 的系统性表现"。

任务要求先测量再修：**若两口径差异=0 则 NO-SHIP**。

**探针**：`h2_sample_exclusion_dx`（新增 §2b 两口径对照块），350K BTC（窗口 2025-09-30→2026-05-31，全量=4613599），level 1-4 buy2/sell2 信号 n=127。对每信号在同一 `exec_moves=tower[lvl]` 上并列计算：
- 端点相等 `find_move_by_end_index(tower[lvl], src)`
- 区间包含 `tower[lvl].iter().any(|m| m.start_index<=src && src<=m.end_index)`

| 口径 | 命中数 | 占比 |
|---|---|---|
| 端点相等 find_move_by_end_index(tower[lvl],src) | 127 | 100.00% |
| 区间包含 start≤src≤end（同 exec_moves） | 127 | 100.00% |
| **false negative（包含命中∧端点未命中）** | **0** | **0.00%** |
| ↳ level 1 / 2 / 3 / 4 FN | 0 / 0 / 0 / 0 | |

**两口径 bit-identical**：base 级端点相等 100% 命中，区间包含 100% 命中，二者无一条差异。全 level 1-4 FN=0。

**跨级/子级补充证据**（同报告既有块，未受本改动影响）：`no_upper=0`（tower[lvl+1] 区间包含找父恒成功）、`no_target=0`（knode.sub_moves 端点相等找子恒成功）。即问题①点名的四处 site 语境——L560/L632（descend 子级）、L736/L1163（base 执行级）——两口径在真实数据上**全部一致**。

## 二、根因订正：depth≥2=0 不是端点相等伪影

问题①假设 base_none 主导是端点相等系统性漏检所致。诊断**否证**该归因：

- base 级端点相等 100% 命中（127/127）⟹ `build_nest_certificate` 返回 None 不是因为执行级 `find_move_by_end_index`（L736/L1163）失败。
- base_none 主导实由 descend 段2 `descend_type1_anchor_depth` 返回 None = **真小转大**（次级别无 Type1 背驰锚点，第29课 L396 定律一下沉无一类精确点）。这是结构性判据，非口径伪影。

区间套.pdf 之所以预期 false negative，前提是 `source` 会落在段**内部**（`t_{j-1}<source<t_j`）。真实数据反例：`source_index` 是信号检测点（背驰/买卖/分型端点），由塔构造（连续 sub_moves compose，父段右端=末子段右端）保证它**恒为各级走势段右端点**。故端点相等在本系统的定位对象上不 over-strong——退化为与区间包含等价。

## 三、结构性保证澄清（为何区间包含已内置）

descend 路径的 `[J_{ℓ-1}⊆J_ℓ]` 由 recursive_tower Compose 不变量**结构性保证**（sub_move 的 `[start,end]⊆parent[start,end]`，econ_positive.rs L544-545 注释）。端点相等只是在这些「已结构包含」的 sub_moves 中选锚，探针证明该选择与点包含从不分歧。PDF §二 警告的"先全局 Sel 再检查包含"反模式在本实装不存在——候选本就是结构连续分解（良式分解），点定位天然唯一（PDF §三 1）。

---

## 结果包六要素

1. **结论**：NO-SHIP。350K BTC 上端点相等与区间包含两口径对 base/子/父三层 rung 定位 bit-identical（FN=0，命中 127/127）。区间套.pdf 问题① 的端点相等 false negative 归因**在真实数据上被否证**；端点相等结论获区间包含加固。depth≥2=0 是真小转大（descend 段2 无 Type1 锚），非本口径伪影。不改 econ_positive.rs/nest.rs 生产代码。
2. **定义依据**：区间套.pdf §一 4「rung 应区间包含」+ 定理2「Γ_old⊊Γ_new」。探针实测 `find_move_by_end_index`（端点相等）与 `start≤src≤end`（区间包含）命中集合相等 ⟹ 本系统 `Γ_old=Γ_new`（真子集退化为相等），因 source_index 恒为各级段右端点（塔 Compose 不变量）。
3. **边界条件**：若换品种/换窗口出现 `base_false_neg>0`（source 落段内部），则问题① 归因复活，须按 §一 4 改区间包含 bottom-up 定位（先定 J_{k-1} 再在父候选找包含它的 J_k，唯一性由良式分解或 Sel_Θ 保证）。当前 BTC 350K 无一例。探针已常驻 `h2_sample_exclusion_dx §2b`，跨品种回归自动复检。
4. **下游推论**：三条「depth>0 归零」链中，端点相等定位链（区间套.pdf 问题①）在 BTC 上**不成立**——depth≥2=0 的伪影嫌疑转由身份层链（anc.pdf，已修 persistent overlay）与 frontier 链（问题②，未修）承担。C3 level==1 关闭（0/84）不受本口径影响（C3 用 `start_index>=source` 新中枢突破，非端点相等 rung）。「高级别无 alpha」结论的端点相等污染项**可移除**；frontier 污染项仍在。
5. **谱系引用**：606（区间套有效域=Type1）、534（nested-recursion-accounting）、`project_interval_nesting_not_called_in_backtest`（95%退化 depth=1，本报告给出 depth≥2=0 的机制=小转大而非口径）、`project_gap3_l2_unreachable_architecture`、`project_level_hole_window_dependence`（C3 有效域=level0）。dlpdf-d-structure-20260702.md §一 1 缺口即报——本报告为其阶段0 收口（否证）。建议 genealogist 评估是否新增「端点相等 rung 在 source=段端点系统上退化为区间包含等价」条目。
6. **影响声明**：改动**仅 test 内**——`econ_positive.rs::h2_sample_exclusion_dx` 新增两口径对照计数器 + §2b 报告块（诊断探针，常驻用于跨品种回归）。无生产符号变更，无 nest.rs 改动。护航：`cargo test --release --lib econ_positive` 33 passed 0 failed（+7 ignored）；探针 test 通过。dx 守恒/Nest 862/三件套 depth 分布**未改**（无生产口径变化 ⟹ 无信号集变化，非重跑对象）。
