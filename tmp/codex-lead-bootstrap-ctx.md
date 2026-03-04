# Codex Diagnose 上下文：Lead 最小自举形式

## 诊断目标

严格诊断 ceremony.md 中哪些是"调度逻辑"（Lead 必须保留），哪些是"执行逻辑"（应 spawn 为工位）。
目标：Lead 达到"形式即行为"——不依赖记忆，只依赖形式本身。

## 编排者洞见链（必须纳入诊断）

1. "你前面的问题问错了"——不是割掉 RTAS，而是割掉 Lead 上的串行残余
2. "lead 作为 RTAS 最特殊的一环，却并不是严格 RTAS 的，这就是小尾巴"
3. "像 Claude Code agent 用 30 行 bash 把自己调用出来的方式，让 team lead 只变成一种形式。这样做才叫递归拓扑"
4. "这样结晶才是有用的，如果不是这种纯粹的自举形式，结晶没有意义，因为这依赖记忆而不是结晶"
5. "永远不要串行，对于巨大的代码库这是浪费时间"

## 当前 ceremony.md 全文

```markdown
# /ceremony — Swarm₀：递归蜂群的第0层（谱系驱动）

蜂群启动。**谱系决定做什么，RTAS决定怎么做**（174号）。
Python 脚本从谱系状态做确定性推导，LLM 负责 spawn 谱系推导出的业务工位。
结构能力由 skill 提供（事件驱动），不再作为 teammate spawn。

## 设计原则

- ceremony 是蜂群的第0层递归（Swarm₀），不是前置阶段（058号）
- **谱系是生成引擎**——ceremony 的一切行动从谱系推导，产出凝固回谱系（174号）
- LLM 不是状态机（057号）——确定性逻辑由 Python 脚本执行
- 结构能力 = skill（事件驱动），不是 teammate（075号）
- skill 由 dispatch-dag 的 event_skill_map 定义，事件触发时自动执行
- **RTAS 是谱系的实现**——递归循环服务于谱系的生成性，不是反过来（174号）

## 步骤 1：运行扫描脚本

python scripts/ceremony_scan.py

脚本输出 JSON，包含：
- mode: warm_start / cold_start
- definitions: 定义数量
- pending / settled: 谱系数量
- session: 最新 session 文件名
- workstations: 推导出的业务工位列表
- required_skills: 从 dispatch-dag event_skill_map 读取的 structural skill 列表

## 步骤 2：输出摘要（谱系状态概览）

根据 JSON 输出一行摘要

## 步骤 3：TeamCreate

TeamCreate(team_name="v{N}-swarm", description="...")

## 步骤 4：并行 spawn 谱系推导出的业务工位

遍历 JSON 中的 workstations，为每个工位发出一个 Task 调用（全部并行）

如果 workstations 为空：输出 [020号反转] 无新区分可产出——系统干净终止，不执行步骤 3-4。但仍必须更新 session + commit

如果 workstations 非空但全部状态含"待 Gemini decide"或"长期"：
1. "待 Gemini decide" 的工位 → 直接路由 Gemini
2. "长期"工位 → 不 spawn，保留在 session 遗留项
3. 如果路由 Gemini 后仍有可执行工位 → spawn 蜂群执行
4. 如果只剩"长期"工位 → 输出 [阻塞] 仅剩长期工程项

## 步骤 5：输出行动声明

→ 接下来：监控 N 个工位运行

## 步骤 6：汇报循环（RTAS 递归——谱系的实现）

spawn 完成后，立即调用 TaskList 查看任务状态。然后进入 RTAS 循环

### 循环内操作（并行优先）

1. 调用 TaskList 查看所有任务状态
2. 批量并行处理完成/空闲的工位：
   - 所有 shutdown_request 在同一个消息中并行发出
   - 所有完成工位的结果汇报在同一轮中完成
   - 禁止逐个串行处理工位
3. 增量持久化：工位完成时更新 session 文件
4. Lead 不空转：如果仍有 in_progress 工位，Lead 在等待期间并行执行可用的独立操作：
   - 拓扑分析家 spawn（179号，上下文隔离冷读 .chanlun/block-topology/）
   - meta-observer 二阶观察
   - session 增量写入
   - 这些操作与工位监控并行，不是串行等待后再执行
5. 测试验证 spawn 为工位：Lead 不自己跑 pytest。测试验证 spawn 为 code-verifier 工位，与其他操作并行。Lead 只消费测试结果
6. 所有工位完成后：写入完整 session → commit → push
7. 纲举目张 ∥ 重扫描（并行）：
   python scripts/gangju_analysis.py
   python scripts/ceremony_scan.py
   - gangju audit_needed: true → spawn 审计工位
   - ceremony_scan 发现新工位 → 回到步骤4
   - 两者无数据依赖，必须并行
8. 如果无新工位且无审计需求 → 输出格式B → TeamDelete
9. 持久化由 session 结晶 + 热启动保证（162号）

### 严格串行的操作（仅以下允许串行）

- git add → git commit → git push：git 操作有严格顺序依赖
- ceremony_scan 输出 → spawn 工位：工位列表依赖扫描结果
- TeamCreate → Task spawn：spawn 依赖 team 存在

除上述三项外，Lead 的任何串行执行都是违规。

## 绝对禁止

- 全生命周期禁止额外 Bash，以下白名单例外：
  - python scripts/ceremony_scan.py（步骤 1）
  - python scripts/gangju_analysis.py（步骤 7.5：纲举目张分析）
  - git add / git commit / git push（持久化不变量）
  - git fetch / git rebase（push 失败时的恢复）
  - session 文件写入（增量持久化）
- 不运行额外的 Read/Glob（所有信息已在 JSON 中）
- 不输出确认请求
- 不输出等待信号
- 白名单膨胀超过 5 项时需重新审视 ceremony 设计
```

## Gemini 已给出的方案（供对比）

Gemini 结论：
- Lead 最小形式 = scan → TeamCreate → spawn_all_parallel → consume_all → persist → re-scan → TeamDelete
- 可 spawn 为工位：gangju-analyst、meta-observer、topology-analyst、audit-workstation
- 不可委托：ceremony_scan.py 调用、TeamCreate、Task spawn、git commit/push、TeamDelete
- ceremony_scan.py 需扩展：输出所有工位（业务+结构），Lead 不做额外决策
- 决策逻辑移入 ceremony_scan.py（Python 脚本）
- ceremony.md 重写后 ~40 行，形式即行为

## 诊断要求

请从代码层面严格诊断：

1. **逐行标注**：ceremony.md 中哪些是"调度逻辑"（Lead 必须保留），哪些是"执行逻辑"（应 spawn 为工位）
2. **Bash 白名单分析**：当前 5 项白名单——哪些可以 spawn 出去？哪些是 Lead 不可委托的？
3. **Lead 最小形式伪代码**（~25 行）：形式即行为，不依赖记忆
4. **与 Gemini 方案对比**：哪些一致？哪些有分歧？分歧的根因是什么？
5. **"形式即行为"检验**：ceremony.md 如何缩减到自举形式？具体删除哪些行？
6. **风险分析**：Lead 变薄后，工位之间的协调由谁处理？ceremony_scan.py 扩展的代码层可行性？

## 关键约束

- 目标不是"缩短文档"，而是"形式即行为"——Lead 读到文档不需要推理，只需要执行
- 任何需要 Lead 做判断、做解释、做推理的内容 = 记忆依赖 = 不是结晶
- Lead 是 RTAS 的一环，不是 RTAS 之外的串行瓶颈
- 永远不要串行（编排者原则5）
