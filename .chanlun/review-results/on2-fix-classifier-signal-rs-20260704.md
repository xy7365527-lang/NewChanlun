# O(n²) 修复结果包：classifier/signal.rs 文件域（H2-extract-signals-l0）

日期：2026-07-04
文件域：`rust/src/theta_v0/classifier/signal.rs`（限定，不收他域 hunk）
热点：H2-extract-signals-l0（stage 07a_extract_signals_l0）
基线 HEAD（session 起）：`ebcd8f2a0f`（任务令）；实测起点已漂移至 `f9440a6e9d`（并行 agent 推进 H4/stage-09/A2）。

## 结论：NO-SHIP（signal.rs 单域无 asymptotic 修复杠杆）

codex 裁决（`codex exec --skip-git-repo-check --sandbox read-only`，2026-07-04）：
Q1=B（07a resume 本质是 mod.rs 域工作，与 07b/B2 同族，不属 signal.rs 单域）；
Q3=NO-SHIP 严格且诚实。

### 根因分析（架构事实，已核实）

1. `signal.rs::extract_signals_with_hist` 是**无状态纯函数**，每 bar 被
   `mod.rs::classify_with_tower_incremental` 的 07a 调一次（memo miss 时）。
2. 其**单次调用成本已是 O(S) 线性**——#93 工位已用 `partition_point` 二分
   （`map_src_range_to_close_idx`/`nearest_confirmed_center_idx`）+ `first_match_idx` HashMap
   + `a_seg_cache` + 窗口化 departure helper（`locate_departure_move_a`/`departure_move_c_start`
   经 partition_point 定界 + 窗口内线性扫）杀掉了单次调用内的 O(S²)。已由 `map_src_range_to_close_idx`
   文档头 + `extract_signals_bit_exact_digest_guard`（GOLDEN=0x90c7_9ee6_17e1_1392 绿）坐实。
3. 观测到的 07a O(n²)（400K=678ms → 1M=5339ms，7.87x/2.5x bar ⟹ exp≈2.25）**纯属结构性**：
   memo miss 次数 ∝ 最终段数 S ∝ n，每次 miss 全量重算全部 S 段 ⟹ O(n)·O(S) = O(n²)。
   memo guard（`bsp_key = (centers.len, upper_moves.len, segments.len)`）只降**频率**不降**单次成本**，
   asymptotic 仍 O(n²)（任务令诊断正确）。
4. 唯一 asymptotic 修法 = frontier-resume（algo-opt-plan B2 泳道）：跨 bar 复用 confirmed 前缀段的
   第一/三类点，只重算末 center 变更点 `e` 之后的段。这需要**跨 bar 缓存状态**（上次提取的 pre-merge
   第一/三类点 + 稳定边界锚）。
5. 该缓存状态**只能挂 caller 的 `mod.rs::LevelCache`**——正是现存 `extract_second_resume` /
   `cached_second` / `cached_second_count` 先例（B2 已对 07b 第二类做过同族 resume，resume 函数在
   **mod.rs 而非 signal.rs**，per-parent 主体 `second_for_parent` 才在 signal 侧调用链）。

### 为何 signal.rs 单域不可 ship（no-patch，非硬修）

- 方案 A（signal.rs 建 `SignalResumeState` struct + caller &mut 传入）：**struct 只建不接 = 死代码**
  （无 LevelCache 字段 + 无 07a 调用点改动 ⟹ 零生产消费者，护栏禁止）；**接了 = 改 mod.rs 07a 调用点
  + 加 LevelCache 字段 = 越域**。mod.rs 是并行 agent 活跃域（本 session HEAD 已推进两次），铁律⑤禁收他域 hunk。
- 方案 B（判定 07a resume 属 mod.rs 域）：codex 采纳。asymptotic 修复的状态/失效/cascade/`prefix_count`
  全在 mod.rs，交 mod.rs 域 agent 统一落地。

## 计时（修前=修后，无 ship）

| 窗口 | wall | 07a_extract_signals_l0 stage |
|------|------|------------------------------|
| 400K | 6.03s | 678.715 ms |
| 1M   | 36.06s | 5339.312 ms |

exp≈2.25（O(n²)，结构性，signal.rs 单域不可动）。无修改 ⟹ 无 before/after 差。

## 守卫（signal.rs 域全绿，未破 bit-exact）

- `extract_signals_bit_exact_digest_guard`：GOLDEN=`0x90c7_9ee6_17e1_1392` PASS
- `extract_signals_bit_exact_vs_orig_per_case`（14 case 逐字段 vs git HEAD oracle）：PASS
（未改任何代码 ⟹ 守卫状态 = 基线状态，bit-exact 不变式天然保持。）

## 边界条件（结论翻转条件）

- 若 mod.rs 域 agent 愿在 LevelCache 落 frontier-resume 状态：signal.rs 侧应补一个纯函数于
  `(稳定前缀段, tail 段)` 的 per-tail 提取入口（镜像 `second_for_parent`），resume 主体（缓存/推进/
  cascade 失效/parity 断言）在 mod.rs。届时本 NO-SHIP 翻转为「signal.rs 提供 resume-friendly 分段入口」。
- Q2 锚口径（codex 裁决，供 mod.rs 域 agent 落地用）：`e` **不是** `decompose_state.frozen_rels` 本身。
  正确锚 = caller frontier 协议的「第一个非稳定 center index」（通常 = `prefix_count`；若仅从 decompose
  关系推 = `frozen_rels + 1` 对应 center，非 `frozen_rels`）。转 source-index 阈值 `e`：精确口径 = 该
  first-unstable center 的 `end_index`；保守口径 = 与 `resume_start` 对应 input source 起点取 min。
  tail 重判：一类重判 `seg.start_index >= e`；三类从 `lower_bound(e) - 1` 起（覆盖 leave/retest 跨边界 pair）。

## 影响声明

无代码改动。本结果包记录 signal.rs 单域对 H2 热点的 NO-SHIP 裁定 + 向 mod.rs 域的 handoff（Q2 锚口径）。
纯性能诊断，不涉及领域概念定义（简化版结果包）。
