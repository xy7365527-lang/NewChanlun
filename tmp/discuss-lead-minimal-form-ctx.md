# 讨论：Lead 作为最小自举形式——递归拓扑的严格闭合

## 编排者的洞见

"像 Claude Code agent 用 30 行 bash 把自己调用出来的方式，让 team lead 只变成一种形式。这样做才叫递归拓扑。"

## 核心命题

当前 Lead 是一个"胖节点"——承载了扫描、测试、审计、持久化、监控等大量逻辑。
这违反了 069号的递归拓扑原则：Lead 应该和工位同构。

### 类比：Claude Code 的自举模式
Claude Code agent 用一个极简的 bash 脚本把自己调用出来。脚本本身不包含任何业务逻辑——它只是一个启动器。所有实质性工作由被调用的 agent 完成。

### Lead 应该是什么
Lead = 最小自举形式 = 一个只做以下事情的调度器：
1. 读取状态（ceremony_scan.py 输出）
2. 并行 spawn 所有工位（包括测试工位、审计工位）
3. 消费工位结果
4. 持久化（session + commit + push）

Lead 不应该：
- 自己跑 pytest
- 自己做 gangju 分析
- 自己做 meta-observer 观察
- 自己做拓扑分析
- 逐个处理工位结果

所有这些都应该是工位——和业务工位同构的节点。

### 这意味着什么
- ceremony_scan.py 和 gangju_analysis.py 的调用也应该是工位（或者至少是并行的 Bash 调用）
- meta-observer 是工位
- 测试验证是工位
- Lead 的 ceremony.md 可以缩减到 ~30 行：读状态 → spawn → 消费 → 持久化

### 需要讨论的问题
1. Lead 的最小形式具体是什么？能否用伪代码描述？
2. ceremony_scan.py 调用本身是否也应该 spawn 为工位？还是说它作为 Lead 的"感知器"可以保留在 Lead 内？
3. git commit/push 是否也应该 spawn 为工位？还是说持久化是 Lead 的不可委托职责？
4. 如果 Lead 变成纯调度器，谁来做"决策"？（比如：gangju 说 audit_needed=true，谁决定 spawn 审计工位？）
5. 这个变更对 ceremony.md 的具体影响是什么？

## 约束
- 069号：递归拓扑异步自指——Lead 必须在拓扑内
- 090号：严格性语法规则——Lead 的胖节点形式不严格
- 057号：LLM 不是状态机——确定性逻辑由脚本执行
- 创世 Gap：Lead 作为 Swarm₀ 的一部分，有不可消除的特殊性（但应最小化）

## 请给出
1. Lead 最小形式的伪代码（目标 ~30 行）
2. 哪些当前 Lead 职责可以 spawn 为工位
3. 哪些职责是 Lead 不可委托的（创世 Gap 的最小残余）
4. 具体的 ceremony.md 重写方案
