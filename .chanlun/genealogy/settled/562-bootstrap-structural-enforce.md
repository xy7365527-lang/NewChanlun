---
id: "562"
title: "bootstrap 结构工位从文本提示升格为机制强制——075 扬弃（结构=teammate）"
type: "语法记录"
status: "已结算"
date: "2026-06-23"
depends_on: ["095", "096", "137", "155"]
related: ["072", "075", "069", "145", "016"]
negation_source: "编排者裁决（结构工位「就得这样」——teammate，必须 spawn）"
negation_form: "Aufhebung（扬弃 075号）——结构=skill 被否定，孤岛动机被 095/096 团队共享 inbox 化解后保留"
negates: ["075"]
negated_by: []
---

# 562号：bootstrap 结构工位从文本提示升格为机制强制

**类型**：语法记录（已在 095/096 + team-topology.json 运作但未机制化强制）
**状态**：已结算
**日期**：2026-06-23

## 矛盾

蜂群 ceremony 后应 spawn 6 常设结构工位（meta-lead / genealogist / quality-guard /
code-verifier / meta-observer / topology-manager，见 `.claude/team-topology.json`），
但该 bootstrap 只以**文本提示**存在：

- `.claude/hooks/agent-team-bootstrap.sh`（095/096号）：SessionStart 注入"ceremony 后自动
  spawn 6 结构工位"的 systemMessage/additionalContext **文本**。
- `.claude/hooks/agent-team-enforce.sh`：PreToolUse on Task，检查 team_name + 两基因，
  **但不检查结构工位是否 spawn**。
- `.claude/hooks/ceremony-completion-guard.sh`：Stop-Guard 的 5 个检查中**无结构工位检查**
  ——075号下游推论 5 显式移除了它（"Stop hook 不再检查 dominator node 存在性"）。

结果：bootstrap 是纯文本提示，无机制强制。**本 session Lead 实际跳过了 bootstrap**——
只 spawn 业务工位 + 按需结构工位（genealogist 经 geneal-p4/geneal-560 覆盖），缺
meta-lead / quality-guard / code-verifier / meta-observer / topology-manager 五个。

这是 137号的精确复现：**否定性文本提示对行为执行层无效**——072号"hook 强制双前提"
（提示 + 机制）只满足了提示前提，缺机制前提。

## 075 vs 095/096 的定义冲突（编排者已裁决）

| 号 | 主张 | 结构工位形态 |
|----|------|-------------|
| 075号 | 结构工位从 teammate 转为 skill + 事件驱动（消除孤岛） | skill |
| 095/096号 + team-topology.json | 结构工位是 `auto_spawn: true` 的 teammate | teammate |

**编排者裁决**：结构能力是 teammate（spawn，095/096），非纯 skill 事件驱动（075）。

这不是不可弥合的矛盾，而是一次 **Aufhebung（扬弃）**：

- **否定**：075 的"结构工位 = skill，ceremony 不再 spawn"被否定。
- **保留**：075 的动机是消除"孤岛"（隔离 subagent 无共享状态、prompt 膨胀、硬编码
  agent 列表）。095/096 真递归 Agent Team 模式提供**共享 inbox**
  （`$HOME/.claude/teams/<session>/inboxes/`），结构 teammate 不再是信息死角——
  孤岛动机被团队模式从根上化解，故"恢复为 teammate"不重建孤岛问题。
- **提升**：结构工位本体是常设 teammate；075 的 skill + 事件驱动层下沉为
  **轻量守卫**（Write 后 quality-guard skill、Stop 时 meta-observer skill 等事件触发），
  与结构工位 teammate 本体并存，不再互斥。

## 修复（机制强制）

在 `ceremony-completion-guard.sh` 新增**检查 1.5**（放在检查 1 死寂检测之后、检查 2
任务队列之前）：

**检测信号（严格可靠，非模糊匹配）= team config.json 的 `member.agentType`。**

- `agentType` 是 `Task(subagent_type=…)` 落盘的规范结构类型。业务命名（display name）
  如 `geneal-p4` / `geneal-560` / `topo-mapper` **不改变** `agentType=genealogist`
  ——故业务命名的结构工位自动算对应结构类已覆盖（满足编排者"已 spawn 放行"要求）。
- 经验证据：本 session `geneal-p4`、`geneal-560` 的 `agentType` 均为 `genealogist`；
  检测精确判定缺失集 = `[meta-lead, quality-guard, code-verifier, meta-observer,
  topology-manager]`，与实际跳过的 5 个完全一致。
