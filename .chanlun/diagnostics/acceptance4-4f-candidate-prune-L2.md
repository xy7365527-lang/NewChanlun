# acceptance[4] 工位 4f：candidate 段剪枝诊断（L2，CL OOS 2023-01..2025-06）

## 结论：codex 理论假设被实测否证 —— O(n²) 不在 candidate 段

任务前提（codex §16 异质证明）：实测 O(n²) = candidate 段重算扫了整个 confirmed 历史 / 把备选 anchors 当 live candidates。真修 = candidate 段 per-bar 只遍历 O(log n) 活跃前沿。

**实测否证此假设。** candidate 段极小且不扫 confirmed；O(n²) 根在 **tree-prefix 全量遍历**。

## L2 实测数据（--release，镜像生产 cached+in_place）

### diag_strategy_hotspot_decompose_16k（extract + merge）
```
   n   | extract_s merge_s | x_exp m_exp | registry_len tree_only cand_last
 2000  |   0.001   0.001   |  NaN  NaN   |     22         0.001      5
 4000  |   0.005   0.004   | 2.29  2.07  |     40         0.003     11
 8000  |   0.019   0.014   | 1.83  1.93  |     86         0.012     21
16000  |   0.081   0.059   | 2.07  2.04  |    193         0.053     47
```

### diag_coverage_step_scaling_16k（完整生产 step 路径，acceptance[4] 口径）
```
   n   | step_s | step_exp | registry_len
 2000  | 0.001  |   NaN    |   21
 4000  | 0.005  |  1.93    |   38
 8000  | 0.020  |  1.86    |   73
16000  | 0.072  |  1.87    |  157
```

## 根因定位（三条独立证据）

1. **`tree_only`（candidate 空时仅 tree snapshot merge）= 0.053 ≈ merge_s 0.059 的 90%**
   → merge 的 O(n²) 与 candidate 无关，candidate 空时 merge 仍 O(n²)。

2. **`cand_last` 16K 仅 47**（registry_len 193）
   → candidate 段规模 = 本 bar bsp 数（≈47），不随历史线性增长，剪它对 exp 无影响。

3. **代码路径**（persistent.rs:222 step 2' `for e in snapshot`）
   snapshot = tree prefix ∝ confirmed 全量。persistent.rs step 1'（snapshot_present 重置）已被工位 K
   增量化（只扫 present_last），但 **step 2' 仍每 bar 全量 upsert 整个 tree prefix 进 registry**
   （O(tree)=O(confirmed)/bar ⟹ O(n²)）。extract 侧同根：tree 走 TreeCache O(1) 命中，但每 bar
   仍 `as_contiguous`/ElementView 物化 tree prefix + role 计算遍历 tree 段。

persistent.rs:510-512 注释已自标此 ceiling：
> tree 段独立即 O(n²)，根因=snapshot(tree prefix)∝confirmed 每 bar 全扫，与 extract 同根
> （§16 candidate 不可缓存 ceiling）。

## 矛盾性质（no-workaround / no-patch）

codex 理论「全序分解 ⟹ 同级 candidate 常数有界 ⟹ candidate 段 O(log n)」可能成立，
但**它定位的热点（candidate 段）不是 O(n²) 源**。真源是 confirmed prefix 每 bar 全量重物化/重 upsert。

§16 ceiling 已声明 confirmed prefix immutable，应「摘要化不重扫」——但当前实装：
- merge step 2' 仍重 upsert 全 prefix（confirmed 部分每 bar 重写同值）
- step 内 role 计算仍遍历全 tree 段

真修方向 = **confirmed prefix 增量维护**（merge step 2' 只 upsert 本 bar 新增/变动元素，
confirmed 部分跳过；role 计算 confirmed 段摘要化）——这是 §16 ceiling 本体，
**不是 candidate 段剪枝**。剪 candidate 段 exp 不动（cand=47 vs prefix=193，且 prefix 是单调增的那个）。

## 认识论等级
- L2（真实数据 CL OOS，单标的单时段，可否证）。否定性结果：candidate 剪枝假设被否证。
- 验证管线 = 现有 diag（生产镜像 cached+in_place / 完整 step）。
