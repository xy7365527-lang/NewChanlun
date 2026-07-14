# O(n²) 修复结果包：backtest/econ_positive.rs (collect_signals)

- **域**：backtest/econ_positive.rs — `collect_signals` / `pair_signals`
- **HEAD 基线**：ebcd8f2a0f（release，opt-level=3 lto=false，debug-assert 剥离）
- **状态**：**NO-SHIP**（econ 层无独立可修 O(n²)；负结果合法，3aee6dd4c7 先例）

## 结论

H8 清单诚实成立，逐点复核确认：

1. **主循环 O(n²) 全部来自共享底座 classify_at**（econ_positive.rs:299-304 `for i in 0..n` 每 bar 调 `classifier_incr.classify_at(i)` = H1-H5 分类器全栈 O(tree) 续算）。这是 classifier 域，非 econ 域——修 H1-H5 即修 C1，econ 层无独立修复面。
2. **econ 自身 per-new-signal 项已优化到位且已 commit（HEAD 内）**：
   - C2：`Rc::ptr_eq` 跳级（:306）——同 Rc ⟹ 整级已 seen，跳过内层循环，bit-exact。
   - C3：`partition_point` 二分含段查找（build_gate_certificate :933），替代线性 `find_move_by_end_index`。
   - `pair_signals` 配对：`next_long`/`next_short` 后缀表（:516-521）已将 O(信号²) 降为 O(信号)（全历史 12626 信号）。
   - per-signal `single` Classification 重建 + `assemble_gamma`（:326-340）仅 `seen.insert` miss 触发（600K bar=1682 次，万级噪声，非 O(bar) 主导）。
   - `sigma_higher_at`（selector.rs:195）每信号调一次为 **O(1)**（仅读 `upper.last()`，无扫描）。
3. **profile 佐证**：400K 代表窗计时探针 `profile_clone_cluster_a0` 测的是 `classify_with_tower_incremental`（classifier），dominant 阶段全为 07a/07b_extract、10_projected_units、05c2_submoves 等 classifier stage——econ 层根本不出现在热点榜。

## 计时（本域代表窗 400K bar）

| | wall |
|---|---|
| 修前（HEAD 基线） | 6.03s |
| 修后（无改动） | **6.11s**（噪声内，无代码改动 ⟹ 前=后） |

命令：`THETA_PROFILE_STAGES=1 A0_PROFILE_BARS=400000 cargo test --release -p newchan_rust --lib backtest::incremental::profile::profile_clone_cluster_a0 -- --ignored --nocapture`

## 守卫（bit-exact 电池）

`cargo test --release --lib` 全绿：**1484 passed / 0 failed / 111 ignored**。
含 GOLDEN digest（`bit_exact_battery_digest` / `extract_signals_bit_exact_digest_guard`）、
`bit_exact_per_bar` / `bit_exact_synthetic` / `incremental_tower_scaling_dominates_full_synthetic`。
无代码改动 ⟹ bit-exact 由构造保证（信号集/GOLDEN digest 逐字节不变）。

## 边界条件（结论翻转条件）

- 若 classify_at（classifier 底座 H1-H5）的 O(n²) 被降阶 ⟹ collect_signals 主循环随之降阶（C1 = H1-H5 的下游，非独立可修）。
- 若未来 econ 新增 per-signal 逻辑再引入线性扫描（破坏 C2/C3 的 O(1)/O(log) 局部性）⟹ NO-SHIP 结论失效，需重测。
- 若 `seen` miss 率从万级噪声（1682/600K）显著上升（如信号密度剧增）⟹ per-signal `single` 重建成本可能升为主导，届时须复评。

## 影响声明

无代码改动。本包为诊断性负结果，销项 #183 on2-sweep 的 econ 域分片。econ 层 C1/C2/C3 优化已在 HEAD ebcd8f2a0f 落地，本轮仅复核确认无剩余独立 O(n²)。
