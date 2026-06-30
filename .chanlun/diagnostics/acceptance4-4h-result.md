# 工位 4h 结果（L2，CL OOS）：candidate 段增量化真修 + 诚实有效域声明（源(b) ≠ strategy 全部）

## 结论

源(b)（candidate/gamma 段每 bar 全量重建）已增量化（bit-exact，codex 异质审两轮 + 三重守卫）。
strategy cost center exp **2.10→1.76**（16K 绝对值 0.099→0.042s，降 58%）。**但 strategy 未达 O(n)**——
诊断（profile + codex Q4/D）证实 strategy 的 O(n²) 是**多源并列**，candidate 段只是其一：
merge（persistent.rs O(bsp)/bar）+ newly_confirmed_step + k_theta_risk_gate 是同根残余，**不在
candidate 段有效域**（独立工位）。

## 任务前提修正（profile 否证 + codex 确认）

任务标题"candidate 段 O(n) 增量化 ⟹ 全引擎 exp≈1.0"前提**部分否证**：
- candidate 段**构建**侧（attach 重复）可增量（前缀缓存命中 100%，只 build 尾部 O(Σ Δbsp)）。
- 但 candidate 段**返回 Vec**（merge 接口需全量快照）= O(bsp)/bar，与 merge 内部 O(bsp)/bar 同阶，
  candidate 前缀缓存救不了（codex Q4 诚实不可消除项）。
- profile 坐实：candidate 构建侧旧 vs 新 exp 1.97→1.98（**几乎不变**），绝对值省 24%——证明 candidate
  路径 O(n²) 主项是返回 Vec 拼接（O(bsp)/bar），不是 attach 构建。
- 端到端 strategy 降 58%（跳遍历2 + 免 cand_sibling_idx 是真实收益），但 exp 1.76 残余在 merge/step/gate。

## bsp 标度（O(n²) 真源，L2）

```
n      bsp_total   tree_len
5000   11          39
10000  29          91
16000  47          120
```
bsp ~ n^1.26（**超线性**——高级别中枢随 n 累积）。BTC 1.31M 外推 bsp≈12592。
merge/step/gate 的 O(bsp)/bar × n bar = O(Σ bsp_i) 超线性，BTC 会爆（外推 ~800s merge）。

## 实装（interp.rs + runner.rs）

### PART1（codex Q1 SOUND）：caller B gamma-free 路径
- `coverage_elements_with_tower_cached_gen` 返回 `(tree, candidates)`，**不产 gamma**（caller B/merge 丢弃 gamma）。
- 跳过遍历2（role/operation_role_indexed_split）+ 不建 cand_sibling_idx（只为 gamma role 服务）。
- merge_in_place_split 只读 candidate {id,eps,level,lambda,rho,parent_id}（persistent.rs:238-291），不读 role/gamma。

### PART2（codex Q2/Q3 SOUND + 实施审 A/C 修复）：candidate 前缀缓存
- `CandidateCache { gen, per_level: Vec<Vec<CoverageElement>>, prefix_fp, base_ci, valid }`。
- 命中三条件（逐级）：(1) base_ci 不变 + (2) bsp 单调追加 + (3) 前缀指纹 (source_index, bits) bit-exact。
- 命中 ⟹ 复用前缀只 build 尾部 O(Σ Δbsp)；任一不满足 ⟹ 重建该级（bit-exact 退化）。

### 共享单一来源（no-patch）
- `tree_segment_cached_gen`：tree 缓存逻辑抽出，全路径 + gamma-free 路径共享（不复制）。
- `build_candidate_element`：单 bsp → CoverageElement，全路径遍历1 + gamma-free 路径共享。

## codex 异质审（两轮，gpt-5.5 实读代码）

