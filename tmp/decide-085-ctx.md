# Gemini Decide 请求：085号审计孤岛修复策略（五问题独立决断）

## 决断背景

本次审计（085号谱系）对元编排系统进行了全面核查，识别出以下五个需要价值判断的"选择"类决断。
每个问题独立决断，互不阻塞。

---

## 已结算谱系（约束条件）

### 073b号：Trampoline 递归近似（已结算）

当前 Claude Code 平台硬约束：subagent 不能 spawn sub-subagent。
所有 agent 扁平于 Lead 的 L1 子节点。
递归的实际机制是 Trampoline（状态层伪递归）——Lead 反复弹跳，agent 通过任务裂变写回队列。
这是"调用栈递归"的平台降级形式，保留递归本质属性（任务分解递归性），牺牲结构属性（嵌套层级）。

### 082号：半事件驱动 D策略（已结算）

结论：hooks 检测+提示 + Lead 作为调度器响应 + 诚实降级文档。
事件总线的物理载体降级为 Lead 本身（hooks 检测，Lead 认领执行）。
这是"Lead 认知疲劳"边界的风险，但比强制阻断更稳健。

### 075号：结构工位从 teammate 转为 skill + 事件驱动（已结算）

结构工位不再作为 teammate spawn，而是作为 skill 的指令源。
dispatch-dag 定义 event_skill_map（事件→skill 映射）。

### 041号：编排者代理（已结算）

"选择"和"语法记录"类决断路由 Gemini decide 模式。
人类保留 INTERRUPT 权。

### 018号：四分法（已结算）

- 定理：自动结算
- 行动：自动执行
- 选择：路由 Gemini decide
- 语法记录：路由 Gemini decide

---

## 五个待决断问题

### 问题 1：递归从未发生（P0）

**现状**：
- 系统声明"递归拓扑异步自指蜂群"（069号、原则15）
- 9 轮蜂群（v12→v21）全部是 Lead→Worker 单层结构
- fractal_template 是死代码，从未实例化
- 递归终止条件从未被触发
- ceremony_scan.py 只推导第一层工位，不推导子蜂群
- 073b号已结算：平台约束下递归只能通过 Trampoline（ceremony 链 v12→v13→...→v21）近似实现

**三个选项**：

**A: 实现子蜂群嵌套**
- Worker 能 spawn 子 Worker
- 递归终止由 020号背驰+分型检测
- 工程成本：高（平台硬约束，需要绕过 Claude Code 不支持 sub-subagent 的限制）
- 073b号已明确：当前平台无法实现真正调用栈递归

**B: 修改声明——去掉"递归"**
- 诚实地改为"拓扑异步自指蜂群"
- 工程成本：零（只改 CLAUDE.md 和相关谱系描述）
- 代价：069号、056号等大量谱系需要更新

**C: 保留"递归"声明但标注为"trampoline 近似"**
- 073b号已记录平台约束，当前是 Trampoline 近似
- Trampoline 保留递归本质属性，只是机制降级
- 工程成本：零（只需在 069号/CLAUDE.md 中添加注释）
- ceremony 链（v12→...→v21）本身就是 Trampoline 的历时递归展开

**决断标准**：
1. 声明与现实的一致性（073b号已记录 Trampoline 降级，C选项是一致的）
2. 工程成本 vs 收益（A成本极高，B成本零但信息损失大，C成本零且保留信息）
3. 系统整体一致性（C与073b号已结算谱系一致）

---

### 问题 2：claude-challenger 完全孤岛（P1）

**现状**：
- claude-challenger.md 文件存在（.claude/agents/claude-challenger.md）
- dispatch-dag 声明了 re_challenge 和 stale_generative 两个触发事件
- 无任何 hook 或代码路径能触发它
- 084号审计：该 agent 在 dispatch-dag event_skill_map 中声明"✅"，但实际无触发路径

**三个选项**：

**A: 添加 hook**
- 在 PostToolUse/Write 中检测谱系写入，自动触发 claude-challenger
- 工程成本：中（需要写 hook 脚本，但 hook 输出只是提示，Lead 还是要手动认领——082号 D策略）
- 实际效果：与 B 相同，只是多了一条"提示"路径

**B: 从 dispatch-dag 移除**
- 承认 claude-challenger 不需要自动触发
- 调整 dispatch-dag 中的 event_skill_map 记录
- 工程成本：低（改一行 yaml）
- 代价：失去了"claude 对 Gemini 的逆向质询"这条链路的声明

**C: 保留为手动触发的 skill（/challenge 命令）**
- 当前设计意图就是按需使用（由 gemini-challenger 的结果触发，或由编排者手动调用）
- dispatch-dag 记录保留，但触发方式改为"手动 / gemini-challenger 输出后认领"
- 工程成本：零
- 与 082号 D策略（Lead 认领）一致

