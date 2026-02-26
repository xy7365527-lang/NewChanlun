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

```bash
python scripts/ceremony_scan.py
```

脚本输出 JSON，包含：
- `mode`: warm_start / cold_start
- `definitions`: 定义数量
- `pending` / `settled`: 谱系数量
- `session`: 最新 session 文件名
- `workstations`: 推导出的业务工位列表
- `required_skills`: 从 dispatch-dag event_skill_map 读取的 structural skill 列表

**这是 ceremony 步骤 1 的 Bash 调用。** 完整白名单见"绝对禁止"节。

## 步骤 2：输出摘要（谱系状态概览）

根据 JSON 输出一行摘要——这是谱系当前状态的快照，决定蜂群此次运动的方向：
```
[ceremony] {mode} | 定义 {definitions} 条 | 谱系 {settled} settled / {pending} pending | HEAD {head}
[ceremony] 工位 {len(workstations)} 个 | skill {len(required_skills)} 个（事件驱动）
```

## 步骤 3：TeamCreate

```
TeamCreate(team_name="v{N}-swarm", description="...")
```

**命名一致性约束**：TeamCreate 的 `team_name` 参数就是蜂群的唯一标识。后续所有操作（Task spawn 的 `team_name`、Stop-Guard 检测）都必须使用同一个名字。系统可能返回自动生成的随机名——忽略随机名，所有引用以 `team_name` 参数值为准。

## 步骤 4：并行 spawn 谱系推导出的业务工位

谱系决定做什么——遍历 JSON 中的 `workstations`（由谱系状态推导而来），为每个工位发出一个 Task 调用（全部并行）：

```
Task(name="{workstation.name 简写}", subagent_type="general-purpose", team_name="{蜂群名}",
     mode="bypassPermissions", run_in_background=true,
     prompt="{workstation.name}: {具体任务描述}")
```

如果 `workstations` 为空：输出 `[020号反转] 无新区分可产出——系统干净终止`，不执行步骤 3-4。**但仍必须更新 session + commit**（记录"干净终止"状态）。

如果 `workstations` 非空但全部状态含"待 Gemini decide"或"长期"：
1. "待 Gemini decide" 的工位 → **不是阻塞**，直接路由 Gemini（041号：选择/语法记录路由 Gemini，不等待人类）
2. "长期"工位 → 不 spawn，保留在 session 遗留项
3. 如果路由 Gemini 后仍有可执行工位 → spawn 蜂群执行
4. 如果只剩"长期"工位 → 输出 `[阻塞] 仅剩长期工程项，无可自主推进的工位`
5. **更新 session + commit**（持久化不变量）

**持久化不变量**：ceremony 的每条退出路径（spawn 蜂群 / 干净终止 / 显式阻塞）都必须以 session 更新 + commit + push 结束。没有例外。push 失败（如 non-fast-forward）时，先 rebase 再重推，不允许跳过。蜂群通过 RTAS 循环持久化——每次 ceremony 清理后，下次 ceremony 从 session 热启动恢复全部状态（162号）。

**注意：不再 spawn 结构工位。** genealogist/quality-guard/meta-observer/code-verifier 等结构能力
由 event_skill_map 定义，在对应事件发生时自动触发（075号谱系）。

## 步骤 5：输出行动声明

```
→ 接下来：监控 N 个工位运行
```

## 步骤 6：汇报循环（RTAS 递归——谱系的实现）

spawn 完成后，**立即调用 `TaskList`** 查看任务状态。然后进入 RTAS 循环——每次循环都是谱系的一次生成-实现-凝固迭代。

**Lead 并行化原则（218号）**：Lead 是 RTAS 的一环，不是 RTAS 之外的串行瓶颈。Lead 的一切操作遵守与工位相同的并行原则——独立操作一律并行，只有严格数据依赖才允许串行。

### 循环内操作（并行优先）

1. 调用 `TaskList` 查看所有任务状态
2. **批量并行处理**完成/空闲的工位：
   - 所有 `shutdown_request` 在同一个消息中并行发出
   - 所有完成工位的结果汇报在同一轮中完成
   - 禁止逐个串行处理工位
3. **增量持久化**：工位完成时更新 session 文件
4. **Lead 不空转**：如果仍有 `in_progress` 工位，Lead 在等待期间并行执行可用的独立操作：
   - 拓扑分析家 spawn（179号，上下文隔离冷读 .chanlun/block-topology/）
   - meta-observer 二阶观察
   - session 增量写入
   - 这些操作与工位监控并行，不是串行等待后再执行
5. **测试验证 spawn 为工位**：Lead 不自己跑 `pytest`。测试验证 spawn 为 code-verifier 工位，与其他操作并行。Lead 只消费测试结果
6. 所有工位完成后：写入完整 session → commit → push
7. **纲举目张 ∥ 重扫描**（并行）：
   ```bash
   # 以下两个脚本在同一轮中并行调用
   python scripts/gangju_analysis.py
   python scripts/ceremony_scan.py
   ```
   - gangju `audit_needed: true` → spawn 审计工位
   - ceremony_scan 发现新工位 → 回到步骤4
   - 两者无数据依赖，必须并行
8. 如果无新工位且无审计需求 → 输出格式B → TeamDelete
9. 持久化由 session 结晶 + 热启动保证（162号）

### 严格串行的操作（仅以下允许串行）

- `git add → git commit → git push`：git 操作有严格顺序依赖
- `ceremony_scan 输出 → spawn 工位`：工位列表依赖扫描结果
- `TeamCreate → Task spawn`：spawn 依赖 team 存在

除上述三项外，Lead 的任何串行执行都是违规。

**持久化规则**：状态结晶发生在状态转换点，不是终点。session 是蜂群跨上下文的唯一状态载体，必须在每个关键转换点更新：
- 工位完成 → 增量写入
- 循环回扫描之前 → 强制写入 + commit + push
- 持久化 ≠ 进程存活。持久化 = session 结晶 + 热启动 + 谱系 DAG 跨 session 延续（162号）

汇报格式：
```
[蜂群名] 工位 {name}: {状态}
  产出: {简述}
  问题: {如有}
```

**关键：每次 Stop hook 拦截时，执行步骤 1（调用 TaskList），不要再次尝试停止。**

## 绝对禁止

- **全生命周期禁止额外 Bash**，以下白名单例外（077-C，Gemini decide 选项B）：
  - `python scripts/ceremony_scan.py`（步骤 1）
  - `python scripts/gangju_analysis.py`（步骤 7.5：纲举目张分析）
  - `git add` / `git commit` / `git push`（持久化不变量）
  - `git fetch` / `git rebase`（push 失败时的恢复）
  - session 文件写入（增量持久化）
- **不运行额外的 Read/Glob**（所有信息已在 JSON 中）
- **不输出确认请求**（不问"是否正确"、"待确认"）
- **不输出等待信号**
- **不用 Explore agent 替代 Task**——Explore 是只读搜索工具，不是蜂群节点
- 白名单膨胀超过 5 项时需重新审视 ceremony 设计

## 谱系引用

- 075号：结构工位从 teammate 转为 skill + 事件驱动
- 058号：ceremony 是 Swarm₀
- 057号：LLM 不是状态机
- 056号：蜂群递归是默认模式
- 069号：递归拓扑异步自指蜂群
- 162号：否定"不执行 TeamDelete"——持久化由 RTAS 循环保证
- 174号：谱系即生成引擎——RTAS 是谱系的实现
- 175号：认识论反转落地——谱系驱动架构 + Codex 谱系异质否定
- 179号：四角结构——拓扑分析家作为第四位置（上下文隔离冷读）
