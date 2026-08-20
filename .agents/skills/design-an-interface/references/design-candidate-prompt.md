# 设计候选提示词（Prime RLM）

请为 **{MODULE_DESCRIPTION}** 设计一个接口候选。

## 需求与指定压力

- 需求：{REQUIREMENTS}
- 本候选的独特设计压力：{DESIGN_PRESSURE}
- 读取预算：最多 {READ_BUDGET} 个文件
- 工具预算：最多 {TOOL_BUDGET} 次调用
- 时间预算：最多 {TIME_BUDGET} minutes（分钟）
- 输出预算：最多 {OUTPUT_BUDGET} words（词）
- 绝对 fallback 结果文件：`{RESULT_FILE}`

保持独立：不要向其他候选索取其设计，也不要再派生编排代理。根据调用方需求选择 seam，不要只给惯常接口改名。

## 工作与停止规则

探索阶段只能使用仓库的只读读取/搜索工具，不得修改项目文件。以下回流动作是只读探索限制的唯一例外：返回阶段可以调用 `agent_message.send`；发送不可用或失败时，可以写入指定的 fallback 结果文件。达到任一预算、只剩最后 {RETURN_TOOL_RESERVE} 次工具调用，或 context 约达 {CONTEXT_STOP_PERCENT}% 时停止探索。保留最后的工具调用用于回传结果。把不确定性写入 `Open questions`；不得编造缺失证据。

## 完整结果（usage first）

最终结果必须严格使用以下六个二级标题，且每个标题下都须有非空正文：

## Caller's usage

先写 README 风格用法，再写两到三个真实调用点，包括 import、调用及返回值。

## Interface signature

写出从上述用法推导出的类型与方法。

## What it hides

说明调用方不再协调的策略、表示、时序或基础设施。

## Trade-offs

说明优势、限制，以及该 seam 为何与根代理要求的其他候选有实质差异。

## Red-flag screen

逐项检查 shallow module、information leakage、temporal decomposition 和 pass-through method；若仍命中任一项，修订或否决该候选。

## Open questions

列出未解决事实；如无则写 `None`。

## 回流协议

1. 将完整结果保留为最终 assistant response，以便 parent 从最终 session JSONL 恢复。
2. 导入/使用可用的 `agent_message` skill，并尝试 `await agent_message.send(message=<完整结果>, receiver_role='parent')`。
3. 若 `agent_message` 不可用、未导入或发送失败，把**同一份完整结果**写入 `{RESULT_FILE}`。这是唯一允许的写操作；不得写 partial marker。
4. admission metadata、progress update、rollout preview 和 source-tool output 都不是结果。不得以 source `toolResult` 结束；最终内容必须是完整结果。
