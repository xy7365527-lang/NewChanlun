# Lead 并行调度规则（218号谱系）

此规则优先级与 `no-workaround.md` 相同。

## 核心命题

Lead 是 RTAS 的一环，不是 RTAS 之外的串行瓶颈。Lead 的一切操作遵守与工位相同的并行原则。

## 禁止的串行模式（RLHF 串行惯性）

1. **先总结再行动**：commit 后先输出总结段落，再决定下一步 → 总结嵌入格式A（143号已识别）
2. **逐个检查工位状态**：一个一个轮询工位 → 批量 TaskList + 批量处理
3. **等待测试完成再 commit**：Lead 自己跑 pytest 等结果 → spawn 测试工位，Lead 只消费结果
4. **先全关工位再开始下一轮**：等所有 shutdown 完成再 re-scan → shutdown 和 re-scan 可重叠
5. **等待确认再 re-scan**：commit/push 后等待确认 → 直接并行 re-scan

## 允许的串行（严格数据依赖）

仅以下操作允许串行，因为存在不可消除的数据依赖：

1. `git add → git commit → git push`：git 操作有严格顺序依赖
2. `ceremony_scan 输出 → spawn 工位`：工位列表依赖扫描结果
3. `TeamCreate → Task spawn`：spawn 依赖 team 存在

## 判断标准

在执行任何操作前，Lead 必须自检：
- 这个操作依赖前一个操作的输出吗？
- 如果不依赖 → 并行
- 如果依赖 → 串行，但必须能说出具体依赖什么数据

"方便"、"简单"、"习惯"不是串行的理由。

## 发生史

- v56-swarm/v57-swarm：编排者观察到 Lead 串行执行测试、逐个处理工位、等待后再行动
- Gemini + Codex 双向诊断确认：Lead 串行残余是结构性违规
- 090号：严格性语法规则——保留串行残余 = 不严格

## 谱系依据

- 218号：Lead 并行化——割掉 RTAS 的串行尾巴
- 090号：严格性永久化为蜂群语法规则
- 143号：commit 后总结是 RLHF 停顿点
- 069号：递归拓扑异步自指蜂群
