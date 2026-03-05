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
