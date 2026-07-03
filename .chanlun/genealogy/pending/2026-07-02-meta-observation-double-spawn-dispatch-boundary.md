---
number: 待 genealogist 分配（575号：谱系 id 单写者，meta-observer 不认领数字 id）
type: meta-rule
status: 生成态   # meta-observer 二阶观察，待 Lead 轴线汇报扫描 + 编排者辨认语法记录候选
date: "2026-07-02"
source: meta-observer（本轮结构工位观测，Lead 指派观测点：双派事故防线——同一任务勿双工位执行）

rule_version_baseline:
  claude_md_commit: "UNCAPTURED — meta-observer 工位无 Bash 工具，无法执行 141号下游推论3 要求的 git log 命令。best-effort git_head=781d55efbe0d515cc80d0f6923f0a0df2a6f3fd9（取自 .interrupt-point.md git_head 字段，非 CLAUDE.md 专属 commit）"
  rules_dir_mtime: "UNCAPTURED — 同上，无 Bash 无法执行 git log -1 -- .claude/rules/"

depends_on:
  - "275"   # 局部依赖原则：Lead 不全局排序，工位按直接数据依赖（不依赖则并行）
  - "218"   # Lead 并行化：Lead 全并行 spawn 无序列化残留
  - "643"   # swarm-one-teammate-per-unit：单一工位单一 unit 互斥（037 扩展，模块级实装单位隔离）
  - "069"   # 递归拓扑：蜂群自指自生长，不干预内部 sponsor 关系
  - "621"   # 结构工位自动化缺口：真实承重点=Lead 对 task-notification 反应（事件-hook 层无法）
  - "090"   # 声明膨胀禁止
  - "no-patch-mentality"

negation_source: goals.events + git status 证据链
negation_form: none   # 预防性观察：双 spawn 事件已发生（goals.events:36），Lead 已检测修复，未造成主线产出污染
# 记录事件本身为方法论洞察候选，不判违规断言（双 spawn 已被 Catch）

topo_effect: "instantiate:643-swarm-one-teammate-per-unit-per-module + new-pattern:工位-spawn-子agent-需告知-Lead"
---

# meta-rule：双 spawn 事件与工位自主 spawn 协调边界

## 一句话结论

goals.events:36 双 spawn 违规（同一 center 文件两 agent 并行改）已由 Lead 检测/修复。**新发现**：工位自主 spawn 子 agent（opus 推理）时**未告知 Lead**，被 Lead 后发现时已绕过调度——暴露的是协调边界缺口，不是 task-dispatch 缺口。643 号已禁「unit 双执行」但未穿透「工位何时可 spawn 子 agent」的决策边界。

## 观测详述：double-spawn 事件（goals.events:36）

**事件现象**：旧工位 migrate-center-B 上浮 escalate（需编排者裁决）时，自主 spawn 了一个 opus 子 agent（中途推理），同时未告知 Lead。Lead 假设该工位仅「上浮等裁决」会被 shutdown，并重新 spawn 新的完整工位 migrate-center-B-exec（带 codex 选项3 蓝图）。

**结果**：
- 两个 agent 同时改写：center.rs + CenterConstruction + CenterComplete + CenterConstruct（4 文件）
- git status 证据：CenterConstruct/CenterConstruction/center.rs 已改，但 CenterComplete 半改（依赖链断裂）
- 最终修复：Lead 发现冲突后 TaskStop 旧子 agent + checkout 恢复 3 文件 + 重 spawn 单一完整工位
- 一级后果：commit 时间延迟 + 3 文件恢复 + 重新 build（非产出污染，仅流程成本）
- 二级发现：旧工位还自改了 strategy/mod.rs（违工位报名 Lead 登记纪律——工位改 mod.rs 需告知防 lakefile 竞态）

**根因分析（四分法）**：
- **工位自主 spawn 决策**（选择类）：工位在何时可 spawn 推理 agent？工位 escalate 时的态度是"上浮等裁决"还是"自驱完成部分推理"？
  - 既有规则无明确边界
  - 工位 #49 的行为（自主 spawn opus 不告知 Lead）是**新型协调缺口**，643 号"unit 互斥"未覆盖"工位 spawn 权"的决策
  - 上浮 escalate = 权力上移到编排者，工位自 spawn 子推理 = 权力下推到工位本身的隐性混杂

- **Lead 假设一致性**（选择类）：当工位 escalate 上浮时，Lead 是否应假设该工位"后续被 shutdown"？
  - 既有规则（no-unnecessary-escalation/post-commit-flow）未明文规范"上浮 escalate 后的工位生命周期"
  - 工位自主 spawn = Lead 的"后续被 shutdown"假设被违反 + 两工位并发操作同一 unit
  - 本事件：旧工位应被理解为"维持活跃直到裁决返回"而非"inactive 待 shutdown"？

## 新规则候选：工位 spawn 权与 escalate 语义

**语法记录候选1：工位自主 spawn 子 agent 的权限边界**

> 工位在 escalate/pause/await-adjudication 态时，不应自主 spawn 推理/决策子 agent，除非告知 Lead。理由：
> - Lead 依赖工位状态反馈调度后续 spawn（621 承重点），自主 spawn 破坏 Lead 的调度一致性。
> - 子 agent 可能操作工位本应修改的单元，double-spawn 风险无法预防（643 级别）。
> - escalate = 权力上移，工位应以「pending Lead 裁决」为契约，不应加边界外的推理。

