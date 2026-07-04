# O(n²) 修复结果包：classifier/mod.rs — stage 10 `10_projected_units_clone`

/ 文件域：`rust/src/theta_v0/classifier/mod.rs`（H7-projected-units-clone）
/ 基线 HEAD：`2572c1f691`（任务书给的 `ebcd8f2a0f` 已被在飞 lane 推进覆盖）
/ 日期：2026-07-04

## 结论

stage 10 `units = lc.projected_units.clone()`（每 bar 每级全量克隆投影 Vec，O(units_L)/bar）
Rc 化为 `Rc<Vec<UnitRange>>` + `Rc::clone`（O(1)）。

改动（全在 mod.rs）：
1. `LevelCache.projected_units: Vec<UnitRange>` → `Rc<Vec<UnitRange>>`。
2. loop-carried `units: Vec<UnitRange>` → `Rc<Vec<UnitRange>>`（loop 内只读：读 len/切片/传 `&[]`，从不原地改）。
3. stage 09 truncate+resume 前 `Rc::make_mut(&mut lc.projected_units)`：上轮 `units` Rc 已被
   loop 尾/下 bar 重赋 drop ⟹ strong_count==1 ⟹ 原地写（O(tail) 不退化）；>1 ⟹ 写时复制（bit-exact 退化）。
4. stage 10 `units = Rc::clone(&lc.projected_units)`（O(1)）。
5. 投影证书 debug_assert 解引用 `*lc.projected_units`。
6. cascade clear 走 `Rc::make_mut(...).clear()`。

## 计时（BTC 真实数据，前 400K bar，THETA_PROFILE_STAGES=1，--release）

| stage | before | after |
|-------|--------|-------|
| 10_projected_units_clone | 167.5 ms | 24.9 ms（6.7x） |

残余 ~25ms = Rc 引用计数 + per-bar-per-level `stage_profile::time` 仪表自身开销。stage 09
（投影计算本身，非本域）不变（~1.15s）。

## 守卫（全绿）

- `bit_exact_per_bar`（CL 真实数据，n=8000，增量 vs legacy 全量逐字段对拍）：PASS。
- `bit_exact_synthetic` + `a3_oracle_minparts_reentry` + `a3_oracle_pop_rescan_empty`：PASS。
- parity 电池（classifier/center/buy/lean/07b_gating）：43 tests PASS。
- 全 lib：1484 passed / 0 failed。

## 边界条件

- bit-exact 依赖 `units` 全程只读（仅整体重赋值，从不原地改）。若未来有代码原地改 `units`
  的某元素，Rc 共享语义会经 `make_mut` 触发写时复制（仍 bit-exact，但退化拷贝）——需复核。
- stage 09 `make_mut` 原地写的前提是上轮 `units` Rc 已 drop（strong_count==1）。tower_snapshots
  存的是 `Rc<Vec<LeveledMove>>`（moves），**不**持 `projected_units` 引用 ⟹ 前提恒成立。

## 影响声明 / 协调冲突（未 commit，须编排者裁定）

改动全在 `classifier/mod.rs`，无跨文件签名变更（`projected_units` 是私有字段，
`project_to_units_resume(&mut Vec)` 契约不变——`make_mut` 返回 `&mut Vec`）。

**冲突**：在飞 H4 lane（stage `00b_l0_units_clone`）**同一文件** mod.rs 有未 commit 改动
（`l0_units_cache: Rc<Vec<UnitRange>>` + 共享声明 `let mut units: Rc<Vec<UnitRange>> = l0_units`）。
H4 与 H7 **互为编译依赖**：两者都要求 `units: Rc<Vec<UnitRange>>`，缺任一不编译。
`git add mod.rs` 会把 H4 未 commit hunk 扫进本 commit（违反防混批铁律⑤"绝不收他域 hunk"）。
本 lane 未 commit，留冲突给编排者裁定（同文件双 lane 合并归口）。