### 第一轮（方案审，/tmp/codex_4h_prompt.md）
- Q1 SOUND：跳遍历2 bit-exact。Q2 SOUND（身份）/UNSOUND（gen-only completeness）。
- Q3 SOUND（须前缀内容校验，非仅 len——L0 seg_len 变可改 bsp 内容而 gen 不 bump）。
- Q4 诚实下界：merge/newly_confirmed_step/risk_gate 是 O(bsp)/bar 残余，candidate 缓存不覆盖。

### 第二轮（实施审，/tmp/codex_4h_impl_audit.md）
- **抓到真实条件性 bit-exact bug（QUESTION A/C UNSOUND）**：fallback ordinal id 依赖全局 flat ci
  （依赖所有前置级 bsp 数）。前置级（L0）增长 ⟹ 后置级（L1）fallback-id 元素 ordinal 须偏移，
  命中复用旧值 → 发散。反例：L0 8→9 时 L1 carrier-miss candidate ordinal 应 tree.len()+8→+9。
- **修复**：CandidateCache 加 `base_ci`，命中时逐级校验 base_ci，前置增长 ⟹ 该级 base_ci 偏移 ⟹ 重建该级。
- B SOUND（per-level 分段修正了早期扁平 Vec 顺序 bug）。D SOUND（残余 claim 正确）。

## bit-exact 守卫（三重）

1. `candidate_incremental_bit_exact_vs_full`（CL 8K，release + debug）：gamma-free 增量 candidates 逐 bar
   == 全路径 candidates。debug 触发 generation soundness debug_assert。
2. `candidate_cache_fallback_ordinal_prefix_shift`（合成，快测）：复现 codex A/C 反例（空 tower ⟹ 全
   carrier-miss + L0 增长），断言 L1 fallback ordinal 重建为 3。**守卫有效性验证**：临时禁用 base_ci
   校验 → 测试 FAIL（陈旧 ordinal 2），恢复 → 通过。
3. 全库 1225 测试通过（+1 新合成）。

## L2 实测（端到端 acceptance[4] profile，CL OOS）

```
            起点 exp    修后 exp    16K 绝对值起→后
classify     1.94       1.93        0.141→0.144s（#20 域，本工位未动）
strategy     2.10       1.76        0.099→0.042s（降 58%）
engine_full  2.00       1.89        0.241→0.186s
```

## 认识论（formalization-validity-domain 231号）

- **L1**：增量 candidates 逐 bar == 全量（bit-exact，三重守卫 + codex 两轮审）。
- **L2**：strategy exp 2.10→1.76（确认性，CL 单标的，降 58%）；**全引擎 exp≈1.0 未达**（否定性）：
  本工位有效域 = candidate 构建侧重复 attach 消除 + caller B 免遍历2/cand_sibling_idx。**不在有效域**：
  merge（persistent.rs O(bsp)/bar）+ newly_confirmed_step + k_theta_risk_gate + classify（#20 域 memo clone）。

## 剩余工位（诚实声明，相邻同根残余）

strategy exp 1.76 残余 + classify exp 1.93 残余的 O(n²) 源（codex Q4/D 确认，不在 candidate 段有效域）：
1. **merge_in_place_split**（persistent.rs:225-327）：cand_ids HashSet O(bsp) + upsert 每 candidate O(bsp)
   + present_last_cand 重建 O(bsp)。无条件 O(bsp)/bar。需 delta merge（codex 严格方案 §3 ElementDelta）。
2. **newly_confirmed_step**（runner.rs:371-394）：filter 全 bsp O(bsp)/bar。可增量（append-only seen-set 已近增量）。
3. **k_theta_risk_gate**（runner.rs:430-437）：bsp_index 全建 O(bsp)/bar。可缓存（prev_active 通常小）。
4. **classify memo clone**（mod.rs:914）：`lc.cached_bsp.clone()` O(bsp)/bar。#20 工位域（classifier）。

这四处是**同根**（每 bar 全量遍历累积 bsp）但**不同 cost center / 不同文件**，candidate 段修复对它们零影响。
真达 acceptance[4] engine exp≈1.0 需独立工位修这四处（或统一 bsp-delta 接口重构 persistent merge）。
