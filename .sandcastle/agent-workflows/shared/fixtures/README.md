# 事件 fixture（#1128/#1173）

这些 JSON 是 GitHub `labeled` 事件 payload 的最小形态（issue 与 pull_request_target 两类），
只保留模型选择所需的 `action` / `label` / `issue` 或 `pull_request.labels` 字段；
workflow 的 EVENT_PAYLOAD 进一步只传 `labels.*.name` 数组。
用于 `shared.test.ts` 的模型 registry 对拍与 `verify-workflows.py` 的 fixture 覆盖检查。

| 文件 | 场景 | 期望 |
| --- | --- | --- |
| `issue-labeled-default-claude.json` | 默认无模型标签 | Claude Sonnet 实装（claudeCode + `CLAUDE_CODE_OAUTH_TOKEN`） |
| `issue-labeled-deepseek-v4-pro.json` | 显式 DeepSeek | Prime Agent `deepseek` / `deepseek-v4-pro`（仅 `DEEPSEEK_API_KEY`） |
| `issue-labeled-conflict.json` | 多模型标签冲突 | fail-loud → `agent:blocked` |
| `issue-labeled-unknown.json` | 未知模型标签（票面字符串） | fail-loud → `agent:blocked`；票面字符串不成为 env/secret key |
| `issue-labeled-deepseek-missing-secret.json` | DeepSeek 缺 secret | 任何模型调用前 fail-loud → `agent:blocked` |
| `issue-labeled-deepseek-retry.json` | 同票重加 `agent:implement` retry | 同一 DeepSeek 选择；retry 不复活 #1002 旧 Kimi 常量 |
| `issue-labeled-not-deepseek-substring.json` | P1 敌对标签 `not-agent:model:deepseek-v4-pro` | 不命中 DeepSeek；默认 Claude（workflow 数组元素精确匹配） |
| `issue-labeled-deepseek-v4-pro-legacy.json` | P1 敌对标签 `agent:model:deepseek-v4-pro-legacy` | 不命中 DeepSeek；TS fail-loud `Unknown model label` |
| `pr-labeled-review.json` | Issue→Draft PR→review（无模型标签） | review 默认 Claude Opus（`CLAUDE_CODE_OAUTH_TOKEN`） |
| `pr-labeled-review-deepseek-v4-pro.json` | PR review 显式 DeepSeek | review 用 Prime Agent `deepseek` / `deepseek-v4-pro`（仅 `DEEPSEEK_API_KEY`；缺 key fail-loud） |
| `pr-labeled-implement-pr-deepseek-v4-pro.json` | PR 修正入口显式 DeepSeek | implement-pr 用 DeepSeek；review 仍独立 session（未打 review 模型标签时默认 Claude） |
| `pr-labeled-implement-pr-not-deepseek-substring.json` | P1 PR 敌对标签 `not-agent:model:deepseek-v4-pro` | 不命中 DeepSeek；默认 Claude |
| `pr-labeled-implement-pr-deepseek-v4-pro-legacy.json` | P1 PR 敌对标签 `agent:model:deepseek-v4-pro-legacy` | 不命中 DeepSeek；TS fail-loud `Unknown model label` |

#1002 的旧「Kimi 常量/票面不声明模型」本地主循环只作为历史存在，
本目录 fixture、workflow 与 registry 均不引用旧本地主循环，不复活。
