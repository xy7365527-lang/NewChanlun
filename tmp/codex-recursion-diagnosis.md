# 蜂群递归缺失诊断报告

**诊断模式**: diagnose
**诊断对象**: 蜂群递归层次缺失——体系声明"无限向下递归"但实际只有一层
**审查工位**: codex-challenger（Sonnet 4.6，本次为内部诊断，未调用外部 Codex API）
**日期**: 2026-02-28

---

## 一、结论

**根因分类**: A（声明-能力缺口）+ B（触发条件缺失）的复合缺口，主根因是 B。

递归机制在规则层完整声明，但没有任何实现层机制将"递归是默认模式"这一声明转化为工位实际行为。
这是一个**声明完备、触发链断裂**的系统性缺口。

---

## 二、精确缺口定位

### 2.1 声明层（规则存在）

以下文件完整声明了递归：

| 文件 | 声明内容 |
|------|---------|
| `.claude/skills/swarm-architecture/SKILL.md` 原则15 | "真递归拓扑异步自指蜂群是默认模式（不需要理由），扁平 Lead→Worker 是退化特例（使用需要理由）" |
| `.claude/skills/sub-swarm-ceremony/SKILL.md` | "真递归是默认模式。只有当子任务不可分解时，才在当前层直接执行（扁平退化特例）" |
| `.chanlun/dispatch-dag.yaml` task_template.rules | "蜂群递归是默认执行模式，不是复杂度触发的优化——任何多目标任务默认并行 spawn" |
| `069号谱系` | "整个体系和元编排必须完全递归拓扑异步自指化" |

声明层完备，且对偏差有明确的负担分配（扁平是退化特例，需要理由）。

### 2.2 触发链（断裂点精确定位）

**断裂发生在工位 spawn 阶段**——ceremony_scan.py 输出工位列表后，Lead 在 spawn 每个工位时**不注入递归触发条件**。

具体断裂：

**断裂点1：spawn prompt 未注入递归判断指令**

`dispatch-dag.yaml` task_template 定义了三基因（topo_address / depth_budget / parent_callback），但这三个字段只是声明模板，没有任何机制强制 Lead 在构建 spawn prompt 时注入它们。Lead spawn 工位的 prompt 内容由 Lead 自由构建，RLHF 倾向是给出足够任务描述就停止，不会主动添加"你是否应该创建子蜂群？"的判断指令。

**断裂点2：sub-swarm-ceremony skill 的触发条件是自判断，无硬触发**

`sub-swarm-ceremony/SKILL.md` 触发条件：
```yaml
triggers:
  - event: TeamCreate（由 teammate 而非 Lead 发起时）
```
问题：没有任何机制让 teammate 感知到"我应该去评估自己是否需要创建子蜂群"。skill 的触发条件是 `TeamCreate 由 teammate 发起时`——但这是描述 skill 何时被调用，而不是触发 teammate 去做这个判断的机制。因果倒置：skill 等待 TeamCreate 事件，但没有机制触发 TeamCreate。

**断裂点3：CLAUDE.md skill 索引的触发条件是描述性的**

CLAUDE.md skill 表中 sub-swarm-ceremony 的触发条件：
> "teammate 需要创建子蜂群时"

这是一个**条件命题的前件描述**，不是触发机制。`需要` 是未定义的判断——工位不知道自己什么时候"需要"。

**断裂点4：ceremony_scan.py 不输出子蜂群触发信号**

`ceremony_scan.py` 扫描 roadmap.yaml、session 遗留、pending 谱系，但输出的工位条目没有字段指示"此工位应递归分解"。`parallel_group` 字段控制的是**同层并行**，不是**向下递归**。
子蜂群触发信号完全缺失于 scan 输出格式。

### 2.3 Agent 定义层（无递归指令）

读取所有 agent 定义文件，没有任何一个 agent 的 prompt 中包含以下类型的指令：
- "评估当前任务是否可以递归分解为子蜂群"
- "如果子任务 ≥2 个独立单元，执行 sub-swarm-ceremony"
- "检查 depth_budget 是否允许继续递归"

工位被 spawn 时接收到的是任务描述，不是含有递归判断的行为规范。

---

## 三、根因分类

**主根因：B（触发条件缺失）**

递归机制本身存在于 sub-swarm-ceremony skill 中，但工位没有任何触发点去执行这个 skill。没有：
- hook 在 spawn 时注入递归评估指令
- ceremony_scan 输出的子蜂群信号
- spawn prompt 模板包含递归判断

