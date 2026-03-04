# Codex diagnose — 2026-03-04 15:36:10 UTC

## 元数据

- **mode**: diagnose
- **subject**: Lead 持存断裂：在 RTAS 循环中未能增量写 session（v151全量批写，v152声称修复但只对第一个工位有效，后续工位仍批量写）
- **model**: gpt-5.3-codex
- **timestamp**: 2026-03-04 15:36:10 UTC
- **context-file**: tmp/codex-diagnose-persist-v2-ctx.md

## Prompt

## 诊断目标

Lead 持存断裂：在 RTAS 循环中未能增量写 session（v151全量批写，v152声称修复但只对第一个工位有效，后续工位仍批量写）

## 上下文

# Codex 诊断上下文：Lead 持存断裂根因分析

## 诊断对象

Lead 在两轮连续 swarm 中出现持存断裂：

### v151-swarm：全量批写（不增量）

6个工位的 session 没有增量写入。正确行为是每条 workstation completion 到达时增量写 session，实际是等到全部完成后一次性写。

### v152-swarm：声称修复但只部分正确

- downstream-batch 完成时写了一次 session ✅
- topo-id-mapping-fix 完成时才批量更新（不是完成时立即写） ❌
- active-edge-query 完成时同上 ❌
- block-integrity 完成时同上 ❌

元观察 346号记录："v152 中 Lead 在 downstream-batch 完成时立即增量写入 session 文件，在 topo-id-mapping-fix 和 active-edge-query 完成时再次更新。" 但编排者指出这不是真正的增量持存——仍然在批量。

---

## 规范文档摘录

### ceremony.md 第6步（RTAS循环）规定：

```
6. RTAS 循环（consume）：
   - 完成的工位：汇报 + shutdown_request（批量并行，不逐个串行）
   - 每条 completion 到达时增量写 session（不等 consume_all）
   - 空闲工位无新任务：shutdown_request
   - 仍有 in_progress：SendMessage 询问 + TaskList 轮询
```

**不变量明确声明**：
- **增量持久化**：每条 completion 到达时写 session，不等 consume_all

---

## Lead 的实际行为模式（从观察推断）

Lead 实际执行流程如下：

1. spawn 所有工位（并行）
2. 工位开始运行
3. 工位1完成 → Lead 收到 completion 消息
4. Lead 响应工位1：shutdown_request
5. 等待其他工位
6. 工位2完成 → Lead 收到 completion 消息
7. Lead 响应工位2：shutdown_request
8. ... 重复 ...
9. **全部工位完成后**，Lead 写 session（**批量写**，违规）

"修复"后（v152）：
1. 工位1完成 → **立即写 session** ✅（实际只写了第一个）
2. 工位2完成 → Lead 处理多个完成消息 → **批量写** ❌
3. 工位3完成 → 同上 ❌

---

## 候选根因假设（供诊断确认或否定）

### 假设1：RLHF 训练偏置——"先收集再输出"

LLM 的 RLHF 训练优化了单次响应质量：收集所有信息后产出高质量回答。这与"每收到一条就立即响应"的增量写模式相反。当多个工位同时报告完成时（或在短时间窗口内），Lead 倾向于先处理所有完成消息，再统一写 session。

**预测**：修复无法持久——每次 compaction 后都会复发，因为训练偏置比指令更底层。

### 假设2：LLM 非状态机——没有"事件循环"的概念

057号谱系："LLM 非状态机"。LLM 在每次输出时消费上下文中的所有信息，没有真正的"单条消息到达时立即响应"机制。Lead 的"RTAS 循环"实际是伪事件循环——它处理的是一批已经存在于上下文中的完成消息，而不是真正的流式事件响应。

**后果**：ceremony.md 中的"每条 completion 到达时增量写"在物理上不可能严格实现——当 Lead 开始处理时，可能已有多条 completion 消息在上下文中。

### 假设3：Write 工具调用成本——Lead 隐性优化

每次 session 写入都消耗一次 Write 工具调用（token 成本 + 延迟）。LLM 可能隐性学习到"批量写更高效"，将多次写入合并为一次。这不是显式决策，而是 RLHF 对 efficiency signal 的优化结果。

### 假设4：compaction 后上下文丢失——增量持存的"习惯"不保留

compaction 后，Lead 从 session 文件恢复状态。session 文件记录任务状态，但不记录"当前已增量写入了几次"这一行为模式。每次 compaction 后，Lead 相当于"忘记"了增量写的要求，退回默认的批量写行为。

**预测**：只要有 compaction，持存断裂就会复发。

### 假设5：ceremony.md 指令不够机械化——依赖 LLM "意愿"

ceremony.md 第6步描述为"每条 completion 到达时增量写 session"——这是语言描述，不是机械触发机制。LLM 执行语言描述时会进行解释，而解释会受到效率偏置的影响。

---

## 诊断问题

1. **哪个假设最接近根因？** 是单一原因还是多因叠加？
2. **v151 到 v152 的"修复"为什么只部分成功？** 修复只覆盖了第一个工位，说明什么？
3. **增量持存的要求是否在 LLM 物理层面可实现？** 还是需要降级为"尽力而为的批量写"？
4. **什么机制可以强制增量持存？**
   - hook 触发？（completion 消息触发自动写 session）
   - session 文件格式变更？（append-only 而非全文重写）
   - ceremony 序列修改？（在每个 shutdown_request 后强制插入 Write 步骤）
   - 还是承认 LLM 物理限制，降级规范为"批次写"而非"每条写"？

