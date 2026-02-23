# 159号谱系：Codex 持久化调用——写代码标配

**id**: 159
**status**: 已结算
**type**: 语法记录
**date**: 2026-02-23
**前置**: 155-codex-heterogeneous-code-review

## 来源

编排者指令："codex 的讨论在写代码或用蜂群的时候应该持久化调用，因为这个是让代码可行的关键，这样后面不需要改那么多 bug"

## 内容

Codex 异质审查从"条件触发"提升为"蜂群写代码标配"：
1. **持久化调用**：dispatch-dag codex-challenger 增加 `file_write` 触发（`src/**/*.py`），蜂群工位写入 Python 代码后自动触发 Codex review
2. **结果持久化**：每次 Codex 调用的完整交互自动写入 `.chanlun/review-results/codex-{mode}-{timestamp}.md`
3. **前置守卫**：Codex 审查不再是事后可选项，而是代码产出流程的标配步骤——减少后续 bug

### 实现

- `ReviewResult` 增加 `to_markdown()` 方法（frozen dataclass，返回新字符串）
- `ReviewResult` 增加可选字段 `context_file: str | None`
- CLI (`__main__.py`) 执行结束后自动将结果写入 `.chanlun/review-results/codex-{mode}-{YYYYMMDD-HHmm}.md`
- 持久化内容包含：元数据头（mode, subject, model, timestamp）、完整 response、context-file 路径（如有）
- 持久化是自动的，无需额外 CLI flag

### 格式

```
# Codex {mode} — {timestamp}

## 元数据

- **mode**: {mode}
- **subject**: {subject}
- **model**: {model}
- **timestamp**: {timestamp}
- **context-file**: {path}  (如有)

## Response

{response}
```

## 边界条件

- 如果同一分钟内多次调用同模式，文件名会重复覆盖——当前接受此约束（实际使用中不太可能同一分钟对同模式调用两次）
- 如果 `.chanlun/review-results/` 目录不存在，`_save_result` 会自动创建

## 下游推论

1. **dispatch-dag file_write 触发**：codex-challenger 增加 `event: file_write, pattern: "src/**/*.py"` 触发——蜂群工位写 Python 代码后自动调用 Codex review
2. `.chanlun/review-results/` 成为异质审查的结构化档案（Codex 审查结果的持久化存储）
3. 蜂群工位写代码后的标准流程：写代码 → Codex review → 根据反馈修改 → 再审查

## 谱系依据

- 155号：Codex 代码层异质审查（Codex challenger 的引入）
- 062号：异质性作为结构性要素（异质审查的持久化 = 保存异质性的生成史）

## 影响声明

- 修改 `.chanlun/dispatch-dag.yaml`：codex-challenger 增加 file_write 触发事件（`src/**/*.py`）
- 修改 `src/newchan/codex/modes.py`：ReviewResult 增加 `context_file` 字段和 `to_markdown()` 方法
- 修改 `src/newchan/codex/__main__.py`：CLI 执行后自动持久化结果
- 修改 `.claude/agents/codex-challenger.md`：产出部分增加持久化要求
- 修改 `.claude/hooks/lead-audit.sh`：扩展白名单消除合法 Bash 误报
- 修改 `tests/test_codex_challenger.py`：增加 to_markdown 和 CLI 持久化测试
- 新增谱系 159号
