# on2w3 candidate 全量拼接+merge 全量快照残余 实装（结果包）

工位 on2-wave3 / task #184 后继。日期：2026-07-04。基线 HEAD：`80e012eaf0`（07a frontier-resume SHIP）。
文件域：`strategy/{interp.rs, persistent.rs}` + `backtest/{runner.rs, incremental.rs}` + `classifier/mod.rs`（stage reset）。

认识论标注（231号）：
- bit-exact 正确性 = **L1**（管线等价：skip 路径 == 强制全量逐字段，`merge_in_place_split` 内嵌
  debug_assert 神谕 + `candidate_incremental_bit_exact_vs_full` + `bit_exact_per_bar` 双跑对照）。
- 计时收益 = **L1**（管线度量，零信息增量）。
- forest_epoch 稳定 ⟺ tree 字节不变前提 = **L2**（on2w2 已证：CL 真实数据 bump 率 2.17%，false_hits=0）。

---

## 1. 结论：SHIP

codex Q4 诚实声明的两个独立残余（candidate 每 bar 全量拼接/attach + merge 每 bar 全量快照消费）
**均已消除**。strategy 段标度指数 **1.90 → 1.75**，engine_full **1.73 → 1.58**（8K→16K，CL OOS L2）。

### 核心机制（诊断先行——ponytail：先 profile 定位，后按根因修）

env-gated stage profile（`profile_cand_stages`，THETA_PROFILE_STAGES=1，CL 8K/16K）拆出两个残余的
成本构成，**否证了「拼接/merge 是 O(bsp)/bar 下界」的旧诚实声明**——两者都藏着可消除的 O(n²)：

**残余①：candidate 构建（`cand_build_merge`，attach 重复）。**
- **根因（诊断坐实）**：CandidateCache 前缀复用在生产**从不命中**——`cand_tail_built` sum 逐字节等于
  `cand_count` sum（8K：102713==102713；16K：481889==481889），即每 bar 全量重建。因 whole-cache
  gate（interp.rs:852 旧代码）用 `tower_gen`（generation），而 generation 靠 `l0_is_root` blunt 兜底
  每-bar bump（98.5%，on2w2 §2 实证），`gen_match` 几乎恒失败 → 全 clear+rebuild。
- **修复（信号替换，非新机制）**：whole-cache gate 改用 `forest_epoch`（on2w2 已证 sound 的紧信号，
  bump 率 2.17%，false_hits=0）。`forest_epoch=Some` 走 epoch（生产）；`None` fallback gen（合成/全量
  非增量）。epoch 稳 ⟺ tower 全级字节不变 ⟺ `extract_carrier_forest(tower)`（tree）字节不变 ⟹ 同
  bsp.source_index 的 `attach_bsp_carrier_indexed`（parent/id/attached_dir）不变。per-level `prefix_fp`
  （bsp 内容）+ `base_ci`（ordinal）独立守 bsp/ordinal 变化 ⟹ 前缀逐字节稳定。
- **收益**：`cand_build_merge` 3.21→0.48ms（8K，6.7×）、12.32→1.70ms（16K，7.2×）。

**残余②：merge 全量快照消费（`cand_merge_consume`，dominant O(n²)）。**
- **根因（诊断坐实）**：`merge_in_place_split` 的 candidate 段每 bar 全量做 `cand_ids` HashSet 建 +
  step1' reset + step2' upsert + `present_last_cand` 重建，全 O(cand)/bar。cand 数 ∝n^1.23（avg
  12.8@8K→30@16K）⟹ Σ_bar = O(n²)。诊断另证 candidate **id 集本质冻结**：16000 bar 仅 3 drop / 63
  add（`cand_dropped`/`cand_added` 跨度），却每 bar 全量重扫。
- **修复（append-only 跳过，tree 段 `tree_dirty` 同构）**：CandidateCache 新增 `dirty` 信号（whole-cache
  miss ∨ 任一级重建 ∨ 有新尾 build ⟹ true）。runner 读 `cand_cache.dirty` 传 merge。`!cand_dirty` ⟹
  candidates 逐字节同上 bar ⟹ 整段 candidate 工作是 no-op：同 id 全 present、同值上 bar 已 upsert
  （invalidate 不被 upsert 重置 ⟹ skip 末态一致）、present_last_cand 恒等 ⟹ 跳过（O(1)）。
- **soundness 陷阱（已封）**：`tree_dirty=true ∧ cand_dirty=false` 时，tree 段可能 upsert 了与 candidate
  碰撞的 carrier id（tree 值）⟹ candidate upsert 须重跑恢复「candidate 胜」覆盖序（断言2）。故 candidate
  upsert 跳过条件是 `!(cand_dirty || tree_dirty)`，reset/id集/present_last 跳过条件是 `!cand_dirty`。
- **收益**：`cand_merge_consume` 4.31→0.76ms（8K，5.7×）、18.6→2.17ms（16K，8.6×）。

## 2. 计时验收（CL OOS 2023-01-01..2025-06-30，THETA_PROFILE_STAGES，--release）

| cost center | 8K 修前 | 8K 修后 | 16K 修前 | 16K 修后 | 16K 加速 |
|-------------|---------|---------|----------|----------|----------|
| cand_build_merge（attach） | 3.208 ms | 0.477 ms | 12.321 ms | 1.699 ms | **7.2×** |
| cand_merge_consume（快照） | 4.305 ms | 0.761 ms | 18.704 ms | 2.167 ms | **8.6×** |
| strategy 净额（engine−classify−clloop） | 0.018 s | 0.013 s | 0.069 s | 0.042 s | 1.6× |

