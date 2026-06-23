# 异质审计深化报告：论点A/B/C 反驳分析 (#72 补充)

**审计工位**：codex-challenger（topo_address: swarm/codex-72）  
**被审对象**：swarm-infra 三论点（来自 swarm-recursion-fix 报告）  
**审计模式**：review（深化质询三具体论点）  
**日期**：2026-06-23  
**认识论等级**：L2（逐行代码 + 架构逻辑推演，基于已读代码文件）  
**前置报告**：`codex-audit-swarm-recursion-72-20260623.md`

---

## 论点A：基因不能 block，只能 advisory（by-instruction 论证）

**swarm-infra 的论证**：Agent 工具无结构化基因 param，prompt 是自由文本，substring 检测会误杀 → fail-open/032 → advisory 是唯一合理选择。

### Codex 质询结论：论证成立，但结论应标注严格的翻转条件。

**支持部分（不可否认）**：

- 当前 harness Agent 工具参数集：`name / subagent_type / prompt / description / run_in_background / isolation`。无 `metadata`，无结构化基因字段。
- 现有检测路径：`'topo_address' not in prompt.lower()` — 字符串存在性检查，非语义有效性。未填充的 placeholder 字面字符串 `topo_address: <swarm/工位名>` 仍能通过检查（F2 已发现）。
- Block 误杀风险：若 Lead 在某调用中省略基因，block 会产生不可恢复的蜂群死锁（无 retry 机制）。Advisory 保留降级通道。

**advisory 的真实理由不是"自由文本无法解析"，而是"误判→死锁无恢复路径"**。这个区分非常重要——如果只记住"自由文本无法解析"，未来 harness 即使增加了结构化字段，代码也不会主动升级到 block 模式。

**翻转条件（论证应明确写出）**：

- **若 harness 在 Agent 工具增加 `metadata` 结构化 param**（如 `metadata.topo_address: string`），enforce hook 可精确匹配，误判率→0，此时 BLOCK 正确，§1#3 结论翻转。
- 当前 advisory 是**harness 约束下的最优**，不是架构永久选择。swarm-infra 报告应写："advisory 依赖 harness Agent 无结构化基因 param 这一事实；该事实改变则结论改变。"

**F2 深化**：即使 harness 不变，"present-by-construction" 命名比原报告描述更严重。当前机制的保证链：

```
bootstrap 注入 template → Lead context window → Lead 手写 prompt → enforce 检测字符串
```

第三步（Lead 手写 prompt 时自愿包含基因）是 RLHF 概率行为，不是结构约束。"by-construction" 在工程语义中要求"无效状态不可产生"，当前机制不满足。

**正确命名**：`"present-by-instruction + advisory-detected + RLHF-dependent"`

---

## 论点B：(c)×275 正交性是否掩盖真矛盾

**swarm-infra 的论证**：spawn 物理层中心化（Lead 执行）⊥ 依赖逻辑局部（每节点声明自己的 blockedBy）→ 正交可调和。

### Codex 质询结论：正交论证基本成立，需精确区分"全局可见性"与"全局调度智能"。

**论证成立的部分**：

- 275号 禁止的具体行为：全局优先级排序 / 越级管理子蜂群 / 中心化依赖调度 / 全局工位评估。
- Lead 的 scan-spawn 规则：`spawn ALL (status=pending, owner=None, blockedBy=empty)` — 无优先级，无跳过，无越级干预。
- blockedBy 的设置者是创建该任务的 teammate（局部声明），Lead 只读结果，不管理依赖内容。
- 这与操盘类比正确：主级别只看次级别，次级别管次次级别；Lead 只执行 spawn，不干涉工位内部拓扑。

**潜在张力（承认但不构成矛盾）**：

Lead 的 scan 给了它**全局 TaskList 可见性**（看到所有任务）。纯局部依赖原则下，每节点只看自己的直接下游，无需全局视图。Lead 的全局视图是 harness spawn-source 的**必然附产物**，不是架构设计选择。

这是**性能层张力**（Lead 忙碌 → 子任务等待时延），不是**架构层矛盾**（依赖逻辑仍由各 teammate 局部声明）。

