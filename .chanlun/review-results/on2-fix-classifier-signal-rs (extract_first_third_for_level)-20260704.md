# O(n²) 修复结果包：classifier/signal.rs (extract_first_third_for_level)

- **域**：classifier — `extract_first_third_for_level`（mod.rs:163 委托 signal.rs:768 `extract_signals_with_hist_anchored`），stage `07a_extract_first_third_ln`
- **HEAD 基线**：ebcd8f2a0f（release，opt-level=3 lto=false，debug-assert 剥离）
- **状态**：**NO-SHIP**（本域内 frontier-resume 无法降阶——根因在共享 cascade 机制，域外；负结果合法，3aee6dd4c7/H8/H9 先例）

## 结论

H5 清单诚实成立（07a exp≈2.2 = O(n²)），但清单给的修复方向（"同 H2 frontier-resume 化"）
经实测**被证伪**——在本域施加 frontier-resume 无法降阶，理由如下：

1. **miss 结构实测**（400K，FT_PROBE 临时探针，已移除）：6926 次 memo-miss（level 1-4），
   其中 **6253 次（90%）为 `cascade=true`**。按 units 加权，**cascade miss 占总扫描成本 79%**
   （cascade units_sum 927235 vs non-cascade 251011）。

2. **cascade 定义性摧毁 frontier 缓存**：cascade_reset（mod.rs:1183-1195）显式清空整塔前缀
   （`upper_moves`/`centers`/`projected_units`/`decompose_state`/`cached_bsp_key=None`）——
   frontier-resume 缓存在 cascade bar 被清 ⟹ 退化全量重扫（bit-exact fallback 的必然）。
   cascade 成本 = O(n) 次 cascade × O(units≈n)/次 = **O(n²)**，frontier-resume 触及不到。

3. **直接反证**：姊妹 stage `07b_extract_second`（`extract_second_resume`，mod.rs:1553）**已经实装**
   本清单要求的 frontier-resume 模式（`cached_second` 前缀 + tail 重扫 + cascade clear）。它**仍是
   O(n²)**，exp=**2.28**（比 07a 的 2.21 还差）。同模式移植到 07a 预测同样无效——这是共享
   cascade 机制的下游，非独立可修面。

4. **per-miss 无常数因子可捞**：每 miss 成本 = `segs` 分配 O(units) + 内部 `decompose`/
   `center_trend_gate`/`center_block_kind`/`first_match_idx` 重建（O(units)/O(centers)）+ 单趟扫描
   O(units)——全部同阶，无子线性重构空间。cascade dirty_from/units 均值仅 0.267（父级可证稳定前缀
   仅 27%），即使跳过稳定前缀，剩余 73% 仍随 n 全扫，不改阶。

真正根因 = **cascade 频率**（recursive_tower.rs frontier 重扫 + mod.rs cascade 块），属
`classifier/recursive_tower.rs` 域（H9 已 NO-SHIP）与共享底座，**不在本文件域**
（`classifier/signal.rs` / `extract_first_third_for_level`）。本域无独立可 bit-exact 降阶的修复面。

## 计时（本域代表窗，本次实测 HEAD 基线）

| stage 07a_extract_first_third_ln | 400K | 1M | ratio | exp |
|---|---|---|---|---|
| 修前（HEAD） | 220.056 ms | 1665.750 ms | 7.57x | 2.21 |
| 修后（无改动） | 同上（前=后） | 同上 | — | — |
| 对照 07b（已含 frontier-resume） | 316.293 ms | 2559.981 ms | 8.09x | 2.28 |

400K wall=6.03s / 1M wall≈21s（test 总 36.74s 含编译）。无代码改动 ⟹ 修后=修前。

命令：`THETA_PROFILE_STAGES=1 A0_PROFILE_BARS={400000,1000000} cargo test --release -p newchan_rust --lib backtest::incremental::profile::profile_clone_cluster_a0 -- --ignored --nocapture`

## 守卫（bit-exact 电池，全绿）

- `cargo test --release --lib theta_v0::classifier::`：**252 passed / 0 failed / 6 ignored**
  （含 `incremental_tower_*`、各 `*_bit_exact`、`compose_level_resume_matches_full_compose`）。
- `cargo test --release --lib backtest::incremental`：**4 passed / 0 failed / 20 ignored**
  （`bit_exact_synthetic`、`bit_exact_confirmed_len_open_tail`、a3_oracle_*；`bit_exact_per_bar` 数据门控 ignored）。
- `extract_signals_bit_exact_digest_guard`（GOLDEN digest）：**1 passed**。
- 无代码改动 ⟹ bit-exact 由构造保证（信号集/GOLDEN digest 逐字节不变）。目标文件
  `signal.rs` / `mod.rs` `git diff HEAD --quiet` CLEAN（临时 FT_PROBE 探针已完整移除）。

## 边界条件（结论翻转条件）

- 若 cascade 频率被降阶（recursive_tower H9 域或共享底座）⟹ 07a/07b 随之降阶（本域是其下游，非独立可修）。
- 若未来 non-cascade miss（append-only）占比显著上升（>50% 成本）⟹ frontier-resume 首次产生正收益，届时须重评本 NO-SHIP。
- 若 per-miss 内部 `decompose`/gate 重建被证明可增量续算（前缀稳定证书）⟹ 可捞 non-cascade 部分常数因子（约 21% 成本），但不改主导 cascade 部分的阶。

## 影响声明

无代码改动。仅新增本结果包文件。不影响任何模块、信号集、GOLDEN digest。