**次根因：A（spec-execution-gap）**

体系在规则层声明了"递归是默认"，但实现层（spawn prompt 构建）不强制这一点。声明与实际不一致。这与 `spec-execution-gap` skill 覆盖的经典模式完全吻合。

**排除 C（平台限制）**

Claude Code 的 Task tool 支持嵌套 TeamCreate，在 sub-swarm-ceremony 中有明确的实现指南（095号谱系正式化了平台能力）。平台不是瓶颈。

**排除 E（任务粒度）**

编排者明确说"默认模式"，排除了"当前任务恰好不需要递归"的解释。且从实际运行观察，蜂群从未有工位发起过 TeamCreate。

**D（RLHF 压制）是次要因素**

RLHF 倾向完成任务而不是 spawn 子蜂群，这与主根因（触发条件缺失）协同作用——即使有触发信号，RLHF 也会倾向自己执行而不是递归。这与 Lead 串行残余（218号谱系）是同构问题：触发条件缺失 + RLHF 基底抵抗 = 递归从未发生。

---

## 四、修复方案

### 方案A：spawn prompt 模板注入（最小侵入）

在 Lead spawn 每个工位时，强制注入递归判断指令：

```
你是 {team_name}/{agent_name} 工位。

[递归判断——必须执行]
在开始执行任务前，评估：
1. 当前任务是否可以分解为 ≥2 个独立子任务？
2. 如果是：读取 .claude/skills/sub-swarm-ceremony/SKILL.md 并执行子蜂群创建
3. 如果否：在当前层直接执行（扁平退化特例，需要记录理由）

depth_budget: {N}  # depth_budget=0 时禁止递归，直接执行
parent_callback: {team_name}/{recipient}
topo_address: {team_name}/{agent_name}
```

这需要修改 Lead 的 spawn 行为规范（dispatch-dag.yaml task_template 或 swarm-architecture skill）。

### 方案B：ceremony_scan 输出子蜂群信号

在 ceremony_scan.py 输出格式中增加 `recursive_spawn` 字段：

```json
{
  "name": "task-X",
  "recursive_spawn": true,   // 此工位应评估是否创建子蜂群
  "depth_budget": 2
}
```

Lead 读取 `recursive_spawn=true` 时，在 spawn prompt 中插入递归评估指令。

### 方案C：hook 强制注入（最强硬约束）

在 `PostToolUse`（Task tool 调用后）hook 中检测 spawn prompt 是否包含递归判断指令，若缺失则拦截并注入。

这与 016号谱系一致："规则没有代码强制就不会被执行"。hook 是最接近强制的机制。

**推荐执行顺序**：A（最小可行）→ 验证效果 → 必要时加 B/C。

---

## 五、边界条件

本诊断的结论在以下条件下翻转：

1. 如果存在未被读取的 hook 或 skill 确实注入了递归指令（本诊断读取了主要文件，但不排除遗漏）
2. 如果 Lead 实际确实在某些场景下执行了 TeamCreate（但编排者观察"从未递归"）
3. 如果平台有未文档化的嵌套 team 限制（排除可能性较高，因为 sub-swarm-ceremony 明确声明了路径）

---

## 六、影响声明

- 涉及文件：`.claude/skills/swarm-architecture/SKILL.md`（原则15声明）、`.claude/skills/sub-swarm-ceremony/SKILL.md`（流程声明）、`dispatch-dag.yaml`（task_template 三基因）、`ceremony_scan.py`（输出格式）
- 修复需要修改：spawn prompt 构建逻辑（在 Lead 的行为规范中）
- 不需要修改：现有 skill 内容（sub-swarm-ceremony 内容完备）、platform 能力（平台支持）
- 对应谱系：016号（规则没有代码强制就不会执行）、137号（否定性禁令对行为执行层无效）、218号（Lead 串行残余的同构问题）

---

## 七、与已知谱系的对应

此缺口与 137号谱系（否定性禁令对行为执行层无效）高度同构：

- 137号：声明"不允许串行"→ 串行仍然发生 → 修复：强制输出格式（正面规则）
- 本缺口：声明"递归是默认"→ 递归从未发生 → 修复方向：强制 spawn prompt 包含递归判断（正面规则）

两者的根因相同：**否定性声明对 RLHF 基底约束无效，必须转换为正面格式强制**。
