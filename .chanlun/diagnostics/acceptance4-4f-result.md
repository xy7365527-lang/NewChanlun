# 工位 4f 真修结果（L2，CL OOS）

## 结论：merge confirmed prefix 增量完成（5× 降，bit-exact 过），全引擎 exp 受限于 TreeKey-miss ceiling

## 实装（serena 落盘真实 fs——之前 Edit 工具改动未持久化，已重做）
- persistent.rs：merge_in_place_split(tree, tree_dirty, candidates, held_legs)。tree_dirty=false（TreeKey 命中）
  跳过 tree 段 step 1'（present_last_tree 全 present）+ step 2'（上 bar 已写同值）。present_last 拆 tree/cand 两段。
- runner.rs / decompose diag：Rc::ptr_eq(prev_tree, tree_ref) 算 tree_dirty。

## L2 实测（decompose diag）
```
   n   | extract_s merge_s | x_exp m_exp | registry_len cand_last
 4000  |   0.005   0.001   | 2.42  2.04  |   40          11
 8000  |   0.018   0.003   | 1.75  1.67  |   86          21
16000  |   0.077   0.012   | 2.09  2.07  |  193          47
```
对照修前 merge_s@16K=0.059 ⟹ **降 5×（0.059→0.012）**。merge 已退出热点（占总 ~13%，extract 0.077 是主项）。

## bit-exact 裁判（全过）
- bit_exact_per_bar（CL 8000 bar 双跑增量==全量）+ cached_vs_nocache（2000 bar）
- persistent 8 单测（I1-I5 不变量 + held detached + explicit close）+ bit_exact_synthetic 2 测

## 剩余 exp 残留根因：TreeKey-miss ceiling（extract/merge 同源，超 merge 工位边界）
m_exp/x_exp 仍 ≈2.0 但**绝对值极小**。根因 = TreeKey miss（CL 16K 仅 26 次）时全量重建：
- extract 侧：extract_elements(tower) + 3×build_*_index 全 O(tree)
- merge 侧：tree_dirty=true 全量 upsert O(tree)
miss 在大 n 端时 tree 大 ⟹ 累积 O(Σtree_at_miss) 小系数超线性。

这是 interp.rs:576-579 已标注的 §16 ceiling：「更高级新出现时根结构重构，前缀不可简单 append 复用」。
彻底修需检测最高级变化 + 索引重映射重用前缀——high bit-exact risk，独立大重构，**非 merge 工位**。

## 认识论
- L1：bit-exact 增量==全量（裁判过）
- L2：merge_s 降 5×（确认性，CL 单标的）；exp 残留是 TreeKey-miss ceiling（否定性：merge 单段优化不足以达全引擎 exp≈1.0，需 extract 侧前缀重用）

## 第二轮（lead 升级优先级：O(n) 解锁全 8 品种）
新增修复 step 路径 O(tree)：`ancestor_close_by_id` 的 `id_to_idx` 每 bar 全 work 重建 O(tree)/bar
→ 复用缓存 `base_id_idx`（base 段）+ overlay 段现建小 map（罕见）。新 `ancestors_by_id_lookup` 双段查。
- step_s@16K：0.071→0.058（降 18%），step_exp 仍 1.83（剩余 O(tree) 未定位完）
- bit-exact 过（bit_exact_per_bar + cached_vs_nocache）

## git 状态（环境澄清）
HEAD=332176e6cb **已含本轮全部改动**（merge_in_place_split / present_last_tree×8 / ancestors_by_id_lookup×2，
磁盘 md5 == HEAD md5 逐字节同）。改动已落盘+已提交（Lead 或并行工位吸收）。工作区干净非"未改"。

## 剩余 step_exp 1.83（未达 O(n)，下一轮）
merge 已 O(n)（tree_dirty 跳过）。step 路径仍有 O(tree) 残留（非 ancestor_close）——候选：held 循环
`raw.contains`（O(prev_active×raw)）/ held_leg_tree_index_indexed / strategy_target_legs。需 profile
拆分 step 内部各段标度才能定位（diag 当前只测 step 总时）。
@16K step_s=0.058s 绝对值小，但 1.3M bar 全窗（BTC）O(n^1.83) 会爆——必须继续降。

## 工位边界判定
merge confirmed prefix 增量 = 完成（5×降 + bit-exact + codex）。
ancestor_close 双段 = 完成（18%降 + bit-exact）。
step 路径剩余 O(tree) = 下一轮（需 step 内部分段 profile）。全窗实测时间 = 真验收口径（非 diag exp）。