| 标度指数（8K→16K） | 修前 | 修后 |
|--------------------|------|------|
| strategy | 1.90 | **1.75** |
| engine_full | 1.73 | **1.58** |

残余最大项转为 `05_compose_resume`/`05c_*`（12.8ms@16K，recursive_tower/classify 域）——07a 结果包
已声明的下一靶，**非本件有效域**。candidate 段三 stage（build_step/build_merge/merge_consume）现全
≤3ms@16K 且亚二次。

## 3. 守卫（全绿）

- **`merge_in_place_split` 内嵌 skip 神谕（新增，debug 构建）**：`!tree_dirty ∨ !cand_dirty` 时 clone
  self、跑强制全量（tree_dirty=true ∧ cand_dirty=true）、逐字段对拍 `elements`+`present_last_{tree,cand}`
  ——任一 bar 发散立即 panic。release 构建 `#[cfg(debug_assertions)]` 完全不编译（零成本）。
- **`merge_skip_oracle_real_cl_debug`（新增，debug/CL 4K）**：全引擎 run_theta_v0_pi，生产 forest_epoch
  稳定 ⟹ merge skip 大量触发，内嵌神谕逐 bar 守。**通过**（4000 bar 全 merge 调用 skip==全量）。
- **`candidate_incremental_bit_exact_vs_full`（CL 8K，ignored）**：gamma-free 增量 candidates == 全路径
  逐 bar bit-identical——epoch swap 后**通过**（8000 bar bit-identical，cand_cache 命中 406→大幅升）。
- **`bit_exact_per_bar`（CL 8K，ignored）+ `bit_exact_per_bar_cached_vs_nocache`**：**2 passed**。
- **runner tests 全套（debug，39 passed）**：merge 神谕在 zigzag/deterministic 真结构路径逐 bar 守。
- **`cargo test --release --lib`：1489 passed / 0 failed**（+1 新测 merge_skip_oracle）。

## 4. 边界条件（结论翻转）

- 若 `merge_in_place_split` skip 神谕任一 bar panic ⟹ 「candidates 逐字节稳定 ∨ id 集恒等」前提被违反
  （forest_epoch 假命中 / dirty 信号漏判）⟹ 回滚。**实测 CL 4K debug 全绿**。
- 若 `candidate_incremental_bit_exact_vs_full` 破裂 ⟹ epoch swap 引入陈旧前缀复用 ⟹ 回滚。实测通过。
- 若生产 forest_epoch 常态每-bar bump（tower 剧烈变）⟹ cand_dirty 恒 true ⟹ 退化全量 merge，无收益
  但仍 bit-exact（skip 条件不满足即走全量分支）。实测 exp 1.90→1.75 ⟹ epoch 稳定率足够，前提成立。
- 若 candidate id 集非 append-only（drop 频繁）⟹ cand_dirty 恒 true（重建触发）⟹ 退化全量。实测
  16000 bar 仅 3 drop ⟹ append-only 前提成立。

## 5. 下游推论

- 信号集/candidate/registry 末态 bit-exact 不变 ⟹ 不影响 W-VERIFY(#13) alpha、不改任何 class_index/
  分桶 key/π 账本。
- on2 战役 candidate 侧两个 codex Q4 残余关闭。strategy 段残余转为纯 recursive_tower 域
  （`05_compose_resume`），交由后续 05 泳道。

## 6. 谱系引用

on2w2-epoch-impl（forest_epoch 生产 + soundness 证明，commit `e1f333ec77`/`f15253ce3f`）——本件复用
forest_epoch 作 CandidateCache whole-cache 判据；on2w3-07a-impl（frontier-resume，commit `80e012eaf0`，
基线）；#160 A12（K_i/T_i 双视图）；090号（bit-exact 铁律 + no-patch：merge/build 全量分支保留作 skip
不满足时的正确 fallback，非删除）；275号（merge skip 状态挂 CandidateCache/PersistentRegistry，不越域）；
231号（L1 计时零信息增量 + forest_epoch 稳定前提 L2）。

## 7. 影响声明

改动 5 文件：
- `strategy/interp.rs`：`CandidateCache` 加 `epoch`（生产 whole-cache 判据）+ `dirty`（merge skip 信号）
  字段；whole-cache gate `tower_gen` → `forest_epoch`（None fallback gen）；build 循环设 `cand_dirty`。
- `strategy/persistent.rs`：`merge_in_place_split` 加 `cand_dirty` 参 + skip 逻辑（cand_ids/reset/upsert/
  present_last_cand 按 dirty 门控）；抽 `merge_in_place_split_core`；新增 `merge_full_oracle`（debug）+
  内嵌 skip 神谕 debug_assert；`merge_in_place` wrapper 加参透传。
- `backtest/runner.rs`：merge 调用点读 `cand_cache.dirty` 传参；stage 计时插桩（cand_build_step/
  cand_build_merge/cand_merge_consume + cand_count 跨度，env-gated）；新增 `merge_skip_oracle_real_cl_debug`。
- `backtest/incremental.rs`：2 diag 调用点加 `cand_dirty` 参；新增 `profile_cand_stages`（stage 拆解）。
- `classifier/mod.rs`：`stage_profile::reset()`（多窗口 profile 隔离）。

**不改**：`build_candidate_element`（attach 核）、`extract_carrier_forest`、generation/forest_epoch 语义、
merge 的全量分支逻辑（保留作 fallback）、任何输出计算路径。