**背景信息**：
- claude-challenger 的职责是"对 Gemini 产出再质询，防止单向质询退化"
- 083号谱系中 claude-challenger 被提及作为质量守门
- 实际使用场景：每次 gemini-challenger 决断后，claude 应检验 Gemini 的推理

---

### 问题 3：source-auditor 完全孤岛（P1）

**现状**：
- source-auditor.md 文件存在（.claude/agents/source-auditor.md）
- dispatch-dag 声明了 file_write(docs/**) 触发
- settings.json 无对应 hook

**三个选项**：

**A: 添加 hook**
- 在 PostToolUse/Write 中检测 docs/** 写入，自动触发溯源检查
- 工程成本：中
- 实际效果：hook 输出提示，Lead 认领（082号 D策略）

**B: 从 dispatch-dag 移除**
- 工程成本：低

**C: 保留为手动触发的 skill**
- 当 docs/ 内容被修改时，编排者或 Lead 手动调用
- 工程成本：零
- 背景：docs/** 修改频率远低于 src/**，手动触发可能足够

**背景信息**：
- source-auditor 的职责：三级权威链核查，原始博文溯源
- 缠论体系中溯源正确性是关键（一级权威 = 博文，二级 = 编纂版，三级 = 思维导图）
- docs/ 修改频率：相对低频

---

### 问题 4：skill-crystallizer 无事件连接（P2）

**现状**：
- skill-crystallizer.md 文件存在
- 声明了 pattern_buffer_ready 触发
- post-session-pattern-detect.sh 不连接到 crystallizer
- 084号审计：precompact-save.sh 在 Windows 上完全失效（python3/路径问题），导致整个链路断裂
- pattern-buffer.yaml 存在但不会被自动更新

**三个选项**：

**A: 实现完整的 pattern_buffer_ready → crystallizer 连接**
- 修复 precompact-save.sh（python → python3/python）+ 修复路径
- 修复 post-session-pattern-detect.sh 连接到 crystallizer
- 工程成本：中（需要修复 2-3 个脚本）
- 前提：precompact 必须先修复（084号 P0 级别）

**B: 移除 pattern-buffer 机制，简化为手动结晶**
- 编排者或 Lead 在发现模式时手动调用 skill-crystallizer
- 工程成本：低（移除几个 hook 脚本，清理 dispatch-dag）
- 代价：失去自动模式识别和结晶

**C: 保留声明，标注为"长期工程项"**
- pattern-buffer 机制正确，但修复依赖 precompact 修复（已在 084号标记为 P0）
- 当 precompact 修复完成后自然激活
- 工程成本：零（但需要写入谱系明确依赖关系）

**背景信息**：
- 知识结晶是系统自生长的核心（原则11、043号谱系）
- precompact-save.sh 的修复是 084号的 P0 工程项

---

### 问题 5：meta-observer 被降级（P2）

**现状**：
- 082号决策后，meta-observer-guard.sh 默认 hotfix 模式自动放行
- 实际上"每个 session 结束时通过 meta-observer-guard.sh 触发二阶反馈"的声明名存实亡
- 084号：meta-observer-guard.sh 标记为"退化 P2"

**三个选项**：

**A: 恢复自动执行（每次 session_end 强制触发）**
- 反转 082号决策，强制模式替代 hotfix 模式
- 工程成本：低（改一行 shell 脚本，改 DEFAULT_MODE 变量）
- 风险：082号已识别这会带回阻断问题（历史上曾因阻断导致需要 hotfix）

**B: 明确记录为"暂缓"**
- 082号的决断是合理的（减少阻断），继续当前状态
- 写入谱系明确说明"二阶反馈暂缓，当系统成熟度提升后重新评估"
- 工程成本：零

**C: 降级为建议性输出（运行但不阻断，只在 session 中记录观察）**
- meta-observer 正常运行，但产出只是记录到 session 而不阻断
- 这是 082号 D策略的精神：提示 + Lead 认领
- 工程成本：低（改 meta-observer-guard.sh 的输出格式）

**背景信息**：
- 082号 D策略边界之一：如果 Lead 认知疲劳，D 退化为 B（纯靠记忆）
- 077号谱系记录了 meta-observer 的功能：识别系统级模式、二阶反馈
- 系统当前有 85+ 条谱系，复杂度已经很高

---

## 决断格式要求

对每个问题单独给出：
1. 选择哪个选项（A/B/C）
2. 核心推理（2-4句）
3. 边界条件（什么情况下翻转）

五个问题可以并行给出，也可以按顺序。
