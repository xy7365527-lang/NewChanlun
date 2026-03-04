# Codex Review Context: 共识仪式触发协议的代码层可行性

## 1. write_consensus_ceremony() 完整签名和参数说明

```python
def write_consensus_ceremony(
    trigger_block_id: str,         # 触发质询的原始 CC 产出区块 id
    conclusion: str,               # 最终达成的结论
    gemini_conceded: list[str],    # Gemini 让步的内容列表
    codex_conceded: list[str],     # Codex 让步的内容列表
    concession_reasons: dict[str, str],  # 让步理由 {item: reason}
    unresolved: list[str],         # 未解决但暂时搁置的部分
    source: str = "cc",            # 来源标识
    base: Path = DEFAULT_BASE,     # 区块拓扑根目录 (.chanlun/block-topology)
) -> dict[str, dict]:              # 返回 {"consensus": ..., "residue": ..., "tension": ...}
```

产生 5 个区块（3 primary + 2 rewrite）和 4 条关系（2 order-1 + 2 order-2）。
通过 `_atomic_write_ceremony()` 原子写入（临时目录策略）。

## 2. 当前质询循环的代码路径

### Gemini 质询路径
```
gemini-challenger agent
  -> 构建上下文文件 (tmp/challenge-ctx.md)
  -> .venv/Scripts/python -m newchan.gemini_challenger challenge "<subject>" --tools --verbose --context-file ...
  -> Gemini 通过 Serena MCP 自主导航代码库
  -> agent 解析推理链，判定否定是否成立
  -> 成立: 写谱系 (.chanlun/genealogy/pending/)
  -> 不成立: 报告误判
```

### Codex 质询路径
```
codex-challenger agent
  -> 读取被审查代码 + 相关定义
  -> 构建上下文文件 (tmp/codex-review-ctx.md)
  -> .venv/Scripts/python -m newchan.codex review "<subject>" --context-file ...
  -> CodexChallenger._run_mode() -> call_with_fallback() -> OpenAI API
  -> ReviewResult 持久化到 .chanlun/review-results/codex-{mode}-{ts}.md
  -> agent 解析结果，判定否定
```

### Plan-Review 对审路径
```
plan-review skill (Phase 2-3)
  -> Opus 出方案 -> Codex 评审 (review 模式)
  -> 多轮: Codex 提质疑 -> Opus 修改 -> Codex 再评审
  -> 每轮持久化到 .chanlun/review-results/plan-review-{ts}-round{N}.md
  -> 终止: 达成共识 (Codex 显式确认) 或 6 轮后 /escalate
```

### 关键缺口
- **无生命周期管理**: 质询循环是 ad-hoc 调用（agent spawn -> 执行 -> 报告 -> 结束），没有"开始"和"结束"事件标记
- **让步轨迹在对话历史中**: Gemini/Codex 的让步发生在自然语言对话中（agent 的 SendMessage 交互），不在结构化数据中
- **无 trigger block**: 当前质询循环不产出 event 区块作为 trigger，write_consensus_ceremony() 的 trigger_block_id 参数无来源
- **plan-review 持久化**: `.chanlun/review-results/` 中的 Markdown 文件是最接近结构化让步记录的数据源，但格式是非结构化的 Markdown

## 3. 总方针 §21 原文

> **共识仪式。** 每次质询循环收敛时，Serena 强制执行共识仪式。三个区块原子性地同时写入：
>
> **共识区块（consensus）：** 最终达成的结论。
> ```json
> {"type": "consensus", "content": {"conclusion": "..."}, "refs": ["触发质询的原始CC产出区块id"]}
> ```
>
> **剩余区块（residue）：** 双方各自放弃了什么，放弃的理由。
> ```json
> {"type": "residue", "content": {"gemini_conceded": "...", "codex_conceded": "...", "reasons": "..."}, "refs": ["对应的consensus区块id"]}
> ```
>
> **张力区块（tension）：** 双方承认未解决但暂时搁置的部分。
> ```json
> {"type": "tension", "content": {"unresolved": "..."}, "refs": ["对应的consensus区块id"]}
> ```

## 4. 总方针 §63 补充

> **质询循环已有基础。** CC为主体，Gemini做概念质询，Codex做代码质询，通过Serena互相可见。共识仪式尚未形式化——质询过程的让步轨迹在对话历史中但没有被结构化提取为residue区块。下一步：在质询循环结束时强制触发共识仪式，产生consensus/residue/tension三区块。

## 5. 问题聚焦

从代码架构角度，共识仪式触发协议应该在哪个层实现？审查以下三种路径的可行性：

### 路径 A: Hook 层实现
- 在 `.claude/hooks/` 中新增 hook（如 `consensus-ceremony-trigger.sh`）
- 触发时机: 质询 agent 完成时（PostToolUse on SendMessage? Stop hook?）
- 优点: 不修改现有 agent 代码
- 问题: hook 如何知道质询循环已收敛？如何提取结构化让步数据？

### 路径 B: Agent 内嵌实现
- 修改 gemini-challenger.md 和 codex-challenger.md，在质询结束时调用 write_consensus_ceremony()
- 触发时机: agent 判定质询结束（达成共识 / 否定不成立）
- 优点: agent 拥有完整的质询上下文
- 问题: 谁来协调 Gemini 和 Codex 的让步合并？单个 agent 只能看到自己的视角

### 路径 C: 独立脚本/ceremony 层实现
- 新增独立的 ceremony 步骤（或扩展现有 ceremony）
- 由 team-lead 或 ceremony 流程在质询完成后调用
- 优点: 统一的协调点，可同时处理 Gemini 和 Codex 的让步
- 问题: 如何从非结构化的 review-results/ Markdown 中提取结构化让步数据？

### 跨路径问题
1. trigger_block_id 的来源: 当前质询循环不产出 event 区块，write_consensus_ceremony() 的 trigger_block_id 从哪来？
2. 让步数据提取: Gemini/Codex 的让步在对话历史（自然语言）中，如何转为 gemini_conceded/codex_conceded 结构化列表？
3. 收敛判定: 谁判断质询循环已"收敛"？plan-review 有显式终止条件（Codex 确认），但 challenge 模式的终止是 agent 隐式决定的
