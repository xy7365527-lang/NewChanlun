# 工位 4f 真修 plan：confirmed prefix 增量维护（merge step 2'）

## 实测根因（L2，见同目录 -L2.md）
extract/merge/step exp≈2.0。`tree_only`=merge_s 的 90% ⟹ O(n²) 在 tree-prefix 全量重 upsert，
不在 candidate（cand@16K=47 有界，坐实 codex「常数有界」）。

## 现状（persistent.rs merge_in_place，runner.rs:608-620 生产调用）
snapshot = tree_ref（TreeCache 命中时逐字节不变，16K 仅 26 次未命中）+ candidates_ref（每 bar 变）。
- step 1'：上 bar present 但本 bar 不 present → snapshot_present=false（已增量，只扫 present_last）
- step 2'：`for e in snapshot` 全量 upsert（**O(snapshot)=O(tree)/bar=O(n²) 根**）

## 真修方向：tree 段脏检查跳过
TreeKey 命中（tree 未变）⟹ tree 段元素上 bar 已 upsert 同值 ⟹ 跳过 tree 段 upsert 等价。
candidate 段始终 upsert（每 bar 变）。runner 已有 tree_cache + TreeKey，复用其 valid+key 命中信号。

接口：`merge_in_place_split(tree: &[CoverageElement], tree_dirty: bool, candidates: &[CoverageElement], held_legs)`
- tree_dirty=true（TreeKey miss / 首 bar）：tree 段全量 upsert（现状路径）
- tree_dirty=false（命中）：tree 段跳过

## ★bit-exact 风险（需 codex 审）

### 风险1：candidate id 与 tree id 碰撞（interp.rs:538 坐实）
candidate.id = carrier_id（hostOf 命中 ⟹ **= 某 tree 元素 id**）或叶子 ordinal(ci)。
注释 persistent.rs:511「candidate 系统性复用 tree id（collide≈cand）」。
⟹ 当前 step 2' 顺序：先 upsert 全 tree，**再** upsert candidate。candidate 用 carrier_id 时
**覆盖** tree 元素的 snapshot_present/rho/dir/parent。
跳过 tree 段后：该 carrier_id entry 仍存在（上 bar tree upsert 留下），candidate 仍覆盖它 ⟹ 等价？
**需证**：上 bar tree upsert 的 carrier entry 值 == 本 bar tree（命中⟹tree 逐字节同）⟹ candidate 覆盖前的
基底相同 ⟹ candidate 覆盖后相同。**但**：candidate 覆盖会把 carrier 的 snapshot_present 改成 candidate
的（true，candidate 都 present）。tree 段若没被跳过，carrier 先被 tree upsert 成 present=true（tree 元素
都 present）再被 candidate 覆盖成 present=true ⟹ 同。跳过后 carrier 保留上 bar 末值——上 bar 末值
是否 present=true？上 bar 该 carrier 被上 bar candidate 覆盖成 present=true（若上 bar 也有 bsp 挂它）
**或** 被上 bar tree upsert 成 present=true（tree 元素恒 present）⟹ 上 bar 末态 present=true。∴ 跳过等价。
⟹ **初判等价，但依赖「tree 元素 upsert 恒 present=true 且 tree 命中⟹值同」，请 codex 核**。

### 风险2：step 1' present_last 口径
present_last = 上 bar snapshot ids（tree+candidate）。本 bar tree 命中 ⟹ tree ids 同上 bar ⟹ 仍 present
⟹ step 1' 不会误置 tree 元素 false（snapshot_ids 含 tree ids，contains 命中）。
**但** snapshot_ids 现由 `for e in snapshot` 同一份 snapshot 算。跳过 tree upsert 不改 snapshot_ids 计算
（snapshot_ids 仍从完整 tree+candidate 算）⟹ step 1' 不变。只跳 step 2' 的 upsert 循环。

### 风险3：held_legs op_parent（step 3'）
不动（held 段本就 O(held)=O(active) 有界）。

## 守卫
bit_exact_per_bar + bit_exact_synthetic + incremental_tower_*_matches_full。
新增脏检查路径必须与「tree_dirty=true 恒走全量」逐位一致（即跳过 ⟺ 重写同值）。

## 验收
@16K step/extract/merge exp 2.0→≈1.0。

## 实装进展（半成品，诚实记录）
已实装 merge_in_place_split + runner/diag ptr_eq 脏检查。bit-exact 全过（per_bar 8000 bar 双跑 + cached_vs_nocache）。
但 m_exp 仍 ≈2.11（merge_s 绝对值降 40%：0.059→0.035）。

**第二个 O(tree) 点定位**：merge_in_place_split 里 `snapshot_ids = tree.iter().chain(candidates).collect::<HashSet>()`
**每 bar 全 tree 构造 HashSet**（O(tree)/bar=O(n²)），与 tree_dirty 无关。跳过 upsert 循环不够。

step 路径（diag_coverage_step_scaling）的 step_exp≈1.87 是**另一个正交 O(n²)**：在 `coverage_step_prebuilt`
内部（merge 在该 diag 计时外 line 571），4f 未触及——extract 侧 role 计算/as_contiguous，非 merge。

**下一步设计**（bit-exact 敏感，需 codex/lead 审）：
tree_dirty=false ⟹ tree ids 同上 bar ⟹ present_last 中 tree 部分恒 present（不会被 step 1' 置 false）。
⟹ snapshot_ids 只需 candidate ids（tree 部分缓存或省略）。须拆 present_last 为 tree/candidate 两段维护：
- present_last_tree（tree_dirty=false 复用，true 重建）
- present_last_cand（每 bar 重建，O(candidate) 有界）
step 1' 遍历两段。bit-exact 风险：tree miss 时 tree present 重置口径须与全量一致。

## 认识论 L1（bit-exact 增量==全量）+ L2（exp 实测）
