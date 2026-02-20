---
id: "006"
title: "级别递归 vs 多周期下钻"
status: "已结算"
type: "概念分离 + 架构决断"
date: "2026-02-16"
depends_on: []
related: ["003", "005b"]
negated_by: []
negates: []
---

# 概念分离 006：级别递归 vs 多周期下钻

**状态**: 已结算
**结算时间**: 2026-02-17
**结算依据**: level_recursion.md v1.0 已结算，编排者决断（递归级别为唯一路径）已全面实现（P1-P9，106测试全GREEN）
**创建时间**: 2026-02-16
**类型**: 概念分离 + 架构决断
**域**: 级别（Level）、中枢（Zhongshu）、走势类型（Move）
**来源**: 编排者指出"多周期在这个项目中应该是不用的，除非能找到周期配合从下到上的递归的方式"

---

## 分离描述

"级别"在缠论实践中有两种理解：
- **递归级别**：从最低 K 线递归构造，每层由下层的走势类型组件构成
- **时间周期级别**：为不同 TF（5m, 30m, 1h...）独立运行管线

两者在概念上不等价，在实现上不互通。

## 编排者决断（2026-02-16）

**递归级别为唯一正式路径。多周期独立管线不在本项目中使用。**

TFOrchestrator 降级为工程参考/调试工具，不作为核心引擎路径。

## 分离后的影响

### 需要实现的 → ✅ 全部完成

1. ✅ **Center[k≥2] 的构造函数**：`zhongshu_from_components()`（P2）
2. ✅ **Move 统一接口**：`MoveProtocol` + `SegmentMoveAdapter`（P1/P3）
3. ✅ **递归调度引擎**：`RecursiveLevelEngine`（P4）+ `RecursiveOrchestrator`（P8）
4. ✅ **Move 完成判定**：`settled` 标记驱动（P4/level_recursion #2）

### 需要明确的 → ✅ 全部已结算

1. ✅ Move[k-1] 完成判定：`settled=True` 时触发上层构造（level_recursion #2）
2. ✅ 递归深度限制：`max_levels` 可配参数 + 自然终止（level_recursion #3）
3. ✅ "真中枢"暂态处理：中枢 `settled` 标记分离结构层与动力学层

## 谱系链接

- 前置：003-segment-concept-separation.md（Move[0] = v1 Segment 为唯一口径）
- 前置：005b-object-negates-object-grammar.md（递归是对象产生对象的语法规则）
- 定义文件：`.chanlun/definitions/level_recursion.md` v0.1
- ~~阻塞：beichi.md #5 区间套~~ → ✅ 已解除（beichi.md v1.1 已结算）
