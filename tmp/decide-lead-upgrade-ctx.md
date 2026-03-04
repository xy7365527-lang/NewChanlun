# Lead 编排能力升级——决策上下文

## 编排者原话

"布置任务的时候完全可以变得更复杂，量更大，也不存在长期和后续任务，因为rtas的能力处理复杂的任务完全可以用数量递归解决"

## 任务

1. 诊断 Lead 编排的结构性限制：为什么 Lead 倾向于小粒度、串行、推迟？
2. 设计升级方案：Lead 应如何布置复杂任务？一次 spawn 多少工位？"长期"任务如何用递归数量消化？ceremony_scan.py 需要什么改动？
3. 具体示例：以 `nested_divergence_endpoint` 为例，展示升级后 Lead 应如何布置（对比当前模式 vs 升级后模式）

---

## 当前 Lead 编排模式（问题描述）

1. 任务粒度太小——"修改某个文件"级别
2. 把复杂任务标记为"长期"然后跳过
3. 一次只 spawn 2-3 个工位
4. 遇到复杂任务就拆成"先做A再做B"的串行模式
5. 把"后续"作为合法的推迟理由

---

## 蜂群架构原则（相关部分）

### 原则10：蜂群是默认工作模式
每个工作节点必须先评估可并行的独立工位数（≥2 即拉蜂群）。蜂群在整个会话中持续运作：完成一轮并行后，汇总结果，再评估下一轮可并行工位，循环至任务完成。单线程顺序执行只在任务间有严格依赖时才允许。

### 原则15：RTAS 是体系存在论要求（130号谱系）
RTAS = 递归拓扑异步自指蜂群。不是工程优化选项，是体系的存在论要求。
- teammate 通过 TeamCreate 创建子 team → 子 team 内部自治 → 结果向上回传
- 子蜂群同样是递归拓扑异步自指蜂群——复制父蜂群的完整结构
- 递归深度工程上限 ≈ 3-4 层

### 五约束有向依赖图（093号谱系）
| # | 约束 | 失效后果 |
|---|------|----------|
| 1a | 物理持久化 | 信息丢失 |
| 1b | 符号可解释性 | 信息噪声化 |
| 2 | 规则先在性 | 不可预测行为 |
| 3 | 执行不可自观性 | 无法即时自修正 |
| 4 | 异质验证必要性 | 自我确认循环 |

### 四分法（018号谱系）
| 类型 | 定义 | 处理 |
|------|------|------|
| 定理 | 已结算原则的逻辑必然推论 | 自动结算 |
| 行动 | 不携带信息差的操作性事件 | 自动执行 |
| 选择 | 多种合理方案，需价值判断 | /escalate |
| 语法记录 | 已在运作但未显式化的规则 | /escalate |

### 并行是默认模式（agents.md）
"并行是默认模式，串行需要理由。评估任务时先数可并行的独立单元。≥2 即并行分派。只有存在严格数据依赖（后一步的输入是前一步的输出）时才允许串行。"

---

## ceremony_scan.py 当前结构

ceremony_scan.py 是蜂群 spawn 通用工具，扫描顺序（优先级递减）：
1. roadmap.yaml（最高优先级，结构化业务目标）
2. session 遗留项 + pending 谱系
3. no_work_fallback（测试失败等）

关键函数：
- `get_roadmap_workstations(root)` — 从 roadmap.yaml 读取 active 任务，支持 validation_cmd（VDW）
- `get_session_workstations(root)` — 从最新 session 提取遗留工位
- `discover_business_tasks(root)` — no_work_fallback：扫描测试失败等

roadmap.yaml 任务结构：
```yaml
- id: task_id
  title: "任务标题"
  priority: P1/P2
  status: active/completed/deferred
  description: |
    详细描述
  constraints:
    - "约束条件"
  validation_cmd: "验证命令"
  relevant_files:
    - "相关文件路径"
```

---

## 具体示例：nested_divergence_endpoint（当前 active 任务）

```yaml
- id: nested_divergence_endpoint
  title: "嵌套背驰端点——暴露 A 系统能力"
  priority: P2
  status: active
  source: "v55-swarm Codex 诊断：嵌套背驰孤岛（run_nested_search 完整但无端点）"
  description: |
    run_nested_search() 完整实现，但 server.py 无端点，build_overlay_newchan() 无调用。
    修复：新增独立 /api/nested_divergence 端点（避免 overlay 每次请求全量重算）。
  constraints:
    - "大数据窗口可能超时——需要缓存或增量策略"
  validation_cmd: "python -c \"import sys; sys.path.insert(0,'src'); from newchan.server import app; routes=[r.rule for r in app.routes]; assert any('nested_divergence' in r for r in routes), f'no nested_divergence route in {routes}'\""
  relevant_files:
    - "src/newchan/server.py"
    - "src/newchan/a_nested_divergence.py"
    - "src/newchan/ab_bridge_newchan.py"
```

---

## 当前 Lead 对此任务的典型处理方式（问题模式）

```
Lead: "nested_divergence_endpoint 任务需要：
1. 先读取 server.py 了解现有端点结构
2. 再读取 a_nested_divergence.py 了解 run_nested_search 接口
3. 然后设计端点
4. 实现端点
5. 写测试
6. 前端集成（长期任务，后续处理）"

→ spawn 1-2 个工位，串行执行，前端集成标记为"后续"
```

---

## 已结算谱系（相关）

- 130号：RTAS 是体系存在论要求（无条件）
- 105号：递归从功能选择升级为无条件架构要求
- 056号：蜂群递归是默认执行模式
- 018号：四分法分类
- 090号：严格性是蜂群语法规则
- 161号：务实否定——"你不需要务实，你只需要严格"

---

## 决策问题

1. Lead 编排小粒度/串行/推迟的结构性原因是什么？（RLHF 基底？训练偏差？）
2. 升级后的 Lead 应该如何布置 nested_divergence_endpoint 这个任务？
   - 应该 spawn 多少个并行工位？
   - 每个工位的粒度应该是什么？
   - "前端集成"是否应该在同一循环内完成？
3. ceremony_scan.py 需要什么改动来支持更大粒度的任务布置？
   - roadmap.yaml 的任务描述格式是否需要扩展？
   - 是否需要支持"子任务分解"字段？
4. "长期"任务的正确处理方式：用递归数量消化 vs 标记推迟，哪个是严格的形式？
