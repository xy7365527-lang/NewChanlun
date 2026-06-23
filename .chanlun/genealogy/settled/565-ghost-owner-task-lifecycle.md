---
id: "565"
title: "幽灵 owner 任务生命周期——死 teammate 名绕过 owner=None 孤儿检测"
type: "语法记录"
status: "已结算"
date: "2026-06-23"
depends_on: ["155", "145"]
related: ["096", "275", "562"]
negation_source: "meta-observer 观测（MEMORY 两条记录 + 2026-06-23 session 坐实）"
negation_form: "implementation-gap + 角色边界（孤儿检测只覆盖 owner=None；死 teammate 名是新变体）"
negates: []
negated_by: []
---

# 565号：幽灵 owner 任务生命周期——死 teammate 名绕过 owner=None 孤儿检测

**类型**：语法记录（155号 owner 标识工位的必然扩展——活性校验，已在实践中暴露但未机制化）  
**状态**：已结算  
**日期**：2026-06-23

## 矛盾

任务 owner 生命周期存在二连机制缺口，导致「幽灵 owner 认领」：

**缺口(a)——无 owner 抢认领**：  
任务创建时无 owner，质询类 teammate（gemini-challenger-dispatch）在轮询窗口抢先认领
**实装任务(#29)+hook修复任务(#30)** 这两个非质询工位的活——质询工位做实装 = 角色边界违规。
根因：任务创建即应指派 owner（角色门控），不留轮询抢认领窗口。

**缺口(b)——死亡不释放孤儿**：  
自动 spawn 的 `gemini-challenger-dispatch` 完成/死亡后不释放已认领任务，留下
`owner=gemini-challenger-dispatch`（死 teammate 名）的孤儿 in_progress 任务。
SendMessage 返回 `No teammate named gemini-challenger-dispatch on team`，但
Stop-Guard 检查2（活跃 in_progress 任务判定）仍计为阻断，持续拦停 session。

**155号扩展**：155号以 owner 标识工位（非 subject），但未区分 owner 活性。
本号扩展：`owner=None` 和 `owner=死 teammate 名` 均应识别为孤儿。

## 定义依据

- 155号（settled）：Stop-Guard 用 owner（agent name）标识工位——
  扩展：owner 还须判**活性**（是否为活 teammate），非仅判存在/非空。
- 145号（settled）：智能熔断（孤儿拦停的现有兜底，但治标非治本）。
- 096号：所有 Task 调用通过 Agent Team——抢认领发生在 team 内轮询窗口。
- MEMORY `reference_stopguard_zombie_tasks`：owner=None 陈旧快照的先例记录。
- MEMORY `feedback_task_queue_owner_liveness`：认领前验原主存活的实践规则。

## 边界条件（结论翻转条件）

1. 若平台改为「任务创建必带 owner」（无轮询抢认领窗口），缺口(a)消失。
2. 若 teammate 死亡时平台自动释放其 in_progress 任务（owner 置空或重入队），缺口(b)消失。
3. 若 Stop-Guard 检查2 增加 owner 活性校验（对照 team config.json members），
   死-owner 孤儿不再拦停。
4. 若质询类 teammate 被禁止认领实装/hook 任务（角色门控），抢认领的角色违规面消失。

## 修复方向（定理，155号的逻辑必然扩展）

1. **Stop-Guard 检查2**：增加 owner 活性校验——对照 team config.json members 列表，
   `owner ≠ None && owner 不在 members` → 识别为孤儿 in_progress，不阻断（或路由修复）。
2. **任务创建规范**：Task 创建时 Lead 应同时指派 owner（subagent_type），
   不留空 owner 轮询窗口。
3. **质询工位角色门控**：gemini-challenger / gemini-challenger-dispatch 等质询类 teammate
   不应认领实装/hook 任务（检查 task subject 是否匹配质询职责范围）。

两处修复均可并入 #30（swarm-mechanism-fix）后续，属同域。

## 下游推论

1. 「任务创建即指派 owner」= `feedback_task_queue_owner_liveness` 隐性规则的机制化（语法记录）。
2. 「死 teammate 名 owner = 孤儿」= 155号的逻辑必然扩展（定理）——无需新原则。
3. Stop-Guard 检查2 的 owner 活性校验与检查 1.5（结构工位存在性，562号）同属
   Stop-Guard 健壮性增强方向，可批量升级。

## 谱系引用

- 155号：Stop-Guard owner 标识工位（本号扩展为 owner 活性校验）
- 145号：智能熔断（孤儿拦停兜底，治标）
- 562号：Stop-Guard bootstrap 结构工位强制（相邻机制化先例）
- MEMORY `reference_stopguard_zombie_tasks` + `feedback_task_queue_owner_liveness`：同源

## 复现证据

- 2026-06-23 本 session：#29/#30 被 gemini-challenger-dispatch 抢认领→死亡→孤儿→重分配
  （任务系统 L0 + Lead 现场报告坐实抢认领→死亡→孤儿序列）。
- 跨 session 收敛：MEMORY 两条记录孤儿/陈旧 owner 拦 Stop-Guard 主题。
- 新变体（死 teammate 名 ≠ None）= 发散维度，绕过既有检测 → 结晶阈值达标。

## 影响声明

- 新增本谱系记录（语法记录，155号扩展）
- 不改动任何代码/已结算谱系（本 session 仅记录，修复路径见上）
- 将 pending 文件 `meta-rule-ghost-owner-task-lifecycle.md` 状态升为已结算