- **仅对 LEAD session 生效**：`config.leadSessionId == 本 session_id`
  （teammate session 不匹配 → 跳过；teammate 无法也不负责 spawn 结构工位）。
- **无 team（未形成蜂群）→ 不强制**：solo 会话无蜂群约束，结构工位仅在蜂群语境强制。
- 缺任一结构 agentType → block + 路由"立即并行 spawn 缺失的结构工位 [具体列表]"。

**145号兼容**：沿用检查 3/4 的计数器写法（增量 `COUNT` + 存 `PRE_ACTIVE_TASKS`），
连续 3 次任务态不变由顶部智能熔断放行，避免死锁。

## 边界条件（结论翻转条件）

1. 若平台改变 `Task(subagent_type=…)` → `member.agentType` 的落盘语义（agentType 不再
   记录 subagent_type），检测信号失效 → 需改用其他规范信号（如 task owner / 显式标记）。
2. 若结构工位形态再次被裁决回 skill（075 复辟），检查 1.5 应移除——但需先化解
   095/096 真递归团队模式提供的共享状态优势如何在 skill 模式下保留。
3. 若 team config.json 的 `leadSessionId` 不再等于 Stop hook 的 `session_id`，
   lead 识别失效 → 检查 1.5 退化为全跳过（fail-safe，不误报）。
4. team-topology.json 的 6 个 `auto_spawn: true` 列表变更 → 需同步更新检查 1.5 的
   `required` 常量（当前硬编码，未来可从 team-topology.json 读取以消除双源）。

## 下游推论

1. 075号下游推论 5（"Stop hook 不再检查 dominator node 存在性"）被本号否定——
   Stop-Guard 重新检查结构工位存在性，但以 agentType 信号（非 075 时代的 dominator node）。
2. Lead 跳过 bootstrap 现在会被机制阻断，不再依赖文本提示的概率性引导
   （137号：否定性提示对行为执行层无效的又一次机制化修复）。
3. agent-team-bootstrap.sh 的文本注入仍保留（提示前提），与检查 1.5（机制前提）
   构成 072号"双前提"完整覆盖。
4. 结构工位"业务命名 + 规范 agentType"的双层命名约定被固化为检测契约——
   未来 spawn 结构工位必须用 `subagent_type=<规范结构名>`，display name 可自由命名。

## 影响声明

- **改动文件**：`.claude/hooks/ceremony-completion-guard.sh`
  - 新增检查 1.5（结构工位 bootstrap 强制，约 50 行，含 SESSION_ID 提取 + 检测 python + block 路由）。
  - 更新文件头注释（line 5-9）：记录 075 扬弃——结构能力恢复为 teammate。
- **不破坏**：现有 Stop-Guard 5 个检查（死寂/任务队列/生成态谱系/@proof-required/四分法）
  bit-exact 保留；检查 1.5 纯加性（block 时 early-exit，放行时直通）。
- **影响模块**：Stop-Guard 行为（新增结构工位强制）；间接影响所有 ceremony 后的 Lead 停机判定。
- **不影响**：业务工位 spawn 方式、agent-team-enforce.sh、agent-team-bootstrap.sh。

## 测试

7 场景全绿（`.chanlun/review-results/bootstrap-structural-enforce-fix-20260623.md` 有详录）：
- A 真缺 5 个 → block（genealogist 经 geneal-* 正确识别为已覆盖）
- B1 全 6 present → 不 block；B2 仅 genealogist → 5 缺（genealogist 不在缺失集）；
  B3 drop 单个 → 精确报告该单个
- C 非 lead session → 跳过；D 空 session_id → 无误匹配
- E 145 熔断：counter=3:0 → 放行（避免死锁）

## 谱系依据

- 095号：Agent Team 真递归是默认模式（结构工位 = teammate 的平台基础）
- 096号：无例外——所有 Task 调用通过 Agent Team
- 075号：结构工位从 teammate 转为 skill（本号扬弃）
- 137号：否定性禁令对行为执行层无效——文本提示升格为机制强制
- 072号：hook 强制双前提（提示 + 机制）
- 155号：Stop-Guard owner 标识工位（检测信号设计先例）
- 145号：智能熔断——连续 3 次状态不变放行
- 016号：规则没有代码强制就不会被执行