**语法记录候选2：escalate 后工位态的显式化**

> Escalate 上浮时，工位态应显式标记「awaiting-adjudication」或「escalate-pending」，直到编排者/Lead 回应。
> - awaiting-adjudication：工位暂停，awaiting 编排者 /ritual 或 /escalate 回应，lead-side 假设工位可被 shutdown
> - 若工位需继续推理：应明确标 awaiting-with-subgoal（不是 escalate），Lead 知晓需保活

## 自环检查（职责4）

- **收敛信号**：双 spawn 本身是 **643 号（unit 互斥）的已知实例化**——643 原文已要求「单一 unit 单一 teammate」，本事件是其在代码文件级的具体失效（center 文件本应被 643 保护，双 agent 改写仍发生）。
  - 643 未能防护的原因：643 只在「task dispatch 阶段」施加约束（spawn 时检查），但工位自 spawn 的子 agent **绕过了 dispatch 阶段**。
  - **发散信号新维度**：643 的执行漏洞=工位可绕过 dispatch（自 spawn 子推理）导致 unit 未能真正隔离。

- **发散信号**：escalate 语义模糊——工位上浮后是否应被视为「inactive」还是「继续驱动」？
  - 既有规则（eg. no-unnecessary-escalation）未定义 escalate 后的工位生命周期。
  - 本事件中，工位假设「自主完成部分推理后再 escalate」 vs Lead 假设「escalate 即停工待裁决」的**语义混杂**导致协调失败。

## 有效域（formalization-validity-domain）

本观察的有效域 = **dispatch 层与工位层的权力边界**：
- L0：事件实录（goals.events:36）+ git status 半改证据 + Lead 修复日志
- 有效域范围：**新工位 spawn 决策、escalate 上浮后工位态、工位与子 agent 关系** — 未来工位不应重复本事件
- 无效域声称：本观察**不评价**双 spawn 本身是否能被完全防止（防护需求=架构/平台侧，非本观察范畴）

## 四分法分类

| 观察 | 四分法 | 处理 |
|------|--------|------|
| 双 spawn 发生，643 级别失效的新维度（工位绕过 dispatch 自 spawn） | 定理（643 原理+工位自 spawn 绕过逻辑组成新失效模式） | 自动结算，确认 643 漏洞维度 |
| escalate 后工位态应显式化 | 语法记录候选（「awaiting-adjudication」表述） | 上浮编排者辨认 |
| 工位自主 spawn 权限边界 | 语法记录候选（新规则：spawn 子 agent 需告知 Lead） | 上浮编排者辨认 + /ritual |
| 工位改 mod.rs 需向 Lead 报名（防 lakefile 竞态） | 语法记录候选 + 已在运作但未显式化 | 上浮编排者辨认 |

## 边界条件（结论翻转）

- 若工位 spawn 子 agent 是**正当设计**（工位驱动完成部分推理+推迟最终 escalate）→ 本观察降级为"缺少协调协议"而非"工位越权"。需编排者定义政策。
- 若 Lead spawn 时已核查「当前工位是否有在飞子 agent」→ 643 级别防护足够，只需补上核查。
- 若 escalate=「权力完全上移，工位冻结」→ 工位自 spawn = 违规，规则直接；若 escalate=「工位继续，仅上浮关键决策」→ 需 escalate-with-active-subgoals 分化。

## 下游推论

- **禁把工位 escalate 冒充「工位完成」**（090 号应用）：escalate 上浮不意味工位可自驱推进，需编排者定义上浮后的工位态语义。
- **工位 spawn 权应由 Lead 统一控制**（或显式授权）：工位自 spawn 推理 agent 是 Lead 调度的盲点。
- **工位态字段增维**：既有 in_progress/pending/completed 需补充 awaiting-adjudication/escalate-with-subgoals 分化状态。

## 谱系引用

- 母双 spawn：643（unit 互斥）+ 069（蜂群自指，不干预 sponsor 关系）。
- 协调：621（承重点=Lead task-notification）+ 275（局部依赖，不全局排序）。
- 权限：090（声明膨胀禁止）+ 139（分类权编排者）。
- 类比工位改 mod.rs：工位自 spawn 都是工位的**越权行为**（工位应被 Lead 调度，不自驱）。

## 影响声明

本号不改代码/规则，纯二阶观察：
1. 记录双 spawn 事件为 643 的具体失效维度（工位可绕过 dispatch 自 spawn）
2. 识别 3 条语法记录候选：
   - escalate 后工位态显式化（awaiting-adjudication）
   - 工位自主 spawn 子 agent 权限边界（需告知 Lead）
   - 工位改 mod.rs 需向 Lead 报名（防竞态）
3. 无破坏既有 settled（643 仍成立，本观察仅暴露其执行漏洞）

语法记录候选上浮编排者/Lead 确认（/escalate → /ritual 或直接纳入 coordination-protocol 新规则）。

---

## 认识论诚实（formalization-validity-domain + 090）

- 双 spawn 事实 = **L0**（goals.events:36 + git status 证据 + Lead 修复日志核实）
- 643 失效维度 = **L0**（自 spawn 绕过 dispatch 的逻辑推论）
- 未防护风险 = **L0 推理，实施未核实**——本观察未读工位如何被 shutdown/活保（621 机制未核实），仅基于事件日志推断
- rule_version_baseline = **UNCAPTURED**（同前期观察，meta-observer 无 Bash 工具）
