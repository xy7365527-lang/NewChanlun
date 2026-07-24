# 值桥语义调研：source_index==seg_c_full.1 多级语境下是否太严

**Issue**: #120  
**日期**: 2026-07-22  
**工作区**: worktree `/tmp/kimi-nest-mainline`  
**纪律**: 纯调研——零 rust 改动、零 cargo、零 git mutation、主仓零写入

---

## 摘要

值桥条件 `source_index == seg_c_full.1` 是**语义正确的精确等值判定**，不是太严。#107 外推的 ~35.6% 覆盖率与实测 5.2% 的差距源于**测量口径的范畴差异**：#107 测的是「结构性包含」（source_index 在中枢区间 [si,ei] 内，range 检查，跨度 91–10980 bars），值桥判的是「事件同一性」（source_index == 离开段终点，exact 点匹配）。这不是「差几 bar 太严」，而是 range 谓词与 equality 谓词的范畴差异。

---

## 1. 值桥键构造的确切语义

### 1.1 代码路径

`typed_lookup_multi`（admission.rs:662–706）的键构造：

```rust
// admission.rs:669-672
let ids = self.by_end_multi.get(&(
    source_index,                                    // 候选 BspPoint.source_index
    matches!(delta, super::super::types::Side::Long), // 方向同向
))?;
```

`by_end_multi` 索引在 `absorb_exts`（admission.rs:546–550）中增量构建：

```rust
// admission.rs:547-550
self.by_end_multi
    .entry((ext.seg_c_full.1, is_long))  // 键 = (离开段终点, 方向)
    .or_default()
    .push(id);
```

### 1.2 seg_c_full.1 指向什么

`seg_c_full` = 裁定 #64 §2(d) 定义的「收束前全段坐标」（`NestCandidateEventExt.seg_c_full`）：

- **Trend 域**（level_view.rs:798–800）：`seg_c_full = pair.seg_c`——R1 收束前的完整 C 段（离开段）。确认时 `interval_b` 收束到 `(seg_c.0, t*)`，但 `seg_c_full` 保留收束前的全段坐标 `pair.seg_c = (seg_c.0, seg_c.1)`。`seg_c_full.1` = 离开段终点（走势极值点）。
- **Pan 域**（level_view.rs:879）：`seg_c_full = structure.seg_c`，恒等于 `interval_b`，`seg_c_full.1 == turn_source`。

### 1.3 source_index 语义

`Candidate.source_index`（interp.rs:74–75）= 买卖点触发点在 L0 原始 K 序的位置。其源端 = `BspPoint.source_index`（bsp.rs:114–115），对一类点 = `end.source_index`（signal.rs:399，C 段破中枢的端点）。

### 1.4 坐标系同一性

两者同在 L0 原始 K 序坐标系（admission.rs:224–232 身份桥确证记录）。值桥问的是：**「候选买卖点是否恰好是某级别走势离开段的终点？」**

---

## 2. #75 身份桥确证锚的多级语境重验

### 2.1 确证锚位置（逐个核查）

| 锚点 | 位置 | 语义 | 多级语境是否仍成立 |
|---|---|---|---|
| bsp.rs:114–115 | `source_index: usize` = L0 原始 K 序位置 | 坐标定义 | ✅ 坐标系不随查询级别变化 |
| signal.rs:399 | `make_first_point(end.source_index, ...)` | 一类点 = C 段端点 | ✅ 一类点恒为段端点 |
| level_view.rs:773–776 | trend 确认分支：`interval_b=(seg_c.0, t*), turn_source=t*` | R1 收束 | ✅ seg_c_full 保留收束前坐标 |
| level_view.rs:874–875 | pan 域：`turn_source = structure.source_index` | pan 无收束 | ✅ seg_c_full == interval_b |
| nest.rs:607–628 | `terminal_bits_in_book`：CWindow 口径窗口匹配 | 终端背书 | ✅ 独立于身份桥 |

### 2.2 多级投影下的值桥语义

#112 已将固定 ℓ+1 查询升级为 `typed_lookup_multi`（扫描所有更深级别 ℓ ≥ level_origin+1）。值桥条件 `source_index == seg_c_full.1` 在多级语境下的语义：

- **不变**：每个 nest 事件的 `seg_c_full.1` 都在 L0 坐标系内（level_view 投影到 L0 K 序）。
- **多级扫描正确**：`by_end_multi` 的键不含级别（`(seg_c_full.1, is_long)`），同一 source_index 在多个级别的证书都可被查到。
- **级别桥和因果守卫独立于值桥**：级别下界（ℓ ≥ origin+1）和 `judge_at ≤ anchor` 是值桥之后的过滤层，不影响值桥本身的语义。

**结论**：#75 确证在多级语境下仍然成立。值桥不需要重新验证——它问的问题（候选点是否为离开段终点）不因扫描更多级别而改变。

---

## 3. typed_none 距离分布分析

### 3.1 数据源

- **NEST_GATE 汇总行**（/tmp/v4_C2.out，wf7 臂C）：
  - total=1724, typed_found=89 (5.2%), typed_none=1635 (94.8%)
  - NEST_GATE_INDEX: events=879, indexed=76（去重后唯一身份）
  - level_hits={1: 48, 2: 27, 3: 14}
