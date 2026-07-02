---
id: 待/ritual编号（日期前缀避 §4 编号碰撞，同 claim10/maimai#4 惯例）
timestamp: 2026-07-02
status: 生成态
type: bias-correction
negation_source: homogeneous
negation_form: separation
topo_effect: "split:codex#1-active-set:local（候选）——CLOSED 节点分裂：审计标记语义保留 + load-bearing 门语义移除；/ritual 核 codex#1 谱系号后确认"
responsible_agents: [编排者]
---

# goal-loop active-set 歧义域收窄到 live goal — CLOSED 从 load-bearing 门降为审计标记

**来源**：蜂群运行时（goal-system；无缠论原文对应，codex#1 active-set 语义派生）
**commit**：b69cffb355（#30 goal-loop dangling-active 根治）；取舍链见 commit message + #30 任务记录

## 矛盾（root cause）

dangling-active bug：goal 被 terminated 但未发 CLOSED 事件 → 悬置在 active-set → goal-loop 无法收敛。
根因=**CLOSED 事件被当作 active-set 的退出门**（load-bearing gate）：terminated-without-CLOSED 的 goal 因缺 CLOSED 而仍被算作 active，即「已完成的 goal 仍被当作竞争方向」。

## 推导链

CLOSED-as-gate 预设「active ⟺ ¬CLOSED」→ 但 goal 的完成事实是**验收**（CHECK_PASS），不是 CLOSED 事件 → 二者可脱耦（已验收但未 CLOSED = dangling）→ 精化成员判据：
- **active = live goal**（未验收的竞争方向），非「¬CLOSED」
- **goal_ready 从验收事实（CHECK_PASS）派生**，不等 CLOSED 事件（同 630 号「open acceptance 直接派生 goal_ready_workstations」的从验收派生模式，commit 6fba4696e8）
- **CLOSED 降级**：从 active-set 的 load-bearing 门 → 纯审计标记（记录 goal 生命周期终点，不参与 active 判定）

歧义域（competing directions）由此收窄到 live goal——**已完成 goal 不是竞争方向**。

## 谱系链接

- **codex#1**（active-set 语义，本条=其精化对象；/ritual 补谱系号）
- 630（从 open acceptance 派生 goal_ready_workstations，同「验收派生」模式）
- 655（GOAL_SET 写入端非幂等）、658（reducer acceptance rollup schema）——goal-system 同域，同批 /ritual
- **域同构**（一句）：`已完成 goal ≠ 当前竞争方向` ≅ 缠论 completed 走势不参与当前歧义域竞争（歧义域=live 未完成方向）——T-递归歧义域收窄的运行时映像

## 影响

- `scripts/goal_reducer.py` / goal-loop 收敛判定（active-set 成员判据 CLOSED→CHECK_PASS 派生）
- 下游推论：任何依赖「¬CLOSED=active」的 goal-system 代码须改判据；CLOSED 不得再作门（防重引入 dangling-active）
- 无 settled 谱系被回溯破坏；无缠论定义受影响
