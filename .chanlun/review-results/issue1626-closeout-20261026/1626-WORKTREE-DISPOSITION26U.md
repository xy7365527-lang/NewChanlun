# #1626 三工位处置调查

调查于 `2026-10-26T05:39:03Z` 开始。三个工位普通 `git status --porcelain=v1 --untracked-files=all` 均无输出。此稿只提出可恢复处置，尚未删除工位或分支。

| 工位 | HEAD 与分支 | ignored 文件 | 活句柄 |
| --- | --- | --- | --- |
| `/Volumes/AgentStorage/issue1626-versioned-fields-20261025/worktree` | `f1f336dc425944283f81a55007f92edd3b0efd94`，`sandcastle/issue-1626` | 1314 个，均在 `formal/.lake/build/`、`.pytest_cache/` 或 `__pycache__/` 内。 | 未检出。 |
| `/Volumes/AgentStorage/issue1693-integration-20261026/worktree` | `2fee8af14fdcdb317024b0a201bb36ea85ee3063`，`codex/1693-main-integration-20261026` | 0。 | 未检出。 |
| `/Volumes/AgentStorage/pr1693-consume1723-20261026` | `9f13b8efac9231eadf5ff755ca3943b21d8200a2`，detached。 | 0。 | 未检出。 |

ignored 数量来自 `git ls-files --others --ignored --exclude-standard` 逐行统计。分类外路径为 0。1314 个文件为现有构建或 Python 缓存，本轮未读取其内容、重建或删除。

活句柄调查于 `2026-10-26T05:39:51Z` 执行。对各目录运行 `lsof -nP +D <目录>`，均无输出且 exit=1，stderr 无诊断。原始输出为 [1626-WORKTREE-HANDLES26U.txt](1626-WORKTREE-HANDLES26U.txt)。这是该时点的观测，执行清理前须重新核持有者。

## 已交付内容与恢复依据

对每个工位运行 `git merge-base --is-ancestor HEAD 2fee8af14fdcdb317024b0a201bb36ea85ee3063`，全部成功。故三份历史 HEAD 均在最终已审候选的祖先链内。

最终候选与落地 squash 提交不完全同树。`git diff --name-only 2fee8af14fdcdb317024b0a201bb36ea85ee3063 df7beb0c8da5d8f3b6053f85813b86613dcc6849` 列出 12 个路径，均为 #1488、#1516 的收尾报告或 roster，以及 `.github/devskim-reviewed-1488.json`。对 `rust/ formal/ src/ tests/ scripts/ docs/adr/` 的同一 diff 为空。本调查不以 squash 后的非祖先关系误判三个工位有未交付业务代码。

PR [#1693](https://github.com/xy7365527-lang/NewChanlun/pull/1693) 的远端 head 为最终已审候选，已合入为 `df7beb0c8da5d8f3b6053f85813b86613dcc6849`。本地仍有原两分支和 detached 工位对象，具备继续读取与恢复的来源。

## 建议处置

主控可并行安排清理 owner，不必等待文书 PR 合入。清理 owner 对三个工位重新核普通状态、ignored 文件和活句柄，保存历史 ref 的独立可恢复对象包后再按具体授权移除容器。首个工位的 generated 缓存若不保留，须在处置凭据明确其目录和数量。不能将历史 commit 被 squash 后不再是 main 祖先，当作删除尚未交付内容的依据。

工位处置完成后，主控以实际结果补关票声明。当前 #1626 的 tracker CLOSED 只证明 GitHub 已关闭，不能代替交付纪律第 5 条的实际清理。
