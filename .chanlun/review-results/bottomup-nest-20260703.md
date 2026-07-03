# bottomup-nest 区间套递归：阶段0 对拍 + 等价固化（NO-SHIP）

**工位**：swarm/ws-bottomup（task #101） · **日期**：2026-07-03 · **数据**：BTC 单标的真实全历史（btc_1m_full.json，全量 4,613,599 bar）

**规格权威**：`docs/formal-chain/回测的问题.pdf`（编排者令：全项照做，PDF 即裁决书）——本工位覆盖**问题①**（端点相等 vs 区间包含 / bottom-up 区间套）。问题②（增量 frontier 重算）不在本工位范围。

---

## 0. TL;DR

现行生产 descend 口径与 PDF §二「最严格实现 = bottom-up」在真实 BTC 上**逐信号 bit-exact 等价**，三窗（50K/300K/350K）差异全 0（含 d2–d4 真跨级 case）。**结论：NO-SHIP**——生产 descend 无需改，等价由新增 parity 测试的 `assert` 机器固化。这与 PDF §三.1「若塔是良式分解，区间包含定位天然唯一」的定理一致。

---

## 1. 结论

现行生产 `build_nest_certificate`（rust `econ_positive.rs`）的 rung 锚定用 **source_index 点包含**（`start_index ≤ source_index ≤ end_index`，`partition_point` 定位含点段）。PDF §二要求 **bottom-up 子区间包含** `J_{k-1} ⊆ I(c)`（`c.start ≤ child.start ∧ child.end ≤ c.end`）+ Sel_Θ 作用于「包含 child 的候选集」+ child 逐级加宽。

**两口径在 recursive_tower Compose 塔上逐信号 bit-exact 等价**，实测差异全 0：

| 窗口 | 到达 cert 候选 n | cert=Some（生产/BU） | Γ 规模（生产/BU） | Γ 差异 | 深度差异 | rungs.len 差异 | 结构差异 |
|---|---|---|---|---|---|---|---|
| 50K（2026-04-27→05-31） | 142 | 134 / 134 | 134 / 134 | 0 | 0 | 0 | 0 |
| 300K（2025-11-04→2026-05-31） | 827 | 732 / 732 | 732 / 732 | 0 | 0 | 0 | 0 |
| 350K（2025-09-30→2026-05-31） | 987 | 869 / 869 | 869 / 869 | 0 | 0 | 0 | 0 |

有效深度分布两口径**完全一致**（350K）：`d0=564 d1=233 d2=54 d3=14 d4=4`（生产=bottom-up 逐桶相等）。**深度 ≥1 占比 35.5%**（305/869），含 d4 真四级跨级——即便这些真区间套 case，两口径也 bit-exact，不是「全退化为 base-case 所以没差异」的平凡等价。

**实装决定**：不改生产（NO-SHIP）。以 `#[ignore]` parity 测试 `acc_bottomup_nest_parity_probe` 固化等价——测试内含 PDF §二完整 bottom-up 实现 `build_nest_certificate_bottomup`（cfg(test)），与生产逐信号对拍并 `assert` 全 0 差异。将来若塔构造改动破坏 refinement，本测试转红，逼出 bottom-up 生产实装（不静默把非严格 refinement 当等价）。

## 2. 定义依据

- **PDF §二**（p4）：`C^δ_k(J_{k-1},t) = {c ∈ C^δ_k(t) : J_{k-1} ⊆ I(c)}`；`Cand^δ_k := [C^δ_k(J_{k-1},t) ≠ ∅]`；`c_k = Sel_Θ(C^δ_k(J_{k-1},t))`；`J^δ_k = I(c_k)`。**Sel_Θ 必须作用在「包含 child interval 的候选集」上**（p4 反例：先全局 Sel 再检包含 → false negative）。
- **PDF §三.1**（p5）：良式分解（`I(m_j)=(t_{j-1},t_j]`，`a=t_0<t_1<…<t_r=b` 连续无缝分解）下，任意内点 `s∈(a,b)` 存在**唯一** j 使 `s∈I(m_j)` ⟹「区间包含定位天然唯一」。
- **PDF §五裁决 (b)**（p8）：改成区间包含后，唯一性由良式分解或 Sel_Θ 保证；`J_child ⊆ J_parent` ⟺ `start(parent)≤start(child) ∧ end(child)≤end(parent)`。
- **等价的结构根据**：`recursive_tower` 的 tower[k] 是 tower[k-1] 连续 `sub_moves` 的 `Compose`（refinement 分区，tower[k] 边界 ⊆ tower[k-1] 边界）。故 (i) 含 source_index 的 tower[k] 段唯一 ⟹ 点包含 = 区间包含（含点段即含子段）；(ii) 含 child 的候选集是单元素 ⟹ Sel_Θ 平凡；(iii) `is_sub`（`n_delta` 内区间包含校验）由 Compose 不变量自动满足。三点合起来 ⟹ 点包含 descend 逐信号等于 bottom-up 区间包含 descend。实测三窗差异全 0 是该结构论断的 L2 经验确认。

