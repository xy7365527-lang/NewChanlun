---
id: "211"
number: 211
type: 概念发现
status: 已结算
date: 2026-02-25
trigger: 合成数据 4 中枢 3 confirmed trends 但 level 2 无法形成中枢——direction 语义缺口
depends_on:
  - "195"
  - "208"
audit_targets:
  - "递归 direction 语义缺口"
---

# 211号：递归引擎 direction 语义缺口——level ≥2 中枢无法形成

## 结论

合成数据产生 4 中枢 → 3 confirmed trends → 递归引擎尝试 level 2，但 `_try_init_center` 的 direction 检查阻止了中枢形成。

根因链：

1. `_try_init_center` 要求 s1.direction == s3.direction（缠论原文：中枢由同向线段构成）
2. level 2 的 moves 是 TrendTypeInstance，其 `direction` 属性继承自内部结构（第一个线段的方向）
3. 3 个 confirmed trends 的 direction 为 [down, down, up] → s1(down) ≠ s3(up) → "Center skip"
4. 价格区间完全重叠（[80, 180]）但 direction 不匹配 → 中枢无法初始化

### 语义分歧

| 层级 | direction 含义 | 来源 |
|------|---------------|------|
| level 1（Segment） | 线段的价格方向（up/down） | 笔的特征序列分型 |
| level 2（TrendTypeInstance） | 走势内部第一个线段的方向 | 继承自 level 1 结构 |

在 level 1 中，s1 和 s3 同向是中枢定义的一部分（连续 3 线段，s1/s3 同向，s2 反向）。
在 level 2 中，TrendTypeInstance 的 direction 不代表它作为高级别 "线段" 的方向——它代表内部结构的方向。

### 缠论原文语义

缠论递归中，高级别的 "线段" 是低级别的走势类型实例。走势类型实例作为高级别线段时，其方向应由其起止价格决定（start_price vs end_price），而非内部结构的方向。

## 定义依据

- 缠论递归定义（§6）：Move[k] = TrendTypeInstance[k-1]
- 中枢定义（§4）：至少 3 线段有公共重叠区间，s1/s3 同向
- `_try_init_center`（a_center_v0.py:243）：direction 检查

## 边界条件

- 如果 TrendTypeInstance 增加 "作为高级别线段的方向" 属性（基于 start/end 价格），direction 检查可能通过
- 如果去掉 direction 检查（只看价格重叠），可能产生不符合缠论定义的中枢
- 日线数据因中枢数量不足（≤2），从未触发此问题——只有合成数据暴露

## 下游推论

1. TrendTypeInstance 需要增加 `as_move_direction` 属性（基于 start/end 价格）→ [resolved: start_price/end_price + as_move_direction property 已实现]
2. `_try_init_center` 在 level ≥2 时应使用 `as_move_direction` 而非 `direction` → [resolved: getattr 优先读 as_move_direction]
3. 修复后合成数据应能产生 ≥2 层递归 → [resolved: 合成数据 10 settled centers → 9 confirmed trends → level 2 形成 1 center]
4. λ 参数校准依赖此修复（需要多层递归数据）→ [unblocked: level 2 已可形成，待校准]

## 谱系引用

- 195号：缠论代码拓扑化
- 208号：递归深度瓶颈诊断
