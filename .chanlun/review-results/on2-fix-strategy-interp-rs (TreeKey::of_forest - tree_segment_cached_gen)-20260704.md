# O(n²) 修复结果包：strategy/interp.rs TreeKey::of_forest / tree_segment_cached_gen

**日期**：2026-07-04
**工位**：on2-sweep #183 文件域「interp.rs (of_forest / tree_segment_cached_gen)」
**状态**：**NO-SHIP（本文件域内无 sound 修复；sound 路径需 classifier 域配合，越界）**

## 结论

热点 [H6-of-forest-fingerprint] 确认为真 O(n²)，但**在本文件域（interp.rs 单独）内不存在 bit-exact-sound 的 O(1)/O(depth) 命中判据**。codex 裁决（bit-exact 铁律路径）：唯一 sound 修复 = classifier 域新增独立 `forest_epoch`，越出本文件域边界。故本域 NO-SHIP，O(n²) 默认保留（优先于不 sound 快路，no-patch-mentality）。

## 计时（修前=修后，未改代码）

- 诊断 `diag_gen_fastpath_extract_scaling_16k`（CL OOS，真 of_forest interp 路径）：
  - n=4000 → 8000：x_exp=2.02；8000 → 16000：x_exp=2.24。**exp≈2.0 坐实 O(n²)**。
  - extract_s：4K=0.0049s / 8K=0.0199s / 16K=0.0936s。
- 根因：`tree_segment_cached_gen`（interp.rs:659）**命中检查前无条件** `TreeKey::of_forest(tower)`，
  遍历全塔所有级别所有 LeveledMove 深度前序发射（含 sub_moves 递归）= O(全塔节点)/bar × n = O(n²)。
  forest 本身已 Rc 缓存，O(n²) 纯来自每 bar 的**指纹计算**（非 forest 提取，miss 才重算 forest）。
- 修后：无代码变更 ⟹ 计时不变。

## 实证（CL OOS 16000 bar，逐 bar 对 extract_carrier_forest 真值对拍，含 bar 1464 古怪线段重划）

| 命中判据 | 命中率 | VIOLATION（命中但 forest 变） | soundness |
|---------|-------|------|-----------|
| forest 真变基准 | — | 126/16000（0.8%）真需 recompute | — |
| `generation`（现有 TowerCache） | 1.5% | **0** | 实证 sound 但过度 bump（无用） |
| 全级 Rc::ptr_eq | 100% | 123 | 不 sound（L0-root in-place make_mut 保指针改内容） |
| 持 clone 强制 CoW 后 ptr_eq | 0% | 0 | classifier 每 bar 无条件 make_mut(moves_tower_l0) ⟹ 持 clone L0 每 bar CoW，0 命中 |
| 每级 (ptr, len) | 97.1% | 6 | 不 sound（同 len frontier 改写漏检） |
| 每级 (len, 首深发射, 尾深发射) shallow/deep | 97.1% | 0 | **L2-实证 sound**，非 L0-代数 sound |

关键发现：
1. interp.rs:641-644 注释「`generation` 对 K_i 不 sound」**实证被证伪**（0 violations，含所引 bar 3020 类）——
   bar 3020 的证据是 BSP/candidate 内容变，不是 tower forest 变。但 generation 因 `l0_is_root || did_extend`
   过度 bump（几乎每 bar +1）命中仅 1.5%，作判据无用。
2. shallow/deep 键 97.1% 命中 0 violation，但 soundness 是 **L2-实证**（16K CL 未撞 + 现有 debug 全量守卫），
   有可构造理论漏检形态（单级 frontier 一次改写 ≥3 元素、首尾指纹恰不变仅中间变——§16 无代数证明 frontier ≤2）。

## codex 裁决（bit-exact 铁律，read-only sandbox，2026-07-04）

- **A.** shallow-key **不进** bit-exact 主路径。release 假命中静默污染 K_i；debug_assert 只在 debug 守。
  bit-exact 主路径不能靠「当前 CL 16K 没撞」。可留作 ignored L2 探针，不作默认命中判据。
- **B.** 不建议现在开 upper `confirmed_len` 跨域接口。sound 路 = classifier 新增独立
  `ki_forest_generation` / `forest_epoch`（不复用现有 `generation`），只承诺 `extract_carrier_forest(tower)`
  不变，命中 O(1)，debug 继续全量对拍。**当前 O(n²) 默认保留，优先于不 sound 快路。**
- **C.** 不改现有 `generation` 语义服务 K_i（还被 candidate cache / T_i 历史契约消费）。要修分离独立 epoch。

## 影响声明

- **本文件域（interp.rs）无代码变更**（诊断探针写在 incremental.rs，已 revert，非本域且非持久产出）。
- O(n²) 热点保留。sound 修复的落地位置 = classifier 域（TowerCache 新增 `forest_epoch` 字段 +
  维护点 + `IncrementalClassifier::forest_epoch()` 透传 + interp `tree_segment_cached_gen` 加 epoch 参数），
  越出本文件域边界，须 classifier 域 agent 或后续跨域工位承接。
- 谱系引用：A12（648 裁决 D，双视图 T_i/K_i 分离）；interp.rs:641-644 注释的 K_i-unsound 结论应订正
  （generation 实证对 forest sound，问题是过度 bump 非不 sound）——留待 forest_epoch 工位一并修文档。

## 边界条件（结论翻转条件）

- 若 classifier 域实装独立 `forest_epoch`（只在 forest 可观察变更时 bump）→ interp 可加 epoch 参数，
  命中 O(1)，O(n²) 消除，bit-exact 保持（debug 全量对拍守卫在位）。此为 sound 落地路径，非本域。
- 若编排者裁定接受 shallow-key 的 L2-实证 soundness（放宽 bit-exact 铁律至「实证 + debug 守卫」）→
  本域可独立落地 97.1% 命中的 shallow-key 修复。当前铁律 + codex 裁决 A 下不接受。

## 守卫状态

- 未改代码 ⟹ GOLDEN digest / bit_exact_per_bar / bit_exact_synthetic / incremental_tower_* 电池不受影响（不需重跑验证变更，因无变更）。
- `cargo build --release --lib` 全绿（诊断期间）。
