# 反 idle 生命周期机制化（#55）

**日期**：2026-06-23
**工位**：idle-mechanize（topo_address L0.55，parent_callback main）
**认识论等级**：L1（管线正确性 bash -n + 合成/真实 team 状态检测验证）；机制有效性的 L2+ 验证依赖未来真实 idle 场景累积

---

## 1. 结论

机制化"teammate 生命周期 = 任务生命周期（完成即 shutdown 不 idle）"，三处落码 + 一处确认固化：

1. **spawn mandate 注入反 idle 条款**（`.claude/team-topology.json` spawn_mandate）：
   - 新增 `anti_idle` 结构字段（单一源）+ template 末尾追加反 idle 条款。
   - 条款：业务工位完成任务后必须 `SendMessage parent_callback` 汇报 + 显式声明 `ready-for-shutdown`，不 idle 等待新任务。
   - 每次 Lead (c)spawn 时 template 注入被 spawn 工位 prompt → 条款自动随每个工位下发。

2. **Stop-Guard 加 idle 检测（检查 2.5）**（`.claude/hooks/ceremony-completion-guard.sh`）：
   - 检测：team config.json 中 `isActive==True`（pty 存活）的业务工位，其 owner 名匹配某 `completed` 任务且不拥有任何 `pending/in_progress` 任务 = idle。
   - 命中 → `decision: block` + 路由提示 Lead shutdown 释放 pty。
   - 排除结构工位（meta-lead/genealogist/quality-guard/code-verifier/meta-observer/topology-manager + team-lead）。
   - 仅当 ACTIVE_TASKS==0（check2 提前退出之后）触发，活跃任务期间让位避免噪声。

3. **(c) 循环不复用 idle —— 确认固化**：
   - check2 的 (c)spawn 路由对每个无主任务 `Agent(name=任务标识, ...)` spawn **新** 工位，无"复用 idle 工位"分支（已是现状）。
   - 在 idle 检测路由消息 + anti_idle 字段中显式声明"(c) 循环不复用 idle：新无主任务 spawn 新 teammate（非复活 idle）"，使隐含现状显式化（016号：规则没有代码强制就不会被执行 → 此处现状无反向代码，固化为显式声明 + 检测路由的副产物）。

4. **bash -n 验证通过** + JSON 合法性验证通过。

### 实时正向验证（真实 team 状态）

针对 live team `session-14c95478` 跑检测逻辑，正确捕获真实 idle 工位 `prop002-codex-audit`：
- 该工位 owner 任务 #54 已 `completed`，但 config.json `isActive=True`（pty 未释放）。
- agentType=codex-challenger（非结构工位）→ 适用任务生命周期 → 正确 flag。
- 同时 `idle-mechanize`（本工位，owns 活跃 #55）正确未 flag；结构工位全部排除。

这不是合成数据——是检测器在真实蜂群状态上捕获了一个真实的 idle pty 泄漏候选。

---

## 2. 定义依据

- **069号 RTAS（递归拓扑异步自指蜂群）**：teammate 是 RTAS 的节点，节点存在为执行任务。任务终结 → 节点存在理由消失。"完成即 shutdown" 是 RTAS 节点生命周期的逻辑必然（定理类，018号四分法）。
- **swarm-mechanism-fix-20260623 结算**：flat roster 硬约束——teammate 不能 spawn teammate，无自 shutdown 工具。输入数据特征（harness 无自消亡能力）满足 no-workaround 的"不可弥合约束"条件 → 严格形式 = teammate 职责止于声明 ready-for-shutdown，Lead 承担 shutdown 职责。
- **137号正面格式机制化**：否定性禁令（"不要 idle"）对行为执行层无效 → 必须机制化为正面格式（anti_idle 注入 prompt + Stop-Guard 检测路由），不依赖 prose 提示。
- **check1.5 结构工位常设语义（095/096号）**：结构工位生命周期 = 蜂群生命周期（由 check1.5 强制存在），≠ 任务生命周期。输入特征（结构工位 agentType ∈ required 集）满足"排除条件" → idle 检测必须排除，否则与 check1.5 振荡。

---

