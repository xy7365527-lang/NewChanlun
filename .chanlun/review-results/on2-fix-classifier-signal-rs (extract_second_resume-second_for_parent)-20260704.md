# on2-fix: classifier/signal.rs (extract_second_resume / second_for_parent) — stage 07b_extract_second

**日期**: 2026-07-04
**基线 HEAD**（任务派发时）: ebcd8f2a0f；**实测 HEAD**（session 中并发工位推进后）: d1b5b9d229
**编译**: release（opt-level=3 lto=false，debug_assert 剥离）
**数据**: BTC 真实 4.6M bar 池取前 N，`classify_with_tower_incremental` 逐 bar

## 结论：NO-SHIP（asymptotic 不可在本文件域降阶）+ 文档订正（声明膨胀修复）

清单给的修复方向「接入 B3 #4 area-memo」**前提已失效**——area-memo（`cached_segment_area`，
`(start,end)→f64` 冻结缓存）**早在 `d516aa42ea`（07-02）已接入并生效**，是当前 HEAD 的祖先。
函数头文档（mod.rs:1553-1555 旧文）自陈「area-memo 未接入」是 `d516aa42ea` 之前的**陈旧声明**
（声明 < 实际，声明膨胀的逆向，090号）。本工位唯一代码产出 = 订正该陈旧文档为经验坐实的真相。

### 直测根因（BTC-1M，cascade vs append 分裂探针）

07b 全部 frontier tail 重扫按 miss 类型分裂：

| 探针 | sum | count | avg | max | 占比 |
|------|-----|-------|-----|-----|------|
| `07b_miss_append_tail`（纯 append miss，门控生效） | 28,570 | 9,938 | 2.87 | 1,794 | 1.4% |
| `07b_miss_cascade_tail`（cascade 清前缀→全量重扫） | 2,015,978 | 20,678 | 97.5 | 2,095 | **98.6%** |

- **门控（`extract_second_resume`）对 append miss 完全生效**（尾 avg=2.87，前缀 B2 一生一算）。
- 残余 O(n²) = **cascade 触发的前缀重扫**：`cascade_reset` 定义性清空 `cached_second`（mod.rs:1192），
  整个 confirmed 前缀 B2 缓存失效被重扫。cascade 频率 × 前缀长度 = O(n²)。
- 这**不是** frontier parent 的 area 累加（旧文档归因错误）。`07b_area` 实测 ~46ns/call
  （10.2M call@1M）= area-memo cache-hit 主导，面积累加已消解。

### 阶数与 per-call 分解（1M，插桩版，含 profiler 开销）

- `07b_sublevel_diverges` 2032ms（占 07b 主体），其中：`position` 递归 eq ~14%、`rmove_direction`
  递归 `.hi()` ~15%、`prevfind` ~14%、`area`（memo hit）~17%——**无单一主导项**，全部 subs.len()≤8
  有界（per-call O(1)）。改写只削常数因子不改指数。

### 为何 NO-SHIP（不可 bit-exact 降阶 / 收益极低）

1. **asymptotic 驱动 = cascade 频率**，位于 `recursive_tower` 域的 frontier 重排，非本文件域。
   姊妹工位 H5（`extract_first_third_for_level`）已 NO-SHIP，同根因（"cascade 定义性清前缀→缓存
   退化全量重扫"），H9（recursive_tower cascade 频率）亦已 NO-SHIP。
2. **per-call 优化**（预算方向指针 eq / 预算 subs 方向消 O(subs²) 递归）只削常数因子（subs≤8 有界），
   指数不动，收益极低——no-patch-mentality 不接受"能修的修"式常数因子补丁充当 O(n²) 解决。
3. cascade 缓存失效若要不清前缀，须改 cascade 语义本身（区分"投影变但 B2 源区间未变"）——那是
   `recursive_tower` cascade 域的判定，不是 `second_for_parent`/`sublevel_diverges` 可决定的。

## 计时（BTC，stage 07b_extract_second，release，无插桩）

| N bar | 修前=修后（本工位无 asymptotic 代码改动） |
|-------|------|
| 400K | 319.6 ms |
| 1M | 2769.9 ms |

scaling 400K→1M（2.5x bar）= 8.68x ⟹ exp = ln(8.68)/ln(2.5) ≈ **2.36（O(n²) 坐实）**。
area-memo 已生效仍 O(n²)，坐实根因非 area 累加。

## 守卫

- `cargo test --release -p newchan_rust --lib`：**1484 passed / 0 failed**（112 ignored=真实数据 `--ignored`）。
- `--lib bit_exact` 电池：50 passed / 0 failed。
- 本工位代码改动 = 0 逻辑行（仅 doc-comment 订正）⟹ 编译产物按 Rust 语义 doc-string 外逐字节等价
  ⟹ GOLDEN digest / bit_exact_per_bar / bit_exact_synthetic / incremental_tower 电池 bit-exact 守卫
  平凡保持（无浮点累加序改动、无信号集改动）。

## 边界条件（结论何时翻转）

- 若 `recursive_tower` 域后续降低 cascade 频率（frontier 重排收窄）⟹ 07b 的 cascade miss 占比下降
  ⟹ 门控生效域扩大 ⟹ 07b 自然线性化。届时本文件无需改动即受益（门控已就位）。
- 若定义层引入"cascade 时区分 B2 源区间是否受影响"的细粒度失效（而非无条件清 `cached_second`），
  可保留 cascade 下的前缀 B2 缓存——但这属 cascade 语义（recursive_tower 域）裁量，非本文件。

## 影响声明

- 改动文件：`rust/src/theta_v0/classifier/mod.rs`（`extract_second_resume` 函数头文档，1 处）。
- 改动性质：陈旧声明订正（area-memo 已接入 + 根因由"area 累加"订正为"cascade 前缀重扫"）。
- 不改：任何信号集 / 判据 / 浮点累加序 / 缓存语义 / GOLDEN digest。
