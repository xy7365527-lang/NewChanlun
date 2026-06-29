# 热点① Rc 塔 + 诊断缺口（task#1 element-view-refactor）

## 结论

热点①（task 点名 classifier/mod.rs:912 `lc.upper_moves.clone()` per-bar 全塔深拷贝）已修：
`LevelCache.upper_moves: Rc<Vec<LeveledMove>>` + `moves_tower`/`tower_snapshots` 全程 Rc::clone
（O(1)），extend 经 `Rc::make_mut`（caller 逐 bar drop snapshot ⟹ strong_count==1 ⟹ 原地 O(tail)）。
返回类型 `Vec<Rc<Vec<LeveledMove>>>` 贯通下游（classify_impl/classify_with_tower/
classify_with_tower_incremental/IncrementalClassifier::classify_at + interp 5 签名 + coverage
extract_elements/pi_theta_step/coverage_step_classification + l3_fullwindow::instrument_bar +
runner 闭包 bound）。

## L1 bit-exact（管线，PASS）

- `incremental_tower_*` 5/5 PASS（incremental==full 逐 LeveledMove 相等）
- `bit_exact_synthetic` PASS（2000 bar 逐 bar incremental==full）
- `theta_v0::strategy::` 194/0 PASS（tower 类型贯通无破裂）

## L2 @16K profile（CL OOS，可否证）

| cost center | baseline exp | 本次 exp | 8K→16K wall（baseline→本次） |
|---|---|---|---|
| classify | 2.15 | **2.04** | 0.060→0.266 vs 0.037→**0.154**（−42%） |
| strategy | 2.20 | 1.95 | 0.093→0.429 vs 0.104→0.402 |
| engine_full | 2.18 | 1.98 | 0.154→0.695 vs 0.141→**0.556** |

## ★诊断缺口（brief 归因不完整，no-patch 要求显式化）

brief 把 classify O(n²) 单一归因于 line 912。**消 912 后 classify exp 仍 2.04**（拆解 profile
`profile_classify_at_decompose_16k`：parser p_exp=1.14 O(n) ✓，tower t_exp=**2.01** O(n²) ✗）。
剩余 O(n²) 根因（line 号为修后文件）：

1. **line 913 `project_to_units(&lc.upper_moves[..])`**：每 bar 对**累积** upper_moves 全量投影，
   且每 move 调 `rmove.lo()/hi()` **递归遍历子树**（RMove::Compose 取 leaf min/max）⟹
   per-bar O(Σ tree_size)。这是剩余主导。
2. **line 836 `cached_units.extend_from_slice(&units)`**：units 全量快照 O(prefix)/bar。
3. **line 908 `centers: lc.centers.clone()`**：每 bar 每级全量 clone centers O(prefix)。

units 增量化需重设计 frontier 检测（818 比对 `units[..scanned]` vs `cached_units[..scanned]`，
依赖"本 bar 重建 units"语义）+ cascade reset 正确性——超 brief 边界，是大 diff，bit-exact 风险高。
**这是"选择"类（继续深挖 units 增量 vs 先交付塔修复并报告），上浮判断而非擅自扩范围。**

## 热点②（strategy ElementView）未做

真凶定位：interp.rs:448 `c.tree.clone()`（memo 命中仍克隆整树）+ coverage.rs:1171
`elements.to_vec()`——同一线性增树每 bar 克隆两次。ElementView（Rc 共享前缀+私有尾部）消费面大
（coverage_step_from_buckets 的 work 及子函数 restore_ancestor_chain/ancestor_close_by_id/
strategy_target_legs 全按 &[CoverageElement] 索引）。待①方向裁决后做。

## 边界条件（结论翻转条件）

- 若某 caller 跨 bar 持有 tower（snapshot 不 drop）⟹ make_mut 每 bar 写时复制 ⟹ Rc 退化为
  原 clone（实测 runner/perf/l3 均逐 bar drop，不发生）。
- exp 是 8K→16K 单段 log-log，单次运行噪声 ±0.05；wall 降 42% 是稳健信号。

## 影响声明

改 6 文件（classifier/mod.rs 核心 + coverage/interp/incremental/l3_fullwindow/runner 类型贯通）。
TreeKey/extract_elements 内部迭代改 `.iter()`（Rc deref）。测试字面量塔加 rc_tower 适配 helper。

## 协调阻塞（commit）

runner.rs 工作区**纠缠 task#7（[5c] HeldVoice→strategy::exit）半成品**——含其失败 WIP 测试
`nautilus::strategy::held_long_stop_hit_yields_close_intent`（非本工位引入，stash 验证确认）。
runner.rs 整文件 commit 会带上 task#7 半成品；只提交我的 5 独占文件则该 commit 单独 checkout
编译失败（runner 调改签名函数）。需 Lead 协调 runner.rs 提交归属。

## 认识论等级

塔 bit-exact = L1（管线）；@16K exp/wall = L2（真实 CL，可否证）。
