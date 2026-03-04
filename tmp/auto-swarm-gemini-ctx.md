# 自动蜂群 + 自动 Gemini 讨论机制

## 问题

048号 Stop-Guard 解决了"不让停"，但注入的指令只是"去扫描"。缺少两个自动化机制：

### 机制 1：自动拉递归蜂群

当前状态：Lead 手动评估任务是否可并行，手动决定是否 spawn 蜂群。
期望状态：检测到 >= 2 个独立任务时，自动 spawn 蜂群。

触发条件候选：
- ceremony 完成后，中断点列表有 >= 2 个独立项
- 单个任务执行中发现可拆分的子任务
- Stop-Guard 阻断后的扫描发现多个可并行工作

### 机制 2：自动与 Gemini 讨论

当前状态：Lead 手动判断问题是否需要 Gemini 质询，手动构建上下文文件，手动调用 gemini_challenger。
期望状态：检测到概念层问题时，自动路由到 Gemini decide()。

触发条件候选：
- 谱系写入时，如果类型是"选择"或"定理"，自动调用 Gemini verify
- 代码变更涉及定义文件（.chanlun/definitions/）时，自动调用 Gemini challenge
- 新的 pending 谱系条目创建时，自动调用 Gemini decide 判断四分法分类

### 实现层级选择

这两个机制可以在不同层级实现：
1. **CLAUDE.md 规则层**：写规则"检测到 >= 2 独立任务时必须拉蜂群"——但 016号说文本规则不够
2. **Hook 层**：PostToolUse hook 检测任务创建，自动触发蜂群——但 hook 是 shell 脚本，无法 spawn agent
3. **Skill 层**：ceremony skill 中内置蜂群评估步骤——已有（step 10），但只在 ceremony 中生效
4. **Agent 层**：专门的 orchestrator agent 持续运行，监控任务队列——最强但最贵
5. **Stop-Guard 增强**：在 Stop-Guard 的注入指令中，不只说"去扫描"，而是说"评估是否需要蜂群/Gemini"

## 相关谱系
- 048号：通用停机阻断
- 041号：编排者代理（Gemini 路由）
- 037号：递归蜂群分形
- 043号：自生长回路
- 046号：一域一角

## 需要 Gemini 决策

1. 这两个机制应该在哪一层实现？
2. 自动蜂群的触发条件是什么？（避免过度 spawn 浪费资源）
3. 自动 Gemini 讨论的触发条件是什么？（避免每个小问题都调用 Gemini）
4. 这两个机制是否应该合并为一个"自动编排"机制？
5. 四分法分类？
