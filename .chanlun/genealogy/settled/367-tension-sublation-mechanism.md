---
id: '367'
title: 张力扬弃（Aufhebung）机制——从调和到跨层级推进
type: 工程
status: settled
date: 2026-03-05
session: v161
depends_on:
  - '089'   # 扬弃的定义（否定+保留+提升）
  - '359'   # 张力消化审计（tension-audit.yaml 数据来源）
  - '134'   # ceremony_scan 张力扫描的初始实现
source: tension-sublation 工位
---

# 367号：张力扬弃（Aufhebung）机制

## 背景

编排者洞察："张力不应该被调和，而是应该被扬弃。因为张力如果调和就变成了停止的不动点，但实际上解决了一个问题在不同的层面会开启或结算其他问题。"

当前 ceremony_scan.py 的张力处理是简单计数（081号/134号）——发现 tensions_with 边 → 输出数量 → 生成工位。这是"调和"模式：张力被处理后标记 resolved，到此为止。

089号定义扬弃三维度：否定（不再是活跃矛盾）+ 保留（认识被保留）+ 提升（在更高层级开启新问题）。本工位将此定义应用于张力处理流程。

## 实现

### 新增文件

- `scripts/tension_sublation.py`：张力扬弃处理器
  - `derive_sublation_event(tension, root)` → 从 resolved 张力推导 sublation 事件
  - `scan_sublation_events(root)` → 批量扫描 resolved 张力
  - `scan_historical_sublation_events(root)` → historical 张力的范式转换扬弃
  - `scan_ongoing_tensions(root)` → ongoing 张力（不产生 sublation）
  - `generate_elevated_gangmu_candidates(events)` → 提取可注入纲目的 elevated 条目
  - `update_tension_audit_with_sublation(root, events)` → 写回 tension-audit.yaml

- `tests/test_tension_sublation.py`：23 个测试

### 修改文件

- `scripts/ceremony_scan.py`：tension_scan 部分升级为扬弃感知
  - ongoing 张力 → 生成工位（保留原行为）
  - resolved 张力 → 产出 sublation_events_count（不再重复生成工位）
  - 降级逻辑：sublation 模块不可用时回退到原始计数

### sublation 事件数据结构

```python
{
    "type": "sublation",
    "tension_from": "007",
    "tension_to": "008",
    "resolved_by": "010",
    "negated": {
        "tension_edge": "007↔008",
        "description": "...",
        "reason": "被 010号 否定——不再是活跃矛盾",
    },
    "preserved": {
        "insight": "...",
        "from_entry": "007",
        "to_entry": "008",
        "resolver": "010",
    },
    "elevated": {
        "level": "cross_layer",  # 或 "structural_insight" / "paradigm_shift"
        "resolver": "010",
        "resolver_title": "...",
        "implications": ["..."],
        "description": "...",
    },
    "timestamp": "...",
}
```

### 三种 elevation 级别

| 级别 | 触发条件 | 纲目注入 |
|------|---------|---------|
| cross_layer | resolver 有 downstream_implications | 候选注入 |
| structural_insight | resolver 无下游推论 | 不注入 |
| paradigm_shift | historical 张力（框架被否定） | 不注入 |

## 推导链

1. 编排者洞察：张力调和 = 不动点（停滞），扬弃 = 跨层级推进
2. 089号已定义扬弃三维度：否定+保留+提升
3. tension-audit.yaml（359号产出）提供张力分类数据
4. 对 resolved 张力：negated=原张力否定, preserved=认识保留, elevated=在resolver层级开启的新推论
5. elevated 维度连接到纲目注入管线——解决问题不是终止，是跨层级传播

## 边界条件

1. 当 tension-audit.yaml 不存在或为空 → 空列表，不报错
2. 当 resolver 谱系无 downstream_implications → elevated 降级为 structural_insight
3. ceremony_scan 中 sublation 模块导入失败 → 降级为原始计数逻辑

## 影响

- `scripts/tension_sublation.py`：新增
- `scripts/ceremony_scan.py`：tension_scan 区块修改
- `tests/test_tension_sublation.py`：23 个新测试

## downstream_implications

1. gangmu_inject 可消费 `generate_elevated_gangmu_candidates` 输出，将 cross_layer 级别的 elevated 条目注入纲目
2. 后续新 resolved 张力自动产出 sublation 事件——蜂群可通过 elevated 推论自主推进，不需要编排者指定下一步
3. tension-audit.yaml 的 sublation 字段为异质审计提供扬弃过程的可观测数据

## 来源

`[元编排]` — 089号扬弃定义在张力处理层面的应用