**正交论证要再精确一步才严格**：

> "Lead 的全局可见性是 harness spawn-source 的必然副产物，不引入调度智能；275号 禁止的是调度智能（优先级排序/跨级干预），不禁止 spawn-source 的全局视图。"

**翻转条件**：若 Lead 开始读取 `priority` 字段并优先 spawn 高优先级任务 → 违反 275 号（全局优先级排序）。当前实现未做此操作 → 正交成立。

---

## 论点C：(c) 是否真的消解了 096"无孤岛 subagent"约束

**问题**：(c) 是完全消解，还是把孤岛问题换了个形式残留？

### Codex 质询结论：(c) 实现了架构层解决，未实现机制层封闭。两者必须分开声明。

**架构层解决（已完成，正确）**：

正确路径：`teammate → TaskCreate(子任务) → Lead scan → Agent(name=T, ...) → T 是命名 teammate → 进入隐式 team → peer 身份 → 可收 SendMessage，可写 TaskList`。

在正确路径下，任意递归深度的 Agent 均有 peer 身份，096 满足。四节点 DAG（task/review/audit/crystallization）全部由 Lead 以命名形式 spawn，peer 状态有保证。

**机制层残余缺口（未封闭）**：

1. **错误路径仍开放**：任何 teammate 在任意时刻调用 `Agent(no name, prompt=...)` 仍然语法合法 → 产生真正的孤岛 subagent → 直接违反 096。
2. **enforce hook 只能 advisory**（fail-open/032，不 block）：错误路径的摩擦仅是文字提示，不是结构性拦截。
3. **Lead 自身的错误路径**：Lead 若调用 `Agent(no name, ...)` → enforce 会 block（is_lead=True, name=None → emit_block）。但此时 swarm 死锁（无法 spawn 任何工位）。

因此：违反 096 的路径从"技术上可行"变为"有 advisory 摩擦的可行"，不是"不可能"。

**(c) 的正确表述**：

> "(c) 提供了 096 合规的唯一正确路径（架构层消解）；错误路径因 harness 限制无法结构性封闭，保留 advisory 摩擦作为降级保障（机制层 advisory-only）。若未来 harness 允许 hook block Agent 工具调用（对所有 session），则机制层亦可封闭。"

---

## 综合判定（深化版）

| 论点 | 深化结论 | 与原报告差异 |
|------|----------|-------------|
| A（advisory vs block） | 成立，但需明确标注 harness 翻转条件 | 原报告未显式化翻转条件 |
| B（(c)×275 正交） | 成立，但需区分"全局可见性"与"全局调度智能" | 原报告精度不足一步 |
| C（096 消解完整性） | 架构层消解，机制层 advisory-only 残缺口 | 原报告未区分两层声明 |

### F2 升格建议

**F2（原：METHODOLOGICAL CONCERN）→ 建议升格 HIGH**

理由：C 项分析显示，"present-by-construction" 的声明精度问题不只是命名歧义，而是掩盖了 096 在机制层的真实残缺口。若此声明在下游被理解为"096 已被结构性封闭"，则未来面临 harness 升级机会时，代码不会主动从 advisory 升级到 block。声明精度直接决定修复路径的可见性。

---

## 边界条件

| 翻转条件 | 影响论点 | 方向 |
|----------|----------|------|
| harness 增加结构化 metadata param | A | advisory → block |
| Lead 实现基于 priority 的排序 | B | 正交 → 违反 275 |
| harness 允许 hook block Agent 工具调用 | C | advisory-only → 机制封闭 |

---

## 谱系引用

- **090号**：声明膨胀禁止 —— A 的翻转条件未明确、C 的两层未分离均属声明膨胀
- **032号**：fail-open 不变量 —— A 的 advisory 选择的谱系依据
- **275号**：局部依赖原则 —— B 的判断依据
- **096号**：无孤岛 subagent 约束 —— C 的根本问题
- **137号**：否定性禁令对行为执行层无效（机制化必要性的谱系来源）—— C 中 advisory 不足以封闭违规路径的理论基础
