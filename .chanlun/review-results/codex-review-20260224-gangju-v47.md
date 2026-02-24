# Codex review — 纲举目张代码层审计 v47

## 元数据

- **mode**: review
- **subject**: 纲举目张代码层审计：异步自指未实现、多轮质询管道不完整、residue 为空的代码原因
- **model**: gpt-5.2-codex（codex-5.3 不可用，降级）
- **context-file**: tmp/codex-gangju-v47-ctx.md

## Response

以下问题基于管道描述与缺失能力清单做代码层否定。

---

- **问题**：异步自指（t 审查 t-1）能力完全缺失，实现与调用链不存在，SS5-9 直接违反  
  **位置**：N/A（全仓库搜索无实现/无调用者）  
  **严重性**：致命  
  **修复建议**：新增定时/触发调度器，明确“上一轮审查结果 -> 新一轮自指审查请求”的生成与落盘；在 scan_and_trigger 或上游 orchestrator 中挂接此调度入口。

- **问题**：多轮质询 stance 序列收集管道不完整，`parse_stance_sequence(texts: Sequence[str])` 需要多文本输入，但当前只从单个 review 文件抽取，导致 `compute_stance_diff` 永远无法得到跨轮差分，residue/tension 恒空  
  **位置**：`scan_and_trigger._extract_stance_sequence_from_review`（文件路径未提供）  
  **严重性**：重要  
  **修复建议**：实现“多轮回复聚合器”：把 Gemini/Codex 多轮输出保存为多文件或单文件多段，并在 scan_and_trigger 中组装为 `Sequence[str]`；或引入 conversation-id 并批量读取同 id 下所有轮次。

- **问题**：residue 为空但未被当作输入不足处理，ceremony 仍落盘，造成“空 ceremony”污染数据  
  **位置**：`scan_and_trigger`/`write_consensus_ceremony`（文件路径未提供）  
  **严重性**：重要  
  **修复建议**：在 `compute_stance_diff` 前做最小轮次校验（>=2）与 concession 非空校验；不满足则跳过写入或标记为 invalid，并输出错误原因。

- **问题**：Nachtraeglichkeit（reopens/supersedes）只在 schema 中声明，缺少任何专用写入调用者，能力“只存在于类型而非流程”，定义不忠实  
  **位置**：`block_topology.py`（定义存在）+ N/A（无调用者）  
  **严重性**：重要  
  **修复建议**：新增显式 API（如 `write_reopen_relation`）并在 review/rewrite 触发链路上挂接；否则在审查流程中产生“追溯改写”永远不会被记录。

- **问题**：共识写入时 `refs=["unknown"]` 作为默认值掩盖触发缺失，破坏共识引用可追溯性  
  **位置**：`scan_and_trigger`（文件路径未提供）  
  **严重性**：建议  
  **修复建议**：将 trigger 缺失视为输入错误，直接失败或写入明确的 `invalid` 标记；不要使用 `"unknown"` 作为成功写入值。

---

```yaml
---stance-declaration---
verdict: fail
stances:
  async_self_reference_missing: reject
  multi_round_stance_pipeline_incomplete: needs_work
  residue_empty_but_written: needs_work
  nachtraeglichkeit_no_trigger: reject
  consensus_refs_unknown_masking: needs_work
concessions: []
---end-stance---
```

