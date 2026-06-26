# SecondKind 第二特征序列动态确认状态机 — 设计方案（#84）

**owner**: cc-parser-finish ｜ **文件**: `rust/src/theta_v0/parser/`（新增 `second_kind.rs` + 改 `segment.rs`）
**日期**: 2026-06-26 ｜ **认识论等级**: L0（结构判据）+ L1（对 Python 参考交叉验证）

## 1. 问题（L2 核心阻塞）

cc-backtest 诊断（OKLO OOS 160,836 bar）：`divide_segments` 只断 `FirstKindConfirmed`，遇
`SecondKindPending` 即停 → 真实数据 9,760 笔 → 仅 1 线段 → <3 无中枢 → 无 bsp → 无订单 → 非 L2。
真实数据大部分线段需 SecondKind（有缺口）动态确认，FirstKind 段不足。

## 2. bit-exact 前置评估结论（Lead 派工要求的首步）

| 层 | 来源 | 状态 |
|---|---|---|
| SecondKind 静态判据 | Lean Claim10（classifyTermination 缺口→secondKind、PostBreakOutcome 古怪线段、wellFormedV1 五硬约束） | Rust segment.rs **已对齐** |
| SecondKind 动态状态机 | Lean **有意不形式化**（Claim10:334-346 诚实声明，指向 Python） | Lean **无 spec** |
| 动态确认权威定义 | frozen reference-theta-v0.md:22 + 67课博文原文 | **权威定义** |
| 已验证动态实装 | Python `a_segment_v1.py`（37 测试，编排者裁定 v1 唯一口径） | **交叉验证参考** |

**路径 A（Lead 未否决）**：从 reference:22 + 67课博文定义独立实装 Rust 动态状态机，Python 作
交叉验证参考。**不复制** Python 性能启发式（TAIL_WINDOW=7 / MAX_SECOND_SEQ_SCAN=50）——
那些是 Python 工程妥协（非缠论可导），进 config 或诚实标注，不漂入 Lean-等价引擎。

## 3. 67课博文权威定义（动态确认语义）

> 第二种情况：特征序列顶分型中，第一、二元素间**存在缺口**，**如果从该分型最高点开始的
> 向下一笔开始的序列的特征序列出现底分型**，则线段在该顶分型高点结束。底分型镜像。
> **第二个序列分型不分第一二种情况，只要有分型就可以。**

三递进层次（Claim10:164-172，关层次混淆）：
- (a) 笔破坏（触发信号，非终结）
- (b) 特征序列分型形成（可能形不成 = 古怪线段）
- (c) 线段终结（分型形成后按二分类判定）

SecondKind = (c) 层的"有缺口"分支：分型已形成但有缺口 → 须第二特征序列出现反向分型才确认。

## 4. 设计

### 4.1 文件结构（coding-style：segment.rs 已 376 行，抽取新文件）

- **新增 `parser/second_kind.rs`**：第二特征序列动态确认状态机（纯函数 + 扫描逻辑）。
- **改 `parser/segment.rs`**：`analyze_termination` 的 SecondKind 分支调用 `second_kind`，
  确认成功 → `FirstKindConfirmed`-同构的 confirmed 段端；确认失败 → 留 tail（现状）。

### 4.2 核心函数（bit-exact 对齐 67课博文 + Python `_second_seq_has_fractal`）

```
second_kind_confirmed(
    strokes: &[Stroke],      // 全笔序列
    seg_dir: Direction,      // 线段方向
    fractal_apex_offset: usize,  // 首特征序列分型极值点对应笔偏移
) -> Option<usize>           // Some(确认的段端笔偏移) / None（未确认，留 tail）
```

算法（67课博文逐字）：
1. 从分型极值点开始的**反向一笔**（seg_dir 反向）后的序列，取 **seg_dir 方向笔** 构成第二特征序列。
2. 对第二特征序列做包含处理（标准化）。
3. 扫描第二特征序列找**任意分型**（不分第一二种，只要有分型）——67课博文"只要有分型就可以"。
4. 出现分型 → 确认终结，返回段端；无分型 → None（留 tail）。

### 4.3 包含处理 + 分型检测（复用 segment.rs 现有 `Interval` 原语）

- 第二特征序列的包含处理 = 复用 `process_feature_inclusion`（已对齐 Claim10）。
- 分型检测 = 复用 `find_feature_fractal` 的极值判据，但"任意分型"（顶或底都算，不限方向）。

### 4.4 confirmed 判定严格性（Lead 硬约束：不放松标准产假线段）

- 仅当第二特征序列**实际出现分型**才断 confirmed（67课博文充要条件）。
- 未出现 → 严格留 tail（PendingSegment，#82 已实装）。不为"多产线段"放松。
- 古怪线段（笔破坏未发展成线段破坏，Claim10 PostBreakOutcome）→ 不强断，留 tail。

### 4.5 扫描边界（诚实标注，不抄 Python 性能启发式）

Python 用 `MAX_SECOND_SEQ_SCAN=50` 限制扫描窗口（O(n²) 性能妥协）。本实装：
- **默认全扫描**（无窗口截断）——bit-exact 优先于性能（reference 总纲：L0 证明无歧义分类）。
- 若性能成问题（160K bar），扫描窗口作 **config 字段** `parse.second_seq_scan_window`
  （default 0=无限），诚实标注 [设计选择,默认值]，不硬编码 50。

## 5. 验证门

- **真实数据**：`.cache/BZ_1min_2024_raw.parquet`（1 分钟真实，符合 Θ v0 最低级别）跑 parse_layer，
  线段数 >> 1（够构造中枢）。
- **单元测试**：SecondKind golden（缺口+第二特征序列分型→确认）+ property（无分型→留 tail，
  古怪线段→不强断）。
- **交叉验证**：同输入 Rust vs Python `segments_from_strokes_v1` 线段数/端点对比（L1，不要求
  逐位等价 Python 内部中间态，要求线段边界一致）。
- `cargo test theta_v0` 全绿。

## 6. 结果包六要素

1. **结论**：Rust 实装 SecondKind 动态确认状态机（新文件 second_kind.rs），SecondKind 段确认
   条件满足成为 confirmed 线段。
2. **定义依据**：reference:22 frozen + 67课博文（第二特征序列出现反向分型→确认，"只要有分型就可以"）
   + Claim10 静态判据（缺口→secondKind、古怪线段）。
3. **边界条件**：第二特征序列无分型 → 留 tail（不 confirmed）；古怪线段 → 不强断。确认严格。
4. **下游推论**：真实数据线段 >>1 → classifier detect_centers 可构造中枢 → bsp → recognize →
   订单 → L2 可跑（cc-backtest 一跑即出）。
5. **谱系引用**：谱系 003（v0/v1 口径分离）；Claim10（task #37）；编排者 xianduan.md:226-237
   裁定 v1 唯一口径。Python a_segment_v1.py 是 v1 已验证实装（37 测试）。
6. **影响声明**：新增 parser/second_kind.rs；改 segment.rs analyze_termination 的 SecondKind 分支；
   可能加 config 字段 second_seq_scan_window。不碰 classifier/strategy/backtest owner 文件。
