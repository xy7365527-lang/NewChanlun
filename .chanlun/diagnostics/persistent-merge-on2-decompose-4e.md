# persistent.rs merge O(n²) 分解诊断（工位 4e L2）

数据源：`diag_strategy_hotspot_decompose_16k`（CL OOS 2023-01-01..2025-06-30，--release，镜像生产 cached+in_place 路径）。
基线 HEAD 5ace35459a（#11 interp _cached + #12 strategy 三索引缓存已合）。

## 标度落点（merge_in_place per-bar，含 candidate snapshot）

```
      n |  extract_s    merge_s | x_exp  m_exp | registry_len  tree_only  cand_last  collide_max
   2000 |      0.001      0.001 |   NaN    NaN  |          22      0.001          5            5
   4000 |      0.005      0.004 |  2.37   2.23  |          40      0.003         11           11
   8000 |      0.018      0.014 |  1.87   1.82  |          86      0.011         21           14
  16000 |      0.080      0.058 |  2.13   2.09  |         193      0.054         47           30
```

- `merge_s`：生产口径 merge（snapshot = tree prefix ++ candidate）。m_exp≈2.09 = **O(n²)**。
- `tree_only`：仅 tree snapshot（candidate 空）的 merge，exp≈1.91 = **O(n²)**（tree 段独立即 O(n²)）。
- `cand_last`：最后窗口 candidate 数 = 47（极小，非主导）。
- `collide_max`：candidate id ∈ tree id 的最大碰撞数 ≈ cand_last（**几乎全部 candidate 复用 tree 元素 id**）。

## 诊断

1. **merge O(n²) 根因 = snapshot（tree prefix）每 bar 线性增长**，merge 每 bar 全扫 snapshot
   刷新 snapshot_present + fields。registry_len 单调增（193@16K），tree prefix ∝ confirmed。
   这是 §16 confirmed prefix 的内禀性质，不是 merge 算法低效。

2. **candidate 段非主因**：cand=47 vs tree=193，且 candidate 系统性复用 tree id
   （collide≈cand，coverage.rs:538 candidate id = carrier 容器 id = tree 元素 id）。

3. **tree 段不可干净跳过**：candidate 顺序在 tree 之后 upsert ⟹ 系统性覆盖 tree 元素
   fields（lambda/rho→source_index）。下 bar tree upsert 复位。candidate 覆盖 + tree 复位
   都是 §9 语义必需，bit-exact 必须保留。要跳过 tree 段需追踪"上 bar 被 candidate 覆盖、
   本 bar 不再覆盖"的 id 显式复位（frontier-only merge）——soundness 非平凡。

## 认识论等级

- L2：@16K 实测标度（CL 真实数据），可否证。
- merge_s 0.058s @16K = 绝对时间小；extract_s 0.080s 是更大的 O(n²) 贡献（candidate role 计算，
  §16 不可缓存，4d 已诚实标注）。

## codex 异质裁决（decide，已执行）

1. **frontier-only merge 非无条件 bit-exact**：仅在窄不变量下等价。最大破裂点 = `structural_parent_id`
   ——candidate 消失时须写回 tree `e.parent_id`，不能沿用 candidate carrier 父；`invalidated` 绝不能
   被 reset 清回 false；held 须保持 insert-only。复制 tree→candidate 覆盖 + 消失复位 + present_last
   隐式状态机，回归风险高。
2. **技术选型**：推荐先做 parser/extract 层 §16 增量，**不做 merge frontier**。merge 仅 0.058s@16K
   收益小；extract 0.080s@16K 更大且有更清楚的 confirmed-prefix/Rc 缓存不变量优化边界。

## 裁决执行（本工位 4e）

不做 merge frontier 增量（codex 选型 + 有效域诊断一致）。理由（no-patch）：
- merge O(n²) 与 extract O(n²) **同根** = §16 candidate 段 ∝ confirmed 不可缓存。
- 即使 merge→O(n)，extract 仍 O(n²) 主导，引擎 per-bar exp 不变 ⟹ 优化 merge 是修错误的层。
- frontier merge 高 soundness 风险（structural_parent_id 复位）+ 低收益（0.058s）。

**acceptance[4] 全引擎 per-bar exp≈1.0 在 candidate 段结构性不可达**——这是 §16 ceiling 的有效域
边界（formalization-validity-domain 231号：有效域 ⊊ 定义域）。彻底修复 = parser 层增量 extract
（confirmed prefix immutable / frontier only mutable），是独立工位，模块头注释 anc.pdf §9/§16 已标 ceiling。

## bit-exact 验收（L1）

本工位无 merge 逻辑改动（仅 diag 测量列 + `pi_theta_step_prebuilt` 可见性 pub→pub(crate)）：
- `backtest::incremental::bit_exact_synthetic` ok
- `backtest::incremental::bit_exact_per_bar`（CL 全 bar 双跑）ok
- `interp::bit_exact_per_bar_cached_vs_nocache` ok
- `classifier/segment_layers/zhongshu::*matches_full` 全 ok
- private_interfaces warning（ElementView more private than pi_theta_step_prebuilt）已消。