- **tower_events.jsonl**：551 个 L1 中枢、129 个 L2、31 个 L3、5 个 L4
- **trades.jsonl**：162 个 L0 BSP 候选的 source_index（执行交易的子集）

### 3.2 限制声明

dump 仅含汇总行，无 per-candidate CHAIN 数据（无逐候选的 source_index 值）。以下分析使用 trades.jsonl 的 162 个 L0 BSP 作为代理样本（执行交易的 source_index 子集），以及 tower_events.jsonl 的中枢区间作为 seg_c_full.1 的位置代理。

### 3.3 距离分布

L0 BSP source_index 到最近的 L1+ 中枢端点（si/ei，seg_c_full.1 的代理）的最小距离：

| 距离区间 | 数量 | 占比 | 性质 |
|---|---|---|---|
| **0（精确匹配）** | 15 | 9.3% | 值桥命中 |
| 1–5 bars | 2 | 1.2% | 近似——可能因代理偏差 |
| 6–20 bars | 11 | 6.8% | 不同位置 |
| 21–100 bars | 71 | 43.8% | 根本不在同一段 |
| >100 bars | 63 | 38.9% | 完全无关 |
| 全部 L1+ 无近邻 | — | — | — |

### 3.4 结构性包含 vs 精确匹配（根因分析）

#107 的「结构性包含」检查：source_index 是否在某级别的中枢区间 [si,ei] 内。

| 级别 | 中心数 | 中枢跨度（中位数） | 结构性包含率 | 包含时到边界的距离中位数 |
|---|---|---|---|---|
| L1 | 551 | 312 bars | 50.6% | 63 bars |
| L2 | 129 | 1249 bars | 58.0% | 391 bars |
| L3 | 31 | 5094 bars | 64.2% | 1559 bars |
| L4 | 5 | 34430 bars | 50.0% | 5889 bars |
| **联合** | — | — | **94.4%** | — |

**关键发现**：当一个 L0 BSP 被 #107 判为「在 L2 有结构性覆盖」时，它距离 L2 中枢边界中位数 391 bars，距离实际的 seg_c_full.1 更远。这个 BSP 在物理上处于中枢内部深处，不是离开段端点。

### 3.5 宽匹配评估

| 窗口 | 命中率 | 评估 |
|---|---|---|
| 精确（当前） | 9.3% | 语义正确 |
| ±5 bars | 10.5% | 边际改善（+1.2pp），可能因代理偏差 |
| ±10 bars | 11.7% | 仍然很低 |
| ±50 bars | 35.8% | 接近 #107 外推，但无语义依据 |

---

## 4. 结论

### 值桥是否太严？

**否。** 值桥 `source_index == seg_c_full.1` 是语义正确的精确事件同一性判定。

### 35.6% vs 5.2% 差距的根因

差距来自 #107 的测量口径与值桥的判定口径之间的**范畴差异**：

1. **#107 的「结构性覆盖」= range 检查**：source_index WITHIN [si,ei]，中枢跨度中位数 L1=312 / L2=1249 / L3=5094 bars。这测的是「BSP 是否落在某级别的中枢空间范围内」。
2. **值桥 = exact 点匹配**：source_index == seg_c_full.1，离开段终点（一个具体的 bar 位置）。这判的是「BSP 是否就是某级别走势的转折点」。

二者不是同一谓词。94.4% 的 BSP 落在某级中枢区间内（结构性覆盖），但只有 5.2% 恰好是离开段端点（事件同一性）。

### 有无等价更宽匹配？

**无有意义的更宽匹配。**

- ±5 bars 仅增加 1.2pp，属于代理偏差噪声。
- 达到 ~35% 需要放宽到 ±50 bars，但这会匹配完全不相关的 BSP 到碰巧在 50 bars 内的事件，无缠论语义依据。
- 离开段端点是唯一的结构转折点——偏离它就不是同一个事件。

### 未确证事项

- 无 per-candidate CHAIN dump（逐候选 source_index 值），距离分布基于 trades.jsonl 的 162 个执行交易子集作为代理，可能存在选择偏差。
- seg_c_full.1 的精确值用中枢端点 si/ei 作代理，实际值可能在段内略有偏移——但偏移量（几个 bars）不影响结论的量级判断。
- 索引中 879 事件去重到 76 唯一身份的高去重率（92%）未深入归因——可能是同位置多级别事件，也可能是 R1 收束导致的身份合并。

---

## 附：源码证据索引

| 文件 | 行 | 内容 |
|---|---|---|
| admission.rs | 540, 547–550 | seg_c_full 登记到 by_end_multi |
| admission.rs | 662–706 | typed_lookup_multi 键查询 |
| admission.rs | 224–240 | 身份桥确证记录（值桥定义） |
| level_view.rs | 773–800 | trend 域 seg_c_full = pair.seg_c |
| level_view.rs | 874–880 | pan 域 seg_c_full = structure.seg_c |
| nest.rs | 396–411 | NestEventIdentity 定义 |
| nest.rs | 607–628 | terminal_bits_in_book CWindow |
| bsp.rs | 114–115 | source_index = L0 K 序位置 |
| signal.rs | 399 | make_first_point(end.source_index) |
| interp.rs | 71–75 | Candidate.source_index |
