---
id: "037"
title: "递归蜂群：分形 spawn 结构"
status: "已结算"
type: "语法记录（已在运作的规则显式化）"
date: "2026-02-20"
depends_on: []
related: ["013", "014", "019d", "033"]
negated_by: []
negates: []
---

# 037 — 递归蜂群：分形 spawn 结构

- **状态**: 已结算
- **日期**: 2026-02-20
- **类型**: 语法记录（已在运作的规则显式化）
- **来源**: `[新缠论]`

## 发现

蜂群的 spawn 结构是无限递归的：任何 teammate 面对可分解的子任务时，可以 spawn 子 teammates，子 teammates 继承完整的 spawn 能力，可以继续 spawn 子子 teammates，以此类推。递归深度没有人为限制——终止条件是递归运动本身出现结构完成信号（背驰+分型），与缠论的级别递归同构。

1. CLAUDE.md 第 10 条已声明"复杂度通过数量扩展（spawn 更多 teammates），不通过单个 agent 膨胀"
2. topology-manager 的设计原则已包含"监控 teammates 状态，判断是否需要新增 teammate"
3. Agent Teams 架构本身支持 team_name 参数让子 teammates 加入同一 team

缺失的是显式化：dispatch-spec 没有声明递归 spawn 的规则、可见性、成本控制。

## 推导链

1. 014-skill-card-deck-decomposition：Agent Teams 要求薄路由+分布式指令 [已结算]
2. topology-manager 设计原则：复杂度通过数量扩展 [已在运作]
3. Agent Teams 架构：team_name 参数支持子 teammates 加入同一 team [平台能力]
4. 实践观察：ritual-writer 等任务工位在执行复杂任务时，可能需要进一步分解 [需求]
5. → 递归 spawn 是分布式指令原则的自然延伸，且是无限递归的
6. → 终止条件不是固定深度限制，而是 topology-manager 检测到的背驰+分型信号（与缠论级别递归同构：019d）
7. → 需要显式化规则：spawn 条件、能力继承、可见性、结构终止、G8 递归适用

## 谱系链接

- 前置: 014-skill-card-deck-decomposition（分布式指令原则）
- 关联: 013-swarm-structural-stations（蜂群结构工位框架）
- 关联: 033-declarative-dispatch-spec（dispatch-spec 是行为空间的正面定义）
- 关联: 019d-tension-check-limits（递归终止 = 背驰+分型，非固定深度）
- 关联: bias-single-agent-bypass（G8 无豁免规则递归适用）

## 结算条件

1. [x] dispatch-spec.yaml 增加 recursive_spawning 段
2. [x] 所有 agent 定义增加 Task/SendMessage 工具（递归 spawn 的前提）
3. [x] topology-manager 负责成本控制

## 边界条件

- 如果 Agent Teams 架构不再支持 team_name 参数 → 递归 spawn 机制需要替换
- 如果 teammate 总数超过平台硬限制（当前未知）→ 成为物理约束，但不改变"无人为深度限制"原则
- 递归终止是结构性的（背驰+分型），不是数值性的（固定深度）——如果 topology-manager 的检测机制失效 → 递归可能过度扩张

## 影响声明

- dispatch-spec.yaml: v1.1→v1.2，新增 recursive_spawning 段
- 所有 structural + optional agent 定义: tools 列表扩展
- topology-manager: 新增递归 spawn 成本控制职责