## 3. 边界条件（结论翻转条件）

结论「NO-SHIP，两口径等价」在以下条件翻转（parity 测试的 assert 即这些条件的机器守卫）：

1. **塔非严格 refinement**：若某处 tower[k] 段边界严格落在 tower[k-1] 段内部（straddle），则执行段/子区间可跨 tower[k] 边界 ⟹ 点包含选到含点段而区间包含找不到含段（链断）⟹ `n_some_mismatch>0` 或 `n_gamma_diff>0`。此时须实装 bottom-up 入生产（PDF §五 b）。
2. **候选重叠（非分区）**：若 tower[k] 从「分区段」改为「重叠候选背驰段」（PDF §三.2 场景），含 child 候选集 |C|>1 ⟹ Sel_Θ 非平凡 ⟹ 点包含（取含点 leftmost）可能 ≠ Sel_Θ 选择 ⟹ 差异>0。
3. 三窗均为 BTC 单标的 L2，**有效域 = BTC**。跨品种（CL 等）refinement 若在别处破裂，差异可非 0——需 L3 复测方能声明品种无关等价（本工位未做，见影响声明）。

## 4. 下游推论

- **「高级别无 alpha」图景不被本工位改变**：Γ 集合、depth 分布两口径逐桶相等 ⟹ 现有基于生产 descend 的 acc-highlevel-mu / depth-distribution 结论**无需重跑**（bottom-up 不产生新信号、不改深度）。（对照 PDF §十裁决：问题②的 frontier-bug 才可能污染高级别计数——那是另一工位，不在此。）
- **区间套「真触达 ≥2 层」的 P1 验收**：350K 实测 depth≥2 占 8.3%（72/869），depth≥1 占 35.5%——两口径一致。此为 `effective_nest_depth` 的 bit-exact 旁证，非本工位新结论。
- **问题① 闭环**：配合 task #100 的两条验收测试（合成 `J0⊂J1⊂J2` 且 `end(J1)≠source(J0)` 分歧案例 + 每成功 cert 的 `debug_assert` 嵌套链）+ 本工位的真实数据等价固化，PDF §十二问题①测试建议全覆盖。

## 5. 谱系引用

- MEMORY `interval-nesting-not-called-in-backtest`：「N^δ 真调用生产但 BTC 实测 95% 退化 base-case；跨级仅 4.6% max 深度=1」——本工位 350K 窗测得 depth≥1 占 35.5%、max 深度=4，denominator 口径不同（本工位计所有 gamma-non-flat 到达 cert 的候选 869 条，非仅 gate-pass 子集），非矛盾；两处均为生产 descend 读数，本工位新增的是**两口径等价**这一独立事实。
- MEMORY `level-hole-window-dependence`（codex #7）：level 塔空洞归因——本工位与之正交（不改判据，只对拍锚定口径）。
- `formalization-validity-domain`（L0/L1/L2 分级）：本工位等价论断是 **L2**（真实 BTC 单标的，可产否定性结果），不是 L0 代数同义反复——三窗任一窗若 refinement 破裂即会产生差异>0 的否定性结果。
- `no-patch-mentality`（090）：不因「点包含实测够用」而回避严格性——反向操作：把 PDF 最严格 bottom-up 完整实现出来作对拍锚，用 assert 固化等价，refinement 一旦破裂立即暴露。

## 6. 影响声明

**改动文件**：`rust/src/theta_v0/backtest/econ_positive.rs`（唯一）。

- 新增 `#[cfg(test)] pub(super) fn build_nest_certificate_bottomup`：PDF §二完整 bottom-up 构造（子区间包含 + Sel_Θ 作用于包含集 + child 加宽）。复用生产同一私有谓词（`bsp_cand_type`/`cand_delta_base_gate`/`cand_delta`/`find_move_by_end_index`），唯一变量是 knode 锚定口径。**release 编译掉，零生产行为改动。**
- 新增 `#[test] #[ignore] fn acc_bottomup_nest_parity_probe`：真实 BTC 逐信号对拍，assert 四项差异全 0（等价固化）。
- **未改任何生产函数体**（`build_nest_certificate`/`build_gate_certificate`/`n_delta`/`is_sub` 全部不动）——NO-SHIP。
- 护航：`cargo test --release --lib` = 1405 passed / 0 failed / 105 ignored（含本探针），无退化（1402 基线 +3 为并行工位新增）。

**复算命令**：
```
ECON_L2_MAX_BARS=350000 cargo test --release --lib acc_bottomup_nest_parity_probe -- --ignored --nocapture
```
（默认 300K；全历史用 `ECON_L2_MAX_BARS=5000000`。确定性——同数据同 seed 逐 bit 复现。）

## 认识论等级

**L2**（真实数据单标的 BTC 全历史窗，可产否定性结果，非 L3 跨品种）。等价论断的有效域 = BTC；跨品种等价需 L3（未做）。