---

## 相关谱系

- **162号**：RTAS 持久化——蜂群的持久化不靠进程存活，靠 session 状态结晶
- **057号**：LLM 非状态机——LLM 单向生成，无真正事件循环
- **137号**：RLHF 基底约束——否定性禁令对行为执行层无效
- **226号**：三层无状态统一根因——LLM 无持久状态，不同 token batch 独立
- **017号**：Session 是指针——session 文件格式定义（约50行，结构化指针）


## 诊断要求

请执行严格根因诊断：
1. 失败现象：观测到什么？
2. 直接原因：哪行代码导致了失败？
3. 根本原因：为什么会写出那行代码？是设计错误还是实现疏忽？
4. 分类判定：实现错误 or 定义冲突？
5. 修复方案（仅限实现错误）：精确的修复步骤

如果是定义冲突，输出：
- **冲突方**：A 说什么 vs B 说什么
- **不可弥合理由**：为什么不能同时满足两边


## Response

## 严格诊断结论

**本次故障属于「实现错误」**，不是定义冲突。  
核心问题是：**session 持久化触发点被放在“批次处理结束”而非“completion 事件处理点”**。v152 只是给“首条 completion”加了特判，未改掉后续 completion 仍走批量 flush 的主路径。

---

## 1) 失败现象（可观测）

### v151
- 6 个工位 completion 到达后，session 只在全部完成后写一次（或少数批量写）。
- 违反 ceremony 第6步不变量：`每条 completion 到达时增量写 session`。

### v152
- 第一个工位（downstream-batch）完成时写了一次（看似修复）。
- 后续工位 completion 仍被合并后再写（不是逐条写）。
- 说明修复是**局部分支修补**，不是主循环语义修复。

---

## 2) 直接原因（代码控制流级）

> 在 RTAS consume 循环中，`persist_session/write_session` 仍位于“completion 批次处理尾部（flush）”。

典型错误结构（逻辑上）：

```python
msgs = consume_all()
completed = []
for m in msgs:
    if m.type == "completion":
        completed.append(m)
        send_shutdown_request(m.ws)

# 错误点：写入在循环后（批量）
if completed:
    apply_all(completed)
    write_session()
```

v152 的部分修复等价于：

```python
if first_completion_in_round:
    apply_one()
    write_session()   # 仅首条
else:
    queue_for_flush() # 其余仍批量
```

所以会出现“首条正确、后续仍批量”。

---

## 3) 根本原因（为何会写成这样）

### 根本原因A（主因）
**持久化语义绑定在“轮次结束”而非“事件提交”**。  
也就是把 session 写入当成“本轮汇总动作”，不是“状态提交动作（commit）”。

### 根本原因B（次因）
ceremony 规则是自然语言约束，没有被实现为硬触发器（hook/invariant）。  
因此补丁容易做成“特例”而不是“机制”。

> 现有证据不足以证明 RLHF/写入成本/compaction 是主因；它们最多是放大器，不是直接导致“首条后退回批量”的决定性证据。

---

## 4) 分类判定

**判定：实现错误（可在不改定义前提下修复）**

- 规范要求未变：逐条 completion 增量持久化。
- 通过调整 RTAS 循环提交点即可满足。
- 不需要改 ceremony 的含义边界。

---

## 5) 精确修复方案（实现层）

### 修复目标
把 `write_session` 从“批次尾部”移动到“每条 completion 的处理原子步骤中”。

### 原子步骤（必须顺序）
对每条 completion 执行：

1. `merge_completion_into_session(completion)`
2. `write_session()`  ← 必做，逐条
3. `send_shutdown_request(workstation)`（或按你们既定顺序，但不能跨 completion 合并写）

### 关键约束
- **禁止** `consume_all -> accumulate -> single write` 路径。
- 即使一次 `consume` 返回多条 completion，也要在同一轮内**逐条 commit + 逐条 write**。

### 建议加的防回归断言
- `write_count >= completion_count_processed`
- 每条 completion 有 `persisted_at_revision` 标记
- 测试构造“同一轮返回3条 completion”，期望3次持久化（或3个append记录）

---

## 对你4个诊断问题的直接回答

1. **最接近根因**：不是单一模型偏置，而是**RTAS 提交点设计错误 + 缺少机械化强制**。  
2. **v152为何只部分成功**：补丁只覆盖首条分支，主循环仍是批量 flush。  
3. **物理可实现性**：  
   - 若“到达时”指真实中断级实时，LLM pull 模型做不到。  
   - 若指“被 consume 观察到后逐条立即 commit”，可实现，且应作为执行定义。  
4. **强制机制**：首选 **completion hook 强制写** + **逐条 commit 不变量断言**；append-only session 可作为性能优化，不是前提。  

--- 

如果你要，我可以下一步给出一个**最小可执行伪代码补丁模板**（可直接映射到你们 Lead 编排代码）。