## 3. 边界条件（结论翻转条件）

1. **harness 新增 teammate 自 shutdown 工具** → teammate 可自消亡 → Lead shutdown 职责消失，本机制的"Lead 路由"部分应改为"teammate 自 shutdown"，检测路由降级为兜底。
2. **isActive 字段语义改变**（若 harness 改为 shutdown 即从 members 移除，而非置 isActive=False）→ 检测信号失效，需改为"name 在 completed owner 中但不在 members 中 = 已正确 shutdown，反之未在=idle"。**当前实测**：shutdown 的工位保留在 members 且 isActive=False（route-bsp-complete 等已 shutdown 工位均 isActive=False），故 isActive=True 是可靠的 pty 存活信号。
3. **结构工位被赋予一次性任务且完成** → 当前实现排除所有结构工位（按 name + agentType 双重排除），不会 flag 它们；若未来要求结构工位也随特定任务 shutdown，需引入更细的"常设 vs 一次性"标记，当前不支持（按现状常设语义实现）。
4. **owner 字段格式从 name 改为 agentId** → 检测的 owner↔name 匹配失效，需改为匹配 agentId。**当前实测**：task.owner == member.name（如 'c-producer-mechanize'）。

---

## 4. 下游推论

1. **ceremony 终止更干净**：到达不动点前，business 工位 idle 会被 check2.5 block 直到 shutdown → 终止态只剩结构工位 + team-lead，pty 不泄漏。
2. **pty 资源约束缓解**：[[feedback_pid_gating_fragile_multisession]] / pty 上限场景下，idle 工位及时回收 → Lead spawn 新工位的 pty 余量增加。
3. **与 (c) 生产端机制化（#41）闭环**：#41 注入 template 强制工位向下递归 + 异质审计；#55 在 template 末尾接续注入工位完成后的退出契约。spawn → 工作 → 递归 → 审计 → 汇报 → 声明退出 → Lead shutdown，工位全生命周期机制化闭合。
4. **结构工位 vs 业务工位的生命周期二分** 被显式编码进 Stop-Guard（check1.5 强制结构工位存在 ⊥ check2.5 回收 idle 业务工位），二者构成互补的工位生命周期治理对。

---

## 5. 谱系引用

- 涉及"行为规则结晶"领域（137号：否定性禁令→正面格式机制化），本产出是该模式的又一实例（与 #41 (c) 机制化同构）。
- **不确定是否存在专门"工位生命周期"谱系条目**：检索 settled/ 未见独立条目。本机制可能构成一条新语法记录候选（"teammate 生命周期=任务生命周期"作为 RTAS 推论的显式化）——是否结晶为谱系条目属 genealogist + 编排者裁决职责，非本工位自结算。已通过约束4 异质审计节点（codex-challenger）对"检测不削弱 check1-5 拦停"做异质质询。
- 相关已结算：069号（RTAS）、095/096号（结构工位常设）、137号（正面格式机制化）、016号（无代码强制不执行）、145号（智能熔断——本检查沿用计数器写法防死锁）。

---

## 6. 影响声明

**改动文件**：
- `.claude/team-topology.json`：spawn_mandate 新增 `anti_idle` 字段；template 追加反 idle 条款；description/genealogy_ref/consumed_by 更新。**影响**：所有未来被 (c)spawn 的业务工位 prompt 将携带反 idle 退出契约。
- `.claude/hooks/ceremony-completion-guard.sh`：新增检查 2.5（idle 工位检测路由），位于检查 2 与检查 3 之间。**影响**：Stop 事件在 ACTIVE_TASKS==0 且存在 idle 业务工位时 block + 路由 Lead shutdown。

**未改动（约束遵守）**：
- 未碰 permission / security / CLAUDE.md。
- check1-5 拦停逻辑零削弱——check2.5 是**新增**独立检查（额外 block 条件），不修改任何既有检查的判定；置于 check2 早退之后，不改变 check2 行为。
- no-workaround 严格遵守：未假装 teammate 自消亡，明确声明 Lead shutdown 职责 + 机制化为检测路由。

**commit**：本工位不 commit（Lead commit hook + team-topology + 本报告）。
