---
type: meta-rule
status: 生成态
title: "幽灵 owner 认领——无 owner 任务被不当 teammate 抢认领 + 死 teammate 名 owner 拦 Stop-Guard"
date: "2026-06-23"
负责工位: meta-observer
rule_version_baseline:
  claude_md_commit: "eeddbdc14e4ff028ed3e6e529f2d7066a62e9960"
  rules_dir_mtime: "2026-03-14 23:27:42 +0000"
depends_on: ["155", "145"]
related: ["096", "275", "562"]
negation_form: "implementation-gap + 角色边界（既有孤儿检测只覆盖 owner=None；死 teammate 名是新变体）"
memory_refs: ["reference_stopguard_zombie_tasks", "feedback_task_queue_owner_liveness"]
---

# 元规则观测：幽灵 owner 认领与任务 owner 生命周期缺口

**类型**：meta-rule（方法论洞察，机制缺口 + 角色边界）
**信号**：收敛（孤儿 in_progress 拦 Stop-Guard 主题已记录）+ 发散（死 teammate 名 owner，绕过 owner=None 判据）

## 结论

任务 owner 生命周期存在二连机制缺口，导致「幽灵 owner 认领」：

1. **无显式 owner ⟹ 不当 teammate 抢认领**：任务创建时无 owner，质询类 teammate（gemini-challenger）轮询抢先认领了**实装(#29)+hook修复(#30)** 这两个非质询工位的活——质询工位做实装 = 角色边界违规。
2. **死亡不释放 ⟹ 孤儿 in_progress 拦 Stop-Guard**：自动 spawn 的 `gemini-challenger-dispatch` 完成/死亡后不释放已认领任务，留下 owner=死 teammate 名 的孤儿 in_progress。SendMessage 返回 `No teammate named gemini-challenger-dispatch on team`，但 Stop-Guard 检查2 仍计为活跃任务持续拦停。

## 定义依据 / 核验

- L0 任务系统核验：当前 #29 owner=`prop4-bidir`（in_progress，正确实装工位）、#30 owner=`swarm-mechanism-fix`（completed）——幽灵 owner 已被重分配，恢复确认。Lead 现场报告坐实抢认领→死亡→孤儿序列。
- 155号（settled）：Stop-Guard 用 owner（agent name）而非 subject 标识工位——本观测扩展：owner 还须判**活性**（是否为活 teammate），非仅判存在/非空。
- 096号：所有 Task 调用通过 Agent Team——抢认领发生在 team 内轮询窗口。

## 边界条件（结论翻转条件）

- 若平台改为「任务创建必带 owner」（无轮询抢认领窗口），缺口(a)消失。
- 若 teammate 死亡时平台自动释放其 in_progress 任务（owner 置空或重入队），缺口(b)消失。
- 若 Stop-Guard 检查2 增加 owner 活性校验（对照 team config.json members），死-owner 孤儿不再拦停。
- 若质询类 teammate 被禁止认领实装/hook 任务（角色门控），抢认领的角色违规面消失。

## 下游推论

- 「任务创建即指派 owner」= `feedback_task_queue_owner_liveness`（认领前验原主存活）已运作隐性规则的机制化（语法记录）。
- 「死 teammate 名 owner 识别为孤儿」= 既有 owner=None 孤儿判据的逻辑必然扩展（定理）。
- 机制修复与 #30（swarm-mechanism-fix：强制 teammate mode / spawn 机制）同域，可并入其后续。
- **构成性冲突防护**：若修复触及 dispatch-dag.yaml，meta-observer 提案须经 gemini-challenger 异质审查。

## 谱系引用

- 155号：Stop-Guard owner 标识工位（被本观测扩展为 owner 活性校验）。
- 145号：智能熔断（孤儿拦停的现有兜底，但治标）。
- 562号：bootstrap 结构工位强制（同一 Stop-Guard 文件的相邻机制化先例）。
- MEMORY `reference_stopguard_zombie_tasks`（owner=None 陈旧快照）+ `feedback_task_queue_owner_liveness`（认领前验原主存活）——同源主题。

## 复现证据（同一规则版本基线）

- 2026-06-23 本 session：#29/#30 被 gemini-challenger-dispatch 抢认领→死亡→孤儿→重分配（任务系统 L0 + Lead 现场）。
- 跨 session 收敛：MEMORY 两条记录孤儿/陈旧 owner 拦 Stop-Guard 主题。
- 新变体（死 teammate 名 ≠ None）= 发散维度，绕过既有检测 → 达结晶阈值。

## 影响声明

- 本记录为 meta-observer 观测，**不携带谱系号**（genealogist 结算时分配）。
- 建议处置：genealogist 结算为语法记录+定理 → Lead 经 /ritual（或并 #30 后续）修任务 owner 生命周期 + Stop-Guard owner 活性校验 + 质询工位角色门控。
- 不改动任何代码/已结算谱系（meta-observer 只观测）。
