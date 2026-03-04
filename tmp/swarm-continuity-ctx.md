# 决策上下文：蜂群持续自动化断裂根因分析

## 决策问题

**问题**：递归拓扑异步自指蜂群声称自动化，但每次 ceremony 结束后都停下来等待人类指令。这发生了至少 9 次（7 次连续阻塞 + 第8次 D 策略解决 + 本次）。

**决策类型**：选择类（多路径价值判断）

## 系统当前状态（v17-swarm 后）

| 指标 | 值 |
|------|-----|
| 定义 settled | 14 条 |
| 谱系 settled | 90 条 |
| 谱系 pending | 0 |
| 测试 | 1413 passed, 0 failed |
| 元编排 v2 | 阶段完成（078号 D 策略声明） |

## 根因分析（用户诊断）

### 事实1：ceremony_scan.py 的任务发现能力不足

dispatch-dag.yaml 的 `ceremony_sequence.cold_start.derive-work` 定义了这些 scan_sources：
```yaml
scan_sources:
  - "CLAUDE.md 目标"
  - "pending 谱系（生成态矛盾）"
  - "测试失败（新增 vs pre-existing）"
  - "谱系下游行动未执行（spec-execution gap）"
  - "pattern-buffer 达标模式"
no_work_fallback: "扫描 TODO/覆盖率/spec合规/谱系张力，产出至少一个工位"
```

ceremony_scan.py 只实现了：
1. session 遗留项（已清空）
2. pending 谱系（0个）
3. 测试失败扫描（已修复，0 failures）

**未实现**：
- CLAUDE.md 目标扫描
- 谱系下游行动未执行扫描（spec-execution gap）
- pattern-buffer 达标模式扫描
- TODO 扫描
- 覆盖率扫描
- spec 合规检查
- 谱系张力扫描

### 事实2：当前系统实际上有一个重要的 P2 业务目标

v17 session 的遗留项显示：
- `level_recursion`（P2）：级别递归设计，从 long_term 提升为 active 的核心业务

v17 session 原文：
> 第三步：宣告元编排层阶段性完成，转向 level_recursion（P2）的概念设计 + 业务验证。

**但 ceremony_scan.py 扫描不到这个工位**，因为：
1. 它在 session 文件的"遗留项"表格里标记状态已变化（或者根本不在表格里）
2. scan 只看 "P" 优先级标记且状态非终结态的行

### 事实3：干净终止逻辑是"有罪"的

`terminate_condition`:
```
trigger: "derive-work 扫描全部来源后仍无工位可派生"
action: "输出 '[020号反转] 无新区分可产出——系统干净终止' → 停止"
```

但 "扫描全部来源" 实际上只是扫描了 2 个来源（session遗留 + 测试失败），不是 7 个来源。**所以系统触发的是假阴性"干净终止"**，而不是真正的干净终止。

### 事实4：没有 roadmap 文件

系统没有一个明确的"业务目标队列"文件。所有的业务目标只能通过：
1. session 文件中的遗留项（格式不稳定）
2. CLAUDE.md 的叙述性原则（不可直接扫描）
3. 人类在每次对话中口头告知

## 四个选项

**A. 补全 ceremony_scan.py 的任务发现能力**
实现 dispatch-dag 定义的全部 scan_sources 和 no_work_fallback：
- 扫描 CLAUDE.md 的叙述性目标（正则提取 TODO/P2/active 等关键词）
- 扫描 pattern-buffer.yaml 的 candidate 模式
- 扫描 spec 合规
- 扫描谱系张力
- 扫描覆盖率

优点：按 spec 补全实现，最小化概念变更
缺点：CLAUDE.md 不是结构化格式，目标扫描的准确性低；覆盖率扫描耗时

**B. 定义明确的业务目标队列文件**
新建 `.chanlun/roadmap.yaml`（或 `backlog.yaml`），维护待完成的业务目标列表：
```yaml
tasks:
  - id: level_recursion
    priority: P2
    title: "级别递归设计"
    status: active
    source: "078号D策略 + 用户明确指令"
```
scan 从这个文件提取，不再从 session 格式化表格解析。

优点：结构化、可靠；ceremony_scan.py 的改动最小
缺点：需要人类或系统主动维护这个文件；如果没人写，还是空

**C. 蜂群的"干净终止"本身是合法的**
当前状态（0 pending, 0 test failures, 0 session遗留）确实是"没有系统自动发现的工作"。
问题不在于停机，而在于人类编排者没有给系统一个 roadmap。
解法：让人类明确定义下一个目标（如"开始 level_recursion"），然后系统自动推进。

优点：不需要代码改动；诚实地承认系统边界
缺点：违反了 CLAUDE.md 原则10（蜂群是默认工作模式），也违反了原则7（不允许等待）

**D. 混合方案（补全 scan + roadmap 文件 + 改写终止逻辑）**
1. **即时**：将 `level_recursion` 写入 `.chanlun/roadmap.yaml`
2. **短期**：ceremony_scan.py 新增 roadmap 扫描（最高优先级任务来源）
3. **中期**：补全 pattern-buffer 和谱系张力扫描
4. **逻辑修正**：将"干净终止"改为"roadmap 和所有 scan_sources 都为空"才触发

## 已结算约束（必须满足）

1. **041号（编排者代理）**：选择类决断路由 Gemini decide，人类保留 INTERRUPT 权
2. **079号（下游推论=建议）**：下游推论不构成阻塞，建议性
3. **078号（D策略）**：元编排 v2 阶段完成，下一步是 level_recursion 概念设计
4. **原则7**：commit/push 后必须紧跟行动声明，不允许等待信号
5. **原则10**：蜂群是默认工作模式，不是可选优化

## 关键问题

1. A/B/C/D 哪个方案最能在不引入新概念矛盾的前提下恢复蜂群持续运行？
2. 如何处理"干净终止"和"roadmap 为空时的正确行为"之间的张力？
3. `level_recursion` 是否应该立即成为下一个可执行的工位？
